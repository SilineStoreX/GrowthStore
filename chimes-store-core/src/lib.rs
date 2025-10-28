pub mod config;
pub mod dbs;
pub mod docs;
pub mod service;
pub mod utils;

use std::cell::UnsafeCell;

pub use crate::utils::executor::init_async_task_pool;

#[repr(transparent)]
pub struct SyncUnsafeCell<T>(UnsafeCell<T>);
unsafe impl<T: Sync> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub fn new(val: T) -> Self {
        SyncUnsafeCell(UnsafeCell::new(val))
    }

    pub fn get(&self) -> *mut T {
        self.0.get()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }
}

#[macro_export]
macro_rules! pin_blockon_async {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL_BLOCK
            .get()
            .unwrap()
            .execute_blockon($name)
    };
}

#[macro_export]
macro_rules! pin_blockon_async_v2 {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::independ_block_on($name)
        // $crate::utils::executor::CHIMES_THREAD_POOL_BLOCK
        //     .get()
        //     .unwrap()
        //     .execute_blockon($name)
    };
}

#[macro_export]
macro_rules! futures_blockon_async {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::futures_exeuctors::futures_executre_block_on($name)
    };
}

#[macro_export]
macro_rules! pin_submit_block {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL_BLOCK
            .get()
            .unwrap()
            .submit($name);
    };
}

#[macro_export]
macro_rules! pin_submit {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL
            .get()
            .unwrap()
            .submit_async($name);
    };
}

#[macro_export]
macro_rules! pin_async_process {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL
            .get()
            .unwrap()
            .async_process($name);
    };
}

#[macro_export]
macro_rules! pin_blockon_process {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL_BLOCK
            .get()
            .unwrap()
            .blockon_process($name);
    };
}

#[macro_export]
macro_rules! pin_spawnthread {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL
            .get()
            .unwrap()
            .spawn_task($name);
    };
}

#[macro_export]
macro_rules! tasklog_info {
    // 模式匹配
    ($taskid:expr, $cookie:expr, $subject:expr, $desc:expr, $success:expr) => {{
        let thread_id = std::thread::current().id();
        let thread_name = std::thread::current()
            .name()
            .map(|f| f.to_owned())
            .unwrap_or(format!("{thread_id:?}"));
        $crate::service::queue::TaskLogger::write_log(
            $taskid,
            &thread_name,
            &format!("{}", $cookie),
            "INFO",
            $subject,
            $desc,
            $success,
        )
        .await;
    }};
}

#[macro_export]
macro_rules! tasklog_error {
    // 模式匹配
    ($taskid:expr, $cookie:expr, $subject:expr, $desc:expr) => {{
        let thread_id = std::thread::current().id();
        let thread_name = std::thread::current()
            .name()
            .map(|f| f.to_owned())
            .unwrap_or(format!("{thread_id:?}"));
        $crate::service::queue::TaskLogger::write_log(
            $taskid,
            &thread_name,
            &format!("{}", $cookie),
            "ERROR",
            $subject,
            $desc,
            false,
        )
        .await;
    }};
}

#[macro_export]
macro_rules! tasklog_success {
    // 模式匹配
    ($taskid:expr, $cookie:expr, $subject:expr, $desc:expr) => {{
        let thread_id = std::thread::current().id();
        let thread_name = std::thread::current()
            .name()
            .map(|f| f.to_owned())
            .unwrap_or(format!("{thread_id:?}"));
        $crate::service::queue::TaskLogger::write_log(
            $taskid,
            &thread_name,
            &format!("{}", $cookie),
            "SUCCESS",
            $subject,
            $desc,
            true,
        )
        .await;
    }};
}

#[macro_export]
macro_rules! tasklog_warn {
    // 模式匹配
    ($taskid:expr, $cookie:expr, $subject:expr, $desc:expr, $succ:expr) => {{
        let thread_id = std::thread::current().id();
        let thread_name = std::thread::current()
            .name()
            .map(|f| f.to_owned())
            .unwrap_or(format!("{thread_id:?}"));
        $crate::service::queue::TaskLogger::write_log(
            $taskid,
            &thread_name,
            &format!("{}", $cookie),
            "WARN",
            $subject,
            $desc,
            $succ,
        )
        .await;
    }};
}
