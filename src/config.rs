use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;
use crate::utils::get_config_path;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct CachedDevice {
    pub name: String,
    pub description: String,
    pub volume_percent: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub unified_mode: bool,
    pub bluetooth_enabled: bool,
    pub last_sink: Option<String>,
    pub last_source: Option<String>,
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub filter_enabled: bool,
    pub excluded_devices: Vec<String>,
    pub hide_unknown_bt: bool,
    pub cached_sinks: Vec<CachedDevice>,
    pub cached_sources: Vec<CachedDevice>,
}

pub fn load_config() -> Config {
    if let Ok(c) = fs::read_to_string(get_config_path()) {
        if let Ok(cfg) = serde_json::from_str(&c) { return cfg; }
    }
    Config { unified_mode: true, ..Default::default() }
}

pub fn save_config(config: &Config) {
    let serialized = serde_json::to_string_pretty(config);
    if let Ok(c) = serialized {
        let _lock = WRITE_LOCK.lock().unwrap();
        let _ = fs::write(get_config_path(), c);
    }
}
