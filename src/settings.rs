use crate::queue::Repeat;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub folders: Vec<PathBuf>,
    pub selected: Option<PathBuf>,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub output_device: Option<String>,
    pub equalizer: crate::equalizer::EqualizerSettings,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            folders: vec![],
            selected: None,
            volume: 0.65,
            shuffle: false,
            repeat: Repeat::Off,
            output_device: None,
            equalizer: Default::default(),
        }
    }
}
impl Settings {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let mut settings: Self = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Не удалось прочитать настройки: {e}"))?;
        settings.volume = if settings.volume.is_finite() {
            settings.volume.clamp(0.0, 1.0)
        } else {
            0.65
        };
        let mut seen = std::collections::HashSet::new();
        settings.folders.retain(|path| seen.insert(path.clone()));
        settings.equalizer.normalize();
        Ok(settings)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        fs::rename(temporary, path).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_settings_roundtrip_and_atomic_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut settings = Settings {
            folders: vec![PathBuf::from("Музыка/Ночной джаз")],
            ..Default::default()
        };
        settings.save(&path).unwrap();
        settings.volume = 0.24;
        settings.output_device = Some("Монитор / HDMI".into());
        settings.equalizer.enabled = true;
        settings.equalizer.gains = [4.0, 2.0, 0.0, -2.0, 1.0];
        settings.save(&path).unwrap();
        let saved = Settings::load(&path).unwrap();
        assert_eq!(saved.folders, settings.folders);
        assert_eq!(saved.volume, 0.24);
        assert_eq!(saved.output_device, settings.output_device);
        assert_eq!(saved.equalizer, settings.equalizer);
        fs::write(&path, "not json").unwrap();
        assert!(Settings::load(&path).is_err());
    }

    #[test]
    fn older_settings_follow_windows_default_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"volume":0.36,"folders":[]}"#).unwrap();
        let settings = Settings::load(&path).unwrap();
        assert!(settings.output_device.is_none());
        assert!(!settings.equalizer.enabled);
        assert_eq!(settings.volume, 0.36);
    }
}
