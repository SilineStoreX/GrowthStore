use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use config::{Config, ProcessState};
use flume::Receiver;
use ipc_channel::ipc::{IpcOneShotServer, IpcSender};
use proc::do_process;
use tokio::runtime::Handle;

use std::{
    fs::File, io::{BufReader, Read, Write}, path::{Path, PathBuf}, sync::Arc, thread::sleep, time::Duration
};
use serde::{Deserialize, Serialize};

mod config;
mod proc;

pub fn load_config<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(toml::from_str(&contents)?)
}

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
struct CommandArgs {
    #[arg(long, short)]    
    pub name: Option<String>,
    #[arg(long, short)]
    pub group: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandOpt {
    pub cmd: String,
    pub group: Option<String>,
    pub name: Option<String>,
}


#[derive(Debug, Clone, Subcommand, Deserialize, Serialize)]
pub enum StoreCommand {
    Start {
        #[arg(long, short)]    
        name: Option<String>,
        #[arg(long, short)]
        group: Option<String>,
    },
    Stop {
        #[arg(long, short)]    
        name: Option<String>,
        #[arg(long, short)]
        group: Option<String>,
    },
    Restart {
        #[arg(long, short)]    
        name: Option<String>,
        #[arg(long, short)]
        group: Option<String>,
    },
    Status,
    Shutdown,
    Service,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct StarterArgs {

    #[command(subcommand)]
    command: StoreCommand,
    
    #[arg(long, short, value_name = "FILE")]
    config: Option<PathBuf>,
}

/**
 * 后台服务过程
 * 服务：
 * 1、根据配置来进行检查每个进程
 * 2、偿试N次后失败后，将进程状态设定为失败；
 * 3、根据失败策略，确定是否重启该应用
 */
fn do_service(config: &config::Config) {
    tokio::runtime::Runtime::new().unwrap().block_on(async move {
        let share_filename = config.shared_file.clone().unwrap_or("share.pid".to_owned());
        let (tx, rx) = flume::unbounded::<CommandOpt>();
        let (tx1, rx1) = flume::unbounded::<Vec<ProcessState>>();

        Handle::current().spawn(async move {
            let rcrx1 = Arc::new(rx1);
            loop {
                let (server, server_name) = IpcOneShotServer::<(CommandOpt, IpcSender<Vec<ProcessState>>)>::new().unwrap();
                if let Ok(mut file) = File::create(share_filename.clone()) {
                    if let Err(err) = file.write(server_name.as_bytes()) {
                        log::error!("error to write server_name into file. {err:?}");
                    }
                }
                let (_, (val, tx1)) = server.accept().unwrap();
                if let Err(err) = tx.send(val.clone()) {
                    log::error!("send command to service with a error {err}");
                } else {
                    if let Ok(recv) = rcrx1.recv() {
                        log::info!("status: {recv:?}");
                        tx1.send(recv).unwrap();
                    }

                    if val.cmd == *"shutdown" {
                        sleep(Duration::from_secs(5));
                        log::info!("shutdown starter now.");
                        return;
                    }
                }
            }
        });

        service_main(Arc::new(rx), Arc::new(tx1), config.clone()).await;
    })
}


async fn service_main(rx: Arc<Receiver<CommandOpt>>, tx: Arc<flume::Sender<Vec<ProcessState>>>, config: config::Config) {
    let mut cmdopt: Option<CommandOpt> = None;
    loop {
        log::info!("now checking the process"); 
        let mut statelist = vec![];
        for proc in config.process.clone() {
            let g = proc.group.clone().unwrap_or("<default>".to_owned());
            let n = proc.name.clone().unwrap_or("<default name>".to_owned());
            

            if proc.is_unstarted() && !proc.is_manualstop() {
                log::info!("process is going to start......");
                if let Err(err) = proc.restart(&config) {
                    log::info!("error to start {}/{}. the error is {err}", g, n);
                }
                continue;
            }

            if !proc.is_manualstop() {
                log::info!("check process {}/{}.", g, n);
                if let Err(err) = do_process(&proc).await {
                    log::info!("process for {}/{} with error {err:?}.", g, n);
                }
            }

            let (start_it, stop_it, _) = match cmdopt {
                Some(ref cmd) => {
                    if cmd.cmd == *"shutdown" {
                        (false, true, false)
                    } else if Some(g.clone()) == cmd.group || Some(n.clone()) == cmd.name {
                        (cmd.cmd == *"start" || cmd.cmd == *"restart", cmd.cmd == *"stop", cmd.cmd == *"status")
                    } else {
                        (false, false, false)
                    }
                },
                None => {
                    (false, false, false)
                }
            };

            if start_it || proc.should_start() {
                let conf = config.clone();
                log::info!("Should to start and restart {g}/{n}");
                if let Err(err) = proc.restart(&conf) {
                    log::info!("restart process failures with error {err:?}");
                }
            }

            if stop_it {
                log::info!("Should to stop {g}/{n}");
                let conf = config.clone();
                if let Err(err) = proc.stop(&conf) {
                    log::info!("stop process failures with error {err:?}");
                }
            }

            {
                let conf = config.clone();
                let state = proc.state(&conf);
                statelist.push(state);
            }
        }

        if let Err(err) = tx.send(statelist) {
            log::error!("Send the state with error {err:?}");
        }

        if cmdopt.is_some() && cmdopt.unwrap().cmd == *"shutdown" {
            log::info!("starter service is going to shutdown...");
            return;
        }

        if let Ok(tc) = rx.recv_timeout(Duration::from_secs(config.interval.unwrap_or(5) as u64)) {
            log::info!("Recv: {tc:?}");
            cmdopt = Some(tc);
        } else {
            cmdopt = None;
        }
    }
}

fn do_command(config: &Config, cmd: &str, group: &str, name: &str) {
    if let Ok(mut file) = File::open(config.shared_file.clone().unwrap_or("share.pid".to_owned())) {
        let mut server_name = String::new();
        if let Err(err) = file.read_to_string(&mut server_name) {
            log::error!("error to read server_name into file. {err:?}");
        }

        let (tx, rx) = ipc_channel::ipc::channel::<Vec<ProcessState>>().unwrap();
        let sender = IpcSender::<(CommandOpt, IpcSender<Vec<ProcessState>>)>::connect(server_name).unwrap();
        let cmd = CommandOpt {
            cmd: cmd.to_owned(),
            group: if group.is_empty() {
                None
            } else {
                Some(group.to_owned())
            },
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_owned())
            },
        };

        if let Err(err) = sender.send((cmd, tx)) {
            log::error!("Send command error {err:?}");
        } else {
            if let Ok(states) = rx.recv() {
                log::info!("{:?}", states);
            }
        }
    }
}

// 1、使用fslock来确保服务只能启动一个实例
// 2、服务是管理（启动/停止）进程，检查进程健康情况的
// 3、--管理命令有：start -- restart -- stop --- status
// 4、--通过assets/configs/Starter.toml来进行配置
// 5、使用进程间通信方法（pipe）来实现命令的传递
fn main() {
    let args = StarterArgs::parse();
    if let Err(err) = simple_logger::init_with_level(log::Level::Info) {
        println!("could not init simple logger {err:?}");
    }
    let exefile = std::env::current_exe().unwrap();
    let binname = exefile.file_stem().unwrap().to_str().unwrap_or("starter");    
    let path = args
            .config
            .clone()
            .unwrap_or("assets/configs/Services.toml".into());
    let config: config::Config = load_config(path).expect("load config failed");
    let current_path = std::env::current_dir().unwrap();

    match args.command {
        StoreCommand::Start { name, group } => {
            println!("start -g {:?} -n {:?}", group, name);
            do_command(&config.clone(), "start", &group.unwrap_or_default(), &name.unwrap_or_default());
            return;
        },
        StoreCommand::Stop { name, group } => {
            do_command(&config.clone(), "stop", &group.unwrap_or_default(), &name.unwrap_or_default());
            return;
        },
        StoreCommand::Status => {
            do_command(&config.clone(), "status", "", "");
            return;
        },
        StoreCommand::Shutdown => {
            do_command(&config.clone(), "shutdown", "", "");
            return;
        },
        StoreCommand::Restart { name, group } => {
            do_command(&config.clone(), "restart", &group.unwrap_or_default(), &name.unwrap_or_default());
            return;
        },
        _ => {

        }
    };

    // let pid = std::process::id();
    let lockfile = current_path.join(format!("assets/{}.pid", binname));    
    match fslock::LockFile::open(&lockfile) {
        Ok(mut t) =>  {
            match t.try_lock() {
                Ok(locked) => {
                    if locked {
                        do_service(&config);
                        if let Err(err) = t.unlock() {
                            log::info!("unlock file error {err:?}");
                        }
                    } else {
                        log::error!("Could not start Starter Service twice.");
                    }
                },
                Err(err) => {
                    log::info!("error to lock file {err:?}");
                },
            }
        },
        Err(err) => {
            log::info!("error to open lock file {err:?}");
        },
    }
}
