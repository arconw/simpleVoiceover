mod pulse;

use crate::storage::Store;
use anyhow::{Context, Result};
use libpulse_binding::proplist::Proplist;
use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock, mpsc},
    time::Duration,
};

const INSTANCE_PROPERTY: &str = "simplevoiceover.instance";
static INSTANCE: OnceLock<String> = OnceLock::new();

fn instance() -> &'static str {
    INSTANCE.get_or_init(|| format!("{}-{}", std::process::id(), uuid::Uuid::new_v4()))
}

pub fn initialize_environment() {
    let mut properties = std::env::var("PULSE_PROP")
        .ok()
        .and_then(|value| Proplist::new_from_string(&value))
        .or_else(Proplist::new);
    if let Some(properties) = properties.as_mut() {
        let _ = properties.set_str(INSTANCE_PROPERTY, instance());
        if let Some(value) = properties.to_string() {
            unsafe { std::env::set_var("PULSE_PROP", value) };
        }
    }
}

#[derive(Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub label: String,
    pub is_default: bool,
    #[serde(skip)]
    pub index: u32,
}

#[derive(Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub inputs: Vec<Device>,
    pub outputs: Vec<Device>,
    pub native_routing: bool,
    pub available: bool,
}

#[derive(Clone, Default)]
struct Preferences {
    input: String,
    output: String,
}

pub struct Routing {
    devices: Arc<Mutex<Catalog>>,
    synchronize: mpsc::Sender<mpsc::Sender<Result<()>>>,
}

impl Routing {
    pub fn start(store: Arc<Mutex<Store>>) -> Self {
        let devices = Arc::new(Mutex::new(Catalog {
            native_routing: true,
            ..Catalog::default()
        }));
        let (synchronize, requests) = mpsc::channel::<mpsc::Sender<Result<()>>>();
        let catalog = devices.clone();
        std::thread::spawn(move || {
            let mut connection = None;
            let mut preferences = Preferences::default();
            loop {
                if let Ok(store) = store.try_lock() {
                    preferences.input = store.config.input_device.clone();
                    preferences.output = store.config.output_device.clone();
                }
                let result = (|| {
                    if connection.is_none() {
                        connection = Some(pulse::Connection::connect()?);
                    }
                    connection.as_mut().unwrap().synchronize(&preferences)
                })();
                let error = match result {
                    Ok(value) => {
                        if let Ok(mut catalog) = catalog.lock() {
                            *catalog = value;
                        }
                        None
                    }
                    Err(error) => {
                        connection = None;
                        if let Ok(mut catalog) = catalog.lock() {
                            catalog.available = false;
                        }
                        Some(error.to_string())
                    }
                };
                match requests.recv_timeout(Duration::from_secs(1)) {
                    Ok(reply) => {
                        if let Ok(store) = store.try_lock() {
                            preferences.input = store.config.input_device.clone();
                            preferences.output = store.config.output_device.clone();
                        }
                        let result = if let Some(connection) = connection.as_mut() {
                            connection.synchronize(&preferences).map(|value| {
                                if let Ok(mut catalog) = catalog.lock() {
                                    *catalog = value;
                                }
                            })
                        } else if preferences.input.is_empty() && preferences.output.is_empty() {
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(error.unwrap_or_else(unavailable)))
                        };
                        if result.is_err() {
                            connection = None;
                            if let Ok(mut catalog) = catalog.lock() {
                                catalog.available = false;
                            }
                        }
                        let _ = reply.send(result);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        Self {
            devices,
            synchronize,
        }
    }

    pub fn catalog(&self) -> Result<Catalog> {
        self.devices
            .lock()
            .map(|value| value.clone())
            .map_err(|_| anyhow::anyhow!(unavailable()))
    }

    pub fn synchronize(&self) -> Result<()> {
        let (reply, response) = mpsc::channel();
        self.synchronize
            .send(reply)
            .map_err(|_| anyhow::anyhow!(unavailable()))?;
        response
            .recv_timeout(Duration::from_secs(4))
            .context(unavailable())?
    }
}

fn unavailable() -> String {
    crate::i18n::message("error.audioDevicesUnavailable")
}

fn target<'a>(devices: &'a [Device], selected: &str) -> Option<&'a Device> {
    devices
        .iter()
        .find(|device| device.id == selected)
        .or_else(|| devices.iter().find(|device| device.is_default))
}

fn owns_stream(
    instance_id: Option<&str>,
    client: Option<u32>,
    owned_clients: &HashSet<u32>,
) -> bool {
    instance_id == Some(instance()) || client.is_some_and(|client| owned_clients.contains(&client))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_default_changes_and_disconnected_devices_have_a_live_fallback() {
        let mut devices = vec![
            Device {
                id: "headphones".into(),
                is_default: true,
                index: 1,
                ..Device::default()
            },
            Device {
                id: "speaker".into(),
                index: 2,
                ..Device::default()
            },
        ];
        assert_eq!(target(&devices, "").unwrap().id, "headphones");
        assert_eq!(target(&devices, "speaker").unwrap().id, "speaker");
        devices[0].is_default = false;
        devices[1].is_default = true;
        assert_eq!(target(&devices, "").unwrap().id, "speaker");
        assert_eq!(target(&devices, "headphones").unwrap().id, "headphones");
        devices.remove(0);
        assert_eq!(target(&devices, "headphones").unwrap().id, "speaker");
        assert!(target(&[], "").is_none());
    }

    #[test]
    fn routing_owns_only_streams_from_this_application_instance() {
        let clients = HashSet::from([12]);
        assert!(owns_stream(Some(instance()), None, &clients));
        assert!(owns_stream(None, Some(12), &clients));
        assert!(!owns_stream(Some("another-instance"), Some(13), &clients));
        assert!(!owns_stream(None, None, &clients));
        assert!(!owns_stream(None, Some(13), &clients));
    }
}
