use crate::Spell;

pub fn give_velocity(spell: &mut Spell, parameters: &[u32], should_execute: bool) -> Option<u32> {
    match should_execute {
        false => Some(f32::to_bits((spell.energy * ((parameters[0] * parameters[0] + parameters[1] * parameters[1] + parameters[2] * parameters[2]) as f64).sqrt()) as f32)),
        true => None
    }
}

pub fn example_function_two(spell: &mut Spell, parameters: &[u32], should_execute: bool) -> Option<u32> {
    return None
}
