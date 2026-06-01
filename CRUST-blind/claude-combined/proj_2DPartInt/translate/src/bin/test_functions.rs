use twoDPartInt::data::{Contact, Particle, ParticleProperties, Vector};
use twoDPartInt::functions;

const TOL: f64 = 0.00005;

fn approx(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= TOL,
        "expected {} got {} (|diff|={})",
        b,
        a,
        (a - b).abs()
    );
}

fn make_particle(x: f64, y: f64, r: f64, idx: i32) -> Particle {
    Particle {
        x_coordinate: x,
        y_coordinate: y,
        radius: r,
        next: None,
        idx,
    }
}

#[test]
fn test_compute_distance_basic() {
    let p1 = make_particle(0.0, 0.0, 50.0, 0);
    let p2 = make_particle(3.0, 4.0, 50.0, 1);
    approx(functions::compute_distance(&p1, &p2), 5.0);
}

#[test]
fn test_compute_distance_zero() {
    let p1 = make_particle(0.0, 0.0, 0.0, 0);
    let p2 = make_particle(0.0, 0.0, 0.0, 1);
    approx(functions::compute_distance(&p1, &p2), 0.0);
}

#[test]
fn test_compute_distance_negative_coords() {
    let p1 = make_particle(1.5, 2.5, 0.0, 0);
    let p2 = make_particle(-3.5, 7.5, 0.0, 1);
    approx(functions::compute_distance(&p1, &p2), 7.071067811865476);
}

#[test]
fn test_compute_overlap_positive() {
    let p1 = make_particle(0.0, 0.0, 50.0, 0);
    let p2 = make_particle(30.0, 40.0, 50.0, 1);
    approx(functions::compute_overlap(&p1, &p2), 50.0);
}

#[test]
fn test_compute_overlap_negative_no_contact() {
    let p1 = make_particle(0.0, 0.0, 50.0, 0);
    let p2 = make_particle(200.0, 0.0, 50.0, 1);
    approx(functions::compute_overlap(&p1, &p2), -100.0);
}

#[test]
fn test_compute_overlap_realistic() {
    let p1 = make_particle(24.9999428493601, 25.0, 50.0, 0);
    let p2 = make_particle(24.7762980060664, 74.6253249615872, 50.0, 1);
    approx(functions::compute_overlap(&p1, &p2), 50.374171094488901);
}

#[test]
fn test_size_triangular_matrix() {
    assert_eq!(functions::size_triangular_matrix(0), 0);
    assert_eq!(functions::size_triangular_matrix(1), 0);
    assert_eq!(functions::size_triangular_matrix(2), 1);
    assert_eq!(functions::size_triangular_matrix(3), 3);
    assert_eq!(functions::size_triangular_matrix(4), 6);
    assert_eq!(functions::size_triangular_matrix(5), 10);
    assert_eq!(functions::size_triangular_matrix(6), 15);
    assert_eq!(functions::size_triangular_matrix(7), 21);
    assert_eq!(functions::size_triangular_matrix(8), 28);
    assert_eq!(functions::size_triangular_matrix(9), 36);
}

#[test]
fn test_apply_gravity() {
    let properties = vec![
        ParticleProperties { mass: 0.5, kn: 0.0, ks: 0.0 },
        ParticleProperties { mass: 1.0, kn: 0.0, ks: 0.0 },
        ParticleProperties { mass: 2.0, kn: 0.0, ks: 0.0 },
    ];
    let mut forces = vec![
        Vector { x_component: 1.0, y_component: 2.0 },
        Vector { x_component: 3.0, y_component: 4.0 },
        Vector { x_component: 5.0, y_component: 6.0 },
    ];
    functions::apply_gravity(3, &properties, &mut forces);
    approx(forces[0].x_component, 1.0);
    approx(forces[0].y_component, -2.905);
    approx(forces[1].x_component, 3.0);
    approx(forces[1].y_component, -5.81);
    approx(forces[2].x_component, 5.0);
    approx(forces[2].y_component, -13.62);
}

#[test]
fn test_apply_gravity_size_zero() {
    let properties = vec![ParticleProperties { mass: 0.5, kn: 0.0, ks: 0.0 }];
    let mut forces = vec![Vector { x_component: 1.0, y_component: 2.0 }];
    functions::apply_gravity(0, &properties, &mut forces);
    // Nothing modified.
    approx(forces[0].x_component, 1.0);
    approx(forces[0].y_component, 2.0);
}

#[test]
fn test_compute_acceleration_one_element() {
    let forces = vec![Vector { x_component: 30.0, y_component: 30.0 }];
    let particle_properties = vec![ParticleProperties { mass: 3.0, kn: 0.0, ks: 0.0 }];
    let mut accelerations = vec![Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_acceleration(0, &particle_properties, &forces, &mut accelerations);
    approx(accelerations[0].x_component, 10.0);
    approx(accelerations[0].y_component, 10.0);
}

#[test]
fn test_compute_acceleration_multiple_elements() {
    let forces = vec![
        Vector { x_component: -12.58, y_component: -15.896 },
        Vector { x_component: 13.945, y_component: -200.826 },
        Vector { x_component: -543.62, y_component: -0.62 },
    ];
    let particle_properties = vec![
        ParticleProperties { mass: 0.367, kn: 0.0, ks: 0.0 },
        ParticleProperties { mass: 3.967, kn: 0.0, ks: 0.0 },
        ParticleProperties { mass: 0.52, kn: 0.0, ks: 0.0 },
    ];
    let mut accelerations = vec![
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.0, y_component: 0.0 },
    ];
    functions::compute_acceleration(0, &particle_properties, &forces, &mut accelerations);
    functions::compute_acceleration(1, &particle_properties, &forces, &mut accelerations);
    functions::compute_acceleration(2, &particle_properties, &forces, &mut accelerations);
    approx(accelerations[0].x_component, -34.2779);
    approx(accelerations[0].y_component, -43.31335149863761);
    approx(accelerations[1].x_component, 3.5152508192588856);
    approx(accelerations[1].y_component, -50.6241);
    approx(accelerations[2].x_component, -1045.4231);
    approx(accelerations[2].y_component, -1.1923);
}

#[test]
fn test_compute_velocity_one_element() {
    let accelerations = vec![Vector { x_component: 42.53, y_component: -631.431 }];
    let dt = 0.00025;
    let mut velocities = vec![Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_velocity(dt, 0, &accelerations, &mut velocities);
    approx(velocities[0].x_component, 0.010632500000000001);
    approx(velocities[0].y_component, -0.15785775000000002);
}

#[test]
fn test_compute_velocity_multiple_elements() {
    let accelerations = vec![
        Vector { x_component: -10.59, y_component: 162.35 },
        Vector { x_component: -53223.12, y_component: 4212.124 },
        Vector { x_component: 532521.2124124, y_component: 12142.512124 },
    ];
    let dt = 0.00025;
    let mut velocities = vec![
        Vector { x_component: 5.332, y_component: 2.123 },
        Vector { x_component: 7.12, y_component: 8.96 },
        Vector { x_component: 61.52, y_component: 1293.123 },
    ];
    functions::compute_velocity(dt, 0, &accelerations, &mut velocities);
    functions::compute_velocity(dt, 1, &accelerations, &mut velocities);
    functions::compute_velocity(dt, 2, &accelerations, &mut velocities);
    approx(velocities[0].x_component, 5.3293525);
    approx(velocities[0].y_component, 2.1635875);
    approx(velocities[1].x_component, -6.18578);
    approx(velocities[1].y_component, 10.013031);
    approx(velocities[2].x_component, 194.6503031031);
    approx(velocities[2].y_component, 1296.158628031);
}

#[test]
fn test_compute_displacement() {
    let velocities = vec![
        Vector { x_component: 1.0, y_component: 2.0 },
        Vector { x_component: 3.0, y_component: 4.0 },
        Vector { x_component: 5.0, y_component: 6.0 },
    ];
    let mut displacements = vec![
        Vector { x_component: 0.1, y_component: 0.2 },
        Vector { x_component: 0.3, y_component: 0.4 },
        Vector { x_component: 0.5, y_component: 0.6 },
    ];
    let dt = 0.001;
    for i in 0..3 {
        functions::compute_displacement(dt, i, &velocities, &mut displacements);
    }
    approx(displacements[0].x_component, 0.101);
    approx(displacements[0].y_component, 0.202);
    approx(displacements[1].x_component, 0.303);
    approx(displacements[1].y_component, 0.404);
    approx(displacements[2].x_component, 0.505);
    approx(displacements[2].y_component, 0.606);
}

#[test]
fn test_displace_particles_one_element() {
    let mut particles = vec![make_particle(0.0, 100.0, 0.0, 0)];
    let displacements = vec![Vector { x_component: 0.05, y_component: -0.05 }];
    functions::displace_particle(0, &displacements, &mut particles);
    approx(particles[0].x_coordinate, 50.0);
    approx(particles[0].y_coordinate, 50.0);
}

#[test]
fn test_displace_particles_multiple_elements() {
    let mut particles = vec![
        make_particle(0.0, 100.0, 0.0, 0),
        make_particle(111.0, 210.0, 0.0, 1),
        make_particle(10.0, -30.0, 0.0, 2),
    ];
    let displacements = vec![
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.015, y_component: -0.033 },
        Vector { x_component: -0.015, y_component: 0.03 },
    ];
    for i in 0..3 {
        functions::displace_particle(i, &displacements, &mut particles);
    }
    approx(particles[0].x_coordinate, 0.0);
    approx(particles[0].y_coordinate, 100.0);
    approx(particles[1].x_coordinate, 126.0);
    approx(particles[1].y_coordinate, 177.0);
    approx(particles[2].x_coordinate, -5.0);
    approx(particles[2].y_coordinate, 0.0);
}

#[test]
fn test_fix_displacement_below_floor() {
    let mut particles = vec![make_particle(0.0, 5.0, 10.0, 0)];
    let mut velocities = vec![Vector { x_component: 1.0, y_component: -3.0 }];
    functions::fix_displacement(0, &mut velocities, &mut particles);
    approx(particles[0].y_coordinate, 10.0);
    approx(velocities[0].y_component, 0.0);
    approx(velocities[0].x_component, 1.0);
}

#[test]
fn test_fix_displacement_above_floor() {
    let mut particles = vec![make_particle(0.0, 25.0, 10.0, 0)];
    let mut velocities = vec![Vector { x_component: 1.0, y_component: -3.0 }];
    functions::fix_displacement(0, &mut velocities, &mut particles);
    approx(particles[0].y_coordinate, 25.0);
    approx(velocities[0].y_component, -3.0);
    approx(velocities[0].x_component, 1.0);
}

#[test]
fn test_fix_displacement_at_floor() {
    let mut particles = vec![make_particle(0.0, 10.0, 10.0, 0)];
    let mut velocities = vec![Vector { x_component: 1.0, y_component: -3.0 }];
    functions::fix_displacement(0, &mut velocities, &mut particles);
    // diff = 10 - 10 = 0, not < 0, so no change.
    approx(particles[0].y_coordinate, 10.0);
    approx(velocities[0].y_component, -3.0);
}

#[test]
fn test_compute_forces_one_contact() {
    let size = 2usize;
    let contacts_size = 1usize;

    let properties = vec![
        ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
        ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
    ];
    let particles = vec![
        make_particle(24.9999428493601, 25.0, 50.0, 0),
        make_particle(24.7762980060664, 74.6253249615872, 50.0, 1),
    ];
    let velocities = vec![
        Vector { x_component: -0.00159733057834793, y_component: 0.0 },
        Vector { x_component: -4.64544001147225, y_component: -6.36971454859269 },
    ];
    let mut normal_forces = vec![
        0.0, 53.2634277334533,
        53.2634277334533, 0.0,
    ];
    let mut tangent_forces = vec![
        0.0, 2.0526062679878,
        2.0526062679878, 0.0,
    ];
    let dt = 0.000025;
    let contacts = vec![Contact { p1_idx: 0, p2_idx: 1, overlap: 42.9554 }];
    let mut forces = vec![
        Vector { x_component: 0.0, y_component: 0.0 },
        Vector { x_component: 0.0, y_component: 0.0 },
    ];

    functions::compute_forces(
        dt, size, contacts_size,
        &particles, &properties, &contacts, &velocities,
        &mut normal_forces, &mut tangent_forces, &mut forces,
    );

    // Match the C executable values exactly.
    approx(forces[0].x_component, 0.0);
    approx(forces[0].y_component, -0.48069);
    approx(forces[1].x_component, 3.858892629808413);
    approx(forces[1].y_component, 92.073599462939384);
    approx(normal_forces[0], 0.0);
    approx(normal_forces[1], 92.535959016287933);
    approx(normal_forces[2], 53.2634277334533);
    approx(normal_forces[3], 0.0);
    approx(tangent_forces[0], 0.0);
    approx(tangent_forces[1], 4.275960623514133);
    approx(tangent_forces[2], 2.0526062679878);
    approx(tangent_forces[3], 0.0);
}

fn main() {}
