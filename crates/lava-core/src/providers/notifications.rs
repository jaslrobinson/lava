use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{DataProvider, ProviderData};

const MAX_NOTIFICATIONS: usize = 10;

#[derive(Clone, Debug)]
struct NotificationEntry {
    app_name: String,
    icon: String,
    title: String,
    body: String,
}

pub struct NotificationProvider {
    buffer: Arc<Mutex<VecDeque<NotificationEntry>>>,
    listener_started: bool,
}

impl NotificationProvider {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            listener_started: false,
        }
    }

    fn start_listener(&self) {
        let buffer = self.buffer.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_dbus_listener(buffer) {
                eprintln!("[notifications] D-Bus listener failed: {}", e);
            }
        });
    }
}

fn run_dbus_listener(buffer: Arc<Mutex<VecDeque<NotificationEntry>>>) -> Result<(), String> {
    use zbus::blocking::Connection;
    use zbus::blocking::MessageIterator;
    use zbus::MatchRule;

    let connection = Connection::session()
        .map_err(|e| format!("Failed to connect to session bus: {}", e))?;

    // Build a match rule for Notify method calls on the Notifications interface.
    let rule: MatchRule<'_> = MatchRule::builder()
        .msg_type(zbus::message::Type::MethodCall)
        .interface("org.freedesktop.Notifications")
        .map_err(|e| format!("Invalid interface: {}", e))?
        .member("Notify")
        .map_err(|e| format!("Invalid member: {}", e))?
        .build();

    // Try BecomeMonitor first (works on modern D-Bus daemons).
    // This turns our connection into a passive monitor that sees all matching messages.
    let proxy = zbus::blocking::fdo::MonitoringProxy::new(&connection)
        .map_err(|e| format!("Failed to create monitoring proxy: {}", e))?;

    match proxy.become_monitor(&[rule.clone()], 0) {
        Ok(()) => {
            eprintln!("[notifications] Monitoring D-Bus notifications via BecomeMonitor");
        }
        Err(e) => {
            eprintln!(
                "[notifications] BecomeMonitor failed ({}), falling back to AddMatch",
                e
            );
            // Fall back to AddMatch with eavesdrop (requires permissive bus policy)
            let dbus_proxy = zbus::blocking::fdo::DBusProxy::new(&connection)
                .map_err(|e| format!("Failed to create DBus proxy: {}", e))?;
            dbus_proxy
                .add_match_rule(rule.clone())
                .map_err(|e| format!("AddMatch failed: {}", e))?;
            eprintln!("[notifications] Monitoring D-Bus notifications via AddMatch");
        }
    }

    let iter = MessageIterator::from(&connection);
    for msg in iter {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[notifications] Message error: {}", e);
                continue;
            }
        };

        // Check if this is a Notify method call
        let header = msg.header();
        let is_notify = header
            .interface()
            .map(|i| i.as_str() == "org.freedesktop.Notifications")
            .unwrap_or(false)
            && header
                .member()
                .map(|m| m.as_str() == "Notify")
                .unwrap_or(false);

        if !is_notify {
            continue;
        }

        // Parse Notify arguments: (app_name: s, replaces_id: u, app_icon: s, summary: s, body: s, ...)
        let body = msg.body();
        let parsed: Result<(String, u32, String, String, String), _> = body.deserialize();

        match parsed {
            Ok((app_name, _replaces_id, app_icon, summary, body_text)) => {
                let entry = NotificationEntry {
                    app_name,
                    icon: app_icon,
                    title: summary,
                    body: body_text,
                };
                if let Ok(mut buf) = buffer.lock() {
                    buf.push_front(entry);
                    while buf.len() > MAX_NOTIFICATIONS {
                        buf.pop_back();
                    }
                }
            }
            Err(e) => {
                eprintln!("[notifications] Failed to parse Notify args: {}", e);
            }
        }
    }

    Err("D-Bus message stream ended".into())
}

impl DataProvider for NotificationProvider {
    fn prefix(&self) -> &str {
        "ni"
    }

    fn poll(&mut self) -> ProviderData {
        if !self.listener_started {
            self.start_listener();
            self.listener_started = true;
        }

        let mut data = ProviderData::new();
        let buf = match self.buffer.lock() {
            Ok(b) => b,
            Err(_) => {
                data.insert("count".into(), "0".into());
                data.insert("scount".into(), "0".into());
                return data;
            }
        };

        let count = buf.len();
        data.insert("count".into(), count.to_string());
        data.insert("scount".into(), count.to_string());

        // Most recent notification fields (unindexed)
        if let Some(latest) = buf.front() {
            data.insert("title".into(), latest.title.clone());
            data.insert("text".into(), latest.body.clone());
            data.insert("app".into(), latest.app_name.clone());
            data.insert("icon".into(), latest.icon.clone());
        } else {
            data.insert("title".into(), String::new());
            data.insert("text".into(), String::new());
            data.insert("app".into(), String::new());
            data.insert("icon".into(), String::new());
        }

        // Indexed access: 0 = most recent
        for (i, entry) in buf.iter().enumerate() {
            let prefix = format!("{}_", i);
            data.insert(format!("{}title", prefix), entry.title.clone());
            data.insert(format!("{}text", prefix), entry.body.clone());
            data.insert(format!("{}app", prefix), entry.app_name.clone());
            data.insert(format!("{}icon", prefix), entry.icon.clone());
        }

        data
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}
