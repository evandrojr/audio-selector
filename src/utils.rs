use std::path::PathBuf;
use std::fs;
use std::io::Read;

pub fn get_config_dir() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
    p.push("audio-selector");
    if !p.exists() { let _ = fs::create_dir_all(&p); }
    p
}

pub fn get_config_path() -> PathBuf { get_config_dir().join("config.json") }
pub fn get_log_path() -> PathBuf { get_config_dir().join("debug.log") }

pub fn append_log(msg: &str) {
    use std::io::Write;
    let log_msg = format!("{} - {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
    print!("{}", log_msg); // Print to terminal as well
    let path = get_log_path();
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(log_msg.as_bytes());
    }
}

pub fn get_log_content(search: &str) -> String {
    let path = get_log_path();
    if let Ok(mut f) = fs::File::open(&path) {
        let mut c = String::new();
        if f.read_to_string(&mut c).is_ok() {
            if search.is_empty() {
                let lines: Vec<&str> = c.lines().rev().take(100).collect();
                return lines.into_iter().rev().collect::<Vec<&str>>().join("\n");
            } else {
                let s = search.to_lowercase();
                let f: Vec<&str> = c.lines().filter(|l| l.to_lowercase().contains(&s)).rev().take(100).collect();
                return f.into_iter().rev().collect::<Vec<&str>>().join("\n");
            }
        }
    }
    "No logs.".to_string()
}
