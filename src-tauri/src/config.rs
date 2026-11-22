use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{Manager, Result};

const POSITIONS: [&str; 9] = [
    "top_left",
    "top_center",
    "top_right",
    "middle_left",
    "middle_center",
    "middle_right",
    "bottom_left",
    "bottom_center",
    "bottom_right",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub texts: Vec<String>,
    pub interval_minutes: u64,
    pub display_seconds: u64,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_position")]
    pub position: String,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_position() -> String {
    "bottom_right".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            texts: vec![
                "Subhanallah".to_string(),
                "Alhamdulillah".to_string(),
                "Allahu Akbar".to_string(),
            ],
            interval_minutes: 15,
            display_seconds: 8,
            language: default_language(),
            position: default_position(),
        }
    }
}

impl Config {
    pub fn clamp(&mut self) {
        self.interval_minutes = self.interval_minutes.clamp(1, 1440);
        self.display_seconds = self.display_seconds.clamp(1, 120);
        self.texts = self
            .texts
            .iter()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .collect();
        self.language = self.language.trim().to_string();
        if self.language.is_empty() {
            self.language = default_language();
        }
        self.position = self.position.trim().to_string();
        if !POSITIONS.contains(&self.position.as_str()) {
            self.position = default_position();
        }
    }
}

pub fn load_config<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Config> {
    let mut config = match fs::read_to_string(config_path(app)?) {
        Ok(contents) => serde_json::from_str::<Config>(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    };
    config.clamp();
    Ok(config)
}

pub fn save_config<R: tauri::Runtime>(app: &tauri::AppHandle<R>, config: &Config) -> Result<()> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

fn config_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("config.json"))
}
