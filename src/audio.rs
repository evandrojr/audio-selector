use serde::Deserialize;
use std::process::Command;
use crate::utils::append_log;

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct PactlDevice {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub volume: Option<serde_json::Value>,
}

impl PactlDevice {
    pub fn get_volume_percent(&self) -> i32 {
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

#[cfg(target_os = "linux")]
fn run_pactl(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("timeout").args(["10s", "pactl"]).args(args).env("LC_ALL", "C").output()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn run_pactl(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("pactl").args(args).output()
}

#[cfg(target_os = "linux")]
pub fn get_pactl_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    // Use timeout command to prevent long hangs if PulseAudio is stuck
    let output = Command::new("timeout").args(["3s", "pactl", "--format=json", "list", target])
        .env("LC_ALL", "C")
        .output();
        
    match output {
        Ok(o) => {
            if !o.status.success() {
                let err = String::from_utf8_lossy(&o.stderr);
                append_log(&format!("pactl list {} failed (code {}): {}", target, o.status.code().unwrap_or(-1), err));
                return Ok(Vec::new());
            }
            let stdout = String::from_utf8_lossy(&o.stdout);
            let json_start = stdout.find('[').or_else(|| stdout.find('{'));
            let json_str = match json_start { Some(start) => &stdout[start..], None => return Ok(Vec::new()) };
            let devices: Vec<PactlDevice> = serde_json::from_str(json_str.trim()).unwrap_or_default();
            Ok(devices.into_iter().filter(|d| { 
                if target == "sources" { !d.name.contains(".monitor") || d.name.contains("bluez_source") } 
                else { true } 
            }).collect())
        },
        Err(e) => {
            append_log(&format!("pactl execution failed: {}", e));
            Err(e.into())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn get_pactl_devices(target: &str) -> anyhow::Result<Vec<PactlDevice>> {
    crate::audio_windows::get_windows_devices(target)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn get_pactl_devices(_: &str) -> anyhow::Result<Vec<PactlDevice>> { Ok(Vec::new()) }

#[cfg(target_os = "linux")]
fn run_pactl_short(args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    Command::new("timeout").args(["5s", "pactl"]).args(args).env("LC_ALL", "C").status()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn run_pactl_short(args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    Command::new("pactl").args(args).status()
}

#[cfg(target_os = "linux")]
pub fn apply_device_change(target: &str, name: &str) -> anyhow::Result<()> {
    let set_cmd = if target == "sinks" { "set-default-sink" } else { "set-default-source" };
    let _ = run_pactl_short(&[set_cmd, name]);
    let cmd = if target == "sinks" { "move-sink-input" } else { "move-source-output" };
    let list_cmd = if target == "sinks" { "sink-inputs" } else { "source-outputs" };
    
    if let Ok(o) = run_pactl(&["list", "short", list_cmd]) {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            if let Some(id) = line.split_whitespace().next() {
                let _ = run_pactl_short(&[cmd, id, name]);
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn apply_device_change(target: &str, name: &str) -> anyhow::Result<()> {
    crate::audio_windows::apply_windows_device_change(target, name)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn apply_device_change(_: &str, _: &str) -> anyhow::Result<()> { Ok(()) }

#[cfg(target_os = "linux")]
pub fn set_sink_volume(name: &str, vol: i32) -> anyhow::Result<()> {
    let _ = run_pactl_short(&["set-sink-volume", name, &format!("{}%", vol)]);
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_sink_volume(name: &str, vol: i32) -> anyhow::Result<()> {
    crate::audio_windows::set_windows_volume(name, vol)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn set_sink_volume(_: &str, _: i32) -> anyhow::Result<()> { Ok(()) }
