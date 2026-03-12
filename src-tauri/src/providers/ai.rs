use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{DataProvider, ProviderData, SharedProviderData};

pub struct AiProvider {
    shared_data: SharedProviderData,
    last_artist: String,
    current_image: Arc<Mutex<String>>,
    status: Arc<Mutex<String>>,
    generating: Arc<Mutex<bool>>,
}

impl AiProvider {
    pub fn new(shared_data: SharedProviderData) -> Self {
        Self {
            shared_data,
            last_artist: String::new(),
            current_image: Arc::new(Mutex::new(String::new())),
            status: Arc::new(Mutex::new("idle".into())),
            generating: Arc::new(Mutex::new(false)),
        }
    }

    fn api_key() -> Option<String> {
        let path = dirs::config_dir()?.join("lava").join("gemini_api_key");
        let key = fs::read_to_string(path).ok()?;
        let key = key.trim().to_string();
        if key.is_empty() { None } else { Some(key) }
    }

    fn cache_dir() -> Option<PathBuf> {
        let dir = dirs::cache_dir()?.join("lava").join("ai-images");
        fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    fn cache_path(artist: &str) -> Option<PathBuf> {
        let safe: String = artist
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
            .collect();
        let safe = safe.trim_matches('_').to_string();
        Some(Self::cache_dir()?.join(format!("{}.png", safe)))
    }

    fn fetch_and_save(artist: String, api_key: String, prompt: String) -> Option<String> {
        let path = Self::cache_path(&artist)?;
        if path.exists() {
            return Some(format!("file://{}", path.display()));
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent?key={}",
            api_key
        );

        eprintln!("[ai] Prompt: {}", prompt);

        let body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "responseModalities": ["TEXT", "IMAGE"]
            }
        });

        let resp = match ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(body)
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[ai] API request failed: {}", e);
                return None;
            }
        };

        let json: serde_json::Value = match resp.into_json() {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[ai] Failed to parse response JSON: {}", e);
                return None;
            }
        };

        // Try candidates[0].content.parts[*].inlineData.data
        let parts = match json["candidates"][0]["content"]["parts"].as_array() {
            Some(p) => p,
            None => {
                eprintln!("[ai] No candidates/parts in response: {}", json);
                return None;
            }
        };
        let b64 = match parts.iter().find_map(|p| p["inlineData"]["data"].as_str()) {
            Some(d) => d,
            None => {
                eprintln!("[ai] No inlineData in parts: {:?}", parts.iter().map(|p| p.to_string().chars().take(120).collect::<String>()).collect::<Vec<_>>());
                return None;
            }
        };

        use base64::Engine;
        let bytes = match base64::engine::general_purpose::STANDARD.decode(b64) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[ai] Base64 decode failed: {}", e);
                return None;
            }
        };
        if let Err(e) = fs::write(&path, &bytes) {
            eprintln!("[ai] Failed to write image to {:?}: {}", path, e);
            return None;
        }

        Some(format!("file://{}", path.display()))
    }

    fn current_artist(&self) -> String {
        let handle = tokio::runtime::Handle::current();
        let shared = self.shared_data.clone();
        handle.block_on(async move {
            shared
                .read()
                .await
                .get("mi")
                .and_then(|m| m.get("artist"))
                .cloned()
                .unwrap_or_default()
        })
    }

    fn get_prompt(&self, artist: &str) -> String {
        let handle = tokio::runtime::Handle::current();
        let shared = self.shared_data.clone();
        let template = handle.block_on(async move {
            shared
                .read()
                .await
                .get("gv")
                .and_then(|g| g.get("ai_prompt"))
                .cloned()
                .unwrap_or_default()
        });

        if template.is_empty() {
            format!(
                "Create a vivid, artistic portrait of the music artist \"{}\". \
                High quality, studio lighting, detailed, professional photo style.",
                artist
            )
        } else {
            template.replace("<artist>", artist)
        }
    }
}

impl DataProvider for AiProvider {
    fn prefix(&self) -> &str {
        "ai"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();

        let artist = self.current_artist();

        // Update status/image from background thread results
        let image = self.current_image.lock().unwrap().clone();

        if artist.is_empty() {
            data.insert("artistImage".into(), image);
            data.insert("status".into(), "no music".into());
            return data;
        }

        // Check if artist changed
        if artist != self.last_artist {
            self.last_artist = artist.clone();

            // Check cache synchronously first (instant)
            if let Some(path) = Self::cache_path(&artist) {
                if path.exists() {
                    let file_url = format!("file://{}", path.display());
                    *self.current_image.lock().unwrap() = file_url.clone();
                    *self.status.lock().unwrap() = "ready".into();
                    data.insert("artistImage".into(), file_url);
                    data.insert("status".into(), "ready".into());
                    return data;
                }
            }

            // Not cached — spawn background thread for API call
            let already_generating = {
                let mut g = self.generating.lock().unwrap();
                if *g { true } else { *g = true; false }
            };

            if !already_generating {
                if let Some(api_key) = Self::api_key() {
                    let image_ref = self.current_image.clone();
                    let status_ref = self.status.clone();
                    let generating_ref = self.generating.clone();
                    let artist_clone = artist.clone();
                    let prompt = self.get_prompt(&artist);

                    *status_ref.lock().unwrap() = "generating".into();

                    std::thread::spawn(move || {
                        let result = Self::fetch_and_save(artist_clone, api_key, prompt);
                        if let Some(path) = result {
                            *image_ref.lock().unwrap() = path;
                            *status_ref.lock().unwrap() = "ready".into();
                        } else {
                            *status_ref.lock().unwrap() = "error".into();
                        }
                        *generating_ref.lock().unwrap() = false;
                    });
                } else {
                    *self.generating.lock().unwrap() = false;
                    *self.status.lock().unwrap() = "no api key".into();
                }
            }
        }

        data.insert("artistImage".into(), self.current_image.lock().unwrap().clone());
        data.insert("status".into(), self.status.lock().unwrap().clone());
        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}
