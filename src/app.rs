use std::sync::Arc;

use anyhow::Context;
use parking_lot::RwLock;

use crate::{
    config::{AppConfig, ConfigStore, GestureAction},
    logging,
    overlay::{OverlayController, TrailStyle},
};

pub struct AppContext {
    store: ConfigStore,
    config: RwLock<AppConfig>,
    overlay: OverlayController,
    port: RwLock<u16>,
}

impl AppContext {
    pub fn new(store: ConfigStore, config: AppConfig, overlay: OverlayController) -> Arc<Self> {
        Arc::new(Self {
            store,
            config: RwLock::new(config),
            overlay,
            port: RwLock::new(0),
        })
    }

    pub fn config_snapshot(&self) -> AppConfig {
        self.config.read().clone()
    }

    pub fn locale(&self) -> String {
        self.config.read().locale.clone()
    }

    pub fn gestures_enabled(&self) -> bool {
        self.config.read().gestures_enabled()
    }

    pub fn set_gestures_enabled(&self, enabled: bool) -> anyhow::Result<AppConfig> {
        let mut updated = self.config_snapshot();
        updated.general.gestures_enabled = enabled;
        self.save_config(updated)
    }

    pub fn save_config(&self, mut updated: AppConfig) -> anyhow::Result<AppConfig> {
        updated.normalize();
        self.store
            .save(&updated)
            .context("failed to persist config file")?;
        self.store
            .apply_autostart(updated.general.autostart)
            .context("failed to update autostart registry value")?;
        *self.config.write() = updated.clone();
        logging::info("configuration saved");
        Ok(updated)
    }

    pub fn set_port(&self, port: u16) {
        *self.port.write() = port;
    }

    pub fn port(&self) -> u16 {
        *self.port.read()
    }

    pub fn server_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port())
    }

    pub fn config_path(&self) -> String {
        self.store.path().display().to_string()
    }

    pub fn overlay(&self) -> OverlayController {
        self.overlay.clone()
    }

    pub fn trail_style(&self) -> TrailStyle {
        let config = self.config.read();
        TrailStyle::from_general(&config.general)
    }

    pub fn resolve_action(&self, process_name: &str, gesture: &str) -> Option<GestureAction> {
        let config = self.config.read();
        config.resolve_action(process_name, gesture)
    }

    pub fn is_process_ignored(&self, process_name: &str) -> bool {
        self.config.read().is_process_ignored(process_name)
    }

    pub fn minimum_distance(&self) -> f32 {
        self.config.read().general.minimum_distance.max(8.0)
    }
}
