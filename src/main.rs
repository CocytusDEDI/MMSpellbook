use std::{f64::consts, io};
use serde::{Serialize, Deserialize};
use serde_json;

// Maybe people have charge instead of energy and use eletric charge equations for force
// Energy used when moving magic = either strength x either cut x magic charge = either strength x distance x magic charge
// Should area be in the equation though?
// Either strength is constant everywhere (could add exceptions for lore reasons)
// Magic charge is always positive
// People are negatively magicely charged causing magic regen
// Like charges repell (Magic particles repell each other, making it hard to compress magic)
// Thus magic capacity is directly proportional to magical negative charge of a person
// Magic isn't a particle and thus can't be collected by running though magic
// Either provides charge to

#[derive(Debug, Serialize, Deserialize)]
struct Cube {
    side: f64
}

#[derive(Debug, Serialize, Deserialize)]
struct Sphere {
    radius: f64
}

#[derive(Debug, Serialize, Deserialize)]
struct Particle {
    mass: f64,
    energy: f64,
    shape: Shape
}

#[derive(Debug, Serialize, Deserialize)]
struct MultiParticle {
    particle_list: Vec<Particle>
}

#[derive(Debug, Serialize, Deserialize)]
struct Spell {
    kind: Kind,
    initial_energy: f64,
    // further_energy_areas: Vec<String>,
    // understanding_levels: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
enum Shape {
    Sphere(Sphere),
    Cube(Cube)
}

#[derive(Debug, Serialize, Deserialize)]
enum Kind {
    Particle(Particle),
    MultiParticle(MultiParticle)
}

trait Volume {
    fn calculate_volume(&self) -> f64;
}

impl Volume for Sphere {
    fn calculate_volume(&self) -> f64 {
        4.0 / 3.0 * consts::PI * self.radius.powi(3)
    }
}

impl Volume for Cube {
    fn calculate_volume(&self) -> f64 {
        self.side.powi(3)
    }
}

impl Volume for Shape {
    fn calculate_volume(&self) -> f64 {
        match self {
            Shape::Sphere(sphere) => sphere.calculate_volume(),
            Shape::Cube(cube) => cube.calculate_volume()
        }
    }
}

impl Sphere {
    pub fn new() -> Self {
        Self {radius: 0.0}
    }
}

impl Cube {
    pub fn new() -> Self {
        Self {side: 0.0}
    }
}

impl Particle {
    pub fn calculate_density(&self) -> f64 {
        self.mass / self.shape.calculate_volume()
    }

    pub fn calculate_energy(&self) -> f64 {
        self.mass * 299792458.0 * 299792458.0
    }

    pub fn new() -> Self {
        Self {mass: 0.0, energy: 0.0, shape: Shape::Sphere(Sphere::new())}
    }

    pub fn apply_force(&mut self, force: f64, time: f64) {
        self.energy -= 0.5 * (force.powi(2) * time.powi(2)) / self.mass;
    } // Energy is half of force squared times time squared over mass
}

impl MultiParticle {
    pub fn new() -> Self {
        Self {particle_list: vec![Particle::new()]}
    }
}

impl Spell {
    fn new()-> Self {
        Self {
        kind: Kind::Particle(Particle {mass: 0.0, energy: 0.0, shape: Shape::Sphere(Sphere {radius: 0.0})}), 
        initial_energy: 0.0,
        }
    }
}

fn create_spell_from_json(spell_json: &str) -> Result<Spell, serde_json::Error> {
    let spell: Spell = serde_json::from_str(spell_json)?;
    Ok(spell)
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Invalid text");
    let spell = create_spell_from_json(&input).expect("Invalid spell"); // Example input: r#"{"kind":{"Particle":{"mass":4.3,"energy":7.7,"shape":{"Sphere":{"radius":4.3}}}},"initial_energy":4.2}"#
    println!("{:?}", spell);
}
