use std::f64::consts;
use serde::{Serialize, Deserialize};

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

const MASS_TO_ENERGY_CONSTANT: f64 = 7.1;

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
    shape: Shape
}

#[derive(Debug, Serialize, Deserialize)]
struct MultiParticle {
    particle_list: Vec<Particle>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Spell {
    kind: Kind
    // instructions: 
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

trait Energy {
    fn calculate_energy(&self) -> f64;
}

trait Volume {
    fn calculate_volume(&self) -> f64;
}

impl Energy for Particle {
    fn calculate_energy(&self) -> f64 {
        self.mass * 299792458.0 * 299792458.0
    }
}

impl Energy for MultiParticle {
    fn calculate_energy(&self) -> f64 {
        let mut energy: f64 = 0.0;
        for particle in &self.particle_list {
            energy += particle.calculate_energy();
        }
        energy
    }
}

impl Energy for Kind {
    fn calculate_energy(&self) -> f64 {
        match self {
            Kind::Particle(particle) => particle.calculate_energy(),
            Kind::MultiParticle(multiparticle) => multiparticle.calculate_energy()
        }
    }
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

    pub fn new() -> Self {
        Self {mass: 0.0, shape: Shape::Sphere(Sphere::new())}
    }

    pub fn apply_force(&mut self, force: f64, time: f64) -> bool {
        let mass_left = self.mass - 0.5 * (force.powi(2) * time.powi(2)) / (self.mass * 299792458.0 * 299792458.0); // Check if true with a physicist 
        if mass_left >= 0.0 {
            self.mass = mass_left;
            true
        } else {
            false
        }
    } // Energy is half of force squared times time squared over mass times c squared
}

impl MultiParticle {
    pub fn new() -> Self {
        Self {particle_list: vec![Particle::new()]}
    }
}

impl Spell {
    fn new()-> Self {
        Self {
        kind: Kind::Particle(Particle {mass: 0.0, shape: Shape::Sphere(Sphere {radius: 0.0})}),
        }
    }

    fn calculate_energy(&mut self) -> f64{
        self.kind.calculate_energy()
    }
}