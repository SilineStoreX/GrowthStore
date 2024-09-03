use std::{ffi::c_void, mem::size_of};
use windows::Win32::{
    Foundation::CloseHandle,
    NetworkManagement::IpHelper::MIB_TCPROW_LH_0,
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, THREADENTRY32,
    },
};
use windows::{
    imp::{GetProcessHeap, HeapAlloc, HeapFree},
    Win32::{
        Foundation::{
            BOOL, BOOLEAN, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, FILETIME, NO_ERROR,
            WIN32_ERROR,
        },
        NetworkManagement::IpHelper::{
            GetExtendedTcpTable, GetPerTcp6ConnectionEStats, GetPerTcpConnectionEStats,
            SetPerTcp6ConnectionEStats, SetPerTcpConnectionEStats, TCP_ESTATS_BANDWIDTH_ROD_v0,
            TCP_ESTATS_BANDWIDTH_RW_v0, TCP_ESTATS_DATA_ROD_v0, TCP_ESTATS_DATA_RW_v0,
            TcpBoolOptEnabled, TcpConnectionEstatsBandwidth, TcpConnectionEstatsData, MIB_TCP6ROW,
            MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID, MIB_TCPROW_LH, MIB_TCPROW_OWNER_PID,
            MIB_TCPTABLE_OWNER_PID, MIB_TCP_STATE, MIB_TCP_STATE_ESTAB, MIB_TCP_STATE_LISTEN,
            MIB_TCP_STATE_SYN_RCVD, MIB_TCP_STATE_SYN_SENT, TCP_TABLE_OWNER_PID_ALL,
        },
        Networking::WinSock::{InetNtopW, AF_INET, AF_INET6, IN_ADDR_0},
        System::{
            Diagnostics::ToolHelp::TH32CS_SNAPTHREAD,
            ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            SystemInformation::{
                GetSystemInfo, GetSystemTimeAsFileTime, GlobalMemoryStatus, MEMORYSTATUS,
                SYSTEM_INFO,
            },
            Threading::{
                GetCurrentProcess, GetCurrentProcessId, GetProcessHandleCount,
                GetProcessIoCounters, GetProcessTimes, IO_COUNTERS,
            },
        },
    },
};

#[cfg(target_os = "windows")]
pub struct WindowsPerformance {}

#[cfg(target_os = "windows")]
impl WindowsPerformance {
    pub fn get_current_process_id() -> u32 {
        unsafe { GetCurrentProcessId() }
    }

    pub fn get_cpu_cores() -> u32 {
        unsafe {
            let mut st = SYSTEM_INFO::default();
            let st_ptr = std::ptr::addr_of_mut!(st) as *mut SYSTEM_INFO;
            GetSystemInfo(st_ptr);
            st.dwNumberOfProcessors
        }
    }

    pub fn get_thread_count(dw_process_id: u32) -> u32 {
        unsafe {
            match CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, dw_process_id) {
                Ok(h_thread_snap) => {
                    let mut th32 = THREADENTRY32::default();
                    let mut count = 0;
                    // th32.dwSize = sizeof(THREADENTRY32);
                    th32.dwSize = size_of::<THREADENTRY32>() as u32;
                    if !Thread32First(h_thread_snap, &mut th32).as_bool() {
                        log::info!("Could not iterate the Thread.");
                        CloseHandle(h_thread_snap);
                        count = 0;
                    } else {
                        if th32.th32OwnerProcessID == dw_process_id {
                            count += 1;
                        }
                        while Thread32Next(h_thread_snap, &mut th32).as_bool() {
                            if th32.th32OwnerProcessID == dw_process_id {
                                count += 1;
                            }
                        }
                        CloseHandle(h_thread_snap);
                    }
                    count
                }
                Err(err) => {
                    log::info!("Error to Get Thread Count of current process: {}", err);
                    0
                }
            }
        }
    }

    pub fn get_process_times() -> (f64, f64, f64) {
        unsafe {
            let hprocess = GetCurrentProcess();
            //if hprocess.is_invalid() {
            //  log::info!("Could not get current process handler for get_process_times {}", GetLastError().0);
            //  return (0f64, 0f64);
            //}

            let mut createtime: FILETIME = FILETIME::default();
            let mut exit_time: FILETIME = FILETIME::default();
            let mut kernel_time: FILETIME = FILETIME::default();
            let mut user_time: FILETIME = FILETIME::default();
            let now = GetSystemTimeAsFileTime();
            let ret = GetProcessTimes(
                hprocess,
                &mut createtime,
                &mut exit_time,
                &mut kernel_time,
                &mut user_time,
            );
            CloseHandle(hprocess);
            if ret.as_bool() {
                let ktime_h = kernel_time.dwHighDateTime as u64;
                let ktime: u64 = (ktime_h << 32) + kernel_time.dwLowDateTime as u64;
                let utime_h = user_time.dwHighDateTime as u64;
                let utime = (utime_h << 32) + user_time.dwLowDateTime as u64;

                let ctime_h = createtime.dwHighDateTime as u64;
                let exit_h = exit_time.dwHighDateTime as u64;
                let now_h = now.dwHighDateTime as u64;
                let ctime = (ctime_h << 32u8) + createtime.dwLowDateTime as u64;
                let _exit_t = (exit_h << 32u8) + exit_time.dwLowDateTime as u64;
                let now_t = (now_h << 32u8) + now.dwLowDateTime as u64;

                (ktime as f64, utime as f64, (now_t - ctime) as f64)
            } else {
                (0f64, 0f64, 0f64)
            }
        }
    }

    pub fn get_memory_usages() -> (u64, u64) {
        unsafe {
            let hprocess = GetCurrentProcess();
            //if hprocess.is_invalid() {
            //  log::info!("Could not get current process handler for get_memory_usages {}", GetLastError().0);
            //  return (0u64, 0u64);
            //}
            let mut pmc = PROCESS_MEMORY_COUNTERS::default();
            let cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            pmc.cb = cb;
            let pmc_ptr = std::ptr::addr_of!(pmc) as *mut PROCESS_MEMORY_COUNTERS;
            let ret = GetProcessMemoryInfo(hprocess, pmc_ptr, cb);
            CloseHandle(hprocess);

            let status = MEMORYSTATUS {
                dwLength: size_of::<MEMORYSTATUS>() as u32,
                ..Default::default()
            };

            let status_ptr = std::ptr::addr_of!(status) as *mut MEMORYSTATUS;

            GlobalMemoryStatus(status_ptr); //调用GlobalMemoryStatus函数获取内存信息

            if ret.as_bool() {
                (status.dwTotalPhys as u64, pmc.WorkingSetSize as u64)
            } else {
                (status.dwTotalPhys as u64, 0u64)
            }
        }
    }

    pub fn get_handle_count() -> u32 {
        unsafe {
            let mut pdwhandlecount = 0;
            let hprocess = GetCurrentProcess();
            GetProcessHandleCount(hprocess, &mut pdwhandlecount);
            CloseHandle(hprocess);
            pdwhandlecount
        }
    }

    pub fn get_io_counter() -> (u64, u64) {
        unsafe {
            let mut ct = IO_COUNTERS::default();
            let hprocess = GetCurrentProcess();
            GetProcessIoCounters(hprocess, &mut ct);
            CloseHandle(hprocess);
            (ct.ReadTransferCount, ct.WriteTransferCount)
        }
    }

    pub fn get_network_io_counter() -> (u64, u64) {
        match NetworkPerformanceItem::scan_network_performance(
            Self::get_current_process_id(),
            false,
        ) {
            Ok(items) => {
                let mut sumitem = NetworkPerformanceItem::default();
                for it in items {
                    sumitem.bytes_in += it.bytes_in;
                    sumitem.bytes_out += it.bytes_out;
                }
                (sumitem.bytes_in, sumitem.bytes_out)
            }
            Err(err) => {
                log::info!("Error on scan networks performance {}", err);
                (0u64, 0u64)
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct NetworkPerformanceItem {
    pub process_id: u32,
    pub state: u32,
    pub conn_state: u32,
    // std::wstring LocalAddress;
    // std::wstring RemoteAddress
    pub local_address: Option<String>,
    pub remote_address: Option<String>,
    pub local_port: i32,
    pub remote_port: i32,
    pub bytes_out: u64,
    pub bytes_in: u64,
    pub outbound_bandwidth: u64,
    pub inbound_bandwidth: u64,
    pub pass: i32,
}

impl NetworkPerformanceItem {
    pub fn scan_network_performance(
        process_id: u32,
        resv: bool,
    ) -> Result<Vec<NetworkPerformanceItem>, anyhow::Error> {
        unsafe {
            let mut items = vec![];
            let mut tcp6 = NetworkPerformanceItem::scan_network_performance_tcp4(process_id, resv)?;
            let mut tcp4 = NetworkPerformanceItem::scan_network_performance_tcp6(process_id, resv)?;
            items.append(&mut tcp4);
            items.append(&mut tcp6);
            Ok(items)
        }
    }

    pub unsafe fn scan_network_performance_tcp4(
        _process_id: u32,
        resv: bool,
    ) -> Result<Vec<NetworkPerformanceItem>, anyhow::Error> {
        // std::vector<unsigned char> buffer;
        let mut dw_size = size_of::<MIB_TCPTABLE_OWNER_PID>() as u32;
        let mut dw_ret_value = ERROR_INSUFFICIENT_BUFFER;
        let mut network_performance_items = vec![];
        let heapbase = GetProcessHeap();
        // let mut buffer: Vec<c_void> = Vec::with_capacity(dw_size as usize);
        let mut buffer: *mut c_void = std::ptr::null::<c_void>() as *mut c_void;

        while dw_ret_value == ERROR_INSUFFICIENT_BUFFER {
            // buffer.resize(dw_size as usize, 0);
            if buffer != std::ptr::null::<c_void>() as *mut c_void {
                HeapFree(heapbase, 0, buffer);
            }
            buffer = HeapAlloc(heapbase, 0x00000008, dw_size as usize); //Vec::with_capacity(dw_size as usize);
            let bl = BOOL(0);
            let ptr = Some(buffer);
            dw_ret_value = WIN32_ERROR(GetExtendedTcpTable(
                ptr,
                &mut dw_size,
                bl,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ));
        }
        if dw_ret_value == ERROR_SUCCESS {
            // let rd = std::slice::from_ref(&*(buffer as * mut MIB_TCPTABLE_OWNER_PID));
            let pt_table = *(buffer as *mut MIB_TCPTABLE_OWNER_PID);
            // let pt_table = rd[0];

            let num = pt_table.dwNumEntries;
            let rawpt = buffer.add(size_of::<u32>()); //pt_table.table.as_ptr() as *const c_void;
            let ctt =
                std::slice::from_raw_parts(rawpt as *const MIB_TCPROW_OWNER_PID, num as usize);
            // caution: array starts with index 0, count starts by 1
            for table in ctt {
                // let tbl =  ctt.get(i as usize); //pt_table.table.get(i as usize); //.get(i as usize);
                if true {
                    //|| table.dwOwningPid == process_id {
                    let mut item = NetworkPerformanceItem {
                        process_id: table.dwOwningPid,
                        state: table.dwState,
                        local_port: table.dwLocalPort as i32,
                        remote_port: table.dwRemotePort as i32,
                        ..Default::default()
                    };

                    if resv {
                        // InetNtopW(AF_NET, paddr, pstringbuf)

                        let mut p = IN_ADDR_0::default(); //Box::new(IN_ADDR_0::default());
                        p.S_addr = table.dwLocalAddr;
                        // let mut paddr: Vec<u16> = Vec::with_capacity(256);
                        let mut pstraddr = [0u16; 128];
                        // TCHAR buff[160];
                        let raw: *const c_void = std::ptr::addr_of_mut!(p) as *const c_void;
                        let ret_addr = InetNtopW(AF_INET.0 as i32, raw, &mut pstraddr); // inet_ntoa(p);
                                                                                        // let errno = WSAGetLastError();
                        item.local_address = if ret_addr.is_null() {
                            None
                        } else {
                            match ret_addr.to_string() {
                                Ok(t) => Some(t),
                                Err(_) => None,
                            }
                        };

                        let mut p = IN_ADDR_0::default(); //Box::new(IN_ADDR_0::default());
                        p.S_addr = table.dwRemoteAddr;

                        let mut pstraddr = [0u16; 128];
                        let raw: *const c_void = std::ptr::addr_of_mut!(p) as *const c_void;
                        let ret_addr = InetNtopW(AF_INET.0 as i32, raw, &mut pstraddr); // inet_ntoa(p);
                        item.remote_address = if ret_addr.is_null() {
                            None
                        } else {
                            match ret_addr.to_string() {
                                Ok(t) => Some(t),
                                Err(_) => None,
                            }
                        };
                    }

                    // log::info!("Resolve address finished. {}  {}", item.local_address.clone().unwrap_or_default(), item.remote_address.clone().unwrap_or_default());

                    let row = MIB_TCPROW_LH {
                        dwLocalAddr: table.dwLocalAddr,
                        dwLocalPort: table.dwLocalPort,
                        dwRemoteAddr: table.dwRemoteAddr,
                        dwRemotePort: table.dwRemotePort,
                        // row.dwState = table.dwState;
                        Anonymous: MIB_TCPROW_LH_0 {
                            dwState: table.dwState,
                        },
                    };

                    item.conn_state = row.Anonymous.dwState;

                    if row.dwRemoteAddr > 0 {
                        let mut win_status;
                        let mut data_rw_ptr = [0u8; size_of::<TCP_ESTATS_DATA_RW_v0>()];
                        let data_rw = data_rw_ptr.as_mut_ptr() as *mut TCP_ESTATS_DATA_RW_v0; //TCP_ESTATS_DATA_RW_v0::default();
                                                                                              // *data_rw.EnableCollection = BOOLEAN(1);
                        (*data_rw).EnableCollection = BOOLEAN(1);

                        let mut band_width_ptr = [0u8; size_of::<TCP_ESTATS_BANDWIDTH_RW_v0>()];
                        // let mut band_width = TCP_ESTATS_BANDWIDTH_RW_v0::default();
                        let band_width =
                            band_width_ptr.as_mut_ptr() as *mut TCP_ESTATS_BANDWIDTH_RW_v0;
                        (*band_width).EnableCollectionInbound = TcpBoolOptEnabled;
                        (*band_width).EnableCollectionOutbound = TcpBoolOptEnabled;
                        win_status = SetPerTcpConnectionEStats(
                            &row,
                            TcpConnectionEstatsData,
                            &data_rw_ptr,
                            0,
                            0,
                        );
                        // log::info!("WinStatus1: {}",win_status);
                        if win_status != NO_ERROR.0 {
                            return Ok(network_performance_items);
                        }
                        win_status = SetPerTcpConnectionEStats(
                            &row,
                            TcpConnectionEstatsBandwidth,
                            &band_width_ptr,
                            0,
                            0,
                        );
                        // log::info!("WinStatus2: {}",win_status);
                        if win_status != NO_ERROR.0 {
                            return Ok(network_performance_items);
                        }

                        let mut rod_ptr = [0u8; size_of::<TCP_ESTATS_DATA_ROD_v0>()];
                        // let rod_ptr = std::ptr::addr_of_mut!(data_rod);//Box::new(Box::into_raw(data_rw) as *const c_void);
                        // let cpp = rod_ptr as * mut [u8; size_of::<TCP_ESTATS_DATA_ROD_v0>()];

                        let rod = Some(rod_ptr.as_mut_slice());

                        win_status = GetPerTcpConnectionEStats(
                            &row,
                            TcpConnectionEstatsData,
                            None,
                            0,
                            None,
                            0,
                            rod,
                            0,
                        );
                        if win_status == NO_ERROR.0
                            && (row.Anonymous.State == MIB_TCP_STATE_LISTEN
                                || row.Anonymous.State == MIB_TCP_STATE_SYN_SENT
                                || row.Anonymous.State == MIB_TCP_STATE_SYN_RCVD
                                || row.Anonymous.State == MIB_TCP_STATE_ESTAB)
                        {
                            // dataRod = (PTCP_ESTATS_DATA_ROD_v0)rod;
                            //wchar_t buf[512] = { 0 };
                            //wsprintf(buf, L"%I64d -- %I64d (local: %d, remote: %d)\n", dataRod->DataBytesIn, dataRod->DataBytesOut, row.dwLocalPort, row.dwRemotePort);
                            //OutputDebugString(buf);

                            let rod_ptr_void = &mut rod_ptr;

                            let data_rod = rod_ptr_void.as_ptr() as *const TCP_ESTATS_DATA_ROD_v0; // rod_ptr;//*cpp;//Box::into_raw(Box::from_raw(cpp.as_mut_ptr())).cast::<TCP_ESTATS_DATA_ROD_v0>();
                            let data_box = *data_rod;
                            // if the dataBytesIn and dataSegsIn are not equqal, the data may be valid
                            if !(data_box.DataBytesIn == data_box.DataSegsIn
                                && data_box.DataBytesOut == data_box.DataSegsOut)
                                && data_box.DataBytesIn > 0u64
                                && data_box.DataBytesOut > 0u64
                            {
                                item.bytes_in = data_box.DataBytesIn;
                                item.bytes_out = data_box.DataBytesOut;
                            }
                        }

                        let mut bandwidth_rod_ptr = [0u8; size_of::<TCP_ESTATS_BANDWIDTH_ROD_v0>()];

                        let bandwidth_rod = Some(rod_ptr.as_mut_slice());

                        win_status = GetPerTcpConnectionEStats(
                            &row,
                            TcpConnectionEstatsBandwidth,
                            None,
                            0,
                            None,
                            0,
                            bandwidth_rod,
                            0,
                        );
                        if win_status == NO_ERROR.0 {
                            let bw_rod_ptr_void = &mut bandwidth_rod_ptr;
                            let data_rod =
                                bw_rod_ptr_void.as_ptr() as *const TCP_ESTATS_BANDWIDTH_ROD_v0; // rod_ptr;//*cpp;//Box::into_raw(Box::from_raw(cpp.as_mut_ptr())).cast::<TCP_ESTATS_DATA_ROD_v0>();
                            let data_box = *data_rod;

                            item.outbound_bandwidth = data_box.InboundInstability / 8;
                            item.inbound_bandwidth = data_box.OutboundInstability / 8;
                        }
                    }
                    network_performance_items.push(item);
                }
                // i = i + 1;
                // log::info!("Loop for iterator {}", i);
            }
        }

        HeapFree(heapbase, 0, buffer);

        Ok(network_performance_items)
    }

    pub unsafe fn scan_network_performance_tcp6(
        _process_id: u32,
        resv: bool,
    ) -> Result<Vec<NetworkPerformanceItem>, anyhow::Error> {
        // std::vector<unsigned char> buffer;
        let mut dw_size = size_of::<MIB_TCP6TABLE_OWNER_PID>() as u32;
        let mut dw_ret_value = ERROR_INSUFFICIENT_BUFFER;
        let mut network_performance_items = vec![];
        let heapbase = GetProcessHeap();
        // let mut buffer: Vec<c_void> = Vec::with_capacity(dw_size as usize);

        let mut buffer: *mut c_void = std::ptr::null::<c_void>() as *mut c_void;

        while dw_ret_value == ERROR_INSUFFICIENT_BUFFER {
            // buffer.resize(dw_size as usize, 0);
            if buffer != std::ptr::null::<c_void>() as *mut c_void {
                HeapFree(heapbase, 0, buffer);
            }
            buffer = HeapAlloc(heapbase, 0x00000008, dw_size as usize); //Vec::with_capacity(dw_size as usize);
            let bl = BOOL(0);
            let ptr = Some(buffer);
            dw_ret_value = WIN32_ERROR(GetExtendedTcpTable(
                ptr,
                &mut dw_size,
                bl,
                AF_INET6.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ));
        }
        if dw_ret_value == ERROR_SUCCESS {
            // let rd = std::slice::from_ref(&*(buffer as * mut MIB_TCPTABLE_OWNER_PID));
            let pt_table = *(buffer as *mut MIB_TCP6TABLE_OWNER_PID);
            // let pt_table = rd[0];

            let num = pt_table.dwNumEntries;
            // let mut i;
            let rawpt = buffer.add(size_of::<u32>()); //pt_table.table.as_ptr() as *const c_void;
            let ctt =
                std::slice::from_raw_parts(rawpt as *const MIB_TCP6ROW_OWNER_PID, num as usize);
            // i = 0u32;
            // caution: array starts with index 0, count starts by 1
            for table in ctt {
                // let tbl =  ctt.get(i as usize); //pt_table.table.get(i as usize); //.get(i as usize);
                if true {
                    //|| table.dwOwningPid == process_id {
                    let mut item = NetworkPerformanceItem {
                        process_id: table.dwOwningPid,
                        state: table.dwState,
                        local_port: table.dwLocalPort as i32,
                        remote_port: table.dwRemotePort as i32,
                        ..Default::default()
                    };
                    if resv {
                        // InetNtopW(AF_NET, paddr, pstringbuf)

                        // let mut paddr: Vec<u16> = Vec::with_capacity(256);
                        let mut pstraddr = [0u16; 128];
                        // TCHAR buff[160];
                        let mut uradds = table.ucLocalAddr;
                        let raw: *const c_void = std::ptr::addr_of_mut!(uradds) as *const c_void;
                        let ret_addr = InetNtopW(AF_INET6.0 as i32, raw, &mut pstraddr); // inet_ntoa(p);
                                                                                         // let errno = WSAGetLastError();
                        item.local_address = if ret_addr.is_null() {
                            None
                        } else {
                            match ret_addr.to_string() {
                                Ok(t) => Some(t),
                                Err(_) => None,
                            }
                        };

                        let mut pstraddr = [0u16; 128];
                        let mut uradds = table.ucRemoteAddr;
                        let raw: *const c_void = std::ptr::addr_of_mut!(uradds) as *const c_void;
                        let ret_addr = InetNtopW(AF_INET6.0 as i32, raw, &mut pstraddr); // inet_ntoa(p);
                        item.remote_address = if ret_addr.is_null() {
                            None
                        } else {
                            match ret_addr.to_string() {
                                Ok(t) => Some(t),
                                Err(_) => None,
                            }
                        };
                    }

                    let mut row = MIB_TCP6ROW::default();

                    //row.LocalAddr = ptTable->table[i].ucLocalAddr;
                    row.LocalAddr.u.Byte = table.ucLocalAddr;
                    row.RemoteAddr.u.Byte = table.ucRemoteAddr;
                    row.dwLocalPort = table.dwLocalPort;
                    row.dwLocalScopeId = table.dwLocalScopeId;
                    row.dwRemotePort = table.dwRemotePort;
                    row.dwRemoteScopeId = table.dwRemoteScopeId;
                    row.State = MIB_TCP_STATE(table.dwState as i32);

                    item.conn_state = table.dwState;

                    if true {
                        let mut win_status;
                        let mut data_rw_ptr = [0u8; size_of::<TCP_ESTATS_DATA_RW_v0>()];
                        let data_rw = data_rw_ptr.as_mut_ptr() as *mut TCP_ESTATS_DATA_RW_v0; //TCP_ESTATS_DATA_RW_v0::default();
                                                                                              // *data_rw.EnableCollection = BOOLEAN(1);
                        (*data_rw).EnableCollection = BOOLEAN(1);

                        let mut band_width_ptr = [0u8; size_of::<TCP_ESTATS_BANDWIDTH_RW_v0>()];
                        // let mut band_width = TCP_ESTATS_BANDWIDTH_RW_v0::default();
                        let band_width =
                            band_width_ptr.as_mut_ptr() as *mut TCP_ESTATS_BANDWIDTH_RW_v0;
                        (*band_width).EnableCollectionInbound = TcpBoolOptEnabled;
                        (*band_width).EnableCollectionOutbound = TcpBoolOptEnabled;
                        win_status = SetPerTcp6ConnectionEStats(
                            &row,
                            TcpConnectionEstatsData,
                            &data_rw_ptr,
                            0,
                            0,
                        );
                        // log::info!("WinStatus1: {}",win_status);
                        if win_status != NO_ERROR.0 {
                            return Ok(network_performance_items);
                        }

                        win_status = SetPerTcp6ConnectionEStats(
                            &row,
                            TcpConnectionEstatsBandwidth,
                            &band_width_ptr,
                            0,
                            0,
                        );
                        if win_status != NO_ERROR.0 {
                            return Ok(network_performance_items);
                        }
                        // log::info!("WinStatus2: {}",win_status);

                        let mut rod_ptr = [0u8; size_of::<TCP_ESTATS_DATA_ROD_v0>()];
                        // let rod_ptr = std::ptr::addr_of_mut!(data_rod);//Box::new(Box::into_raw(data_rw) as *const c_void);
                        // let cpp = rod_ptr as * mut [u8; size_of::<TCP_ESTATS_DATA_ROD_v0>()];

                        let rod = Some(rod_ptr.as_mut_slice());

                        win_status = GetPerTcp6ConnectionEStats(
                            &row,
                            TcpConnectionEstatsData,
                            None,
                            0,
                            None,
                            0,
                            rod,
                            0,
                        );
                        if win_status == NO_ERROR.0
                            && (row.State == MIB_TCP_STATE_LISTEN
                                || row.State == MIB_TCP_STATE_SYN_SENT
                                || row.State == MIB_TCP_STATE_SYN_RCVD
                                || row.State == MIB_TCP_STATE_ESTAB)
                        {
                            // dataRod = (PTCP_ESTATS_DATA_ROD_v0)rod;
                            //wchar_t buf[512] = { 0 };
                            //wsprintf(buf, L"%I64d -- %I64d (local: %d, remote: %d)\n", dataRod->DataBytesIn, dataRod->DataBytesOut, row.dwLocalPort, row.dwRemotePort);
                            //OutputDebugString(buf);

                            let rod_ptr_void = &mut rod_ptr;

                            let data_rod = rod_ptr_void.as_ptr() as *const TCP_ESTATS_DATA_ROD_v0; // rod_ptr;//*cpp;//Box::into_raw(Box::from_raw(cpp.as_mut_ptr())).cast::<TCP_ESTATS_DATA_ROD_v0>();
                            let data_box = *data_rod;
                            // if the dataBytesIn and dataSegsIn are not equqal, the data may be valid
                            if !(data_box.DataBytesIn == data_box.DataSegsIn
                                && data_box.DataBytesOut == data_box.DataSegsOut)
                                && data_box.DataBytesIn > 0u64
                                && data_box.DataBytesOut > 0u64
                            {
                                item.bytes_in = data_box.DataBytesIn;
                                item.bytes_out = data_box.DataBytesOut;
                            }
                        }

                        let mut bandwidth_rod_ptr = [0u8; size_of::<TCP_ESTATS_BANDWIDTH_ROD_v0>()];

                        let bandwidth_rod = Some(rod_ptr.as_mut_slice());

                        win_status = GetPerTcp6ConnectionEStats(
                            &row,
                            TcpConnectionEstatsBandwidth,
                            None,
                            0,
                            None,
                            0,
                            bandwidth_rod,
                            0,
                        );
                        if win_status == NO_ERROR.0 {
                            let bw_rod_ptr_void = &mut bandwidth_rod_ptr;
                            let data_rod =
                                bw_rod_ptr_void.as_ptr() as *const TCP_ESTATS_BANDWIDTH_ROD_v0; // rod_ptr;//*cpp;//Box::into_raw(Box::from_raw(cpp.as_mut_ptr())).cast::<TCP_ESTATS_DATA_ROD_v0>();
                            let data_box = *data_rod;

                            item.outbound_bandwidth = data_box.InboundInstability / 8;
                            item.inbound_bandwidth = data_box.OutboundInstability / 8;
                        }
                    }
                    network_performance_items.push(item);
                }
                // i = i + 1;
                // log::info!("Loop for iterator {}", i);
            }
        }

        HeapFree(heapbase, 0, buffer);

        Ok(network_performance_items)
    }
}
