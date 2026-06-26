use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use crate::utils::get_config_path;

fn write_queue() -> &'static Sender<String> {
    static QUEUE: OnceLock<Sender<String>> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let (tx, rx) = channel::<String>();
        std::thread::spawn(move || {
            for data in rx {
                let _ = fs::write(get_config_path(), &data);
            }
        });
        tx
    })
}

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
    if let Ok(c) = serde_json::to_string_pretty(config) {
        let _ = write_queue().send(c);
    }
}
