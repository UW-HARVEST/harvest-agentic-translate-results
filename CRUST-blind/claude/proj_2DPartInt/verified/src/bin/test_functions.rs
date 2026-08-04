use twoDPartInt::data;
use twoDPartInt::functions;

const TOL: f64 = 0.00005;

fn assert_close(a: f64, b: f64, msg: &str) {
    assert!(
        (a - b).abs() <= TOL,
        "{}: {} != {}",
        msg,
        a,
        b
    );
}

fn make_particle(x: f64, y: f64, r: f64, idx: i32) -> data::Particle {
    data::Particle {
        x_coordinate: x,
        y_coordinate: y,
        radius: r,
        next: None,
        idx,
    }
}

#[test]
fn test_compute_distance_basic() {
    let p1 = make_particle(0.0, 0.0, 1.0, 0);
    let p2 = make_particle(3.0, 4.0, 1.0, 1);
    let d = functions::compute_distance(&p1, &p2);
    assert_close(d, 5.0, "distance (0,0)-(3,4) should be 5");
}

#[test]
fn test_compute_distance_arbitrary() {
    let p1 = make_particle(1.5, 2.5, 1.0, 0);
    let p2 = make_particle(-3.5, 7.0, 1.0, 1);
    let d = functions::compute_distance(&p1, &p2);
    assert_close(d, 6.726812023536855, "distance (1.5,2.5)-(-3.5,7.0)");
}

#[test]
fn test_compute_distance_same_position() {
    let p1 = make_particle(5.0, 5.0, 1.0, 0);
    let p2 = make_particle(5.0, 5.0, 1.0, 1);
    let d = functions::compute_distance(&p1, &p2);
    assert_close(d, 0.0, "distance same position");
}

#[test]
fn test_compute_overlap_overlapping() {
    let p1 = make_particle(0.0, 0.0, 50.0, 0);
    let p2 = make_particle(10.0, 0.0, 50.0, 1);
    let o = functions::compute_overlap(&p1, &p2);
    assert_close(o, 90.0, "overlap when overlapping");
}

#[test]
fn test_compute_overlap_far() {
    let p1 = make_particle(0.0, 0.0, 50.0, 0);
    let p2 = make_particle(200.0, 0.0, 50.0, 1);
    let o = functions::compute_overlap(&p1, &p2);
    assert_close(o, -100.0, "overlap when far");
}

#[test]
fn test_compute_overlap_touching() {
    let p1 = make_particle(0.0, 0.0, 50.0, 0);
    let p2 = make_particle(100.0, 0.0, 50.0, 1);
    let o = functions::compute_overlap(&p1, &p2);
    assert_close(o, 0.0, "overlap when touching");
}

#[test]
fn test_apply_gravity() {
    let props = vec![
        data::ParticleProperties { mass: 1.0, kn: 0.0, ks: 0.0 },
        data::ParticleProperties { mass: 2.0, kn: 0.0, ks: 0.0 },
        data::ParticleProperties { mass: 0.5, kn: 0.0, ks: 0.0 },
    ];
    let mut forces = vec![
        data::Vector { x_component: 1.0, y_component: 1.0 },
        data::Vector { x_component: 2.0, y_component: 2.0 },
        data::Vector { x_component: 3.0, y_component: 3.0 },
    ];
    functions::apply_gravity(3, &props, &mut forces);
    assert_close(forces[0].x_component, 1.0, "gravity[0].x");
    assert_close(forces[0].y_component, -8.81, "gravity[0].y");
    assert_close(forces[1].x_component, 2.0, "gravity[1].x");
    assert_close(forces[1].y_component, -17.62, "gravity[1].y");
    assert_close(forces[2].x_component, 3.0, "gravity[2].x");
    assert_close(forces[2].y_component, -1.905, "gravity[2].y");
}

#[test]
fn test_compute_acceleration_one_element() {
    let forces = vec![data::Vector { x_component: 30.0, y_component: 30.0 }];
    let props = vec![data::ParticleProperties { mass: 3.0, kn: 0.0, ks: 0.0 }];
    let mut accelerations = vec![data::Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_acceleration(0, &props, &forces, &mut accelerations);
    assert_close(accelerations[0].x_component, 10.0, "accel[0].x");
    assert_close(accelerations[0].y_component, 10.0, "accel[0].y");
}

#[test]
fn test_compute_acceleration_multiple_elements() {
    let forces = vec![
        data::Vector { x_component: -12.58, y_component: -15.896 },
        data::Vector { x_component: 13.945, y_component: -200.826 },
        data::Vector { x_component: -543.62, y_component: -0.62 },
    ];
    let props = vec![
        data::ParticleProperties { mass: 0.367, kn: 0.0, ks: 0.0 },
        data::ParticleProperties { mass: 3.967, kn: 0.0, ks: 0.0 },
        data::ParticleProperties { mass: 0.52, kn: 0.0, ks: 0.0 },
    ];
    let mut accelerations = vec![
        data::Vector { x_component: 0.0, y_component: 0.0 },
        data::Vector { x_component: 0.0, y_component: 0.0 },
        data::Vector { x_component: 0.0, y_component: 0.0 },
    ];

    functions::compute_acceleration(0, &props, &forces, &mut accelerations);
    functions::compute_acceleration(1, &props, &forces, &mut accelerations);
    functions::compute_acceleration(2, &props, &forces, &mut accelerations);

    let expected = [
        (-34.2779, -43.31335149863761),
        (3.5152508192588856, -50.6241),
        (-1045.4231, -1.1923),
    ];
    for (i, (ex, ey)) in expected.iter().enumerate() {
        assert_close(accelerations[i].x_component, *ex, "accel.x");
        assert_close(accelerations[i].y_component, *ey, "accel.y");
    }
}

#[test]
fn test_compute_velocity_one_element() {
    let accelerations = vec![data::Vector { x_component: 42.53, y_component: -631.431 }];
    let dt = 0.00025;
    let mut velocities = vec![data::Vector { x_component: 0.0, y_component: 0.0 }];
    functions::compute_velocity(dt, 0, &accelerations, &mut velocities);
    assert_close(velocities[0].x_component, 0.010632500000000001, "vel[0].x");
    assert_close(velocities[0].y_component, -0.15785775000000002, "vel[0].y");
}

#[test]
fn test_compute_velocity_multiple_elements() {
    let accelerations = vec![
        data::Vector { x_component: -10.59, y_component: 162.35 },
        data::Vector { x_component: -53223.12, y_component: 4212.124 },
        data::Vector { x_component: 532521.2124124, y_component: 12142.512124 },
    ];
    let dt = 0.00025;
    let mut velocities = vec![
        data::Vector { x_component: 5.332, y_component: 2.123 },
        data::Vector { x_component: 7.12, y_component: 8.96 },
        data::Vector { x_component: 61.52, y_component: 1293.123 },
    ];
    functions::compute_velocity(dt, 0, &accelerations, &mut velocities);
    functions::compute_velocity(dt, 1, &accelerations, &mut velocities);
    functions::compute_velocity(dt, 2, &accelerations, &mut velocities);
    let expected = [
        (5.3293525, 2.1635875),
        (-6.18578, 10.013031),
        (194.6503031031, 1296.158628031),
    ];
    for (i, (ex, ey)) in expected.iter().enumerate() {
        assert_close(velocities[i].x_component, *ex, "vel.x");
        assert_close(velocities[i].y_component, *ey, "vel.y");
    }
}

#[test]
fn test_compute_displacement() {
    let velocities = vec![
        data::Vector { x_component: 1.0, y_component: 2.0 },
        data::Vector { x_component: -3.0, y_component: 4.0 },
    ];
    let mut disps = vec![
        data::Vector { x_component: 0.5, y_component: 0.5 },
        data::Vector { x_component: 1.0, y_component: 1.0 },
    ];
    let dt = 0.1;
    functions::compute_displacement(dt, 0, &velocities, &mut disps);
    functions::compute_displacement(dt, 1, &velocities, &mut disps);
    assert_close(disps[0].x_component, 0.6, "disp[0].x");
    assert_close(disps[0].y_component, 0.7, "disp[0].y");
    assert_close(disps[1].x_component, 0.7, "disp[1].x");
    assert_close(disps[1].y_component, 1.4, "disp[1].y");
}

#[test]
fn test_displace_particles_one_element() {
    let mut particles = vec![make_particle(0.0, 100.0, 0.0, 0)];
    let displacements = vec![data::Vector { x_component: 0.05, y_component: -0.05 }];
    functions::displace_particle(0, &displacements, &mut particles);
    assert_close(particles[0].x_coordinate, 50.0, "x");
    assert_close(particles[0].y_coordinate, 50.0, "y");
}

#[test]
fn test_displace_particles_multiple_elements() {
    let mut particles = vec![
        make_particle(0.0, 100.0, 0.0, 0),
        make_particle(111.0, 210.0, 0.0, 1),
        make_particle(10.0, -30.0, 0.0, 2),
    ];
    let displacements = vec![
        data::Vector { x_component: 0.0, y_component: 0.0 },
        data::Vector { x_component: 0.015, y_component: -0.033 },
        data::Vector { x_component: -0.015, y_component: 0.03 },
    ];
    functions::displace_particle(0, &displacements, &mut particles);
    functions::displace_particle(1, &displacements, &mut particles);
    functions::displace_particle(2, &displacements, &mut particles);

    assert_close(particles[0].x_coordinate, 0.0, "p0.x");
    assert_close(particles[0].y_coordinate, 100.0, "p0.y");
    assert_close(particles[1].x_coordinate, 126.0, "p1.x");
    assert_close(particles[1].y_coordinate, 177.0, "p1.y");
    assert_close(particles[2].x_coordinate, -5.0, "p2.x");
    assert_close(particles[2].y_coordinate, 0.0, "p2.y");
}

#[test]
fn test_fix_displacement_below_radius() {
    let mut particles = vec![make_particle(0.0, 25.0, 50.0, 0)];
    let mut velocities = vec![data::Vector { x_component: 1.0, y_component: -2.0 }];
    functions::fix_displacement(0, &mut velocities, &mut particles);
    assert_close(particles[0].y_coordinate, 50.0, "y should be reset to radius");
    assert_close(velocities[0].y_component, 0.0, "vy should be 0");
    // x should not be modified
    assert_close(particles[0].x_coordinate, 0.0, "x unchanged");
    assert_close(velocities[0].x_component, 1.0, "vx unchanged");
}

#[test]
fn test_fix_displacement_above_radius() {
    let mut particles = vec![make_particle(0.0, 100.0, 50.0, 0)];
    let mut velocities = vec![data::Vector { x_component: 3.0, y_component: 4.0 }];
    functions::fix_displacement(0, &mut velocities, &mut particles);
    assert_close(particles[0].y_coordinate, 100.0, "y unchanged");
    assert_close(velocities[0].y_component, 4.0, "vy unchanged");
}

#[test]
fn test_compute_forces_one_contact() {
    let size = 2;
    let contacts_size = 1;
    let properties = vec![
        data::ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
        data::ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 },
    ];
    let particles = vec![
        make_particle(24.9999428493601, 25.0, 50.0, 0),
        make_particle(24.7762980060664, 74.6253249615872, 50.0, 1),
    ];
    let velocities = vec![
        data::Vector { x_component: -0.00159733057834793, y_component: 0.0 },
        data::Vector { x_component: -4.64544001147225, y_component: -6.36971454859269 },
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
    let contacts = vec![data::Contact { p1_idx: 0, p2_idx: 1, overlap: 42.9554 }];
    let mut resultant = vec![
        data::Vector { x_component: 0.0, y_component: 0.0 },
        data::Vector { x_component: 0.0, y_component: 0.0 },
    ];
    functions::compute_forces(
        dt,
        size,
        contacts_size,
        &particles,
        &properties,
        &contacts,
        &velocities,
        &mut normal_forces,
        &mut tangent_forces,
        &mut resultant,
    );

    // P1: only gravity is applied (no contact targets P1 since p1->p2)
    assert_close(resultant[0].x_component, 0.0, "rf[0].x");
    assert_close(resultant[0].y_component, -0.48069, "rf[0].y");

    // P2: contact + gravity
    assert_close(resultant[1].x_component, 3.858892629808413, "rf[1].x");
    assert_close(resultant[1].y_component, 92.073599462939384, "rf[1].y");

    // nf[2] and tf[2] are NOT touched by this contact (contact stores forces at index p1*size + p2 = 1)
    assert_close(normal_forces[2], 53.2634277334533, "nf[2] unchanged");
    assert_close(tangent_forces[2], 2.0526062679878, "tf[2] unchanged");

    // The forces actually updated by the contact are at index p1*size + p2 = 0*2 + 1 = 1
    assert_close(normal_forces[1], 92.535959016287933, "nf[1]");
    assert_close(tangent_forces[1], 4.275960623514133, "tf[1]");
}

#[test]
fn test_compute_forces_multiple_contacts() {
    let size = 9;
    let contacts_size = 3;
    let mut properties = vec![];
    for _ in 0..9 {
        properties.push(data::ParticleProperties {
            mass: 0.049,
            kn: 247435.829652697,
            ks: 19033.5253578998,
        });
    }
    let particles = vec![
        make_particle(24.9996682317, 25.0, 50.0, 0),
        make_particle(24.3329247490, 74.1788246441, 50.0, 1),
        make_particle(20.8181703235, 122.9449251651, 50.0, 2),
        make_particle(16.8606509998, 172.4918861911, 50.0, 3),
        make_particle(75.0003317683, 25.0, 50.0, 4),
        make_particle(75.6670752510, 74.1788246441, 50.0, 5),
        make_particle(79.1818296765, 122.9449251651, 50.0, 6),
        make_particle(83.1393490002, 172.4918861911, 50.0, 7),
        make_particle(50.0, 75.7713467697, 50.0, 8),
    ];
    let velocities = vec![
        data::Vector { x_component: -0.00728767793878, y_component: 0.0 },
        data::Vector { x_component: -10.44576385514790, y_component: -9.61529833378254 },
        data::Vector { x_component: -22.72352179173930, y_component: -6.11340796404052 },
        data::Vector { x_component: -22.22735951506210, y_component: -4.22438903830371 },
        data::Vector { x_component: 0.00728767793878, y_component: 0.0 },
        data::Vector { x_component: 10.44576385514790, y_component: -9.61529833378254 },
        data::Vector { x_component: 22.72352179173930, y_component: -6.11340796404052 },
        data::Vector { x_component: 22.22735951506210, y_component: -4.22438903830371 },
        data::Vector { x_component: -0.00000000000001, y_component: -259.77483981875700 },
    ];
    let mut normal_forces: Vec<f64> = vec![
        0.0, 143.165149, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        201.763569, 0.0, 297.810731, 0.0, 0.0, 0.0, 0.0, 0.0, 6205.981479,
        0.0, 297.810731, 0.0, 81.040751, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 81.040751, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 143.165149, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 143.165149, 0.0, 297.810731, 0.0, 6205.981479,
        0.0, 0.0, 0.0, 0.0, 0.0, 297.810731, 0.0, 81.040751, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 81.040751, 0.0, 0.0,
        0.0, 6205.981479, 0.0, 0.0, 0.0, 6205.981479, 0.0, 0.0, 0.0,
    ];
    let mut tangent_forces: Vec<f64> = vec![
        0.0, 7.77476, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        12.803354, 0.0, 61.408871, 0.0, 0.0, 0.0, 0.0, 0.0, -553.7956,
        0.0, 61.40887, 0.0, 46.788899128, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 46.788899, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, -7.77476, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, -7.7747607, 0.0, -61.408871, 0.0, 553.7956,
        0.0, 0.0, 0.0, 0.0, 0.0, -61.4089, 0.0, -46.78890, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -46.788899, 0.0, 0.0,
        0.0, -553.79557, 0.0, 0.0, 0.0, 553.7956, 0.0, 0.0, 0.0,
    ];
    let dt = 0.000025;
    let contacts = vec![
        data::Contact { p1_idx: 1, p2_idx: 0, overlap: 49.18334413 },
        data::Contact { p1_idx: 1, p2_idx: 2, overlap: 48.89259718 },
        data::Contact { p1_idx: 1, p2_idx: 8, overlap: 25.71643207 },
    ];
    let mut resultant = vec![data::Vector { x_component: 0.0, y_component: 0.0 }; 9];

    functions::compute_forces(
        dt,
        size,
        contacts_size,
        &particles,
        &properties,
        &contacts,
        &velocities,
        &mut normal_forces,
        &mut tangent_forces,
        &mut resultant,
    );

    // From C output:
    // multi_rf[0]=(-14.300766831362052, -261.060489208139870)
    // multi_rf[1]=(0.000000000000000, -0.480690000000000)
    // multi_rf[2]=(47.479456276423605, 274.388316374699400)
    // multi_rf[8]=(6183.675608651633411, 1057.391870344235940)
    assert_close(resultant[0].x_component, -14.300766831362052, "rf[0].x");
    assert_close(resultant[0].y_component, -261.060489208139870, "rf[0].y");
    assert_close(resultant[1].x_component, 0.0, "rf[1].x");
    assert_close(resultant[1].y_component, -0.48069, "rf[1].y");
    assert_close(resultant[2].x_component, 47.479456276423605, "rf[2].x");
    assert_close(resultant[2].y_component, 274.388316374699400, "rf[2].y");
    assert_close(resultant[8].x_component, 6183.675608651633411, "rf[8].x");
    assert_close(resultant[8].y_component, 1057.391870344235940, "rf[8].y");

    // P3,4,5,6,7 only have gravity applied
    for &i in &[3usize, 4, 5, 6, 7] {
        assert_close(resultant[i].x_component, 0.0, "rf only gravity x");
        assert_close(resultant[i].y_component, -0.48069, "rf only gravity y");
    }

    // Indices that are NOT updated (contacts go p1->p2, only update p1*size+p2)
    // Contacts: (1,0), (1,2), (1,8) => updates indices 1*9+0=9, 1*9+2=11, 1*9+8=17
    // Original test asserts the Pre-existing values at nf[1], nf[19], nf[73] are unchanged
    assert_close(normal_forces[1], 143.165149, "nf[1] unchanged");
    assert_close(tangent_forces[1], 7.77476, "tf[1] unchanged");
    assert_close(normal_forces[19], 297.810731, "nf[19] unchanged");
    assert_close(tangent_forces[19], 61.40887, "tf[19] unchanged");
    assert_close(normal_forces[73], 6205.981479, "nf[73] unchanged");
    assert_close(tangent_forces[73], -553.79557, "tf[73] unchanged");
}

#[test]
fn test_size_triangular_matrix() {
    // size_triangular_matrix function exists in Rust and follows the formula n*(n-1)/2
    // The C header declares this but doesn't define it; test that the Rust impl is consistent.
    assert_eq!(functions::size_triangular_matrix(0), 0);
    assert_eq!(functions::size_triangular_matrix(1), 0);
    assert_eq!(functions::size_triangular_matrix(2), 1);
    assert_eq!(functions::size_triangular_matrix(3), 3);
    assert_eq!(functions::size_triangular_matrix(4), 6);
    assert_eq!(functions::size_triangular_matrix(5), 10);
    assert_eq!(functions::size_triangular_matrix(10), 45);
}

fn main() {}
