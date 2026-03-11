use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use webkit2gtk::{gio, UserContentManager, UserContentManagerExt, WebView, WebViewExt};

fn main() {
    let url = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: klwp-wallpaper <url> [project-file]");
        std::process::exit(1);
    });
    let project_path = std::env::args().nth(2);

    // Read project JSON if provided
    let project_json = project_path.and_then(|p| {
        std::fs::read_to_string(&p)
            .map_err(|e| eprintln!("[klwp-wallpaper] Failed to read project file: {}", e))
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
    window.set_namespace("klwp-wallpaper");

    // Set up a UserContentManager with a message handler so JS can call into Rust.
    // Frontend uses: window.webkit.messageHandlers.klwp.postMessage(jsonString)
    let ucm = UserContentManager::new();
    ucm.register_script_message_handler("klwp");

    ucm.connect_script_message_received(Some("klwp"), |_ucm, js_result| {
        if let Some(js_val) = js_result.js_value() {
            use javascriptcore::ValueExt;
            let msg = js_val.to_str();
            handle_message(&msg);
        }
    });

    let webview = WebView::with_user_content_manager(&ucm);

    // Once the page finishes loading, inject the project data via JS
    if let Some(json) = project_json {
        eprintln!("[klwp-wallpaper] Will inject project ({} bytes) after page load", json.len());
        webview.connect_load_changed(move |wv, event| {
            if event == webkit2gtk::LoadEvent::Finished {
                let js = format!("window.__KLWP_PROJECT = {};", json);
                eprintln!("[klwp-wallpaper] Injecting project data now");
                wv.run_javascript(&js, None::<&gio::Cancellable>, |result| {
                    match result {
                        Ok(_) => eprintln!("[klwp-wallpaper] Project injection succeeded"),
                        Err(e) => eprintln!("[klwp-wallpaper] Project injection failed: {}", e),
                    }
                });
            }
        });
    }

    webview.load_uri(&url);

    window.add(&webview);
    window.show_all();

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
                    eprintln!("[klwp-wallpaper] Opening URL: {}", url);
                    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
                }
            }
        }
        Some("launch_app") => {
            if let Some(cmd) = extract_json_field(msg, "command") {
                eprintln!("[klwp-wallpaper] Launching app: {}", cmd);
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
