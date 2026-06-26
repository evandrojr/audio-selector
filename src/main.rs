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
                        // The string might be "100%" or "100,00%" or "100.00%"
                        let clean_str = pct_str.replace("%", "").replace(",", ".").trim().to_string();
                        // Parse as f32 first to handle decimals like "50.00", then convert to i32
                        if let Ok(val) = clean_str.parse::<f32>() {
                            return val.round() as i32;
                        }
                    }
                }
            }
        }
        50 // Fallback
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
    status_bt_on: &'static str,
    status_bt_off: &'static str,
    status_applied: &'static str,
    status_error: &'static str,
    status_connecting: &'static str,
    status_unsupported: &'static str,
    filter_active: &'static str,
    exclude_instruction: &'static str,
    volume: &'static str,
    menu_quit: &'static str,
}

const EN: Translations = Translations {
    title: "Audio Selector",
    tab_devices: "Devices",
    advanced_options: "Advanced Options",
    hide_unknown_bt: "Hide unknown Bluetooth devices (MAC addresses)",
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
    status_unsupported: "Operating System not fully supported for switching yet.",
    filter_active: "Enable Excluded Devices",
    exclude_instruction: "Check devices below to hide them from the main list:",
    volume: "Volume",
    menu_quit: "Quit",
};

const PT: Translations = Translations {
    title: "Seletor de Áudio",
    tab_devices: "Dispositivos",
    advanced_options: "Opções Avançadas",
    hide_unknown_bt: "Ocultar dispositivos Bluetooth desconhecidos (Endereços MAC)",
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
    status_unsupported: "Sistema operacional ainda não suportado para chaveamento.",
    filter_active: "Ativar Dispositivos Excluídos",
    exclude_instruction: "Marque os dispositivos abaixo para ocultá-los da lista principal:",
    volume: "Volume",
    menu_quit: "Sair",
};

const ES: Translations = Translations {
    title: "Selector de Audio",
    tab_devices: "Dispositivos",
    advanced_options: "Opciones Avanzadas",
    hide_unknown_bt: "Ocultar dispositivos Bluetooth desconocidos (Direcciones MAC)",
    bluetooth: "Bluetooth",
    unified: "Mismo dispositivo para entrada/salida",
    output: "Dispositivo de Salida",
    input: "Dispositivo de Entrada",
    audio_device: "Dispositivo de Audio",
    refresh: "Actualizar Dispositivos",
    status_ready: "Listo",
    status_bt_on: "Bluetooth ACTIVADO",
    status_bt_off: "Bluetooth DESACTIVADO",
    status_applied: "Aplicado",
    status_error: "Error",
    status_connecting: "Conectando Bluetooth...",
    status_unsupported: "Sistema operativo aún no soportado para conmutación.",
    filter_active: "Activar Dispositivos Excluidos",
    exclude_instruction: "Marque los dispositivos a continuación para ocultarlos de la lista principal:",
    volume: "Volumen",
    menu_quit: "Salir",
};

const FR: Translations = Translations {
    title: "Sélecteur d'Audio",
    tab_devices: "Appareils",
    advanced_options: "Options Avancées",
    hide_unknown_bt: "Masquer les appareils Bluetooth inconnus (Adresses MAC)",
    bluetooth: "Bluetooth",
    unified: "Même appareil pour l'entrée/sortie",
    output: "Appareil de Sortie",
    input: "Appareil d'Entrée",
    audio_device: "Appareil Audio",
    refresh: "Actualiser les Appareils",
    status_ready: "Prêt",
    status_bt_on: "Bluetooth ACTIVÉ",
    status_bt_off: "Bluetooth DÉSACTIVÉ",
    status_applied: "Appliqué",
    status_error: "Erreur",
    status_connecting: "Connexion Bluetooth...",
    status_unsupported: "Système d'exploitation non encore supporté.",
    filter_active: "Activer Appareils Exclus",
    exclude_instruction: "Cochez les appareils ci-dessous pour les masquer de la liste principale :",
    volume: "Volume",
    menu_quit: "Quitter",
};

const DE: Translations = Translations {
    title: "Audio-Selector",
    tab_devices: "Geräte",
    advanced_options: "Erweiterte Optionen",
    hide_unknown_bt: "Unbekannte Bluetooth-Geräte ausblenden (MAC-Adressen)",
    bluetooth: "Bluetooth",
    unified: "Gleiches Gerät für Ein-/Ausgabe",
    output: "Ausgabegerät",
    input: "Eingabegerät",
    audio_device: "Audiogerät",
    refresh: "Geräte aktualisieren",
    status_ready: "Bereit",
    status_bt_on: "Bluetooth EIN",
    status_bt_off: "Bluetooth AUS",
    status_applied: "Angewendet",
    status_error: "Fehler",
    status_connecting: "Bluetooth wird verbunden...",
    status_unsupported: "Betriebssystem wird noch nicht unterstützt.",
    filter_active: "Ausgeschlossene Geräte aktivieren",
    exclude_instruction: "Geräte unten ankreuzen, um sie aus der Hauptliste auszublenden:",
    volume: "Lautstärke",
    menu_quit: "Beenden",
};

const IT: Translations = Translations {
    title: "Selettore Audio",
    tab_devices: "Dispositivi",
    advanced_options: "Opzioni Avanzate",
    hide_unknown_bt: "Nascondi dispositivi Bluetooth sconosciuti (Indirizzi MAC)",
    bluetooth: "Bluetooth",
    unified: "Stesso dispositivo per ingresso/uscita",
    output: "Dispositivo di Uscita",
    input: "Dispositivo di Ingresso",
    audio_device: "Dispositivo Audio",
    refresh: "Aggiorna Dispositivi",
    status_ready: "Pronto",
    status_bt_on: "Bluetooth ATTIVO",
    status_bt_off: "Bluetooth NON ATTIVO",
    status_applied: "Applicato",
    status_error: "Errore",
    status_connecting: "Connessione Bluetooth...",
    status_unsupported: "Sistema operativo non ancora supportato.",
    filter_active: "Abilita Dispositivi Esclusi",
    exclude_instruction: "Seleziona i dispositivi qui sotto per nasconderli dalla lista principale:",
    volume: "Volume",
    menu_quit: "Esci",
};

#[cfg(target_os = "linux")]
fn get_pactl_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    let output = Command::new("pactl").env("LC_ALL", "C").args(["--format=json", "list", target]).output()?;
    if !output.status.success() { return Ok(Vec::new()); }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_start = stdout.find('[').or_else(|| stdout.find('{'));
    let json_str = match json_start { Some(start) => &stdout[start..], None => return Ok(Vec::new()) };
    let devices: Vec<PactlDevice> = serde_json::from_str(json_str.trim()).unwrap_or_default();
    
    Ok(devices.into_iter().filter(|d| {
        if target == "sources" {
            !d.name.contains(".monitor") || d.name.contains("bluez_source")
        } else {
            true
        }
    }).collect())
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
                let mac = parts[1];
                let name = parts[2..].join(" ");
                bt_devices.push(PactlDevice { name: format!("bluez_connect.{}", mac), description: format!("{} (Bluetooth)", name), volume: None });
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
fn set_sink_volume(name: &str, vol: i32) -> anyhow::Result<()> {
    let _ = Command::new("pactl").env("LC_ALL", "C").args(["set-sink-volume", name, &format!("{}%", vol)]).status();
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn set_sink_volume(_: &str, _: i32) -> anyhow::Result<()> { Ok(()) }

#[cfg(target_os = "linux")]
fn check_bluetooth_hardware() -> bool {
    Command::new("bluetoothctl").arg("show").output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).contains("No default controller available"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn check_bluetooth_hardware() -> bool { false }

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
    let home = dirs::home_dir().context("Could not find home directory")?;
    
    let bin_dir = home.join(".local").join("bin");
    if !bin_dir.exists() { fs::create_dir_all(&bin_dir)?; }
    let target_bin = bin_dir.join("audio-selector");
    fs::copy(&current_exe, &target_bin).context("Failed to copy binary. You might need sudo if permissions are denied.")?;
    
    let icon_dir = home.join(".local").join("share").join("icons").join("hicolor").join("256x256").join("apps");
    if !icon_dir.exists() { let _ = fs::create_dir_all(&icon_dir); }
    let target_icon = icon_dir.join("audio-selector.png");
    if PathBuf::from("ui/assets/icon.png").exists() {
        let _ = fs::copy("ui/assets/icon.png", &target_icon);
    }

    let desktop_content = format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Name=Audio Selector\n\
        Comment=Audio Device Selector & Bluetooth Manager\n\
        Exec={}\n\
        Icon={}\n\
        Terminal=false\n\
        Categories=AudioVideo;Audio;Utility;\n\
        StartupNotify=true",
        target_bin.to_string_lossy(),
        target_icon.to_string_lossy()
    );

    let app_dir = home.join(".local").join("share").join("applications");
    if !app_dir.exists() { fs::create_dir_all(&app_dir)?; }
    fs::write(app_dir.join("audio-selector.desktop"), &desktop_content)?;

    let autostart_dir = home.join(".config").join("autostart");
    if !autostart_dir.exists() { fs::create_dir_all(&autostart_dir)?; }
    fs::write(autostart_dir.join("audio-selector.desktop"), &desktop_content)?;

    // Force GNOME/Desktop environments to update their icon caches
    let _ = Command::new("gtk-update-icon-cache")
        .arg("-f")
        .arg("-t")
        .arg(home.join(".local").join("share").join("icons").join("hicolor"))
        .status();

    println!("Successfully installed to ~/.local/bin/audio-selector");
    println!("Desktop shortcut created in Applications menu.");
    println!("Autostart entry created (will launch with window manager).");
    Ok(())
}

fn append_log(msg: &str) {
    use std::io::Write;
    let log_path = dirs::home_dir().unwrap_or_default().join("audio-selector-debug.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = file.write_all(msg.as_bytes());
    }
}

fn main() -> anyhow::Result<()> {
    let _ = fs::write(dirs::home_dir().unwrap_or_default().join("audio-selector-debug.log"), ""); // clear log
    append_log("Starting Audio Selector...\n");

    if std::env::args().any(|x| x == "-install") { 
        append_log("Running installer...\n");
        return install_app(); 
    }
    
    append_log("Loading config...\n");
    let config_data = load_config();
    
    append_log("Building UI...\n");
    let ui = AppWindow::new()?;
    
    append_log("Applying geometry...\n");
    if let (Some(w), Some(h)) = (config_data.window_width, config_data.window_height) { ui.window().set_size(slint::PhysicalSize::new(w as u32, h as u32)); }
    if let (Some(x), Some(y)) = (config_data.window_x, config_data.window_y) { ui.window().set_position(slint::PhysicalPosition::new(x, y)); }
    let ui_weak = ui.as_weak();
    let ui_handle = Arc::new(Mutex::new(ui_weak.clone()));
    let locale = get_locale().unwrap_or_else(|| "en".to_string());
    let t_static = if locale.starts_with("pt") { &PT } else if locale.starts_with("es") { &ES } else if locale.starts_with("fr") { &FR } else if locale.starts_with("de") { &DE } else if locale.starts_with("it") { &IT } else { &EN };
    
    ui.set_l_title(t_static.title.into()); 
    ui.set_l_tab_devices(t_static.tab_devices.into()); 
    ui.set_l_advanced_options(t_static.advanced_options.into());
    ui.set_l_hide_unknown_bt(t_static.hide_unknown_bt.into());
    ui.set_l_bluetooth(t_static.bluetooth.into());
    ui.set_l_unified(t_static.unified.into()); 
    ui.set_l_output(t_static.output.into());
    ui.set_l_input(t_static.input.into()); 
    ui.set_l_audio_device(t_static.audio_device.into());
    ui.set_l_refresh(t_static.refresh.into()); 
    ui.set_l_filter_active(t_static.filter_active.into());
    ui.set_l_exclude_instruction(t_static.exclude_instruction.into());
    ui.set_l_volume(t_static.volume.into());

    #[cfg(target_os = "linux")] ui.set_status(t_static.status_ready.into());
    #[cfg(not(target_os = "linux"))] ui.set_status(t_static.status_unsupported.into());
    
    let config = Arc::new(Mutex::new(config_data));
    {
        let c = config.lock().unwrap();
        ui.set_unified_mode(c.unified_mode); 
        ui.set_bluetooth_enabled(c.bluetooth_enabled);
        ui.set_filter_enabled(c.filter_enabled); 
        ui.set_hide_unknown_bt(c.hide_unknown_bt);
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
        
        let excluded_names = &c.excluded_devices;

        let hide_unk = c.hide_unknown_bt;
        let is_unknown_bt = |desc: &str| -> bool {
            if !hide_unk { return false; }
            let d = desc.to_uppercase();
            let base = d.replace(" (BLUETOOTH)", "").trim().to_string();
            if base.len() == 17 {
                let dashes = base.chars().filter(|c| *c == '-').count();
                let colons = base.chars().filter(|c| *c == ':').count();
                if dashes == 5 || colons == 5 {
                    return true;
                }
            }
            false
        };

        let bt_devs = get_bluetooth_devices();

        let mut raw_all_sinks = Vec::new();
        if let Ok(sinks) = get_pactl_devices("sinks") { raw_all_sinks.extend(sinks); }
        for bt in bt_devs.iter() {
            let mac = bt.name.replace("bluez_connect.", "").replace(":", "_");
            if !raw_all_sinks.iter().any(|s| s.name.contains(&mac)) {
                raw_all_sinks.push(bt.clone());
            }
        }
        
        let mut raw_all_sources = Vec::new();
        if let Ok(sources) = get_pactl_devices("sources") { raw_all_sources.extend(sources); }
        for bt in bt_devs.iter() {
            let mac = bt.name.replace("bluez_connect.", "").replace(":", "_");
            if !raw_all_sources.iter().any(|s| s.name.contains(&mac)) {
                raw_all_sources.push(bt.clone());
            }
        }

        let mut all_unique_devices: Vec<PactlDevice> = Vec::new();
        for d in raw_all_sinks.iter().chain(raw_all_sources.iter()) {
            if !all_unique_devices.iter().any(|u| u.name == d.name) && !is_unknown_bt(&d.description) {
                all_unique_devices.push(d.clone());
            }
        }
        
        let toggle_list: Vec<crate::DeviceToggle> = all_unique_devices.iter().map(|d| {
            crate::DeviceToggle {
                name: d.name.clone().into(),
                description: d.description.clone().into(),
                excluded: excluded_names.contains(&d.name),
            }
        }).collect();
        ui.set_all_devices(ModelRc::from(Rc::new(VecModel::from(toggle_list))));

        let filtered_sinks: Vec<PactlDevice> = raw_all_sinks.into_iter().filter(|d| {
            (!c.filter_enabled || !excluded_names.contains(&d.name)) && !is_unknown_bt(&d.description)
        }).collect();
        
        ui.set_sink_names(ModelRc::from(Rc::new(VecModel::from(filtered_sinks.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()))));
        
        let mut sink_idx_to_select = 0;
        if let Some(last) = &c.last_sink {
            if let Some(idx) = filtered_sinks.iter().position(|s| s.name == *last) {
                sink_idx_to_select = idx;
            }
        }
        
        if !filtered_sinks.is_empty() {
            ui.set_selected_sink_index(sink_idx_to_select as i32);
            ui.set_sink_volume(filtered_sinks[sink_idx_to_select].get_volume_percent());
        }
        
        *s_cache_ref.lock().unwrap() = filtered_sinks;

        let filtered_sources: Vec<PactlDevice> = raw_all_sources.into_iter().filter(|d| {
            (!c.filter_enabled || !excluded_names.contains(&d.name)) && !is_unknown_bt(&d.description)
        }).collect();
        
        ui.set_source_names(ModelRc::from(Rc::new(VecModel::from(filtered_sources.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()))));
        if let Some(idx) = c.last_source.and_then(|last| filtered_sources.iter().position(|s| s.name == last)) { ui.set_selected_source_index(idx as i32); }
        else if !filtered_sources.is_empty() { ui.set_selected_source_index(0); }
        *src_cache_ref.lock().unwrap() = filtered_sources;
    };
    
    refresh_fn();
    let r1 = refresh_fn.clone(); ui.on_refresh(r1);
    let config_bt = Arc::clone(&config); ui.on_toggle_bluetooth(move |on| { let _ = set_bluetooth_power(on); let mut c = config_bt.lock().unwrap(); c.bluetooth_enabled = on; save_config(&c); });
    let config_uni = Arc::clone(&config); ui.on_toggle_unified(move |on| { let mut c = config_uni.lock().unwrap(); c.unified_mode = on; save_config(&c); });
    
    let config_filter = Arc::clone(&config); 
    let r2 = refresh_fn.clone(); 
    ui.on_toggle_filter(move |on| { { let mut c = config_filter.lock().unwrap(); c.filter_enabled = on; save_config(&c); } r2(); });
    
    let config_hide_unk = Arc::clone(&config);
    let r_hide = refresh_fn.clone();
    ui.on_toggle_hide_unknown_bt(move |on| {
        {
            let mut c = config_hide_unk.lock().unwrap();
            c.hide_unknown_bt = on;
            save_config(&c);
        }
        r_hide();
    });

    let config_exclusion = Arc::clone(&config);
    let r_exclusion = refresh_fn.clone();
    ui.on_toggle_device_exclusion(move |name, excluded| {
        let name_str = name.to_string();
        {
            let mut c = config_exclusion.lock().unwrap();
            if excluded {
                if !c.excluded_devices.contains(&name_str) {
                    c.excluded_devices.push(name_str);
                }
            } else {
                c.excluded_devices.retain(|x| x != &name_str);
            }
            save_config(&c);
        }
        r_exclusion();
    });

    let sinks_change = Arc::clone(&sinks_cache);
    let sources_change = Arc::clone(&sources_cache);
    let config_change = Arc::clone(&config);
    let ui_change = Arc::clone(&ui_handle);
    let locale_change = locale.clone();
    
    let change_handler = move |sink_idx: i32, source_idx: i32| {
        let ui_weak_lock = ui_change.lock().unwrap();
        let ui_weak = (*ui_weak_lock).clone();
        let ui = ui_weak.unwrap();
        
        if sink_idx >= 0 {
            let sinks = sinks_change.lock().unwrap();
            if (sink_idx as usize) < sinks.len() {
                let s_name = sinks[sink_idx as usize].name.clone();
                let s_vol = sinks[sink_idx as usize].get_volume_percent();
                ui.set_sink_volume(s_vol);

                let unified = ui.get_unified_mode();
                let ui_async = ui_weak.clone();
                let ui_async_status = ui_weak.clone();
                let sources_async = Arc::clone(&sources_change);
                let config_async = Arc::clone(&config_change);
                let locale_async = locale_change.clone();

                thread::spawn(move || {
                    let t_async = if locale_async.starts_with("pt") { &PT } else if locale_async.starts_with("es") { &ES } else if locale_async.starts_with("fr") { &FR } else if locale_async.starts_with("de") { &DE } else if locale_async.starts_with("it") { &IT } else { &EN };
                    let mut actual_name = s_name.clone();
                    
                    #[cfg(target_os = "linux")]
                    if s_name.starts_with("bluez_connect.") {
                        let mac = s_name.replace("bluez_connect.", "");
                        let ui_c = ui_async.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            ui_c.unwrap().set_status(t_async.status_connecting.into());
                        });
                        
                        let _ = Command::new("bluetoothctl").args(["connect", &mac]).status();
                        thread::sleep(std::time::Duration::from_millis(2000));
                        
                        if let Ok(pactl_sinks) = get_pactl_devices("sinks") {
                            if let Some(found) = pactl_sinks.iter().find(|s| s.name.contains(&mac.replace(":", "_"))) {
                                actual_name = found.name.clone();
                            }
                        }
                    }

                    if let Err(_) = apply_device_change("sinks", &actual_name) { return; }
                    
                    let mut c = config_async.lock().unwrap();
                    c.last_sink = Some(actual_name.clone());

                    if unified {
                        let base = actual_name.replace("bluez_sink", "").replace(".a2dp_sink", "").replace(".hifi", "");
                        let sources = sources_async.lock().unwrap();
                        for src in sources.iter() {
                            if src.name.contains(&base) {
                                let _ = apply_device_change("sources", &src.name);
                                c.last_source = Some(src.name.clone());
                                break;
                            }
                        }
                    }
                    save_config(&c);
                    let _ = slint::invoke_from_event_loop(move || {
                        ui_async_status.unwrap().set_status(format!("{} - {}", t_async.status_applied, chrono::Local::now().format("%H:%M:%S")).into());
                    });
                });
            }
        }

        if !ui.get_unified_mode() && source_idx >= 0 {
            let sources = sources_change.lock().unwrap();
            if (source_idx as usize) < sources.len() {
                let s_name = sources[source_idx as usize].name.clone();
                let config_async = Arc::clone(&config_change);
                let ui_async_status = ui_weak.clone();
                let locale_async = locale_change.clone();
                
                thread::spawn(move || {
                    let t_async = if locale_async.starts_with("pt") { &PT } else if locale_async.starts_with("es") { &ES } else if locale_async.starts_with("fr") { &FR } else if locale_async.starts_with("de") { &DE } else if locale_async.starts_with("it") { &IT } else { &EN };
                    let _ = apply_device_change("sources", &s_name);
                    let mut c = config_async.lock().unwrap();
                    c.last_source = Some(s_name);
                    save_config(&c);
                    let _ = slint::invoke_from_event_loop(move || {
                        ui_async_status.unwrap().set_status(format!("{} - {}", t_async.status_applied, chrono::Local::now().format("%H:%M:%S")).into());
                    });
                });
            }
        }
    };

    let change_sink = change_handler.clone();
    ui.on_sink_changed(move |idx| {
        change_sink(idx, -1);
    });

    let change_source = change_handler.clone();
    ui.on_source_changed(move |idx| {
        change_source(-1, idx);
    });

    let sinks_vol = Arc::clone(&sinks_cache);
    let ui_vol = Arc::clone(&ui_handle);
    ui.on_sink_volume_changed(move |vol| {
        let ui_weak_lock = ui_vol.lock().unwrap();
        let ui = ui_weak_lock.unwrap();
        
        // Prevent infinite loop: check if UI really needs to update volume
        // We only send command if the slider was actually moved by user
        let idx = ui.get_selected_sink_index();
        if idx >= 0 {
            let sinks = sinks_vol.lock().unwrap();
            if (idx as usize) < sinks.len() {
                let name = &sinks[idx as usize].name;
                // Only change volume for REAL sinks, not the virtual 'bluez_connect'
                if !name.starts_with("bluez_connect.") {
                    let _ = set_sink_volume(name, vol);
                }
            }
        }
    });

    let window = ui.window();
    let config_exit = Arc::clone(&config);
    
    ui.run()?;
    
    let mut c = config_exit.lock().unwrap();
    let size = window.size();
    c.window_width = Some(size.width as f32);
    c.window_height = Some(size.height as f32);
    let pos = window.position();
    c.window_x = Some(pos.x);
    c.window_y = Some(pos.y);
    save_config(&c);
    
    Ok(())
}
