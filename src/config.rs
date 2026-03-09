use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub default_actions: Vec<GestureBinding>,
    #[serde(default)]
    pub app_rules: Vec<ApplicationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    #[serde(default = "default_trail_color")]
    pub trail_color: String,
    #[serde(default = "default_trail_width")]
    pub trail_width: f32,
    #[serde(default = "default_minimum_distance")]
    pub minimum_distance: f32,
    #[serde(default = "default_fade_duration")]
    pub fade_duration_ms: u64,
    #[serde(default)]
    pub autostart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationRule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub process_names: Vec<String>,
    #[serde(default)]
    pub gestures: Vec<GestureBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestureBinding {
    pub gesture: String,
    pub action: GestureAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GestureAction {
    None,
    Hotkey { hotkey: HotkeySpec },
    Shell { command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySpec {
    #[serde(default)]
    pub modifiers: Vec<String>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    dir: PathBuf,
    path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            locale: default_locale(),
            general: GeneralSettings::default(),
            default_actions: vec![GestureBinding {
                gesture: "DR".to_string(),
                action: GestureAction::Hotkey {
                    hotkey: HotkeySpec {
                        modifiers: vec!["Alt".to_string()],
                        key: "Tab".to_string(),
                    },
                },
            }],
            app_rules: vec![
                ApplicationRule {
                    id: "chrome".to_string(),
                    name: "Chrome / Edge".to_string(),
                    process_names: vec!["chrome.exe".to_string(), "msedge.exe".to_string()],
                    gestures: vec![
                        GestureBinding::hotkey("L", &["Alt"], "ArrowLeft"),
                        GestureBinding::hotkey("R", &["Alt"], "ArrowRight"),
                        GestureBinding::hotkey("D", &["Ctrl"], "KeyW"),
                    ],
                },
                ApplicationRule {
                    id: "explorer".to_string(),
                    name: "Explorer".to_string(),
                    process_names: vec!["explorer.exe".to_string()],
                    gestures: vec![
                        GestureBinding::hotkey("L", &["Alt"], "ArrowUp"),
                        GestureBinding::hotkey("R", &["Alt"], "ArrowRight"),
                    ],
                },
                ApplicationRule {
                    id: "vscode".to_string(),
                    name: "Visual Studio Code".to_string(),
                    process_names: vec!["code.exe".to_string()],
                    gestures: vec![
                        GestureBinding::hotkey("L", &["Alt"], "ArrowLeft"),
                        GestureBinding::hotkey("R", &["Alt"], "ArrowRight"),
                    ],
                },
            ],
        }
    }
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            trail_color: default_trail_color(),
            trail_width: default_trail_width(),
            minimum_distance: default_minimum_distance(),
            fade_duration_ms: default_fade_duration(),
            autostart: false,
        }
    }
}

impl GestureBinding {
    pub fn hotkey(gesture: &str, modifiers: &[&str], key: &str) -> Self {
        Self {
            gesture: gesture.to_string(),
            action: GestureAction::Hotkey {
                hotkey: HotkeySpec {
                    modifiers: modifiers.iter().map(|item| (*item).to_string()).collect(),
                    key: key.to_string(),
                },
            },
        }
    }
}

impl AppConfig {
    pub fn normalize(&mut self) {
        self.locale = match self.locale.as_str() {
            "en-US" => "en-US".to_string(),
            _ => "zh-CN".to_string(),
        };
        self.general.trail_width = self.general.trail_width.clamp(1.0, 24.0);
        self.general.minimum_distance = self.general.minimum_distance.clamp(8.0, 120.0);
        self.general.fade_duration_ms = self.general.fade_duration_ms.clamp(60, 2_000);
        self.general.trail_color = normalize_color(&self.general.trail_color);

        for binding in &mut self.default_actions {
            binding.gesture = normalize_gesture(&binding.gesture);
        }

        for rule in &mut self.app_rules {
            rule.process_names = rule
                .process_names
                .iter()
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect();
            for binding in &mut rule.gestures {
                binding.gesture = normalize_gesture(&binding.gesture);
            }
        }

        self.default_actions
            .retain(|binding| !binding.gesture.is_empty());
        self.app_rules.retain(|rule| !rule.process_names.is_empty());
    }

    pub fn resolve_action(&self, process_name: &str, gesture: &str) -> Option<GestureAction> {
        let process_name = process_name.to_ascii_lowercase();
        let gesture = normalize_gesture(gesture);

        for rule in &self.app_rules {
            if rule
                .process_names
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&process_name))
            {
                if let Some(binding) = rule
                    .gestures
                    .iter()
                    .find(|binding| normalize_gesture(&binding.gesture) == gesture)
                {
                    return Some(binding.action.clone());
                }
            }
        }

        self.default_actions
            .iter()
            .find(|binding| normalize_gesture(&binding.gesture) == gesture)
            .map(|binding| binding.action.clone())
    }
}

impl ConfigStore {
    pub fn new() -> anyhow::Result<Self> {
        let base = config_dir().ok_or_else(|| anyhow!("failed to locate %AppData% directory"))?;
        let dir = base.join("Gesto");
        let path = dir.join("config.json");
        Ok(Self { dir, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_create(&self) -> anyhow::Result<AppConfig> {
        fs::create_dir_all(&self.dir).context("failed to create %AppData%/Gesto directory")?;

        if !self.path.exists() {
            let mut config = AppConfig::default();
            config.normalize();
            self.save(&config)?;
            return Ok(config);
        }

        let raw = fs::read_to_string(&self.path).context("failed to read config.json")?;
        let mut parsed: AppConfig =
            serde_json::from_str(&raw).context("failed to parse config.json")?;
        parsed.normalize();
        Ok(parsed)
    }

    pub fn save(&self, config: &AppConfig) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir).context("failed to create config directory")?;
        let json = serde_json::to_string_pretty(config).context("failed to serialize config")?;
        fs::write(&self.path, json).context("failed to write config file")?;
        Ok(())
    }

    pub fn apply_autostart(&self, enabled: bool) -> anyhow::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run_key, _) = hkcu
            .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .context("failed to open Run registry key")?;
        if enabled {
            let executable =
                std::env::current_exe().context("failed to locate current exe path")?;
            let command = format!("\"{}\"", executable.display());
            run_key
                .set_value("Gesto", &command)
                .context("failed to set autostart value")?;
        } else {
            let _ = run_key.delete_value("Gesto");
        }
        Ok(())
    }
}

pub fn normalize_gesture(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| match ch.to_ascii_uppercase() {
            'U' | 'D' | 'L' | 'R' => Some(ch.to_ascii_uppercase()),
            _ => None,
        })
        .collect()
}

fn normalize_color(value: &str) -> String {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() == 6 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        format!("#{}", trimmed.to_ascii_lowercase())
    } else {
        default_trail_color()
    }
}

fn default_version() -> u32 {
    1
}

fn default_locale() -> String {
    "zh-CN".to_string()
}

fn default_trail_color() -> String {
    "#3b82f6".to_string()
}

fn default_trail_width() -> f32 {
    6.0
}

fn default_minimum_distance() -> f32 {
    28.0
}

fn default_fade_duration() -> u64 {
    220
}
