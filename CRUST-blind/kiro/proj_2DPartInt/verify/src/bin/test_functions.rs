use twoDPartInt::data;
use twoDPartInt::functions;

const TOLERANCE: f64 = 0.00005;

fn assert_close(a: f64, b: f64, msg: &str) {
    assert!((a - b).abs() <= TOLERANCE, "{}: {} != {}", msg, a, b);
}

#[test]
fn test_compute_distance_simple() {
    let p1 = data::Particle { x_coordinate: 3.0, y_coordinate: 4.0, radius: 1.0, next: None, idx: 0 };
    let p2 = data::Particle { x_coordinate: 0.0, y_coordinate: 0.0, radius: 1.0, next: None, idx: 1 };
    assert_close(functions::compute_distance(&p1, &p2), 5.0, "distance(3,4->0,0)");
}

#[test]
fn test_compute_distance_decimal() {
    let p1 = data::Particle { x_coordinate: 10.5, y_coordinate: 20.3, radius: 1.0, next: None, idx: 0 };
    let p2 = data::Particle { x_coordinate: 7.2, y_coordinate: 16.1, radius: 1.0, next: None, idx: 1 };
    assert_close(functions::compute_distance(&p1, &p2), 5.3413481444294559, "distance decimal");
}

#[test]
fn test_compute_overlap_overlapping() {
    let p1 = data::Particle { x_coordinate: 0.0, y_coordinate: 0.0, radius: 50.0, next: None, idx: 0 };
    let p2 = data::Particle { x_coordinate: 80.0, y_coordinate: 0.0, radius: 50.0, next: None, idx: 1 };
    assert_close(functions::compute_overlap(&p1, &p2), 20.0, "overlap");
}

#[test]
fn test_compute_overlap_no_overlap() {
    let p1 = data::Particle { x_coordinate: 0.0, y_coordinate: 0.0, radius: 50.0, next: None, idx: 0 };
    let p2 = data::Particle { x_coordinate: 200.0, y_coordinate: 0.0, radius: 50.0, next: None, idx: 1 };
    assert_close(functions::compute_overlap(&p1, &p2), -100.0, "no overlap");
}

#[test]
fn test_size_triangular_matrix() {
    assert_eq!(functions::size_triangular_matrix(5), 10);
    assert_eq!(functions::size_triangular_matrix(1), 0);
    assert_eq!(functions::size_triangular_matrix(10), 45);
}

#[test]
fn test_apply_gravity() {
    let props = [
        data::ParticleProperties { mass: 2.0, kn: 0.0, ks: 0.0 },
        data::ParticleProperties { mass: 3.0, kn: 0.0, ks: 0.0 },
    ];
    let mut forces = [
        data::Vector { x_component: 10.0, y_component: 20.0 },
        data::Vector { x_component: 5.0, y_component: 15.0 },
    ];
    functions::apply_gravity(2, &props, &mut forces);
    assert_close(forces[0].x_component, 10.0, "gravity f0.x");
    assert_close(forces[0].y_component, 0.37999999999999901, "gravity f0.y");
    assert_close(forces[1].x_component, 5.0, "gravity f1.x");
    assert_close(forces[1].y_component, -14.43, "gravity f1.y");
}

#[test]
fn test_compute_acceleration_one_element() {
    let forces = [data::Vector { x_component: 30.0, y_component: 30.0 }];
    let props = [data::ParticleProperties { mass: 3.0, kn: 0.0, ks: 0.0 }];
    let mut accel = [data::Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_acceleration(0, &props, &forces, &mut accel);
    assert_close(accel[0].x_component, 10.0, "accel1 x");
    assert_close(accel[0].y_component, 10.0, "accel1 y");
}

#[test]
fn test_compute_acceleration_multiple() {
    let forces = [
        data::Vector { x_component: -12.58, y_component: -15.896 },
        data::Vector { x_component: 13.945, y_component: -200.826 },
        data::Vector { x_component: -543.62, y_component: -0.62 },
    ];
    let props = [
        data::ParticleProperties { mass: 0.367, kn: 0.0, ks: 0.0 },
        data::ParticleProperties { mass: 3.967, kn: 0.0, ks: 0.0 },
        data::ParticleProperties { mass: 0.52, kn: 0.0, ks: 0.0 },
    ];
    let mut accel = [data::Vector { x_component: 0.0, y_component: 0.0 }; 3];
    functions::compute_acceleration(0, &props, &forces, &mut accel);
    functions::compute_acceleration(1, &props, &forces, &mut accel);
    functions::compute_acceleration(2, &props, &forces, &mut accel);
    let expected = [
        (-34.2779, -43.31335149863761),
        (3.5152508192588856, -50.6241),
        (-1045.4231, -1.1923),
    ];
    for (i, (ex, ey)) in expected.iter().enumerate() {
        assert_close(accel[i].x_component, *ex, &format!("accel_multi x[{}]", i));
        assert_close(accel[i].y_component, *ey, &format!("accel_multi y[{}]", i));
    }
}

#[test]
fn test_compute_velocity_one_element() {
    let accel = [data::Vector { x_component: 42.53, y_component: -631.431 }];
    let mut vel = [data::Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_velocity(0.00025, 0, &accel, &mut vel);
    assert_close(vel[0].x_component, 0.010632500000000001, "vel1 x");
    assert_close(vel[0].y_component, -0.15785775000000002, "vel1 y");
}

#[test]
fn test_compute_velocity_multiple() {
    let accel = [
        data::Vector { x_component: -10.59, y_component: 162.35 },
        data::Vector { x_component: -53223.12, y_component: 4212.124 },
        data::Vector { x_component: 532521.2124124, y_component: 12142.512124 },
    ];
    let mut vel = [
        data::Vector { x_component: 5.332, y_component: 2.123 },
        data::Vector { x_component: 7.12, y_component: 8.96 },
        data::Vector { x_component: 61.52, y_component: 1293.123 },
    ];
    functions::compute_velocity(0.00025, 0, &accel, &mut vel);
    functions::compute_velocity(0.00025, 1, &accel, &mut vel);
    functions::compute_velocity(0.00025, 2, &accel, &mut vel);
    let expected = [
        (5.3293525, 2.1635875),
        (-6.18578, 10.013031),
        (194.6503031031, 1296.158628031),
    ];
    for (i, (ex, ey)) in expected.iter().enumerate() {
        assert_close(vel[i].x_component, *ex, &format!("vel_multi x[{}]", i));
        assert_close(vel[i].y_component, *ey, &format!("vel_multi y[{}]", i));
    }
}

#[test]
fn test_compute_displacement() {
    let vel = [data::Vector { x_component: 5.0, y_component: -3.0 }];
    let mut disp = [data::Vector { x_component: 1.0, y_component: 2.0 }];
    functions::compute_displacement(0.5, 0, &vel, &mut disp);
    assert_close(disp[0].x_component, 3.5, "disp x");
    assert_close(disp[0].y_component, 0.5, "disp y");
}

#[test]
fn test_displace_particle_one() {
    let disp = [data::Vector { x_component: 0.05, y_component: -0.05 }];
    let mut particles = [data::Particle { x_coordinate: 0.0, y_coordinate: 100.0, radius: 0.0, next: None, idx: 0 }];
    functions::displace_particle(0, &disp, &mut particles);
    assert_close(particles[0].x_coordinate, 50.0, "displace1 x");
    assert_close(particles[0].y_coordinate, 50.0, "displace1 y");
}

#[test]
fn test_displace_particle_multiple() {
    let disp = [
        data::Vector { x_component: 0.0, y_component: 0.0 },
        data::Vector { x_component: 0.015, y_component: -0.033 },
        data::Vector { x_component: -0.015, y_component: 0.03 },
    ];
    let mut particles = [
        data::Particle { x_coordinate: 0.0, y_coordinate: 100.0, radius: 0.0, next: None, idx: 0 },
        data::Particle { x_coordinate: 111.0, y_coordinate: 210.0, radius: 0.0, next: None, idx: 1 },
        data::Particle { x_coordinate: 10.0, y_coordinate: -30.0, radius: 0.0, next: None, idx: 2 },
    ];
    functions::displace_particle(0, &disp, &mut particles);
    functions::displace_particle(1, &disp, &mut particles);
    functions::displace_particle(2, &disp, &mut particles);
    let expected = [(0.0, 100.0), (126.0, 177.0), (-5.0, 0.0)];
    for (i, (ex, ey)) in expected.iter().enumerate() {
        assert_close(particles[i].x_coordinate, *ex, &format!("displace_multi x[{}]", i));
        assert_close(particles[i].y_coordinate, *ey, &format!("displace_multi y[{}]", i));
    }
}

#[test]
fn test_fix_displacement_below() {
    let mut particles = [data::Particle { x_coordinate: 10.0, y_coordinate: 3.0, radius: 5.0, next: None, idx: 0 }];
    let mut vel = [data::Vector { x_component: 1.0, y_component: -2.0 }];
    functions::fix_displacement(0, &mut vel, &mut particles);
    assert_close(particles[0].y_coordinate, 5.0, "fix below y");
    assert_close(vel[0].y_component, 0.0, "fix below vy");
}

#[test]
fn test_fix_displacement_above() {
    let mut particles = [data::Particle { x_coordinate: 10.0, y_coordinate: 10.0, radius: 5.0, next: None, idx: 0 }];
    let mut vel = [data::Vector { x_component: 1.0, y_component: -2.0 }];
    functions::fix_displacement(0, &mut vel, &mut particles);
    assert_close(particles[0].y_coordinate, 10.0, "fix above y");
    assert_close(vel[0].y_component, -2.0, "fix above vy");
}

#[test]
fn test_collide_two_particles() {
    let p1 = data::Particle { x_coordinate: 0.0, y_coordinate: 80.0, radius: 50.0, next: None, idx: 0 };
    let p2 = data::Particle { x_coordinate: 0.0, y_coordinate: 0.0, radius: 50.0, next: None, idx: 1 };
    let v1 = data::Vector { x_component: 0.0, y_component: 0.0 };
    let v2 = data::Vector { x_component: 0.0, y_component: 10.0 };
    let props2 = data::ParticleProperties { mass: 1.0, kn: 100000.0, ks: 50000.0 };
    let mut prev_normal = 0.0;
    let mut prev_tangent = 0.0;
    let mut force_p2 = data::Vector { x_component: 0.0, y_component: 0.0 };
    functions::collide_two_particles(0.001, 80.0, &p1, &p2, &v1, &v2, &props2, &mut prev_normal, &mut prev_tangent, &mut force_p2);
    assert_close(force_p2.x_component, 0.0, "collide fx");
    assert_close(force_p2.y_component, -1000.0, "collide fy");
    assert_close(prev_normal, 1000.0, "collide pn");
    assert_close(prev_tangent, 0.0, "collide pt");
}

#[test]
fn test_collide_two_particles_with_history() {
    let p1 = data::Particle { x_coordinate: 24.9999428493601, y_coordinate: 25.0, radius: 50.0, next: None, idx: 0 };
    let p2 = data::Particle { x_coordinate: 24.7762980060664, y_coordinate: 74.6253249615872, radius: 50.0, next: None, idx: 1 };
    let distance = functions::compute_distance(&p1, &p2);
    let v1 = data::Vector { x_component: -0.00159733057834793, y_component: 0.0 };
    let v2 = data::Vector { x_component: -4.64544001147225, y_component: -6.36971454859269 };
    let props2 = data::ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 };
    let mut prev_normal = 53.2634277334533;
    let mut prev_tangent = 2.0526062679878;
    let mut force_p2 = data::Vector { x_component: 0.0, y_component: 0.0 };
    functions::collide_two_particles(0.000025, distance, &p1, &p2, &v1, &v2, &props2, &mut prev_normal, &mut prev_tangent, &mut force_p2);
    assert_close(force_p2.x_component, 3.858892629808413, "collide_hist fx");
    assert_close(force_p2.y_component, 92.55428946293938, "collide_hist fy");
    assert_close(prev_normal, 92.535959016287933, "collide_hist pn");
    assert_close(prev_tangent, 4.2759606235141332, "collide_hist pt");
}

#[test]
fn test_compute_forces_one_contact() {
    let particles = [
        data::Particle { x_coordinate: 24.9999428493601, y_coordinate: 25.0, radius: 50.0, next: None, idx: 0 },
        data::Particle { x_coordinate: 24.7762980060664, y_coordinate: 74.6253249615872, radius: 50.0, next: None, idx: 1 },
    ];
    let properties = [
        data::ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
        data::ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
    ];
    let velocities = [
        data::Vector { x_component: -0.00159733057834793, y_component: 0.0 },
        data::Vector { x_component: -4.64544001147225, y_component: -6.36971454859269 },
    ];
    let mut normal_forces = [0.0, 53.2634277334533, 53.2634277334533, 0.0];
    let mut tangent_forces = [0.0, 2.0526062679878, 2.0526062679878, 0.0];
    let contacts = [data::Contact { p1_idx: 0, p2_idx: 1, overlap: 42.9554 }];
    let mut forces = [
        data::Vector { x_component: 0.0, y_component: 0.0 },
        data::Vector { x_component: 0.0, y_component: 0.0 },
    ];
    functions::compute_forces(0.000025, 2, 1, &particles, &properties, &contacts, &velocities, &mut normal_forces, &mut tangent_forces, &mut forces);
    assert_close(forces[0].x_component, 0.0, "cf f0.x");
    assert_close(forces[0].y_component, -0.48069000000000006, "cf f0.y");
    assert_close(forces[1].x_component, 3.858892629808413, "cf f1.x");
    assert_close(forces[1].y_component, 92.073599462939384, "cf f1.y");
    assert_close(normal_forces[1], 92.535959016287933, "cf nf[1]");
    assert_close(tangent_forces[1], 4.2759606235141332, "cf tf[1]");
}

fn main() {}
