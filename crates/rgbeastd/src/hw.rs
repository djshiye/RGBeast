//! The hardware worker: one thread owns every driver and serialises access.
//! SMBus writes need millisecond pauses, so they never run on the async loop.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    thread,
};

use rgbeast_core::{
    DeviceInfo, DeviceState, Driver, Error, KeyboardOptions,
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

/// A copy of an error for every caller whose request was merged into one.
fn same_error(e: &Error) -> Error {
    match e {
        Error::Io(io) => Error::Protocol(io.to_string()),
        Error::Hid(m) => Error::Hid(m.clone()),
        Error::Smbus(m) => Error::Smbus(m.clone()),
        Error::Protocol(m) => Error::Protocol(m.clone()),
        Error::Unsupported(m) => Error::Unsupported(m.clone()),
        Error::Invalid(m) => Error::Invalid(m.clone()),
        Error::NoDevice(m) => Error::NoDevice(m.clone()),
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
    SetUnlocked(String, bool, Reply<DeviceInfo>),
    GetKeyboard(String, Reply<KeyboardOptions>),
    SetKeyboard(String, KeyboardOptions, Reply<KeyboardOptions>),
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
    /// Untested devices the owner has not turned control on for: nothing
    /// is ever sent to them.
    locked: BTreeSet<String>,
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
                    locked: BTreeSet::new(),
                    log: Vec::new(),
                    events: ev2,
                };
                inner.rescan();
                let mut pending: VecDeque<Cmd> = VecDeque::new();
                loop {
                    let cmd = match pending.pop_front() {
                        Some(c) => c,
                        None => match rx.blocking_recv() {
                            Some(c) => c,
                            None => break,
                        },
                    };
                    let Cmd::Set(id, mut state, reply) = cmd else {
                        inner.handle(cmd);
                        continue;
                    };
                    // A slider drag sends a state every few milliseconds and
                    // some devices take most of a second per state: apply
                    // only the newest of the waiting states for a device.
                    while let Ok(c) = rx.try_recv() {
                        pending.push_back(c);
                    }
                    let mut replies = vec![reply];
                    while matches!(pending.front(), Some(Cmd::Set(next, _, _)) if *next == id) {
                        if let Some(Cmd::Set(_, s, r)) = pending.pop_front() {
                            state = s;
                            replies.push(r);
                        }
                    }
                    inner.set(id, state, replies);
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
    pub async fn set_unlocked(&self, id: String, on: bool) -> Result<DeviceInfo, Error> {
        self.call(|r| Cmd::SetUnlocked(id, on, r)).await
    }
    pub async fn keyboard(&self, id: String) -> Result<KeyboardOptions, Error> {
        self.call(|r| Cmd::GetKeyboard(id, r)).await
    }
    pub async fn set_keyboard(
        &self,
        id: String,
        options: KeyboardOptions,
    ) -> Result<KeyboardOptions, Error> {
        self.call(|r| Cmd::SetKeyboard(id, options, r)).await
    }
}

impl Inner {
    fn find(&mut self, id: &str) -> Result<&mut Box<dyn Driver>, Error> {
        self.devices
            .iter_mut()
            .find(|d| d.info().id == id)
            .ok_or_else(|| Error::NoDevice(id.to_string()))
    }

    fn info(&self, d: &dyn Driver) -> DeviceInfo {
        let mut info = d.info().clone();
        info.locked = self.locked.contains(&info.id);
        info
    }

    fn infos(&self) -> Vec<DeviceInfo> {
        self.devices.iter().map(|d| self.info(d.as_ref())).collect()
    }

    fn check_unlocked(&self, id: &str) -> Result<(), Error> {
        if self.locked.contains(id) {
            let name = self
                .devices
                .iter()
                .find(|d| d.info().id == id)
                .map(|d| d.info().name.clone())
                .unwrap_or_else(|| id.to_string());
            return Err(Error::Unsupported(format!(
                "Turn on lighting control for {name} first."
            )));
        }
        Ok(())
    }

    fn set(&mut self, id: String, state: DeviceState, replies: Vec<Reply<DeviceState>>) {
        let res = self.check_unlocked(&id).and_then(|_| {
            let dev = self.find(&id)?;
            let state = validate(dev.info(), &state)?;
            dev.apply(&state)?;
            Ok((state, dev.keyboard_options()))
        });
        let res = match res {
            Ok((s, options)) => {
                if let Some(o) = options {
                    self.store.set_keyboard_options(&id, &o);
                }
                self.states.insert(id.clone(), s.clone());
                self.store.set_device_state(&id, &s);
                self.events.send(Event::StateChanged(id, s.clone())).ok();
                Ok(s)
            }
            Err(e) => Err(e),
        };
        for r in replies {
            let copy = match &res {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(same_error(e)),
            };
            r.send(copy).ok();
        }
    }

    /// Discover devices and bring each to its stored state (or read what it
    /// is showing, or leave it alone and assume the neutral default).
    fn rescan(&mut self) {
        let config = self.store.discovery_config();
        let found = scan(self.simulate, &config);
        self.devices = found.devices;
        self.log = found.log;
        self.states.clear();
        self.locked = self
            .devices
            .iter()
            .map(|d| d.info())
            .filter(|i| i.untested && !self.store.unlocked(&i.id))
            .map(|i| i.id.clone())
            .collect();
        for line in &self.log {
            tracing::info!("{line}");
        }
        for d in self.devices.iter_mut() {
            let id = d.info().id.clone();
            if self.locked.contains(&id) || d.keeps_state() {
                // Locked: never written to. Keeps its lighting: showing
                // it already, so the stored state is only adopted.
                let state = self
                    .store
                    .device_state(&id)
                    .or_else(|| d.read_state().ok().flatten())
                    .and_then(|s| validate(d.info(), &s).ok())
                    .unwrap_or_else(|| {
                        validate(d.info(), &DeviceState::default()).unwrap_or_default()
                    });
                self.states.insert(id, state);
                continue;
            }
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
                        // Nothing stored and nothing readable: leave whatever
                        // the device is showing (its own power-on default) and
                        // start from the neutral state on the next change.
                        tracing::info!("{id}: no stored state, leaving the device as it is");
                        validate(d.info(), &DeviceState::default()).unwrap_or_default()
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
            Cmd::Set(id, state, r) => self.set(id, state, vec![r]),
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
            Cmd::SetUnlocked(id, on, r) => {
                let res = match self.devices.iter().find(|d| d.info().id == id) {
                    None => Err(Error::NoDevice(id.clone())),
                    Some(d) if !d.info().untested => {
                        Err(Error::Unsupported("this device needs no turning on".into()))
                    }
                    Some(_) => {
                        self.store.set_unlocked(&id, on);
                        if on {
                            self.locked.remove(&id);
                        } else {
                            self.locked.insert(id.clone());
                        }
                        tracing::info!("{id}: control turned {}", if on { "on" } else { "off" });
                        self.events.send(Event::DevicesChanged).ok();
                        let d = self.devices.iter().find(|d| d.info().id == id);
                        Ok(self.info(d.expect("found above").as_ref()))
                    }
                };
                r.send(res).ok();
            }
            Cmd::GetKeyboard(id, r) => {
                let res = self.find(&id).and_then(|d| {
                    d.keyboard_options()
                        .ok_or_else(|| Error::Unsupported("not a Glorious keyboard".into()))
                });
                r.send(res).ok();
            }
            Cmd::SetKeyboard(id, options, r) => {
                let lighting = self.states.get(&id).cloned().unwrap_or_default();
                let res = self.check_unlocked(&id).and_then(|_| {
                    let dev = self.find(&id)?;
                    dev.set_keyboard_options(&options, &lighting)?;
                    Ok(dev.keyboard_options().unwrap_or(options))
                });
                if let Ok(o) = &res {
                    self.store.set_keyboard_options(&id, o);
                }
                r.send(res).ok();
            }
        }
    }
}
