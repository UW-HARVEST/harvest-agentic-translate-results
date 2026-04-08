use twoDPartInt::data::{Contact, Particle, ParticleProperties, Vector};
use twoDPartInt::functions;

const TOL: f64 = 0.00005;

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < TOL, "expected {b}, got {a}");
}

fn mk_particle(x: f64, y: f64, r: f64, idx: i32) -> Particle {
    Particle { x_coordinate: x, y_coordinate: y, radius: r, next: None, idx }
}

// --- compute_distance ---

#[test]
fn test_compute_distance_nonzero() {
    let p1 = mk_particle(0.0, 0.0, 50.0, 0);
    let p2 = mk_particle(3.0, 4.0, 50.0, 1);
    approx(functions::compute_distance(&p1, &p2), 5.0);
}

#[test]
fn test_compute_distance_zero() {
    let p1 = mk_particle(0.0, 0.0, 50.0, 0);
    let p2 = mk_particle(0.0, 0.0, 50.0, 1);
    approx(functions::compute_distance(&p1, &p2), 0.0);
}

// --- compute_overlap ---

#[test]
fn test_compute_overlap_positive() {
    let p1 = mk_particle(0.0, 0.0, 50.0, 0);
    let p2 = mk_particle(30.0, 40.0, 50.0, 1);
    approx(functions::compute_overlap(&p1, &p2), 50.0);
}

#[test]
fn test_compute_overlap_zero() {
    let p1 = mk_particle(0.0, 0.0, 50.0, 0);
    let p2 = mk_particle(100.0, 0.0, 50.0, 1);
    approx(functions::compute_overlap(&p1, &p2), 0.0);
}

#[test]
fn test_compute_overlap_negative() {
    let p1 = mk_particle(0.0, 0.0, 50.0, 0);
    let p2 = mk_particle(200.0, 0.0, 50.0, 1);
    approx(functions::compute_overlap(&p1, &p2), -100.0);
}

// --- apply_gravity ---

#[test]
fn test_apply_gravity() {
    let props = [
        ParticleProperties { mass: 2.0, kn: 0.0, ks: 0.0 },
        ParticleProperties { mass: 3.0, kn: 0.0, ks: 0.0 },
    ];
    let mut forces = [
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.0, y_component: 0.0 },
    ];
    functions::apply_gravity(2, &props, &mut forces);
    approx(forces[0].y_component, -19.62);
    approx(forces[1].y_component, -29.43);
    approx(forces[0].x_component, 0.0);
    approx(forces[1].x_component, 0.0);
}

// --- size_triangular_matrix ---

#[test]
fn test_size_triangular_matrix() {
    assert_eq!(functions::size_triangular_matrix(0), 0);
    assert_eq!(functions::size_triangular_matrix(1), 0);
    assert_eq!(functions::size_triangular_matrix(5), 10);
    assert_eq!(functions::size_triangular_matrix(10), 45);
}

// --- compute_acceleration ---

#[test]
fn test_compute_acceleration_one() {
    let forces = [Vector { x_component: 30.0, y_component: 30.0 }];
    let props = [ParticleProperties { mass: 3.0, kn: 0.0, ks: 0.0 }];
    let mut accel = [Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_acceleration(0, &props, &forces, &mut accel);
    approx(accel[0].x_component, 10.0);
    approx(accel[0].y_component, 10.0);
}

#[test]
fn test_compute_acceleration_multiple() {
    let forces = [
        Vector { x_component: -12.58, y_component: -15.896 },
        Vector { x_component: 13.945, y_component: -200.826 },
        Vector { x_component: -543.62, y_component: -0.62 },
    ];
    let props = [
        ParticleProperties { mass: 0.367, kn: 0.0, ks: 0.0 },
        ParticleProperties { mass: 3.967, kn: 0.0, ks: 0.0 },
        ParticleProperties { mass: 0.52, kn: 0.0, ks: 0.0 },
    ];
    let mut accel = [Vector { x_component: 0.0, y_component: 0.0 }; 3];
    for i in 0..3 {
        functions::compute_acceleration(i, &props, &forces, &mut accel);
    }
    approx(accel[0].x_component, -34.2779);
    approx(accel[0].y_component, -43.31335149863761);
    approx(accel[1].x_component, 3.5152508192588856);
    approx(accel[1].y_component, -50.6241);
    approx(accel[2].x_component, -1045.4231);
    approx(accel[2].y_component, -1.1923);
}

// --- compute_velocity ---

#[test]
fn test_compute_velocity_one() {
    let accel = [Vector { x_component: 42.53, y_component: -631.431 }];
    let mut vels = [Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_velocity(0.00025, 0, &accel, &mut vels);
    approx(vels[0].x_component, 0.010632500000000001);
    approx(vels[0].y_component, -0.15785775000000002);
}

#[test]
fn test_compute_velocity_multiple() {
    let accel = [
        Vector { x_component: -10.59, y_component: 162.35 },
        Vector { x_component: -53223.12, y_component: 4212.124 },
        Vector { x_component: 532521.2124124, y_component: 12142.512124 },
    ];
    let mut vels = [
        Vector { x_component: 5.332, y_component: 2.123 },
        Vector { x_component: 7.12, y_component: 8.96 },
        Vector { x_component: 61.52, y_component: 1293.123 },
    ];
    for i in 0..3 {
        functions::compute_velocity(0.00025, i, &accel, &mut vels);
    }
    approx(vels[0].x_component, 5.3293525);
    approx(vels[0].y_component, 2.1635875);
    approx(vels[1].x_component, -6.18578);
    approx(vels[1].y_component, 10.013031);
    approx(vels[2].x_component, 194.6503031031);
    approx(vels[2].y_component, 1296.158628031);
}

// --- compute_displacement ---

#[test]
fn test_compute_displacement() {
    let vels = [Vector { x_component: 10.0, y_component: -20.0 }];
    let mut disps = [Vector { x_component: 1.0, y_component: 2.0 }];
    functions::compute_displacement(0.5, 0, &vels, &mut disps);
    approx(disps[0].x_component, 6.0);
    approx(disps[0].y_component, -8.0);
}

// --- displace_particle ---

#[test]
fn test_displace_particle_one() {
    let mut parts = [mk_particle(0.0, 100.0, 0.0, 0)];
    let disps = [Vector { x_component: 0.05, y_component: -0.05 }];
    functions::displace_particle(0, &disps, &mut parts);
    approx(parts[0].x_coordinate, 50.0);
    approx(parts[0].y_coordinate, 50.0);
}

#[test]
fn test_displace_particle_multiple() {
    let mut parts = [
        mk_particle(0.0, 100.0, 0.0, 0),
        mk_particle(111.0, 210.0, 0.0, 1),
        mk_particle(10.0, -30.0, 0.0, 2),
    ];
    let disps = [
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.015, y_component: -0.033 },
        Vector { x_component: -0.015, y_component: 0.03 },
    ];
    for i in 0..3 {
        functions::displace_particle(i, &disps, &mut parts);
    }
    approx(parts[0].x_coordinate, 0.0);
    approx(parts[0].y_coordinate, 100.0);
    approx(parts[1].x_coordinate, 126.0);
    approx(parts[1].y_coordinate, 177.0);
    approx(parts[2].x_coordinate, -5.0);
    approx(parts[2].y_coordinate, 0.0);
}

// --- fix_displacement ---

#[test]
fn test_fix_displacement_below_ground() {
    let mut parts = [mk_particle(0.0, 10.0, 50.0, 0)];
    let mut vels = [Vector { x_component: 5.0, y_component: -10.0 }];
    functions::fix_displacement(0, &mut vels, &mut parts);
    approx(parts[0].y_coordinate, 50.0);
    approx(vels[0].y_component, 0.0);
    approx(vels[0].x_component, 5.0); // x unchanged
}

#[test]
fn test_fix_displacement_above_ground() {
    let mut parts = [mk_particle(0.0, 100.0, 50.0, 0)];
    let mut vels = [Vector { x_component: 5.0, y_component: -10.0 }];
    functions::fix_displacement(0, &mut vels, &mut parts);
    approx(parts[0].y_coordinate, 100.0);
    approx(vels[0].y_component, -10.0);
}

// --- compute_forces ---

#[test]
fn test_compute_forces_gravity_only() {
    let props = [
        ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
        ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
    ];
    let parts = [mk_particle(0.0, 0.0, 50.0, 0), mk_particle(200.0, 0.0, 50.0, 1)];
    let vels = [
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.0, y_component: 0.0 },
    ];
    let mut nf = vec![0.0; 4];
    let mut tf = vec![0.0; 4];
    let mut rf = [
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.0, y_component: 0.0 },
    ];
    functions::compute_forces(0.000025, 2, 0, &parts, &props, &[], &vels, &mut nf, &mut tf, &mut rf);
    approx(rf[0].x_component, 0.0);
    approx(rf[0].y_component, -0.48069);
    approx(rf[1].x_component, 0.0);
    approx(rf[1].y_component, -0.48069);
}

#[test]
fn test_compute_forces_one_contact() {
    let props = [
        ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
        ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
    ];
    let parts = [
        mk_particle(24.9999428493601, 25.0, 50.0, 0),
        mk_particle(24.7762980060664, 74.6253249615872, 50.0, 1),
    ];
    let vels = [
        Vector { x_component: -0.00159733057834793, y_component: 0.0 },
        Vector { x_component: -4.64544001147225, y_component: -6.36971454859269 },
    ];
    let mut nf = vec![0.0, 53.2634277334533, 53.2634277334533, 0.0];
    let mut tf = vec![0.0, 2.0526062679878, 2.0526062679878, 0.0];
    let contacts = [Contact { p1_idx: 0, p2_idx: 1, overlap: 42.9554 }];
    let mut rf = [
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.0, y_component: 0.0 },
    ];
    functions::compute_forces(0.000025, 2, 1, &parts, &props, &contacts, &vels, &mut nf, &mut tf, &mut rf);
    // P0 gets only gravity
    approx(rf[0].x_component, 0.0);
    approx(rf[0].y_component, -0.48069);
    // P1 gets collision force + gravity
    approx(rf[1].x_component, 3.858892629808413);
    approx(rf[1].y_component, 92.073599462939384);
    approx(nf[1], 92.535959016287933);
    approx(tf[1], 4.275960623514133);
}

fn main() {}
