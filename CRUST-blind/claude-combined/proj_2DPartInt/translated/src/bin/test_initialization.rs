use twoDPartInt::config::Config;
use twoDPartInt::initialization;

fn build_cfg(xp: i32, yp: i32) -> Config {
    Config {
        simulation_time: 1.0,
        dt: 0.00025,
        x_particles: xp,
        y_particles: yp,
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
    }
}

#[test]
fn test_initialize_basic() {
    // 6x6 grid + 1 falling particle = 37.
    let cfg = build_cfg(6, 6);
    assert_eq!(initialization::initialize(&cfg), 37);
}

#[test]
fn test_initialize_one_by_one() {
    let cfg = build_cfg(1, 1);
    assert_eq!(initialization::initialize(&cfg), 2);
}

#[test]
fn test_initialize_zero_particles() {
    let cfg = build_cfg(0, 0);
    assert_eq!(initialization::initialize(&cfg), 1);
}

#[test]
fn test_initialize_rectangular() {
    let cfg = build_cfg(4, 5);
    assert_eq!(initialization::initialize(&cfg), 21);
}

fn main() {}
