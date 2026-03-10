#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(not(target_os = "windows"))]
compile_error!("Gesto only supports Windows.");

mod actions;
mod app;
mod config;
mod gesture;
mod http_server;
mod logging;
mod overlay;
mod tray;
mod win;

use anyhow::Context;
use app::AppContext;
use config::ConfigStore;
use win::enable_per_monitor_dpi_awareness;

fn main() -> anyhow::Result<()> {
    let store = ConfigStore::new().context("failed to prepare config store")?;
    logging::init(store.logs_dir()).context("failed to initialize logger")?;
    logging::install_panic_hook();

    if let Err(error) = run(store) {
        logging::error(format!("fatal application error: {error:#}"));
        return Err(error);
    }

    Ok(())
}

fn run(store: ConfigStore) -> anyhow::Result<()> {
    logging::info("starting Gesto");
    enable_per_monitor_dpi_awareness().context("failed to enable per-monitor DPI awareness")?;

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
    logging::info(format!("local web server listening on http://127.0.0.1:{port}"));

    tray::run(context).context("tray loop exited unexpectedly")?;
    logging::info("tray loop exited");
    drop(runtime);
    Ok(())
}
