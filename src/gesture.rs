use std::{collections::VecDeque, sync::Arc, thread};

use anyhow::{Context, anyhow};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
    UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEINPUT, SendInput,
    },
    UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, HC_ACTION, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT,
        PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL, WM_APP,
        WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
    },
};

use crate::{
    actions,
    app::AppContext,
    config::{GestureAction, normalize_gesture},
    logging,
    win::{
        MonitorBounds, WindowToken, ensure_current_thread_per_monitor_dpi_awareness,
        gesture_target_window_for_point, monitor_from_point, monitor_scale_factor,
        process_name_at_point, process_name_for_window,
    },
};

static ENGINE: OnceCell<Arc<GestureEngine>> = OnceCell::new();
static HOOK_THREAD_ID: OnceCell<u32> = OnceCell::new();
static PENDING_ACTIONS: Lazy<Mutex<VecDeque<PendingAction>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

const WM_EXECUTE_GESTURE_ACTION: u32 = WM_APP + 41;

pub fn start_global_hook(context: Arc<AppContext>) -> anyhow::Result<()> {
    let engine = Arc::new(GestureEngine::new(context));
    let _ = ENGINE.set(engine);

    thread::Builder::new()
        .name("gesto-mouse-hook".to_string())
        .spawn(move || {
            if let Err(error) = hook_loop() {
                logging::error(format!("mouse hook error: {error:#}"));
            }
        })
        .context("failed to spawn mouse hook thread")?;

    Ok(())
}

fn hook_loop() -> anyhow::Result<()> {
    unsafe {
        ensure_current_thread_per_monitor_dpi_awareness();
        let _ = HOOK_THREAD_ID.set(GetCurrentThreadId());

        let hinstance = GetModuleHandleW(None).context("failed to load current module handle")?;
        let hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_proc),
            Some(HINSTANCE(hinstance.0)),
            0,
        )
        .map_err(|_| anyhow!("failed to install WH_MOUSE_LL hook"))?;

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            if message.message == WM_EXECUTE_GESTURE_ACTION {
                run_pending_actions();
            }
        }

        UnhookWindowsHookEx(hook).context("failed to uninstall mouse hook")?;
        Ok(())
    }
}

struct GestureEngine {
    context: Arc<AppContext>,
    state: Mutex<GestureState>,
}

#[derive(Default)]
struct GestureState {
    right_button_down: bool,
    gesture_mode: bool,
    minimum_distance: f32,
    start_monitor_bounds: Option<MonitorBounds>,
    start_process_name: Option<String>,
    start_target_window: Option<WindowToken>,
    start_point: Option<windows::Win32::Foundation::POINT>,
    last_point: Option<windows::Win32::Foundation::POINT>,
    direction_anchor: Option<windows::Win32::Foundation::POINT>,
    points: Vec<windows::Win32::Foundation::POINT>,
    directions: String,
}

struct PendingAction {
    action: GestureAction,
    process_name: String,
    directions: String,
    target_window: Option<WindowToken>,
}

impl GestureEngine {
    fn new(context: Arc<AppContext>) -> Self {
        Self {
            context,
            state: Mutex::new(GestureState::default()),
        }
    }

    fn handle_event(&self, message: u32, data: &MSLLHOOKSTRUCT) -> bool {
        if data.flags & (LLMHF_INJECTED as u32) != 0 {
            return false;
        }

        let point = data.pt;

        match message {
            WM_RBUTTONDOWN => {
                if !self.context.gestures_enabled() {
                    return false;
                }

                let base_minimum_distance = self.context.minimum_distance();
                let start_monitor = monitor_from_point(point);
                let minimum_distance = start_monitor
                    .map(|(monitor, _)| base_minimum_distance * monitor_scale_factor(monitor))
                    .unwrap_or(base_minimum_distance);
                let point_process_name = process_name_at_point(point);
                let start_target_window = gesture_target_window_for_point(point);
                let start_process_name = start_target_window.and_then(process_name_for_window);

                if point_process_name
                    .as_deref()
                    .is_some_and(|process_name| self.context.is_process_ignored(process_name))
                    || start_process_name
                        .as_deref()
                        .is_some_and(|process_name| self.context.is_process_ignored(process_name))
                {
                    logging::info(format!(
                        "bypassing gesture interception for ignored process: point={:?}, target={:?}",
                        point_process_name, start_process_name
                    ));
                    return false;
                }

                {
                    let mut state = self.state.lock();
                    state.right_button_down = true;
                    state.gesture_mode = false;
                    state.minimum_distance = minimum_distance;
                    state.start_point = Some(point);
                    state.last_point = Some(point);
                    state.direction_anchor = Some(point);
                    state.start_process_name = start_process_name;
                    state.start_target_window = start_target_window;
                    state.points.clear();
                    state.points.push(point);
                    state.directions.clear();
                    if let Some((_, bounds)) = start_monitor {
                        state.start_monitor_bounds = Some(bounds);
                    } else {
                        state.start_monitor_bounds = None;
                    }
                }

                true
            }
            WM_MOUSEMOVE => {
                let mut state = self.state.lock();
                if !state.right_button_down {
                    return false;
                }

                let start_point = match state.start_point {
                    Some(value) => value,
                    None => return false,
                };
                let minimum_distance = state.minimum_distance.max(8.0);

                let total_distance = euclidean_distance(start_point, point);

                if !state.gesture_mode {
                    if total_distance < minimum_distance {
                        state.last_point = Some(point);
                        return false;
                    }
                    state.gesture_mode = true;
                }

                if state
                    .last_point
                    .map(|last| euclidean_distance(last, point) >= 2.0)
                    .unwrap_or(true)
                {
                    state.points.push(point);
                    state.last_point = Some(point);
                }

                if let Some(anchor) = state.direction_anchor {
                    if euclidean_distance(anchor, point) >= minimum_distance {
                        let direction = dominant_direction(anchor, point);
                        if state.directions.chars().last() != Some(direction) {
                            state.directions.push(direction);
                        }
                        state.direction_anchor = Some(point);
                    }
                }

                if let (Some(bounds), true) = (state.start_monitor_bounds, state.gesture_mode) {
                    self.context
                        .overlay()
                        .show(bounds, &state.points, self.context.trail_style());
                }

                false
            }
            WM_RBUTTONUP => {
                let release = {
                    let mut state = self.state.lock();
                    if !state.right_button_down {
                        return false;
                    }

                    let snapshot = CompletedGesture {
                        gesture_mode: state.gesture_mode,
                        process_name: state.start_process_name.clone().unwrap_or_default(),
                        target_window: state.start_target_window,
                        directions: normalize_gesture(&state.directions),
                    };
                    reset_state(&mut state);

                    if snapshot.gesture_mode {
                        MouseRelease::Gesture(snapshot)
                    } else {
                        MouseRelease::SyntheticClick
                    }
                };

                match release {
                    MouseRelease::Gesture(final_state) => {
                        self.context.overlay().finish();
                        if !final_state.directions.is_empty() {
                            if let Some(action) = self
                                .context
                                .resolve_action(&final_state.process_name, &final_state.directions)
                            {
                                let process_name = final_state.process_name.clone();
                                let directions = final_state.directions.clone();
                                logging::info(format!(
                                    "recognized gesture '{}' for process '{}'",
                                    directions, process_name
                                ));
                                if let Err(error) = queue_pending_action(PendingAction {
                                    action,
                                    process_name,
                                    directions,
                                    target_window: final_state.target_window,
                                }) {
                                    logging::error(format!(
                                        "failed to queue gesture action '{}': {error:#}",
                                        final_state.directions
                                    ));
                                }
                            } else {
                                logging::warn(format!(
                                    "no action resolved for gesture '{}' in process '{}'",
                                    final_state.directions, final_state.process_name
                                ));
                            }
                        }

                        true
                    }
                    MouseRelease::SyntheticClick => {
                        self.context.overlay().hide();
                        if let Err(error) = send_right_click() {
                            logging::error(format!("failed to replay right click: {error:#}"));
                        }
                        true
                    }
                }
            }
            _ => false,
        }
    }
}

#[derive(Default)]
struct CompletedGesture {
    gesture_mode: bool,
    process_name: String,
    target_window: Option<WindowToken>,
    directions: String,
}

enum MouseRelease {
    Gesture(CompletedGesture),
    SyntheticClick,
}

fn queue_pending_action(action: PendingAction) -> anyhow::Result<()> {
    let hook_thread_id = *HOOK_THREAD_ID
        .get()
        .ok_or_else(|| anyhow!("gesture hook thread is not ready"))?;

    PENDING_ACTIONS.lock().push_back(action);
    unsafe {
        PostThreadMessageW(hook_thread_id, WM_EXECUTE_GESTURE_ACTION, WPARAM(0), LPARAM(0))
            .context("failed to post gesture action message")?;
    }

    Ok(())
}

fn run_pending_actions() {
    loop {
        let pending = PENDING_ACTIONS.lock().pop_front();
        let Some(pending) = pending else {
            break;
        };

        if let Err(error) = actions::execute(&pending.action, pending.target_window) {
            logging::error(format!(
                "failed to execute gesture '{}' for process '{}': {error:#}",
                pending.directions, pending.process_name
            ));
        }
    }
}

fn reset_state(state: &mut GestureState) {
    state.right_button_down = false;
    state.gesture_mode = false;
    state.minimum_distance = 0.0;
    state.start_monitor_bounds = None;
    state.start_process_name = None;
    state.start_target_window = None;
    state.start_point = None;
    state.last_point = None;
    state.direction_anchor = None;
    state.points.clear();
    state.directions.clear();
}

fn dominant_direction(
    start: windows::Win32::Foundation::POINT,
    end: windows::Win32::Foundation::POINT,
) -> char {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if dx.abs() >= dy.abs() {
        if dx >= 0 { 'R' } else { 'L' }
    } else if dy >= 0 {
        'D'
    } else {
        'U'
    }
}

fn euclidean_distance(
    start: windows::Win32::Foundation::POINT,
    end: windows::Win32::Foundation::POINT,
) -> f32 {
    let dx = (end.x - start.x) as f32;
    let dy = (end.y - start.y) as f32;
    (dx * dx + dy * dy).sqrt()
}

fn send_right_click() -> anyhow::Result<()> {
    let inputs = [
        mouse_input(MOUSEEVENTF_RIGHTDOWN),
        mouse_input(MOUSEEVENTF_RIGHTUP),
    ];

    send_mouse_inputs(&inputs)
}

fn send_mouse_inputs(inputs: &[INPUT]) -> anyhow::Result<()> {
    unsafe {
        let sent = SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(anyhow!(
                "SendInput sent {} of {} mouse events",
                sent,
                inputs.len()
            ));
        }
    }

    Ok(())
}

fn mouse_input(flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        if let Some(engine) = ENGINE.get() {
            let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            if engine.handle_event(wparam.0 as u32, data) {
                return LRESULT(1);
            }
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}