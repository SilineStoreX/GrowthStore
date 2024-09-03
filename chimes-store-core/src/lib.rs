pub mod config;
pub mod dbs;
pub mod docs;
pub mod service;
pub mod utils;


#[macro_export]
macro_rules! pin_blockon_async {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL.execute_async($name)
    };
}

#[macro_export]
macro_rules! pin_submit {
    // 模式匹配
    ($name:expr) => {
        // 宏的展开
        $crate::utils::executor::CHIMES_THREAD_POOL.submit($name);
    };
}
