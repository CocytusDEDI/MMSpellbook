use std::io;
use serde_json;
use crate::libmmspellbook::Spell;

mod libmmspellbook;

fn create_spell_from_json(spell_json: &str) -> Result<Spell, serde_json::Error> {
    let spell: Spell = serde_json::from_str(spell_json)?;
    Ok(spell)
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Invalid text");
    let spell = create_spell_from_json(&input).expect("Invalid spell"); // Example input: r#"{"kind":{"Particle":{"mass":4.3,"shape":{"Sphere":{"radius":4.3}}}}}"#
    println!("{:?}", spell);
}
