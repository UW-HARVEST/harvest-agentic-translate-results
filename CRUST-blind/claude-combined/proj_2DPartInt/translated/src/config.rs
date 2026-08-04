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
                    "time" => {
                        config.simulation_time = value.parse::<f64>().unwrap_or(0.0)
                    }
                    "dt" => config.dt = value.parse::<f64>().unwrap_or(0.0),
                    "x_particles" => {
                        config.x_particles = value.parse::<i32>().unwrap_or(0)
                    }
                    "y_particles" => {
                        config.y_particles = value.parse::<i32>().unwrap_or(0)
                    }
                    "x_squares" => {
                        config.x_squares = value.parse::<i32>().unwrap_or(0)
                    }
                    "y_squares" => {
                        config.y_squares = value.parse::<i32>().unwrap_or(0)
                    }
                    "square_in_grid_length" => {
                        config.square_in_grid_length =
                            value.parse::<f64>().unwrap_or(0.0)
                    }
                    "radius" => config.radius = value.parse::<f64>().unwrap_or(0.0),
                    "kn" => config.kn = value.parse::<f64>().unwrap_or(0.0),
                    "ks" => config.ks = value.parse::<f64>().unwrap_or(0.0),
                    "rho" => config.rho = value.parse::<f64>().unwrap_or(0.0),
                    "thickness" => {
                        config.thickness = value.parse::<f64>().unwrap_or(0.0)
                    }
                    "v0" => config.v0 = value.parse::<f64>().unwrap_or(0.0),
                    "r0" => config.r0 = value.parse::<f64>().unwrap_or(0.0),
                    _ => {}
                }
            }
        }
        config
    }

    pub fn initialize(&self) -> usize {
        // Number of grid particles plus the falling particle.
        ((self.x_particles * self.y_particles) + 1) as usize
    }

    pub fn compute_mass(&self) -> f64 {
        // Mass of a cylindrical particle: pi * r^2 * thickness * rho.
        PI * self.radius * self.radius * self.thickness * self.rho
    }
}
