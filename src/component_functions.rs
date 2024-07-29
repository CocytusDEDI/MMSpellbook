use godot::prelude::*;

use crate::Spell;

const APPLY_TO_SPELL_COEFFICIENT: f64 = 70.0;

pub fn give_velocity(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    let x_speed: f32 = f64::from_bits(parameters[0]) as f32;
    let y_speed: f32 = f64::from_bits(parameters[1]) as f32;
    let z_speed: f32 = f64::from_bits(parameters[2]) as f32;
    if should_execute {
        let new_velocity = spell.velocity + Vector3 {x: x_speed, y: y_speed, z: z_speed };
        spell.velocity = new_velocity;
        return None
    } else {
        return Some(vec![f64::to_bits((spell.energy / 2.0) * ((x_speed * x_speed + y_speed * y_speed + z_speed * z_speed) as f64).sqrt() / APPLY_TO_SPELL_COEFFICIENT)]) // E_K = (1/2)mv^2
    }
}

pub fn moving(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    // Static energy return
    if !should_execute {
        return Some(vec![f64::to_bits(0.1)])
    }

    let parameter_speed = f64::from_bits(parameters[0]);

    if (spell.velocity.x.powi(2) + spell.velocity.y.powi(2) + spell.velocity.z.powi(2)).sqrt() >= parameter_speed as f32 {
        return Some(vec![100])
    } else {
        return Some(vec![101])
    }
}

pub fn take_form(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    let form_code = f64::from_bits(parameters[0]) as u64;

    if !should_execute {
        return Some(vec![f64::to_bits(spell.config.forms.get(&form_code).expect("Expected form code to map to a form").energy_required)])
    }

    spell.set_form(form_code);
    return None
}

pub fn undo_form(spell: &mut Spell, _parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![0])
    }

    spell.undo_form();
    return None
}

pub fn get_time(spell: &mut Spell, _parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![102, f64::to_bits(0.1)]) // TODO: Should be set in config
    }

    let current_time = match spell.time {
        Some(ref ms) => ms,
        None => panic!("Time wasn't created")
    };

    let start_time = match spell.start_time {
        Some(ref ms) => ms,
        None => panic!("Time wasn't created")
    };

    return Some(vec![102, f64::to_bits((current_time.get_ticks_msec() - start_time) as f64 / 1000.0)])
}
