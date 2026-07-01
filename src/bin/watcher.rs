use notify::{Watcher, RecursiveMode, Config, Event};
use std::process::{Command, Child};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!(">>> Starting Rust Watcher for audio-selector...");
    
    let child_process = Arc::new(Mutex::new(None::<Child>));
    let child_clone = Arc::clone(&child_process);

    let run_app = move || {
        let mut child_lock = child_clone.lock().unwrap();
        
        // Kill existing process if running
        if let Some(mut child) = child_lock.take() {
            println!(">>> Change detected, restarting...");
            let _ = child.kill();
            let _ = child.wait();
        }

        // Start new process
        match Command::new("cargo").args(["run", "--bin", "audio-selector"]).spawn() {
            Ok(child) => *child_lock = Some(child),
            Err(e) => eprintln!(">>> Failed to start application: {}", e),
        }
    };

    // Initial run
    let run_initial = run_app.clone();
    run_initial();

    // Setup watcher
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::RecommendedWatcher::new(tx, Config::default())?;

    watcher.watch(Path::new("src"), RecursiveMode::Recursive)?;
    watcher.watch(Path::new("ui"), RecursiveMode::Recursive)?;

    println!(">>> Watching src/ and ui/ for changes...");

    let mut last_event = std::time::Instant::now();
    let debounce_duration = Duration::from_millis(500);

    for res in rx {
        match res {
            Ok(event) => {
                if is_relevant_event(event) {
                    if last_event.elapsed() > debounce_duration {
                        run_app();
                        last_event = std::time::Instant::now();
                    }
                }
            }
            Err(e) => eprintln!(">>> Watcher error: {:?}", e),
        }
    }

    Ok(())
}

fn is_relevant_event(event: Event) -> bool {
    // Only care about data/metadata changes or creation/deletion
    event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove()
}
