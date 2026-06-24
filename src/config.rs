use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

pub const PRO_NAMES: &[&str] = &[
    "BlueArchive.exe",
    "BlueArchiveNexon.exe",
    "BlueArchiveJP.exe",
];

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub player_name: String,
    pub friend_code: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let Ok(bytes) = fs::read(config_path()) else {
            return Self::default();
        };
        serde_json::from_slice::<Self>(&bytes)
            .map(Self::normalized)
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes =
            serde_json::to_vec_pretty(&self.clone().normalized()).map_err(io::Error::other)?;
        fs::write(path, bytes)
    }

    pub fn normalized(mut self) -> Self {
        self.player_name = compact(&self.player_name, 48);
        self.friend_code = compact(&self.friend_code, 48);
        self
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.player_name.trim().is_empty() {
            return Err("名前を入力してください");
        }
        Ok(())
    }
}

pub fn config_exists() -> bool {
    config_path().is_file()
}

fn compact(value: &str, max_chars: usize) -> String {
    value
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(max_chars)
        .collect()
}

fn config_path() -> PathBuf {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("AppData")
        .join("LocalLow")
        .join("Y2KDevs")
        .join("BluePresence")
        .join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn normal() {
        let config = AppConfig {
            player_name: "  名\n前  ".into(),
            friend_code: "  ABC-123  ".into(),
        }
        .normalized();

        assert_eq!(config.player_name, "名前");
        assert_eq!(config.friend_code, "ABC-123");
    }

    #[test]
    fn reqplayername() {
        assert!(AppConfig::default().validate().is_err());
        assert!(
            AppConfig {
                player_name: "Sensei".into(),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn configg() {
        assert!(config_path().ends_with(Path::new(
            r"AppData\LocalLow\Y2KDevs\BluePresence\config.json"
        )));
    }
}
