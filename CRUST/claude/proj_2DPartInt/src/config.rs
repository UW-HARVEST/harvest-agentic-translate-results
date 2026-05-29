use std::fs;

pub struct Config {
    pub simulation_time: f64,
    pub dt: f64,
    pub x_particles: i32,
    pub y_particles: i32,
    pub x_squares: i32,
    pub y_squares: i32,
    pub square_in_grid_length: f64,
    pub radius: f64,
    pub kn: f64,
    pub ks: f64,
    pub rho: f64,
    pub thickness: f64,
    pub v0: f64,
    pub r0: f64,
}
impl Config {
    pub fn parse_config(filename: &str) -> Self {
        let mut simulation_time: f64 = 0.0;
        let mut dt: f64 = 0.0;
        let mut x_particles: i32 = 0;
        let mut y_particles: i32 = 0;
        let mut x_squares: i32 = 0;
        let mut y_squares: i32 = 0;
        let mut square_in_grid_length: f64 = 0.0;
        let mut radius: f64 = 0.0;
        let mut kn: f64 = 0.0;
        let mut ks: f64 = 0.0;
        let mut rho: f64 = 0.0;
        let mut thickness: f64 = 0.0;
        let mut v0: f64 = 0.0;
        let mut r0: f64 = 0.0;

        let contents = fs::read_to_string(filename).unwrap_or_default();
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "time" => simulation_time = value.parse().unwrap_or(0.0),
                    "dt" => dt = value.parse().unwrap_or(0.0),
                    "x_particles" => x_particles = value.parse().unwrap_or(0),
                    "y_particles" => y_particles = value.parse().unwrap_or(0),
                    "x_squares" => x_squares = value.parse().unwrap_or(0),
                    "y_squares" => y_squares = value.parse().unwrap_or(0),
                    "square_in_grid_length" => {
                        square_in_grid_length = value.parse().unwrap_or(0.0)
                    }
                    "radius" => radius = value.parse().unwrap_or(0.0),
                    "kn" => kn = value.parse().unwrap_or(0.0),
                    "ks" => ks = value.parse().unwrap_or(0.0),
                    "rho" => rho = value.parse().unwrap_or(0.0),
                    "thickness" => thickness = value.parse().unwrap_or(0.0),
                    "v0" => v0 = value.parse().unwrap_or(0.0),
                    "r0" => r0 = value.parse().unwrap_or(0.0),
                    _ => {}
                }
            }
        }

        Config {
            simulation_time,
            dt,
            x_particles,
            y_particles,
            x_squares,
            y_squares,
            square_in_grid_length,
            radius,
            kn,
            ks,
            rho,
            thickness,
            v0,
            r0,
        }
    }
    pub fn initialize(&self) -> usize {
        // Returns the number of particles in the simulation. There is one
        // additional falling particle.
        (self.x_particles * self.y_particles + 1) as usize
    }
    pub fn compute_mass(&self) -> f64 {
        // Mass = rho * volume; volume of a disc of given radius and thickness
        // is PI * radius^2 * thickness.
        std::f64::consts::PI * self.radius * self.radius * self.thickness * self.rho
    }
}
