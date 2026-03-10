use anyhow::anyhow;
use std::{mem::size_of, os::windows::ffi::OsStrExt, path::Path};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND, POINT, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
            MonitorFromWindow,
        },
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        },
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
                SetProcessDpiAwarenessContext, SetThreadDpiAwarenessContext,
            },
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    },
    core::PWSTR,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorToken(pub isize);

#[derive(Clone, Copy, Debug)]
pub struct MonitorBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl From<RECT> for MonitorBounds {
    fn from(value: RECT) -> Self {
        Self {
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

pub fn to_wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn enable_per_monitor_dpi_awareness() -> anyhow::Result<()> {
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_ok() {
            return Ok(());
        }

        if SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).0
            != std::ptr::null_mut()
        {
            return Ok(());
        }

        Err(anyhow!("failed to enable per-monitor DPI awareness"))
    }
}

pub fn ensure_current_thread_per_monitor_dpi_awareness() {
    unsafe {
        let _ = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn monitor_scale_factor(monitor: MonitorToken) -> f32 {
    monitor_effective_dpi(monitor)
        .map(|dpi| dpi as f32 / 96.0)
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0)
}

pub fn monitor_from_point(point: POINT) -> Option<(MonitorToken, MonitorBounds)> {
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            return None;
        }

        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info as *mut MONITORINFO as *mut _)
            .ok()
            .ok()?;
        Some((
            MonitorToken(monitor.0 as isize),
            MonitorBounds::from(info.rcMonitor),
        ))
    }
}

pub fn monitor_from_hwnd(hwnd: HWND) -> Option<MonitorToken> {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            None
        } else {
            Some(MonitorToken(monitor.0 as isize))
        }
    }
}

pub fn foreground_process_on_monitor(monitor: MonitorToken) -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        if monitor_from_hwnd(hwnd)? != monitor {
            return None;
        }
        process_name_for_hwnd(hwnd)
    }
}

pub fn process_name_for_hwnd(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id == 0 {
            return None;
        }

        let process = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) {
            Ok(handle) => handle,
            Err(_) => return None,
        };

        let mut buffer = vec![0u16; 1024];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            windows::Win32::System::Threading::PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);
        if result.is_err() {
            return None;
        }

        let full_path = String::from_utf16_lossy(&buffer[..length as usize]);
        Path::new(&full_path)
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
    }
}

fn monitor_effective_dpi(monitor: MonitorToken) -> Option<u32> {
    unsafe {
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        GetDpiForMonitor(
            HMONITOR(monitor.0 as *mut _),
            MDT_EFFECTIVE_DPI,
            &mut dpi_x,
            &mut dpi_y,
        )
        .ok()?;
        Some(dpi_x.max(dpi_y))
    }
}
