use std::f64::consts::PI;
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
    /// Parses a "key=value" formatted text file into a Config.
    /// Lines that do not match the expected format are ignored.
    pub fn parse_config(filename: &str) -> Self {
        let mut cfg = Config {
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

        let contents = match fs::read_to_string(filename) {
            Ok(c) => c,
            Err(_) => return cfg,
        };

        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "time" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.simulation_time = v;
                        }
                    }
                    "dt" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.dt = v;
                        }
                    }
                    "x_particles" => {
                        if let Ok(v) = value.parse::<i32>() {
                            cfg.x_particles = v;
                        }
                    }
                    "y_particles" => {
                        if let Ok(v) = value.parse::<i32>() {
                            cfg.y_particles = v;
                        }
                    }
                    "x_squares" => {
                        if let Ok(v) = value.parse::<i32>() {
                            cfg.x_squares = v;
                        }
                    }
                    "y_squares" => {
                        if let Ok(v) = value.parse::<i32>() {
                            cfg.y_squares = v;
                        }
                    }
                    "square_in_grid_length" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.square_in_grid_length = v;
                        }
                    }
                    "radius" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.radius = v;
                        }
                    }
                    "kn" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.kn = v;
                        }
                    }
                    "ks" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.ks = v;
                        }
                    }
                    "rho" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.rho = v;
                        }
                    }
                    "thickness" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.thickness = v;
                        }
                    }
                    "v0" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.v0 = v;
                        }
                    }
                    "r0" => {
                        if let Ok(v) = value.parse::<f64>() {
                            cfg.r0 = v;
                        }
                    }
                    _ => {}
                }
            }
        }
        cfg
    }

    /// Returns the total number of particles in the initial grid layout.
    /// One additional particle is allocated for the falling particle, matching
    /// the original initialize() behaviour from the C codebase.
    pub fn initialize(&self) -> usize {
        (self.x_particles as usize) * (self.y_particles as usize) + 1
    }

    /// Computes the mass of a single particle, given its physical properties.
    /// mass = rho * thickness * area = rho * thickness * pi * r^2
    pub fn compute_mass(&self) -> f64 {
        self.rho * self.thickness * PI * self.radius * self.radius
    }
}
