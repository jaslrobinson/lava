use std::process::Command;
use std::time::Duration;

use super::{DataProvider, ProviderData};

pub struct LocationProvider {
    /// Cached latitude from last successful fetch
    last_lat: f64,
    /// Cached longitude from last successful fetch
    last_lon: f64,
    /// Cached geocode results (addr, loc, country, countrycode)
    cached_geocode: Option<GeocodeResult>,
}

struct GeocodeResult {
    addr: String,
    loc: String,
    country: String,
    countrycode: String,
}

impl LocationProvider {
    pub fn new() -> Self {
        Self {
            last_lat: 0.0,
            last_lon: 0.0,
            cached_geocode: None,
        }
    }

    /// Try GeoClue2 via gdbus CLI. Returns (lat, lon, alt, speed, accuracy) on success.
    fn try_geoclue2(&self) -> Option<(f64, f64, f64, f64, f64)> {
        // Create a client via GeoClue2 Manager
        let client_output = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                "/org/freedesktop/GeoClue2/Manager",
                "--method",
                "org.freedesktop.GeoClue2.Manager.GetClient",
            ])
            .output()
            .ok()?;

        if !client_output.status.success() {
            return None;
        }

        // Parse client path from output like "('/org/freedesktop/GeoClue2/Client/42',)"
        let client_text = String::from_utf8_lossy(&client_output.stdout);
        let client_path = client_text
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim_end_matches(',')
            .trim_matches('\'');

        if client_path.is_empty() || !client_path.starts_with('/') {
            return None;
        }

        // Set DesktopId property
        let _ = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                client_path,
                "--method",
                "org.freedesktop.DBus.Properties.Set",
                "org.freedesktop.GeoClue2.Client",
                "DesktopId",
                "<'lava'>",
            ])
            .output();

        // Set RequestedAccuracyLevel to EXACT (8)
        let _ = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                client_path,
                "--method",
                "org.freedesktop.DBus.Properties.Set",
                "org.freedesktop.GeoClue2.Client",
                "RequestedAccuracyLevel",
                "<uint32 8>",
            ])
            .output();

        // Start the client
        let start_output = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                client_path,
                "--method",
                "org.freedesktop.GeoClue2.Client.Start",
            ])
            .output()
            .ok()?;

        if !start_output.status.success() {
            return None;
        }

        // Brief pause for location to be acquired
        std::thread::sleep(Duration::from_millis(500));

        // Read the Location property
        let loc_output = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                client_path,
                "--method",
                "org.freedesktop.DBus.Properties.Get",
                "org.freedesktop.GeoClue2.Client",
                "Location",
            ])
            .output()
            .ok()?;

        if !loc_output.status.success() {
            // Stop client before returning
            let _ = stop_geoclue_client(client_path);
            return None;
        }

        let loc_text = String::from_utf8_lossy(&loc_output.stdout);
        let location_path = loc_text
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim_end_matches(',')
            .trim_start_matches("<objectpath ")
            .trim_start_matches("<'")
            .trim_end_matches("'>")
            .trim_matches('\'');

        if location_path.is_empty()
            || !location_path.starts_with('/')
            || location_path == "/"
        {
            let _ = stop_geoclue_client(client_path);
            return None;
        }

        // Read location properties
        let lat = read_geoclue_double(location_path, "Latitude");
        let lon = read_geoclue_double(location_path, "Longitude");
        let alt = read_geoclue_double(location_path, "Altitude");
        let speed = read_geoclue_double(location_path, "Speed");
        let accuracy = read_geoclue_double(location_path, "Accuracy");

        // Stop the client
        let _ = stop_geoclue_client(client_path);

        if let (Some(lat), Some(lon)) = (lat, lon) {
            Some((
                lat,
                lon,
                alt.unwrap_or(0.0),
                speed.unwrap_or(0.0),
                accuracy.unwrap_or(0.0),
            ))
        } else {
            None
        }
    }

    /// Fallback: IP-based geolocation via ipinfo.io
    fn try_ip_geolocation(&self) -> Option<(f64, f64, String, String, String)> {
        let resp: serde_json::Value = ureq::get("https://ipinfo.io/json")
            .set("User-Agent", "lava-desktop")
            .call()
            .ok()?
            .into_json()
            .ok()?;

        let loc_str = resp.get("loc")?.as_str()?;
        let parts: Vec<&str> = loc_str.split(',').collect();
        if parts.len() != 2 {
            return None;
        }
        let lat: f64 = parts[0].parse().ok()?;
        let lon: f64 = parts[1].parse().ok()?;

        let city = resp
            .get("city")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let country = resp
            .get("country")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let region = resp
            .get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Build a rough address from available fields
        let addr = if !city.is_empty() && !region.is_empty() {
            format!("{}, {}", city, region)
        } else if !city.is_empty() {
            city.clone()
        } else {
            region.clone()
        };

        Some((lat, lon, addr, city, country))
    }

    /// Reverse geocode via Nominatim. Only called when location changed significantly.
    fn reverse_geocode(&self, lat: f64, lon: f64) -> Option<GeocodeResult> {
        let url = format!(
            "https://nominatim.openstreetmap.org/reverse?lat={}&lon={}&format=json&zoom=18",
            lat, lon
        );

        let resp: serde_json::Value = ureq::get(&url)
            .set("User-Agent", "lava-desktop")
            .call()
            .ok()?
            .into_json()
            .ok()?;

        let display_name = resp
            .get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let address = resp.get("address");

        let loc = address
            .and_then(|a| {
                a.get("city")
                    .or_else(|| a.get("town"))
                    .or_else(|| a.get("village"))
                    .or_else(|| a.get("hamlet"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let country = address
            .and_then(|a| a.get("country"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let countrycode = address
            .and_then(|a| a.get("country_code"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_uppercase();

        Some(GeocodeResult {
            addr: display_name,
            loc,
            country,
            countrycode,
        })
    }

    /// Check if coordinates changed enough to warrant a new reverse geocode
    fn location_changed_significantly(&self, lat: f64, lon: f64) -> bool {
        (self.last_lat - lat).abs() > 0.001 || (self.last_lon - lon).abs() > 0.001
    }
}

fn stop_geoclue_client(client_path: &str) {
    let _ = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            client_path,
            "--method",
            "org.freedesktop.GeoClue2.Client.Stop",
        ])
        .output();
}

fn read_geoclue_double(location_path: &str, property: &str) -> Option<f64> {
    let output = Command::new("gdbus")
        .args([
            "call",
            "--system",
            "--dest",
            "org.freedesktop.GeoClue2",
            "--object-path",
            location_path,
            "--method",
            "org.freedesktop.DBus.Properties.Get",
            "org.freedesktop.GeoClue2.Location",
            property,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Output looks like "(<double 37.7749>,)" or "(<1.7976931348623157e+308>,)"
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();

    // Extract the numeric value from various gdbus output formats
    parse_gdbus_double(text)
}

/// Parse a double value from gdbus output.
/// Handles formats like "(<double 37.7749>,)", "(<37.7749>,)", "(<1.79e+308>,)"
fn parse_gdbus_double(text: &str) -> Option<f64> {
    // Strip wrapping parens and angle brackets
    let inner = text
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim_end_matches(',')
        .trim_start_matches('<')
        .trim_end_matches('>');

    // Remove "double " prefix if present
    let num_str = inner.trim_start_matches("double ").trim();

    let val: f64 = num_str.parse().ok()?;

    // GeoClue2 uses DBL_MAX (1.7976931348623157e+308) for unavailable values
    if val > 1.0e+300 || val < -1.0e+300 {
        return None;
    }

    Some(val)
}

impl DataProvider for LocationProvider {
    fn prefix(&self) -> &str {
        "li"
    }

    fn poll(&mut self) -> ProviderData {
        let mut data = ProviderData::new();

        // Try GeoClue2 first
        if let Some((lat, lon, alt, speed, accuracy)) = self.try_geoclue2() {
            data.insert("lat".into(), format!("{:.6}", lat));
            data.insert("lon".into(), format!("{:.6}", lon));
            data.insert("alt".into(), format!("{:.1}", alt));
            data.insert("speed".into(), format!("{:.2}", speed));
            data.insert("accuracy".into(), format!("{:.1}", accuracy));

            // Reverse geocode if location changed
            if self.location_changed_significantly(lat, lon) || self.cached_geocode.is_none() {
                if let Some(geo) = self.reverse_geocode(lat, lon) {
                    self.cached_geocode = Some(geo);
                }
                self.last_lat = lat;
                self.last_lon = lon;
            }

            if let Some(geo) = &self.cached_geocode {
                data.insert("addr".into(), geo.addr.clone());
                data.insert("loc".into(), geo.loc.clone());
                data.insert("country".into(), geo.country.clone());
                data.insert("countrycode".into(), geo.countrycode.clone());
            }

            return data;
        }

        // Fallback: IP-based geolocation
        if let Some((lat, lon, addr, city, country)) = self.try_ip_geolocation() {
            data.insert("lat".into(), format!("{:.6}", lat));
            data.insert("lon".into(), format!("{:.6}", lon));
            data.insert("alt".into(), "0".into());
            data.insert("speed".into(), "0".into());
            data.insert("accuracy".into(), "0".into());

            // IP geolocation gives us city/country directly, but try Nominatim
            // for a full address if location changed significantly
            if self.location_changed_significantly(lat, lon) || self.cached_geocode.is_none() {
                if let Some(geo) = self.reverse_geocode(lat, lon) {
                    self.cached_geocode = Some(geo);
                } else {
                    // Use ipinfo data as fallback geocode
                    self.cached_geocode = Some(GeocodeResult {
                        addr,
                        loc: city,
                        country: country.clone(),
                        countrycode: country,
                    });
                }
                self.last_lat = lat;
                self.last_lon = lon;
            }

            if let Some(geo) = &self.cached_geocode {
                data.insert("addr".into(), geo.addr.clone());
                data.insert("loc".into(), geo.loc.clone());
                data.insert("country".into(), geo.country.clone());
                data.insert("countrycode".into(), geo.countrycode.clone());
            }

            return data;
        }

        // Both methods failed - return empty strings
        data.insert("lat".into(), String::new());
        data.insert("lon".into(), String::new());
        data.insert("alt".into(), String::new());
        data.insert("speed".into(), String::new());
        data.insert("accuracy".into(), String::new());
        data.insert("addr".into(), String::new());
        data.insert("loc".into(), String::new());
        data.insert("country".into(), String::new());
        data.insert("countrycode".into(), String::new());

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }
}
