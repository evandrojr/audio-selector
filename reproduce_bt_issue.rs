use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    let mac = "44:6D:7F:4B:57:00"; // Echo Dot from logs
    
    println!("Testing connection with scan on...");
    let _ = Command::new("bluetoothctl").args(["scan", "on"]).spawn().expect("Failed to spawn scan on");
    thread::sleep(Duration::from_secs(2));
    
    let status = Command::new("bluetoothctl").args(["connect", mac]).status().expect("Failed to run connect");
    println!("Connect status with scan on: {}", status);
    
    let _ = Command::new("bluetoothctl").args(["scan", "off"]).status();
    
    thread::sleep(Duration::from_secs(5));
    
    println!("Testing connection with scan off...");
    let status = Command::new("bluetoothctl").args(["connect", mac]).status().expect("Failed to run connect");
    println!("Connect status with scan off: {}", status);
}
