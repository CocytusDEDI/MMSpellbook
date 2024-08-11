use lazy_static::lazy_static;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::PI;

// Godot imports
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

mod spelltranslator;
mod component_functions;
mod magical_entity;
mod saver;

use saver::*;
use spelltranslator::*;

// When a spell has energy below this level it is discarded as being insignificant
pub const ENERGY_CONSIDERATION_LEVEL: f64 = 1.0;

// Used to control how fast efficiency increases with each cast
const EFFICIENCY_INCREASE_RATE: f64 = 15.0;

// Used to control how fast energy is lost passively over time. Is a fraction of total spell energy.
const ENERGY_LOSE_RATE: f64 = 0.05;

// Used to determin how Transparent the default spell is. 0 = fully transparent, 1 = opaque
const SPELL_TRANSPARENCY: f32 = 0.9;

const RADIUS_UPDATE_RATE: usize = 7;

const DEFAULT_DENSITY: f64 = 1.0;
const DEFAULT_DENSITY_RANGE: f64 = 0.5;

#[derive(Serialize, Deserialize)]
struct CustomColor {
    r: f32,
    g: f32,
    b: f32
}

impl CustomColor {
    pub fn into_spell_color(self) -> Color {
        Color { r: self.r, g: self.g, b: self.b, a: SPELL_TRANSPARENCY }
    }
}

const DEFAULT_COLOR: CustomColor = CustomColor { r: 1.0, g: 1.0, b: 1.0 };

#[derive(Deserialize, Serialize, Clone)]
pub struct ComponentCatalogue {
    pub component_catalogue: HashMap<u64, Vec<Vec<u64>>>
}

impl ComponentCatalogue {
    fn new() -> Self {
        ComponentCatalogue { component_catalogue: HashMap::new() }
    }
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

struct Process {
    counter: usize,
    frequency: usize,
    instructions: Vec<u64>
}

impl Process {
    fn new(frequency: usize, instructions: Vec<u64>) -> Self {
        Process { counter: 0, frequency, instructions}
    }

    fn increment(&mut self) {
        self.counter = (self.counter + 1) % self.frequency
    }

    fn should_run(&self) -> bool {
        self.counter == 0
    }
}

#[derive(GodotClass)]
#[class(base=Area3D)]
struct Spell {
    base: Base<Area3D>,
    energy: f64,
    color: Color,
    counter: usize,
    density: f64,
    density_range: f64, // TODO: Allow players to set their own density within their density range
    energy_lose_rate: f64,
    form_set: bool,
    config: Config,
    velocity: Vector3,
    time: Option<Gd<Time>>,
    start_time: Option<u64>,
    component_catalogue: ComponentCatalogue,
    check_component_return_value: bool,
    ready_instructions: Vec<u64>,
    process_instructions: Vec<Process>,
    component_efficiency_levels: HashMap<u64, f64>
}

#[godot_api]
impl IArea3D for Spell {
    fn init(base: Base<Area3D>) -> Self {
        Self {
            base,
            energy: 0.0,
            color: DEFAULT_COLOR.into_spell_color(),
            counter: 0,
            density: DEFAULT_DENSITY,
            density_range: DEFAULT_DENSITY_RANGE,
            energy_lose_rate: ENERGY_LOSE_RATE,
            form_set: false,
            config: Config::get_config().unwrap_or_else(|error| {
                godot_warn!("{}", error);
                Config::default()
            }),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            time: None,
            start_time: None,
            component_catalogue: ComponentCatalogue::new(),
            check_component_return_value: true,
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

        if self.energy <= 0.0 {
            self.free_spell();
        }

        // Creating visual representation of spell in godot
        let mut collision_shape = CollisionShape3D::new_alloc();
        collision_shape.set_name("spell_collision_shape".into_godot());
        let mut shape = SphereShape3D::new_gd();
        shape.set_name("spell_sphere_shape".into_godot());
        let radius = self.get_radius();
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

        let spell_result = {
            let instructions = std::mem::take(&mut self.ready_instructions);
            let result = self.spell_virtual_machine(&instructions);
            self.ready_instructions = instructions;
            result
        };

        // Handle instructions, throws error if it doesn't have enough energy to cast a component
        match spell_result {
            Ok(()) => {},
            Err(_) => self.free_spell()
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

        {
        let mut instructions = std::mem::take(&mut self.process_instructions);

        for process in instructions.iter_mut() {
            // Handle instructions, frees the spell if it fails

            process.increment();

            if !process.should_run() { continue };

            match self.spell_virtual_machine(&process.instructions) {
                Ok(()) => {},
                Err(_) => self.free_spell()
            }

            // Check if spell should be deleted due to lack of energy
            if self.energy < ENERGY_CONSIDERATION_LEVEL {
                self.free_spell();
            }
        }

        self.process_instructions = instructions;
        }

        // Handle energy lose
        self.energy = self.energy - self.energy * self.energy_lose_rate * delta;

        if !self.form_set && self.counter == 0 {
            // Radius changing of collision shape
            let radius = self.get_radius();

            let collsion_shape = self.base_mut().get_node_as::<CollisionShape3D>("spell_collision_shape");
            let shape = collsion_shape.get_shape().unwrap();
            let mut sphere = shape.cast::<SphereShape3D>();
            sphere.set_radius(radius);

            // Changing radius of csg sphere
            let mut csg_sphere = self.base_mut().get_node_as::<CsgSphere3D>("spell_csg_sphere");
            csg_sphere.set_radius(radius);
        }

        self.counter = (self.counter + 1) % RADIUS_UPDATE_RATE;

        // Check if spell should be deleted due to lack of energy
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.free_spell();
        }
    }
}


impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u64]) -> Result<(), &'static str> {
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
                            102 => rpn_stack.extend(vec![102, *instructions_iter.next().expect("Expected following value")]), // if 102, next bits are a number literal
                            103 => { // Component
                                rpn_stack.extend(self.execute_component(&mut instructions_iter)?)
                            }
                            200 => { // And statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::and(bool_one, bool_two).unwrap_or_else(|err| panic!("{}", err)));
                            },
                            201 => { // Or statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::or(bool_one, bool_two).unwrap_or_else(|err| panic!("{}", err)));
                            },
                            202 => { // Not statement
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::not(bool_one).unwrap_or_else(|err| panic!("{}", err)));
                            },
                            203 => { // Xor statement
                                let bool_two = rpn_stack.pop().expect("Expected value to compair");
                                let bool_one = rpn_stack.pop().expect("Expected value to compair");
                                rpn_stack.push(boolean_logic::xor(bool_one, bool_two).unwrap_or_else(|err| panic!("{}", err)));
                            },
                            300 => { // equals
                                let argument_two = rpn_stack.pop().expect("Expected value to compair");
                                let opcode_or_bool = rpn_stack.pop().expect("Expected value to compair");
                                if opcode_or_bool == 102 {
                                    let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                    let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                    if argument_one == f64::from_bits(argument_two) {
                                        rpn_stack.push(100);
                                    } else {
                                        rpn_stack.push(101);
                                    }
                                } else {
                                    if opcode_or_bool == argument_two {
                                        rpn_stack.push(100);
                                    } else {
                                        rpn_stack.push(101);
                                    }
                                }
                            },
                            301 => { // greater than
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                if argument_one > argument_two {
                                    rpn_stack.push(100);
                                } else {
                                    rpn_stack.push(101);
                                }
                            },
                            302 => { // lesser than
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                if argument_one < argument_two {
                                    rpn_stack.push(100);
                                } else {
                                    rpn_stack.push(101);
                                }
                            },
                            600 => { // multiply
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                rpn_stack.extend(vec![102, f64::to_bits(argument_one * argument_two)]);
                            }
                            601 => { // divide
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                rpn_stack.extend(vec![102, f64::to_bits(argument_one / argument_two)]);
                            }
                            602 => { // add
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                rpn_stack.extend(vec![102, f64::to_bits(argument_one + argument_two)]);
                            }
                            603 => { // subtract
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                rpn_stack.extend(vec![102, f64::to_bits(argument_one - argument_two)]);
                            }
                            604 => { // power
                                let argument_two = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compair"));
                                let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                rpn_stack.extend(vec![102, f64::to_bits(argument_one.powf(argument_two))]);
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

    fn execute_component<'a>(&mut self, instructions_iter: &mut impl Iterator<Item = &'a u64>) -> Result<Vec<u64>, &'static str> {
        let component_code = instructions_iter.next().expect("Expected component");
        let number_of_component_parameters = Spell::get_number_of_component_parameters(component_code);
        let mut parameters: Vec<u64> = Vec::new();
        for parameter_number in 0..number_of_component_parameters {
            let parameter = *instructions_iter.next().expect("Expected parameter");
            match parameter {
                100..=101 => parameters.push(parameter),
                102 => {
                    parameters.push(parameter);
                    parameters.push(*instructions_iter.next().expect("Expected number after number literal opcode"));
                },
                103 => {
                    let component_return = self.execute_component(instructions_iter)?;
                    // Checks if component return is an allowed parameter as it can't be known at compile time
                    if self.check_component_return_value {
                        let allowed_parameters_list: &Vec<Vec<u64>> = self.component_catalogue.component_catalogue.get(&component_code.to_godot()).ok_or("Component isn't in component catalogue")?;
                        Spell::check_if_parameter_allowed(&component_return, &allowed_parameters_list[parameter_number])?;
                    }
                    parameters.extend(component_return);
                },
                _ => panic!("Invalid parameter")
            }
        }

        return self.call_component(component_code, parameters)
    }

    fn free_spell(&mut self) {
        self.base_mut().queue_free();
    }

    fn call_component(&mut self, component_code: &u64, parameters: Vec<u64>) -> Result<Vec<u64>, &'static str> {

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
                        return Ok(value)
                    } else {
                        return Ok(Vec::new())
                    }
                } else {
                    return Err("Not enough energy")
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

    fn get_radius(&self) -> f32 {
        ((3.0 * self.get_volume()) / (4.0 * PI)).powf(1.0 / 3.0)
    }

    fn get_volume(&self) -> f32 {
        (self.energy / self.density) as f32
    }

    fn get_number_of_component_parameters(component_code: &u64) -> usize {
        if let Some((_, number_of_parameters, _)) = COMPONENT_TO_FUNCTION_MAP.get(&component_code) {
            return number_of_parameters.len()
        } else {
            panic!("Component doesn't exist")
        }
    }

    fn set_form(&mut self, form_code: u64) {
        if self.form_set {
            self.undo_form();
        }
        let form_config = self.config.forms.get(&form_code).expect("Expected form code to map to a form");

        let scene: Gd<PackedScene> = load(&form_config.path);

        self.form_set = true;
        let mut csg_sphere: Gd<CsgSphere3D> = self.base_mut().get_node_as("spell_csg_sphere".into_godot());
        csg_sphere.set_visible(false);
        let mut instantiated_scene = scene.instantiate().expect("Expected to be able to create scene");
        instantiated_scene.set_name("form".into_godot());
        self.base_mut().add_child(instantiated_scene);
    }

    fn undo_form(&mut self) {
        if self.form_set == false {
            return
        }
        self.form_set = false;
        let form: Gd<Node> = self.base_mut().get_node_as("form".into_godot());
        form.free();
        let mut csg_sphere: Gd<CsgSphere3D> = self.base_mut().get_node_as("spell_csg_sphere".into_godot());
        csg_sphere.set_visible(true);
    }

    fn check_if_parameter_allowed(parameter: &Vec<u64>, allowed_values: &Vec<u64>) -> Result<(), &'static str> {
        let mut allowed_iter = allowed_values.iter();
        match parameter[0] {
            100 => {
                while let Some(&value) = allowed_iter.next() {
                    if value == 100 || value == 104 {
                        return Ok(())
                    }
                }
            },
            101 => {
                while let Some(&value) = allowed_iter.next() {
                    if value == 101 || value == 104 {
                        return Ok(())
                    }
                }
            },
            102 => {
                while let Some(&value) = allowed_iter.next() {
                    if value == 104 {
                        return Ok(())
                    }
                    let start_float_range = match value {
                        102 => f64::from_bits(*allowed_iter.next().expect("Expected value after number literal")),
                        _ => return Err("Invalid type: Expected float")
                    };
                    let stop_float_range = match allowed_iter.next().expect("Expected range of numbers") {
                        102 => f64::from_bits(*allowed_iter.next().expect("Expected value after number literal")),
                        _ => return Err("Invalid type: Expected float")
                    };
                    let range = start_float_range..=stop_float_range;
                    if range.contains(&f64::from_bits(parameter[1])) {
                        return Ok(())
                    }
                }
            },
            _ => return Err("Invalid parameter type")
        };
        return Err("Parameter not allowed")
    }

    /// Checks if the magical entity has access to the component and can cast it with the given parameters. Doesn't check the return of components that are parameters.
    fn check_allowed_to_cast_component<'a>(instructions_iter: &mut impl Iterator<Item = &'a u64>, component_catalogue: &ComponentCatalogue) -> Result<(), &'static str> {
        let component_code = *instructions_iter.next().expect("Expected component code"); // Get component num to work out how many parameters to skip
        let number_of_component_parameters = Spell::get_number_of_component_parameters(&component_code);
        let allowed_parameters_list: &Vec<Vec<u64>> = component_catalogue.component_catalogue.get(&component_code.to_godot()).ok_or("Component isn't in component catalogue")?;

        for index in 0..number_of_component_parameters {
            let parameter = match *instructions_iter.next().expect("Expected parameter") {
                100 => vec![100],
                101 => vec![101],
                102 => vec![102, *instructions_iter.next().expect("Expected parameter")],
                103 => {
                    _ = instructions_iter.next();
                    continue
                },
                _ => panic!("Invalid parameter")
            };
            Spell::check_if_parameter_allowed(&parameter, &allowed_parameters_list[index])?;
        }
        return Ok(())
    }

    fn internal_check_allowed_to_cast(instructions: Vec<u64>, component_catalogue: &ComponentCatalogue) -> Result<(), &'static str> {
        let mut instructions_iter = instructions.iter();
        while let Some(&bits) = instructions_iter.next() {
            match bits {
                102 => _ = instructions_iter.next(),
                103 => _ = Spell::check_allowed_to_cast_component(&mut instructions_iter, &component_catalogue)?,
                _ => {}
            }
        }
        return Ok(())
    }

    fn add_component_to_component_catalogue(component_code: u64, parameter_restrictions: Vec<Vec<&str>>, component_catalogue: &mut ComponentCatalogue) {
        let mut parsed_parameter_restrictions: Vec<Vec<u64>> = Vec::new();
        let mut index = 0;
        for parameter_allowed_values in parameter_restrictions {
            parsed_parameter_restrictions.push(Vec::new());
            for allowed_value in parameter_allowed_values {
                match allowed_value {
                    "ANY" => {
                        parsed_parameter_restrictions[index].push(104);
                        break;
                    },
                    "true" => parsed_parameter_restrictions[index].push(100),
                    "false" => parsed_parameter_restrictions[index].push(101),
                    something => {
                        if let Ok(number) = something.parse::<f64>() {
                            parsed_parameter_restrictions[index].extend(vec![102, f64::to_bits(number), 102, f64::to_bits(number)]);
                        } else if something.contains('-') {
                            let numbers: Vec<&str> = something.split('-').collect();
                            if let (Ok(start_range), Ok(stop_range)) = (numbers[0].trim().parse::<f64>(), numbers[1].trim().parse::<f64>()) {
                                parsed_parameter_restrictions[index].extend(vec![102, f64::to_bits(start_range), 102, f64::to_bits(stop_range)]);
                            } else {
                                panic!("Couldn't parse range")
                            }
                        }
                    }
                }
            }
            index += 1;
        }

        component_catalogue.component_catalogue.insert(component_code, parsed_parameter_restrictions);
    }

    /// Gives a spell instance its instructions, used to avoid json translation
    fn internal_set_instructions(&mut self, instructions: Vec<u64>) {
        let mut section_instructions: Vec<u64> = Vec::new();
        let mut last_section: u64 = 0;
        let mut instructions_iter = instructions.iter();
        while let Some(&instruction) = instructions_iter.next() {
            match instruction {
                102 => { // Number literal
                    section_instructions.push(instruction);
                    let something = *instructions_iter.next().expect("Expected number after literal opcode");
                    section_instructions.push(something);
                },
                500..=501 => { // Section opcodes
                    match last_section {
                        0 => {},
                        500 => self.ready_instructions = section_instructions.clone(),
                        501 => {
                            section_instructions.remove(0);
                            self.process_instructions.push(Process::new(f64::from_bits(section_instructions.remove(0)) as usize, section_instructions.clone()))
                        },
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
            501 => {
                section_instructions.remove(0);
                self.process_instructions.push(Process::new(f64::from_bits(section_instructions.remove(0)) as usize, section_instructions.clone()))
            },
            _ => panic!("Invalid section")
        }
    }

    fn translate_instructions(instructions_json: &GString) -> Vec<u64> {
        let instructions_string = instructions_json.to_string();
        serde_json::from_str(&instructions_string).expect("Couldn't parse json instructions")
    }

    fn internal_set_efficiency_levels(&mut self, efficiency_levels: HashMap<u64, f64>) {
        self.component_efficiency_levels = efficiency_levels;
    }
}

#[godot_api]
impl Spell {
    /// Checks instructions against the component catalogue to see if the player is allowed to cast all components in the spell and with the parameters entered.
    #[func]
    fn check_allowed_to_cast(instructions_json: GString, component_catalogue_path: GString) -> Dictionary {
        let component_catalogue: ComponentCatalogue = godot_json_saver::from_path(&component_catalogue_path.to_string()).unwrap();
        let (allowed_to_cast, denial_reason) = match Spell::internal_check_allowed_to_cast(Spell::translate_instructions(&instructions_json), &component_catalogue) {
            Ok(_) => (true, ""),
            Err(error_message) => (false, error_message)
        };
        return dict! {"allowed_to_cast": allowed_to_cast, "denial_reason": denial_reason}
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
        self.internal_set_instructions(Spell::translate_instructions(&instructions_json));
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
    fn set_check_component_return_value(&mut self, boolean: bool) {
        self.check_component_return_value = boolean;
    }

    #[func]
    fn get_check_component_return_value(&self) -> bool {
        self.check_component_return_value
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
    pub fn and(first: u64, second: u64) -> Result<u64, &'static str> {
        // 100 = true, 101 = false
        match (first, second) {
            (100, 100) => Ok(100),
            (100, 101) => Ok(101),
            (101, 100) => Ok(101),
            (101, 101) => Ok(101),
            _ => Err("Boolean logic can only compair booleans")
        }
    }

    pub fn or(first: u64, second: u64) -> Result<u64, &'static str> {
        // 100 = true, 101 = false
        match (first, second) {
            (100, 100) => Ok(100),
            (100, 101) => Ok(100),
            (101, 100) => Ok(100),
            (101, 101) => Ok(101),
            _ => Err("Boolean logic can only compair booleans")
        }
    }

    pub fn xor(first: u64, second: u64) -> Result<u64, &'static str> {
        // 100 = true, 101 = false
        match (first, second) {
            (100, 100) => Ok(101),
            (100, 101) => Ok(100),
            (101, 100) => Ok(100),
            (101, 101) => Ok(101),
            _ => Err("Boolean logic can only compair booleans")
        }
    }

    pub fn not(first: u64) -> Result<u64, &'static str> {
        // 100 = true, 101 = false
        match first {
            100 => Ok(101),
            101 => Ok(100),
            _ => Err("Not can only be used on booleans")
        }
    }
}
