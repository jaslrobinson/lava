use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use webkit2gtk::{gio, UserContentManager, UserContentManagerExt, WebView, WebViewExt};

fn main() {
    let url = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: lava-wallpaper <url> [project-file]");
        std::process::exit(1);
    });
    let project_path = std::env::args().nth(2);

    // Read project JSON if provided
    let project_json = project_path.and_then(|p| {
        std::fs::read_to_string(&p)
            .map_err(|e| eprintln!("[lava-wallpaper] Failed to read project file: {}", e))
            .ok()
    });

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

    let webview = WebView::with_user_content_manager(&ucm);

    // Once the page finishes loading, inject the project data via JS
    if let Some(json) = project_json {
        eprintln!("[lava-wallpaper] Will inject project ({} bytes) after page load", json.len());
        webview.connect_load_changed(move |wv, event| {
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

    webview.load_uri(&url);

    window.add(&webview);
    window.show_all();

    // Poll /tmp/lava-wallpaper-opacity to fade wallpaper when apps are focused
    {
        let win = window.clone();
        let mut last_opacity: f64 = 1.0;
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
            let target = std::fs::read_to_string("/tmp/lava-wallpaper-opacity")
                .ok()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);

            if (target - last_opacity).abs() > 0.01 {
                win.set_opacity(target);
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
        Some("launch_command") => {
            if let Some(cmd) = extract_json_field(msg, "command") {
                eprintln!("[lava-wallpaper] Running command: {}", cmd);
                let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
            }
        }
        _ => {
            // Legacy: no type field — try url field directly
            if let Some(url) = extract_json_field(msg, "url") {
                if url.starts_with("http://") || url.starts_with("https://") {
                    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                }
            }
        }
    }
}

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
