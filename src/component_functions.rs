use crate::Spell;


pub fn give_velocity(spell: &mut Spell, parameters: &[u32], should_execute: bool) -> Option<u32> {
    let parameter_one: f32 = f32::from_bits(parameters[0]);
    let parameter_two: f32 = f32::from_bits(parameters[1]);
    let parameter_three: f32 = f32::from_bits(parameters[2]);
    match should_execute {
        false => Some(f32::to_bits(((spell.energy / 10.0) * ((parameter_one * parameter_one + parameter_two * parameter_two + parameter_three * parameter_three) as f64).sqrt()) as f32)),
        true => None
    }
}

pub fn example_function_two(spell: &mut Spell, parameters: &[u32], should_execute: bool) -> Option<u32> {
    return None
}
