use godot::prelude::*;
use godot::classes::Area3D;
use godot::classes::IArea3D;
use godot::classes::CollisionShape3D;
use godot::classes::SphereShape3D;
use godot::classes::CsgSphere3D;
use godot::classes::Shape3D;
use lazy_static::lazy_static;
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
    ready_instructions: Vec<Vec<u8>>,
    process_instructions: Vec<Vec<u8>>
}


#[godot_api]
impl IArea3D for Spell {
    fn init(base: Base<Area3D>) -> Self {
        Self {
            base,
            energy: 0.0,
            ready_instructions: vec![vec![]],
            process_instructions: vec![vec![]]
        }
    }

    fn ready(&mut self) {
        let mut collision_shape: Gd<CollisionShape3D> = CollisionShape3D::new_alloc();
        let shape = SphereShape3D::new_gd();
        collision_shape.set_shape(shape.upcast::<Shape3D>());
        self.base_mut().add_child(collision_shape.upcast());
        self.base_mut().add_child(CsgSphere3D::new_alloc().upcast());

        self.spell_virtual_machine(0);
    }

    fn physics_process(&mut self, delta: f64) {
        self.spell_virtual_machine(1)
    }
}

lazy_static! {
    static ref COMPONENT_MAP: HashMap<u8, fn(&mut Spell, &[u8])> = {
        let mut component_map = HashMap::new();
        component_map.insert(0, component_functions::example_function as fn(&mut Spell, &[u8]));
        return component_map
    };
}

impl Spell {
    fn spell_virtual_machine(&mut self, called_from: u8) {
        for instruction in match called_from {
            // Cloning here is exponsive and could be changed
            0 => self.ready_instructions.clone(),
            1 => self.process_instructions.clone(),
            _ => panic!("Not valid instruction call")
        } {
            if let Some((&component, parameters)) = instruction.split_first() {
                if let Some(&function) = COMPONENT_MAP.get(&component) {
                    function(self, parameters);
                } else {
                    panic!("Unknown component");
                }
            }
        }
    }
}

// ToDo: Add in energy useage for each component called and use the efficiency of each component
fn use_energy(spell: &mut Spell, component: u8) {}
