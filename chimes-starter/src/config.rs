use std::
    sync::{atomic::{AtomicI64, AtomicU16, AtomicU32}, Arc}
;

use serde::{Deserialize, Serialize};

use crate::proc::{execute_start, execute_stop};


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessDeamon {
    pub group: Option<String>,          // 进程的分组
    pub name: Option<String>,           // 进程的名字
    pub current_dir: Option<String>,    // 执行时的当前路径
    pub start_command: Option<String>,  // 启动的命令
    pub stop_command: Option<String>,   // 停止的命令
    pub ipc_checker: Option<String>,    // 使用ipc_checker方法
    pub health_url: Option<String>,     // 使用health_checker的方法，GET <heath_url> 方法来检查对方进程是否有
    pub json_cheker: Option<String>,    // Use JsonPath to check success
    pub fail_count: Option<i64>,        // 通过health_checker的检查，连续几次fail表示该应该没有启动
    pub fail_start: bool,               // 被判断失败后，进行进程的重启
    
    #[serde(skip_serializing)]
    pub failures:           Arc<AtomicU32>,     // 失败次数
    #[serde(skip_serializing)]
    pub failures_total:     Arc<AtomicU32>,     // 失败总次数
    #[serde(skip_serializing)]
    pub manual_stop:        Arc<AtomicU16>,     // 停止状态     
    #[serde(skip_serializing)]
    pub process_id:         Arc<AtomicI64>,     // process_id
}



#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProcessState {
    pub group: Option<String>,
    pub name: Option<String>,
    pub process_id: u64,
    pub failures: u64,
    pub running: bool,
}

impl ProcessDeamon {
    pub fn should_start(&self) -> bool {
        let failures = self.failures.load(std::sync::atomic::Ordering::Acquire);
        self.fail_start && !self.is_manualstop() && failures > self.fail_count.unwrap_or(6) as u32
    }

    pub fn is_manualstop(&self) -> bool {
        self.manual_stop.load(std::sync::atomic::Ordering::Acquire) > 0
    }

    pub fn is_unstarted(&self) -> bool {
        self.process_id.load(std::sync::atomic::Ordering::Acquire) == 0
    } 

    pub fn restart(&self, conf: &Config) -> Result<bool, anyhow::Error> {
        execute_stop(conf, self)?;
        execute_start(conf, self)?;
        Ok(true)
    }

    pub fn stop(&self, conf: &Config) -> Result<bool, anyhow::Error> {
        execute_stop(conf, self)?;
        self.manual_stop.store(1, std::sync::atomic::Ordering::Release);
        Ok(true)
    }

    pub fn state(&self, _conf: &Config) -> ProcessState {
        let pid = self.process_id.load(std::sync::atomic::Ordering::Acquire) as u64;
        ProcessState {
            group: self.group.clone(),
            name: self.name.clone(),
            process_id: pid,
            failures: self.failures_total.load(std::sync::atomic::Ordering::Acquire) as u64,
            running: pid > 0 && self.manual_stop.load(std::sync::atomic::Ordering::Acquire) == 0,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub lockfile: Option<String>,           // 使用lockfile防止应用多开
    pub shared_file: Option<String>,        // 使用shared_file来让多个starter进行通信，实现starter -stop命令
    pub process: Vec<ProcessDeamon>,        // 被管理的进程
    pub codepage: Option<String>,           // Stdio的CodePage
    pub interval: Option<i64>,              // 间隔的秒数
}
