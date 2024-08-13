use std::f64::consts::E;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::{ComponentCatalogue, DEFAULT_COLOR};
use crate::{Spell, ENERGY_CONSIDERATION_LEVEL, saver::*, parse_spell, get_component_num};

// Godot imports
use godot::prelude::*;
use godot::classes::CharacterBody3D;
use godot::classes::ICharacterBody3D;

const FOCUS_LEVEL_TO_FOCUS: f64 = 0.05;

#[derive(Deserialize, Serialize)]
pub struct SpellCatalogue {
    pub spell_catalogue: HashMap<String, String>
}

impl SpellCatalogue {
    fn new() -> Self {
        SpellCatalogue { spell_catalogue: HashMap::new() }
    }

    pub fn save_spell(spell_name: String, spell: String, save_path: &str) {
        let mut spell_catalogue: SpellCatalogue = godot_json_saver::from_path(save_path).unwrap();
        spell_catalogue.spell_catalogue.insert(spell_name, spell);
        godot_json_saver::save(spell_catalogue, &format!("{}/spell_catalogue", save_path)).unwrap();
    }
}

#[derive(GodotClass)]
#[class(base=CharacterBody3D)]
pub struct MagicalEntity {
    base: Base<CharacterBody3D>,
    input: Gd<Input>,
    save_path: Option<String>,
    check_allowed_to_cast: bool,
    component_catalogue: ComponentCatalogue,
    spell_color: Color,
    #[export]
    health: f64,
    #[export]
    shield: f64,
    #[export]
    max_health: f64,
    #[export]
    max_shield: f64,
    loaded_spell: Vec<u64>,
    spells_cast: Vec<Gd<Spell>>,
    energy_charged: f64,
    #[export]
    focus_level: f64,
    #[export]
    max_control: f64,
    #[export]
    max_power: f64,
    power_left: f64, // Percentage
    component_efficiency_levels: HashMap<u64, f64>
}

#[godot_api]
impl ICharacterBody3D for MagicalEntity {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            base,
            input: Input::singleton(),
            save_path: None,
            check_allowed_to_cast: true,
            component_catalogue: ComponentCatalogue::new(),
            spell_color: DEFAULT_COLOR.into_spell_color(),
            max_health: 0.0,
            max_shield: 0.0,
            health: 0.0,
            shield: 0.0,
            loaded_spell: Vec::new(),
            spells_cast: Vec::new(),
            energy_charged: 0.0,
            focus_level: 0.0,
            max_control: 100.0,
            max_power: 10.0,
            power_left: 1.0,
            component_efficiency_levels: HashMap::new()
        }
    }
}

impl MagicalEntity {
    fn get_save_path_reference(&self) -> &str {
        match self.save_path {
            Some(ref path) => path,
            None => panic!("Save path wasn't set")
        }
    }

    pub fn owns_spell(&self, spell: Gd<Spell>) -> bool {
        for owned_spell in &self.spells_cast {
            if &spell == owned_spell {
                return true
            }
        }
        return false
    }
}

#[godot_api]
impl MagicalEntity {
    #[func]
    pub fn get_health_and_shield(&self) -> f64 {
        self.health + self.shield
    }

    #[func]
    pub fn take_damage(&mut self, energy: f64) {
        if self.shield - energy > 0.0 {
            self.shield -= energy;
        } else {
            let energy_remaining = energy - self.shield;
            self.shield = 0.0;
            if self.health - energy_remaining > 0.0 {
                self.health -= energy_remaining;
            } else {
                self.health = 0.0;
                self.perish();
            }
        }
    }

    #[func]
    fn perish(&mut self) {
        self.base_mut().queue_free();
    }

    /// Focus factors into current power output and current control. Focus ranges from 0 to 2 with the default state being 1.
    #[func]
    fn get_focus(&self) -> f64 {
        2.0 / (1.0 + E.powf(-self.focus_level * FOCUS_LEVEL_TO_FOCUS))
    }

    #[func]
    fn get_power(&self) -> f64 {
        self.max_power * self.get_focus() * self.power_left
    }

    #[func]
    fn get_control(&mut self) -> f64 {
        let mut spell_energies: f64 = 0.0;
        self.spells_cast.retain(|spell| {
            if spell.is_instance_valid() {
                let spell_bind = spell.bind();
                spell_energies += spell_bind.get_energy();
                true
            } else {
                false
            }
        });

        self.max_control * self.get_focus() - spell_energies
    }

    #[func]
    fn handle_player_spell_casting(&mut self, delta: f64) {
        let control = self.get_control();
        if self.input.is_action_pressed("cast".into()) {
            let extra_energy = self.get_power() * delta;
            if control >= self.energy_charged + extra_energy {
                self.energy_charged += extra_energy;
            } else {
                self.energy_charged = control;
            }
        } else if self.input.is_action_just_released("cast".into()) {
            if self.energy_charged < ENERGY_CONSIDERATION_LEVEL {
                return
            }
            if control < self.energy_charged {
                self.energy_charged = control;
            }

            self.cast_spell();
        }
    }

    #[func]
    fn cast_spell(&mut self) {
        let mut spell = Spell::new_alloc();
        spell.set_position(self.base().get_global_position());


        if self.check_allowed_to_cast {
            if Spell::internal_check_allowed_to_cast(self.loaded_spell.clone(), &self.component_catalogue).is_err() {
                return
            }
        }

        {
            let mut spell_bind = spell.bind_mut();

            spell_bind.set_energy(self.energy_charged);
            spell_bind.set_color(self.spell_color);
            spell_bind.connect_player(self.to_gd().upcast());
            spell_bind.internal_set_efficiency_levels(self.component_efficiency_levels.clone());
            spell_bind.internal_set_instructions(self.loaded_spell.clone());
        }

        self.base_mut().get_tree().expect("Expected scene tree").get_root().expect("Expected root").add_child(&spell);
        self.spells_cast.push(spell);

        self.energy_charged = 0.0;
    }

    #[func]
    fn set_efficiency_levels(&mut self, efficiency_levels_bytecode_json: GString) {
        let json_string = efficiency_levels_bytecode_json.to_string();

        match serde_json::from_str(&json_string) {
            Ok(Value::Object(efficiency_levels_object)) => {
                let mut temp_hashmap: HashMap<u64, f64> = HashMap::new();
                for (key, value) in efficiency_levels_object {
                    if let (Ok(parsed_key), Some(parsed_value)) = (key.parse::<u64>(), value.as_f64()) {
                        temp_hashmap.insert(parsed_key, parsed_value);
                    }
                }
                self.component_efficiency_levels = temp_hashmap;
            },
            Ok(_) => panic!("Invalid Json: Must be object"),
            Err(_) => panic!("Invalid Json: Incorrect format")
        }
    }

    #[func]
    fn increase_component_efficiency(&mut self, component: u64, efficiency_increase: f64) {
        let current_efficiency_level = self.component_efficiency_levels.get(&component).unwrap_or(&1.0);
        self.component_efficiency_levels.insert(component, current_efficiency_level + efficiency_increase);
    }

    #[func]
    fn set_loaded_spell(&mut self, spell: GString) {
        self.loaded_spell = Spell::translate_instructions(&spell)
    }

    #[func]
    fn unset_loaded_spell(&mut self) {
        self.loaded_spell = Vec::new();
    }

    #[func]
    fn get_spell_names(&self) -> Array<GString> {
        let mut array = Array::new();
        for spell_name in godot_json_saver::from_path::<SpellCatalogue>(&(format!("{}/spell_catalogue", self.get_save_path_reference()))).unwrap().spell_catalogue.keys() {
            array.push(spell_name.clone().into_godot());
        }
        return array
    }

    /// Returns true if the spell was loaded successfully and returns false if not
    #[func]
    fn load_spell(&mut self, name: GString) -> bool {
        let spell_catalogue = godot_json_saver::from_path::<SpellCatalogue>(self.get_save_path_reference()).unwrap().spell_catalogue;
        let spell_option = spell_catalogue.get(&name.to_string());
        let spell = match spell_option {
            Some(spell) => spell,
            None => return false
        };

        self.loaded_spell = match parse_spell(spell) {
            Ok(instr) => instr,
            Err(_) => return false
        };

        return true
    }

    #[func]
    fn save_spell(&self, spell_name: GString, spell: GString) {
        SpellCatalogue::save_spell(spell_name.to_string(), spell.to_string(), self.get_save_path_reference());
    }

    #[func]
    fn get_spell(&mut self, name: GString) -> Dictionary {
        let spell_catalogue = godot_json_saver::from_path::<SpellCatalogue>(self.get_save_path_reference()).unwrap().spell_catalogue;
        match spell_catalogue.get(&name.to_string()) {
            Some(spell) => dict! {"spell": spell.clone(), "successful": true},
            None => dict! {"spell": String::new(), "successful": false}
        }
    }

    #[func]
    fn delete_spell_catalogue(&self) {
        godot_json_saver::save(SpellCatalogue::new(), &format!("{}/spell_catalogue", self.get_save_path_reference())).unwrap();
    }

    #[func]
    fn delete_component_catalogue(&mut self) {
        godot_json_saver::save(SpellCatalogue::new(), &format!("{}/component_catalogue", self.get_save_path_reference())).unwrap();
    }

    #[func]
    fn reset_component_catalogue(&mut self) {
        self.component_catalogue = ComponentCatalogue::new();
    }

    #[func]
    fn save_component_catalogue(&self) {
        godot_json_saver::save(self.component_catalogue.clone(), &format!("{}/component_catalogue", self.get_save_path_reference())).unwrap();
    }

    #[func]
    fn set_save_path(&mut self, save_path: GString) {
        self.save_path = Some(save_path.to_string())
    }

    #[func]
    fn load_saved_data(&mut self) {
        match godot_json_saver::from_path::<PlayerConfig>(&format!("{}/player_config", self.get_save_path_reference())) {
            Ok(player_config) => self.spell_color = player_config.color.into_spell_color(),
            Err(_) => {}
        };
        match godot_json_saver::from_path::<ComponentCatalogue>(&format!("{}/component_catalogue", self.get_save_path_reference())) {
            Ok(component_catalogue) => self.component_catalogue = component_catalogue,
            Err(_) => {}
        };
    }

    #[func]
    fn add_component(&mut self, component: GString) {
        let component_code = get_component_num(&component.to_string()).expect("Component doesn't exist");
        let number_of_parameters = Spell::get_number_of_component_parameters(&component_code);
        let mut parameter_restrictions: Vec<Vec<&str>> = Vec::new();
        for _ in 0..number_of_parameters {
            parameter_restrictions.push(vec!["ANY"]);
        }
        Spell::add_component_to_component_catalogue(component_code, parameter_restrictions, &mut self.component_catalogue);
    }

    #[func]
    fn add_restricted_component(&mut self, component: GString, parameter_restrictions: GString) {
        let component_code = get_component_num(&component.to_string()).expect("Component doesn't exist");
        let string_parameter_restrictions = parameter_restrictions.to_string();
        let parameter_restrictions: Vec<Vec<&str>> = serde_json::from_str(&string_parameter_restrictions).expect("Couldn't parse JSON");
        Spell::add_component_to_component_catalogue(component_code, parameter_restrictions, &mut self.component_catalogue);
    }

    #[func]
    fn remove_component(&mut self, component: GString) {
        let component_code = get_component_num(&component.to_string()).expect("Component doesn't exist");
        self.component_catalogue.component_catalogue.remove(&component_code);
    }

    #[func]
    fn check_allowed_to_cast(&self, instructions_json: GString) -> Dictionary {
        let (allowed_to_cast, denial_reason) = match Spell::internal_check_allowed_to_cast(Spell::translate_instructions(&instructions_json), &self.component_catalogue) {
            Ok(_) => (true, ""),
            Err(error_message) => (false, error_message)
        };
        return dict! {"allowed_to_cast": allowed_to_cast, "denial_reason": denial_reason}
    }

    #[func]
    fn increase_energy_charged(&mut self, energy: f64) {
        self.energy_charged += energy;
    }
}
