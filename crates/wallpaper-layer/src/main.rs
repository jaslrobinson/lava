use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use webkit2gtk::{gio, WebView, WebViewExt};

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

    let webview = WebView::new();

    // Once the page finishes loading, inject the project data via JS
    if let Some(json) = project_json {
        eprintln!("[klwp-wallpaper] Will inject project ({} bytes) after page load", json.len());
        webview.connect_load_changed(move |wv, event| {
            if event == webkit2gtk::LoadEvent::Finished {
                // Valid JSON is valid JS, so we can assign directly
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
