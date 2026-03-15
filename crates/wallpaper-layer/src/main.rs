use gtk::prelude::*;
use gtk::gdk;
use gtk_layer_shell::LayerShell;
use webkit2gtk::{gio, UserContentManager, UserContentManagerExt, WebContext, WebView, WebViewExt, WebViewExtManual, WebsiteDataManager};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--standalone") {
        let dev_mode = args.iter().any(|a| a == "--dev");
        run_standalone(dev_mode);
    } else {
        // Legacy mode: lava-wallpaper <url> [project-file]
        let url = args.get(1).cloned().unwrap_or_else(|| {
            eprintln!("Usage: lava-wallpaper <url> [project-file]");
            eprintln!("       lava-wallpaper --standalone [--dev]");
            std::process::exit(1);
        });
        let project_path = args.get(2).cloned();
        run_legacy(url, project_path);
    }
}

// ---------------------------------------------------------------------------
// Legacy mode (editor-spawned)
// ---------------------------------------------------------------------------

fn run_legacy(url: String, project_path: Option<String>) {
    let project_json = project_path.and_then(|p| {
        std::fs::read_to_string(&p)
            .map_err(|e| eprintln!("[lava-wallpaper] Failed to read project file: {}", e))
            .ok()
    });
    run_gtk_wallpaper(&url, project_json);
}

// ---------------------------------------------------------------------------
// Standalone mode (self-sufficient, no editor needed)
// ---------------------------------------------------------------------------

fn run_standalone(dev_mode: bool) {
    use lava_core::{settings, server, audio, pid, providers, plugins};
    use lava_core::providers::manager::ProviderManager;

    eprintln!("[lava-wallpaper] Starting in standalone mode");

    // 1. Write PID file
    pid::write_pid();

    // 2. Acquire provider master lock
    if !pid::try_acquire_provider_lock() {
        eprintln!("[lava-wallpaper] Warning: another process holds the provider lock");
    }

    // 3. Load project
    let project_path = settings::last_project_path().unwrap_or_else(|| {
        eprintln!("[lava-wallpaper] No lastProjectPath in settings");
        std::process::exit(1);
    });
    let project_json = std::fs::read_to_string(&project_path).unwrap_or_else(|e| {
        eprintln!("[lava-wallpaper] Failed to read project {}: {}", project_path, e);
        std::process::exit(1);
    });

    // 4. Start HTTP server (or use Vite dev server)
    let wallpaper_url = if dev_mode {
        eprintln!("[lava-wallpaper] Dev mode: using Vite at localhost:1420");
        "http://localhost:1420?wallpaper=true".to_string()
    } else {
        let dist_dir = settings::find_dist_dir().unwrap_or_else(|| {
            eprintln!("[lava-wallpaper] Could not find frontend dist directory");
            std::process::exit(1);
        });
        let server_url = server::start_wallpaper_server(dist_dir).unwrap_or_else(|e| {
            eprintln!("[lava-wallpaper] Failed to start HTTP server: {}", e);
            std::process::exit(1);
        });
        format!("{}?wallpaper=true", server_url)
    };

    // 5. Start providers
    let mut manager = ProviderManager::new();
    manager.register(Box::new(providers::datetime::DateTimeProvider));
    manager.register(Box::new(providers::battery::BatteryProvider::new()));
    manager.register(Box::new(providers::sysinfo_provider::SysInfoProvider::new()));
    manager.register(Box::new(providers::resource_monitor::ResourceMonitorProvider::new()));
    manager.register(Box::new(providers::music::MusicProvider));
    manager.register(Box::new(providers::network::NetworkProvider));
    manager.register(Box::new(providers::traffic::TrafficProvider::new()));
    let (weather, forecast) = providers::weather::create_providers();
    manager.register(Box::new(weather));
    manager.register(Box::new(forecast));
    manager.register(Box::new(providers::radar::RadarProvider));
    manager.register(Box::new(providers::hyprland::HyprlandProvider::new()));

    for provider in plugins::load_plugins() {
        manager.register(provider);
    }

    let data = manager.data();
    manager.register(Box::new(providers::ai::AiProvider::new(data.clone())));

    let _provider_handle = manager.start(
        None::<fn(&std::collections::HashMap<String, std::collections::HashMap<String, String>>, bool)>,
    );

    // 6. Start Hyprland event listener
    providers::hyprland::HyprlandProvider::start_event_listener(data, None);

    // 7. Start audio capture
    let audio_bands = audio::new_shared_bands();
    audio::start_audio_capture(|_bands| { /* no-op callback */ }, audio_bands);

    // 8. Create GTK window + WebKit with standalone URL + project
    run_gtk_wallpaper(&wallpaper_url, Some(project_json));

    // Cleanup on exit
    pid::cleanup_pid();
    pid::release_provider_lock();
}

// ---------------------------------------------------------------------------
// Shared GTK/WebKit wallpaper window
// ---------------------------------------------------------------------------

fn run_gtk_wallpaper(url: &str, project_json: Option<String>) {
    gtk::init().expect("Failed to init GTK");

    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    // Init layer shell BEFORE the window is realized
    window.init_layer_shell();
    window.set_layer(gtk_layer_shell::Layer::Bottom);
    window.set_anchor(gtk_layer_shell::Edge::Top, true);
    window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
    window.set_anchor(gtk_layer_shell::Edge::Left, true);
    window.set_anchor(gtk_layer_shell::Edge::Right, true);
    window.set_exclusive_zone(-1);
    window.set_namespace("lava-wallpaper");

    // Set up a UserContentManager with a message handler so JS can call into Rust.
    // Frontend uses: window.webkit.messageHandlers.lava.postMessage(jsonString)
    let ucm = UserContentManager::new();
    ucm.register_script_message_handler("lava");

    ucm.connect_script_message_received(Some("lava"), |_ucm, js_result| {
        if let Some(js_val) = js_result.js_value() {
            use javascriptcore::ValueExt;
            let msg = js_val.to_str();
            handle_message(&msg);
        }
    });

    // Use a separate WebKit data directory to avoid conflicts with the editor
    let data_manager = WebsiteDataManager::new_ephemeral();
    let web_context = WebContext::with_website_data_manager(&data_manager);
    let webview = WebView::new_with_context_and_user_content_manager(&web_context, &ucm);

    // Handle web process crashes — reload instead of showing blank
    {
        let url_for_reload = url.to_string();
        webview.connect_web_process_terminated(move |wv: &WebView, reason| {
            eprintln!("[lava-wallpaper] Web process terminated (reason: {:?})! Reloading in 1s...", reason);
            let url = url_for_reload.clone();
            let wv = wv.clone();
            gtk::glib::timeout_add_local_once(std::time::Duration::from_secs(1), move || {
                wv.load_uri(&url);
            });
        });
    }

    // Once the page finishes loading, inject the project data via JS
    if let Some(json) = project_json {
        eprintln!("[lava-wallpaper] Will inject project ({} bytes) after page load", json.len());
        webview.connect_load_changed(move |wv: &WebView, event| {
            if event == webkit2gtk::LoadEvent::Finished {
                let js = format!("window.__LAVA_PROJECT = {};", json);
                eprintln!("[lava-wallpaper] Injecting project data now");
                wv.run_javascript(&js, None::<&gio::Cancellable>, |result| {
                    match result {
                        Ok(_) => eprintln!("[lava-wallpaper] Project injection succeeded"),
                        Err(e) => eprintln!("[lava-wallpaper] Project injection failed: {}", e),
                    }
                });
            }
        });
    }

    // Make the WebKit background fully transparent so the GTK window
    // can be a "clear" layer surface when content is hidden via JS opacity
    let transparent = gdk::RGBA::new(0.0, 0.0, 0.0, 0.0);
    webview.set_background_color(&transparent);

    webview.load_uri(url);

    window.add(&webview);
    window.show_all();

    // Poll /tmp/lava-wallpaper-opacity to fade wallpaper when apps are focused.
    // We control visibility via JS document.body.style.opacity instead of
    // GTK window.set_opacity() — this avoids compositor flickering on other
    // layer-shell surfaces (e.g. Quickshell bars) while keeping the surface
    // mapped so WebKitGTK doesn't suspend rendering.
    {
        let wv = webview.clone();
        let mut last_opacity: f64 = 1.0;
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
            let target = std::fs::read_to_string("/tmp/lava-wallpaper-opacity")
                .ok()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);

            if (target - last_opacity).abs() > 0.01 {
                let js = format!("document.body.style.opacity='{:.2}'", target);
                wv.run_javascript(&js, None::<&gio::Cancellable>, |_| {});
                last_opacity = target;
            }

            gtk::glib::ControlFlow::Continue
        });
    }

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        gtk::glib::Propagation::Stop
    });

    gtk::main();
}

// ---------------------------------------------------------------------------
// Message handling (JS -> Rust bridge)
// ---------------------------------------------------------------------------

/// Handle a message from the frontend JS.
/// Messages are JSON strings like: {"type":"open_url","url":"https://..."}
fn handle_message(msg: &str) {
    match extract_json_field(msg, "type") {
        Some("open_url") => {
            if let Some(url) = extract_json_field(msg, "url") {
                if url.starts_with("http://") || url.starts_with("https://") {
                    eprintln!("[lava-wallpaper] Opening URL: {}", url);
                    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                }
            }
        }
        Some("launch_app") => {
            if let Some(cmd) = extract_json_field(msg, "command") {
                eprintln!("[lava-wallpaper] Launching app: {}", cmd);
                let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
            }
        }
        Some("show_editor") => {
            eprintln!("[lava-wallpaper] Writing show-editor signal");
            let _ = std::fs::write("/tmp/lava-show-editor", "1");
            // In standalone mode, also try to launch the editor if not running
            if !is_editor_running() {
                if let Some(editor) = find_editor_binary() {
                    eprintln!("[lava-wallpaper] Launching editor: {:?}", editor);
                    let _ = std::process::Command::new(editor).spawn();
                }
            }
        }
        Some("adjust_volume") => {
            if let Some(delta_str) = extract_json_field(msg, "delta") {
                if let Ok(delta) = delta_str.parse::<i32>() {
                    let arg = if delta >= 0 {
                        format!("{}%+", delta)
                    } else {
                        format!("{}%-", -delta)
                    };
                    let _ = std::process::Command::new("wpctl")
                        .args(["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &arg])
                        .spawn();
                }
            }
        }
        Some("music_control") => {
            if let Some(action) = extract_json_field(msg, "action") {
                let method = match action {
                    "play-pause" => "PlayPause",
                    "play" => "Play",
                    "pause" => "Pause",
                    "stop" => "Stop",
                    "next" => "Next",
                    "previous" => "Previous",
                    _ => {
                        eprintln!("[lava-wallpaper] Unknown music action: {}", action);
                        return;
                    }
                };
                eprintln!("[lava-wallpaper] Music control: {}", method);
                let _ = std::process::Command::new("dbus-send")
                    .args([
                        "--print-reply",
                        "--dest=org.mpris.MediaPlayer2.playerctld",
                        "/org/mpris/MediaPlayer2",
                        &format!("org.mpris.MediaPlayer2.Player.{}", method),
                    ])
                    .spawn();
            }
        }
        Some("launch_command") => {
            if let Some(cmd) = extract_json_field(msg, "command") {
                eprintln!("[lava-wallpaper] Running command: {}", cmd);
                let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
            }
        }
        _ => {
            // Legacy: no type field -- try url field directly
            if let Some(url) = extract_json_field(msg, "url") {
                if url.starts_with("http://") || url.starts_with("https://") {
                    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Editor discovery (for standalone show_editor)
// ---------------------------------------------------------------------------

/// Check if the editor process is still running via its PID file.
fn is_editor_running() -> bool {
    std::fs::read_to_string("/tmp/lava-editor.pid")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map(|pid| std::path::Path::new(&format!("/proc/{}", pid)).exists())
        .unwrap_or(false)
}

/// Try to find the `lava` editor binary.
fn find_editor_binary() -> Option<std::path::PathBuf> {
    // Check next to our own binary first
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("lava");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    // Check well-known system paths
    for path in ["/usr/bin/lava", "/usr/local/bin/lava"] {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a string field value from a simple JSON object.
fn extract_json_field<'a>(json: &'a str, field: &str) -> Option<&'a str> {
    let key = format!("\"{}\"", field);
    let start = json.find(&key)? + key.len();
    let rest = &json[start..];
    let quote_start = rest.find('"')? + 1;
    let inner = &rest[quote_start..];
    let quote_end = inner.find('"')?;
    Some(&inner[..quote_end])
}
