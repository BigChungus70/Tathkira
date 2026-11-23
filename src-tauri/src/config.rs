use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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

const THEMES: [&str; 3] = ["system", "dark", "light"];
const AUTOSTART_FLAG: &str = "autostart.flag";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub texts: Vec<String>,
    pub interval_minutes: u64,
    pub display_seconds: u64,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_position")]
    pub position: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub launch_on_startup: bool,
}

fn default_language() -> String {
    "en".to_string()
}

fn default_position() -> String {
    "top_right".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            texts: vec![
                "سبحان الله".to_string(),
                "الحمدلله".to_string(),
                "الله أكبر".to_string(),
                "لا إله إلا الله".to_string(),
                "أستغفر الله".to_string(),
                "تبارك الله".to_string(),
                "لا حول ولا قوة إلا بالله".to_string(),
                "سبحان الله وبحمده سبحان الله العظيم".to_string(),
                "لا إله إلا أنت سبحانك إني كنت من الظالمين".to_string(),
                "اللهم صلِّ وسلم وبارك على سيدنا محمد وعلى آله وأصحابه أجمعين".to_string(),
                "اللهم أنت ربي لا إله إلا أنت خلقتني وأنا عبدك وأنا على عهدك ووعدك ما استطعت أعوذ بك من شر ما صنعت أبوء لك بنعمتك علي وأبوء بذنبي فاغفرلي فإنه لا يغفر الذنوب إلا أنت".to_string(),
                "لا إله إلا الله وحده لا شريك له, له الملك وله الحمد وهو على كل شيء قدير".to_string(),
                "ربِّ اغفر لي ولوالديّ".to_string(),
                "سبحانك اللهم وبحمدك أشهد ألّا إله إلا أنت أستغفرك وأتوب إليك".to_string(),
                "أشهد ألّا إله إلا الله و أشهد أن محمداً عبده ورسوله".to_string(),
                "سبحان الله وبحمده عدد خلقه ورضا نفسه وزنة عرشه ومداد كلماته".to_string(),
                "يا حي يا قيوم برحمتك أستغيث أصلح لي شأني ولا تكلني إلى نفسي طرفة عين".to_string(),
                "اللهم فاطر السماوات والأرض عالم الغيب والشهادة أنت تحكم بين عبادك فيما كانوا فيه يختلفون اهدني لما اختلف به من الحق بإذنك إنك تهدي من تشاء إلى صراط مستقيم".to_string(),
            ],
            interval_minutes: 2,
            display_seconds: 10,
            language: default_language(),
            position: default_position(),
            theme: default_theme(),
            launch_on_startup: false,
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
        self.theme = self.theme.trim().to_string();
        if !THEMES.contains(&self.theme.as_str()) {
            self.theme = default_theme();
        }
    }
}

pub fn load_config<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Config> {
    let mut config = match fs::read_to_string(config_path(app)?) {
        Ok(contents) => serde_json::from_str::<Config>(&contents).unwrap_or_default(),
        Err(_) => {
            let legacy = app.path().app_config_dir()?.join("config.json");
            match fs::read_to_string(&legacy) {
                Ok(contents) => {
                    let migrated = serde_json::from_str::<Config>(&contents).unwrap_or_default();
                    let _ = save_config(app, &migrated);
                    let _ = fs::remove_file(&legacy);
                    migrated
                }
                Err(_) => Config::default(),
            }
        }
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

pub fn apply_installer_autostart<R: tauri::Runtime>(app: &tauri::AppHandle<R>, config: &mut Config) {
    let Ok(directory) = config_dir(app) else {
        return;
    };
    let flag_path = directory.join(AUTOSTART_FLAG);
    let Ok(contents) = fs::read_to_string(&flag_path) else {
        return;
    };
    config.launch_on_startup = contents.trim().eq_ignore_ascii_case("true");
    let _ = save_config(app, config);
    let _ = fs::remove_file(flag_path);
}

fn config_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    Ok(config_dir(app)?.join("config.json"))
}

fn config_dir<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    if let Ok(executable) = std::env::current_exe() {
        let portable = executable
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("config");
        if fs::create_dir_all(&portable).is_ok() {
            return Ok(portable);
        }
    }
    app.path().app_config_dir()
}
