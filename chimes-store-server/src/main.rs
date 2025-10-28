use anyhow::anyhow;
use chimes_store_core::{
    service::{
        invoker::setup_generate_authorization_token_func, registry::SchemaRegistry, sched::{setup_convert_into_job_invoker, JobInvoker}
    },
    utils::{
        build_path, executor::init_global_runtime, num_of_cpus, GlobalConfig, GlobalSecurityConfig,
    },
};
use chimes_store_utils::{file::get_current_dir, template::json_path_get_string_with};
use clap::{CommandFactory, Parser};
use fastrace::collector::Config;
use flexi_logger::{trc::FormatConfig, writers::FileLogWriter, Age, Cleanup, Criterion, FileSpec, LogSpecification, Logger, Naming, WriteMode};
use salvo::http::request::set_global_secure_max_size;
use serde_json::Value;
use std::{
    fmt::Debug, fs::{self, File}, io::{BufReader, Read}, net::IpAddr, panic, path::{Path, PathBuf}, str::FromStr, sync::atomic::AtomicUsize
};
use substring::Substring;
use utils::{custom_performance_counter_add, AsyncRequest, PerformanceTaskCounter};
use uuid::Uuid;

use crate::auth_service::generate_jwt_authorization_token;
mod api;
mod auth_service;
mod config;
mod hoops;
mod manager;
mod plugin;
mod salvo_main;
mod utils;
mod websoket;


pub fn load_config<T>(path: impl AsRef<Path>) -> Result<T, anyhow::Error>
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
#[command(author, version, about)]
pub struct Args {
    #[arg(long, short, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(long, short)]
    ip: Option<IpAddr>,
    #[arg(long, short)]
    port: Option<u16>,

    #[clap(flatten)]
    features: clap_cargo::Features,
}

fn calc_filesize(filesize: Option<String>) -> u64 {
    filesize
        .map(|f| {
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

            println!("size: {unit}, {tk}");

            match tk.parse::<u64>() {
                Ok(m) => m * unit,
                Err(_) => 1024 * 1024,
            }
        })
        .unwrap_or(1024 * 1024)
}

fn storex_convert_value_job_invoker(
    val: Value,
) -> Result<(String, Box<dyn JobInvoker + Send + Sync>), anyhow::Error> {
    let mut job = match serde_json::from_value::<AsyncRequest>(val) {
        Ok(j) => j,
        Err(err) => {
            return Err(anyhow!("Error to convert to async request {err}"));
        }
    };

    let jobid = if let Some(uid) = job.uuid.clone() {
        if uid.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            uid.clone()
        }
    } else {
        Uuid::new_v4().to_string()
    };

    job.uuid = Some(jobid.clone());

    Ok((jobid, Box::new(job)))
}

fn config_tracing_reporter(config: &config::Config) {
    if let Some(reporter) = config.tracing_report.clone() {
        // Use the custom report
        match reporter.as_str() {
            "jaeger" => {
                let (agent_addr, service_name) = match config.report_config.clone() {
                    Some(val) => {
                        (json_path_get_string_with(&val, "$.agent_addr", "127.0.0.1:6831"), json_path_get_string_with(&val, "$.service_name", "growthstore"))
                    },
                    None => {
                        ("127.0.0.1:6831".to_owned(), "growthstore".to_owned())
                    }
                };
                if let Ok(r) =  fastrace_jaeger::JaegerReporter::new(agent_addr.parse().unwrap(), service_name) {
                    fastrace::set_reporter(r, Config::default());
                }
            },
            "datadog" => {
                let (agent_addr, service_name, resource, trace_type) = match config.report_config.clone() {
                    Some(val) => {
                        (json_path_get_string_with(&val, "$.agent_addr", "127.0.0.1:8126"), 
                            json_path_get_string_with(&val, "$.service_name", "growthstore"),
                            json_path_get_string_with(&val, "$.resource", "all"),
                            json_path_get_string_with(&val, "$.trace_type", "select"))
                    },
                    None => {
                        ("127.0.0.1:8126".to_owned(), "growthstore".to_owned(), "all".to_string(), "select".to_string())
                    }
                };

                let r =  fastrace_datadog::DatadogReporter::new(agent_addr.parse().unwrap(), service_name, resource, trace_type);
                fastrace::set_reporter(r, Config::default());
            },
            #[cfg(feature = "opentelemetry_reporter")]
            "opentelemetry"  => {
                use std::borrow::Cow;
                use opentelemetry_otlp::WithExportConfig;
                let (endpoint, service_name, resource, version) = match config.report_config.clone() {
                    Some(val) => {

                        (json_path_get_string_with(&val, "$.endpoint", "http://127.0.0.1:4317"), 
                            json_path_get_string_with(&val, "$.service_name", "growthstore"),
                            json_path_get_string_with(&val, "$.resource", "all"),
                            json_path_get_string_with(&val, "$.version", "1.1.0"))
                    },
                    None => {
                        ("http://127.0.0.1:4317".to_owned(), "growthstore".to_owned(), "all".to_string(), "1.1.0".to_string())
                    }
                };

                let r = fastrace_opentelemetry::OpenTelemetryReporter::new(
                        opentelemetry_otlp::SpanExporter::builder()
                                                .with_tonic()
                                                .with_endpoint(endpoint)
                                                .with_protocol(opentelemetry_otlp::Protocol::Grpc)
                                                .with_timeout(opentelemetry_otlp::OTEL_EXPORTER_OTLP_TIMEOUT_DEFAULT)
                                                .build()
                                                .expect("initialize oltp exporter"), 
                    Cow::Owned(
                                opentelemetry_sdk::Resource::builder()
                                .with_attributes([opentelemetry::KeyValue::new(
                                    "service.name",
                                    service_name,
                                )])
                                .build(),
                                ), 
        opentelemetry::InstrumentationScope::builder(resource)
                                .with_version(version)
                                .build());
                fastrace::set_reporter(r, Config::default());
            },
            _ => {
                fastrace::set_reporter(fastrace::collector::ConsoleReporter, fastrace::collector::Config::default());
            }
        }
    }
}

fn config_flexi_tracing(current_path: PathBuf, config: &config::Config) {
    let mut logger_builder = LogSpecification::builder();
    logger_builder.default(
        log::LevelFilter::from_str(&config.trace_level.clone().unwrap_or("Debug".to_string()))
            .unwrap_or(log::LevelFilter::Info),
    );
    // logger_builder.module("rbatis", log::LevelFilter::Info);

    // for logger in config.loggers.clone() {
    //     let level = log::LevelFilter::from_str(&logger.level.unwrap_or("Debug".to_string()))
    //         .unwrap_or(log::LevelFilter::Info);
    //     let module_name = logger.logger.clone();
    //     logger_builder.module(&module_name, level);
    // }

    let logpath = build_path(current_path.clone(), "logs").unwrap_or(current_path.clone());
    if let Err(err) = fs::create_dir_all(logpath.clone()) {
        println!("error to create logs directory {err:?}");
    }
    let file = config.log_file.clone().unwrap_or("storex.log".to_string());
    let logfile = build_path(logpath, file).unwrap_or(current_path.clone());
    let logdir = logfile.parent().unwrap();
    if let Err(err) = fs::create_dir_all(logdir) {
        println!("error to create logs directory {err:?}");
    }
    let logext = logfile
        .clone()
        .extension()
        .unwrap()
        .to_str()
        .map(|f| f.to_string())
        .unwrap_or("log".to_string());
    let logfilename = logfile
        .file_stem()
        .unwrap()
        .to_str()
        .map(|f| format!("{}-trace", f.to_string()))
        .unwrap();

    let filespec = FileSpec::default()
        .directory(logdir)
        .basename(logfilename)
        .suppress_timestamp()
        .suffix(logext);

    let keepfiles = config.log_keepfiles.unwrap_or(5u64) as usize;
    let wm = config
        .log_writemode
        .clone()
        .unwrap_or("direct".to_string())
        .to_lowercase();

    let write_mode: WriteMode = if wm == *"bufferandflush" {
        WriteMode::BufferAndFlush
    } else if wm == *"async" {
        WriteMode::Async
    } else {
        WriteMode::Direct
    };

    let flwb = FileLogWriter::builder(filespec)
                .rotate(
                    Criterion::Age(Age::Day),
                    Naming::Timestamps,
                    Cleanup::KeepLogFiles(keepfiles),
                )
                .write_mode(write_mode)
                .cleanup_in_background_thread(true);


    let fmtconf = FormatConfig::default()
                .with_ansi(false)
                .with_level(true)
                .with_thread_names(true)
                .with_thread_ids(true)
                .with_target(true)
                .with_line_number(true)
                .with_time(true);
            
    match flexi_logger::trc::setup_tracing(logger_builder.build(), None, flwb, &fmtconf) {
        Ok(_) => {
            eprintln!("tracing was setup successfully.");
        },
        Err(err) => {
            eprintln!("FlexiLoggerError: {err:?}");
        }
    }

}

fn config_flexi_logger(current_path: PathBuf, config: &config::Config) {
    let mut logger_builder = LogSpecification::builder();
    logger_builder.default(
        log::LevelFilter::from_str(&config.log_level.clone().unwrap_or("Debug".to_string()))
            .unwrap_or(log::LevelFilter::Info),
    );
    logger_builder.module("rbatis", log::LevelFilter::Info);
    logger_builder.module("store-server", log::LevelFilter::Info);

    let wm = config
        .log_writemode
        .clone()
        .unwrap_or("direct".to_string())
        .to_lowercase();

    let write_mode: WriteMode = if wm == *"bufferandflush" {
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
        if config.log_compress.unwrap_or_default() {
            flexi_logger::Cleanup::KeepCompressedFiles(keepfiles)
        } else {
            flexi_logger::Cleanup::KeepLogFiles(keepfiles)
        }
    } else {
        flexi_logger::Cleanup::KeepLogFiles(keepfiles)
    };

    let logger_ = Logger::with(logger_builder.build())
        .format(fmt)
        .adaptive_format_for_stdout(flexi_logger::AdaptiveFormat::WithThread)
        .cleanup_in_background_thread(true)
        .append()
        .print_message()
        .rotate(
            flexi_logger::Criterion::Size(rotation_size),
            flexi_logger::Naming::Timestamps,
            cleanup,
        )
        .write_mode(write_mode);
    let _ = match config.log_file.clone() {
        Some(file) => {
            let logpath = build_path(current_path.clone(), "logs").unwrap_or(current_path.clone());
            if let Err(err) = fs::create_dir_all(logpath.clone()) {
                println!("error to create logs directory {err:?}");
            }

            let logfile = build_path(logpath, file).unwrap_or(current_path.clone());
            let logdir = logfile.parent().unwrap();
            if let Err(err) = fs::create_dir_all(logdir) {
                println!("error to create logs directory {err:?}");
            }
            let logext = logfile
                .clone()
                .extension()
                .unwrap()
                .to_str()
                .map(|f| f.to_string())
                .unwrap_or("log".to_string());
            let logfilename = logfile
                .file_stem()
                .unwrap()
                .to_str()
                .map(|f| f.to_string())
                .unwrap();
            let filespec = FileSpec::default()
                .directory(logdir)
                .basename(logfilename)
                .suppress_timestamp()
                .suffix(logext);
            if config.log_console.unwrap_or_default() {
                logger_
                    .log_to_file(filespec.clone())
                    .duplicate_to_stdout(flexi_logger::Duplicate::All)
                    .start()
                    .unwrap()
            } else {
                logger_.log_to_file(filespec.clone()).start().unwrap()
            }
        }
        None => logger_.log_to_stdout().start().unwrap(),
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
    let current_path = get_current_dir().unwrap();
    config.web.model_path = build_path(current_path.clone(), config.web.model_path).unwrap();
    config.web.config_path = build_path(current_path.clone(), config.web.config_path).unwrap();

    config_flexi_logger(current_path.clone(), &config);

    if config.enable_tracing.unwrap_or_default() {
        config_flexi_tracing(current_path.clone(), &config);
        // tracing::subscriber::set_global_default(subscriber).unwrap();
    }    

    let cmd = Args::command();
    tracing::info!(
        "{} {} is starting...",
        cmd.get_name(),
        cmd.get_version().unwrap_or_default()
    );
    tracing::info!(
        "load work-threads: {}, pool_size: {}",
        config.web.work_threads,
        config.web.pool_size
    );

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
        rsa_password_public_key: config.rsa_public_key.clone(),
        rsa_password_private_key: config.rsa_private_key.clone(),
        aes_encryption_key: config.aes_encrytion_key.clone(),
        aes_encryption_solt: config.aes_encrytion_solt.clone(),
        work_threads: wt,
        pool_size: ps,
        logfile: config.log_file.clone(),
    });

    if let Some(consumer) = config.performance_consumer.clone() {
        SchemaRegistry::get_mut().set_performance_consumer(&consumer);
    }

    custom_performance_counter_add(9, wt as u64);
    custom_performance_counter_add(10, ps as u64);

    panic::set_hook(Box::new(|pc| {
        if let Some(loc) = pc.location() {
            println!(
                "At file {} line: {}:{}",
                loc.file(),
                loc.line(),
                loc.column()
            );
        }
        println!("system was exit on unhandle exception {pc:?}");
    }));

    setup_convert_into_job_invoker(storex_convert_value_job_invoker);
    setup_generate_authorization_token_func(generate_jwt_authorization_token);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(wt)
        .max_blocking_threads(ps)
        .enable_all()
        .thread_name_fn(|| {
            static ATOMIC_THREAD_ID: AtomicUsize = AtomicUsize::new(0);
            format!(
                "growth-worker-{}",
                ATOMIC_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            )
        })
        .build()
        .unwrap();

    let rt_ref = init_global_runtime(rt);
    rt_ref.block_on(async move {
        if config.enable_tracing.unwrap_or_default() {
            config_tracing_reporter(&config);
        }
        chimes_store_core::init_async_task_pool(Box::new(PerformanceTaskCounter()));
        SchemaRegistry::get_mut().start_performance_consumer();
        salvo_main::salvo_main(args, config).await
    })
}
