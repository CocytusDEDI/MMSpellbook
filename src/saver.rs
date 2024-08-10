use std::collections::HashMap;

// File system imports
use godot::classes::file_access::ModeFlags;
use godot::classes::FileAccess;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::fs;
use toml;

use godot::prelude::*;

use crate::CustomColor;

const SPELL_CONFIG_PATH: &'static str = "Spell/config.toml";

#[derive(Deserialize, Serialize)]
pub struct PlayerConfig {
    pub color: CustomColor
}

#[derive(Default)]
pub struct Config {
    pub forms: HashMap<u64, FormConfig>
}

#[derive(Deserialize)]
struct StringConfig {
    #[serde(default)]
    forms: HashMap<String, FormConfig>
}

#[derive(Deserialize, Clone)]
pub struct FormConfig {
    pub path: String,
    pub energy_required: f64
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

    pub fn save<T>(object: T, path: &str) -> Result<(), &'static str>
    where
        T: Serialize
    {
        let mut json_file = match FileAccess::open(path.into_godot(), ModeFlags::WRITE) {
            Some(file) => file,
            None => {
                let pos = path.rfind('/').ok_or("Couldn't open file and couldn't create file")?;
                let dir_path = &path[..pos];
                godot_print!("dir_path: {}", dir_path);
                godot_print!("directory: {}", &path[pos+1..]);
                DirAccess::open("user://".to_godot()).ok_or("Couldn't open user dir")?.make_dir(GString::from(&path[pos+1..]));
                FileAccess::open(path.into_godot(), ModeFlags::WRITE).ok_or("Couldn't open file with write access")?
            }
        };
        json_file.store_string(serde_json::to_string::<T>(&object).map_err(|_| "Couldn't serialize data")?.into_godot());
        json_file.close();
        Ok(())
    }
}

impl Config {
    pub fn get_config() -> Result<Config, &'static str> {
        StringConfig::load_string_config()?.into_config()
    }
}

impl StringConfig {
    fn into_config(self) -> Result<Config, &'static str> {
        let mut config = Config {forms: HashMap::new()};
        for (key, value) in &self.forms {
            config.forms.insert(key.parse().map_err(|_| "Couldn't parse config.toml: Failed to parse form keys into numbers")?, value.clone());
        }
        Ok(config)
    }

    fn load_string_config() -> Result<StringConfig, &'static str> {
        let config_file = fs::read_to_string(SPELL_CONFIG_PATH).unwrap_or_default();
        toml::de::from_str(&config_file).map_err(|_| "Couldn't parse config.toml forms section")
    }
}
