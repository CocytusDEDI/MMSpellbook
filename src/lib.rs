use godot::prelude::*;
use godot::classes::Area3D;
use godot::classes::IArea3D;
use godot::classes::CollisionShape3D;
use godot::classes::SphereShape3D;
use godot::classes::CsgSphere3D;
use godot::classes::Shape3D;
use lazy_static::lazy_static;
use serde_json::Value;
use std::collections::HashMap;

// When a spell has energy below this level it is discarded as being insignificant
const ENERGY_CONSIDERATION_LEVEL: f64 = 0.001;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

mod spelltranslator;
mod component_functions;

#[derive(GodotClass)]
#[class(base=Area3D)]
struct Spell {
    base: Base<Area3D>,
    energy: f64,
    ready_instructions: Vec<u64>,
    process_instructions: Vec<u64>,
    component_efficiencies: Option<HashMap<u64, f64>>
}


#[godot_api]
impl IArea3D for Spell {
    fn init(base: Base<Area3D>) -> Self {
        Self {
            base,
            energy: 10.0,
            // Instructions are in u64, to represent f64 convert it to bits with f64::to_bits()
            ready_instructions: vec![],
            process_instructions: vec![],
            component_efficiencies: None // Replace None with the following commented out code to run manually without providing efficiencies through json:
            /*
                {
                let mut component_efficiencies_map = HashMap::new();
                component_efficiencies_map.insert(0, 0.5);
                Some(component_efficiencies_map)
            }
            */
        }
    }

    fn ready(&mut self) {
        let mut collision_shape: Gd<CollisionShape3D> = CollisionShape3D::new_alloc();
        let shape = SphereShape3D::new_gd();
        collision_shape.set_shape(shape.upcast::<Shape3D>());
        self.base_mut().add_child(collision_shape.upcast());
        self.base_mut().add_child(CsgSphere3D::new_alloc().upcast());
        self.spell_virtual_machine(&self.ready_instructions.clone(), None);
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.base_mut().queue_free();
        }
    }

    fn physics_process(&mut self, delta: f64) {
        self.spell_virtual_machine(&self.process_instructions.clone(), Some(delta));
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.base_mut().queue_free();
        }
    }
}

lazy_static! {
    static ref COMPONENT_TO_FUNCTION_MAP: HashMap<u64, (fn(&mut Spell, &[u64], bool) -> Option<u64>, u64)> = {
        let mut component_map = HashMap::new();
        component_map.insert(0, (component_functions::give_velocity as fn(&mut Spell, &[u64], bool) -> Option<u64>, 3));
        return component_map
    };
}


impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u64], option_delta: Option<f64>) -> Result<(), ()> {
        // ToDo: Code such as rpn evaluation should be in it's own subroutine to be available to call for other logic statements.
        // ToDo: Decide to remove or not remove strings from Parameter
        let delta: u64 = match option_delta {
            Some(time) => f64::to_bits(time),
            None => f64::to_bits(0.0)
        };
        let mut instructions_iter = instructions.iter();
        while let Some(bits) = instructions_iter.next() {
            match bits {
                103 => { // 103 = component
                    let component_code = *instructions_iter.next().expect("Expected component");
                    let number_of_component_parameters = self.get_number_of_component_parameters(component_code);
                    let mut parameters: Vec<u64> = vec![delta];
                    for _ in 0..number_of_component_parameters {
                        parameters.push(*instructions_iter.next().expect("Expected parameter"));
                    }
                    self.call_component(component_code, parameters)?;
                },
                400 => { // 400 = if statement

                },
                _ => panic!("Not valid opcode")
            }
        }
        return Ok(())
    }


    fn call_component(&mut self, component_code: u64, parameters: Vec<u64>) -> Result<Option<u64>, ()> {
        if let Some((function, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            if let Some(component_efficiencies) = self.component_efficiencies.clone() {
                if let Some(efficiency) = component_efficiencies.get(&component_code) {
                    if let Some(base_energy) = function(self, &parameters, false) {
                        let base_energy = f64::from_bits(base_energy);
                        let energy_needed = base_energy / efficiency;
                        if self.energy >= energy_needed {
                            self.energy -= energy_needed;
                            if let Some(value) = function(self, &parameters, true) {
                                return Ok(Some(value))
                            } else {
                                return Ok(None)
                            }
                        } else {
                            return Err(())
                        }
                    } else {
                        panic!("Function should return base_energy when should_execute is false")
                    }
                } else {
                    panic!("Function does not have an efficiency")
                }
            } else {
                panic!("Component efficiencies not given")
            }
        } else {
            panic!("Component does not exist")
        }
    }

    fn get_number_of_component_parameters(&self, component_code: u64) -> u64 {
        if let Some((_, number_of_parameters)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            return *number_of_parameters
        } else {
            panic!("Component doesn't exist")
        }
    }

    fn give_efficiencies(&mut self, efficiencies_json: GString) {
        let json_string = efficiencies_json.to_string();

        match serde_json::from_str(&json_string) {
            Ok(Value::Object(efficiencies_object)) => {
                let mut temp_hashmap: HashMap<u64, f64> = HashMap::new();
                for (key, value) in efficiencies_object {
                    if let (Ok(parsed_key), Some(parsed_value)) = (key.parse::<u64>(), value.as_f64()) {
                        temp_hashmap.insert(parsed_key, parsed_value);
                    }
                }
                self.component_efficiencies = Some(temp_hashmap);
            },
            Ok(_) => panic!("Invalid Json: Must be object"),
            Err(_) => panic!("Invalid Json: Incorrect format")
        }
    }
}

fn custom_bool_and(first: u64, second: u64) -> u64 {
    // 100 = true, 101 = false
    match (first, second) {
        (100, 100) => 100,
        (100, 101) => 101,
        (101, 100) => 101,
        (101, 101) => 101,
        _ => panic!("Parameters must be 100 or 101")
    }
}

fn custom_bool_or(first: u64, second: u64) -> u64 {
    // 100 = true, 101 = false
    match (first, second) {
        (100, 100) => 100,
        (100, 101) => 100,
        (101, 100) => 100,
        (101, 101) => 101,
        _ => panic!("Parameters must be 100 or 101")
    }
}

fn custom_bool_xor(first: u64, second: u64) -> u64 {
    // 100 = true, 101 = false
    match (first, second) {
        (100, 100) => 101,
        (100, 101) => 100,
        (101, 100) => 100,
        (101, 101) => 101,
        _ => panic!("Parameters must be 100 or 101")
    }
}

fn custom_bool_not(first: u64) -> u64 {
    // 100 = true, 101 = false
    match first {
        100 => 101,
        101 => 100,
        _ => panic!("Parameters must be 100 or 101")
    }
}

