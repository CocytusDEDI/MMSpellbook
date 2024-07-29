use godot::classes::file_access;
use godot::classes::FileAccess;
use godot::prelude::*;
use godot::classes::Time;
use godot::classes::Area3D;
use godot::classes::IArea3D;
use godot::classes::CollisionShape3D;
use godot::classes::SphereShape3D;
use godot::classes::CsgSphere3D;
use godot::classes::Shape3D;
use godot::classes::StandardMaterial3D;
use godot::classes::base_material_3d::Transparency;
use godot::classes::base_material_3d::Feature;
use lazy_static::lazy_static;
use serde_json::{Value, json};
use serde::Deserialize;
use spelltranslator::get_component_num;
use spelltranslator::parse_spell;
use std::collections::HashMap;
use std::fs;
use toml;

mod spelltranslator;
mod component_functions;

const SPELL_CONFIG_PATH: &'static str = "Spell/config.toml";
const SPELL_CATALOGUE_PATH: &'static str = "user://spell_catalogue.tres";

// When a spell has energy below this level it is discarded as being insignificant
const ENERGY_CONSIDERATION_LEVEL: f64 = 1.0;

// Used to control how fast efficiency increases with each cast
const EFFICIENCY_INCREASE_RATE: f64 = 15.0;

// Used to control how fast energy is lost passively over time. Is a fraction of total spell energy.
const ENERGY_LOSE_RATE: f64 = 0.05;

// Used to determin how Transparent the default spell is. 0 = fully transparent, 1 = opaque
const SPELL_TRANSPARENCY: f32 = 0.9;

const ENERGY_TO_RADIUS_CONSTANT: f64 = 100.0;

// Default spell color
struct DefaultColor {
    r: f32,
    g: f32,
    b: f32
}

const DEFAULT_COLOR: DefaultColor = DefaultColor { r: 1.0, g: 1.0, b: 1.0 };

#[derive(Deserialize)]
struct StringConfig {
    #[serde(default)]
    forms: HashMap<String, FormConfig>
}

#[derive(Deserialize, Clone)]
struct FormConfig {
    path: String,
    energy_required: f64
}

impl StringConfig {
    fn to_config(&self) -> Config {
        let mut config = Config {forms: HashMap::new()};
        for (key, value) in &self.forms {
            config.forms.insert(key.parse().expect("Couldn't parse config.toml forms section"), value.clone());
        }
        return config
    }
}

struct Config {
    forms: HashMap<u64, FormConfig>
}

fn load_string_config() -> StringConfig {
    let config_file = fs::read_to_string(SPELL_CONFIG_PATH).unwrap_or_default();
    toml::de::from_str::<StringConfig>(&config_file).expect("Couldn't parse config.toml")
}

fn load_spell_catalogue() -> Dictionary {
    let spell_catalogue_file = match FileAccess::open(SPELL_CATALOGUE_PATH.into_godot(), file_access::ModeFlags::READ) {
        Some(file) => file,
        None => panic!("Couldn't open spell catalogue")
    };
    spell_catalogue_file.get_var().to()
}

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

enum ReturnType {
    Float,
    Boolean,
    None
}

static COMPONENT_0_ARGS: &[u64] = &[1, 1, 1];
static COMPONENT_1_ARGS: &[u64] = &[1];
static COMPONENT_2_ARGS: &[u64] = &[];

lazy_static! {
    // Component_bytecode -> (function, parameter types represented by u64, return type of the function for if statements)
    // The u64 type conversion goes as follows: 1 = f64, 2 = bool
    static ref COMPONENT_TO_FUNCTION_MAP: HashMap<u64, (fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, &'static[u64], ReturnType)> = {
        let mut component_map = HashMap::new();
        // Utility:
        component_map.insert(0, (component_functions::give_velocity as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_0_ARGS, ReturnType::None));
        component_map.insert(1, (component_functions::take_form as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_1_ARGS, ReturnType::None));
        component_map.insert(2, (component_functions::undo_form as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_2_ARGS, ReturnType::None));

        // Logic:
        component_map.insert(1000, (component_functions::moving as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_1_ARGS, ReturnType::Boolean));
        component_map.insert(1001, (component_functions::get_time as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_2_ARGS, ReturnType::Float));

        // Power:
        // None

        return component_map
    };
}

#[derive(GodotClass)]
#[class(base=Area3D)]
struct Spell {
    base: Base<Area3D>,
    energy: f64,
    color: Color,
    energy_lose_rate: f64,
    form_set: bool,
    config: Config,
    velocity: Vector3,
    time: Option<Gd<Time>>,
    start_time: Option<u64>,
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
            color: Color::from_rgba(DEFAULT_COLOR.r, DEFAULT_COLOR.g, DEFAULT_COLOR.b, SPELL_TRANSPARENCY),
            energy_lose_rate: ENERGY_LOSE_RATE,
            form_set: false,
            config: load_string_config().to_config(),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            time: None,
            start_time: None,
            // Instructions are in u64, to represent f64 convert it to bits with f64::to_bits()
            ready_instructions: Vec::new(),
            process_instructions: Vec::new(),
            component_efficiency_levels: HashMap::new()
        }
    }

    fn ready(&mut self) {
        // Starting time
        self.time = Some(Time::singleton());
        if let Some(ref time) = self.time {
            self.start_time = Some(time.get_ticks_msec());
        } else {
            panic!("Time not available")
        }

        // Creating visual representation of spell in godot
        let mut collision_shape = CollisionShape3D::new_alloc();
        collision_shape.set_name("spell_collision_shape".into_godot());
        let mut shape = SphereShape3D::new_gd();
        shape.set_name("spell_sphere_shape".into_godot());
        let radius = Spell::energy_to_radius(self.energy);
        shape.set_radius(radius);
        collision_shape.set_shape(shape.upcast::<Shape3D>());
        self.base_mut().add_child(collision_shape.upcast::<Node>());
        let mut csg_sphere = CsgSphere3D::new_alloc();
        csg_sphere.set_name("spell_csg_sphere".into_godot());
        csg_sphere.set_radial_segments(20);
        csg_sphere.set_rings(18);
        csg_sphere.set_radius(radius);
        let mut csg_material = StandardMaterial3D::new_gd();

        // Player defined material properties
        csg_material.set_albedo(self.color);

        // Constant material properties
        csg_material.set_transparency(Transparency::ALPHA); // Transparency type
        csg_material.set_feature(Feature::EMISSION, true); // Allows spell to emit light
        csg_material.set_emission(self.color); // Chooses what light to emit
        csg_sphere.set_material(csg_material);
        self.base_mut().add_child(csg_sphere.upcast::<Node>());

        // Hanlde instructions, throws error if it doesn't have enough energy to cast a component
        match self.spell_virtual_machine(&self.ready_instructions.clone()) {
            Ok(()) => {},
            Err(()) => self.free_spell()
        }

        // Check if spell should be deleted due to lack of energy
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.free_spell();
        }
    }

    fn physics_process(&mut self, delta: f64) {
        // Handle velocity
        let f32_delta: f32 = delta as f32;
        let previous_position = self.base_mut().get_position();
        let new_position = previous_position + Vector3 {x: self.velocity.x * f32_delta, y: self.velocity.y * f32_delta, z: self.velocity.z * f32_delta};
        self.base_mut().set_position(new_position);

        // Hanlde instructions, throws error if it doesn't have enough energy to cast a component
        match self.spell_virtual_machine(&self.process_instructions.clone()) {
            Ok(()) => {},
            Err(()) => self.free_spell()
        }

        // Handle energy lose
        self.energy = self.energy - self.energy * self.energy_lose_rate * delta;

        if !self.form_set {
            // Radius changing of collision shape
            let radius = Spell::energy_to_radius(self.energy);

            let collsion_shape = self.base_mut().get_node_as::<CollisionShape3D>("spell_collision_shape");
            let shape = collsion_shape.get_shape().unwrap();
            let mut sphere = shape.cast::<SphereShape3D>();
            sphere.set_radius(radius);

            // Changing radius of csg sphere
            let mut csg_sphere = self.base_mut().get_node_as::<CsgSphere3D>("spell_csg_sphere");
            csg_sphere.set_radius(radius);
        }


        // Check if spell should be deleted due to lack of energy
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.free_spell();
        }
    }
}


impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u64]) -> Result<(), ()> {
        let mut instructions_iter = instructions.iter();
        while let Some(&bits) = instructions_iter.next() {
            match bits {
                0 => {}, // 0 = end of scope, if reached naturely, move on
                103 => { // 103 = component
                    let _ = self.execute_component(&mut instructions_iter);
                },
                400 => { // 400 = if statement
                    let mut rpn_stack: Vec<u64> = vec![];
                    while let Some(&if_bits) = instructions_iter.next() {
                        match if_bits {
                            0 => break,
                            100..=101 => rpn_stack.push(if_bits), // true and false
                            102 => rpn_stack.push(*instructions_iter.next().expect("Expected following value")), // if 102, next bits are a number literal
                            103 => { // Component
                                rpn_stack.extend(self.execute_component(&mut instructions_iter)?)
                            }
                            200 => { // And statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::and(bool_one, bool_two));
                            },
                            201 => { // Or statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::or(bool_one, bool_two));
                            },
                            202 => { // Not statement
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::not(bool_one));
                            },
                            203 => { // Xor statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::xor(bool_one, bool_two));
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
                                        self.skip_component(&mut instructions_iter);
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

    fn skip_component<'a>(&mut self, instructions_iter: &mut impl Iterator<Item = &'a u64>) {
        let component_code = instructions_iter.next().expect("Expected component");
        let number_of_component_parameters = Spell::get_number_of_component_parameters(component_code);
        for _ in 0..number_of_component_parameters {
            let parameter = *instructions_iter.next().expect("Expected parameter");
            match parameter {
                100..=101 => {},
                102 => _ = *instructions_iter.next().expect("Expected number after number literal opcode"),
                103 => _ = self.execute_component(instructions_iter),
                _ => panic!("Invalid parameter skipped")
            };
        }
    }

    fn execute_component<'a>(&mut self, instructions_iter: &mut impl Iterator<Item = &'a u64>) -> Result<Vec<u64>, ()> {
        let component_code = instructions_iter.next().expect("Expected component");
        let number_of_component_parameters = Spell::get_number_of_component_parameters(component_code);
        let mut parameters: Vec<u64> = vec![];
        for _ in 0..number_of_component_parameters {
            let parameter = *instructions_iter.next().expect("Expected parameter");
            match parameter {
                100..=101 => parameters.push(parameter),
                102 => {
                    parameters.push(parameter);
                    parameters.push(*instructions_iter.next().expect("Expected number after number literal opcode"));
                },
                103 => parameters.extend(self.execute_component(instructions_iter)?),
                _ => panic!("Invalid parameter")
            }
        }

        return self.call_component(component_code, parameters)?.ok_or(())
    }

    fn free_spell(&mut self) {
        self.base_mut().queue_free();
    }

    fn call_component(&mut self, component_code: &u64, parameters: Vec<u64>) -> Result<Option<Vec<u64>>, ()> {

        // Removes number literal opcodes
        let mut compressed_parameters: Vec<u64> = Vec::new();
        let mut parameter_iter = parameters.iter();
        while let Some(parameter) = parameter_iter.next() {
            match parameter {
                102 => compressed_parameters.push(*parameter_iter.next().expect("Expected parameter after number literal opcode")),
                100..=101 => compressed_parameters.push(*parameter),
                _ => panic!("Invalid parameter: isn't float or bool")
            }
        }

        // Getting component cast count
        if let Some((function, _, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            let mut component_efficiency_level = self.component_efficiency_levels.entry(*component_code).or_insert(1.0).clone();

            // Getting energy required
            if let Some(base_energy_bits) = function(self, &compressed_parameters, false) {
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

                    if let Some(value) = function(self, &compressed_parameters, true) {
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

    fn energy_to_radius(energy: f64) -> f32 {
        (energy / ENERGY_TO_RADIUS_CONSTANT) as f32
    }

    fn get_number_of_component_parameters(component_code: &u64) -> usize {
        if let Some((_, number_of_parameters, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            return number_of_parameters.len()
        } else {
            panic!("Component doesn't exist")
        }
    }

    fn set_form(&mut self, form_code: u64) {
        let form_config = self.config.forms.get(&form_code).expect("Expected form code to map to a form");

        let mut scene: Gd<PackedScene> = load(&form_config.path);

        self.form_set = true;
        scene.set_name("form".into_godot());
        let mut csg_sphere: Gd<CsgSphere3D> = self.base_mut().get_node_as("spell_csg_sphere".into_godot());
        csg_sphere.set_visible(false);
        self.base_mut().add_child(scene.instantiate().expect("Expected to be able to create scene"));
    }

    fn undo_form(&mut self) { // TODO: Fix
        if self.form_set == false {
            return
        }
        self.form_set = false;
        let mut form: Gd<Node> = self.base_mut().get_node_as("form".into_godot());
        form.queue_free();
        let mut csg_sphere: Gd<CsgSphere3D> = self.base_mut().get_node_as("spell_csg_sphere".into_godot());
        csg_sphere.set_visible(true);
    }

    // TODO: Finish
    fn check_if_parameter_allowed(parameter: u64, allowed_arrary: Array<Variant>) -> Result<(), &'static str> {
        return Ok(())
    }

    // TODO: Finish
    fn internal_check_allowed_to_cast(&self, instructions_json: GString) -> Result<(), &'static str> {
        let instructions_string = instructions_json.to_string();
        let instructions: Vec<u64> = serde_json::from_str(&instructions_string).expect("Couldn't parse json instructions");
        let spell_catalogue = load_spell_catalogue();

        let mut instructions_iter = instructions.iter();
        while let Some(&bits) = instructions_iter.next() {
            match bits {
                102 => _ = instructions_iter.next(),
                103 => {
                    let component_code = instructions_iter.next().expect("Expected component code"); // Get component num to work out how many parameters to skip
                    let number_of_component_parameters = Spell::get_number_of_component_parameters(component_code);
                    let allowed_parameters_list: Array<Array<Variant>> = spell_catalogue.get(component_code.to_godot()).ok_or("Component isn't in spell catalogue")?.to();
                    for index in 0..number_of_component_parameters {
                        Spell::check_if_parameter_allowed(*instructions_iter.next().expect("Expected parameter"), allowed_parameters_list.at(index))?;
                    }
                },
                _ => {}
            }
        }
        return Ok(())
    }
}

#[godot_api]
impl Spell {
    // TODO: Finish
    #[func]
    fn check_allowed_to_cast(&self, instructions_json: GString) -> Dictionary {
        dict! {}
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

    /// Takes instructions in the format of a json list which can be obtained from the output of the method `get_bytecode_instructions`. The instructions are called once the spell is put in the scene tree
    #[func]
    fn set_instructions(&mut self, instructions_json: GString) {
        let instructions_string = instructions_json.to_string();
        let instructions: Vec<u64> = serde_json::from_str(&instructions_string).expect("Couldn't parse json instructions");
        let mut section_instructions: Vec<u64> = vec![];
        let mut last_section: u64 = 0;
        let mut instructions_iter = instructions.iter();
        while let Some(&instruction) = instructions_iter.next() {
            match instruction {
                102 => { // Number literal
                    section_instructions.push(instruction);
                    section_instructions.push(*instructions_iter.next().expect("Expected number after literal opcode"))
                }
                103 => { // Component
                    section_instructions.push(instruction);
                    let component_code = instructions_iter.next().expect("Expected component code"); // Get component num to work out how many parameters to skip
                    section_instructions.push(*component_code);
                    let number_of_component_parameters = Spell::get_number_of_component_parameters(component_code);
                    for _ in 0..number_of_component_parameters {
                        section_instructions.push(*instructions_iter.next().expect("Expected parameters for component"));
                    }
                },
                500..=501 => { // Section opcodes
                    match last_section {
                        0 => {},
                        500 => {self.ready_instructions = section_instructions.clone()},
                        501 => self.process_instructions = section_instructions.clone(),
                        _ => panic!("Invalid section")
                    }

                    section_instructions.clear();
                    last_section = instruction;
                },
                _ => section_instructions.push(instruction)
            }
        }

        // match the end section
        match last_section {
            0 => {},
            500 => self.ready_instructions = section_instructions.clone(),
            501 => self.process_instructions = section_instructions.clone(),
            _ => panic!("Invalid section")
        }
    }

    /// Takes in spell instructions in string format and returns a dictionary containing `instructions` (a json list), `successful` (a boolean) and `error_message` (a string)
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

    /// The parameter `energy_lose_rate` is a fraction of the total energy of the spell, not a constant amount and should range between 0 and 1
    #[func]
    fn set_energy_lose_rate(&mut self, energy_lose_rate: f64) {
        self.energy_lose_rate = energy_lose_rate;
    }

    #[func]
    fn get_energy_lose_rate(&self) -> f64 {
        self.energy_lose_rate
    }

    /// Requires Color(r, g, b) where r, g and b are floats ranging from 0 to 1
    #[func]
    fn set_color(&mut self, color: Color) {
        self.color = Color::from_rgba(color.r, color.g, color.b, SPELL_TRANSPARENCY);
    }

    #[func]
    fn get_color(&self) -> Color {
        Color::from_rgb(self.color.r as f32, self.color.g as f32, self.color.b as f32)
    }

    /// Once `connect_player()` is called, whenever a component is cast, the provided node's `update_component_efficiency` method will be called
    #[func]
    fn connect_player(&mut self, player: Gd<Node>) {
        let update_function = player.callable("update_component_efficiency");
        self.base_mut().connect("component_cast".into(), update_function);
    }

    #[signal]
    fn component_cast(component_code: u64, efficiency_increase: f64);
}

mod boolean_logic {
    pub fn and(first: u64, second: u64) -> u64 {
        // 100 = true, 101 = false
        match (first, second) {
            (100, 100) => 100,
            (100, 101) => 101,
            (101, 100) => 101,
            (101, 101) => 101,
            _ => panic!("Parameters must be 100 or 101")
        }
    }

    pub fn or(first: u64, second: u64) -> u64 {
        // 100 = true, 101 = false
        match (first, second) {
            (100, 100) => 100,
            (100, 101) => 100,
            (101, 100) => 100,
            (101, 101) => 101,
            _ => panic!("Parameters must be 100 or 101")
        }
    }

    pub fn xor(first: u64, second: u64) -> u64 {
        // 100 = true, 101 = false
        match (first, second) {
            (100, 100) => 101,
            (100, 101) => 100,
            (101, 100) => 100,
            (101, 101) => 101,
            _ => panic!("Parameters must be 100 or 101")
        }
    }

    pub fn not(first: u64) -> u64 {
        // 100 = true, 101 = false
        match first {
            100 => 101,
            101 => 100,
            _ => panic!("Parameters must be 100 or 101")
        }
    }
}
