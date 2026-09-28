//! The hardware worker: one thread owns every driver and serialises access.
//! SMBus writes need millisecond pauses, so they never run on the async loop.

use std::{collections::BTreeMap, thread};

use rgbeast_core::{
    DeviceInfo, DeviceState, Driver, Error,
    discover::{self, DiscoveryConfig},
    model::validate,
    sim,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::store::Store;

pub struct Scan {
    pub devices: Vec<Box<dyn Driver>>,
    pub log: Vec<String>,
}

pub fn scan(simulate: bool, config: &DiscoveryConfig) -> Scan {
    if simulate {
        let devices = sim::devices();
        let log = devices
            .iter()
            .map(|d| format!("simulated: {}", d.info().name))
            .collect();
        Scan { devices, log }
    } else {
        let found = discover::discover(config);
        Scan {
            devices: found.devices,
            log: found.log,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    DevicesChanged,
    StateChanged(String, DeviceState),
}

type Reply<T> = oneshot::Sender<Result<T, Error>>;

enum Cmd {
    List(Reply<Vec<DeviceInfo>>),
    Log(Reply<Vec<String>>),
    Get(String, Reply<DeviceState>),
    Set(String, DeviceState, Reply<DeviceState>),
    SetZoneLeds(String, String, u32, Reply<DeviceInfo>),
    Save(String, Reply<()>),
    Rescan(Reply<Vec<DeviceInfo>>),
    RestoreAll(Reply<()>),
}

#[derive(Clone)]
pub struct Worker {
    tx: mpsc::Sender<Cmd>,
    events: broadcast::Sender<Event>,
}

struct Inner {
    simulate: bool,
    store: Store,
    devices: Vec<Box<dyn Driver>>,
    states: BTreeMap<String, DeviceState>,
    log: Vec<String>,
    events: broadcast::Sender<Event>,
}

impl Worker {
    pub fn spawn(simulate: bool, store: Store) -> Worker {
        let (tx, mut rx) = mpsc::channel::<Cmd>(64);
        let (events, _) = broadcast::channel(64);
        let ev2 = events.clone();
        thread::Builder::new()
            .name("rgbeast-hw".into())
            .spawn(move || {
                let mut inner = Inner {
                    simulate,
                    store,
                    devices: Vec::new(),
                    states: BTreeMap::new(),
                    log: Vec::new(),
                    events: ev2,
                };
                inner.rescan();
                while let Some(cmd) = rx.blocking_recv() {
                    inner.handle(cmd);
                }
            })
            .expect("spawn hardware thread");
        Worker { tx, events }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    async fn call<T>(&self, make: impl FnOnce(Reply<T>) -> Cmd) -> Result<T, Error> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(make(tx))
            .await
            .map_err(|_| Error::Protocol("hardware thread stopped".into()))?;
        rx.await
            .map_err(|_| Error::Protocol("hardware thread dropped the request".into()))?
    }

    pub async fn list(&self) -> Result<Vec<DeviceInfo>, Error> {
        self.call(Cmd::List).await
    }
    pub async fn log(&self) -> Result<Vec<String>, Error> {
        self.call(Cmd::Log).await
    }
    pub async fn get(&self, id: String) -> Result<DeviceState, Error> {
        self.call(|r| Cmd::Get(id, r)).await
    }
    pub async fn set(&self, id: String, state: DeviceState) -> Result<DeviceState, Error> {
        self.call(|r| Cmd::Set(id, state, r)).await
    }
    pub async fn set_zone_leds(
        &self,
        id: String,
        zone: String,
        leds: u32,
    ) -> Result<DeviceInfo, Error> {
        self.call(|r| Cmd::SetZoneLeds(id, zone, leds, r)).await
    }
    pub async fn save(&self, id: String) -> Result<(), Error> {
        self.call(|r| Cmd::Save(id, r)).await
    }
    pub async fn rescan(&self) -> Result<Vec<DeviceInfo>, Error> {
        self.call(Cmd::Rescan).await
    }
    pub async fn restore_all(&self) -> Result<(), Error> {
        self.call(Cmd::RestoreAll).await
    }
}

impl Inner {
    fn find(&mut self, id: &str) -> Result<&mut Box<dyn Driver>, Error> {
        self.devices
            .iter_mut()
            .find(|d| d.info().id == id)
            .ok_or_else(|| Error::NoDevice(id.to_string()))
    }

    fn infos(&self) -> Vec<DeviceInfo> {
        self.devices.iter().map(|d| d.info().clone()).collect()
    }

    /// Discover devices and bring each to its stored state (or read what it
    /// is showing, or fall back to a sensible default).
    fn rescan(&mut self) {
        let config = self.store.discovery_config();
        let found = scan(self.simulate, &config);
        self.devices = found.devices;
        self.log = found.log;
        self.states.clear();
        for line in &self.log {
            tracing::info!("{line}");
        }
        for d in self.devices.iter_mut() {
            let id = d.info().id.clone();
            let state = match self.store.device_state(&id) {
                Some(saved) => match validate(d.info(), &saved) {
                    Ok(s) => {
                        if let Err(e) = d.apply(&s) {
                            tracing::warn!("{id}: restore failed: {e}");
                        }
                        s
                    }
                    Err(e) => {
                        tracing::warn!("{id}: stored state invalid ({e}), reading device");
                        d.read_state().ok().flatten().unwrap_or_default()
                    }
                },
                None => match d.read_state() {
                    Ok(Some(s)) => validate(d.info(), &s).unwrap_or_default(),
                    _ => {
                        let s = validate(d.info(), &DeviceState::default()).unwrap_or_default();
                        d.apply(&s).ok();
                        s
                    }
                },
            };
            self.states.insert(id, state);
        }
        self.events.send(Event::DevicesChanged).ok();
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::List(r) => {
                r.send(Ok(self.infos())).ok();
            }
            Cmd::Log(r) => {
                r.send(Ok(self.log.clone())).ok();
            }
            Cmd::Get(id, r) => {
                let res = if self.devices.iter().any(|d| d.info().id == id) {
                    Ok(self.states.get(&id).cloned().unwrap_or_default())
                } else {
                    Err(Error::NoDevice(id))
                };
                r.send(res).ok();
            }
            Cmd::Set(id, state, r) => {
                let res = (|| {
                    let dev = self.find(&id)?;
                    let state = validate(dev.info(), &state)?;
                    dev.apply(&state)?;
                    Ok(state)
                })();
                if let Ok(s) = &res {
                    self.states.insert(id.clone(), s.clone());
                    self.store.set_device_state(&id, s);
                    self.events.send(Event::StateChanged(id, s.clone())).ok();
                }
                r.send(res).ok();
            }
            Cmd::SetZoneLeds(id, zone, leds, r) => {
                let res = (|| {
                    let dev = self.find(&id)?;
                    dev.set_zone_leds(&zone, leds)?;
                    Ok(dev.info().clone())
                })();
                if let Ok(info) = &res {
                    if let Some(idx) = info
                        .zones
                        .iter()
                        .filter(|z| z.is_sizable())
                        .position(|z| z.id == zone)
                    {
                        self.store.set_header_leds(&id, idx, leds);
                    }
                    // Re-apply so the newly sized zone lights up.
                    if let Some(state) = self.states.get(&id).cloned()
                        && let Ok(dev) = self.find(&id)
                        && let Ok(s) = validate(dev.info(), &state)
                    {
                        dev.apply(&s).ok();
                        self.states.insert(id.clone(), s);
                    }
                    self.events.send(Event::DevicesChanged).ok();
                }
                r.send(res).ok();
            }
            Cmd::Save(id, r) => {
                let res = self.find(&id).and_then(|d| d.save());
                r.send(res).ok();
            }
            Cmd::Rescan(r) => {
                self.rescan();
                r.send(Ok(self.infos())).ok();
            }
            Cmd::RestoreAll(r) => {
                // After sleep, USB controllers may have re-enumerated.
                self.rescan();
                r.send(Ok(())).ok();
            }
        }
    }
}
