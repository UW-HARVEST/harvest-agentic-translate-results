use std::f64::consts::PI;

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
        let mut config = Self {
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

        let Ok(contents) = std::fs::read_to_string(filename) else {
            return config;
        };

        for line in contents.lines() {
            if line.is_empty() {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                eprintln!("Missing '=' in: {line}");
                continue;
            };

            match key {
                "time" => config.simulation_time = value.parse().unwrap_or(config.simulation_time),
                "dt" => config.dt = value.parse().unwrap_or(config.dt),
                "y_particles" => config.y_particles = value.parse().unwrap_or(config.y_particles),
                "x_particles" => config.x_particles = value.parse().unwrap_or(config.x_particles),
                "y_squares" => config.y_squares = value.parse().unwrap_or(config.y_squares),
                "x_squares" => config.x_squares = value.parse().unwrap_or(config.x_squares),
                "square_in_grid_length" => {
                    config.square_in_grid_length =
                        value.parse().unwrap_or(config.square_in_grid_length)
                }
                "radius" => config.radius = value.parse().unwrap_or(config.radius),
                "kn" => config.kn = value.parse().unwrap_or(config.kn),
                "ks" => config.ks = value.parse().unwrap_or(config.ks),
                "rho" => config.rho = value.parse().unwrap_or(config.rho),
                "thickness" => config.thickness = value.parse().unwrap_or(config.thickness),
                "v0" => config.v0 = value.parse().unwrap_or(config.v0),
                "r0" => config.r0 = value.parse().unwrap_or(config.r0),
                _ => eprintln!("Invalid key: {key}"),
            }
        }

        config
    }
    pub fn initialize(&self) -> usize {
        (self.x_particles as usize * self.y_particles as usize) + 1
    }
    pub fn compute_mass(&self) -> f64 {
        self.rho * self.thickness * PI * self.radius * self.radius
    }
}
