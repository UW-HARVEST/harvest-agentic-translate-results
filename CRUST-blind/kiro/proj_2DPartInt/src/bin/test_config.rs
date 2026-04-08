use twoDPartInt::config::Config;
use std::io::Write;

#[test]
fn test_parse_config() {
    let dir = std::env::temp_dir().join("test_parse_config");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "time=1\ndt=0.00025\nx_particles=6\ny_particles=6\nx_squares=7\ny_squares=7\nsquare_in_grid_length=120\nradius=50\nkn=2474358.297\nks=190335.254\nrho=0.00000078\nthickness=30\nv0=0\nr0=50\n").unwrap();
    let cfg = Config::parse_config(path.to_str().unwrap());
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
    assert!((cfg.rho - 0.00000078).abs() < 1e-15);
    assert_eq!(cfg.thickness, 30.0);
    assert_eq!(cfg.v0, 0.0);
    assert_eq!(cfg.r0, 50.0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_compute_mass() {
    let cfg = Config {
        simulation_time: 0.0, dt: 0.0, x_particles: 0, y_particles: 0,
        x_squares: 0, y_squares: 0, square_in_grid_length: 0.0,
        radius: 50.0, kn: 0.0, ks: 0.0, rho: 0.00000078, thickness: 30.0,
        v0: 0.0, r0: 0.0,
    };
    let mass = cfg.compute_mass();
    let expected = 0.00000078 * std::f64::consts::PI * 50.0 * 50.0 * 30.0;
    assert!((mass - expected).abs() < 1e-10);
}

#[test]
fn test_initialize() {
    let cfg = Config {
        simulation_time: 0.0, dt: 0.0, x_particles: 6, y_particles: 6,
        x_squares: 0, y_squares: 0, square_in_grid_length: 0.0,
        radius: 0.0, kn: 0.0, ks: 0.0, rho: 0.0, thickness: 0.0,
        v0: 0.0, r0: 0.0,
    };
    assert_eq!(cfg.initialize(), 36);
}

fn main() {}
