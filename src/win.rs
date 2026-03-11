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
            AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
                SetProcessDpiAwarenessContext, SetThreadDpiAwarenessContext,
            },
            Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
                SetActiveWindow, SetFocus, VK_MENU,
            },
            WindowsAndMessaging::{
                BringWindowToTop, GA_ROOT, GetAncestor, GetForegroundWindow,
                GetWindowThreadProcessId, IsIconic, IsWindow, SW_RESTORE, SetForegroundWindow,
                ShowWindow, WindowFromPoint,
            },
        },
    },
    core::PWSTR,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorToken(pub isize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowToken(pub isize);

impl WindowToken {
    pub fn hwnd(self) -> HWND {
        HWND(self.0 as *mut _)
    }
}

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

pub fn foreground_window() -> Option<WindowToken> {
    unsafe { normalize_window_handle(GetForegroundWindow()) }
}

pub fn foreground_window_on_monitor(monitor: MonitorToken) -> Option<WindowToken> {
    let hwnd = foreground_window()?;
    if monitor_from_hwnd(hwnd.hwnd())? != monitor {
        return None;
    }

    Some(hwnd)
}

pub fn gesture_target_window_for_point(point: POINT) -> Option<WindowToken> {
    monitor_from_point(point)
        .and_then(|(monitor, _)| foreground_window_on_monitor(monitor))
        .or_else(|| window_at_point(point))
}

pub fn window_at_point(point: POINT) -> Option<WindowToken> {
    unsafe { normalize_window_handle(WindowFromPoint(point)) }
}


pub fn process_name_at_point(point: POINT) -> Option<String> {
    process_name_for_window(window_at_point(point)?)
}

pub fn process_name_for_window(window: WindowToken) -> Option<String> {
    process_name_for_hwnd(window.hwnd())
}


pub fn activate_window(window: WindowToken) -> anyhow::Result<()> {
    unsafe {
        let hwnd = window.hwnd();
        if hwnd.0.is_null() || !IsWindow(Some(hwnd)).as_bool() {
            return Err(anyhow!("invalid target window handle"));
        }

        if foreground_window() == Some(window) {
            return Ok(());
        }

        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        let current_thread_id = GetCurrentThreadId();
        let target_thread_id = GetWindowThreadProcessId(hwnd, None);
        let current_foreground = GetForegroundWindow();
        let foreground_thread_id = if current_foreground.0.is_null() {
            0
        } else {
            GetWindowThreadProcessId(current_foreground, None)
        };

        let attached_thread_ids =
            attach_input_threads(current_thread_id, &[foreground_thread_id, target_thread_id]);

        let _ = try_focus_window(hwnd);
        if foreground_window() == Some(window) {
            detach_input_threads(current_thread_id, &attached_thread_ids);
            return Ok(());
        }

        let _ = tap_alt_key();
        let _ = try_focus_window(hwnd);
        let activated = foreground_window() == Some(window);

        detach_input_threads(current_thread_id, &attached_thread_ids);

        if activated {
            Ok(())
        } else {
            Err(anyhow!(
                "failed to activate target window after attach-thread-input fallback"
            ))
        }
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

unsafe fn try_focus_window(hwnd: HWND) -> anyhow::Result<()> {
    let _ = BringWindowToTop(hwnd);
    let _ = SetActiveWindow(hwnd);
    let _ = SetFocus(Some(hwnd));

    if SetForegroundWindow(hwnd).as_bool() {
        return Ok(());
    }

    Err(anyhow!("SetForegroundWindow rejected target window"))
}

unsafe fn attach_input_threads(current_thread_id: u32, thread_ids: &[u32]) -> Vec<u32> {
    let mut attached = Vec::new();

    for thread_id in thread_ids {
        if *thread_id == 0 || *thread_id == current_thread_id || attached.contains(thread_id) {
            continue;
        }

        if AttachThreadInput(current_thread_id, *thread_id, true).as_bool() {
            attached.push(*thread_id);
        }
    }

    attached
}

unsafe fn detach_input_threads(current_thread_id: u32, thread_ids: &[u32]) {
    for thread_id in thread_ids {
        let _ = AttachThreadInput(current_thread_id, *thread_id, false);
    }
}

unsafe fn tap_alt_key() -> anyhow::Result<()> {
    let inputs = [
        keyboard_input(VK_MENU, Default::default()),
        keyboard_input(VK_MENU, KEYEVENTF_KEYUP),
    ];

    let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    if sent != inputs.len() as u32 {
        return Err(anyhow!(
            "SendInput sent {} of {} events while unlocking foreground",
            sent,
            inputs.len()
        ));
    }

    Ok(())
}

fn keyboard_input(
    vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
    flags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS,
) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn normalize_window_handle(hwnd: HWND) -> Option<WindowToken> {
    unsafe {
        if hwnd.0.is_null() || !IsWindow(Some(hwnd)).as_bool() {
            return None;
        }

        let root = GetAncestor(hwnd, GA_ROOT);
        let hwnd = if root.0.is_null() { hwnd } else { root };
        if hwnd.0.is_null() || !IsWindow(Some(hwnd)).as_bool() {
            return None;
        }

        Some(WindowToken(hwnd.0 as isize))
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