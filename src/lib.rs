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
use spelltranslator::Parameter;

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
    static ref COMPONENT_TO_FUNCTION_MAP: HashMap<u8, fn(&mut Spell, &[u8], bool) -> f64> = {
        let mut component_map = HashMap::new();
        component_map.insert(0, component_functions::give_velocity as fn(&mut Spell, &[u8], bool) -> f64);
        return component_map
    };
}

impl Spell {
    fn spell_virtual_machine(&mut self, instructions: &[u32]) -> Result<(), u32> {
        // ToDo: Code such as rpn evaluation should be in it's own subroutine to be available to call for other logic statements.
        // ToDo: Decide to remove or not remove strings from Parameter

        let in_component = false;
        let mut instructions_iter = instructions.iter();
        let mut skip_block = false;
        while let Some(bits) = instructions_iter.next() {
            if !in_component {
                let opcode: u32 = bits & 0x80000000;
                let value: u32 = bits & !0x80000000;

                // Checking if first most significant bit is 1, if so, it is a component
                if opcode == 0x80000000 {

                } else {
                    // bits represent logic statement
                    match bits {
                        0x00000000 => skip_block = false, // End current scope
                        0x00000001 => { // If statement
                            let mut rpn_stack: Vec<Parameter> = vec![];
                            while let Some(bits) = instructions_iter.next() {
                                let opcode: u32 = bits & 0x80000000;
                                let value: u32 = bits & !0x80000000;

                                if opcode == 0x80000000 {
                                    rpn_stack.push(Parameter::Integer(*bits as i32));
                                } else {
                                    match bits {
                                        // Deal with logic units:
                                        // 0x00000002 => rpn_stack.push(Parameter::Boolean(rpn_stack.pop() && rnp_stack.pop())),
                                        _ => panic!("Not valid if statement logic")
                                    }
                                }
                            }
                            // Use RPN to evaluate expression to bool


                        },
                        _ => panic!("Logic statement does not exist")
                    }
                }

            } else {

            }
        }

        // Old code, some still needed for later:
        /*
        for instruction in instructions {
            if let Some((component, parameters)) = instruction.split_first() {
                if let Some(function) = COMPONENT_TO_FUNCTION_MAP.get(component) {
                    // Cloning here is expensive
                    if let Some(component_efficiencies) = self.component_efficiencies.clone() {
                        if let Some(efficiency) = component_efficiencies.get(component) {
                            if self.energy >= function(self, parameters, false) / efficiency {
                                self.energy -= function(self, parameters, true) / efficiency;
                            } else {
                                return Err(1)
                            }
                        } else {
                            return Err(0) // Placeholder for error code currently
                        }
                    } else {
                        panic!("No component efficiencies")
                    }
                } else {
                    panic!("Unknown component")
                }
            }
        }
        */

        return Ok(())
    }

    fn call_component(&mut self, component_code: u32) -> Result<Option<Parameter>, ()> {
        return Ok(Some(Parameter::Boolean(true)))
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
