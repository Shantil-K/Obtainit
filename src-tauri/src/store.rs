//! JSON persistence for apps and settings.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::model::{App, Settings};

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("obtainit")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("obtainit")
}

pub fn downloads_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("obtainit").join("downloads")
}

pub fn icons_dir() -> PathBuf {
    data_dir().join("icons")
}

pub struct Store {
    pub apps: Vec<App>,
    pub settings: Settings,
    dir: PathBuf,
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn load_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    match std::fs::read_to_string(path) {
        Ok(t) => serde_json::from_str(&t).unwrap_or_else(|e| {
            log::error!("{} is unreadable ({e}); keeping a backup and starting fresh", path.display());
            let _ = std::fs::copy(path, path.with_extension("json.bad"));
            T::default()
        }),
        Err(_) => T::default(),
    }
}

impl Store {
    pub fn load() -> Self {
        let dir = config_dir();
        let _ = std::fs::create_dir_all(&dir);
        Store {
            apps: load_json(&dir.join("apps.json")),
            settings: load_json(&dir.join("settings.json")),
            dir,
        }
    }

    pub fn save_apps(&self) {
        let res = serde_json::to_vec_pretty(&self.apps)
            .map_err(anyhow::Error::from)
            .and_then(|d| write_atomic(&self.dir.join("apps.json"), &d));
        if let Err(e) = res {
            log::error!("saving apps: {e:#}");
        }
    }

    pub fn save_settings(&self) {
        let res = serde_json::to_vec_pretty(&self.settings)
            .map_err(anyhow::Error::from)
            .and_then(|d| write_atomic(&self.dir.join("settings.json"), &d));
        if let Err(e) = res {
            log::error!("saving settings: {e:#}");
        }
    }

    pub fn get(&self, id: &str) -> Option<&App> {
        self.apps.iter().find(|a| a.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut App> {
        self.apps.iter_mut().find(|a| a.id == id)
    }
}
