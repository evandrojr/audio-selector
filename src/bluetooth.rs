use std::process::Command;
use crate::audio::PactlDevice;
use crate::utils::append_log;

#[cfg(target_os = "linux")]
pub fn get_bluetooth_devices() -> Vec<PactlDevice> {
    let output = Command::new("bluetoothctl").arg("devices").output();
    let mut bt = Vec::new();
    if let Ok(o) = output {
        for line in String::from_utf8_lossy(&o.stdout).lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 3 {
                bt.push(PactlDevice {
                    name: format!("bluez_connect.{}", p[1]),
                    description: format!("{} (Bluetooth)", p[2..].join(" ")),
                    volume: None,
                });
            }
        }
    } else if let Err(e) = output {
        append_log(&format!("bluetoothctl failed: {}", e));
    }
    bt
}

#[cfg(not(target_os = "linux"))]
pub fn get_bluetooth_devices() -> Vec<PactlDevice> { Vec::new() }

#[cfg(target_os = "linux")]
pub fn set_bluetooth_power(on: bool) -> anyhow::Result<()> {
    let _ = Command::new("bluetoothctl").args(["power", if on { "on" } else { "off" }]).status();
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn set_bluetooth_power(_: bool) -> anyhow::Result<()> { Ok(()) }
