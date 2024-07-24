use godot::prelude::*;
use godot::classes::Area3D;
use godot::classes::IArea3D;
use godot::classes::CollisionShape3D;
use godot::classes::SphereShape3D;
use godot::classes::CsgSphere3D;
use godot::classes::Shape3D;
use lazy_static::lazy_static;
use serde_json::{Value, json};
use spelltranslator::get_component_num;
use spelltranslator::parse_spell;
use std::collections::HashMap;

mod spelltranslator;
mod component_functions;

// When a spell has energy below this level it is discarded as being insignificant
const ENERGY_CONSIDERATION_LEVEL: f64 = 0.001;

// Used to control how fast efficiency increases with each cast
const EFFICIENCY_INCREASE_RATE: f64 = 10.0;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

enum ReturnType {
    Float,
    Boolean,
    None
}

static COMPONENT_0_ARGS: &[u64] = &[1, 1, 1];

lazy_static! {
    // Component_bytecode -> (function, parameter types represented by u64, return type of the function for if statements)
    // The u64 type conversion goes as follows: 0 = u64, 1 = f64, 2 = bool
    static ref COMPONENT_TO_FUNCTION_MAP: HashMap<u64, (fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, &'static[u64], ReturnType)> = {
        let mut component_map = HashMap::new();
        component_map.insert(0, (component_functions::give_velocity as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_0_ARGS, ReturnType::None));
        return component_map
    };
}

#[derive(GodotClass)]
#[class(base=Area3D)]
struct Spell {
    base: Base<Area3D>,
    energy: f64,
    energy_lose_rate: f64,
    velocity: Vector3,
    ready_instructions: Vec<u64>,
    process_instructions: Vec<u64>,
    component_efficiency_levels: HashMap<u64, f64>
}


#[godot_api]
impl IArea3D for Spell {
    fn init(base: Base<Area3D>) -> Self {
        Self {
            base,
            energy: 0.0,
            energy_lose_rate: 0.005,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            // Instructions are in u64, to represent f64 convert it to bits with f64::to_bits()
            ready_instructions: vec![],
            process_instructions: vec![],
            component_efficiency_levels: HashMap::new()
        }
    }

    fn ready(&mut self) {
        // Creating visual representation of spell in godot (not required, just a place holder)
        let mut collision_shape: Gd<CollisionShape3D> = CollisionShape3D::new_alloc();
        let shape = SphereShape3D::new_gd();
        collision_shape.set_shape(shape.upcast::<Shape3D>());
        self.base_mut().add_child(collision_shape.upcast::<Node>());
        self.base_mut().add_child(CsgSphere3D::new_alloc().upcast::<Node>());

        // Hanlde instructions
        self.spell_virtual_machine(&self.ready_instructions.clone());

        // Check if spell should be deleted due to lack of energy
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.base_mut().queue_free();
        }
    }

    fn physics_process(&mut self, delta: f64) {
        // Handle velocity
        let f32_delta: f32 = delta as f32;
        let previous_position = self.base_mut().get_position();
        let new_position = previous_position + Vector3 {x: self.velocity.x * f32_delta, y: self.velocity.y * f32_delta, z: self.velocity.z * f32_delta};
        self.base_mut().set_position(new_position);

        // Hanlde instructions
        self.spell_virtual_machine(&self.process_instructions.clone());

        // Handle energy lose
        self.energy = self.energy - self.energy * self.energy_lose_rate * delta;

        // Check if spell should be deleted due to lack of energy
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.base_mut().queue_free();
        }
    }
}


impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u64]) -> Result<(), ()> { // TODO: Handle result in process and ready
        let mut instructions_iter = instructions.iter();
        while let Some(&bits) = instructions_iter.next() {
            match bits {
                0 => {}, // 0 = end of scope, if reached naturely, move on
                103 => { // 103 = component
                    let component_code = instructions_iter.next().expect("Expected component");
                    let number_of_component_parameters = get_number_of_component_parameters(component_code);
                    let mut parameters: Vec<u64> = vec![];
                    for _ in 0..number_of_component_parameters {
                        parameters.push(*instructions_iter.next().expect("Expected parameter"));
                    }
                    self.call_component(component_code, parameters)?;
                },
                400 => { // 400 = if statement
                    let mut rpn_stack: Vec<u64> = vec![];
                    while let Some(&if_bits) = instructions_iter.next() {
                        match if_bits {
                            0 => break,
                            100..=101 => rpn_stack.push(if_bits), // true and false
                            102 => rpn_stack.push(*instructions_iter.next().expect("Expected following value")), // if 102, next bits are a number literal
                            103 => { // Component
                                let component_code = instructions_iter.next().expect("Expected component");
                                let number_of_component_parameters = get_number_of_component_parameters(component_code);
                                let mut parameters: Vec<u64> = vec![];
                                for _ in 0..number_of_component_parameters {
                                    parameters.push(*instructions_iter.next().expect("Expected parameter"));
                                }
                                rpn_stack.extend(self.call_component(component_code, parameters)?.expect("Expected return from function"));
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
                            600 => { // multiply
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                rpn_stack.push(f64::to_bits(argumunt_one * argument_two));
                            }
                            601 => { // divide
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                rpn_stack.push(f64::to_bits(argumunt_one / argument_two));
                            }
                            602 => { // add
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                rpn_stack.push(f64::to_bits(argumunt_one + argument_two));
                            }
                            603 => { // subtract
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                rpn_stack.push(f64::to_bits(argumunt_one - argument_two));
                            }
                            604 => { // power
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let argumunt_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                rpn_stack.push(f64::to_bits(argumunt_one.powf(argument_two)));
                            }
                            _ => panic!("Opcode doesn't exist")
                        }
                    }
                    match rpn_stack.pop().expect("Expected final bool") {
                        100 => {}, // if true, execute by going back into normal loop
                        101 => { // if false, skip to the end of scope
                            let mut skip_amount: usize = 1;
                            while let Some(&skipping_bits) = instructions_iter.next() {
                                match skipping_bits {
                                    0 => skip_amount -= 1, // If end of scope
                                    102 => _ = instructions_iter.next(), // Ignores number literals
                                    103 => {
                                        let component_code = instructions_iter.next().expect("Expected component code"); // Get component num to work out how many parameters to skip
                                        let number_of_component_parameters = get_number_of_component_parameters(component_code);
                                        for _ in 0..number_of_component_parameters {
                                            _ = instructions_iter.next();
                                        }
                                    }
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
                },
                _ => panic!("Not valid opcode")
            }
        }
        return Ok(())
    }


    fn call_component(&mut self, component_code: &u64, parameters: Vec<u64>) -> Result<Option<Vec<u64>>, ()> {
        // Getting component cast count
        if let Some((function, _, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            let mut component_efficiency_level = self.component_efficiency_levels.entry(*component_code).or_insert(1.0).clone();

            // Getting energy required
            if let Some(base_energy_bits) = function(self, &parameters, false) {
                let base_energy = f64::from_bits(*base_energy_bits.first().expect("Expected energy useage return"));
                // Getting efficiency from component_efficiency_level
                let efficiency = component_efficiency_level / (component_efficiency_level + EFFICIENCY_INCREASE_RATE);

                let energy_needed = base_energy / efficiency;
                if self.energy >= energy_needed {
                    self.energy -= energy_needed;

                    // Updating component cast count
                    let efficiency_increase = base_energy;
                    component_efficiency_level += efficiency_increase;
                    self.component_efficiency_levels.insert(*component_code, component_efficiency_level);

                    // Emit signal to say component has been cast
                    self.emit_component_cast(*component_code, efficiency_increase);

                    if let Some(value) = function(self, &parameters, true) {
                        return Ok(Some(value))
                    } else {
                        return Ok(None)
                    }
                } else {
                    return Err(()) // Not enough energy to cast. Maybe change error type to inform what component or what line of bytecode couldn't be cast
                }
            } else {
                panic!("Function should return base_energy when should_execute is false")
            }
        } else {
            panic!("Component does not exist")
        }
    }

    fn emit_component_cast(&mut self, component_code: u64, efficiency_increase: f64) {
        self.base_mut().emit_signal("component_cast".into(), &[Variant::from(component_code), Variant::from(efficiency_increase)]);
    }
}

fn get_number_of_component_parameters(component_code: &u64) -> u64 {
    if let Some((_, number_of_parameters, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
        return number_of_parameters.len() as u64
    } else {
        panic!("Component doesn't exist")
    }
}

#[godot_api]
impl Spell {
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
    fn get_bytecode_efficiency_levels(efficiency_levels_json: GString) -> GString {
        let json_string = efficiency_levels_json.to_string();

        match serde_json::from_str(&json_string) {
            Ok(Value::Object(efficiency_levels_object)) => {
                let mut return_hashmap: HashMap<u64, f64> = HashMap::new();
                for (key, value) in efficiency_levels_object {
                    if let (Some(parsed_key), Some(parsed_value)) = (get_component_num(&key), value.as_f64()) {
                        return_hashmap.insert(parsed_key, parsed_value);
                    }
                }
                let json_object: Value = json!(return_hashmap);
                GString::from(json_object.to_string())
            },
            Ok(_) => panic!("Invalid Json: Must be object"),
            Err(_) => panic!("Invalid Json: Incorrect format")
        }
    }

    #[func]
    fn set_instructions(&mut self, instructions_json: GString) {
        let instructions_string = instructions_json.to_string();
        let instructions: Vec<u64> = serde_json::from_str(&instructions_string).expect("Couldn't parse json instructions");
        let mut section_instructions: Vec<u64> = vec![];
        let mut last_section: u64 = 0;
        for instruction in instructions {
            match instruction {
                500 => match last_section {
                    0 => last_section = 500,
                    501 => {
                        last_section = 500;
                        self.process_instructions = section_instructions.clone();
                        section_instructions.clear();
                    },
                    _ => panic!("Invalid section")
                },
                501 => match last_section {
                    0 => last_section = 501,
                    500 => {
                        last_section = 501;
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
            0 => {},
            _ => panic!("Invalid section")
        }
    }

    #[func]
    fn get_bytecode_instructions(instructions_json: GString) -> Dictionary {
        // Returns a dictionary of the instructions and successful
        let (instructions, successful, error_message) = match parse_spell(&instructions_json.to_string()) {
            Ok(succesful_instructions) => (succesful_instructions, true, GString::new()),
            Err(error) => (Vec::new(), false, GString::from(error))
        };
        return dict!{"instructions": GString::from(serde_json::to_string(&instructions).expect("Failed to parse instructions into json")), "successful": successful, "error_message": error_message}
    }

    #[func]
    fn set_energy(&mut self, energy: f64) {
        self.energy = energy;
    }

    #[func]
    fn get_energy(&self) -> f64 {
        self.energy
    }

    #[func]
    fn set_energy_lose_rate(&mut self, energy_lose_rate: f64) {
        self.energy_lose_rate = energy_lose_rate;
    }

    #[func]
    fn connect_player(&mut self, &player: Gd<Node>) {
        let update_function = player.callable("update_component_efficiency");
        self.base_mut().connect("component_cast".into(), update_function);
    }

    #[signal]
    fn component_cast(component_code: u64, efficiency_increase: f64);
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
