use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::types::AppSettings;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    #[allow(dead_code)]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub fn settings_path(app_dir: &std::path::Path) -> PathBuf {
    app_dir.join("settings.json")
}

pub fn load_settings(app_dir: &std::path::Path) -> AppSettings {
    let path = settings_path(app_dir);
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(app_dir: &std::path::Path, settings: &AppSettings) -> Result<(), AppError> {
    std::fs::create_dir_all(app_dir)?;
    let path = settings_path(app_dir);
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct KillResult {
    pub ok: bool,
    pub message: String,
}
