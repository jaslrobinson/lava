use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};

static SERVER_PORT: AtomicU16 = AtomicU16::new(0);

/// Get the wallpaper server URL, or None if not running.
pub fn get_server_url() -> Option<String> {
    let port = SERVER_PORT.load(Ordering::Relaxed);
    if port == 0 { None } else { Some(format!("http://127.0.0.1:{}", port)) }
}

/// Start a lightweight HTTP server that serves the frontend dist files
/// and proxies provider/audio/asset data for the wallpaper WebKitGTK process.
/// Returns the base URL (e.g., "http://127.0.0.1:9847").
pub fn start_wallpaper_server(dist_dir: PathBuf) -> Result<String, String> {
    if let Some(url) = get_server_url() {
        return Ok(url);
    }

    // Bind to port 0 to let the OS pick a free port
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| format!("Failed to start wallpaper server: {}", e))?;

    let port = server.server_addr().to_ip()
        .map(|a| a.port())
        .unwrap_or(0);

    if port == 0 {
        return Err("Failed to bind wallpaper server".into());
    }

    SERVER_PORT.store(port, Ordering::Relaxed);
    let url = format!("http://127.0.0.1:{}", port);
    eprintln!("[wallpaper-server] Serving dist from {:?} on {}", dist_dir, url);

    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let url_path = request.url().to_string();

            if url_path.starts_with("/__lava_providers") {
                serve_json_file(request, "lava-provider-data.json", "{}");
            } else if url_path.starts_with("/__lava_audio") {
                serve_json_file(request, "lava-audio-bands.json", "[]");
            } else if url_path.starts_with("/__lava_assets") {
                serve_asset(request, &url_path);
            } else {
                serve_static(request, &dist_dir, &url_path);
            }
        }
        SERVER_PORT.store(0, Ordering::Relaxed);
        eprintln!("[wallpaper-server] Server stopped");
    });

    Ok(format!("http://127.0.0.1:{}", port))
}

#[allow(dead_code)]
pub fn stop_wallpaper_server() {
    SERVER_PORT.store(0, Ordering::Relaxed);
}

fn serve_json_file(request: tiny_http::Request, filename: &str, fallback: &str) {
    let path = std::env::temp_dir().join(filename);
    let data = std::fs::read_to_string(&path).unwrap_or_else(|_| fallback.to_string());
    let response = tiny_http::Response::from_string(data)
        .with_header("Content-Type: application/json".parse::<tiny_http::Header>().unwrap())
        .with_header("Cache-Control: no-cache".parse::<tiny_http::Header>().unwrap())
        .with_header("Access-Control-Allow-Origin: *".parse::<tiny_http::Header>().unwrap());
    let _ = request.respond(response);
}

fn serve_asset(request: tiny_http::Request, url_path: &str) {
    let file_path = urlencoding_decode(&url_path["/__lava_assets".len()..]);
    let path = Path::new(&file_path);

    if !path.is_absolute() || !path.exists() || !path.is_file() {
        let response = tiny_http::Response::from_string("Not found")
            .with_status_code(404);
        let _ = request.respond(response);
        return;
    }

    let content_type = mime_for_path(path);
    match std::fs::read(path) {
        Ok(data) => {
            let response = tiny_http::Response::from_data(data)
                .with_header(format!("Content-Type: {}", content_type).parse::<tiny_http::Header>().unwrap())
                .with_header("Cache-Control: max-age=3600".parse::<tiny_http::Header>().unwrap())
                .with_header("Access-Control-Allow-Origin: *".parse::<tiny_http::Header>().unwrap());
            let _ = request.respond(response);
        }
        Err(_) => {
            let _ = request.respond(tiny_http::Response::from_string("Read error").with_status_code(500));
        }
    }
}

fn serve_static(request: tiny_http::Request, dist_dir: &Path, url_path: &str) {
    // Strip query string
    let path_part = url_path.split('?').next().unwrap_or("/");
    let relative = if path_part == "/" { "index.html" } else { path_part.trim_start_matches('/') };
    let file_path = dist_dir.join(relative);

    // Security: ensure resolved path is within dist_dir
    if let Ok(canonical) = file_path.canonicalize() {
        if let Ok(base) = dist_dir.canonicalize() {
            if !canonical.starts_with(&base) {
                let _ = request.respond(tiny_http::Response::from_string("Forbidden").with_status_code(403));
                return;
            }
        }
    }

    if file_path.exists() && file_path.is_file() {
        let content_type = mime_for_path(&file_path);
        match std::fs::read(&file_path) {
            Ok(data) => {
                let response = tiny_http::Response::from_data(data)
                    .with_header(format!("Content-Type: {}", content_type).parse::<tiny_http::Header>().unwrap())
                    .with_header("Cache-Control: no-cache".parse::<tiny_http::Header>().unwrap());
                let _ = request.respond(response);
            }
            Err(_) => {
                let _ = request.respond(tiny_http::Response::from_string("Read error").with_status_code(500));
            }
        }
    } else {
        // SPA fallback: serve index.html for non-file routes
        let index = dist_dir.join("index.html");
        if let Ok(data) = std::fs::read(&index) {
            let response = tiny_http::Response::from_data(data)
                .with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
            let _ = request.respond(response);
        } else {
            let _ = request.respond(tiny_http::Response::from_string("Not found").with_status_code(404));
        }
    }
}

fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html",
        "js" => "application/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().unwrap_or(b'0');
            let lo = chars.next().unwrap_or(b'0');
            let val = hex_val(hi) * 16 + hex_val(lo);
            result.push(val as char);
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}
