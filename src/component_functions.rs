use godot::{builtin::Vector3, log::godot_print, obj::WithBaseField};

use crate::Spell;

const APPLY_TO_SPELL_COEFFICIENT: f64 = 100.0;

pub fn give_velocity(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<u64> {
    let delta: f32 = match parameters[0] {
        0 => 1.0 as f32,
        num => f64::from_bits(num) as f32
    };
    godot_print!("{:?}", spell.energy);
    let parameter_one: f32 = f64::from_bits(parameters[1]) as f32;
    let parameter_two: f32 = f64::from_bits(parameters[2]) as f32;
    let parameter_three: f32 = f64::from_bits(parameters[3]) as f32;
    match should_execute {
        false => Some(f64::to_bits((spell.energy / APPLY_TO_SPELL_COEFFICIENT) * ((parameter_one * parameter_one + parameter_two * parameter_two + parameter_three * parameter_three) as f64).sqrt())),
        true => {
            let previous_position = spell.base_mut().get_position();
            let new_position = previous_position + Vector3 {x: parameter_one * delta, y: parameter_two * delta, z: parameter_three * delta};
            spell.base_mut().set_position(new_position);
            return None
        }
    }
}

pub fn example_function_two(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<u64> {
    return None
}
