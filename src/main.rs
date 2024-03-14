use std::f64::consts;

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

#[derive(Debug)]
struct Cube {
    side: f64
}

#[derive(Debug)]
struct Sphere {
    radius: f64
}

#[derive(Debug)]
struct Particle {
    mass: f64,
    energy: f64,
    shape: Shape
}

#[derive(Debug)]
struct MultiParticle {
    particle_list: Vec<Particle>
}

#[derive(Debug)]
struct Spell {
    kind: Kind,
    initial_energy: f64,
    further_energy_areas: Vec<String>,
    understanding_levels: Vec<String>
}

#[derive(Debug)]
enum Shape {
    Sphere(Sphere),
    Cube(Cube)
}

#[derive(Debug)]
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
        further_energy_areas: Vec::new(), 
        understanding_levels: Vec::new()
        }
    }
}

impl PartialEq for Kind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Kind::Particle(_), Kind::Particle(_)) => true,
            (Kind::MultiParticle(_), Kind::MultiParticle(_)) => true,
            _ => false,
        }
    }
}

// Spell construction: Kind {options} Target Condition
// E.g. Particle {radius: 7, mass: 3} Self
// Parsed into spell initalise energy, mantitory upkeep energy and extra energy fields

fn text_to_spell(spell_string: &str) {
    let spell_tokens: Vec<String> = spell_string.to_lowercase().split(" ").map(String::from).collect();
    let mut spell: Spell = Spell::new();
    let mut in_options: bool = false;
    let mut major_token_count: i32 = 0;
    for token in spell_tokens {
        if major_token_count == 0 {
            let string_kind: String = token.to_lowercase();
            match string_kind.as_str() {
                "particle" => spell.kind = Kind::Particle(Particle::new()),
                "multiparticle" => spell.kind = Kind::MultiParticle(MultiParticle::new()),
                _ => panic!("Not a valid kind")
            };
            major_token_count += 1;
        } else if token.chars().nth(0) == Some('{') {
            in_options = true;
            if major_token_count == 0 {
                if spell.kind == Kind::Particle(Particle::new()) {
                    match &token[1..] {
                        "energy: " => "",
                        _ => panic!("Not a valid option for kind")
                    };
                }
                
            }
        }
    }
}

struct SpellTokens {
    kind: String,
    kind_options: Vec<String>,
    self_instructions: Vec<String>
}

fn spell_constructor(spell_tokens: &SpellTokens) -> Spell{ // ToDo: RECURSION
    // Create new spell to edit later
    let mut spell = Spell::new();

    // Change spell kind
    match spell_tokens.kind.as_str() {
        "particle" => spell.kind = Kind::Particle(Particle::new()),
        "multiparticle" => spell.kind = Kind::MultiParticle(MultiParticle::new()),
        _ => panic!("Not a valid kind")
    };

    match spell.kind {
        Kind::Particle(_) => {
            // make big match statement

            for kind_option in &spell_tokens.kind_options {
                let option_parts: Vec<String> = kind_option.split(':').map(|s| s.trim().to_lowercase()).collect();
                
                match option_parts[0].as_str() {
                    "energy" => if let Kind::Particle(particle) = &mut spell.kind {
                        particle.energy = option_parts[1].parse().expect("Not a valid option");
                    }
                    "mass" => if let Kind::Particle(particle) = &mut spell.kind {
                        particle.mass = option_parts[1].parse().expect("Not a valid option");
                    }
                    "shape" => if let Kind::Particle(particle) = &mut spell.kind {
                        particle.shape = match option_parts[1].as_str() {
                            "sphere" => Shape::Sphere(Sphere::new()),
                            "cube" => Shape::Cube(Cube::new()),
                            _ => panic!("Not valid")
                        }
                    }
                    _ => panic!("Not valid option")
                }
            }
        }
        Kind::MultiParticle(_) => {
            for kind_option in &spell_tokens.kind_options {
                match kind_option.as_str() {
                    "particle_list" => if let Kind::MultiParticle(multiparticle) = &mut spell.kind {
                        multiparticle.particle_list = vec![];
                    }
                    _ => panic!("Not valid option")
                }
            }
        }
    }
    spell
}

// fn create_spell(parsed_spell: Vec<>) {}

/*
fn match_spell_attributes(choice: &str, options: enum, spell: &mut Spell) {
    match choice {
        option => spell.option = option.parse(),
        _ => panic!("Not a valid option")
    }
}
*/

#[derive(Debug)]
enum Value {
    String(String),
    Float(f64),
    Boolean(bool),
    Object(String, Vec<(String, Value)>),
} // Vec<(String, Value)>, E.g. ["Kind", Object("Particle", [("energy", 4.3), ("shape", Object("Sphere", [("radius", 3.2)]))])]


fn create_spell(properties: Vec<(String, Value)>) -> Spell {
    let mut spell = Spell::new();

    fn create_spell_inner(properties: Vec<(String, Value)>, spell: &mut Spell) {
        for property in properties{
            match property {
                (_, _) => spell.kind = Kind::Particle(Particle::new()) // Filler code, change later.
            }
        }
    }
    return spell;
}

fn main() {
    let rannis_dark_moon_tokens: SpellTokens = SpellTokens{kind: String::from("particle"), kind_options: vec![String::from("energy: 3.2"), String::from("shape: sphere"), String::from("shape: cube")], self_instructions: vec![]};
    let sasdlfj = Kind::Particle(Particle::new());
    println!("{:?}", spell_constructor(&rannis_dark_moon_tokens));
    let test = vec![("kind", "particle", (("energy", 2.3), ("mass", 2.3), ("shape", "sphere", ("radius", 3.4))))];
}
