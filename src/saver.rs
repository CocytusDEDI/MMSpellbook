use std::collections::HashMap;

// File system imports
use godot::classes::file_access::ModeFlags;
use godot::classes::FileAccess;
use serde::{Deserialize, Serialize};
use std::fs;
use toml;

use godot::prelude::*;

use crate::CustomColor;

const SPELL_CONFIG_PATH: &'static str = "Spell/config.toml";
const COMPONENT_CATALOGUE_PATH: &'static str = "user://component_catalogue.json";
const SPELL_CATALOGUE_PATH: &'static str = "user://spell_catalogue.json";
const PLAYER_CONFIG_PATH: &'static str = "user://player_config.json";

#[derive(Deserialize, Serialize)]
pub struct ComponentCatalogue {
    pub component_catalogue: HashMap<u64, Vec<Vec<u64>>>
}

#[derive(Deserialize, Serialize)]
pub struct SpellCatalogue {
    pub spell_catalogue: HashMap<String, String>
}

#[derive(Deserialize, Serialize)]
pub struct PlayerConfig {
    pub color: CustomColor
}

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

impl SpellCatalogue {
    pub fn get_or_create_spell_catalogue() -> SpellCatalogue {
        let option_spell_catalogue_file = FileAccess::open(SPELL_CATALOGUE_PATH.into_godot(), ModeFlags::READ);
        let mut spell_catalogue_file = match option_spell_catalogue_file {
            Some(file) => file,
            None => {
                let mut spell_catalogue_file = FileAccess::open(SPELL_CATALOGUE_PATH.into_godot(), ModeFlags::WRITE).expect("Expected to be able to write to spell catalogue");
                spell_catalogue_file.store_string("{}".into_godot());
                spell_catalogue_file.close();
                FileAccess::open(SPELL_CATALOGUE_PATH.into_godot(), ModeFlags::READ).expect("Expected to be able to read spell catalogue")
            }
        };
        let mut spell_catalogue: String = spell_catalogue_file.get_as_text().into();
        spell_catalogue_file.close();
        if spell_catalogue.is_empty() {
            spell_catalogue = "{\"spell_catalogue\": {}}".to_string();
        }
        serde_json::from_str(&spell_catalogue).expect("Couldn't parse spell catalogue")
    }

    pub fn store_spell_catalogue(spell_catalogue: SpellCatalogue) {
        let mut spell_catalogue_file = FileAccess::open(SPELL_CATALOGUE_PATH.into_godot(), ModeFlags::WRITE).expect("Couldn't write to spell catalogue");
        spell_catalogue_file.store_string(serde_json::to_string(&spell_catalogue).expect("Couldn't turn spell catalogue into JSON").into_godot());
        spell_catalogue_file.close()
    }
}

impl PlayerConfig {
    pub fn get_or_create_player_config() -> PlayerConfig {
        let option_player_config_file = FileAccess::open(PLAYER_CONFIG_PATH.into_godot(), ModeFlags::READ);
        let mut player_config_file = match option_player_config_file {
            Some(file) => file,
            None => {
                let mut player_config_file = FileAccess::open(PLAYER_CONFIG_PATH.into_godot(), ModeFlags::WRITE).expect("Expected to be able to write to player config");
                player_config_file.store_string("{}".into_godot());
                player_config_file.close();
                FileAccess::open(PLAYER_CONFIG_PATH.into_godot(), ModeFlags::READ).expect("Expected to be able to read player config")
            }
        };
        let mut player_config: String = player_config_file.get_as_text().into();
        player_config_file.close();
        if player_config.is_empty() {
            player_config = "{\"player_config\": {\"color\": {\"r\": 1, \"g\": 1, \"b\": 1}}}".to_string();
        }
        serde_json::from_str(&player_config).expect("Couldn't parse player config")
    }

    pub fn store_player_config(player_config: PlayerConfig) {
        let mut player_config_file = FileAccess::open(PLAYER_CONFIG_PATH.into_godot(), ModeFlags::WRITE).expect("Couldn't write to player config");
        player_config_file.store_string(serde_json::to_string(&player_config).expect("Couldn't turn player config into JSON").into_godot());
        player_config_file.close()
    }
}

impl Config {
    pub fn get_config() -> Config {
        StringConfig::load_string_config().into_config()
    }
}

impl StringConfig {
    fn into_config(self) -> Config {
        let mut config = Config {forms: HashMap::new()};
        for (key, value) in &self.forms {
            config.forms.insert(key.parse().expect("Couldn't parse config.toml forms section"), value.clone());
        }
        return config
    }

    fn load_string_config() -> StringConfig {
        let config_file = fs::read_to_string(SPELL_CONFIG_PATH).unwrap_or_default();
        toml::de::from_str::<StringConfig>(&config_file).expect("Couldn't parse config.toml")
    }
}

impl ComponentCatalogue {
    pub fn get_component_catalogue() -> ComponentCatalogue {
        let mut component_catalogue_file = FileAccess::open(COMPONENT_CATALOGUE_PATH.into_godot(), ModeFlags::READ).expect("Couldn't open component catalogue");
        let component_catalogue: String = component_catalogue_file.get_as_text().into();
        component_catalogue_file.close();
        serde_json::from_str(&component_catalogue).expect("Couldn't parse component catalogue")
    }

    pub fn get_or_create_component_catalogue() -> ComponentCatalogue {
        let option_component_catalogue_file = FileAccess::open(COMPONENT_CATALOGUE_PATH.into_godot(), ModeFlags::READ);
        let mut component_catalogue_file = match option_component_catalogue_file {
            Some(file) => file,
            None => {
                let mut component_catalogue_file = FileAccess::open(COMPONENT_CATALOGUE_PATH.into_godot(), ModeFlags::WRITE).expect("Expected to be able to write to component catalogue");
                component_catalogue_file.store_string("{}".into_godot());
                component_catalogue_file.close();
                FileAccess::open(COMPONENT_CATALOGUE_PATH.into_godot(), ModeFlags::READ).expect("Expected to be able to read component catalogue")
            }
        };
        let mut component_catalogue: String = component_catalogue_file.get_as_text().into();
        component_catalogue_file.close();
        if component_catalogue.is_empty() {
            component_catalogue = "{\"component_catalogue\": {}}".to_string();
        }
        serde_json::from_str(&component_catalogue).expect("Couldn't parse component catalogue")
    }

    pub fn store_component_catalogue(component_catalogue: ComponentCatalogue) {
        let mut component_catalogue_file = FileAccess::open(COMPONENT_CATALOGUE_PATH.into_godot(), ModeFlags::WRITE).expect("Couldn't write to component catalogue");
        component_catalogue_file.store_string(serde_json::to_string(&component_catalogue).expect("Couldn't turn component catalogue into JSON").into_godot());
        component_catalogue_file.close()
    }
}
