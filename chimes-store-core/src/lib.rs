pub mod config;
pub mod dbs;
pub mod docs;
pub mod service;
pub mod utils;

pub use utils::executor::init_async_task_pool;


#[macro_export]
macro_rules! pin_blockon_async {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL
            .get()
            .unwrap()
            .execute_blockon($name)
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

