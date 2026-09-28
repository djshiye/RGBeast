//! D-Bus client for `io.github.djshiye.RGBeast1`. The app never touches
//! hardware; everything goes through the daemon, which enforces polkit.

use rgbeast_core::{DeviceInfo, DeviceState};
use zbus::zvariant::OwnedValue;

#[zbus::proxy(
    interface = "io.github.djshiye.RGBeast1.Manager",
    default_service = "io.github.djshiye.RGBeast1",
    default_path = "/io/github/djshiye/RGBeast1"
)]
pub trait Manager {
    fn list_devices(&self) -> zbus::Result<String>;
    fn get_state(&self, id: &str) -> zbus::Result<String>;
    fn set_state(&self, id: &str, state: &str) -> zbus::Result<String>;
    fn set_zone_leds(&self, id: &str, zone: &str, leds: u32) -> zbus::Result<String>;
    fn save_to_device(&self, id: &str) -> zbus::Result<()>;
    fn rescan(&self) -> zbus::Result<String>;
    fn discovery_log(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property)]
    fn version(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn simulated(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn restore_on_resume(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_restore_on_resume(&self, on: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn devices_changed(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn state_changed(&self, id: &str, state: &str) -> zbus::Result<()>;
}

#[derive(Clone, Debug)]
pub enum ClientError {
    /// The daemon is not on the bus.
    Unavailable(String),
    /// polkit refused the caller.
    Denied(String),
    /// Anything else the daemon reported.
    Failed(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Unavailable(m) | ClientError::Denied(m) | ClientError::Failed(m) => {
                f.write_str(m)
            }
        }
    }
}

fn map_err(e: zbus::Error) -> ClientError {
    match &e {
        zbus::Error::FDO(fdo) => match &**fdo {
            zbus::fdo::Error::ServiceUnknown(m) | zbus::fdo::Error::NameHasNoOwner(m) => {
                ClientError::Unavailable(m.clone())
            }
            zbus::fdo::Error::AccessDenied(m) | zbus::fdo::Error::AuthFailed(m) => {
                ClientError::Denied(m.clone())
            }
            other => ClientError::Failed(other.to_string()),
        },
        zbus::Error::MethodError(name, msg, _) => {
            let n = name.as_str();
            let m = msg.clone().unwrap_or_else(|| n.to_string());
            if n.ends_with("ServiceUnknown") || n.ends_with("NameHasNoOwner") {
                ClientError::Unavailable(m)
            } else if n.ends_with("AccessDenied") || n.ends_with("AuthFailed") {
                ClientError::Denied(m)
            } else {
                ClientError::Failed(m)
            }
        }
        _ => ClientError::Failed(e.to_string()),
    }
}

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Clone)]
pub struct Client {
    proxy: ManagerProxy<'static>,
}

impl Client {
    /// Connect to the system bus (or the session bus when `RGBEAST_BUS=session`,
    /// for development against `rgbeastd --session --simulate`).
    pub async fn connect() -> Result<Client> {
        let session = std::env::var("RGBEAST_BUS")
            .map(|v| v == "session")
            .unwrap_or(false);
        let conn = if session {
            zbus::Connection::session().await
        } else {
            zbus::Connection::system().await
        }
        .map_err(|e| ClientError::Unavailable(e.to_string()))?;
        let proxy = ManagerProxy::builder(&conn)
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await
            .map_err(map_err)?;
        // Touch the service so a missing daemon fails here, not later.
        proxy.version().await.map_err(map_err)?;
        Ok(Client { proxy })
    }

    pub async fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        let json = self.proxy.list_devices().await.map_err(map_err)?;
        serde_json::from_str(&json)
            .map_err(|e| ClientError::Failed(format!("bad device list: {e}")))
    }

    pub async fn get_state(&self, id: &str) -> Result<DeviceState> {
        let json = self.proxy.get_state(id).await.map_err(map_err)?;
        serde_json::from_str(&json).map_err(|e| ClientError::Failed(format!("bad state: {e}")))
    }

    pub async fn set_state(&self, id: &str, state: &DeviceState) -> Result<DeviceState> {
        let json = serde_json::to_string(state).map_err(|e| ClientError::Failed(e.to_string()))?;
        let back = self.proxy.set_state(id, &json).await.map_err(map_err)?;
        serde_json::from_str(&back).map_err(|e| ClientError::Failed(format!("bad state: {e}")))
    }

    pub async fn set_zone_leds(&self, id: &str, zone: &str, leds: u32) -> Result<DeviceInfo> {
        let back = self
            .proxy
            .set_zone_leds(id, zone, leds)
            .await
            .map_err(map_err)?;
        serde_json::from_str(&back).map_err(|e| ClientError::Failed(format!("bad device: {e}")))
    }

    pub async fn save_to_device(&self, id: &str) -> Result<()> {
        self.proxy.save_to_device(id).await.map_err(map_err)
    }

    pub async fn rescan(&self) -> Result<Vec<DeviceInfo>> {
        let json = self.proxy.rescan().await.map_err(map_err)?;
        serde_json::from_str(&json)
            .map_err(|e| ClientError::Failed(format!("bad device list: {e}")))
    }

    pub async fn discovery_log(&self) -> Result<Vec<String>> {
        self.proxy.discovery_log().await.map_err(map_err)
    }

    pub async fn version(&self) -> Result<String> {
        self.proxy.version().await.map_err(map_err)
    }

    pub async fn simulated(&self) -> Result<bool> {
        self.proxy.simulated().await.map_err(map_err)
    }

    pub async fn restore_on_resume(&self) -> Result<bool> {
        self.proxy.restore_on_resume().await.map_err(map_err)
    }

    pub async fn set_restore_on_resume(&self, on: bool) -> Result<()> {
        self.proxy.set_restore_on_resume(on).await.map_err(map_err)
    }

    /// Stream of `DevicesChanged` signals.
    pub async fn devices_changed(&self) -> Result<DevicesChangedStream> {
        self.proxy.receive_devices_changed().await.map_err(map_err)
    }

    /// Stream of `StateChanged(id, state)` signals.
    pub async fn state_changed(&self) -> Result<StateChangedStream> {
        self.proxy.receive_state_changed().await.map_err(map_err)
    }
}

#[allow(dead_code)]
fn _owned_value_is_used(_: OwnedValue) {}
