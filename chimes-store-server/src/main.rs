use anyhow::Result;
use chimes_store_core::utils::{build_path, num_of_cpus, GlobalConfig, GlobalSecurityConfig};
use clap::Parser;
use flexi_logger::{FileSpec, LogSpecification, Logger, WriteMode};
use salvo::http::request::set_global_secure_max_size;
use std::{
    fs::{self, File},
    io::{BufReader, Read},
    net::IpAddr,
    path::{Path, PathBuf}, str::FromStr,
};
use substring::Substring;
mod api;
mod auth_service;
mod config;
mod manager;
mod plugin;
mod salvo_main;
mod utils;

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

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long, short, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(long, short)]
    ip: Option<IpAddr>,
    #[arg(long, short)]
    port: Option<u16>,
}

fn calc_filesize(filesize: Option<String>) -> u64 {
    filesize.map(|f| {
        let cs = f.to_lowercase();
        
        let (unit, tk) = if cs.ends_with("kb") {
            (1024u64, cs.substring(0, cs.len() - 2))
        } else if cs.ends_with("mb") {
            (1024u64 * 1024u64, cs.substring(0, cs.len() - 2))
        } else if cs.ends_with("gb") {
            (1024u64 * 1024u64 * 1024u64, cs.substring(0, cs.len() - 2))
        } else {
            (1u64, cs.as_str())
        };

        println!("size: {}, {}", unit, tk);

        match tk.parse::<u64>() {
            Ok(m) => m * unit,
            Err(_) => 1024 * 1024,
        }
    }).unwrap_or(1024 * 1024)
}

fn config_flexi_logger(current_path: PathBuf, config: &config::Config) {
    let mut logger_builder = LogSpecification::builder();
    logger_builder.default(log::LevelFilter::from_str(&config.log_level.clone().unwrap_or("Debug".to_string())).unwrap_or(log::LevelFilter::Info));
    logger_builder.module("rbatis", log::LevelFilter::Info);
    logger_builder.module("store-server", log::LevelFilter::Info);

    let wm = config.log_writemode.clone().unwrap_or("direct".to_string()).to_lowercase();

    let write_mode = if wm == *"bufferandflush" {
        WriteMode::BufferAndFlush
    } else if wm == *"async" {
        WriteMode::Async
    } else {
        WriteMode::Direct
    };

    for l in config.loggers.clone() {
        if let Some(level) = l.level {
            if let Ok(lv) = log::LevelFilter::from_str(&level) {
                logger_builder.module(&l.logger, lv);
            } else {
                logger_builder.module(&l.logger, log::LevelFilter::Info);
            }
        } else {
            logger_builder.module(&l.logger, log::LevelFilter::Info);
        }
    }

    for pl in config.plugins.clone() {
        // If the plugin redefined the logger level, we will replace it
        let plugin_path: PathBuf = pl.plugin_dylib.into();
        if let Some(filename) = plugin_path.file_stem() {
            let dylib_name = filename.to_string_lossy().to_string();
            if let Some(level) = pl.logger {
                if let Ok(lv) = log::LevelFilter::from_str(&level) {
                    logger_builder.module(&dylib_name, lv);
                }
            }
        }
    }

    let rotation_size = calc_filesize(config.log_rotation.clone());
    let keepfiles = config.log_keepfiles.unwrap_or(5u64) as usize;

    let fmt = if config.log_json.unwrap_or_default() {
        flexi_logger::json_format
    } else {
        flexi_logger::with_thread
    };

    let cleanup = if cfg!(target_os = "linux") {
        flexi_logger::Cleanup::KeepCompressedFiles(keepfiles)
    } else {
        flexi_logger::Cleanup::KeepLogFiles(keepfiles)
    };

    let logger_ = Logger::with(logger_builder.build()).format(fmt)
                        .adaptive_format_for_stdout(flexi_logger::AdaptiveFormat::WithThread)
                        .cleanup_in_background_thread(true)
                        .append()
                        .print_message()
                        .rotate(flexi_logger::Criterion::Size(rotation_size), flexi_logger::Naming::Timestamps, cleanup)
                        .write_mode(write_mode);
    let _ = match config.log_file.clone() {
        Some(file) => {
            let logpath = build_path(current_path.clone(), "logs").unwrap_or(current_path.clone());
            if let Err(err) = fs::create_dir_all(logpath.clone()) {
                println!("error to create logs directory {:?}", err);
            }

            let logfile = build_path(logpath, file).unwrap_or(current_path.clone());
            let logdir = logfile.parent().unwrap();
            if let Err(err) = fs::create_dir_all(logdir) {
                println!("error to create logs directory {:?}", err);
            }
            let logext = logfile.clone().extension().unwrap().to_str().map(|f| f.to_string()).unwrap_or("log".to_string());
            let logfilename = logfile.file_stem().unwrap().to_str().map(|f| f.to_string()).unwrap();
            let filespec = FileSpec::default().directory(logdir).basename(logfilename).suppress_timestamp().suffix(logext);
            if config.log_console.unwrap_or_default() {
                logger_.log_to_file(filespec.clone()).duplicate_to_stdout(flexi_logger::Duplicate::All).start().unwrap()
            } else {
                logger_.log_to_file(filespec.clone()).start().unwrap()
            }
        },
        None => {
            logger_.log_to_stdout().start().unwrap()
        }
    };
}

// work_threads 暂时设置为 30个，这样可以满足正常的需求，
// 以及Pool的需求
//#[tokio::main(flavor = "multi_thread", worker_threads = 30)]
fn main() {
    let args = Args::parse();

    let path = args
            .config
            .clone()
            .unwrap_or("assets/configs/Config.toml".into());
    let mut config: config::Config = load_config(path).expect("load config failed");
    let current_path = std::env::current_dir().unwrap();
    config.web.model_path = build_path(current_path.clone(), config.web.model_path).unwrap();
    config.web.config_path = build_path(current_path.clone(), config.web.config_path).unwrap();

    
    config_flexi_logger(current_path.clone(), &config);
    
    let wt = if config.web.work_threads == 0 {
        2 * num_of_cpus()
    } else {
        config.web.work_threads as usize
    };

    let ps = if config.web.pool_size == 0 {
        num_of_cpus()
    } else {
        config.web.pool_size as usize
    };

    set_global_secure_max_size(10 * 1024 * 1024);

    GlobalConfig::update(&GlobalSecurityConfig {
        console_code_page: Some(config.web.code_page.clone()),
        rsa_password_public_key: None,
        rsa_password_private_key: None,
        work_threads: wt,
        pool_size: ps,
    });

    tokio::runtime::Builder::new_multi_thread()
            .worker_threads(wt)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
        salvo_main::salvo_main(args, config).await
    })
}
