use std::f64::consts::E;
use std::collections::HashMap;
use serde_json::Value;

use crate::{Spell, ENERGY_CONSIDERATION_LEVEL, saver::PlayerConfig, saver::SpellCatalogue, parse_spell};

// Godot imports
use godot::prelude::*;
use godot::classes::CharacterBody3D;
use godot::classes::ICharacterBody3D;

const FOCUS_LEVEL_TO_FOCUS: f64 = 0.05;

#[derive(GodotClass)]
#[class(base=CharacterBody3D)]
pub struct MagicalEntity {
    base: Base<CharacterBody3D>,
    input: Gd<Input>,
    default_spell_color: Color,
    #[export]
    health: f64,
    #[export]
    shield: f64,
    loaded_spell: Vec<u64>,
    spells_cast: Vec<Gd<Spell>>,
    energy_charged: f64,
    focus_level: f64,
    max_control: f64,
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
            default_spell_color: PlayerConfig::get_or_create_player_config().color.into_spell_color(),
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

#[godot_api]
impl MagicalEntity {
    #[func]
    fn take_damage(&mut self, energy: f64) {
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
    fn update_component_efficiency(&mut self, component: u64, efficiency_increase: f64) {
        let current_efficiency_level = self.component_efficiency_levels.get(&component).unwrap_or(&1.0);
        self.component_efficiency_levels.insert(component, current_efficiency_level + efficiency_increase);
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

    // Could just use self.focus instead and have the increase focus method take a focus_level to increase by

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
    fn handle_spell_casting(&mut self, delta: f64) {
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

            let mut spell = Spell::new_alloc();
            spell.set_position(self.base().get_global_position());

            if Spell::internal_check_allowed_to_cast(self.loaded_spell.clone()).is_err() {
                return
            }

            {
            let mut spell_bind = spell.bind_mut();

            spell_bind.set_energy(self.energy_charged);
            spell_bind.set_color(self.default_spell_color);
            spell_bind.connect_player(self.to_gd().upcast());
            spell_bind.internal_set_efficiency_levels(self.component_efficiency_levels.clone());
            spell_bind.set_instructions_internally(self.loaded_spell.clone());
            }

            self.base_mut().get_tree().expect("Expected scene tree").get_root().expect("Expected root").add_child(&spell);
            self.spells_cast.push(spell);

            self.energy_charged = 0.0;
        }
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
    fn set_instructions(&mut self, instructions: GString) {
        self.loaded_spell = Spell::translate_instructions(instructions)
    }

    #[func]
    fn get_spell_names() -> Array<GString> {
        let mut array = Array::new();
        for spell_name in SpellCatalogue::get_or_create_spell_catalogue().spell_catalogue.keys() {
            array.push(spell_name.clone().into_godot());
        }
        return array
    }

    /// Returns true if the spell was loaded successfully and returns false if not
    #[func]
    fn load_spell(&mut self, name: GString) -> bool {
        let spell_catalogue = SpellCatalogue::get_or_create_spell_catalogue().spell_catalogue;
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
    fn get_spell(&mut self, name: GString) -> Dictionary {
        let spell_catalogue = SpellCatalogue::get_or_create_spell_catalogue().spell_catalogue;
        match spell_catalogue.get(&name.to_string()) {
            Some(spell) => dict! {"spell": spell.clone(), "successful": true},
            None => dict! {"spell": String::new(), "successful": false}
        }
    }

    #[func]
    fn save_spell(spell_name: GString, spell: GString) {
        SpellCatalogue::save_spell(spell_name.into(), spell.into());
    }

    #[func]
    fn reset_spell_catalogue() {
        SpellCatalogue::store_spell_catalogue(SpellCatalogue { spell_catalogue: HashMap::new() });
    }
}
