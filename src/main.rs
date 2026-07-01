mod audio;
mod bluetooth;
mod config;
mod i18n;
mod utils;

slint::include_modules!();

use anyhow::Context;
use std::process::Command;
use slint::{ModelRc, VecModel, SharedString, ComponentHandle};
use std::rc::Rc;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIconBuilder,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "ui/assets/"]
struct Assets;

use crate::audio::{get_pactl_devices, apply_device_change, set_sink_volume, PactlDevice};
use crate::bluetooth::{get_bluetooth_devices, set_bluetooth_power};
use crate::config::{load_config, save_config, Config, CachedDevice};
use crate::i18n::get_current_translations;
use crate::utils::{append_log, get_log_content, get_bluetooth_mac};

fn install_app() -> anyhow::Result<()> {
    append_log("Installing application...");
    let cur = std::env::current_exe()?;
    let home = dirs::home_dir().context("No home directory found")?;
    
    // 1. Create binary directory
    let bin_dir = home.join(".local").join("bin");
    if !bin_dir.exists() { fs::create_dir_all(&bin_dir)?; }
    let target_bin = bin_dir.join("audio-selector");
    append_log(&format!("Copying binary to {:?}", target_bin));
    fs::copy(&cur, &target_bin)?;
    
    // 2. Create icon directory and export embedded icon
    let icon_dir = home.join(".local").join("share").join("icons").join("hicolor").join("256x256").join("apps");
    if !icon_dir.exists() { fs::create_dir_all(&icon_dir)?; }
    let target_icon = icon_dir.join("audio-selector.png");
    
    if let Some(icon_data) = Assets::get("icon.png") {
        append_log(&format!("Exporting icon to {:?}", target_icon));
        fs::write(&target_icon, icon_data.data)?;
    }
    
    let desktop_base = format!(
        "[Desktop Entry]\nType=Application\nName=Audio Selector\nIcon={}\nTerminal=false\nCategories=AudioVideo;Audio;Utility;\nStartupNotify=true",
        target_icon.to_string_lossy()
    );
    
    // 3. Create Desktop and Autostart entries
    let app_dir = home.join(".local").join("share").join("applications");
    if !app_dir.exists() { fs::create_dir_all(&app_dir)?; }
    fs::write(app_dir.join("audio-selector.desktop"), format!("{}\nExec={}", desktop_base, target_bin.to_string_lossy()))?;
    
    let autostart = home.join(".config").join("autostart");
    if !autostart.exists() { fs::create_dir_all(&autostart)?; }
    fs::write(autostart.join("audio-selector.desktop"), format!("{}\nExec={} --tray", desktop_base, target_bin.to_string_lossy()))?;
    
    append_log("Installation successful.");
    Ok(())
}

fn load_tray_icon() -> tray_icon::Icon {
    // Load from embedded assets instead of filesystem path
    let icon_data = Assets::get("icon.png").expect("Icon not found in embedded assets");
    let img = image::load_from_memory(&icon_data.data).expect("Failed to decode icon");
    let img = image::imageops::resize(&img, 64, 64, image::imageops::FilterType::Nearest);
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("Tray icon creation failed")
}

fn update_ui_models(ui: &AppWindow, sinks: &[PactlDevice], sources: &[PactlDevice], config: &Config) {
    let excl = &config.excluded_devices;
    let f_e = config.filter_enabled;
    let h_u = config.hide_unknown_bt;

    let is_unknown = |desc: &str| {
        if !h_u { return false; }
        let d = desc.to_uppercase();
        let b = d.replace(" (BLUETOOTH)", "").trim().to_string();
        b.len() == 17 && (b.chars().filter(|c| *c == '-').count() == 5 || b.chars().filter(|c| *c == ':').count() == 5)
    };

    let fsinks: Vec<PactlDevice> = sinks.iter()
        .filter(|d| (!f_e || !excl.contains(&d.name)) && !is_unknown(&d.description))
        .cloned()
        .collect();
    let fsrcs: Vec<PactlDevice> = sources.iter()
        .filter(|d| (!f_e || !excl.contains(&d.name)) && !is_unknown(&d.description))
        .cloned()
        .collect();

    let mut all_unique: Vec<PactlDevice> = Vec::new();
    for d in sinks.iter().chain(sources.iter()) {
        if !all_unique.iter().any(|u| u.name == d.name) { all_unique.push(d.clone()); }
    }

    ui.set_all_devices(ModelRc::from(Rc::new(VecModel::from(
        all_unique.iter().map(|d| crate::DeviceToggle {
            name: d.name.clone().into(),
            description: d.description.clone().into(),
            excluded: excl.contains(&d.name)
        }).collect::<Vec<_>>()
    ))));

    ui.set_sink_names(ModelRc::from(Rc::new(VecModel::from(
        fsinks.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()
    ))));

    if let Some(idx) = config.last_sink.as_ref().and_then(|l| fsinks.iter().position(|s| &s.name == l)) {
        ui.set_selected_sink_index(idx as i32);
        ui.set_sink_volume(fsinks[idx].get_volume_percent());
    } else if !fsinks.is_empty() {
        ui.set_selected_sink_index(0);
        ui.set_sink_volume(fsinks[0].get_volume_percent());
    }

    ui.set_source_names(ModelRc::from(Rc::new(VecModel::from(
        fsrcs.iter().map(|d| d.description.as_str().into()).collect::<Vec<SharedString>>()
    ))));

    if let Some(idx) = config.last_source.as_ref().and_then(|l| fsrcs.iter().position(|s| &s.name == l)) {
        ui.set_selected_source_index(idx as i32);
    } else if !fsrcs.is_empty() {
        ui.set_selected_source_index(0);
    }
}

fn main() -> anyhow::Result<()> {
    append_log(">>> APPLICATION STARTING");
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|x| x == "-install") { 
        return install_app(); 
    }
    let start_in_tray = args.iter().any(|x| x == "--tray");
    
    // Prevent multiple instances if possible (simple file lock check)
    let lock_path = crate::utils::get_config_dir().join("app.lock");
    if let Ok(m) = fs::metadata(&lock_path) {
        if let Ok(age) = m.modified().map(|t| t.elapsed().unwrap_or_default()) {
            if age.as_secs() < 5 {
                append_log("Another instance might be running. Exiting to avoid hang.");
                // return Ok(()); // Disabled for now, just logging
            }
        }
    }
    let _ = fs::write(&lock_path, std::process::id().to_string());

    let config_data = load_config();
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();
    
    // Show window immediately if not in tray mode
    if !start_in_tray {
        ui.window().show().unwrap();
    }
    
    if let (Some(w), Some(h)) = (config_data.window_width, config_data.window_height) {
        ui.window().set_size(slint::PhysicalSize::new(w as u32, h as u32));
    }
    if let (Some(x), Some(y)) = (config_data.window_x, config_data.window_y) {
        ui.window().set_position(slint::PhysicalPosition::new(x, y));
    }
    
    let t = get_current_translations();
    ui.set_l_title(t.title.into());
    ui.set_l_tab_devices(t.tab_devices.into());
    ui.set_l_advanced_options(t.advanced_options.into());
    ui.set_l_hide_unknown_bt(t.hide_unknown_bt.into());
    ui.set_l_bluetooth(t.bluetooth.into());
    ui.set_l_unified(t.unified.into());
    ui.set_l_output(t.output.into());
    ui.set_l_input(t.input.into());
    ui.set_l_audio_device(t.audio_device.into());
    ui.set_l_refresh(t.refresh.into());
    ui.set_l_filter_active(t.filter_active.into());
    ui.set_l_exclude_instruction(t.exclude_instruction.into());
    ui.set_l_volume(t.volume.into());
    ui.set_l_open_logs(t.open_logs.into());
    #[cfg(target_os = "linux")] ui.set_status(t.status_ready.into());
    
    let cfg_arc = Arc::new(Mutex::new(config_data.clone()));
    {
        let c = cfg_arc.lock().unwrap();
        ui.set_unified_mode(c.unified_mode);
        ui.set_bluetooth_enabled(c.bluetooth_enabled);
        ui.set_filter_enabled(c.filter_enabled);
        ui.set_hide_unknown_bt(c.hide_unknown_bt);
        
        if !c.cached_sinks.is_empty() || !c.cached_sources.is_empty() {
            append_log("Loading cached devices into UI...");
            let cached_sinks: Vec<PactlDevice> = c.cached_sinks.iter().map(|d| PactlDevice {
                name: d.name.clone(),
                description: d.description.clone(),
                volume: Some(serde_json::json!({"0": {"value_percent": format!("{}%", d.volume_percent)}}))
            }).filter(|d| c.bluetooth_enabled || (!d.name.contains("bluez_connect.") && !d.name.contains("bluez_sink"))).collect();
            let cached_sources: Vec<PactlDevice> = c.cached_sources.iter().map(|d| PactlDevice {
                name: d.name.clone(),
                description: d.description.clone(),
                volume: None
            }).filter(|d| c.bluetooth_enabled || (!d.name.contains("bluez_connect.") && !d.name.contains("bluez_source") && !d.name.contains("bluez_input"))).collect();
            update_ui_models(&ui, &cached_sinks, &cached_sources, &c);
        }
    }

    #[cfg(target_os = "linux")] {
        let ui_weak_tray = ui_weak.clone();
        let tray_title = t.title.to_string();
        let menu_quit_text = t.menu_quit.to_string();
        
        thread::spawn(move || {
            append_log("Initializing system tray in background...");
            if gtk::init().is_ok() {
                let menu = Menu::new();
                let q_i = MenuItem::new(&menu_quit_text, true, None);
                let q_id = q_i.id().clone();
                let _ = menu.append_items(&[&q_i]);
                
                // load_tray_icon now uses embedded Assets
                let icon_res = load_tray_icon();
                
                if let Ok(icon) = TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_tooltip(&tray_title)
                    .with_icon(icon_res)
                    .build() {
                    
                    let m_c = tray_icon::menu::MenuEvent::receiver();
                    thread::spawn(move || {
                        loop { if let Ok(e) = m_c.recv() { if e.id == q_id { std::process::exit(0); } } }
                    });
                    
                    let t_c = tray_icon::TrayIconEvent::receiver();
                    let u_i = ui_weak_tray.clone();
                    thread::spawn(move || {
                        loop {
                            if let Ok(e) = t_c.recv() {
                                if let tray_icon::TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. } = e {
                                    let ui_inner = u_i.clone();
                                    let _ = slint::invoke_from_event_loop(move || { if let Some(uw) = ui_inner.upgrade() { uw.window().show().unwrap(); } });
                                }
                            }
                        }
                    });
                    
                    let _ = Box::leak(Box::new(icon));
                    gtk::main();
                }
            }
        });
    }

    ui.window().on_close_requested(|| { slint::CloseRequestResponse::HideWindow });

    ui.on_open_logs(move || {
        if let Ok(log_ui) = LogWindow::new() {
            let lw = log_ui.as_weak();
            let r_logs = move || { if let Some(u) = lw.upgrade() { u.set_log_text(get_log_content(&u.get_log_search()).into()); } };
            r_logs();
            let r_logs_cb = r_logs.clone();
            log_ui.on_refresh_logs(move || r_logs_cb());
            log_ui.show().unwrap();
            Box::leak(Box::new(log_ui));
        }
    });

    let sinks_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new())); 
    let sources_cache = Arc::new(Mutex::new(Vec::<PactlDevice>::new()));
    let ui_weak_refresh = ui_weak.clone();
    let config_arc_refresh = Arc::clone(&cfg_arc); 
    let s_c_ref = Arc::clone(&sinks_cache); 
    let src_c_ref = Arc::clone(&sources_cache);
    
    let refresh_fn = move || {
        let u_w = ui_weak_refresh.clone();
        let cfg_locked = config_arc_refresh.clone();
        let sc = Arc::clone(&s_c_ref); 
        let srcc = Arc::clone(&src_c_ref);
        thread::spawn(move || {
            append_log("Scan: Scanning devices...");
            let bt = get_bluetooth_devices();
            let mut rs = Vec::new();
            if let Ok(s) = get_pactl_devices("sinks") { rs.extend(s); }
            for b in bt.iter() {
                let m = b.name.replace("bluez_connect.", "").replace(":", "_").to_lowercase();
                if !rs.iter().any(|s| s.name.to_lowercase().contains(&m)) { rs.push(b.clone()); }
            }
            let mut rsrc = Vec::new();
            if let Ok(s) = get_pactl_devices("sources") { rsrc.extend(s); }
            for b in bt.iter() {
                let m = b.name.replace("bluez_connect.", "").replace(":", "_").to_lowercase();
                if !rsrc.iter().any(|s| s.name.to_lowercase().contains(&m)) { rsrc.push(b.clone()); }
            }
            
            let rs_c = rs.clone();
            let rsrc_c = rsrc.clone();
            
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = u_w.upgrade() {
                    let mut c = cfg_locked.lock().unwrap();
                    let mut final_sinks = rs_c.clone();
                    let mut final_sources = rsrc_c.clone();
                    if !c.bluetooth_enabled {
                        final_sinks.retain(|d| !d.name.contains("bluez_connect.") && !d.name.contains("bluez_sink"));
                        final_sources.retain(|d| !d.name.contains("bluez_connect.") && !d.name.contains("bluez_source") && !d.name.contains("bluez_input"));
                    }
                    if !final_sinks.is_empty() || !final_sources.is_empty() {
                        update_ui_models(&ui, &final_sinks, &final_sources, &c);
                        c.cached_sinks = final_sinks.iter().map(|d| CachedDevice {
                            name: d.name.clone(),
                            description: d.description.clone(),
                            volume_percent: d.get_volume_percent(),
                        }).collect();
                        c.cached_sources = final_sources.iter().map(|d| CachedDevice {
                            name: d.name.clone(),
                            description: d.description.clone(),
                            volume_percent: 0,
                        }).collect();
                        save_config(&c);
                    }
                    *sc.lock().unwrap() = final_sinks;
                    *srcc.lock().unwrap() = final_sources;
                    append_log("Scan: Completed and UI updated.");

                    // Auto-restore last devices
                    let last_sink = c.last_sink.clone();
                    let last_source = c.last_source.clone();
                    let u_auto = u_w.clone();
                    
                    if let Some(ls) = last_sink {
                        append_log(&format!("Auto-Restore: Checking sink: {}", ls));
                        let sinks = sc.lock().unwrap();
                        if let Some(idx) = sinks.iter().position(|s| s.name == ls) {
                            append_log("Auto-Restore: Exact sink match found.");
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ua) = u_auto.upgrade() { ua.invoke_sink_changed(idx as i32); }
                            });
                        } else if let Some(mac) = get_bluetooth_mac(&ls) {
                            append_log(&format!("Auto-Restore: Sink missing, attempting BT reconnect for MAC: {}", mac));
                            let target = format!("bluez_connect.{}", mac);
                            if let Some(idx) = sinks.iter().position(|s| s.name == target) {
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(ua) = u_auto.upgrade() { ua.invoke_sink_changed(idx as i32); }
                                });
                            }
                        }
                    }
                    
                    let u_auto_src = u_w.clone();
                    if let Some(lsrc) = last_source {
                        append_log(&format!("Auto-Restore: Checking source: {}", lsrc));
                        let sources = srcc.lock().unwrap();
                        if let Some(idx) = sources.iter().position(|s| s.name == lsrc) {
                            append_log("Auto-Restore: Exact source match found.");
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ua) = u_auto_src.upgrade() {
                                    if !ua.get_unified_mode() { ua.invoke_source_changed(idx as i32); }
                                }
                            });
                        } else if let Some(mac) = get_bluetooth_mac(&lsrc) {
                            append_log(&format!("Auto-Restore: Source missing, attempting BT reconnect for MAC: {}", mac));
                            let target = format!("bluez_connect.{}", mac);
                            if let Some(idx) = sources.iter().position(|s| s.name == target) {
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(ua) = u_auto_src.upgrade() {
                                        if !ua.get_unified_mode() { ua.invoke_source_changed(idx as i32); }
                                    }
                                });
                            }
                        }
                    }
                }
            });
        });
    };
    
    let r_init = refresh_fn.clone();
    slint::Timer::single_shot(std::time::Duration::from_millis(500), move || { r_init(); });
    ui.on_refresh(refresh_fn.clone());

    let c_bt = Arc::clone(&cfg_arc);
    let r_bt = refresh_fn.clone();
    ui.on_toggle_bluetooth(move |on| {
        let c = c_bt.clone();
        let r = r_bt.clone();
        thread::spawn(move || {
            let _ = set_bluetooth_power(on);
            {
                let mut cfg = c.lock().unwrap();
                cfg.bluetooth_enabled = on;
                save_config(&cfg);
            }
            r();
        });
    });

    let c_uni = Arc::clone(&cfg_arc);
    ui.on_toggle_unified(move |on| {
        let mut c = c_uni.lock().unwrap();
        c.unified_mode = on;
        save_config(&c);
    });

    let c_f = Arc::clone(&cfg_arc);
    let r2 = refresh_fn.clone();
    ui.on_toggle_filter(move |on| {
        { let mut c = c_f.lock().unwrap(); c.filter_enabled = on; save_config(&c); }
        r2();
    });

    let c_h = Arc::clone(&cfg_arc);
    let r_h = refresh_fn.clone();
    ui.on_toggle_hide_unknown_bt(move |on| {
        { let mut c = c_h.lock().unwrap(); c.hide_unknown_bt = on; save_config(&c); }
        r_h();
    });

    let c_e = Arc::clone(&cfg_arc);
    let r_e = refresh_fn.clone();
    ui.on_toggle_device_exclusion(move |n, e| {
        let ns = n.to_string();
        {
            let mut c = c_e.lock().unwrap();
            if e { if !c.excluded_devices.contains(&ns) { c.excluded_devices.push(ns); } }
            else { c.excluded_devices.retain(|x| x != &ns); }
            save_config(&c);
        }
        r_e();
    });
    
    let sc_c = Arc::clone(&sinks_cache);
    let src_c = Arc::clone(&sources_cache);
    let cfg_handler = Arc::clone(&cfg_arc);
    let ui_weak_handler = ui_weak.clone();
    
    let handler = move |s_i: i32, src_i: i32| {
        if let Some(u) = ui_weak_handler.upgrade() {
            if s_i >= 0 {
                let sks = sc_c.lock().unwrap();
                if (s_i as usize) < sks.len() {
                    let n = sks[s_i as usize].name.clone();
                    let u_a = ui_weak_handler.clone();
                    let sr_a = Arc::clone(&src_c);
                    let cf_a = Arc::clone(&cfg_handler);
                    let current_vol = u.get_sink_volume();
                    u.set_sink_volume(sks[s_i as usize].get_volume_percent());
                    thread::spawn(move || {
                        let ta = get_current_translations();
                        let mut an = n.clone();
                        if n.starts_with("bluez_connect.") {
                            let mac = n.replace("bluez_connect.", "");
                            let ui_conn = u_a.clone();
                            let _ = slint::invoke_from_event_loop(move || { if let Some(ua) = ui_conn.upgrade() { ua.set_status(ta.status_connecting.into()); } });
                            
                            let mut found = false;
                            append_log(&format!("BT: Starting connection sequence for {}", mac));
                            
                            for attempt in 1..=3 {
                                append_log(&format!("BT: Connection attempt {}/3 for {}", attempt, mac));
                                
                                if attempt == 2 {
                                    append_log("BT: Attempting trust and brief scan to wake up device...");
                                    let _ = Command::new("bluetoothctl").args(["trust", &mac]).status();
                                    let _ = Command::new("bluetoothctl").args(["scan", "on"]).spawn();
                                    thread::sleep(std::time::Duration::from_millis(1500));
                                    let _ = Command::new("bluetoothctl").args(["scan", "off"]).status();
                                }

                                let output = Command::new("bluetoothctl").args(["connect", &mac]).output();
                                match output {
                                    Ok(o) if o.status.success() => {
                                        append_log("BT: bluetoothctl connect returned success.");
                                    }
                                    Ok(o) => {
                                        let out = String::from_utf8_lossy(&o.stdout);
                                        let err = String::from_utf8_lossy(&o.stderr);
                                        append_log(&format!("BT: connect failed. Out: {} Err: {}", out.trim(), err.trim()));
                                    }
                                    Err(e) => {
                                        append_log(&format!("BT: Failed to execute bluetoothctl: {}", e));
                                    }
                                }

                                // Give PulseAudio/PipeWire time to see the device
                                thread::sleep(std::time::Duration::from_millis(1500 * attempt as u64));
                                
                                if let Ok(p) = get_pactl_devices("sinks") {
                                    if let Some(f) = p.iter().find(|s| s.name.contains(&mac.replace(":", "_"))) { 
                                        an = f.name.clone(); 
                                        found = true;
                                        append_log(&format!("BT: Device found in pactl sinks: {}", an));
                                        break;
                                    }
                                }
                                append_log("BT: Device not yet in pactl sinks.");
                            }

                            let ui_status = u_a.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ua) = ui_status.upgrade() {
                                    let t = get_current_translations();
                                    ua.set_status((if found { t.status_connected } else { t.status_failed }).into());
                                    if found { ua.invoke_refresh(); }
                                }
                            });
                            
                            if !found { 
                                append_log("BT: Connection failed after all attempts.");
                                return; 
                            }
                        }
                        let _ = apply_device_change("sinks", &an);
                        let _ = set_sink_volume(&an, current_vol);
                        
                        let mut c = cf_a.lock().unwrap();
                        c.last_sink = Some(an.clone());
                        let ui_uni = u_a.clone();
                        if let Some(ua) = ui_uni.upgrade() {
                            if ua.get_unified_mode() {
                                let base = if let Some(mac) = get_bluetooth_mac(&an) {
                                    mac.replace(":", "_")
                                } else {
                                    an.replace("alsa_output", "alsa_input").replace(".sink", ".source")
                                };
                                let src = sr_a.lock().unwrap();
                                for s in src.iter() {
                                    if s.name.contains(&base) {
                                        let _ = apply_device_change("sources", &s.name);
                                        c.last_source = Some(s.name.clone());
                                        break;
                                    }
                                }
                            }
                        }
                        save_config(&c);
                        let ui_final = u_a.clone();
                        let _ = slint::invoke_from_event_loop(move || { if let Some(ua) = ui_final.upgrade() { ua.set_status(format!("{} - {}", ta.status_applied, chrono::Local::now().format("%H:%M:%S")).into()); } });
                    });
                }
            }
            if !u.get_unified_mode() && src_i >= 0 {
                let srcs = src_c.lock().unwrap();
                if (src_i as usize) < srcs.len() {
                    let n = srcs[src_i as usize].name.clone();
                    let cf_a = Arc::clone(&cfg_handler);
                    let u_a = ui_weak_handler.clone();
                    thread::spawn(move || {
                        let ta = get_current_translations();
                        let mut an = n.clone();
                        if n.starts_with("bluez_connect.") {
                            let mac = n.replace("bluez_connect.", "");
                            let ui_conn = u_a.clone();
                            let _ = slint::invoke_from_event_loop(move || { if let Some(ua) = ui_conn.upgrade() { ua.set_status(ta.status_connecting.into()); } });
                            
                            append_log(&format!("BT (Source): Starting connection sequence for {}", mac));
                            
                            let mut found = false;
                            for attempt in 1..=3 {
                                append_log(&format!("BT (Source): Connection attempt {}/3 for {}", attempt, mac));

                                if attempt == 2 {
                                    append_log("BT (Source): Attempting trust and brief scan to wake up device...");
                                    let _ = Command::new("bluetoothctl").args(["trust", &mac]).status();
                                    let _ = Command::new("bluetoothctl").args(["scan", "on"]).spawn();
                                    thread::sleep(std::time::Duration::from_millis(1500));
                                    let _ = Command::new("bluetoothctl").args(["scan", "off"]).status();
                                }

                                let output = Command::new("bluetoothctl").args(["connect", &mac]).output();
                                match output {
                                    Ok(o) if o.status.success() => {
                                        append_log("BT (Source): bluetoothctl connect returned success.");
                                    }
                                    Ok(o) => {
                                        let out = String::from_utf8_lossy(&o.stdout);
                                        let err = String::from_utf8_lossy(&o.stderr);
                                        append_log(&format!("BT (Source): connect failed. Out: {} Err: {}", out.trim(), err.trim()));
                                    }
                                    Err(e) => {
                                        append_log(&format!("BT (Source): Failed to execute bluetoothctl: {}", e));
                                    }
                                }

                                // Give PulseAudio/PipeWire time to see the device
                                thread::sleep(std::time::Duration::from_millis(1500 * attempt as u64));
                                
                                if let Ok(p) = get_pactl_devices("sources") {
                                    if let Some(f) = p.iter().find(|s| s.name.contains(&mac.replace(":", "_"))) { 
                                        an = f.name.clone(); 
                                        found = true;
                                        append_log(&format!("BT (Source): Device found in pactl sources: {}", an));
                                        break;
                                    }
                                }
                                append_log("BT (Source): Device not yet in pactl sources.");
                            }

                            let ui_status = u_a.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ua) = ui_status.upgrade() {
                                    let t = get_current_translations();
                                    ua.set_status((if found { t.status_connected } else { t.status_failed }).into());
                                    if found { ua.invoke_refresh(); }
                                }
                            });
                            
                            if !found { 
                                append_log("BT (Source): Connection failed after all attempts.");
                                return; 
                            }
                        }
                        let _ = apply_device_change("sources", &an);
                        let mut c = cf_a.lock().unwrap();
                        c.last_source = Some(an);
                        save_config(&c);
                        let ui_final = u_a.clone();
                        let _ = slint::invoke_from_event_loop(move || { if let Some(ua) = ui_final.upgrade() { ua.set_status(format!("{} - {}", ta.status_applied, chrono::Local::now().format("%H:%M:%S")).into()); } });
                    });
                }
            }
        }
    };
    
    let h1 = handler.clone(); ui.on_sink_changed(move |idx| h1(idx, -1));
    let h2 = handler.clone(); ui.on_source_changed(move |idx| h2(-1, idx));

    let s_vol = Arc::clone(&sinks_cache);
    let ui_v_ref = ui_weak.clone();
    ui.on_sink_volume_changed(move |v| {
        if let Some(u) = ui_v_ref.upgrade() {
            let idx = u.get_selected_sink_index();
            if idx >= 0 {
                let s = s_vol.clone();
                thread::spawn(move || {
                    let sinks = s.lock().unwrap();
                    if (idx as usize) < sinks.len() {
                        let n = &sinks[idx as usize].name;
                        if !n.starts_with("bluez_connect.") { let _ = set_sink_volume(n, v); }
                    }
                });
            }
        }
    });

    ui.run()?;

    append_log("Application shutting down.");
    let mut c = cfg_arc.lock().unwrap();
    let sz = ui.window().size();
    c.window_width = Some(sz.width as f32);
    c.window_height = Some(sz.height as f32);
    let p = ui.window().position();
    c.window_x = Some(p.x);
    c.window_y = Some(p.y);
    save_config(&c);
    append_log("Shutdown complete.");
    Ok(())
    }