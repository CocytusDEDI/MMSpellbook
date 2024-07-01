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
    energy: f64
}

#[godot_api]
impl ICharacterBody3D for Spell {
    fn init(base: Base<CharacterBody3D>) -> Self {
        Self {
            base,
            energy: 0.0
        }
    }
}
