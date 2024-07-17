use godot::prelude::*;
use godot::classes::Area3D;
use godot::classes::IArea3D;
use godot::classes::CollisionShape3D;
use godot::classes::SphereShape3D;
use godot::classes::CsgSphere3D;
use godot::classes::Shape3D;
use lazy_static::lazy_static;
use serde_json::Value;
use spelltranslator::get_component_num;
use spelltranslator::parse_spell;
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

static COMPONENT_0_ARGS: &[u64] = &[1, 1, 1];

lazy_static! {
    // Component_bytecode -> (function, parameter types represented by u64)
    // The u64 type conversion goes as follows: 0 = u64, 1 = f64, 2 = bool
    static ref COMPONENT_TO_FUNCTION_MAP: HashMap<u64, (fn(&mut Spell, &[u64], f64, bool) -> Option<u64>, &'static[u64])> = {
        let mut component_map = HashMap::new();
        component_map.insert(0, (component_functions::give_velocity as fn(&mut Spell, &[u64], f64, bool) -> Option<u64>, COMPONENT_0_ARGS));
        return component_map
    };
}


impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u64], option_delta: Option<f64>) -> Result<(), ()> {
        // ToDo: Code such as rpn evaluation should be in it's own subroutine to be available to call for other logic statements.
        // ToDo: Decide to remove or not remove strings from Parameter
        let mut instructions_iter = instructions.iter();
        while let Some(&bits) = instructions_iter.next() {
            match bits {
                0 => {}, // 0 = end of scope, if reached naturely, move on
                103 => { // 103 = component
                    let component_code = *instructions_iter.next().expect("Expected component");
                    let number_of_component_parameters = self.get_number_of_component_parameters(component_code);
                    let mut parameters: Vec<u64> = vec![];
                    for _ in 0..number_of_component_parameters {
                        parameters.push(*instructions_iter.next().expect("Expected parameter"));
                    }
                    self.call_component(component_code, parameters, option_delta)?;
                },
                400 => { // 400 = if statement
                    let mut rpn_stack: Vec<u64> = vec![];
                    while let Some(&if_bits) = instructions_iter.next() {
                        match if_bits {
                            0 => break,
                            100..=101 => rpn_stack.push(if_bits), // true and false
                            102 => rpn_stack.push(*instructions_iter.next().expect("Expected following value")), // if 102, next bits are a number literal
                            103 => { // Component
                                let component_code = *instructions_iter.next().expect("Expected component");
                                let number_of_component_parameters = self.get_number_of_component_parameters(component_code);
                                let mut parameters: Vec<u64> = vec![];
                                for _ in 0..number_of_component_parameters {
                                    parameters.push(*instructions_iter.next().expect("Expected parameter"));
                                }
                                rpn_stack.push(self.call_component(component_code, parameters, option_delta)?.expect("Expected return from function"));
                            }
                            200 => { // And statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(custom_bool_and(bool_one, bool_two));
                            },
                            201 => { // Or statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(custom_bool_or(bool_one, bool_two));
                            },
                            202 => { // Not statement
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(custom_bool_not(bool_one));
                            },
                            203 => { // Xor statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(custom_bool_xor(bool_one, bool_two));
                            },
                            300 => { // equals
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                if argumunt_one == argument_two {
                                    rpn_stack.push(100);
                                } else {
                                    rpn_stack.push(101);
                                }
                            },
                            301 => { // greater than
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                if argumunt_one > argument_two {
                                    rpn_stack.push(100);
                                } else {
                                    rpn_stack.push(101);
                                }
                            },
                            302 => { // lesser than
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                if argumunt_one < argument_two {
                                    rpn_stack.push(100);
                                } else {
                                    rpn_stack.push(101);
                                }
                            },
                            _ => panic!("Opcode doesn't exist")
                        }
                    match rpn_stack.pop().expect("Expected final bool") {
                        100 => {}, // if true, execute by going back into normal loop
                        101 => { // if false, skip to the end of scope
                            let mut skip_amount: u32 = 1;
                            while let Some(&skipping_bits) = instructions_iter.next() {
                                match skipping_bits {
                                    0 => skip_amount -= 1, // If end of scope
                                    102 => _ = instructions_iter.next(), // Ignores number literals
                                    400 => skip_amount += 2, // Ignore next two end of scopes because if statements have two end of scopes
                                    _ => {}
                                }
                                if skip_amount == 0 {
                                    break;
                                }
                            }
                        }
                        _ => panic!("Expected bool")
                    };
                    }
                },
                _ => panic!("Not valid opcode")
            }
        }
        return Ok(())
    }


    fn call_component(&mut self, component_code: u64, parameters: Vec<u64>, option_delta: Option<f64>) -> Result<Option<u64>, ()> {
        let delta = match option_delta {
            Some(num) => num,
            None => 1.0
        };
        if let Some((function, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            if let Some(component_efficiencies) = self.component_efficiencies.clone() {
                if let Some(efficiency) = component_efficiencies.get(&component_code) {
                    if let Some(base_energy) = function(self, &parameters, delta, false) {
                        let base_energy = f64::from_bits(base_energy);
                        let energy_needed = base_energy / efficiency;
                        if self.energy >= energy_needed {
                            self.energy -= energy_needed;
                            if let Some(value) = function(self, &parameters, delta, true) {
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
            return number_of_parameters.len() as u64
        } else {
            panic!("Component doesn't exist")
        }
    }
}

#[godot_api]
impl Spell {
    #[func]
    fn give_efficiencies(&mut self, efficiencies_json: GString) {
        let json_string = efficiencies_json.to_string();

        match serde_json::from_str(&json_string) {
            Ok(Value::Object(efficiencies_object)) => {
                let mut temp_hashmap: HashMap<u64, f64> = HashMap::new();
                for (key, value) in efficiencies_object {
                    if let (Ok(parsed_key), Some(parsed_value)) = (key.parse::<String>(), value.as_f64()) {
                        if let Some(component_num) = get_component_num(&parsed_key) {
                            temp_hashmap.insert(component_num, parsed_value);
                        } else {
                            panic!("Component doesn't exist");
                        }
                    }
                }
                self.component_efficiencies = Some(temp_hashmap);
            },
            Ok(_) => panic!("Invalid Json: Must be object"),
            Err(_) => panic!("Invalid Json: Incorrect format")
        }
    }

    #[func]
    fn give_instructions(&mut self, instructions_json: GString) {
        let instructions_string = instructions_json.to_string();
        let instructions: Vec<u64> = serde_json::from_str(&instructions_string).expect("Couldn't parse json instructions");
        let mut section_instructions: Vec<u64> = vec![];
        let mut last_section: u64 = 0;
        for instruction in instructions {
            match instruction {
                500 => match last_section {
                    0 => last_section = 500,
                    501 => {
                        self.process_instructions = section_instructions.clone();
                        section_instructions.clear();
                    },
                    _ => panic!("Invalid section")
                },
                501 => match last_section {
                    0 => last_section = 501,
                    500 => {
                        self.ready_instructions = section_instructions.clone();
                        section_instructions.clear();
                    },
                    _ => panic!("Invalid section")
                },
                num => section_instructions.push(num)
            }
        }
        match last_section {
            500 => self.ready_instructions = section_instructions.clone(),
            501 => self.process_instructions = section_instructions.clone(),
            _ => panic!("Invalid section")
        }
    }

    #[func]
    fn get_instructions(instructions_json: GString) -> GString {
        return GString::from(serde_json::to_string(&parse_spell(&instructions_json.to_string()).expect("Failed to turn instructions into bytecode")).expect("Failed to parse instructions into json"))
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

