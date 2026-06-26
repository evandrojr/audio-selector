slint::include_modules!();

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::process::Command;
use slint::{ModelRc, VecModel, SharedString, ComponentHandle, Model};
use std::rc::Rc;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use sys_locale::get_locale;
use std::path::PathBuf;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, Icon,
};

const CONFIG_FILE: &str = "config.json";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
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
    excluded_devices: Vec<String>,
    hide_unknown_bt: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
struct PactlDevice {
    name: String,
    description: String,
    #[serde(default)]
    volume: Option<serde_json::Value>,
}

impl PactlDevice {
    fn get_volume_percent(&self) -> i32 {
        if let Some(vol_obj) = &self.volume {
            if let Some(obj) = vol_obj.as_object() {
                if let Some((_, chan)) = obj.iter().next() {
                    if let Some(pct_str) = chan.get("value_percent").and_then(|v| v.as_str()) {
                        let clean_str = pct_str.replace("%", "").replace(",", ".").trim().to_string();
                        if let Ok(val) = clean_str.parse::<f32>() {
                            return val.round() as i32;
                        }
                    }
                }
            }
        }
        50
    }
}

struct Translations {
    title: &'static str,
    tab_devices: &'static str,
    advanced_options: &'static str,
    hide_unknown_bt: &'static str,
    bluetooth: &'static str,
    unified: &'static str,
    output: &'static str,
    input: &'static str,
    audio_device: &'static str,
    refresh: &'static str,
    status_ready: &'static str,
    status_applied: &'static str,
    status_connecting: &'static str,
    filter_active: &'static str,
    exclude_instruction: &'static str,
    volume: &'static str,
    menu_quit: &'static str,
    open_logs: &'static str,
}

const EN: Translations = Translations {
    title: "Audio Selector", tab_devices: "Devices", advanced_options: "Advanced Options",
    hide_unknown_bt: "Hide unknown Bluetooth devices (MAC addresses)", bluetooth: "Bluetooth",
    unified: "Use same device for input/output", output: "Output Device", input: "Input Device",
    audio_device: "Audio Device", refresh: "Refresh Devices", status_ready: "Ready",
    status_applied: "Applied", status_connecting: "Connecting Bluetooth...",
    filter_active: "Enable Excluded Devices", exclude_instruction: "Check devices below to hide them:",
    volume: "Volume", menu_quit: "Quit", open_logs: "Open Application Logs",
};

const PT: Translations = Translations {
    title: "Seletor de Áudio", tab_devices: "Dispositivos", advanced_options: "Opções Avançadas",
    hide_unknown_bt: "Ocultar dispositivos Bluetooth desconhecidos (MACs)", bluetooth: "Bluetooth",
    unified: "Mesmo dispositivo para entrada/saída", output: "Dispositivo de Saída", input: "Dispositivo de Entrada",
    audio_device: "Dispositivo de Áudio", refresh: "Atualizar Dispositivos", status_ready: "Pronto",
    status_applied: "Aplicado", status_connecting: "Conectando Bluetooth...",
    filter_active: "Ativar Dispositivos Excluídos", exclude_instruction: "Marque os dispositivos abaixo para ocultar:",
    volume: "Volume", menu_quit: "Sair", open_logs: "Abrir Logs da Aplicação",
};

const ES: Translations = Translations {
    title: "Selector de Audio", tab_devices: "Dispositivos", advanced_options: "Opciones Avanzadas",
    hide_unknown_bt: "Ocultar dispositivos Bluetooth desconocidos (MAC)", bluetooth: "Bluetooth",
    unified: "Mismo dispositivo para entrada/salida", output: "Dispositivo de Salida", input: "Dispositivo de Entrada",
    audio_device: "Dispositivo de Audio", refresh: "Actualizar Dispositivos", status_ready: "Listo",
    status_applied: "Aplicado", status_connecting: "Conectando Bluetooth...",
    filter_active: "Activar Dispositivos Excluidos", exclude_instruction: "Marque los dispositivos a continuación:",
    volume: "Volumen", menu_quit: "Salir", open_logs: "Abrir Logs de la Aplicación",
};

const FR: Translations = Translations {
    title: "Sélecteur d'Audio", tab_devices: "Appareils", advanced_options: "Options Avancées",
    hide_unknown_bt: "Masquer les appareils Bluetooth inconnus (MAC)", bluetooth: "Bluetooth",
    unified: "Même appareil pour l'entrée/sortie", output: "Appareil de Sortie", input: "Appareil d'Entrée",
    audio_device: "Appareil Audio", refresh: "Actualiser les Appareils", status_ready: "Prêt",
    status_applied: "Appliqué", status_connecting: "Connexion Bluetooth...",
    filter_active: "Activer Appareils Exclus", exclude_instruction: "Cochez les appareils ci-dessous:",
    volume: "Volume", menu_quit: "Quitter", open_logs: "Ouvrir os Logs de l'Application",
};

const DE: Translations = Translations {
    title: "Audio-Selector", tab_devices: "Geräte", advanced_options: "Erweiterte Optionen",
    hide_unknown_bt: "Unbekannte Bluetooth-Geräte ausblenden (MAC)", bluetooth: "Bluetooth",
    unified: "Gleiches Gerät für Ein-/Ausgabe", output: "Ausgabegerät", input: "Eingabegerät",
    audio_device: "Audiogerät", refresh: "Geräte aktualisieren", status_ready: "Bereit",
    status_applied: "Angewendet", status_connecting: "Bluetooth wird verbunden...",
    filter_active: "Ausgeschlossene Geräte aktivieren", exclude_instruction: "Geräte unten ankreuzen:",
    volume: "Lautstärke", menu_quit: "Beenden", open_logs: "Anwendungsprotokolle öffnen",
};

const IT: Translations = Translations {
    title: "Selettore Audio", tab_devices: "Dispositivi", advanced_options: "Opzioni Avanzate",
    hide_unknown_bt: "Nascondi dispositivos Bluetooth sconosciuti (MAC)", bluetooth: "Bluetooth",
    unified: "Stesso dispositivo per ingresso/uscita", output: "Dispositivo di Uscita", input: "Dispositivo de Ingresso",
    audio_device: "Dispositivo Audio", refresh: "Aggiorna Dispositivi", status_ready: "Pronto",
    status_applied: "Applicato", status_connecting: "Connessione Bluetooth...",
    filter_active: "Abilita Dispositivi Esclusi", exclude_instruction: "Seleziona i dispositivos qui sotto:",
    volume: "Volume", menu_quit: "Esci", open_logs: "Apri i Log dell'Applicazione",
};

#[cfg(target_os = "linux")]
fn get_pactl_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    let output = Command::new("pactl").env("LC_ALL", "C").args(["--format=json", "list", target]).output()?;
    if !output.status.success() { return Ok(Vec::new()); }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_start = stdout.find('[').or_else(|| stdout.find('{'));
    let json_str = match json_start { Some(start) => &stdout[start..], None => return Ok(Vec::new()) };
    let devices: Vec<PactlDevice> = serde_json::from_str(json_str.trim()).unwrap_or_default();
    Ok(devices.into_iter().filter(|d| { if target == "sources" { !d.name.contains(".monitor") || d.name.contains("bluez_source") } else { true } }).collect())
}

#[cfg(not(target_os = "linux"))]
fn get_pactl_devices(_: &str) -> anyhow::Result<Vec<PactlDevice>> { Ok(Vec::new()) }

#[cfg(target_os = "linux")]
fn get_bluetooth_devices() -> Vec<PactlDevice> {
    let output = Command::new("bluetoothctl").arg("devices").output();
    let mut bt = Vec::new();
    if let Ok(o) = output {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 3 { bt.push(PactlDevice { name: format!("bluez_connect.{}", p[1]), description: format!("{} (Bluetooth)", p[2..].join(" ")), volume: None }); }
        }
    }
    bt
}

#[cfg(not(target_os = "linux"))]
fn get_bluetooth_devices() -> Vec<PactlDevice> { Vec::new() }

#[cfg(target_os = "linux")]
fn apply_device_change(target: &str, name: &str) -> anyhow::Result<()> {
    Command::new("pactl").env("LC_ALL", "C").args([if target == "sinks" { "set-default-sink" } else { "set-default-source" }, name]).status()?;
    let cmd = if target == "sinks" { "move-sink-input" } else { "move-source-output" };
    let list_cmd = if target == "sinks" { "sink-inputs" } else { "source-outputs" };
    if let Ok(o) = Command::new("pactl").env("LC_ALL", "C").args(["list", "short", list_cmd]).output() {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            if let Some(id) = line.split_whitespace().next() { let _ = Command::new("pactl").env("LC_ALL", "C").args([cmd, id, name]).status(); }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn apply_device_change(_: &str, _: &str) -> anyhow::Result<()> { Ok(()) }

#[cfg(target_os = "linux")]
fn set_sink_volume(name: &str, vol: i32) -> anyhow::Result<()> {
    let _ = Command::new("pactl").env("LC_ALL", "C").args(["set-sink-volume", name, &format!("{}%", vol)]).status();
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn set_sink_volume(_: &str, _: i32) -> anyhow::Result<()> { Ok(()) }

#[cfg(target_os = "linux")]
fn set_bluetooth_power(on: bool) -> anyhow::Result<()> {
    let _ = Command::new("bluetoothctl").args(["power", if on { "on" } else { "off" }]).status();
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn set_bluetooth_power(_: bool) -> anyhow::Result<()> { Ok(()) }

fn load_tray_icon() -> Icon {
    let img = image::open("ui/assets/icon.png").expect("No icon");
    let img = image::imageops::resize(&img, 64, 64, image::imageops::FilterType::Lanczos3);
    let (w, h) = img.dimensions(); Icon::from_rgba(img.into_raw(), w, h).expect("Tray icon fail")
}

fn load_config() -> Config {
    if let Ok(c) = fs::read_to_string(CONFIG_FILE) { if let Ok(cfg) = serde_json::from_str(&c) { return cfg; } }
    Config { unified_mode: true, ..Default::default() }
}

fn save_config(config: &Config) { if let Ok(c) = serde_json::to_string_pretty(config) { let _ = fs::write(CONFIG_FILE, c); } }

fn install_app() -> anyhow::Result<()> {
    let cur = std::env::current_exe()?; let home = dirs::home_dir().context("No home")?;
    let bin_dir = home.join(".local").join("bin"); if !bin_dir.exists() { fs::create_dir_all(&bin_dir)?; }
    let target_bin = bin_dir.join("audio-selector"); fs::copy(&cur, &target_bin)?;
    let icon_dir = home.join(".local").join("share").join("icons").join("hicolor").join("256x256").join("apps");
    if !icon_dir.exists() { let _ = fs::create_dir_all(&icon_dir); }
    let target_icon = icon_dir.join("audio-selector.png");
    if PathBuf::from("ui/assets/icon.png").exists() { let _ = fs::copy("ui/assets/icon.png", &target_icon); }
    let desktop = format!("[Desktop Entry]\nType=Application\nName=Audio Selector\nExec={}\nIcon={}\nTerminal=false\nCategories=AudioVideo;Audio;Utility;\nStartupNotify=true", target_bin.to_string_lossy(), target_icon.to_string_lossy());
    let app_dir = home.join(".local").join("share").join("applications"); if !app_dir.exists() { fs::create_dir_all(&app_dir)?; }
    fs::write(app_dir.join("audio-selector.desktop"), &desktop)?;
    let autostart = home.join(".config").join("autostart"); if !autostart.exists() { fs::create_dir_all(&autostart)?; }
    fs::write(autostart.join("audio-selector.desktop"), &desktop)?;
    Ok(())
}

use std::io::Read;
fn get_log_content(search: &str) -> String {
    let path = dirs::home_dir().unwrap_or_default().join("audio-selector-debug.log");
    if let Ok(mut f) = std::fs::File::open(&path) {
        let mut c = String::new();
        if f.read_to_string(&mut c).is_ok() {
            if search.is_empty() { let lines: Vec<&str> = c.lines().rev().take(100).collect(); return lines.into_iter().rev().collect::<Vec<&str>>().join("\n"); }
            else { let s = search.to_lowercase(); let f: Vec<&str> = c.lines().filter(|l| l.to_lowercase().contains(&s)).rev().take(100).collect(); return f.into_iter().rev().collect::<Vec<&str>>().join("\n"); }
        }
    }
    "No logs.".to_string()
}

fn append_log(msg: &str) {
    use std::io::Write;
    let path = dirs::home_dir().unwrap_or_default().join("audio-selector-debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) { let _ = f.write_all(format!("{}\n", msg).as_bytes()); }
}

fn main() -> anyhow::Result<()> {
    if std::env::args().any(|x| x == "-install") { return install_app(); }
    let config_data = load_config(); let ui = AppWindow::new()?;
    if let (Some(w), Some(h)) = (config_data.window_width, config_data.window_height) { ui.window().set_size(slint::PhysicalSize::new(w as u32, h as u32)); }
    if let (Some(x), Some(y)) = (config_data.window_x, config_data.window_y) { ui.window().set_position(slint::PhysicalPosition::new(x, y)); }
    let ui_w = ui.as_weak(); let ui_h = Arc::new(Mutex::new(ui_w.clone()));
    let loc = get_locale().unwrap_or_else(|| "en".to_string());
    let t = if loc.starts_with("pt") { &PT } else if loc.starts_with("es") { &ES } else if loc.starts_with("fr") { &FR } else if loc.starts_with("de") { &DE } else if loc.starts_with("it") { &IT } else { &EN };
    ui.set_l_title(t.title.into()); ui.set_l_tab_devices(t.tab_devices.into()); ui.set_l_advanced_options(t.advanced_options.into()); ui.set_l_hide_unknown_bt(t.hide_unknown_bt.into()); ui.set_l_bluetooth(t.bluetooth.into()); ui.set_l_unified(t.unified.into()); ui.set_l_output(t.output.into()); ui.set_l_input(t.input.into()); ui.set_l_audio_device(t.audio_device.into()); ui.set_l_refresh(t.refresh.into()); ui.set_l_filter_active(t.filter_active.into()); ui.set_l_exclude_instruction(t.exclude_instruction.into()); ui.set_l_volume(t.volume.into()); ui.set_l_open_logs(t.open_logs.into());
    #[cfg(target_os = "linux")] ui.set_status(t.status_ready.into());
    let cfg = Arc::new(Mutex::new(config_data));
    { let c = cfg.lock().unwrap(); ui.set_unified_mode(c.unified_mode); ui.set_bluetooth_enabled(c.bluetooth_enabled); ui.set_filter_enabled(c.filter_enabled); ui.set_hide_unknown_bt(c.hide_unknown_bt); }
    if ui.get_bluetooth_enabled() { let _ = set_bluetooth_power(true); }

    #[cfg(target_os = "linux")] { if gtk::init().is_ok() {
        let menu = Menu::new(); let q_i = MenuItem::new(t.menu_quit, true, None); let q_id = q_i.id().clone(); let _ = menu.append_items(&[&q_i]);
        if let Ok(tray_icon) = TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip(t.title).with_icon(load_tray_icon()).build() {
            thread::spawn(move || { let m_c = MenuEvent::receiver(); loop { if let Ok(e) = m_c.recv() { if e.id == q_id { std::process::exit(0); } } } });
            let u_i = ui_w.clone(); thread::spawn(move || { let t_c = tray_icon::TrayIconEvent::receiver(); loop { if let Ok(e) = t_c.recv() { if let tray_icon::TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. } = e { let _ = slint::invoke_from_event_loop(move || { let win = u_i.unwrap().window(); win.show().unwrap(); }); } } } });
            let _ = Box::leak(Box::new(tray_icon));
            let gtk_t = slint::Timer::default(); gtk_t.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(50), move || { while gtk::events_pending() { gtk::main_iteration_do(false); } });
            Box::leak(Box::new(gtk_t));
        }
    }}

    ui.window().on_close_requested(|| { slint::CloseRequestResponse::HideWindow });

    ui.on_open_logs(move || { if let Ok(log_ui) = LogWindow::new() {
        let lw = log_ui.as_weak(); let r_logs = move || { if let Some(ui) = lw.upgrade() { ui.set_log_text(get_log_content(&ui.get_log_search()).into()); } };
        r_logs(); let r_logs_cb = r_logs.clone(); log_ui.on_refresh_logs(move || r_logs_cb());
        log_ui.show().unwrap(); Box::leak(Box::new(log_ui));
    }});

    let s_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new())); let src_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new()));
    let ui_ref = Arc::clone(&ui_h); let config_ref = Arc::clone(&cfg); let s_c_ref = Arc::clone(&s_cache); let src_c_ref = Arc::clone(&src_cache);
    let refresh_fn = move || {
        let u_w = ui_ref.lock().unwrap().clone(); let c = config_ref.lock().unwrap().clone(); let sc = Arc::clone(&s_c_ref); let srcc = Arc::clone(&src_c_ref);
        thread::spawn(move || {
            append_log("Refresh triggered...");
            let bt = get_bluetooth_devices(); let mut rs = Vec::new(); if let Ok(s) = get_pactl_devices("sinks") { rs.extend(s); }
            for b in bt.iter() { let m = b.name.replace("bluez_connect.", "").replace(":", "_").to_lowercase(); if !rs.iter().any(|s| s.name.to_lowercase().contains(&m)) { rs.push(b.clone()); } }
            let mut rsrc = Vec::new(); if let Ok(s) = get_pactl_devices("sources") { rsrc.extend(s); }
            for b in bt.iter() { let m = b.name.replace("bluez_connect.", "").replace(":", "_").to_lowercase(); if !rsrc.iter().any(|s| s.name.to_lowercase().contains(&m)) { rsrc.push(b.clone()); } }
            let h_u = c.hide_unknown_bt; let is_u = |desc: &str| { if !h_u { return false; } let d = desc.to_uppercase(); let b = d.replace(" (BLUETOOTH)", "").trim().to_string(); b.len() == 17 && (b.chars().filter(|c| *c == '-').count() == 5 || b.chars().filter(|c| *c == ':').count() == 5) };
            let excl = c.excluded_devices.clone(); let f_e = c.filter_enabled;
            let fsinks: Vec<PactlDevice> = rs.into_iter().filter(|d| (!f_e || !excl.contains(&d.name)) && !is_u(&d.description)).collect();
            let fsrcs: Vec<PactlDevice> = rsrc.into_iter().filter(|d| (!f_e || !excl.contains(&d.name)) && !is_u(&d.description)).collect(); let excl2 = excl.clone();
            let mut all_u: Vec<PactlDevice> = Vec::new(); for d in fsinks.iter().chain(fsrcs.iter()) { if !all_u.iter().any(|u| u.name == d.name) { all_u.push(d.clone()); } }
            let ls = c.last_sink.clone(); let lsrc = c.last_source.clone();
            let _ = slint::invoke_from_event_loop(move || { if let Some(ui) = u_w.upgrade() {
                ui.set_all_devices(ModelRc::from(Rc::new(VecModel::from(all_u.iter().map(|d| crate::DeviceToggle { name: d.name.clone().into(), description: d.description.clone().into(), excluded: excl2.contains(&d.name) }).collect::<Vec<_>>()))));
                ui.set_sink_names(ModelRc::from(Rc::new(VecModel::from(fsinks.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()))));
                if let Some(idx) = ls.and_then(|l| fsinks.iter().position(|s| s.name == l)) { ui.set_selected_sink_index(idx as i32); ui.set_sink_volume(fsinks[idx].get_volume_percent()); }
                else if !fsinks.is_empty() { ui.set_selected_sink_index(0); ui.set_sink_volume(fsinks[0].get_volume_percent()); }
                ui.set_source_names(ModelRc::from(Rc::new(VecModel::from(fsrcs.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()))));
                if let Some(idx) = lsrc.and_then(|l| fsrcs.iter().position(|s| s.name == l)) { ui.set_selected_source_index(idx as i32); }
                else if !fsrcs.is_empty() { ui.set_selected_source_index(0); }
                *sc.lock().unwrap() = fsinks; *srcc.lock().unwrap() = fsrcs;
                append_log("Refresh UI update done.");
            }});
        });
    };
    refresh_fn(); ui.on_refresh(refresh_fn.clone());
    let c_bt = Arc::clone(&cfg); ui.on_toggle_bluetooth(move |on| { let _ = set_bluetooth_power(on); let mut c = c_bt.lock().unwrap(); c.bluetooth_enabled = on; save_config(&c); });
    let c_uni = Arc::clone(&cfg); ui.on_toggle_unified(move |on| { let mut c = c_uni.lock().unwrap(); c.unified_mode = on; save_config(&c); });
    let c_f = Arc::clone(&cfg); let r2 = refresh_fn.clone(); ui.on_toggle_filter(move |on| { { let mut c = c_f.lock().unwrap(); c.filter_enabled = on; save_config(&c); } r2(); });
    let c_h = Arc::clone(&cfg); let r_h = refresh_fn.clone(); ui.on_toggle_hide_unknown_bt(move |on| { { let mut c = c_h.lock().unwrap(); c.hide_unknown_bt = on; save_config(&c); } r_h(); });
    let c_e = Arc::clone(&cfg); let r_e = refresh_fn.clone(); ui.on_toggle_device_exclusion(move |n, e| { let ns = n.to_string(); { let mut c = c_e.lock().unwrap(); if e { if !c.excluded_devices.contains(&ns) { c.excluded_devices.push(ns); } } else { c.excluded_devices.retain(|x| x != &ns); } save_config(&c); } r_e(); });
    let sc_c = Arc::clone(&s_cache); let src_c = Arc::clone(&src_cache); let c_c = Arc::clone(&cfg); let u_c = ui_h.clone();
    let handler = move |s_i: i32, src_i: i32| {
        let u = u_c.lock().unwrap().clone().upgrade().unwrap(); let t = if get_locale().unwrap_or_default().starts_with("pt") { &PT } else { &EN };
        if s_i >= 0 {
            let sks = sc_c.lock().unwrap(); if (s_i as usize) < sks.len() {
                let n = sks[s_i as usize].name.clone(); let u_a = u_c.lock().unwrap().clone(); let sr_a = Arc::clone(&src_c); let cf_a = Arc::clone(&c_c); let loc = get_locale().unwrap_or_default(); let u_a2 = u_a.clone(); let u_a3 = u_a.clone();
                u.set_sink_volume(sks[s_i as usize].get_volume_percent());
                thread::spawn(move || {
                    let ta = if loc.starts_with("pt") { &PT } else { &EN }; let mut an = n.clone();
                    if n.starts_with("bluez_connect.") {
                        let mac = n.replace("bluez_connect.", ""); let _ = slint::invoke_from_event_loop(move || { u_a2.upgrade().unwrap().set_status(ta.status_connecting.into()); });
                        let _ = Command::new("bluetoothctl").args(["connect", &mac]).status(); thread::sleep(std::time::Duration::from_millis(2000));
                        if let Ok(p) = get_pactl_devices("sinks") { if let Some(f) = p.iter().find(|s| s.name.contains(&mac.replace(":", "_"))) { an = f.name.clone(); } }
                    }
                    let _ = apply_device_change("sinks", &an); let mut c = cf_a.lock().unwrap(); c.last_sink = Some(an.clone());
                    if u_a.upgrade().unwrap().get_unified_mode() { let base = an.replace("bluez_sink", "").replace(".a2dp_sink", "").replace(".hifi", ""); let src = sr_a.lock().unwrap(); for s in src.iter() { if s.name.contains(&base) { let _ = apply_device_change("sources", &s.name); c.last_source = Some(s.name.clone()); break; } } }
                    save_config(&c); let _ = slint::invoke_from_event_loop(move || { u_a3.upgrade().unwrap().set_status(format!("{} - {}", ta.status_applied, chrono::Local::now().format("%H:%M:%S")).into()); });
                });
            }
        }
        if !u.get_unified_mode() && src_i >= 0 {
            let srcs = src_c.lock().unwrap(); if (src_i as usize) < srcs.len() {
                let n = srcs[src_i as usize].name.clone(); let cf_a = Arc::clone(&c_c); let u_a = u_c.lock().unwrap().clone(); let loc = get_locale().unwrap_or_default(); let u_a2 = u_a.clone();
                thread::spawn(move || {
                    let ta = if loc.starts_with("pt") { &PT } else { &EN }; let mut an = n.clone();
                    if n.starts_with("bluez_connect.") {
                        let mac = n.replace("bluez_connect.", ""); let _ = slint::invoke_from_event_loop(move || { u_a2.upgrade().unwrap().set_status(ta.status_connecting.into()); });
                        let _ = Command::new("bluetoothctl").args(["connect", &mac]).status(); thread::sleep(std::time::Duration::from_millis(2000));
                        if let Ok(p) = get_pactl_devices("sources") { if let Some(f) = p.iter().find(|s| s.name.contains(&mac.replace(":", "_"))) { an = f.name.clone(); } }
                    }
                    let _ = apply_device_change("sources", &an); let mut c = cf_a.lock().unwrap(); c.last_source = Some(an); save_config(&c);
                    let _ = slint::invoke_from_event_loop(move || { u_a.upgrade().unwrap().set_status(format!("{} - {}", ta.status_applied, chrono::Local::now().format("%H:%M:%S")).into()); });
                });
            }
        }
    };
    let h1 = handler.clone(); ui.on_sink_changed(move |idx| h1(idx, -1)); let h2 = handler.clone(); ui.on_source_changed(move |idx| h2(-1, idx));
    let s_vol = Arc::clone(&s_cache); let u_v = Arc::clone(&ui_h); ui.on_sink_volume_changed(move |v| { let u = u_v.lock().unwrap().upgrade().unwrap(); let idx = u.get_selected_sink_index(); if idx >= 0 { let s = s_vol.lock().unwrap(); if (idx as usize) < s.len() { let n = &s[idx as usize].name; if !n.starts_with("bluez_connect.") { let _ = set_sink_volume(n, v); } } } });
    let win = ui.window(); let c_ex = Arc::clone(&cfg); ui.run()?;
    let mut c = c_ex.lock().unwrap(); let sz = win.size(); c.window_width = Some(sz.width as f32); c.window_height = Some(sz.height as f32); let p = win.position(); c.window_x = Some(p.x); c.window_y = Some(p.y); save_config(&c); Ok(())
}
