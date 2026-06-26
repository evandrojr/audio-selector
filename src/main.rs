slint::include_modules!();

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::process::Command;
use slint::{ModelRc, VecModel, SharedString, ComponentHandle};
use std::rc::Rc;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use sys_locale::get_locale;
use std::path::PathBuf;

const CONFIG_FILE: &str = "config.json";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Config {
    unified_mode: bool,
    bluetooth_enabled: bool,
    last_sink: Option<String>,
    last_source: Option<String>,
    window_width: Option<f32>,
    window_height: Option<f32>,
    window_x: Option<i32>,
    window_y: Option<i32>,
    filter_enabled: bool,
    filter_words: String,
}

#[derive(Deserialize, Debug, Clone)]
struct PactlDevice {
    name: String,
    description: String,
}

struct Translations {
    title: &'static str,
    bluetooth: &'static str,
    unified: &'static str,
    output: &'static str,
    input: &'static str,
    audio_device: &'static str,
    refresh: &'static str,
    status_ready: &'static str,
    status_bt_on: &'static str,
    status_bt_off: &'static str,
    status_applied: &'static str,
    status_error: &'static str,
    status_connecting: &'static str,
    filter_active: &'static str,
    filter_words: &'static str,
}

const EN: Translations = Translations {
    title: "Audio Selector",
    bluetooth: "Bluetooth",
    unified: "Use same device for input/output",
    output: "Output Device",
    input: "Input Device",
    audio_device: "Audio Device",
    refresh: "Refresh Devices",
    status_ready: "Ready",
    status_bt_on: "Bluetooth ON",
    status_bt_off: "Bluetooth OFF",
    status_applied: "Applied",
    status_error: "Error",
    status_connecting: "Connecting Bluetooth...",
    filter_active: "Enable Device Filter",
    filter_words: "Blacklist (comma separated):",
};

const PT: Translations = Translations {
    title: "Seletor de Áudio",
    bluetooth: "Bluetooth",
    unified: "Mesmo dispositivo para entrada/saída",
    output: "Dispositivo de Saída",
    input: "Dispositivo de Entrada",
    audio_device: "Dispositivo de Áudio",
    refresh: "Atualizar Dispositivos",
    status_ready: "Pronto",
    status_bt_on: "Bluetooth LIGADO",
    status_bt_off: "Bluetooth DESLIGADO",
    status_applied: "Aplicado",
    status_error: "Erro",
    status_connecting: "Conectando Bluetooth...",
    filter_active: "Ativar Filtro de Dispositivos",
    filter_words: "Lista Negra (separada por vírgula):",
};

#[cfg(target_os = "linux")]
fn get_pactl_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    let output = Command::new("pactl").env("LC_ALL", "C").args(["--format=json", "list", target]).output()?;
    if !output.status.success() { return Ok(Vec::new()); }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_start = stdout.find('[').or_else(|| stdout.find('{'));
    let json_str = match json_start { Some(start) => &stdout[start..], None => return Ok(Vec::new()) };
    let devices: Vec<PactlDevice> = serde_json::from_str(json_str.trim()).unwrap_or_default();
    // For sources, we want both hardware inputs AND potentially monitor sources if they are needed, 
    // but usually the user wants actual microphones. 
    // However, we'll keep all non-monitor sources to ensure we don't miss anything.
    Ok(devices.into_iter().filter(|d| !d.name.contains(".monitor") || target == "sinks").collect())
}

#[cfg(not(target_os = "linux"))]
fn get_pactl_devices(_: &str) -> anyhow::Result<Vec<PactlDevice>> { Ok(Vec::new()) }

#[cfg(target_os = "linux")]
fn get_bluetooth_devices() -> Vec<PactlDevice> {
    let output = Command::new("bluetoothctl").arg("devices").output();
    let mut bt_devices = Vec::new();
    if let Ok(o) = output {
        let stdout = String::from_utf8_lossy(&o.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                bt_devices.push(PactlDevice { name: format!("bluez_connect.{}", parts[1]), description: format!("{} (Bluetooth)", parts[2..].join(" ")) });
            }
        }
    }
    bt_devices
}

#[cfg(not(target_os = "linux"))]
fn get_bluetooth_devices() -> Vec<PactlDevice> { Vec::new() }

#[cfg(target_os = "linux")]
fn apply_device_change(target: &str, name: &str) -> anyhow::Result<()> {
    Command::new("pactl").env("LC_ALL", "C").args([if target == "sinks" { "set-default-sink" } else { "set-default-source" }, name]).status()?;
    if target == "sinks" {
        if let Ok(output) = Command::new("pactl").env("LC_ALL", "C").args(["list", "short", "sink-inputs"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(id) = line.split_whitespace().next() { let _ = Command::new("pactl").env("LC_ALL", "C").args(["move-sink-input", id, name]).status(); }
            }
        }
    } else {
        if let Ok(output) = Command::new("pactl").env("LC_ALL", "C").args(["list", "short", "source-outputs"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(id) = line.split_whitespace().next() { let _ = Command::new("pactl").env("LC_ALL", "C").args(["move-source-output", id, name]).status(); }
            }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_device_change(_: &str, _: &str) -> anyhow::Result<()> { Ok(()) }

#[cfg(target_os = "linux")]
fn set_bluetooth_power(on: bool) -> anyhow::Result<()> {
    let _ = Command::new("bluetoothctl").args(["power", if on { "on" } else { "off" }]).status();
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn set_bluetooth_power(_: bool) -> anyhow::Result<()> { Ok(()) }

fn load_config() -> Config {
    if let Ok(content) = fs::read_to_string(CONFIG_FILE) {
        if let Ok(config) = serde_json::from_str(&content) { return config; }
    }
    Config { unified_mode: true, ..Default::default() }
}

fn save_config(config: &Config) {
    if let Ok(content) = serde_json::to_string_pretty(config) { let _ = fs::write(CONFIG_FILE, content); }
}

fn install_app() -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;
    let exe_name = current_exe.file_name().unwrap();
    let home = dirs::home_dir().context("Could not find home directory")?;
    let paths = vec![home.join("bin"), home.join(".local").join("bin"), PathBuf::from("/usr/local/bin")];
    for path in paths {
        if !path.exists() { let _ = fs::create_dir_all(&path); }
        let target = path.join(exe_name);
        match fs::copy(&current_exe, &target) {
            Ok(_) => { println!("Successfully installed to {:?}", target); return Ok(()); }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => println!("Permission denied for {:?}. Try with sudo.", path),
            Err(e) => println!("Failed to copy to {:?}: {}", path, e),
        }
    }
    Err(anyhow::anyhow!("Could not install to any PATH directory."))
}

fn main() -> anyhow::Result<()> {
    if std::env::args().any(|x| x == "-install") { return install_app(); }
    let config_data = load_config();
    let ui = AppWindow::new()?;
    if let (Some(w), Some(h)) = (config_data.window_width, config_data.window_height) { ui.window().set_size(slint::PhysicalSize::new(w as u32, h as u32)); }
    if let (Some(x), Some(y)) = (config_data.window_x, config_data.window_y) { ui.window().set_position(slint::PhysicalPosition::new(x, y)); }
    let ui_weak = ui.as_weak();
    let ui_handle = Arc::new(Mutex::new(ui_weak.clone()));
    let locale = get_locale().unwrap_or_else(|| "en".to_string());
    let t_static = if locale.starts_with("pt") { &PT } else { &EN };
    ui.set_l_title(t_static.title.into()); ui.set_l_bluetooth(t_static.bluetooth.into());
    ui.set_l_unified(t_static.unified.into()); ui.set_l_output(t_static.output.into());
    ui.set_l_input(t_static.input.into()); ui.set_l_audio_device(t_static.audio_device.into());
    ui.set_l_refresh(t_static.refresh.into()); ui.set_l_filter_active(t_static.filter_active.into());
    ui.set_l_filter_words(t_static.filter_words.into());
    #[cfg(target_os = "linux")] ui.set_status(t_static.status_ready.into());
    let config = Arc::new(Mutex::new(config_data));
    {
        let c = config.lock().unwrap();
        ui.set_unified_mode(c.unified_mode); ui.set_bluetooth_enabled(c.bluetooth_enabled);
        ui.set_filter_enabled(c.filter_enabled); ui.set_filter_words(c.filter_words.clone().into());
    }
    if ui.get_bluetooth_enabled() { let _ = set_bluetooth_power(true); }

    let sinks_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new()));
    let sources_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new()));
    let ui_ref = Arc::clone(&ui_handle);
    let config_ref = Arc::clone(&config);
    let s_cache_ref = Arc::clone(&sinks_cache);
    let src_cache_ref = Arc::clone(&sources_cache);
    let refresh_fn = move || {
        let ui = ui_ref.lock().unwrap().unwrap();
        let c = config_ref.lock().unwrap().clone();
        let blacklist: Vec<String> = if c.filter_enabled { c.filter_words.to_lowercase().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() } else { Vec::new() };
        let is_blacklisted = |desc: &str| { let d = desc.to_lowercase(); blacklist.iter().any(|word| d.contains(word)) };
        let mut all_sinks = Vec::new();
        if let Ok(sinks) = get_pactl_devices("sinks") { all_sinks.extend(sinks.into_iter().filter(|d| !is_blacklisted(&d.description))); }
        all_sinks.extend(get_bluetooth_devices().into_iter().filter(|d| !is_blacklisted(&d.description)));
        ui.set_sink_names(ModelRc::from(Rc::new(VecModel::from(all_sinks.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()))));
        if let Some(idx) = c.last_sink.and_then(|last| all_sinks.iter().position(|s| s.name == last)) { ui.set_selected_sink_index(idx as i32); }
        else if !all_sinks.is_empty() { ui.set_selected_sink_index(0); }
        *s_cache_ref.lock().unwrap() = all_sinks;
        if let Ok(sources) = get_pactl_devices("sources") {
            let filtered: Vec<PactlDevice> = sources.into_iter().filter(|d| !is_blacklisted(&d.description)).collect();
            ui.set_source_names(ModelRc::from(Rc::new(VecModel::from(filtered.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()))));
            if let Some(idx) = c.last_source.and_then(|last| filtered.iter().position(|s| s.name == last)) { ui.set_selected_source_index(idx as i32); }
            else if !filtered.is_empty() { ui.set_selected_source_index(0); }
            *src_cache_ref.lock().unwrap() = filtered;
        }
    };
    refresh_fn();
    let r1 = refresh_fn.clone(); ui.on_refresh(r1);
    let config_bt = Arc::clone(&config); ui.on_toggle_bluetooth(move |on| { let _ = set_bluetooth_power(on); let mut c = config_bt.lock().unwrap(); c.bluetooth_enabled = on; save_config(&c); });
    let config_uni = Arc::clone(&config); ui.on_toggle_unified(move |on| { let mut c = config_uni.lock().unwrap(); c.unified_mode = on; save_config(&c); });
    let config_filter = Arc::clone(&config); let r2 = refresh_fn.clone(); ui.on_toggle_filter(move |on| { { let mut c = config_filter.lock().unwrap(); c.filter_enabled = on; save_config(&c); } r2(); });
    let config_words = Arc::clone(&config); let r3 = refresh_fn.clone(); ui.on_filter_words_changed(move |words| { { let mut c = config_words.lock().unwrap(); c.filter_words = words.to_string(); save_config(&c); } r3(); });
    let sinks_change = Arc::clone(&sinks_cache);
    let sources_change = Arc::clone(&sources_cache);
    let config_change = Arc::clone(&config);
    let ui_weak_change = ui_weak.clone();
    let locale_change = locale.clone();
    let handler = move |sink_idx: i32, source_idx: i32| {
        let ui = ui_weak_change.unwrap();
        let t = if locale_change.starts_with("pt") { &PT } else { &EN };
        if sink_idx >= 0 {
            let sinks = sinks_change.lock().unwrap();
            if (sink_idx as usize) < sinks.len() {
                let s_name = sinks[sink_idx as usize].name.clone();
                let unified = ui.get_unified_mode();
                let ui_async = ui_weak_change.clone();
                let src_async = Arc::clone(&sources_change);
                let cfg_async = Arc::clone(&config_change);
                let loc_async = locale_change.clone();
                thread::spawn(move || {
                    let t_async = if loc_async.starts_with("pt") { &PT } else { &EN };
                    let mut actual_name = s_name.clone();
                    #[cfg(target_os = "linux")]
                    if s_name.starts_with("bluez_connect.") {
                        let mac = s_name.replace("bluez_connect.", "");
                        let ui_c = ui_async.clone();
                        let _ = slint::invoke_from_event_loop(move || { ui_c.unwrap().set_status(t_async.status_connecting.into()); });
                        let _ = Command::new("bluetoothctl").args(["connect", &mac]).status();
                        thread::sleep(std::time::Duration::from_millis(2000));
                        if let Ok(p) = get_pactl_devices("sinks") { if let Some(f) = p.iter().find(|s| s.name.contains(&mac.replace(":", "_"))) { actual_name = f.name.clone(); } }
                    }
                    let _ = apply_device_change("sinks", &actual_name);
                    let mut c = cfg_async.lock().unwrap();
                    c.last_sink = Some(actual_name.clone());
                    if unified {
                        let base = actual_name.replace("bluez_sink", "").replace(".a2dp_sink", "").replace(".hifi", "");
                        let sources = src_async.lock().unwrap();
                        for src in sources.iter() { if src.name.contains(&base) { let _ = apply_device_change("sources", &src.name); c.last_source = Some(src.name.clone()); break; } }
                    }
                    save_config(&c);
                    let _ = slint::invoke_from_event_loop(move || { ui_async.unwrap().set_status(format!("{} - {}", t_async.status_applied, chrono::Local::now().format("%H:%M:%S")).into()); });
                });
            }
        }
        if !ui.get_unified_mode() && source_idx >= 0 {
            let sources = sources_change.lock().unwrap();
            if (source_idx as usize) < sources.len() {
                let s_name = sources[source_idx as usize].name.clone();
                let mut c = config_change.lock().unwrap();
                let _ = apply_device_change("sources", &s_name);
                c.last_source = Some(s_name); save_config(&c);
                ui.set_status(format!("{} - {}", t.status_applied, chrono::Local::now().format("%H:%M:%S")).into());
            }
        }
    };
    let h1 = handler.clone(); ui.on_sink_changed(move |idx| h1(idx, -1));
    let h2 = handler.clone(); ui.on_source_changed(move |idx| h2(-1, idx));
    let window = ui.window(); let config_exit = Arc::clone(&config); ui.run()?;
    let mut c = config_exit.lock().unwrap();
    let size = window.size(); c.window_width = Some(size.width as f32); c.window_height = Some(size.height as f32);
    let pos = window.position(); c.window_x = Some(pos.x); c.window_y = Some(pos.y);
    save_config(&c); Ok(())
}
