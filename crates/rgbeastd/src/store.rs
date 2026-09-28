//! Persistent daemon state: the last state applied to each device and the
//! LED counts configured for addressable headers.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rgbeast_core::{DeviceState, discover::DiscoveryConfig};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Persisted {
    pub devices: BTreeMap<String, DeviceState>,
    pub header_leds: BTreeMap<String, Vec<u32>>,
    /// Re-apply the stored state when the machine wakes from sleep.
    pub restore_on_resume: bool,
}

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
    data: Arc<Mutex<Persisted>>,
}

impl Store {
    pub fn open(dir: &Path) -> Store {
        let path = dir.join("state.json");
        let data = fs::read_to_string(&path)
            .ok()
            .and_then(|s| {
                serde_json::from_str::<Persisted>(&s)
                    .map_err(|e| tracing::warn!("ignoring {}: {e}", path.display()))
                    .ok()
            })
            .unwrap_or_else(|| Persisted {
                restore_on_resume: true,
                ..Default::default()
            });
        Store {
            path,
            data: Arc::new(Mutex::new(data)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Persisted> {
        self.data.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn flush(&self, data: &Persisted) {
        if let Some(dir) = self.path.parent()
            && let Err(e) = fs::create_dir_all(dir)
        {
            tracing::warn!("cannot create {}: {e}", dir.display());
            return;
        }
        let tmp = self.path.with_extension("json.tmp");
        match serde_json::to_vec_pretty(data) {
            Ok(bytes) => {
                if let Err(e) = fs::write(&tmp, bytes).and_then(|_| fs::rename(&tmp, &self.path)) {
                    tracing::warn!("cannot write {}: {e}", self.path.display());
                }
            }
            Err(e) => tracing::warn!("cannot serialise state: {e}"),
        }
    }

    pub fn device_state(&self, id: &str) -> Option<DeviceState> {
        self.lock().devices.get(id).cloned()
    }

    pub fn set_device_state(&self, id: &str, state: &DeviceState) {
        let mut d = self.lock();
        d.devices.insert(id.to_string(), state.clone());
        self.flush(&d);
    }

    pub fn set_header_leds(&self, id: &str, zone_index: usize, leds: u32) {
        let mut d = self.lock();
        let v = d.header_leds.entry(id.to_string()).or_default();
        if v.len() <= zone_index {
            v.resize(zone_index + 1, 0);
        }
        v[zone_index] = leds;
        self.flush(&d);
    }

    pub fn restore_on_resume(&self) -> bool {
        self.lock().restore_on_resume
    }

    pub fn set_restore_on_resume(&self, on: bool) {
        let mut d = self.lock();
        d.restore_on_resume = on;
        self.flush(&d);
    }

    pub fn discovery_config(&self) -> DiscoveryConfig {
        let d = self.lock();
        DiscoveryConfig {
            header_leds: d
                .header_leds
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgbeast_core::Rgb;

    #[test]
    fn roundtrip_and_header_leds() {
        let dir = std::env::temp_dir().join(format!("rgbeast-store-{}", std::process::id()));
        let s = Store::open(&dir);
        assert!(s.restore_on_resume());
        s.set_device_state("a", &DeviceState::static_color(Rgb::new(1, 2, 3)));
        s.set_header_leds("a", 2, 36);
        let s2 = Store::open(&dir);
        assert_eq!(
            s2.device_state("a").unwrap().primary_color(),
            Rgb::new(1, 2, 3)
        );
        assert_eq!(
            s2.discovery_config().header_leds,
            vec![("a".to_string(), vec![0, 0, 36])]
        );
        fs::remove_dir_all(&dir).ok();
    }
}
