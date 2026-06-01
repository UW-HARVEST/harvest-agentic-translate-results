use std::fs;
use std::path::PathBuf;
use twoDPartInt::config::Config;

fn write_temp(name: &str, contents: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(name);
    fs::write(&p, contents).unwrap();
    p
}

#[test]
fn test_parse_config_full() {
    let path = write_temp(
        "test_config_full.txt",
        "time=1\n\
         dt=0.00025\n\
         \n\
         x_particles=6\n\
         y_particles=6\n\
         x_squares=7\n\
         y_squares=7\n\
         square_in_grid_length=120\n\
         \n\
         radius=50\n\
         kn=2474358.297\n\
         ks=190335.254\n\
         rho=0.00000078\n\
         thickness=30\n\
         v0=0\n\
         r0=50\n",
    );
    let cfg = Config::parse_config(path.to_str().unwrap());
    assert_eq!(cfg.simulation_time, 1.0);
    assert_eq!(cfg.dt, 0.00025);
    assert_eq!(cfg.x_particles, 6);
    assert_eq!(cfg.y_particles, 6);
    assert_eq!(cfg.x_squares, 7);
    assert_eq!(cfg.y_squares, 7);
    assert_eq!(cfg.square_in_grid_length, 120.0);
    assert_eq!(cfg.radius, 50.0);
    assert!((cfg.kn - 2474358.297).abs() < 1e-6);
    assert!((cfg.ks - 190335.254).abs() < 1e-6);
    assert!((cfg.rho - 0.00000078).abs() < 1e-15);
    assert_eq!(cfg.thickness, 30.0);
    assert_eq!(cfg.v0, 0.0);
    assert_eq!(cfg.r0, 50.0);
}

#[test]
fn test_initialize_returns_count() {
    let cfg = Config {
        simulation_time: 1.0,
        dt: 0.00025,
        x_particles: 6,
        y_particles: 6,
        x_squares: 7,
        y_squares: 7,
        square_in_grid_length: 120.0,
        radius: 50.0,
        kn: 2474358.297,
        ks: 190335.254,
        rho: 0.00000078,
        thickness: 30.0,
        v0: 0.0,
        r0: 50.0,
    };
    // 6 * 6 grid particles + 1 falling particle = 37.
    assert_eq!(cfg.initialize(), 37);
}

#[test]
fn test_initialize_zero_particles() {
    let cfg = Config {
        simulation_time: 1.0,
        dt: 0.00025,
        x_particles: 0,
        y_particles: 0,
        x_squares: 7,
        y_squares: 7,
        square_in_grid_length: 120.0,
        radius: 50.0,
        kn: 0.0,
        ks: 0.0,
        rho: 0.0,
        thickness: 0.0,
        v0: 0.0,
        r0: 0.0,
    };
    assert_eq!(cfg.initialize(), 1);
}

#[test]
fn test_compute_mass_simple() {
    let cfg = Config {
        simulation_time: 0.0,
        dt: 0.0,
        x_particles: 0,
        y_particles: 0,
        x_squares: 0,
        y_squares: 0,
        square_in_grid_length: 0.0,
        radius: 1.0,
        kn: 0.0,
        ks: 0.0,
        rho: 1.0,
        thickness: 1.0,
        v0: 0.0,
        r0: 0.0,
    };
    // mass = pi * 1 * 1 * 1 = pi.
    assert!((cfg.compute_mass() - std::f64::consts::PI).abs() < 1e-12);
}

#[test]
fn test_compute_mass_realistic() {
    let cfg = Config {
        simulation_time: 0.0,
        dt: 0.0,
        x_particles: 0,
        y_particles: 0,
        x_squares: 0,
        y_squares: 0,
        square_in_grid_length: 0.0,
        radius: 50.0,
        kn: 0.0,
        ks: 0.0,
        rho: 0.00000078,
        thickness: 30.0,
        v0: 0.0,
        r0: 0.0,
    };
    // mass = pi * 50 * 50 * 30 * 0.00000078
    let expected = std::f64::consts::PI * 50.0 * 50.0 * 30.0 * 0.00000078;
    assert!((cfg.compute_mass() - expected).abs() < 1e-12);
}

fn main() {}
