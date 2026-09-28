//! Scenes: a named snapshot of every device's state, kept per user in
//! `~/.config/rgbeast/scenes.json`.

use std::{collections::BTreeMap, fs, path::PathBuf};

use gtk::glib;
use rgbeast_core::{DeviceState, Rgb};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub name: String,
    pub states: BTreeMap<String, DeviceState>,
}

impl Scene {
    /// Representative colours for the sidebar swatch.
    pub fn colors(&self) -> Vec<Rgb> {
        let mut out: Vec<Rgb> = self.states.values().map(|s| s.primary_color()).collect();
        if out.is_empty() {
            out.push(Rgb::BLACK);
        }
        out
    }
}

pub struct SceneStore {
    path: PathBuf,
    pub scenes: Vec<Scene>,
}

impl SceneStore {
    pub fn load() -> SceneStore {
        let path = glib::user_config_dir().join("rgbeast").join("scenes.json");
        let scenes = fs::read_to_string(&path)
            .ok()
            .and_then(|s| {
                serde_json::from_str(&s)
                    .map_err(|e| tracing::warn!("ignoring scenes.json: {e}"))
                    .ok()
            })
            .unwrap_or_default();
        SceneStore { path, scenes }
    }

    fn save(&self) {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).ok();
        }
        match serde_json::to_vec_pretty(&self.scenes) {
            Ok(bytes) => {
                if let Err(e) = fs::write(&self.path, bytes) {
                    tracing::warn!("cannot save scenes: {e}");
                }
            }
            Err(e) => tracing::warn!("cannot serialise scenes: {e}"),
        }
    }

    pub fn add(&mut self, scene: Scene) {
        self.scenes.retain(|s| s.name != scene.name);
        self.scenes.push(scene);
        self.save();
    }

    pub fn remove(&mut self, name: &str) {
        self.scenes.retain(|s| s.name != name);
        self.save();
    }

    pub fn rename(&mut self, old: &str, new: &str) {
        if let Some(s) = self.scenes.iter_mut().find(|s| s.name == old) {
            s.name = new.to_string();
        }
        self.save();
    }
}
