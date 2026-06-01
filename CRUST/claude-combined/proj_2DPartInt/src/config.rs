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
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim();
            let value = parts[1].trim();
            match key {
                "time" => cfg.simulation_time = value.parse().unwrap_or(0.0),
                "dt" => cfg.dt = value.parse().unwrap_or(0.0),
                "x_particles" => cfg.x_particles = value.parse().unwrap_or(0),
                "y_particles" => cfg.y_particles = value.parse().unwrap_or(0),
                "x_squares" => cfg.x_squares = value.parse().unwrap_or(0),
                "y_squares" => cfg.y_squares = value.parse().unwrap_or(0),
                "square_in_grid_length" => cfg.square_in_grid_length = value.parse().unwrap_or(0.0),
                "radius" => cfg.radius = value.parse().unwrap_or(0.0),
                "kn" => cfg.kn = value.parse().unwrap_or(0.0),
                "ks" => cfg.ks = value.parse().unwrap_or(0.0),
                "rho" => cfg.rho = value.parse().unwrap_or(0.0),
                "thickness" => cfg.thickness = value.parse().unwrap_or(0.0),
                "v0" => cfg.v0 = value.parse().unwrap_or(0.0),
                "r0" => cfg.r0 = value.parse().unwrap_or(0.0),
                _ => {}
            }
        }
        cfg
    }

    pub fn initialize(&self) -> usize {
        // Total number of particles produced by the simulation grid plus the
        // single falling particle.
        ((self.x_particles * self.y_particles) as usize) + 1
    }

    pub fn compute_mass(&self) -> f64 {
        // Mass = density * volume = rho * (PI * r^2 * thickness)
        self.rho * std::f64::consts::PI * self.radius * self.radius * self.thickness
    }
}
