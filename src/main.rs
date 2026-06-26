slint::include_modules!();

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::process::Command;
use slint::{ModelRc, VecModel, SharedString, Model, ComponentHandle};
use std::rc::Rc;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use sys_locale::get_locale;

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
};

const ES: Translations = Translations {
    title: "Selector de Audio",
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
};

const FR: Translations = Translations {
    title: "Sélecteur d'Audio",
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
};

const DE: Translations = Translations {
    title: "Audio-Selector",
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
};

const IT: Translations = Translations {
    title: "Selettore Audio",
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
};

fn get_pactl_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    let output = Command::new("pactl")
        .env("LC_ALL", "C")
        .args(["--format=json", "list", target])
        .output()
        .context(format!("Failed to execute pactl list {}", target))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_start = stdout.find('[').or_else(|| stdout.find('{'));
    
    let json_str = match json_start {
        Some(start) => &stdout[start..],
        None => return Ok(Vec::new()),
    };

    let devices: Vec<PactlDevice> = serde_json::from_str(json_str.trim()).unwrap_or_default();
    Ok(devices.into_iter().filter(|d| !d.name.contains(".monitor")).collect())
}

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
                bt_devices.push(PactlDevice {
                    name: format!("bluez_connect.{}", mac),
                    description: format!("{} (Bluetooth)", name), // (Bluetooth) after name
                });
            }
        }
    }
    bt_devices
}

fn apply_device_change(target: &str, name: &str) -> anyhow::Result<()> {
    let arg = if target == "sinks" { "set-default-sink" } else { "set-default-source" };
    Command::new("pactl").env("LC_ALL", "C").args([arg, name]).status()?;

    if target == "sinks" {
        let inputs_output = Command::new("pactl").env("LC_ALL", "C").args(["list", "short", "sink-inputs"]).output()?;
        let inputs = String::from_utf8_lossy(&inputs_output.stdout);
        for line in inputs.lines() {
            if let Some(id) = line.split_whitespace().next() {
                let _ = Command::new("pactl").env("LC_ALL", "C").args(["move-sink-input", id, name]).status();
            }
        }
    }
    Ok(())
}

fn check_bluetooth_hardware() -> bool {
    Command::new("bluetoothctl").arg("show").output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).contains("No default controller available"))
        .unwrap_or(false)
}

fn set_bluetooth_power(on: bool) -> anyhow::Result<()> {
    let state = if on { "on" } else { "off" };
    let _ = Command::new("bluetoothctl").args(["power", state]).status();
    Ok(())
}

fn load_config() -> Config {
    if let Ok(content) = fs::read_to_string(CONFIG_FILE) {
        if let Ok(config) = serde_json::from_str(&content) {
            return config;
        }
    }
    Config { unified_mode: true, bluetooth_enabled: false, ..Default::default() }
}

fn save_config(config: &Config) {
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = fs::write(CONFIG_FILE, content);
    }
}

fn main() -> anyhow::Result<()> {
    let mut config_data = load_config();
    let ui = AppWindow::new()?;
    
    // Set window size/position if persisted
    if let (Some(w), Some(h)) = (config_data.window_width, config_data.window_height) {
        ui.window().set_size(slint::PhysicalSize::new(w as u32, h as u32));
    }
    if let (Some(x), Some(y)) = (config_data.window_x, config_data.window_y) {
        ui.window().set_position(slint::PhysicalPosition::new(x, y));
    }

    let ui_handle = Arc::new(Mutex::new(ui.as_weak()));
    // Localization
    let locale = get_locale().unwrap_or_else(|| "en".to_string());
    let t_static = if locale.starts_with("pt") {
        &PT
    } else if locale.starts_with("es") {
        &ES
    } else if locale.starts_with("fr") {
        &FR
    } else if locale.starts_with("de") {
        &DE
    } else if locale.starts_with("it") {
        &IT
    } else {
        &EN
    };

    
    ui.set_l_title(t_static.title.into());
    ui.set_l_bluetooth(t_static.bluetooth.into());
    ui.set_l_unified(t_static.unified.into());
    ui.set_l_output(t_static.output.into());
    ui.set_l_input(t_static.input.into());
    ui.set_l_audio_device(t_static.audio_device.into());
    ui.set_l_refresh(t_static.refresh.into());
    ui.set_status(t_static.status_ready.into());

    let sinks_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new()));
    let sources_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new()));
    let config = Arc::new(Mutex::new(config_data));

    {
        let c = config.lock().unwrap();
        ui.set_unified_mode(c.unified_mode);
        ui.set_bluetooth_enabled(c.bluetooth_enabled);
    }

    let bt_available = check_bluetooth_hardware();
    ui.set_bluetooth_available(bt_available);
    if bt_available && ui.get_bluetooth_enabled() {
        let _ = set_bluetooth_power(true);
    }

    // Refresh Logic
    let sinks_ref = Arc::clone(&sinks_cache);
    let sources_ref = Arc::clone(&sources_cache);
    let ui_ref = Arc::clone(&ui_handle);
    let config_ref = Arc::clone(&config);
    let t_ref = if locale.starts_with("pt") {
        &PT
    } else if locale.starts_with("es") {
        &ES
    } else if locale.starts_with("fr") {
        &FR
    } else if locale.starts_with("de") {
        &DE
    } else if locale.starts_with("it") {
        &IT
    } else {
        &EN
    };

    let refresh_fn = move || {
        let ui_weak = ui_ref.lock().unwrap();
        let ui = ui_weak.unwrap();
        let c = config_ref.lock().unwrap().clone();
        
        let mut all_sinks = Vec::new();
        if let Ok(sinks) = get_pactl_devices("sinks") {
            all_sinks.extend(sinks);
        }
        all_sinks.extend(get_bluetooth_devices());

        let descriptions: Vec<SharedString> = all_sinks.iter().map(|d| d.description.as_str().into()).collect();
        ui.set_sink_names(ModelRc::from(Rc::new(VecModel::from(descriptions))));
        
        if let Some(last) = &c.last_sink {
            if let Some(idx) = all_sinks.iter().position(|s| &s.name == last) {
                ui.set_selected_sink_index(idx as i32);
            } else if !all_sinks.is_empty() {
                ui.set_selected_sink_index(0);
            }
        } else if !all_sinks.is_empty() {
            ui.set_selected_sink_index(0);
        }
        *sinks_ref.lock().unwrap() = all_sinks;

        if let Ok(sources) = get_pactl_devices("sources") {
            let descriptions: Vec<SharedString> = sources.iter().map(|d| d.description.as_str().into()).collect();
            ui.set_source_names(ModelRc::from(Rc::new(VecModel::from(descriptions))));
            
            if let Some(last) = &c.last_source {
                if let Some(idx) = sources.iter().position(|s| &s.name == last) {
                    ui.set_selected_source_index(idx as i32);
                } else if !sources.is_empty() {
                    ui.set_selected_source_index(0);
                }
            } else if !sources.is_empty() {
                ui.set_selected_source_index(0);
            }
            *sources_ref.lock().unwrap() = sources;
        }
        ui.set_status(t_ref.status_ready.into());
    };

    refresh_fn();
    ui.on_refresh(refresh_fn);

    // Bluetooth Toggle
    let config_bt = Arc::clone(&config);
    let ui_bt = Arc::clone(&ui_handle);
    let t_bt = if locale.starts_with("pt") {
        &PT
    } else if locale.starts_with("es") {
        &ES
    } else if locale.starts_with("fr") {
        &FR
    } else if locale.starts_with("de") {
        &DE
    } else if locale.starts_with("it") {
        &IT
    } else {
        &EN
    };
    ui.on_toggle_bluetooth(move |on| {
        let _ = set_bluetooth_power(on);
        let mut c = config_bt.lock().unwrap();
        c.bluetooth_enabled = on;
        save_config(&c);
        let ui_weak = ui_bt.lock().unwrap();
        let ui = ui_weak.unwrap();
        ui.set_status(if on { t_bt.status_bt_on.into() } else { t_bt.status_bt_off.into() });
    });

    // Unified Toggle
    let config_uni = Arc::clone(&config);
    ui.on_toggle_unified(move |on| {
        let mut c = config_uni.lock().unwrap();
        c.unified_mode = on;
        save_config(&c);
    });

    // Change Logic
    let sinks_change = Arc::clone(&sinks_cache);
    let sources_change = Arc::clone(&sources_cache);
    let config_change = Arc::clone(&config);
    let ui_change = Arc::clone(&ui_handle);
    let locale_change = locale.clone();

    let change_handler = move |sink_idx: i32, source_idx: i32| {
        let ui_weak_lock = ui_change.lock().unwrap();
        let ui_weak = (*ui_weak_lock).clone();
        let ui = ui_weak.unwrap();
        let t = if locale_change.starts_with("pt") {
            &PT
        } else if locale_change.starts_with("es") {
            &ES
        } else if locale_change.starts_with("fr") {
            &FR
        } else if locale_change.starts_with("de") {
            &DE
        } else if locale_change.starts_with("it") {
            &IT
        } else {
            &EN
        };
        
        if sink_idx >= 0 {
            let sinks = sinks_change.lock().unwrap();
            if (sink_idx as usize) < sinks.len() {
                let s_name = sinks[sink_idx as usize].name.clone();
                let unified = ui.get_unified_mode();
                
                let ui_async = ui_weak.clone();
                let ui_async_status = ui_weak.clone();
                let sources_async = Arc::clone(&sources_change);
                let config_async = Arc::clone(&config_change);
                let t_async = if locale_change.starts_with("pt") {
                    &PT
                } else if locale_change.starts_with("es") {
                    &ES
                } else if locale_change.starts_with("fr") {
                    &FR
                } else if locale_change.starts_with("de") {
                    &DE
                } else if locale_change.starts_with("it") {
                    &IT
                } else {
                    &EN
                };

                thread::spawn(move || {
                    let mut actual_name = s_name.clone();
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
                let mut c = config_change.lock().unwrap();
                let _ = apply_device_change("sources", &s_name);
                c.last_source = Some(s_name);
                save_config(&c);
                ui.set_status(format!("{} - {}", t.status_applied, chrono::Local::now().format("%H:%M:%S")).into());
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

    // Save window state before exit
    let window = ui.window();
    let config_exit = Arc::clone(&config);
    
    ui.run()?;
    
    // Final save of window geometry
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
