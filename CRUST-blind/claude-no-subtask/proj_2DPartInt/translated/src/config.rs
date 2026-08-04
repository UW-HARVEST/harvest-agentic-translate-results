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
    /// Parses the provided config file and returns a `Config`.
    ///
    /// The file is expected to contain lines of the form `key=value`,
    /// matching the example simulation config format.
    pub fn parse_config(filename: &str) -> Self {
        // Default-initialize all fields to zero.
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

        let contents = match fs::read_to_string(filename) {
            Ok(c) => c,
            Err(_) => return config,
        };

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            // Allow comments starting with '#'.
            if line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(2, '=');
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

    /// Returns the total number of particles in the simulation:
    /// the rectangular grid of bed particles plus a single falling particle.
    pub fn initialize(&self) -> usize {
        let bed = (self.x_particles as i64).max(0) as usize
            * (self.y_particles as i64).max(0) as usize;
        bed + 1
    }

    /// Computes the mass of a single bed particle, modelled as a disc of
    /// radius `radius` and thickness `thickness` with density `rho`.
    pub fn compute_mass(&self) -> f64 {
        self.rho * self.thickness * PI * self.radius * self.radius
    }
}
