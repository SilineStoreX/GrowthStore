use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicU32, AtomicU64};
use substring::Substring;

pub struct LinuxPerformance {}

impl LinuxPerformance {

    #[allow(dead_code)]
    pub fn get_current_process_id() -> u32 {
        unsafe { libc::getpid() as u32 }
    }

    #[allow(dead_code)]
    pub fn get_cpu_cores() -> u32 {
        match File::open("/proc/cpuinfo") {
            Ok(mut fl) => {
                let mut text = String::new();
                match fl.read_to_string(&mut text) {
                    Ok(_) => {
                        let count = AtomicU32::new(0);
                        for line in text.lines() {
                            if line.starts_with("processor") {
                                count.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                            }
                        }
                        count.load(std::sync::atomic::Ordering::Acquire)
                    }
                    Err(err) => {
                        log::debug!("Read file error {}", err);
                        0u32
                    }
                }
            }
            Err(err) => {
                log::debug!("Open File error {}", err);
                0u32
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_thread_count(_dw_process_id: u32) -> u32 {
        let filename = format!("/proc/{}/stat", Self::get_current_process_id());
        match File::open(filename) {
            Ok(mut fl) => {
                let mut text = String::new();
                match fl.read_to_string(&mut text) {
                    Ok(_) => {
                        for line in text.lines() {
                            let items = line.split_whitespace().collect::<Vec<&str>>();
                            if items.len() > 20 {
                                let thc = items[19].parse::<u32>().unwrap_or(0u32);
                                if thc > 0 {
                                    return thc;
                                }
                            }
                        }
                        0u32
                    }
                    Err(err) => {
                        log::debug!("Read file error {}", err);
                        0u32
                    }
                }
            }
            Err(err) => {
                log::debug!("Open File error {}", err);
                0u32
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_process_times() -> (f64, f64, f64) {
        match File::open("/proc/stat") {
            Ok(mut fl) => {
                let mut text = String::new();
                match fl.read_to_string(&mut text) {
                    Ok(_) => {
                        // let count = AtomicU32::new(0);
                        for line in text.lines() {
                            if line.starts_with("cpu ") {
                                let items = line.split_whitespace().collect::<Vec<&str>>();
                                if items[0] == "cpu" {
                                    let total_user = match items[1].parse::<u64>() {
                                        Ok(t) => t as f64,
                                        Err(_) => 0f64,
                                    };
                                    let total_user_low = match items[2].parse::<u64>() {
                                        Ok(t) => t as f64,
                                        Err(_) => 0f64,
                                    };
                                    let total_sys = match items[3].parse::<u64>() {
                                        Ok(t) => t as f64,
                                        Err(_) => 0f64,
                                    };
                                    let total_idle = match items[4].parse::<u64>() {
                                        Ok(t) => t as f64,
                                        Err(_) => 0f64,
                                    };
                                    return (total_user + total_user_low, total_sys, total_idle);
                                }
                            }
                        }
                        (0f64, 0f64, 0f64)
                    }
                    Err(err) => {
                        log::debug!("Read file error {}", err);
                        (0f64, 0f64, 0f64)
                    }
                }
            }
            Err(err) => {
                log::debug!("Open File error {}", err);
                (0f64, 0f64, 0f64)
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_memory_usages() -> (u64, u64, u64) {
        let (vsize, vrss, _) = LinuxPerformance::get_program_memory();
        match nix::sys::sysinfo::sysinfo() {
            Ok(mem) => (mem.ram_total(), vrss, vsize),
            Err(err) => {
                log::debug!("Error {}", err);
                (0u64, 0u64, 0u64)
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_program_memory() -> (u64, u64, u64) {
        match File::open("/proc/self/status") {
            Ok(mut fl) => {
                let mut text = String::new();
                if let Ok(_) = fl.read_to_string(&mut text) {
                    // let count = AtomicU32::new(0);
                    let vsize = AtomicU64::new(0);
                    let vrss = AtomicU64::new(0);
                    let vdata = AtomicU64::new(0);

                    for line in text.lines() {
                        if line.starts_with("Vm") {
                            let cps: Vec<String> = line
                                .split(" ")
                                .filter(|p| !p.is_empty())
                                .map(|f| f.to_owned())
                                .collect();
                            if cps.len() >= 2 {
                                let name = cps[0].trim();
                                let size = cps[1].clone();
                                // let _unit = cps[2].clone();

                                if name == "VmSize:" {
                                    let s = size.parse::<u64>().unwrap_or_default();
                                    vsize.store(s * 1024, std::sync::atomic::Ordering::Release);
                                } else if name == "VmRSS:" {
                                    let s = size.parse::<u64>().unwrap_or_default();
                                    vrss.store(s * 1024, std::sync::atomic::Ordering::Release);
                                } else if name == "VmData:" {
                                    let s = size.parse::<u64>().unwrap_or_default();
                                    vdata.store(s * 1024, std::sync::atomic::Ordering::Release);
                                } else if name.starts_with("VmSize:\t") {
                                    let xnames: Vec<String> = name
                                        .split("\t")
                                        .filter(|p| !p.is_empty())
                                        .map(|f| f.to_owned())
                                        .collect();
                                    if xnames.len() >= 2 {
                                        let xsize = xnames[1].clone();
                                        let xs = xsize.parse::<u64>().unwrap_or_default();
                                        vsize
                                            .store(xs * 1024, std::sync::atomic::Ordering::Release);
                                    }
                                }
                            }
                        }
                    }
                    (
                        vsize.load(std::sync::atomic::Ordering::Acquire),
                        vrss.load(std::sync::atomic::Ordering::Acquire),
                        vdata.load(std::sync::atomic::Ordering::Acquire),
                    )
                } else {
                    (0u64, 0u64, 0u64)
                }
            }
            Err(err) => {
                log::debug!("Open File error {}", err);
                (0u64, 0u64, 0u64)
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_handle_count() -> u32 {
        let path = format!("/proc/{}/fdinfo", Self::get_current_process_id());
        match std::fs::read_dir(path) {
            Ok(rs) => {
                let count = AtomicU32::new(0u32);
                for fl in rs {
                    match fl {
                        Ok(_) => {
                            count.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                        }
                        Err(err) => {
                            log::debug!("Error {}", err);
                        }
                    }
                }
                count.load(std::sync::atomic::Ordering::Acquire)
            }
            Err(err) => {
                log::debug!("Error {}", err);
                0u32
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_io_counter() -> (u64, u64) {
        let path = format!("/proc/{}/io", Self::get_current_process_id());
        match File::open(&path) {
            Ok(mut fl) => {
                let mut text = String::new();
                match fl.read_to_string(&mut text) {
                    Ok(_) => {
                        let read_bytes = AtomicU64::new(0u64);
                        let write_bytes = AtomicU64::new(0u64);
                        for line in text.lines() {
                            if line.starts_with("read_bytes:") {
                                let numb_text = line.substring("read_bytes:".len() + 1, line.len());
                                if let Ok(t) = numb_text.parse::<u64>() {
                                    read_bytes.fetch_add(t, std::sync::atomic::Ordering::Acquire);
                                };
                            }
                            if line.starts_with("write_bytes:") {
                                let numb_text =
                                    line.substring("write_bytes:".len() + 1, line.len());
                                if let Ok(t) = numb_text.parse::<u64>() {
                                    write_bytes.fetch_add(t, std::sync::atomic::Ordering::Acquire);
                                };
                            }
                        }
                        (
                            read_bytes.load(std::sync::atomic::Ordering::Acquire),
                            write_bytes.load(std::sync::atomic::Ordering::Acquire),
                        )
                    }
                    Err(err) => {
                        log::debug!("Read file error {}", err);
                        (0u64, 0u64)
                    }
                }
            }
            Err(err) => {
                log::debug!("Open File error {}", err);
                (0u64, 0u64)
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_network_io_counter() -> (u64, u64) {
        let path = format!("/proc/{}/net/dev", Self::get_current_process_id());
        match File::open(&path) {
            Ok(mut fl) => {
                let mut text = String::new();
                match fl.read_to_string(&mut text) {
                    Ok(_) => {
                        let read_bytes = AtomicU64::new(0u64);
                        let write_bytes = AtomicU64::new(0u64);
                        for line in text.lines() {
                            let items = line.split_whitespace().collect::<Vec<&str>>();
                            if items.len() > 10 {
                                match items[1].parse::<u64>() {
                                    Ok(t) => {
                                        read_bytes
                                            .fetch_add(t, std::sync::atomic::Ordering::Acquire);
                                    }
                                    Err(_) => {
                                        continue;
                                    }
                                };
                                match items[9].parse::<u64>() {
                                    Ok(t) => {
                                        write_bytes
                                            .fetch_add(t, std::sync::atomic::Ordering::Acquire);
                                    }
                                    Err(_) => {
                                        continue;
                                    }
                                }
                            }
                        }
                        (
                            read_bytes.load(std::sync::atomic::Ordering::Acquire),
                            write_bytes.load(std::sync::atomic::Ordering::Acquire),
                        )
                    }
                    Err(err) => {
                        log::debug!("Read file error {}", err);
                        (0u64, 0u64)
                    }
                }
            }
            Err(err) => {
                log::debug!("Open File error {}", err);
                (0u64, 0u64)
            }
        }
    }
}
