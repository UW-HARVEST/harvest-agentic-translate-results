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
        let content = fs::read_to_string(filename).expect("Could not read config file");
        let mut config = Config {
            simulation_time: 0.0, dt: 0.0, x_particles: 0, y_particles: 0,
            x_squares: 0, y_squares: 0, square_in_grid_length: 0.0, radius: 0.0,
            kn: 0.0, ks: 0.0, rho: 0.0, thickness: 0.0, v0: 0.0, r0: 0.0,
        };
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim();
                match key {
                    "time" => config.simulation_time = val.parse().unwrap(),
                    "dt" => config.dt = val.parse().unwrap(),
                    "x_particles" => config.x_particles = val.parse().unwrap(),
                    "y_particles" => config.y_particles = val.parse().unwrap(),
                    "x_squares" => config.x_squares = val.parse().unwrap(),
                    "y_squares" => config.y_squares = val.parse().unwrap(),
                    "square_in_grid_length" => config.square_in_grid_length = val.parse().unwrap(),
                    "radius" => config.radius = val.parse().unwrap(),
                    "kn" => config.kn = val.parse().unwrap(),
                    "ks" => config.ks = val.parse().unwrap(),
                    "rho" => config.rho = val.parse().unwrap(),
                    "thickness" => config.thickness = val.parse().unwrap(),
                    "v0" => config.v0 = val.parse().unwrap(),
                    "r0" => config.r0 = val.parse().unwrap(),
                    _ => {}
                }
            }
        }
        config
    }
    pub fn initialize(&self) -> usize {
        (self.x_particles * self.y_particles) as usize
    }
    pub fn compute_mass(&self) -> f64 {
        self.rho * std::f64::consts::PI * self.radius * self.radius * self.thickness
    }
}
