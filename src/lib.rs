use godot::classes::BoxShape3D;
use godot::classes::CsgBox3D;
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
use godot::builtin::Basis;

mod spelltranslator;
mod component_functions;
mod magical_entity;
mod saver;
mod codes;

use saver::*;
use spelltranslator::*;
use magical_entity::MagicalEntity;
use codes::componentcodes::*;
use codes::attributecodes::*;
use codes::opcodes::*;
use codes::shapecodes::*;

// When a spell has energy below this level it is discarded as being insignificant
pub const ENERGY_CONSIDERATION_LEVEL: f64 = 0.1;

// Used to control how fast efficiency increases with each cast
const EFFICIENCY_INCREASE_RATE: f64 = 15.0;

// Used to control how fast energy is lost passively over time. Is a fraction of total spell energy.
const ENERGY_LOSE_RATE: f64 = 0.05;

const MASS_MOVEMENT_COST: f64 = 0.5;

// Used to determin how Transparent the default spell is. 0 = fully transparent, 1 = opaque
const SPELL_TRANSPARENCY: f32 = 0.9;

const RADIUS_UPDATE_RATE: usize = 5;

const ENERGY_TO_VOLUME: f32 = 0.01;

const VOLUME_TO_RADIUS: f32 = 0.3;

const CSG_SPHERE_DETAIL: (i32, i32) = (18, 20); // In the format (rings, radial segments)

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

const COMPONENT_0_ARGS: &[u64] = &[1, 1, 1];
const COMPONENT_1_ARGS: &[u64] = &[1];
const COMPONENT_2_ARGS: &[u64] = &[];

lazy_static! {
    // Component_bytecode -> (function, parameter types represented by u64, return type of the function for if statements)
    // The u64 type conversion goes as follows: 1 = f64, 2 = bool
    static ref COMPONENT_TO_FUNCTION_MAP: HashMap<u64, (fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, &'static[u64], ReturnType)> = {
        let mut component_map = HashMap::new();
        // Utility:
        component_map.insert(GIVE_VELOCITY, (component_functions::give_velocity as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_0_ARGS, ReturnType::None));
        component_map.insert(TAKE_FORM, (component_functions::take_form as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_1_ARGS, ReturnType::None));
        component_map.insert(UNDO_FORM, (component_functions::undo_form as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_2_ARGS, ReturnType::None));
        component_map.insert(RECHARGE_TO, (component_functions::recharge_to as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_1_ARGS, ReturnType::None));
        component_map.insert(ANCHOR, (component_functions::anchor as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_2_ARGS, ReturnType::None));
        component_map.insert(UNDO_ANCHOR, (component_functions::undo_anchor as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_2_ARGS, ReturnType::None));
        component_map.insert(PERISH, (component_functions::perish as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_2_ARGS, ReturnType::None));

        // Logic:
        component_map.insert(MOVING, (component_functions::moving as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_1_ARGS, ReturnType::Boolean));
        component_map.insert(GET_TIME, (component_functions::get_time as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_2_ARGS, ReturnType::Float));

        // Power:
        component_map.insert(SET_DAMAGE, (component_functions::set_damage as fn(&mut Spell, &[u64], bool) -> Option<Vec<u64>>, COMPONENT_1_ARGS, ReturnType::None));

        return component_map
    };
}

/// A process is a set of instructions used in the method `physics_process`. A process keeps track of when it should run using a counter.
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

#[derive(Clone, Copy)]
enum Shape {
    Sphere,
    Box
}

const SPELL_COLLISION_SHAPE_NAME: &'static str = "spell_collision_shape";
const SPELL_SHAPE_NAME: &'static str = "spell_shape";
const SPELL_CSG_SHAPE_NAME: &'static str = "spell_csg_shape";
const FORM_NAME: &'static str = "form";

impl Shape {
    fn set_as_shape(&self, spell: &mut Spell) {
        // Collision shape
        let mut collision_shape = CollisionShape3D::new_alloc();
        collision_shape.set_name(SPELL_COLLISION_SHAPE_NAME.into_godot());

        // Material
        let mut csg_material = StandardMaterial3D::new_gd();

        // Player defined material properties
        csg_material.set_albedo(spell.color);

        // Constant material properties
        csg_material.set_transparency(Transparency::ALPHA); // Transparency type
        csg_material.set_feature(Feature::EMISSION, true); // Allows spell to emit light
        csg_material.set_emission(spell.color); // Chooses what light to emit

        match self {
            Self::Sphere => {
                // Creating sphere shape
                let mut shape = SphereShape3D::new_gd();
                shape.set_name(SPELL_SHAPE_NAME.into_godot());
                let radius = self.get_radius(spell.energy);
                shape.set_radius(radius);
                collision_shape.set_shape(shape.upcast::<Shape3D>());

                // Creating visual representation of spell in godot
                let mut csg_sphere = CsgSphere3D::new_alloc();
                csg_sphere.set_name(SPELL_CSG_SHAPE_NAME.into_godot());
                csg_sphere.set_rings(CSG_SPHERE_DETAIL.0);
                csg_sphere.set_radial_segments(CSG_SPHERE_DETAIL.1);
                csg_sphere.set_radius(radius);
                csg_sphere.set_material(csg_material);
                spell.base_mut().add_child(csg_sphere.upcast::<Node>());
            },
            Self::Box => {
                // Creating box shape
                let mut shape = BoxShape3D::new_gd();
                shape.set_name(SPELL_SHAPE_NAME.into_godot());
                let side = self.get_side(spell.energy);
                let size = Vector3 { x: side, y: side, z: side };
                shape.set_size(size);
                collision_shape.set_shape(shape.upcast::<Shape3D>());

                // Creating visual representation of spell in godot
                let mut csg_box = CsgBox3D::new_alloc();
                csg_box.set_name(SPELL_CSG_SHAPE_NAME.into_godot());
                csg_box.set_size(size);
                csg_box.set_material(csg_material);
                spell.base_mut().add_child(csg_box.upcast::<Node>());
            }
        };

        spell.base_mut().add_child(collision_shape.upcast::<Node>());
    }

    fn update_size(&self, spell: &mut Spell) {
        let collsion_shape = spell.base_mut().get_node_as::<CollisionShape3D>(SPELL_COLLISION_SHAPE_NAME);
        let shape = collsion_shape.get_shape().unwrap();

        match self {
            Self::Sphere => {
                // Radius changing of collision shape
                let radius = self.get_radius(spell.energy);

                let mut sphere_shape = shape.cast::<SphereShape3D>();
                sphere_shape.set_radius(radius);

                // Changing radius of csg sphere
                let mut csg_sphere = spell.base_mut().get_node_as::<CsgSphere3D>(SPELL_CSG_SHAPE_NAME);
                csg_sphere.set_radius(radius);
            },
            Self::Box => {
                let side = self.get_side(spell.energy);
                let size = Vector3 { x: side, y: side, z: side };

                let mut box_shape = shape.cast::<BoxShape3D>();
                box_shape.set_size(size);

                let mut csg_box = spell.base_mut().get_node_as::<CsgBox3D>(SPELL_CSG_SHAPE_NAME);
                csg_box.set_size(size);
            }
        }
    }

    fn set_visibility(&self, spell: &mut Spell, visible: bool) {
        match self {
            Self::Sphere => {
                let mut csg_sphere: Gd<CsgSphere3D> = spell.base_mut().get_node_as(SPELL_CSG_SHAPE_NAME.into_godot());
                csg_sphere.set_visible(visible);
            },
            Self::Box => {
                let mut csg_box: Gd<CsgBox3D> = spell.base_mut().get_node_as(SPELL_CSG_SHAPE_NAME.into_godot());
                csg_box.set_visible(visible)
            }
        }
    }

    fn get_side(&self, energy: f64) -> f32 {
        self.get_volume(energy).powf(1.0/3.0)
    }

    fn get_radius(&self, energy: f64) -> f32 {
        ((3.0 * self.get_volume(energy)) / (4.0 * PI)).powf(1.0 / 3.0) * VOLUME_TO_RADIUS
    }

    fn get_volume(&self, energy: f64) -> f32 {
        f32::ln(ENERGY_TO_VOLUME * (energy as f32) + 1.0).powi(2)
    }

    fn from_num(number: u64) -> Self {
        match number {
            SPHERE => Self::Sphere,
            BOX => Self::Box,
            _ => panic!("Shape code doesn't exist")
        }
    }
}

#[derive(GodotClass)]
#[class(base=Area3D)]
struct Spell {
    base: Base<Area3D>,
    #[export]
    energy: f64,
    #[export]
    color: Color,
    shape: Shape,
    counter: usize,
    #[export]
    energy_lose_rate: f64,
    config: Config,
    component_catalogue: ComponentCatalogue,
    check_component_return_value: bool,
    ready_instructions: Vec<u64>,
    process_instructions: Vec<Process>,
    component_efficiency_levels: HashMap<u64, f64>,

    // Component fields
    damage: f64,
    energy_requested: f64,
    original_direction: Basis,
    velocity: Vector3,
    time: Option<Gd<Time>>,
    start_time: Option<u64>,
    form_set: bool,
    anchored_to: Option<Gd<MagicalEntity>>,
}

#[godot_api]
impl IArea3D for Spell {
    fn init(base: Base<Area3D>) -> Self {
        Self {
            base,
            energy: 0.0,
            color: DEFAULT_COLOR.into_spell_color(),
            shape: Shape::Sphere,
            counter: 0,
            energy_lose_rate: ENERGY_LOSE_RATE,
            config: Config::get_config().unwrap_or_else(|error| {
                godot_warn!("{}", error);
                Config::default()
            }),
            component_catalogue: ComponentCatalogue::new(),
            check_component_return_value: true,
            ready_instructions: Vec::new(),
            process_instructions: Vec::new(),
            component_efficiency_levels: HashMap::new(),

            // Component fields
            damage: 0.0,
            energy_requested: 0.0,
            original_direction: Basis::default(),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            time: None,
            start_time: None,
            form_set: false,
            anchored_to: None,
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
            self.perish();
        }

        // Get direction to move in
        self.original_direction = match self.base().get_parent() {
            Some(parent) => match parent.try_cast::<Node3D>() {
                Ok(node3d) => node3d.get_transform().basis,
                Err(_) => Basis::default()
            },
            None => Basis::default()
        };

        let shape = self.shape;
        shape.set_as_shape(self);

        // Execute the spell and get the result
        let spell_result = {
            let instructions = std::mem::take(&mut self.ready_instructions);
            let result = self.spell_virtual_machine(&instructions);
            self.ready_instructions = instructions;
            result
        };

        // Frees the spell if it ran out of energy to cast a component
        match spell_result {
            Ok(()) => {},
            Err(_) => self.perish()
        };

        // Check if spell should be deleted due to lack of energy
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.perish();
        }
    }

    fn physics_process(&mut self, delta: f64) {
        // Handle velocity
        if let Some(ref mut anchored_to) = self.anchored_to {
            let previous_velocity = anchored_to.get_velocity();
            let direction = (self.original_direction * self.velocity).normalized_or_zero();
            anchored_to.set_velocity(previous_velocity + direction * self.velocity.length());
            self.velocity = Vector3::ZERO;
        } else {
            let f32_delta: f32 = delta as f32;
            let previous_position = self.base_mut().get_global_position();
            let direction = (self.original_direction * self.velocity).normalized_or_zero();
            let new_position = previous_position + direction * self.velocity.length() * f32_delta;
            self.base_mut().set_global_position(new_position);
        }

        // Reduces energy due to anchor if there is one
        if !self.surmount_anchor_resistance() {
            self.perish();
            return
        }

        // handle instructions
        let mut instructions = std::mem::take(&mut self.process_instructions);
        for process in instructions.iter_mut() {
            // Handle instructions, frees the spell if it fails

            process.increment();

            if !process.should_run() { continue };

            match self.spell_virtual_machine(&process.instructions) {
                Ok(()) => {},
                Err(_) => self.perish()
            }

            // Check if spell should be deleted due to lack of energy
            if self.energy < ENERGY_CONSIDERATION_LEVEL {
                self.perish();
            }
        }
        self.process_instructions = instructions;

        // Deal damage
        if self.damage != 0.0 && self.anchored_to == None {
            let objects = self.base().get_overlapping_bodies();

            let mut number_of_magical_entities: usize = 0;

            for object in objects.iter_shared() {
                if let Ok(magical_entity_object) = object.try_cast::<MagicalEntity>() {
                    let bind_magical_entity = magical_entity_object.bind();
                    if !bind_magical_entity.owns_spell(self.to_gd()) {
                        number_of_magical_entities += 1;
                    }
                }
            }

            for object in objects.iter_shared() {
                if let Ok(mut magical_entity_object) = object.try_cast::<MagicalEntity>() {
                    let mut bind_magical_entity = magical_entity_object.bind_mut();
                    if !bind_magical_entity.owns_spell(self.to_gd()) {
                        // Damage is split among magical_entities
                        let damage = self.damage / number_of_magical_entities as f64;

                        // Code ensures energy used is at max the magic_entities health and that if it can't do damage specified it does as much of that damage as it can before destroying itself
                        let possible_damage = damage.min(bind_magical_entity.get_energy_to_kill());

                        if self.energy - possible_damage < ENERGY_CONSIDERATION_LEVEL {
                            bind_magical_entity.take_damage(self.energy);
                            self.perish();
                            return;
                        }

                        self.energy -= possible_damage;

                        bind_magical_entity.take_damage(possible_damage);
                    }
                }
            }
        }

        // Handle energy lose
        self.energy -= self.energy * self.energy_lose_rate * delta;

        // Decreases the radius of the sphere if form isn't set
        if !self.form_set && self.anchored_to == None && self.counter == 0 {
            let shape = self.shape;
            shape.update_size(self);
        }

        self.counter = (self.counter + 1) % RADIUS_UPDATE_RATE;

        // Check if spell should be deleted due to lack of energy
        if self.energy < ENERGY_CONSIDERATION_LEVEL {
            self.perish();
        }
    }
}


impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u64]) -> Result<(), &'static str> {
        let mut instructions_iter = instructions.iter();
        while let Some(&bits) = instructions_iter.next() {
            match bits {
                END_OF_SCOPE => {}, // 0 = end of scope, if reached naturely, move on
                COMPONENT => { // 103 = component
                    self.execute_component(&mut instructions_iter)?;
                },
                IF => { // 400 = if statement
                    let mut rpn_stack: Vec<u64> = Vec::new();
                    while let Some(&if_bits) = instructions_iter.next() {
                        match if_bits {
                            END_OF_SCOPE => break,
                            TRUE | FALSE => rpn_stack.push(if_bits), // true and false
                            NUMBER_LITERAL => rpn_stack.extend(vec![NUMBER_LITERAL, *instructions_iter.next().expect("Expected following value")]), // if 102, next bits are a number literal
                            COMPONENT => rpn_stack.extend(self.execute_component(&mut instructions_iter)?), // Component
                            AND => rpn_operations::binary_operation(&mut rpn_stack, boolean_logic::and).unwrap_or_else(|err| panic!("{}", err)), // And statement
                            OR => rpn_operations::binary_operation(&mut rpn_stack, boolean_logic::or).unwrap_or_else(|err| panic!("{}", err)), // Or statement
                            NOT => { // Not statement
                                let bool_one = rpn_stack.pop().expect("Expected value to compare");
                                rpn_stack.push(boolean_logic::not(bool_one).unwrap_or_else(|err| panic!("{}", err)));
                            },
                            XOR => rpn_operations::binary_operation(&mut rpn_stack, boolean_logic::xor).unwrap_or_else(|err| panic!("{}", err)), // Xor statement
                            EQUALS => { // Equals statement
                                let argument_two = rpn_stack.pop().expect("Expected value to compare");
                                let opcode_or_bool = rpn_stack.pop().expect("Expected value to compare");
                                if opcode_or_bool == NUMBER_LITERAL {
                                    let argument_one = f64::from_bits(rpn_stack.pop().expect("Expected value to compare"));
                                    let _ = rpn_stack.pop().expect("Expected number literal opcode");
                                    if argument_one == f64::from_bits(argument_two) {
                                        rpn_stack.push(TRUE);
                                    } else {
                                        rpn_stack.push(FALSE);
                                    }
                                } else {
                                    if opcode_or_bool == argument_two {
                                        rpn_stack.push(TRUE);
                                    } else {
                                        rpn_stack.push(FALSE);
                                    }
                                }
                            },
                            GREATER_THAN => rpn_operations::compare_operation(&mut rpn_stack, |a, b| a > b).unwrap_or_else(|err| panic!("{}", err)), // Greater than
                            LESSER_THAN => rpn_operations::compare_operation(&mut rpn_stack, |a, b| a < b).unwrap_or_else(|err| panic!("{}", err)), // Lesser than
                            MULTIPLY => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a * b).unwrap_or_else(|err| panic!("{}", err)), // Multiply
                            DIVIDE => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a / b).unwrap_or_else(|err| panic!("{}", err)), // Divide
                            ADD => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a + b).unwrap_or_else(|err| panic!("{}", err)), // Add
                            SUBTRACT => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a - b).unwrap_or_else(|err| panic!("{}", err)), // Subtract
                            POWER => rpn_operations::maths_operation(&mut rpn_stack, |a, b| a.powf(b)).unwrap_or_else(|err| panic!("{}", err)), // Power
                            _ => panic!("Opcode doesn't exist")
                        };
                    }
                    match rpn_stack.pop().expect("Expected final bool") {
                        TRUE => {}, // if true, execute by going back into normal loop
                        FALSE => { // if false, skip to the end of scope
                            let mut skip_amount: usize = 1;
                            while let Some(&skipping_bits) = instructions_iter.next() {
                                match skipping_bits {
                                    END_OF_SCOPE => skip_amount -= 1, // If end of scope
                                    NUMBER_LITERAL => _ = instructions_iter.next(), // Ignores number literals
                                    COMPONENT => {
                                        self.skip_component(&mut instructions_iter);
                                    }
                                    IF => skip_amount += 2, // Ignore next two end of scopes because if statements have two end of scopes
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
        Ok(())
    }

    fn skip_component<'a>(&mut self, instructions_iter: &mut impl Iterator<Item = &'a u64>) {
        let component_code = instructions_iter.next().expect("Expected component");
        let number_of_component_parameters = Spell::get_number_of_component_parameters(component_code);
        for _ in 0..number_of_component_parameters {
            let parameter = *instructions_iter.next().expect("Expected parameter");
            match parameter {
                TRUE | FALSE => {},
                NUMBER_LITERAL => _ = *instructions_iter.next().expect("Expected number after number literal opcode"),
                COMPONENT => _ = self.execute_component(instructions_iter),
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
                TRUE | FALSE => parameters.push(parameter),
                NUMBER_LITERAL => {
                    parameters.push(parameter);
                    parameters.push(*instructions_iter.next().expect("Expected number after number literal opcode"));
                },
                COMPONENT => {
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

    fn perish(&mut self) {
        self.base_mut().queue_free();
    }

    fn anchor(&mut self) {
        let parent = match self.base().get_parent() {
            Some(node) => node.cast::<MagicalEntity>(),
            None => panic!("Expected parent node")
        };
        let distance = (self.base().get_global_position() - parent.get_global_position()).length();
        let shape = self.shape;
        if shape.get_radius(self.energy) >= distance {
            self.base_mut().set_position(parent.get_global_position());
            self.anchored_to = Some(parent);
            self.base_mut().set_as_top_level(false);
            let shape = self.shape;
            shape.set_visibility(self, false);
        }
    }

    fn undo_anchor(&mut self) {
        self.base_mut().set_as_top_level(true);
        let position = self.base().get_global_position();
        match self.anchored_to {
            Some(ref mut magical_entity) => magical_entity.set_position(position),
            None => return
        }
        self.anchored_to = None;
        if !self.form_set {
            let shape = self.shape;
            shape.set_visibility(self, true);
        }
    }

    fn surmount_anchor_resistance(&mut self) -> bool {
        let mut spell_owned = false;

        let magical_entity_option = std::mem::take(&mut self.anchored_to);

        if let Some(ref magical_entity) = magical_entity_option {
            let bind_magical_entity = magical_entity.bind();
            spell_owned = bind_magical_entity.owns_spell(self.to_gd())
        }

        self.anchored_to = magical_entity_option;

        if let Some(ref mut magical_entity) = self.anchored_to {
            let mut bind_magical_entity = magical_entity.bind_mut();

            // Surmounting magical entity's charged energy
            if !spell_owned {
                let energy_charged = bind_magical_entity.get_energy_charged();
                if self.energy >= energy_charged {
                    bind_magical_entity.set_energy_charged(0.0);
                    self.energy -= energy_charged;
                } else {
                    bind_magical_entity.set_energy_charged(energy_charged - self.energy);
                    self.energy = 0.0;
                    return false
                }
            }

            // Surmounting magical entity's mass
            self.energy -= bind_magical_entity.get_mass() * MASS_MOVEMENT_COST;

            if !(self.energy > 0.0) {
                return false
            }
        }

        return true
    }

    fn call_component(&mut self, component_code: &u64, parameters: Vec<u64>) -> Result<Vec<u64>, &'static str> {
        // Removes number literal opcodes
        let mut compressed_parameters: Vec<u64> = Vec::new();
        let mut parameter_iter = parameters.iter();
        while let Some(&parameter) = parameter_iter.next() {
            match parameter {
                NUMBER_LITERAL => compressed_parameters.push(*parameter_iter.next().expect("Expected parameter after number literal opcode")),
                TRUE | FALSE => compressed_parameters.push(parameter),
                _ => panic!("Invalid parameter: isn't float or boolean")
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
        let shape = self.shape;
        shape.set_visibility(self, false);
        let mut instantiated_scene = scene.instantiate().expect("Expected to be able to create scene");
        instantiated_scene.set_name(FORM_NAME.into_godot());
        self.base_mut().add_child(instantiated_scene);
    }

    fn undo_form(&mut self) {
        if self.form_set == false {
            return
        }
        self.form_set = false;
        let form: Gd<Node> = self.base_mut().get_node_as(FORM_NAME.into_godot());
        form.free();
        if self.anchored_to == None {
            let shape = self.shape;
            shape.set_visibility(self, true);
        }
    }

    fn check_if_parameter_allowed(parameter: &Vec<u64>, allowed_values: &Vec<u64>) -> Result<(), &'static str> {
        let mut allowed_iter = allowed_values.iter();
        match parameter[0] {
            TRUE => {
                while let Some(&value) = allowed_iter.next() {
                    if value == TRUE || value == ANY {
                        return Ok(())
                    }
                }
            },
            FALSE => {
                while let Some(&value) = allowed_iter.next() {
                    if value == FALSE || value == ANY {
                        return Ok(())
                    }
                }
            },
            NUMBER_LITERAL => {
                while let Some(&value) = allowed_iter.next() {
                    if value == ANY {
                        return Ok(())
                    }
                    let start_float_range = match value {
                        NUMBER_LITERAL => f64::from_bits(*allowed_iter.next().expect("Expected value after number literal")),
                        _ => return Err("Invalid type: Expected float")
                    };
                    let stop_float_range = match *allowed_iter.next().expect("Expected range of numbers") {
                        NUMBER_LITERAL => f64::from_bits(*allowed_iter.next().expect("Expected value after number literal")),
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
                TRUE => vec![TRUE],
                FALSE => vec![FALSE],
                NUMBER_LITERAL => vec![NUMBER_LITERAL, *instructions_iter.next().expect("Expected parameter")],
                COMPONENT => {
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
        let mut section: Option<u64> = None;
        while let Some(&bits) = instructions_iter.next() {
            if section.is_some_and(|x| x == METADATA_SECTION) && !(READY_SECTION..=599).contains(&bits)  { // ignore all checks in metadata section
                continue;
            }
            match bits {
                NUMBER_LITERAL => _ = instructions_iter.next(),
                COMPONENT => _ = Spell::check_allowed_to_cast_component(&mut instructions_iter, &component_catalogue)?,
                READY_SECTION..=METADATA_SECTION => {
                    section = Some(bits)
                },
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
                        parsed_parameter_restrictions[index].push(ANY);
                        break;
                    },
                    "true" => parsed_parameter_restrictions[index].push(TRUE),
                    "false" => parsed_parameter_restrictions[index].push(FALSE),
                    something => {
                        if let Ok(number) = something.parse::<f64>() {
                            parsed_parameter_restrictions[index].extend(vec![NUMBER_LITERAL, f64::to_bits(number), NUMBER_LITERAL, f64::to_bits(number)]);
                        } else if something.contains('-') {
                            let numbers: Vec<&str> = something.split('-').collect();
                            if let (Ok(start_range), Ok(stop_range)) = (numbers[0].trim().parse::<f64>(), numbers[1].trim().parse::<f64>()) {
                                parsed_parameter_restrictions[index].extend(vec![NUMBER_LITERAL, f64::to_bits(start_range), NUMBER_LITERAL, f64::to_bits(stop_range)]);
                            } else {
                                panic!("Couldn't parse the range: {} to {}", numbers[0], numbers[1]);
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
                NUMBER_LITERAL => { // Number literal
                    section_instructions.push(instruction);
                    let something = *instructions_iter.next().expect("Expected number after literal opcode");
                    section_instructions.push(something);
                },
                READY_SECTION..=METADATA_SECTION => { // Section opcodes
                    match last_section {
                        END_OF_SCOPE => {},
                        READY_SECTION => self.ready_instructions = section_instructions.clone(),
                        PROCESS_SECTION => {
                            section_instructions.remove(0);
                            self.process_instructions.push(Process::new(f64::from_bits(section_instructions.remove(0)) as usize, section_instructions.clone()))
                        },
                        METADATA_SECTION => {
                            self.set_meta_data(section_instructions.clone())
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
            END_OF_SCOPE => {},
            READY_SECTION => self.ready_instructions = section_instructions.clone(),
            PROCESS_SECTION => {
                section_instructions.remove(0);
                self.process_instructions.push(Process::new(f64::from_bits(section_instructions.remove(0)) as usize, section_instructions.clone()))
            },
            METADATA_SECTION => {
                self.set_meta_data(section_instructions.clone())
            },
            _ => panic!("Invalid section")
        }
    }

    fn set_meta_data(&mut self, attributes: Vec<u64>) {
        let mut codes = attributes.into_iter();
        while let Some(code) = codes.next() {
            match code {
                COLOR => { // Set colour
                    match match vec![codes.next(), codes.next(), codes.next()].into_iter().collect::<Option<Vec<u64>>>(){ // Transpose vec of option into option of vec
                        Some(colour_vector) => colour_vector,
                        None => panic!("Invalid data: There should be three color values")
                    }.into_iter()
                    .map(|x| f64::from_bits(x) as f32)
                    .collect::<Vec<f32>>()[..] {
                        [red, green, blue] => self.color = Color{r: red, g: green, b: blue, a: SPELL_TRANSPARENCY},
                        _ => panic!("Failed to parse colors")
                    }
                },
                SHAPE => {
                    self.shape = Shape::from_num(codes.next().expect("Expected shape value after shape code"))
                }
                _ => panic!("Invalid attribute")
            }
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
    fn remove_component(&mut self, component: GString) {
        let component_code = get_component_num(&component.to_string()).expect("Component doesn't exist");
        self.component_catalogue.component_catalogue.remove(&component_code);
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
            Ok(_) => panic!("Invalid: Must be dictionary"),
            Err(_) => panic!("Invalid Json")
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
            Ok(_) => panic!("Invalid: Must be dictionary"),
            Err(_) => panic!("Invalid Json")
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

    /// Once `connect_player()` is called, whenever a component is cast, the provided node's `increase_component_efficiency` method will be called
    #[func]
    fn connect_player(&mut self, player: Gd<Node>) {
        let update_function = player.callable("increase_component_efficiency");
        self.base_mut().connect("component_cast".into(), update_function);
    }

    #[signal]
    fn component_cast(component_code: u64, efficiency_increase: f64);
}

mod boolean_logic { // 100 = true, 101 = false
    use super::{TRUE, FALSE};

    pub fn and(a: u64, b: u64) -> Result<u64, &'static str> {
        match (a, b) {
            (TRUE, TRUE) => Ok(TRUE),
            (TRUE, FALSE) => Ok(FALSE),
            (FALSE, TRUE) => Ok(FALSE),
            (FALSE, FALSE) => Ok(FALSE),
            _ => Err("Boolean logic can only compare booleans")
        }
    }

    pub fn or(a: u64, b: u64) -> Result<u64, &'static str> {
        match (a, b) {
            (TRUE, TRUE) => Ok(TRUE),
            (TRUE, FALSE) => Ok(TRUE),
            (FALSE, TRUE) => Ok(TRUE),
            (FALSE, FALSE) => Ok(FALSE),
            _ => Err("Boolean logic can only compare booleans")
        }
    }

    pub fn xor(a: u64, b: u64) -> Result<u64, &'static str> {
        match (a, b) {
            (TRUE, TRUE) => Ok(FALSE),
            (TRUE, FALSE) => Ok(TRUE),
            (FALSE, TRUE) => Ok(TRUE),
            (FALSE, FALSE) => Ok(FALSE),
            _ => Err("Boolean logic can only compare booleans")
        }
    }

    pub fn not(a: u64) -> Result<u64, &'static str> {
        match a {
            TRUE => Ok(FALSE),
            FALSE => Ok(TRUE),
            _ => Err("Not can only be used on booleans")
        }
    }
}

mod rpn_operations {
    use super::{NUMBER_LITERAL, TRUE, FALSE};

    pub fn binary_operation<T>(rpn_stack: &mut Vec<u64>, operation: T) -> Result<(), &'static str>
    where
        T: FnOnce(u64, u64) -> Result<u64, &'static str>
    {
        let bool_two = rpn_stack.pop().ok_or_else(|| "Expected value to compare")?;
        let bool_one = rpn_stack.pop().ok_or_else(|| "Expected value to compare")?;
        match operation(bool_one, bool_two) {
            Ok(num) => rpn_stack.push(num),
            Err(err) => return Err(err)
        };
        Ok(())
    }

    pub fn compare_operation<T>(rpn_stack: &mut Vec<u64>, operation: T) -> Result<(), &'static str>
    where
        T: FnOnce(f64, f64) -> bool
    {
        let argument_two = f64::from_bits(rpn_stack.pop().ok_or_else(|| "Expected value to compare")?);
        let _ = rpn_stack.pop().ok_or_else(|| "Expected number literal opcode")?;
        let argument_one = f64::from_bits(rpn_stack.pop().ok_or_else(|| "Expected value to compare")?);
        let _ = rpn_stack.pop().ok_or_else(|| "Expected number literal opcode")?;
        match operation(argument_one, argument_two) {
            true => rpn_stack.push(TRUE),
            false => rpn_stack.push(FALSE)
        };
        Ok(())
    }

    pub fn maths_operation<T>(rpn_stack: &mut Vec<u64>, operation: T) -> Result<(), &'static str>
    where
        T: FnOnce(f64, f64) -> f64
    {
        let argument_two = f64::from_bits(rpn_stack.pop().ok_or_else(|| "Expected value to compare")?);
        let _ = rpn_stack.pop().ok_or_else(|| "Expected number literal opcode")?;
        let argument_one = f64::from_bits(rpn_stack.pop().ok_or_else(|| "Expected value to compare")?);
        let _ = rpn_stack.pop().ok_or_else(|| "Expected number literal opcode")?;
        rpn_stack.extend(vec![NUMBER_LITERAL, f64::to_bits(operation(argument_one, argument_two))]);
        Ok(())
    }
}
