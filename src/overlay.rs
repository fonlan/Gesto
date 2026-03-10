use std::{
    ffi::c_void,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow};
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke};
use windows::{
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
            CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
            HGDIOBJ, ReleaseDC, SelectObject,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW, MSG,
            PM_REMOVE, PeekMessageW, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
            SetWindowPos, ShowWindow, TranslateMessage, ULW_ALPHA, UpdateLayeredWindow,
            WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
    core::PCWSTR,
};

use crate::{
    config::GeneralSettings,
    win::{MonitorBounds, ensure_current_thread_per_monitor_dpi_awareness, to_wide},
};

#[derive(Clone)]
pub struct OverlayController {
    tx: Sender<OverlayCommand>,
}

#[derive(Clone, Copy, Debug)]
pub struct TrailStyle {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
    pub width: f32,
    pub fade_duration_ms: u64,
}

#[derive(Clone)]
enum OverlayCommand {
    Show {
        monitor: MonitorBounds,
        points: Vec<POINT>,
        style: TrailStyle,
    },
    Finish,
    Hide,
}

#[derive(Default)]
struct OverlayState {
    monitor: Option<MonitorBounds>,
    points: Vec<POINT>,
    style: Option<TrailStyle>,
    fade_started_at: Option<Instant>,
    visible: bool,
}

impl TrailStyle {
    pub fn from_general(general: &GeneralSettings) -> Self {
        let (red, green, blue) = parse_hex_color(&general.trail_color).unwrap_or((59, 130, 246));
        Self {
            red,
            green,
            blue,
            alpha: opacity_percent_to_alpha(general.trail_opacity),
            width: general.trail_width,
            fade_duration_ms: general.fade_duration_ms,
        }
    }
}

fn opacity_percent_to_alpha(opacity_percent: f32) -> u8 {
    ((opacity_percent.clamp(0.0, 100.0) / 100.0) * 255.0).round() as u8
}

impl OverlayController {
    pub fn spawn() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel::<OverlayCommand>();
        thread::Builder::new()
            .name("gesto-overlay".to_string())
            .spawn(move || {
                if let Err(error) = run_overlay_thread(rx) {
                    eprintln!("[Gesto] overlay thread error: {error:#}");
                }
            })
            .context("failed to spawn overlay thread")?;
        Ok(Self { tx })
    }

    pub fn show(&self, monitor: MonitorBounds, points: &[POINT], style: TrailStyle) {
        let _ = self.tx.send(OverlayCommand::Show {
            monitor,
            points: points.to_vec(),
            style,
        });
    }

    pub fn finish(&self) {
        let _ = self.tx.send(OverlayCommand::Finish);
    }

    pub fn hide(&self) {
        let _ = self.tx.send(OverlayCommand::Hide);
    }
}

fn run_overlay_thread(rx: Receiver<OverlayCommand>) -> anyhow::Result<()> {
    unsafe {
        ensure_current_thread_per_monitor_dpi_awareness();

        let class_name = to_wide("GestoOverlayWindow");
        let hinstance = GetModuleHandleW(None).context("failed to get module handle")?;
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(overlay_wnd_proc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&wnd_class);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(
                WS_EX_LAYERED.0
                    | WS_EX_TRANSPARENT.0
                    | WS_EX_TOOLWINDOW.0
                    | WS_EX_TOPMOST.0
                    | WS_EX_NOACTIVATE.0,
            ),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE(WS_POPUP.0),
            0,
            0,
            1,
            1,
            None,
            None,
            Some(HINSTANCE(hinstance.0)),
            None,
        )
        .context("failed to create overlay window")?;

        let _ = ShowWindow(hwnd, SW_HIDE);
        overlay_loop(hwnd, rx)?;
        Ok(())
    }
}

unsafe fn overlay_loop(hwnd: HWND, rx: Receiver<OverlayCommand>) -> anyhow::Result<()> {
    let mut state = OverlayState::default();

    loop {
        while let Ok(command) = rx.try_recv() {
            apply_command(&mut state, command, hwnd);
        }

        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        if state.visible {
            let should_continue = render_state(hwnd, &mut state)?;
            if !should_continue {
                let _ = ShowWindow(hwnd, SW_HIDE);
                state.visible = false;
                state.fade_started_at = None;
            }
        }

        match rx.recv_timeout(Duration::from_millis(16)) {
            Ok(command) => apply_command(&mut state, command, hwnd),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

unsafe fn apply_command(state: &mut OverlayState, command: OverlayCommand, hwnd: HWND) {
    match command {
        OverlayCommand::Show {
            monitor,
            points,
            style,
        } => {
            state.monitor = Some(monitor);
            state.points = points;
            state.style = Some(style);
            state.fade_started_at = None;
            state.visible = true;
        }
        OverlayCommand::Finish => {
            if state.visible && state.fade_started_at.is_none() {
                state.fade_started_at = Some(Instant::now());
            }
        }
        OverlayCommand::Hide => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            state.visible = false;
            state.fade_started_at = None;
            state.points.clear();
        }
    }
}

unsafe fn render_state(hwnd: HWND, state: &mut OverlayState) -> anyhow::Result<bool> {
    let monitor = match state.monitor {
        Some(value) => value,
        None => return Ok(false),
    };
    let style = match state.style {
        Some(value) => value,
        None => return Ok(false),
    };
    if state.points.is_empty() {
        return Ok(false);
    }

    let alpha_scale = if let Some(started_at) = state.fade_started_at {
        let elapsed = started_at.elapsed().as_secs_f32();
        let total = style.fade_duration_ms.max(60) as f32 / 1_000.0;
        let ratio = (1.0 - elapsed / total).clamp(0.0, 1.0);
        if ratio <= 0.0 {
            return Ok(false);
        }
        ratio
    } else {
        1.0
    };

    let bounds = compute_bounds(&state.points, monitor, style.width)?;
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    if width <= 0 || height <= 0 {
        return Ok(false);
    }

    let mut pixmap = Pixmap::new(width as u32, height as u32)
        .ok_or_else(|| anyhow!("failed to allocate overlay pixmap"))?;

    if state.points.len() == 1 {
        if let Some(circle) = PathBuilder::from_circle(
            (state.points[0].x - bounds.left) as f32,
            (state.points[0].y - bounds.top) as f32,
            style.width.max(4.0) * 0.5,
        ) {
            let mut paint = Paint::default();
            paint.set_color_rgba8(
                style.red,
                style.green,
                style.blue,
                (style.alpha as f32 * alpha_scale) as u8,
            );
            paint.anti_alias = true;
            pixmap.fill_path(
                &circle,
                &paint,
                tiny_skia::FillRule::Winding,
                Default::default(),
                None,
            );
        }
    } else {
        let mut builder = PathBuilder::new();
        builder.move_to(
            (state.points[0].x - bounds.left) as f32,
            (state.points[0].y - bounds.top) as f32,
        );
        for point in state.points.iter().skip(1) {
            builder.line_to(
                (point.x - bounds.left) as f32,
                (point.y - bounds.top) as f32,
            );
        }
        if let Some(path) = builder.finish() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(
                style.red,
                style.green,
                style.blue,
                (style.alpha as f32 * alpha_scale) as u8,
            );
            paint.anti_alias = true;
            let stroke = Stroke {
                width: style.width.max(1.0),
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Default::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Default::default(), None);
        }
    }

    upload_pixmap(hwnd, &pixmap, bounds)?;
    Ok(true)
}

fn compute_bounds(
    points: &[POINT],
    monitor: MonitorBounds,
    line_width: f32,
) -> anyhow::Result<RECT> {
    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut right = i32::MIN;
    let mut bottom = i32::MIN;

    for point in points {
        left = left.min(point.x);
        top = top.min(point.y);
        right = right.max(point.x);
        bottom = bottom.max(point.y);
    }

    if left == i32::MAX {
        return Err(anyhow!("no trail points available"));
    }

    let margin = line_width.ceil() as i32 + 16;
    Ok(RECT {
        left: (left - margin).max(monitor.left),
        top: (top - margin).max(monitor.top),
        right: (right + margin).min(monitor.right),
        bottom: (bottom + margin).min(monitor.bottom),
    })
}

unsafe fn upload_pixmap(hwnd: HWND, pixmap: &Pixmap, bounds: RECT) -> anyhow::Result<()> {
    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;
    let mut bgra = vec![0u8; pixmap.data().len()];
    for (source, target) in pixmap.data().chunks_exact(4).zip(bgra.chunks_exact_mut(4)) {
        target[0] = source[2];
        target[1] = source[1];
        target[2] = source[0];
        target[3] = source[3];
    }

    let screen_dc = GetDC(None);
    let memory_dc = CreateCompatibleDC(Some(screen_dc));
    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = std::ptr::null_mut::<c_void>();
    let bitmap = CreateDIBSection(
        Some(screen_dc),
        &bitmap_info,
        DIB_RGB_COLORS,
        &mut pixels,
        None,
        0,
    )
    .context("CreateDIBSection failed for overlay surface")?;
    if bitmap.0.is_null() {
        let _ = DeleteDC(memory_dc);
        let _ = ReleaseDC(None, screen_dc);
        return Err(anyhow!("CreateDIBSection failed for overlay surface"));
    }

    std::ptr::copy_nonoverlapping(bgra.as_ptr(), pixels.cast::<u8>(), bgra.len());

    let original = SelectObject(memory_dc, HGDIOBJ(bitmap.0));
    let window_position = POINT {
        x: bounds.left,
        y: bounds.top,
    };
    let bitmap_size = SIZE {
        cx: width,
        cy: height,
    };
    let source_point = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    let _ = SetWindowPos(
        hwnd,
        None,
        bounds.left,
        bounds.top,
        width,
        height,
        SWP_NOACTIVATE,
    );
    UpdateLayeredWindow(
        hwnd,
        Some(screen_dc),
        Some(&window_position),
        Some(&bitmap_size),
        Some(memory_dc),
        Some(&source_point),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    )
    .context("UpdateLayeredWindow failed")?;
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

    let _ = SelectObject(memory_dc, original);
    let _ = DeleteObject(HGDIOBJ(bitmap.0));
    let _ = DeleteDC(memory_dc);
    let _ = ReleaseDC(None, screen_dc);
    Ok(())
}

fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((red, green, blue))
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
