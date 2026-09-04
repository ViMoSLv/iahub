#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Manager;

/// State shared between Tauri commands
struct AppState {
    /// Whether the backend sidecar is running
    backend_running: Arc<AtomicBool>,
    /// The port the backend is listening on
    backend_port: Arc<std::sync::Mutex<Option<u16>>>,
}

#[tauri::command]
fn get_backend_port(state: tauri::State<AppState>) -> Option<u16> {
    *state.backend_port.lock().unwrap()
}

#[tauri::command]
fn is_backend_running(state: tauri::State<AppState>) -> bool {
    state.backend_running.load(Ordering::SeqCst)
}

fn main() {
    let backend_running = Arc::new(AtomicBool::new(false));
    let backend_port = Arc::new(std::sync::Mutex::new(None));

    let app_state = AppState {
        backend_running: backend_running.clone(),
        backend_port: backend_port.clone(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            // Spawn the mega-brain-server sidecar
            let running = backend_running.clone();
            let port_holder = backend_port.clone();
            let app_handle = app.handle().clone();

            std::thread::spawn(move || {
                // Try to find the server binary
                let server_bin = if cfg!(debug_assertions) {
                    // In dev mode, look for cargo-built binary
                    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
                    let workspace_root = std::path::Path::new(&manifest_dir)
                        .parent()
                        .and_then(|p| p.parent())
                        .unwrap_or(std::path::Path::new("."));
                    workspace_root.join("target").join("debug").join("mega-brain-server.exe")
                } else {
                    // In production, look for sidecar in resources
                    app_handle.path()
                        .resource_dir()
                        .map(|p| p.join("mega-brain-server.exe"))
                        .unwrap_or_else(|_| std::path::PathBuf::from("mega-brain-server.exe"))
                };

                eprintln!("[Tauri] Looking for server at: {}", server_bin.display());

                if !server_bin.exists() {
                    eprintln!("[Tauri] Server binary not found at {}. Backend will not start.", server_bin.display());
                    eprintln!("[Tauri] Run 'cargo build --bin mega-brain-server' first, or start it manually.");
                    return;
                }

                let mut child = match Command::new(&server_bin)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[Tauri] Failed to spawn server: {}", e);
                        return;
                    }
                };

                running.store(true, Ordering::SeqCst);
                eprintln!("[Tauri] Server spawned with PID {:?}", child.id());

                // Read stdout to discover the port
                if let Some(stdout) = child.stdout.take() {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().flatten() {
                        eprintln!("[Server] {}", line);
                        if line.starts_with("IAHUB_LISTENING ") {
                            if let Some(addr) = line.strip_prefix("IAHUB_LISTENING ") {
                                if let Some(port_str) = addr.split(':').nth(1) {
                                    if let Ok(port) = port_str.trim().parse::<u16>() {
                                        eprintln!("[Tauri] Backend listening on port {}", port);
                                        *port_holder.lock().unwrap() = Some(port);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                // Wait for the child process
                match child.wait() {
                    Ok(status) => {
                        eprintln!("[Tauri] Server exited with status: {}", status);
                    }
                    Err(e) => {
                        eprintln!("[Tauri] Error waiting for server: {}", e);
                    }
                }
                running.store(false, Ordering::SeqCst);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_backend_port, is_backend_running])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}