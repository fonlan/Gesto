use std::{
    process::Command,
    thread,
    time::Duration,
};

use anyhow::{Context, anyhow};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_BACK,
    VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_HOME, VK_LEFT, VK_LWIN, VK_MENU,
    VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
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
    let restore_window = if let Some(target_window) = target_window {
        let current_foreground = foreground_window();
        if current_foreground != Some(target_window) {
            activate_window(target_window)
                .context("failed to activate target window for hotkey delivery")?;
            thread::sleep(Duration::from_millis(HOTKEY_TARGET_SETTLE_DELAY_MS));
            current_foreground
        } else {
            None
        }
    } else {
        None
    };

    let result = send_hotkey_inputs(spec);

    if let Some(window) = restore_window {
        schedule_foreground_restore(window);
    }

    result
}

fn schedule_foreground_restore(window: WindowToken) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(HOTKEY_FOREGROUND_RESTORE_DELAY_MS));
        if let Err(error) = activate_window(window) {
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
        inputs.push(keyboard_input(key, Default::default()));
    }

    let key = token_to_vk(&spec.key).ok_or_else(|| anyhow!("unsupported key: {}", spec.key))?;
    inputs.push(keyboard_input(key, Default::default()));
    inputs.push(keyboard_input(key, KEYEVENTF_KEYUP));

    for modifier in modifier_keys.into_iter().rev() {
        inputs.push(keyboard_input(modifier, KEYEVENTF_KEYUP));
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

fn keyboard_input(
    vk: VIRTUAL_KEY,
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

fn modifier_to_vk(token: &str) -> Option<VIRTUAL_KEY> {
    match token {
        "Ctrl" => Some(VK_CONTROL),
        "Alt" => Some(VK_MENU),
        "Shift" => Some(VK_SHIFT),
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