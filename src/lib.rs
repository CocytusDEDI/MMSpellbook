use godot::prelude::*;
use godot::engine::Node3D;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

mod spelltranslator;

#[derive(GodotClass)]
#[class(base=Node3D)]
struct Spell {
    base: Base<Node3D>,
    energy: f64
}

#[godot_api]
impl INode3D for Spell {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            energy: 0.0
        }
    }
}
