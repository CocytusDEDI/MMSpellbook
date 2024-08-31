use std::collections::HashMap;

// File system imports
use godot::classes::file_access::ModeFlags;
use godot::classes::FileAccess;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::fs;
use toml;

use godot::prelude::*;

use crate::{CustomColor, Shape};

const SPELL_CONFIG_PATH: &'static str = "Spell/config.toml";
const SPELL_SAVE_FOLDER: &'static str = "SpellSave";

pub type StringCustomTranslation = HashMap<String, HashMap<String, u64>>;

#[derive(Deserialize, Serialize)]
pub struct PlayerConfig {
    pub color: CustomColor
}

#[derive(Default)]
pub struct Config {
    pub forms: HashMap<u64, FormConfig>,
    pub custom_translation: StringCustomTranslation
}

#[derive(Deserialize)]
struct StringConfig {
    #[serde(default)]
    forms: HashMap<String, FormConfig>,
    #[serde(default)]
    custom_translation: StringCustomTranslation
}

#[derive(Deserialize, Clone)]
pub struct FormConfig {
    pub path: String,
    pub energy_required: f64,
    pub shape: Shape
}

pub mod godot_json_saver {
    use godot::classes::DirAccess;

    use super::*;

    pub fn from_path<T>(path: &str) -> Result<T, &'static str>
    where
        T: DeserializeOwned
    {
        let mut json_file = FileAccess::open(path.into_godot(), ModeFlags::READ).ok_or("Couldn't open file")?;
        let file_text: String = json_file.get_as_text().into();
        json_file.close();
        let object: T = serde_json::from_str(&file_text).map_err(|_| "Couldn't parse json")?;
        Ok(object)
    }

    pub fn save<T>(object: T, local_path: &str) -> Result<(), &'static str>
    where
        T: Serialize
    {
        let parsed_path = local_path.trim().strip_prefix('/').unwrap_or(local_path);
        let file_path: Vec<&str> = parsed_path.split('/').collect();

        let mut json_file = match FileAccess::open(parsed_path.into_godot(), ModeFlags::WRITE) {
            Some(file) => file,
            None => {
                DirAccess::open("user://".to_godot()).ok_or("Couldn't open user dir")?.make_dir_recursive(GString::from(format!("{}/{}", SPELL_SAVE_FOLDER, file_path[..file_path.len()-1].join("/"))));
                FileAccess::open(format!("user://{}/{}", SPELL_SAVE_FOLDER, file_path.join("/")).into_godot(), ModeFlags::WRITE).ok_or("Couldn't open file with write access")?
            }
        };
        json_file.store_string(serde_json::to_string::<T>(&object).map_err(|_| "Couldn't serialize data")?.into_godot());
        json_file.close();
        Ok(())
    }
}

impl Config {
    pub fn get_config() -> Result<Config, String> {
        StringConfig::load_string_config()?.into_config()
    }
}

impl StringConfig {
    /// Consumes self and converts the `StringConfig` into a normal `Config` wrapped in a result
    fn into_config(self) -> Result<Config, String> {
        let mut config = Config {forms: HashMap::new(), custom_translation: self.custom_translation};
        for (key, value) in &self.forms {
            config.forms.insert(key.parse().map_err(|_| "Couldn't parse config.toml: Failed to parse form keys into numbers")?, value.clone());
        }
        Ok(config)
    }

    fn load_string_config() -> Result<StringConfig, String> {
        let config_file = fs::read_to_string(SPELL_CONFIG_PATH).unwrap_or_default();
        toml::de::from_str(&config_file).map_err(|err| format!("Couldn't parse config.toml: {}", err.message()))
    }
}
