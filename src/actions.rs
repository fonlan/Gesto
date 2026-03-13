use std::{process::Command, thread, time::Duration};

use anyhow::{Context, anyhow};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1,
    VK_HOME, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_NEXT, VK_PRIOR, VK_RETURN,
    VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
};

use crate::{
    config::{GestureAction, HotkeySpec},
    logging,
    win::{WindowToken, activate_window, foreground_window},
};

const HOTKEY_TARGET_SETTLE_DELAY_MS: u64 = 40;
const HOTKEY_FOREGROUND_RESTORE_DELAY_MS: u64 = 150;

pub fn execute(action: &GestureAction, target_window: Option<WindowToken>) -> anyhow::Result<()> {
    match action {
        GestureAction::None => Ok(()),
        GestureAction::Shell { command } => {
            Command::new("cmd")
                .args(["/C", command])
                .spawn()
                .with_context(|| format!("failed to spawn shell command: {}", command))?;
            logging::info(format!("spawned shell action: {}", command));
            Ok(())
        }
        GestureAction::Hotkey { hotkey } => {
            send_hotkey(hotkey, target_window)?;
            logging::info(format!(
                "sent hotkey action: {}+{}",
                hotkey.modifiers.join("+"),
                hotkey.key
            ));
            Ok(())
        }
    }
}

fn send_hotkey(spec: &HotkeySpec, target_window: Option<WindowToken>) -> anyhow::Result<()> {
    let restore_request = if let Some(target_window) = target_window {
        let current_foreground = foreground_window();
        if current_foreground != Some(target_window) {
            activate_window(target_window)
                .context("failed to activate target window for hotkey delivery")?;
            thread::sleep(Duration::from_millis(HOTKEY_TARGET_SETTLE_DELAY_MS));
            current_foreground.map(|previous_window| (previous_window, target_window))
        } else {
            None
        }
    } else {
        None
    };

    let result = send_hotkey_inputs(spec);

    if let Some((previous_window, activated_target_window)) = restore_request {
        schedule_foreground_restore(previous_window, activated_target_window);
    }

    result
}

fn schedule_foreground_restore(previous_window: WindowToken, activated_target_window: WindowToken) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(HOTKEY_FOREGROUND_RESTORE_DELAY_MS));
        let current_foreground = foreground_window();
        if current_foreground != Some(activated_target_window) {
            logging::info(format!(
                "skip restoring previous foreground window because current foreground changed away from target: target={:?} current={:?}",
                activated_target_window, current_foreground
            ));
            return;
        }

        if let Err(error) = activate_window(previous_window) {
            logging::warn(format!(
                "failed to restore previous foreground window after hotkey: {error:#}"
            ));
        }
    });
}

fn send_hotkey_inputs(spec: &HotkeySpec) -> anyhow::Result<()> {
    let mut inputs = Vec::new();
    let mut modifier_keys = Vec::new();

    for modifier in &spec.modifiers {
        let key = modifier_to_vk(modifier)
            .ok_or_else(|| anyhow!("unsupported modifier: {}", modifier))?;
        modifier_keys.push(key);
        inputs.push(keyboard_input(key, false));
    }

    let key = token_to_vk(&spec.key).ok_or_else(|| anyhow!("unsupported key: {}", spec.key))?;
    inputs.push(keyboard_input(key, false));
    inputs.push(keyboard_input(key, true));

    for modifier in modifier_keys.into_iter().rev() {
        inputs.push(keyboard_input(modifier, true));
    }

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(anyhow!(
                "SendInput sent {} of {} events",
                sent,
                inputs.len()
            ));
        }
    }

    Ok(())
}

fn keyboard_input(vk: VIRTUAL_KEY, is_key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: keyboard_event_flags(vk, is_key_up),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn keyboard_event_flags(vk: VIRTUAL_KEY, is_key_up: bool) -> KEYBD_EVENT_FLAGS {
    let mut flags = 0;

    if is_extended_key(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY.0;
    }

    if is_key_up {
        flags |= KEYEVENTF_KEYUP.0;
    }

    KEYBD_EVENT_FLAGS(flags)
}

fn is_extended_key(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_DELETE | VK_END | VK_HOME | VK_LEFT | VK_NEXT | VK_PRIOR | VK_RIGHT | VK_UP | VK_DOWN
    )
}

fn modifier_to_vk(token: &str) -> Option<VIRTUAL_KEY> {
    match token {
        "Ctrl" => Some(VK_LCONTROL),
        "Alt" => Some(VK_LMENU),
        "Shift" => Some(VK_LSHIFT),
        "Win" => Some(VK_LWIN),
        _ => None,
    }
}

fn token_to_vk(token: &str) -> Option<VIRTUAL_KEY> {
    match token {
        "ArrowLeft" => Some(VK_LEFT),
        "ArrowRight" => Some(VK_RIGHT),
        "ArrowUp" => Some(VK_UP),
        "ArrowDown" => Some(VK_DOWN),
        "Enter" => Some(VK_RETURN),
        "Tab" => Some(VK_TAB),
        "Space" => Some(VK_SPACE),
        "Backspace" => Some(VK_BACK),
        "Delete" => Some(VK_DELETE),
        "Escape" => Some(VK_ESCAPE),
        "Home" => Some(VK_HOME),
        "End" => Some(VK_END),
        "PageUp" => Some(VK_PRIOR),
        "PageDown" => Some(VK_NEXT),
        _ if token.starts_with("Key") && token.len() == 4 => {
            let ch = token.chars().nth(3)?;
            if ch.is_ascii_alphabetic() {
                Some(VIRTUAL_KEY(ch.to_ascii_uppercase() as u16))
            } else {
                None
            }
        }
        _ if token.starts_with("Digit") && token.len() == 6 => {
            let ch = token.chars().nth(5)?;
            if ch.is_ascii_digit() {
                Some(VIRTUAL_KEY(ch as u16))
            } else {
                None
            }
        }
        _ if token.starts_with('F') => {
            let number = token[1..].parse::<u16>().ok()?;
            if (1..=24).contains(&number) {
                Some(VIRTUAL_KEY(VK_F1.0 + number - 1))
            } else {
                None
            }
        }
        _ if token.len() == 1 => {
            let ch = token.chars().next()?.to_ascii_uppercase();
            if ch.is_ascii_alphanumeric() {
                Some(VIRTUAL_KEY(ch as u16))
            } else {
                None
            }
        }
        _ => None,
    }
}
