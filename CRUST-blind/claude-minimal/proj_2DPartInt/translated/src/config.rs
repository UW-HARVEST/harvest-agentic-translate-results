use std::fs::File;
use std::io::{BufRead, BufReader};

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
    /// Parses a config file with key=value lines, mirroring the C `parse_config`.
    pub fn parse_config(filename: &str) -> Self {
        let mut config = Config {
            simulation_time: 0.0,
            dt: 0.0,
            x_particles: 0,
            y_particles: 0,
            x_squares: 0,
            y_squares: 0,
            square_in_grid_length: 0.0,
            radius: 0.0,
            kn: 0.0,
            ks: 0.0,
            rho: 0.0,
            thickness: 0.0,
            v0: 0.0,
            r0: 0.0,
        };

        let file = match File::open(filename) {
            Ok(f) => f,
            Err(_) => return config,
        };
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut parts = trimmed.splitn(2, '=');
            let key = match parts.next() {
                Some(k) => k.trim(),
                None => continue,
            };
            let value = match parts.next() {
                Some(v) => v.trim(),
                None => continue,
            };

            match key {
                "time" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.simulation_time = v;
                    }
                }
                "dt" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.dt = v;
                    }
                }
                "x_particles" => {
                    if let Ok(v) = value.parse::<i32>() {
                        config.x_particles = v;
                    }
                }
                "y_particles" => {
                    if let Ok(v) = value.parse::<i32>() {
                        config.y_particles = v;
                    }
                }
                "x_squares" => {
                    if let Ok(v) = value.parse::<i32>() {
                        config.x_squares = v;
                    }
                }
                "y_squares" => {
                    if let Ok(v) = value.parse::<i32>() {
                        config.y_squares = v;
                    }
                }
                "square_in_grid_length" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.square_in_grid_length = v;
                    }
                }
                "radius" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.radius = v;
                    }
                }
                "kn" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.kn = v;
                    }
                }
                "ks" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.ks = v;
                    }
                }
                "rho" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.rho = v;
                    }
                }
                "thickness" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.thickness = v;
                    }
                }
                "v0" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.v0 = v;
                    }
                }
                "r0" => {
                    if let Ok(v) = value.parse::<f64>() {
                        config.r0 = v;
                    }
                }
                _ => {}
            }
        }

        config
    }

    /// Returns the number of particles to be simulated. There is one extra
    /// "falling" particle in addition to the grid of x_particles by y_particles.
    pub fn initialize(&self) -> usize {
        (self.x_particles as usize) * (self.y_particles as usize) + 1
    }

    /// Computes the mass of a particle given its radius, thickness and density.
    /// mass = volume * rho = (PI * r^2 * thickness) * rho
    pub fn compute_mass(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius * self.thickness * self.rho
    }
}
