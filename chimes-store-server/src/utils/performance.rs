use chimes_store_core::utils::executor::TaskCounter;
use chimes_store_core::utils::get_local_timestamp;
use lazy_static::lazy_static;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::sync::{atomic::AtomicU64, Mutex};

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ChimesPerformanceInfo {
    pub timestamp: u64,          // Timestamp lable
    pub cpu_cores: u32,          // number of cpu cores
    pub kernel_cpu_usages: f64,  // cpu usage percent
    pub user_cpu_usages: f64,    // cpu usage percent
    pub idle_cpu_usages: f64,    // cpu idle percent
    pub now_cpu_time: f64,       // Current time
    pub memory_used: u64,        // memory usage MB
    pub memory_total: u64,       // memory usage MB
    pub disk_read_total: u64,    // disk io speed for read
    pub disk_write_total: u64,   // disk io speed for write
    pub network_recv_total: u64, // network io speed for recv
    pub network_send_total: u64, // network io speed for send
    pub threads: u64,            // threads
    pub handlers: u64,           // handlers
    pub success: bool,           // success or not
    pub counter: CustomCounterInfo,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct CustomCounterInfo {
    pub task_1_count: u64,
    pub task_2_count: u64,
    pub task_3_count: u64,
    pub task_4_count: u64,
    pub task_5_count: u64,
    pub task_6_count: u64,
    pub task_7_count: u64,
    pub task_8_count: u64,
    pub task_9_count: u64,
    pub task_10_count: u64,
}

#[derive(Default)]
pub struct CustomCounter {
    pub task_1_count: AtomicU64,
    pub task_2_count: AtomicU64,
    pub task_3_count: AtomicU64,
    pub task_4_count: AtomicU64,
    pub task_5_count: AtomicU64,
    pub task_6_count: AtomicU64,
    pub task_7_count: AtomicU64,
    pub task_8_count: AtomicU64,
    pub task_9_count: AtomicU64,
    pub task_10_count: AtomicU64,
}

impl CustomCounter {
    pub fn to_counter(&self) -> CustomCounterInfo {
        CustomCounterInfo {
            task_1_count: self.task_1_count.load(std::sync::atomic::Ordering::Acquire),
            task_2_count: self.task_2_count.load(std::sync::atomic::Ordering::Acquire),
            task_3_count: self.task_3_count.load(std::sync::atomic::Ordering::Acquire),
            task_4_count: self.task_4_count.load(std::sync::atomic::Ordering::Acquire),
            task_5_count: self.task_5_count.load(std::sync::atomic::Ordering::Acquire),
            task_6_count: self.task_6_count.load(std::sync::atomic::Ordering::Acquire),
            task_7_count: self.task_7_count.load(std::sync::atomic::Ordering::Acquire),
            task_8_count: self.task_8_count.load(std::sync::atomic::Ordering::Acquire),
            task_9_count: self.task_9_count.load(std::sync::atomic::Ordering::Acquire),
            task_10_count: self
                .task_10_count
                .load(std::sync::atomic::Ordering::Acquire),
        }
    }

    #[allow(dead_code)]
    pub fn get_task_count(&self, t: u32) -> u64 {
        match t {
            1 => self.task_1_count.load(std::sync::atomic::Ordering::Acquire),
            2 => self.task_2_count.load(std::sync::atomic::Ordering::Acquire),
            3 => self.task_3_count.load(std::sync::atomic::Ordering::Acquire),
            4 => self.task_4_count.load(std::sync::atomic::Ordering::Acquire),
            5 => self.task_5_count.load(std::sync::atomic::Ordering::Acquire),
            6 => self.task_6_count.load(std::sync::atomic::Ordering::Acquire),
            7 => self.task_7_count.load(std::sync::atomic::Ordering::Acquire),
            8 => self.task_8_count.load(std::sync::atomic::Ordering::Acquire),
            9 => self.task_9_count.load(std::sync::atomic::Ordering::Acquire),
            10 => self
                .task_10_count
                .load(std::sync::atomic::Ordering::Acquire),
            _ => 0u64,
        }
    }
}

lazy_static! {
    pub static ref CUSTOM_PERFORMANCE_COUNTER: Mutex<RefCell<CustomCounter>> =
        Mutex::new(RefCell::new(CustomCounter::default()));
}

#[allow(dead_code)]
pub fn get_custom_performance_counter() -> &'static CustomCounter {
    unsafe { &*CUSTOM_PERFORMANCE_COUNTER.lock().unwrap().as_ptr() }
}

#[allow(dead_code)]
pub fn custom_performance_counter_increase(it: i32) {
    match it {
        1 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_1_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        2 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_2_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        3 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_3_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        4 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_4_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        5 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_5_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        6 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_6_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        7 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_7_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        8 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_8_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        9 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_9_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        10 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_10_count
            .fetch_add(1u64, std::sync::atomic::Ordering::Release),
        _ => 0u64,
    };
}

#[allow(dead_code)]
pub fn custom_performance_counter_add(it: i32, val: u64) {
    match it {
        1 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_1_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        2 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_2_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        3 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_3_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        4 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_4_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        5 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_5_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        6 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_6_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        7 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_7_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        8 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_8_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        9 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_9_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        10 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_10_count
            .fetch_add(val, std::sync::atomic::Ordering::Release),
        _ => 0u64,
    };
}

#[allow(dead_code)]
pub fn custom_performance_counter_reset(it: i32) {
    match it {
        1 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_1_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        2 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_2_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        3 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_3_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        4 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_4_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        5 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_5_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        6 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_6_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        7 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_7_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        8 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_8_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        9 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_9_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        10 => CUSTOM_PERFORMANCE_COUNTER
            .lock()
            .unwrap()
            .borrow_mut()
            .task_10_count
            .store(0u64, std::sync::atomic::Ordering::Release),
        _ => {}
    };
}

impl ChimesPerformanceInfo {
    #[cfg(target_os = "windows")]
    pub fn get_performance_info() -> Result<Self, anyhow::Error> {
        use super::windows_performance::WindowsPerformance;

        let hc = WindowsPerformance::get_handle_count();
        let tc = WindowsPerformance::get_thread_count(WindowsPerformance::get_current_process_id());
        let (kcpu, ucpu, now) = WindowsPerformance::get_process_times();
        let (ttl, mem) = WindowsPerformance::get_memory_usages();
        let (dr, dw) = WindowsPerformance::get_io_counter();
        let cores = WindowsPerformance::get_cpu_cores();
        let (inrc, wrc) = WindowsPerformance::get_network_io_counter();
        let now_time = get_local_timestamp();

        let newitem = Self {
            timestamp: now_time,
            kernel_cpu_usages: kcpu,
            user_cpu_usages: ucpu,
            cpu_cores: cores,
            idle_cpu_usages: 0f64,
            now_cpu_time: now,
            memory_used: mem,
            memory_total: ttl,
            disk_read_total: dr,
            disk_write_total: dw,
            network_recv_total: inrc,
            network_send_total: wrc,
            threads: tc as u64,
            handlers: hc as u64,
            success: true,
            counter: get_custom_performance_counter().to_counter(),
        };

        Ok(newitem)
    }

    #[cfg(not(target_os = "windows"))]
    pub fn get_performance_info() -> Result<Self, anyhow::Error> {
        use super::linux_performance::LinuxPerformance;

        let hc = LinuxPerformance::get_handle_count();
        let tc = LinuxPerformance::get_thread_count(LinuxPerformance::get_current_process_id());
        let (kcpu, ucpu, idle) = LinuxPerformance::get_process_times();
        let (ttl, mem) = LinuxPerformance::get_memory_usages();
        let (dr, dw) = LinuxPerformance::get_io_counter();
        let cores = LinuxPerformance::get_cpu_cores();
        let (inrc, wrc) = LinuxPerformance::get_network_io_counter();
        let now_time = get_local_timestamp();

        let newitem = Self {
            timestamp: now_time,
            kernel_cpu_usages: kcpu,
            user_cpu_usages: ucpu,
            idle_cpu_usages: idle,
            cpu_cores: cores,
            now_cpu_time: now_time as f64,
            memory_used: mem as u64,
            memory_total: ttl as u64,
            disk_read_total: dr,
            disk_write_total: dw,
            network_recv_total: inrc,
            network_send_total: wrc,
            threads: tc as u64,
            handlers: hc as u64,
            success: true,
            counter: get_custom_performance_counter().to_counter(),
        };
        Ok(newitem)
    }
}

pub struct PerformanceTaskCounter();

unsafe impl Send for PerformanceTaskCounter {}

unsafe impl Sync for PerformanceTaskCounter {}

impl TaskCounter for PerformanceTaskCounter {
    fn increase_completed(&self) {
        custom_performance_counter_increase(1);
    }

    fn increase_task(&self) {
        custom_performance_counter_increase(2);
    }

    fn increase_error(&self) {
        custom_performance_counter_increase(3);
    }

    fn increase_longlive(&self) {
        custom_performance_counter_increase(4);
    }

    fn increase_exitlive(&self) {
        custom_performance_counter_increase(5);
    }    
}
