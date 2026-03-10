use anyhow::Context;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND},
        System::Threading::CreateMutexW,
        UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW},
    },
    core::PCWSTR,
};

use crate::{logging, tray, win::to_wide};

const SINGLE_INSTANCE_MUTEX: &str = "Local\\Gesto.Singleton";

pub enum InstanceState {
    Primary(SingleInstanceGuard),
    Secondary,
}

pub struct SingleInstanceGuard {
    handle: HANDLE,
}

pub fn acquire() -> anyhow::Result<InstanceState> {
    let mutex_name = to_wide(SINGLE_INSTANCE_MUTEX);

    unsafe {
        let handle = CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr()))
            .context("failed to create single-instance mutex")?;

        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            logging::warn("detected an existing Gesto instance");

            if tray::notify_existing_instance_open_config() {
                logging::info("forwarded open-config request to existing instance");
            } else {
                logging::warn("existing instance found, but tray window was not ready");
                show_already_running_notice();
            }

            return Ok(InstanceState::Secondary);
        }

        logging::info("single-instance mutex acquired");
        Ok(InstanceState::Primary(SingleInstanceGuard { handle }))
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

fn show_already_running_notice() {
    let title = to_wide("Gesto");
    let message = to_wide("Gesto is already running.\n已在运行。");

    unsafe {
        let _ = MessageBoxW(
            Some(HWND::default()),
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
