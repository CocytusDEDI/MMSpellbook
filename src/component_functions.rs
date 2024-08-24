use godot::prelude::*;

use crate::{Spell, codes::opcodes::*, codes::shapecodes::*, Shape, Sphere, Cube, HasShape};

const APPLY_TO_SPELL_COEFFICIENT: f64 = 70.0;

// Utility:

pub fn give_velocity(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    let x_speed: f32 = f64::from_bits(parameters[0]) as f32;
    let y_speed: f32 = f64::from_bits(parameters[1]) as f32;
    let z_speed: f32 = f64::from_bits(parameters[2]) as f32;
    if should_execute {
        let new_velocity = spell.velocity + Vector3 {x: x_speed, y: y_speed, z: z_speed };
        spell.velocity = new_velocity;
        return None
    }

    return Some(vec![f64::to_bits((spell.energy / 2.0) * ((x_speed * x_speed + y_speed * y_speed + z_speed * z_speed) as f64) / APPLY_TO_SPELL_COEFFICIENT)]) // E_K = (1/2)mv^2
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

pub fn recharge_to(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![f64::to_bits(0.0)])
    }

    let energy_wanted = f64::from_bits(parameters[0]);

    if spell.energy >= energy_wanted {
        return None
    }

    spell.energy_requested = energy_wanted - spell.energy;

    return None
}

pub fn anchor(spell: &mut Spell, _parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![f64::to_bits(0.0)])
    }

    if spell.anchored_to != None {
        return None
    }

    spell.anchor();

    return None
}

pub fn undo_anchor(spell: &mut Spell, _parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![f64::to_bits(0.0)])
    }

    spell.undo_anchor();

    return None
}

pub fn perish(spell: &mut Spell, _parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![f64::to_bits(0.0)])
    }

    spell.perish();

    return None
}

pub fn take_shape(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![f64::to_bits(0.0)])
    }

    spell.undo_form();

    let shape_num = f64::from_bits(parameters[0]) as u64;
    let size_1 = f64::from_bits(parameters[1]) as f32;
    let size_2 = f64::from_bits(parameters[2]) as f32;
    let size_3 = f64::from_bits(parameters[3]) as f32;

    let shape = match shape_num {
        SPHERE => Shape::Sphere(Sphere { radius: size_1 }),
        CUBE => Shape::Cube(Cube { length: size_1, width: size_2, height: size_3 }),
        _ => panic!("Not a valid shape")
    };

    spell.shape = Some(shape);
    spell.set_shape(shape);

    return None
}

pub fn undo_shape(spell: &mut Spell, _parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![f64::to_bits(0.0)])
    }

    spell.undo_form();

    spell.set_shape(Shape::Sphere(Sphere::from_volume(spell.get_natural_volume(spell.energy as f32))));

    return None
}

// Logic:

pub fn get_time(spell: &mut Spell, _parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![NUMBER_LITERAL, f64::to_bits(0.1)]) // TODO: Should be set in config
    }

    let current_time = match spell.time {
        Some(ref ms) => ms,
        None => panic!("Time wasn't created")
    };

    let start_time = match spell.start_time {
        Some(ref ms) => ms,
        None => panic!("Time wasn't created")
    };

    return Some(vec![NUMBER_LITERAL, f64::to_bits((current_time.get_ticks_msec() - start_time) as f64 / 1000.0)])
}

pub fn moving(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    // Static energy return
    if !should_execute {
        return Some(vec![f64::to_bits(0.1)]) // TODO: Adjust energy requirements
    }

    let parameter_speed = f64::from_bits(parameters[0]);

    if (spell.velocity.x.powi(2) + spell.velocity.y.powi(2) + spell.velocity.z.powi(2)).sqrt() >= parameter_speed as f32 {
        return Some(vec![TRUE])
    } else {
        return Some(vec![FALSE])
    }
}

// Power:
pub fn set_damage(spell: &mut Spell, parameters: &[u64], should_execute: bool) -> Option<Vec<u64>> {
    if !should_execute {
        return Some(vec![f64::to_bits(0.0)])
    }

    spell.damage = f64::from_bits(parameters[0]);

    return None
}
