use twoDPartInt::config;
use std::fs;
use std::io::Write;

fn write_temp_config(name: &str, contents: &str) -> String {
    let path = format!("/tmp/{}_test_config_twoDPartInt.txt", name);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(contents.as_bytes()).unwrap();
    path
}

#[test]
fn test_parse_example_config() {
    let path = write_temp_config(
        "example",
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
    let cfg = config::Config::parse_config(&path);
    assert_eq!(cfg.simulation_time, 1.0);
    assert_eq!(cfg.dt, 0.00025);
    assert_eq!(cfg.x_particles, 6);
    assert_eq!(cfg.y_particles, 6);
    assert_eq!(cfg.x_squares, 7);
    assert_eq!(cfg.y_squares, 7);
    assert_eq!(cfg.square_in_grid_length, 120.0);
    assert_eq!(cfg.radius, 50.0);
    assert_eq!(cfg.kn, 2474358.297);
    assert_eq!(cfg.ks, 190335.254);
    assert_eq!(cfg.rho, 0.00000078);
    assert_eq!(cfg.thickness, 30.0);
    assert_eq!(cfg.v0, 0.0);
    assert_eq!(cfg.r0, 50.0);

    // Test initialize: x*y + 1 = 6*6 + 1 = 37
    assert_eq!(cfg.initialize(), 37);

    // mass = pi * r^2 * thickness * rho
    // = pi * 2500 * 30 * 0.00000078
    let expected_mass = std::f64::consts::PI * 50.0 * 50.0 * 30.0 * 0.00000078;
    assert!((cfg.compute_mass() - expected_mass).abs() < 1e-12);
    let _ = fs::remove_file(&path);
}

#[test]
fn test_parse_missing_file_returns_zeros() {
    let cfg = config::Config::parse_config("/tmp/this_file_does_not_exist_twoDPartInt.txt");
    assert_eq!(cfg.simulation_time, 0.0);
    assert_eq!(cfg.dt, 0.0);
    assert_eq!(cfg.x_particles, 0);
    assert_eq!(cfg.y_particles, 0);
    assert_eq!(cfg.x_squares, 0);
    assert_eq!(cfg.y_squares, 0);
    assert_eq!(cfg.square_in_grid_length, 0.0);
    assert_eq!(cfg.radius, 0.0);
    assert_eq!(cfg.kn, 0.0);
    assert_eq!(cfg.ks, 0.0);
    assert_eq!(cfg.rho, 0.0);
    assert_eq!(cfg.thickness, 0.0);
    assert_eq!(cfg.v0, 0.0);
    assert_eq!(cfg.r0, 0.0);
}

#[test]
fn test_initialize_zero() {
    let cfg = config::Config {
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
    // 0 * 0 + 1 = 1
    assert_eq!(cfg.initialize(), 1);
    // mass = pi * 0 * 0 * 0 * 0 = 0
    assert_eq!(cfg.compute_mass(), 0.0);
}

#[test]
fn test_compute_mass_specific() {
    let cfg = config::Config {
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
    // mass = pi * 1 * 1 * 1 * 1 = pi
    assert!((cfg.compute_mass() - std::f64::consts::PI).abs() < 1e-15);
}

fn main() {}
