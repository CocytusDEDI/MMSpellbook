use godot::prelude::*;

use crate::Spell;

const APPLY_TO_SPELL_COEFFICIENT: f64 = 70.0;

pub fn give_velocity(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    let parameter_one: f32 = f64::from_bits(parameters[0]) as f32;
    let parameter_two: f32 = f64::from_bits(parameters[1]) as f32;
    let parameter_three: f32 = f64::from_bits(parameters[2]) as f32;
    if should_execute {
        let new_velocity = spell.velocity + Vector3 {x: parameter_one, y: parameter_two, z: parameter_three };
        spell.velocity = new_velocity;
        return None
    } else {
        return Some(vec![f64::to_bits((spell.energy / 2.0) * ((parameter_one * parameter_one + parameter_two * parameter_two + parameter_three * parameter_three) as f64).sqrt() / APPLY_TO_SPELL_COEFFICIENT)]) // E_K = (1/2)mv^2
    }
}
