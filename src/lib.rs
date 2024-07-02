use godot::prelude::*;
use godot::classes::CharacterBody3D;
use godot::classes::ICharacterBody3D;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

mod spelltranslator;

#[derive(GodotClass)]
#[class(base=CharacterBody3D)]
struct Spell {
    base: Base<CharacterBody3D>,
    energy: f64,
    ready_instructions: Vec<Vec<u8>>,
    process_instructions: Vec<Vec<u8>>
}


#[godot_api]
impl ICharacterBody3D for Spell {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            base,
            energy: 0.0,
            ready_instructions: vec![vec![]],
            process_instructions: vec![vec![]]
        }
    }

    fn ready(&mut self) {
        // Insert code here dynamically
    }

    fn physics_process(&mut self, delta: f64) {
        // Insert code here dynamically
    }
}

fn spell_virtual_machine(instructions: Vec<Vec<u8>>) {

}
