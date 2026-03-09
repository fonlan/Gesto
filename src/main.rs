#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(not(target_os = "windows"))]
compile_error!("Gesto only supports Windows.");

mod actions;
mod app;
mod config;
mod gesture;
mod http_server;
mod overlay;
mod tray;
mod win;

use anyhow::Context;
use app::AppContext;
use config::ConfigStore;
use win::enable_per_monitor_dpi_awareness;

fn main() -> anyhow::Result<()> {
    enable_per_monitor_dpi_awareness().context("failed to enable per-monitor DPI awareness")?;

    let store = ConfigStore::new().context("failed to prepare config store")?;
    let config = store
        .load_or_create()
        .context("failed to load Gesto config")?;
    store
        .apply_autostart(config.general.autostart)
        .context("failed to sync autostart setting")?;

    let overlay = overlay::OverlayController::spawn().context("failed to start overlay thread")?;
    let context = AppContext::new(store, config, overlay);

    gesture::start_global_hook(context.clone()).context("failed to start global mouse hook")?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;

    let port = runtime
        .block_on(http_server::spawn(context.clone()))
        .context("failed to start local web server")?;
    context.set_port(port);

    tray::run(context).context("tray loop exited unexpectedly")?;
    drop(runtime);
    Ok(())
}
