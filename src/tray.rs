use std::sync::Arc;

use anyhow::{Context, anyhow};
use once_cell::sync::OnceCell;
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
                Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
                DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW, IDI_APPLICATION,
                LoadIconW, MF_CHECKED, MF_STRING, MF_UNCHECKED, MSG, PostMessageW, PostQuitMessage,
                RegisterClassW, SW_HIDE, SetForegroundWindow, ShowWindow, TPM_LEFTALIGN,
                TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_APP, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
                WM_NULL, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPEDWINDOW,
            },
        },
    },
    core::PCWSTR,
};

use crate::{app::AppContext, win::to_wide};

const WM_TRAYICON: u32 = WM_APP + 1;
const ID_TOGGLE_GESTURES: usize = 1001;
const ID_OPEN_CONFIG: usize = 1002;
const ID_EXIT: usize = 1003;
const APP_ICON_RESOURCE_ID: u16 = 1;

static CONTEXT: OnceCell<Arc<AppContext>> = OnceCell::new();

pub fn run(context: Arc<AppContext>) -> anyhow::Result<()> {
    let _ = CONTEXT.set(context);

    unsafe {
        let class_name = to_wide("GestoTrayWindow");
        let hinstance = GetModuleHandleW(None).context("failed to get module handle")?;
        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(HINSTANCE(hinstance.0)),
            None,
        )
        .context("failed to create tray message window")?;

        add_tray_icon(hwnd)?;
        let _ = ShowWindow(hwnd, SW_HIDE);

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        remove_tray_icon(hwnd);
        Ok(())
    }
}

unsafe fn add_tray_icon(hwnd: HWND) -> anyhow::Result<()> {
    let mut data = tray_icon_data(hwnd)?;
    if !Shell_NotifyIconW(NIM_ADD, &mut data).as_bool() {
        return Err(anyhow!("failed to add tray icon"));
    }
    Ok(())
}

unsafe fn refresh_tray_icon(hwnd: HWND) {
    if let Ok(mut data) = tray_icon_data(hwnd) {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &mut data);
    }
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    if let Ok(mut data) = tray_icon_data(hwnd) {
        let _ = Shell_NotifyIconW(NIM_DELETE, &mut data);
    }
}

fn make_int_resource(resource_id: u16) -> PCWSTR {
    PCWSTR(resource_id as usize as *const u16)
}

unsafe fn load_tray_icon() -> anyhow::Result<windows::Win32::UI::WindowsAndMessaging::HICON> {
    let hinstance = HINSTANCE(
        GetModuleHandleW(None)
            .context("failed to get module handle")?
            .0,
    );
    LoadIconW(Some(hinstance), make_int_resource(APP_ICON_RESOURCE_ID))
        .or_else(|_| LoadIconW(None, IDI_APPLICATION))
        .context("failed to load tray icon")
}

unsafe fn tray_icon_data(hwnd: HWND) -> anyhow::Result<NOTIFYICONDATAW> {
    let mut data = NOTIFYICONDATAW::default();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uFlags = NIF_MESSAGE | NIF_TIP | NIF_ICON;
    data.uCallbackMessage = WM_TRAYICON;
    data.hIcon = load_tray_icon()?;

    let tip = to_wide(tray_tooltip_label());
    for (index, ch) in tip
        .iter()
        .take(data.szTip.len().saturating_sub(1))
        .enumerate()
    {
        data.szTip[index] = *ch;
    }

    Ok(data)
}

unsafe fn show_context_menu(hwnd: HWND) {
    refresh_tray_icon(hwnd);

    let menu = match CreatePopupMenu() {
        Ok(menu) => menu,
        Err(_) => return,
    };
    let gestures_enabled = tray_gestures_enabled();
    let (toggle_label, open_label, exit_label) = tray_menu_labels();
    let toggle_flags = if gestures_enabled {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING | MF_UNCHECKED
    };
    let toggle_text = to_wide(toggle_label);
    let open_text = to_wide(open_label);
    let exit_text = to_wide(exit_label);
    let _ = AppendMenuW(
        menu,
        toggle_flags,
        ID_TOGGLE_GESTURES,
        PCWSTR(toggle_text.as_ptr()),
    );
    let _ = AppendMenuW(menu, MF_STRING, ID_OPEN_CONFIG, PCWSTR(open_text.as_ptr()));
    let _ = AppendMenuW(menu, MF_STRING, ID_EXIT, PCWSTR(exit_text.as_ptr()));

    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_RIGHTBUTTON,
        cursor.x,
        cursor.y,
        Some(0),
        hwnd,
        None,
    );
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(menu);
}

fn current_locale() -> String {
    CONTEXT
        .get()
        .map(|context| context.config_snapshot().locale)
        .unwrap_or_else(|| "zh-CN".to_string())
}

fn tray_gestures_enabled() -> bool {
    CONTEXT
        .get()
        .map(|context| context.gestures_enabled())
        .unwrap_or(true)
}

fn tray_tooltip_label() -> &'static str {
    match current_locale().as_str() {
        "en-US" => "Gesto - Mouse Gestures",
        _ => "Gesto - 鼠标手势",
    }
}

fn tray_menu_labels() -> (&'static str, &'static str, &'static str) {
    match current_locale().as_str() {
        "en-US" => ("Enable Gestures", "Open Config", "Exit"),
        _ => ("启用鼠标手势", "打开配置", "退出"),
    }
}

fn toggle_gestures_enabled() {
    if let Some(context) = CONTEXT.get() {
        let next_enabled = !context.gestures_enabled();
        if let Err(error) = context.set_gestures_enabled(next_enabled) {
            eprintln!("[Gesto] failed to update gesture toggle: {error:#}");
        }
    }
}

fn open_config_page() {
    if let Some(context) = CONTEXT.get() {
        let _ = webbrowser::open(&context.server_url());
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            match (wparam.0 & 0xffff) as usize {
                ID_TOGGLE_GESTURES => toggle_gestures_enabled(),
                ID_OPEN_CONFIG => open_config_page(),
                ID_EXIT => {
                    remove_tray_icon(hwnd);
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_TRAYICON => {
            match lparam.0 as u32 {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => open_config_page(),
                WM_RBUTTONUP | WM_CONTEXTMENU => show_context_menu(hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
