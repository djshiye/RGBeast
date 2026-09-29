//! The D-Bus interface `io.github.djshiye.RGBeast1.Manager`.
//!
//! Device descriptions and states cross the bus as JSON strings whose schema
//! is `rgbeast_core::DeviceInfo` / `rgbeast_core::DeviceState` (strict: unknown
//! fields are rejected). Every mutating method is checked with polkit.

use rgbeast_core::{DeviceState, Error};
use zbus::{fdo, message::Header, object_server::SignalEmitter};

use crate::{
    auth,
    hw::{Event, Worker},
    store::Store,
};

pub struct Manager {
    worker: Worker,
    store: Store,
    simulated: bool,
    system_bus: bool,
}

impl Manager {
    pub fn new(worker: Worker, store: Store, simulated: bool, system_bus: bool) -> Self {
        Manager {
            worker,
            store,
            simulated,
            system_bus,
        }
    }

    async fn authorise(&self, conn: &zbus::Connection, hdr: &Header<'_>) -> fdo::Result<()> {
        auth::check(conn, hdr, auth::ACTION_CONTROL, self.system_bus).await
    }
}

fn to_fdo(e: Error) -> fdo::Error {
    match e {
        Error::NoDevice(id) => fdo::Error::UnknownObject(format!("no device {id}")),
        Error::Invalid(m) => fdo::Error::InvalidArgs(m),
        Error::Unsupported(m) => fdo::Error::NotSupported(m),
        other => fdo::Error::Failed(other.to_string()),
    }
}

fn parse_state(json: &str) -> fdo::Result<DeviceState> {
    if json.len() > 64 * 1024 {
        return Err(fdo::Error::InvalidArgs("state too large".into()));
    }
    serde_json::from_str(json).map_err(|e| fdo::Error::InvalidArgs(format!("bad state: {e}")))
}

#[zbus::interface(name = "io.github.djshiye.RGBeast1.Manager")]
impl Manager {
    /// JSON array of DeviceInfo.
    async fn list_devices(&self) -> fdo::Result<String> {
        let list = self.worker.list().await.map_err(to_fdo)?;
        serde_json::to_string(&list).map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// JSON DeviceState of one device.
    async fn get_state(&self, id: String) -> fdo::Result<String> {
        let s = self.worker.get(id).await.map_err(to_fdo)?;
        serde_json::to_string(&s).map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Apply a JSON DeviceState. Returns the normalised state as applied.
    async fn set_state(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: Header<'_>,
        id: String,
        state: String,
    ) -> fdo::Result<String> {
        self.authorise(conn, &hdr).await?;
        let state = parse_state(&state)?;
        let applied = self.worker.set(id, state).await.map_err(to_fdo)?;
        serde_json::to_string(&applied).map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Resize an addressable zone. Returns the updated JSON DeviceInfo.
    async fn set_zone_leds(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: Header<'_>,
        id: String,
        zone: String,
        leds: u32,
    ) -> fdo::Result<String> {
        self.authorise(conn, &hdr).await?;
        let info = self
            .worker
            .set_zone_leds(id, zone, leds)
            .await
            .map_err(to_fdo)?;
        serde_json::to_string(&info).map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// Store the current effect as the device's power-on default.
    async fn save_to_device(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: Header<'_>,
        id: String,
    ) -> fdo::Result<()> {
        self.authorise(conn, &hdr).await?;
        self.worker.save(id).await.map_err(to_fdo)
    }

    /// Re-run discovery. Returns the JSON device list.
    async fn rescan(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: Header<'_>,
    ) -> fdo::Result<String> {
        self.authorise(conn, &hdr).await?;
        let list = self.worker.rescan().await.map_err(to_fdo)?;
        serde_json::to_string(&list).map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    /// What discovery found and skipped, one line each.
    async fn discovery_log(&self) -> fdo::Result<Vec<String>> {
        self.worker.log().await.map_err(to_fdo)
    }

    #[zbus(property)]
    async fn version(&self) -> String {
        crate::VERSION.to_string()
    }

    #[zbus(property)]
    async fn simulated(&self) -> bool {
        self.simulated
    }

    #[zbus(property)]
    async fn restore_on_resume(&self) -> bool {
        self.store.restore_on_resume()
    }

    #[zbus(property)]
    async fn set_restore_on_resume(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] hdr: Option<Header<'_>>,
        on: bool,
    ) -> zbus::Result<()> {
        let hdr = hdr.ok_or_else(|| zbus::Error::Failure("no message header".into()))?;
        self.authorise(conn, &hdr).await?;
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.set_restore_on_resume(on))
            .await
            .map_err(|e| zbus::Error::Failure(e.to_string()))?;
        Ok(())
    }

    #[zbus(signal)]
    async fn devices_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, id: &str, state: &str) -> zbus::Result<()>;
}

/// Turn worker events (from restores and rescans) into signals.
pub async fn emit(conn: &zbus::Connection, ev: Event) {
    let Ok(emitter) = SignalEmitter::new(conn, crate::OBJECT_PATH) else {
        return;
    };
    match ev {
        Event::DevicesChanged => {
            Manager::devices_changed(&emitter).await.ok();
        }
        Event::StateChanged(id, state) => {
            if let Ok(json) = serde_json::to_string(&state) {
                Manager::state_changed(&emitter, &id, &json).await.ok();
            }
        }
    }
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1 {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

/// Re-apply lighting when the machine wakes up, if the user wants that.
/// Controllers lose their state or re-enumerate across suspend; a short
/// delay lets USB settle.
pub async fn watch_sleep(conn: zbus::Connection, worker: Worker, store: Store) -> zbus::Result<()> {
    use futures_lite::StreamExt;
    let proxy = Login1Proxy::new(&conn).await?;
    let mut stream = proxy.receive_prepare_for_sleep().await?;
    while let Some(sig) = stream.next().await {
        let Ok(args) = sig.args() else { continue };
        if !args.start {
            if !store.restore_on_resume() {
                tracing::info!("resumed from sleep, restore is off");
                continue;
            }
            tracing::info!("resumed from sleep, restoring lighting");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if let Err(e) = worker.restore_all().await {
                tracing::warn!("restore after resume failed: {e}");
            }
        }
    }
    Ok(())
}
