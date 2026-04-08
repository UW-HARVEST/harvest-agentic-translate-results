use twoDPartInt::config::Config;
use std::io::Write;

const TOLERANCE: f64 = 0.00005;

fn assert_close(a: f64, b: f64, msg: &str) {
    assert!((a - b).abs() <= TOLERANCE, "{}: {} != {}", msg, a, b);
}

#[test]
fn test_parse_config() {
    let dir = std::env::temp_dir().join("test_config_parse");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "time=1\ndt=0.00025\nx_particles=6\ny_particles=6\nx_squares=7\ny_squares=7\nsquare_in_grid_length=120\nradius=50\nkn=2474358.297\nks=190335.254\nrho=0.00000078\nthickness=30\nv0=0\nr0=50\n").unwrap();
    let cfg = Config::parse_config(path.to_str().unwrap());
    assert_close(cfg.simulation_time, 1.0, "time");
    assert_close(cfg.dt, 0.00025, "dt");
    assert_eq!(cfg.x_particles, 6);
    assert_eq!(cfg.y_particles, 6);
    assert_eq!(cfg.x_squares, 7);
    assert_eq!(cfg.y_squares, 7);
    assert_close(cfg.square_in_grid_length, 120.0, "sq_len");
    assert_close(cfg.radius, 50.0, "radius");
    assert_close(cfg.kn, 2474358.297, "kn");
    assert_close(cfg.ks, 190335.254, "ks");
    assert_close(cfg.rho, 0.00000078, "rho");
    assert_close(cfg.thickness, 30.0, "thickness");
    assert_close(cfg.v0, 0.0, "v0");
    assert_close(cfg.r0, 50.0, "r0");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_initialize() {
    let dir = std::env::temp_dir().join("test_config_init");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "time=1\ndt=0.00025\nx_particles=6\ny_particles=6\nx_squares=7\ny_squares=7\nsquare_in_grid_length=120\nradius=50\nkn=2474358.297\nks=190335.254\nrho=0.00000078\nthickness=30\nv0=0\nr0=50\n").unwrap();
    let cfg = Config::parse_config(path.to_str().unwrap());
    assert_eq!(cfg.initialize(), 36);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_compute_mass() {
    let dir = std::env::temp_dir().join("test_config_mass");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "time=1\ndt=0.00025\nx_particles=6\ny_particles=6\nx_squares=7\ny_squares=7\nsquare_in_grid_length=120\nradius=50\nkn=2474358.297\nks=190335.254\nrho=0.00000078\nthickness=30\nv0=0\nr0=50\n").unwrap();
    let cfg = Config::parse_config(path.to_str().unwrap());
    assert_close(cfg.compute_mass(), 0.1837831702350029, "compute_mass");
    std::fs::remove_dir_all(&dir).ok();
}

fn main() {}
