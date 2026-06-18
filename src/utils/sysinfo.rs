use std::sync::OnceLock;

#[derive(Clone, Default)]
pub struct SystemInfo {
    pub os: String,
    pub os_build: String,
    pub os_display: String,
    pub cpu: String,
    pub cpu_cores: u32,
    pub ram_total_mb: u64,
    pub gpu: String,
    pub arch: String,
    pub screen: String,
    pub monitors: u32,
}

static INFO: OnceLock<SystemInfo> = OnceLock::new();
static SUMMARY: OnceLock<String> = OnceLock::new();

pub fn get() -> &'static SystemInfo {
    INFO.get_or_init(collect)
}

pub fn summary_json() -> String {
    SUMMARY
        .get_or_init(|| {
            let i = get();
            serde_json::json!({
                "os": i.os,
                "os_build": i.os_build,
                "os_display": i.os_display,
                "cpu": i.cpu,
                "cpu_cores": i.cpu_cores,
                "ram_total_mb": i.ram_total_mb,
                "gpu": i.gpu,
                "arch": i.arch,
                "screen": i.screen,
                "monitors": i.monitors,
            })
            .to_string()
        })
        .clone()
}

#[cfg(windows)]
pub fn available_memory_mb() -> u64 {
    mem_status()
        .map(|m| m.ullAvailPhys / 1024 / 1024)
        .unwrap_or(u64::MAX)
}

#[cfg(not(windows))]
pub fn available_memory_mb() -> u64 {
    u64::MAX
}

#[cfg(windows)]
pub fn total_memory_mb() -> u64 {
    mem_status()
        .map(|m| m.ullTotalPhys / 1024 / 1024)
        .unwrap_or(0)
}

#[cfg(not(windows))]
pub fn total_memory_mb() -> u64 {
    0
}

#[cfg(windows)]
fn mem_status() -> Option<windows::Win32::System::SystemInformation::MEMORYSTATUSEX> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut ms).is_ok() {
            return Some(ms);
        }
    }
    None
}

fn cpu_cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(0)
}

#[cfg(windows)]
fn collect() -> SystemInfo {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;

    let mut os = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "ProductName",
    )
    .unwrap_or_else(|| "Windows".to_string());
    let mut os_build = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .unwrap_or_default();
    if let Ok(build_num) = os_build.parse::<u32>() {
        if build_num >= 22000 {
            os = os.replacen("Windows 10", "Windows 11", 1);
        }
    }
    if let Some(ubr) = reg_read_u32(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "UBR",
    ) {
        if !os_build.is_empty() {
            os_build = format!("{}.{}", os_build, ubr);
        }
    }
    let os_display = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "DisplayVersion",
    )
    .unwrap_or_default();
    let cpu = reg_read_string(
        HKEY_LOCAL_MACHINE,
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "ProcessorNameString",
    )
    .unwrap_or_default();

    SystemInfo {
        os,
        os_build,
        os_display,
        cpu,
        cpu_cores: cpu_cores(),
        ram_total_mb: total_memory_mb(),
        gpu: primary_gpu(),
        arch: std::env::consts::ARCH.to_string(),
        screen: primary_screen(),
        monitors: monitor_count(),
    }
}

#[cfg(not(windows))]
fn collect() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        cpu_cores: cpu_cores(),
        ram_total_mb: total_memory_mb(),
        arch: std::env::consts::ARCH.to_string(),
        ..Default::default()
    }
}

#[cfg(windows)]
fn primary_gpu() -> String {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW};
    let mut dd = DISPLAY_DEVICEW {
        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    unsafe {
        if EnumDisplayDevicesW(PCWSTR::null(), 0, &mut dd, 0).as_bool() {
            return utf16_to_string(&dd.DeviceString);
        }
    }
    String::new()
}

#[cfg(windows)]
fn primary_screen() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w > 0 && h > 0 {
            return format!("{}x{}", w, h);
        }
    }
    String::new()
}

#[cfg(windows)]
fn monitor_count() -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};
    unsafe { GetSystemMetrics(SM_CMONITORS).max(0) as u32 }
}

#[cfg(windows)]
fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end]).trim().to_string()
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn reg_read_string(
    root: windows::Win32::System::Registry::HKEY,
    subkey: &str,
    value: &str,
) -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_VALUE_TYPE,
    };
    unsafe {
        let mut hkey = HKEY::default();
        let sk = wide(subkey);
        if RegOpenKeyExW(root, PCWSTR(sk.as_ptr()), 0, KEY_READ, &mut hkey).is_err() {
            return None;
        }
        let val = wide(value);
        let mut ty = REG_VALUE_TYPE::default();
        let mut size: u32 = 0;
        let rc = RegQueryValueExW(
            hkey,
            PCWSTR(val.as_ptr()),
            None,
            Some(&mut ty),
            None,
            Some(&mut size),
        );
        if rc.is_err() || size == 0 {
            let _ = RegCloseKey(hkey);
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let rc = RegQueryValueExW(
            hkey,
            PCWSTR(val.as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr()),
            Some(&mut size),
        );
        let _ = RegCloseKey(hkey);
        if rc.is_err() {
            return None;
        }
        let count = (size as usize) / 2;
        let u16s = std::slice::from_raw_parts(buf.as_ptr() as *const u16, count);
        Some(utf16_to_string(u16s))
    }
}

#[cfg(windows)]
fn reg_read_u32(
    root: windows::Win32::System::Registry::HKEY,
    subkey: &str,
    value: &str,
) -> Option<u32> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_VALUE_TYPE,
    };
    unsafe {
        let mut hkey = HKEY::default();
        let sk = wide(subkey);
        if RegOpenKeyExW(root, PCWSTR(sk.as_ptr()), 0, KEY_READ, &mut hkey).is_err() {
            return None;
        }
        let val = wide(value);
        let mut ty = REG_VALUE_TYPE::default();
        let mut data: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let rc = RegQueryValueExW(
            hkey,
            PCWSTR(val.as_ptr()),
            None,
            Some(&mut ty),
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut size),
        );
        let _ = RegCloseKey(hkey);
        if rc.is_err() {
            return None;
        }
        Some(data)
    }
}
