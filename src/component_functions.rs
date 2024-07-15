use godot::prelude::*;

use crate::Spell;

const APPLY_TO_SPELL_COEFFICIENT: f64 = 100.0;

pub fn give_velocity(spell: &mut Spell, parameters: &[u64], delta: f64, should_execute: bool) -> Option<u64> {
    let f32_delta: f32 = delta as f32;
    let parameter_one: f32 = f64::from_bits(parameters[0]) as f32;
    let parameter_two: f32 = f64::from_bits(parameters[1]) as f32;
    let parameter_three: f32 = f64::from_bits(parameters[2]) as f32;
    match should_execute {
        false => Some(f64::to_bits((spell.energy / APPLY_TO_SPELL_COEFFICIENT) * ((parameter_one * parameter_one + parameter_two * parameter_two + parameter_three * parameter_three) as f64).sqrt())),
        true => {
            let previous_position = spell.base_mut().get_position();
            let new_position = previous_position + Vector3 {x: parameter_one * f32_delta, y: parameter_two * f32_delta, z: parameter_three * f32_delta};
            spell.base_mut().set_position(new_position);
            return None
        }
    }
}
