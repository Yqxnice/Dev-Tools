//! 系统信息采集：操作系统版本、架构、CPU、内存、运行时间等。
//!
//! 通过 Windows API（注册表 + GlobalMemoryStatusEx + GetTickCount64）获取系统信息，
//! 供前端"系统信息"页面展示。无需额外依赖（windows crate 已在项目中）。

use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_VALUE_TYPE,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, GetTickCount64, MEMORYSTATUSEX};

use crate::types::SystemInfo;

/// 从注册表读取字符串值
fn reg_read_string(hkey: HKEY, sub_key: &str, value_name: &str) -> Option<String> {
    let mut sub_key_wide: Vec<u16> = sub_key.encode_utf16().collect();
    sub_key_wide.push(0);
    let mut value_name_wide: Vec<u16> = value_name.encode_utf16().collect();
    value_name_wide.push(0);

    let mut h_subkey = HKEY::default();
    unsafe {
        if RegOpenKeyExW(hkey, PCWSTR(sub_key_wide.as_ptr()), 0, KEY_READ, &mut h_subkey)
            != ERROR_SUCCESS
        {
            return None;
        }
    }

    let mut data_type: REG_VALUE_TYPE = REG_VALUE_TYPE(0);
    let mut data_size: u32 = 0;
    unsafe {
        // 第一次调用获取所需缓冲区大小
        let _ = RegQueryValueExW(
            h_subkey,
            PCWSTR(value_name_wide.as_ptr()),
            None,
            Some(&mut data_type),
            None,
            Some(&mut data_size),
        );
    }

    if data_size == 0 {
        unsafe {
            let _ = RegCloseKey(h_subkey);
        }
        return None;
    }

    let mut buffer: Vec<u8> = vec![0u8; data_size as usize];
    unsafe {
        let result = RegQueryValueExW(
            h_subkey,
            PCWSTR(value_name_wide.as_ptr()),
            None,
            Some(&mut data_type),
            Some(buffer.as_mut_ptr()),
            Some(&mut data_size),
        );
        let _ = RegCloseKey(h_subkey);
        if result != ERROR_SUCCESS {
            return None;
        }
    }

    // REG_SZ (1) 或 REG_EXPAND_SZ (2)：UTF-16 字符串
    let raw_type = data_type.0;
    if raw_type == 1 || raw_type == 2 {
        let wide: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        // 去掉末尾的 null 终止符
        let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        let s = String::from_utf16_lossy(&wide[..len]);
        return if s.is_empty() { None } else { Some(s) };
    }

    // REG_DWORD (4)：转字符串
    if raw_type == 4 && buffer.len() >= 4 {
        let val = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        return Some(val.to_string());
    }

    None
}

/// 修正 Windows 产品名称。
///
/// 兼容性原因：Windows 11 的注册表 ProductName 仍写作 "Windows 10 Pro" 等旧名称，
/// 必须通过 CurrentBuild 判断真实系统：Build >= 22000 为 Windows 11，
/// 此时将名称前缀 "Windows 10" 替换为 "Windows 11"（如 "Windows 10 Pro" → "Windows 11 Pro"）。
fn fix_windows_product_name(
    product_name: Option<String>,
    current_build: Option<&str>,
) -> Option<String> {
    let build: Option<u32> = current_build.and_then(|b| b.parse().ok());
    match (product_name, build) {
        (Some(name), Some(build)) if build >= 22000 && name.starts_with("Windows 10") => {
            Some(name.replacen("Windows 10", "Windows 11", 1))
        }
        (name, _) => name,
    }
}

/// 获取 Windows 显示版本号（如 "23H2"）
fn get_windows_display_version() -> Option<String> {
    // 优先读 DisplayVersion（Win10 2009+），回退到 ReleaseId
    reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "DisplayVersion",
    )
    .or_else(|| {
        reg_read_string(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            "ReleaseId",
        )
    })
}

/// 采集当前系统信息
#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    // 操作系统信息（注册表）
    let raw_product_name = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "ProductName",
    );
    let display_version = get_windows_display_version();
    let current_build = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuild",
    );
    // Windows 11 的 ProductName 注册表值仍为 "Windows 10 ..."，按 Build 号修正
    let os_name = fix_windows_product_name(raw_product_name, current_build.as_deref());
    let kernel_version = current_build.clone();
    let os_version = match (display_version, current_build) {
        (Some(d), Some(b)) => Some(format!("{} (Build {})", d, b)),
        (None, Some(b)) => Some(format!("Build {}", b)),
        (Some(d), None) => Some(d),
        (None, None) => None,
    };
    let host_name = std::env::var("COMPUTERNAME").ok();

    // 架构（编译时确定，桌面应用场景足够）
    let architecture = Some(std::env::consts::ARCH.to_string());

    // CPU 信息（注册表）
    let cpu_name = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "ProcessorNameString",
    )
    .unwrap_or_default()
    .trim()
    .to_string();
    let cpu_frequency_mhz = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "~MHz",
    )
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(0);
    let cpu_logical_cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(0);
    // 物理核心数：Windows 无简单 API，通过环境变量 NUMBER_OF_PROCESSORS（逻辑核）
    // 物理核需要 GetLogicalProcessorInformation，这里暂用逻辑核占位
    let cpu_physical_cores = cpu_logical_cores;

    // 内存信息（GlobalMemoryStatusEx）
    let mut mem_status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    let (total_memory_bytes, used_memory_bytes, available_memory_bytes) = unsafe {
        if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
            let total = mem_status.ullTotalPhys;
            let avail = mem_status.ullAvailPhys;
            let used = total.saturating_sub(avail);
            (total, used, avail)
        } else {
            (0, 0, 0)
        }
    };

    // 系统运行时间（GetTickCount64，毫秒 → 秒）
    let uptime_seconds = unsafe { GetTickCount64() / 1000 };

    SystemInfo {
        os_name,
        os_version,
        kernel_version,
        host_name,
        architecture,
        cpu_name,
        cpu_frequency_mhz,
        cpu_physical_cores,
        cpu_logical_cores,
        total_memory_bytes,
        used_memory_bytes,
        available_memory_bytes,
        uptime_seconds,
    }
}
