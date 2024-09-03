use std::{process::{self, Stdio}, str::FromStr};

use anyhow::anyhow;
use jsonpath_rust::JsonPathInst;
use reqwest::Client;
use serde_json::Value;

use crate::config::{Config, ProcessDeamon};

async fn do_request(url: &str, jsonpath: Option<String>) -> Result<bool, anyhow::Error> {
    let builder = Client::builder();
    let client = builder.build()?;
    let resp = client.get(url).send().await?;
    let ret = resp.json::<Value>().await?;
    if let Some(jsonpath) = jsonpath {
        if let Ok(jsoninst) = JsonPathInst::from_str(&jsonpath) {
            let veclist = jsoninst.find_slice(&ret);
            for vcl in veclist.iter() {
                log::info!("json check result: {:?}", vcl.to_string());
            }
            return Ok(!veclist.is_empty()) 
        } else {
            log::error!("could not parse the jsonpath {jsonpath}. ignored.");
        }
    }
    Ok(true)
}

pub async fn do_process(proc: &ProcessDeamon) -> Result<(), anyhow::Error> {
    if let Some(health_url) = proc.health_url.clone() {
        log::info!("health_url: {health_url}");
        match do_request(&health_url, proc.json_cheker.clone()).await {
            Ok(t) => {
                if !t {
                    proc.failures.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                    proc.failures_total.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                } else {
                    proc.failures.store(0, std::sync::atomic::Ordering::Release);
                }
            },
            Err(err) => {
                log::error!("error {err:?}");
                proc.failures.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                proc.failures_total.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            }
         }
    }
    Ok(())
}

pub fn execute_start(_conf: &Config, proc: &ProcessDeamon) -> Result<String, anyhow::Error> {
    let cs = proc.start_command.clone().unwrap();
    let lines = cs
        .lines()
        .map(|f| f.to_owned())
        .collect::<Vec<String>>()
        .join(" && ");

    let currdir = proc.current_dir.clone().map(|f| f.into()).unwrap_or(std::env::current_dir().unwrap());

    #[cfg(windows)]
    let res = process::Command::new("cmd")
        .current_dir(&currdir)
        .arg("/c")
        .arg(lines)
        .stdout(Stdio::null())
        .spawn();

    #[cfg(not(windows))]
    let res = process::Command::new("bash")
        .current_dir(&currdir)    
        .arg("-c")
        .arg(lines)
        .stdout(Stdio::null())
        .spawn();

    match res {
        Ok(child) => {
            let id = child.id();
            proc.process_id.store(id as i64, std::sync::atomic::Ordering::Release);
            proc.manual_stop.store(0, std::sync::atomic::Ordering::Release);
            proc.failures.store(0u32, std::sync::atomic::Ordering::Release);
            Ok(format!("{id}"))
        },
        Err(err) => {
            log::warn!("Could not execute the shell script {:?}", err);
            Err(anyhow!(err))
        }
    }
}


pub fn execute_stop(conf: &Config, proc: &ProcessDeamon) -> Result<String, anyhow::Error> {
    let cs = proc.stop_command.clone().unwrap();
    let pid = proc.process_id.load(std::sync::atomic::Ordering::Acquire);
    if pid == 0 {
        return Ok("".to_owned());
    }
    let lines = cs
        .replace("${pid}", &format!("{pid}"))
        .lines()
        .map(|f| f.to_owned())
        .collect::<Vec<String>>()
        .join(" && ");

    log::warn!("Stop Command: {lines}");

    let currdir = proc.current_dir.clone().map(|f| f.into()).unwrap_or(std::env::current_dir().unwrap());

    #[cfg(windows)]
    let res = process::Command::new("cmd")
        .current_dir(&currdir)
        .arg("/c")
        .arg(lines)
        .stdout(Stdio::piped())
        .spawn();

    #[cfg(not(windows))]
    let res = process::Command::new("bash")
        .current_dir(&currdir)    
        .arg("-c")
        .arg(lines)
        .stdout(Stdio::piped())
        .spawn();

    match res {
        Ok(child) => { 
            match child.wait_with_output() {
                Ok(output) => {
                    proc.process_id.store(0, std::sync::atomic::Ordering::Release);
                    let codepage = conf.codepage.clone().unwrap_or("utf-8".to_owned());
                    let (text, _enc, _repl) =
                        match encoding_rs::Encoding::for_label(codepage.as_bytes()) {
                            Some(enc) => enc.decode(&output.stdout),
                            None => encoding_rs::UTF_8.decode(&output.stdout),
                        };
                    log::debug!("{}", text.to_string());
                    Ok(text.to_string())
                }
                Err(err) => {
                    log::warn!("Could not wait to execute the shell script {:?}", err);
                    Err(anyhow!(err))
                }
            }
        },
        Err(err) => {
            log::warn!("Could not execute the shell script {:?}", err);
            Err(anyhow!(err))
        }
    }
}
