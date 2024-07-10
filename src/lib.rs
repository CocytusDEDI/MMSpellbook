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
    ready_instructions: Vec<u32>,
    process_instructions: Vec<u32>,
    component_efficiencies: Option<HashMap<u32, f64>>
}


#[godot_api]
impl IArea3D for Spell {
    fn init(base: Base<Area3D>) -> Self {
        Self {
            base,
            energy: 0.0,
            ready_instructions: vec![],
            process_instructions: vec![],
            component_efficiencies: None
        }
    }

    fn ready(&mut self) {
        let mut collision_shape: Gd<CollisionShape3D> = CollisionShape3D::new_alloc();
        let shape = SphereShape3D::new_gd();
        collision_shape.set_shape(shape.upcast::<Shape3D>());
        self.base_mut().add_child(collision_shape.upcast());
        self.base_mut().add_child(CsgSphere3D::new_alloc().upcast());

        self.spell_virtual_machine(&self.ready_instructions.clone());
    }

    fn physics_process(&mut self, delta: f64) {
        self.spell_virtual_machine(&self.process_instructions.clone());
    }
}

lazy_static! {
    static ref COMPONENT_TO_FUNCTION_MAP: HashMap<u32, (fn(&mut Spell, &[u32], bool) -> Option<u32>, u32)> = {
        let mut component_map = HashMap::new();
        component_map.insert(0, (component_functions::give_velocity as fn(&mut Spell, &[u32], bool) -> Option<u32>, 3));
        return component_map
    };
}


impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u32]) -> Result<(), ()> {
        // ToDo: Code such as rpn evaluation should be in it's own subroutine to be available to call for other logic statements.
        // ToDo: Decide to remove or not remove strings from Parameter

        let mut instructions_iter = instructions.iter();
        while let Some(bits) = instructions_iter.next() {
            match bits {
                103 => { // 103 = component
                    let component_code = *instructions_iter.next().expect("Expected component");
                    let number_of_component_parameters = self.get_number_of_component_parameters(component_code);
                    let mut parameters: Vec<u32> = vec![];
                    for _ in 0..number_of_component_parameters {
                        parameters.push(*instructions_iter.next().expect("Expected parameter"));
                    }
                    self.call_component(component_code, parameters)?;
                },
                400 => {

                },
                _ => panic!("Not valid opcode")
            }
        }
        return Ok(())
    }


    fn call_component(&mut self, component_code: u32, parameters: Vec<u32>) -> Result<Option<u32>, ()> {
        if let Some((function, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            if let Some(component_efficiencies) = self.component_efficiencies.clone() {
                if let Some(efficiency) = component_efficiencies.get(&component_code) {
                    if let Some(base_energy) = function(self, &parameters, false) {
                        let base_energy = f32::from_bits(base_energy) as f64;
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

    fn get_number_of_component_parameters(&self, component_code: u32) -> u32 {
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
                let mut temp_hashmap: HashMap<u32, f64> = HashMap::new();
                for (key, value) in efficiencies_object {
                    if let (Ok(parsed_key), Some(parsed_value)) = (key.parse::<u32>(), value.as_f64()) {
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

fn custom_bool_and(first: u32, second: u32) -> u32 {
    // 100 = true, 101 = false
    match (first, second) {
        (100, 100) => 100,
        (100, 101) => 101,
        (101, 100) => 101,
        (101, 101) => 101,
        _ => panic!("Parameters must be 100 or 101")
    }
}

fn custom_bool_or(first: u32, second: u32) -> u32 {
    // 100 = true, 101 = false
    match (first, second) {
        (100, 100) => 100,
        (100, 101) => 100,
        (101, 100) => 100,
        (101, 101) => 101,
        _ => panic!("Parameters must be 100 or 101")
    }
}

fn custom_bool_xor(first: u32, second: u32) -> u32 {
    // 100 = true, 101 = false
    match (first, second) {
        (100, 100) => 101,
        (100, 101) => 100,
        (101, 100) => 100,
        (101, 101) => 101,
        _ => panic!("Parameters must be 100 or 101")
    }
}

fn custom_bool_not(first: u32) -> u32 {
    // 100 = true, 101 = false
    match first {
        100 => 101,
        101 => 100,
        _ => panic!("Parameters must be 100 or 101")
    }
}

