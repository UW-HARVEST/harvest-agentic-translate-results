use crate::data;

// tan((30 * PI) / 180).
const TAN_30_PI_180: f64 = 0.5773502691896257;

pub fn compute_velocity(
    dt: f64,
    particle_index: usize,
    accelerations: &[data::Vector],
    velocities: &mut [data::Vector],
) {
    velocities[particle_index].x_component =
        velocities[particle_index].x_component + accelerations[particle_index].x_component * dt;
    velocities[particle_index].y_component =
        velocities[particle_index].y_component + accelerations[particle_index].y_component * dt;
}

pub fn compute_forces(
    dt: f64,
    particles_size: usize,
    contacts_size: usize,
    particles: &[data::Particle],
    properties: &[data::ParticleProperties],
    contacts: &[data::Contact],
    velocities: &[data::Vector],
    normal_forces: &mut [f64],
    tangent_forces: &mut [f64],
    forces: &mut [data::Vector],
) {
    for i in 0..contacts_size {
        let p1_idx = contacts[i].p1_idx;
        let p2_idx = contacts[i].p2_idx;
        let p2_p1_idx = (p1_idx * particles_size) + p2_idx;
        let p1 = &particles[p1_idx];
        let p2 = &particles[p2_idx];
        let distance = compute_distance(p1, p2);

        // Inline collide_two_particles logic for P1 collides P2.
        let velocity_p1 = &velocities[p1_idx];
        let velocity_p2 = &velocities[p2_idx];
        let properties_p2 = &properties[p2_idx];

        let normal_x = (p1.x_coordinate - p2.x_coordinate) / distance;
        let normal_y = (p1.y_coordinate - p2.y_coordinate) / distance;

        let velocity_x_diff = velocity_p2.x_component - velocity_p1.x_component;
        let velocity_y_diff = velocity_p2.y_component - velocity_p1.y_component;
        let normal_velocity = (normal_x * velocity_x_diff) + (normal_y * velocity_y_diff);
        let tangent_velocity = (normal_y * velocity_x_diff) - (normal_x * velocity_y_diff);

        let dfn = normal_velocity * properties_p2.kn * dt;
        let dfs = tangent_velocity * properties_p2.ks * dt;

        // Forces for P2 with respect to P1.
        let mut fn_1_2 = normal_forces[p2_p1_idx] + dfn;
        let mut fs_1_2 = tangent_forces[p2_p1_idx] + dfs;

        if fn_1_2 < 0.0 {
            fn_1_2 = 0.0;
            fs_1_2 = 0.0;
        }

        let fs_1_2_max = fn_1_2 * TAN_30_PI_180;
        if fs_1_2.abs() > fs_1_2_max {
            fs_1_2 = (fs_1_2_max.abs() * fs_1_2.abs()) / fs_1_2;
        }

        // Update the forces of p2.
        forces[p2_idx].x_component += (-normal_x * fn_1_2) - (normal_y * fs_1_2);
        forces[p2_idx].y_component += (-normal_y * fn_1_2) + (normal_x * fs_1_2);

        // Update the normal and tangent forces between p1 and p2 for the next simulation step.
        normal_forces[p2_p1_idx] = fn_1_2;
        tangent_forces[p2_p1_idx] = fs_1_2;
    }
    apply_gravity(particles_size, properties, forces);
}

pub fn compute_distance(p1: &data::Particle, p2: &data::Particle) -> f64 {
    let x_diff = p1.x_coordinate - p2.x_coordinate;
    let y_diff = p1.y_coordinate - p2.y_coordinate;
    ((x_diff * x_diff) + (y_diff * y_diff)).sqrt()
}

pub fn apply_gravity(
    size: usize,
    particles_properties: &[data::ParticleProperties],
    forces: &mut [data::Vector],
) {
    for i in 0..size {
        forces[i].y_component -= particles_properties[i].mass * 9.81;
    }
}

pub fn compute_overlap(p1: &data::Particle, p2: &data::Particle) -> f64 {
    let d = p1.radius + p2.radius;
    let distance = compute_distance(p1, p2);
    d - distance
}

pub fn fix_displacement(
    particle_index: usize,
    velocities: &mut [data::Vector],
    particles: &mut [data::Particle],
) {
    let diff = particles[particle_index].y_coordinate - particles[particle_index].radius;
    if diff < 0.0 {
        particles[particle_index].y_coordinate = particles[particle_index].radius;
        velocities[particle_index].y_component = 0.0;
    }
}

pub fn compute_acceleration(
    particle_index: usize,
    particles_properties: &[data::ParticleProperties],
    forces: &[data::Vector],
    accelerations: &mut [data::Vector],
) {
    accelerations[particle_index].x_component =
        forces[particle_index].x_component / particles_properties[particle_index].mass;
    accelerations[particle_index].y_component =
        forces[particle_index].y_component / particles_properties[particle_index].mass;
}

pub fn compute_displacement(
    dt: f64,
    particle_index: usize,
    velocities: &[data::Vector],
    displacements: &mut [data::Vector],
) {
    displacements[particle_index].x_component =
        displacements[particle_index].x_component + velocities[particle_index].x_component * dt;
    displacements[particle_index].y_component =
        displacements[particle_index].y_component + velocities[particle_index].y_component * dt;
}

pub fn size_triangular_matrix(n: usize) -> usize {
    // Size of a triangular matrix without the diagonal: n * (n - 1) / 2.
    n * (n - 1) / 2
}

pub fn displace_particle(
    particle_index: usize,
    displacements: &[data::Vector],
    particles: &mut [data::Particle],
) {
    particles[particle_index].x_coordinate += displacements[particle_index].x_component * 1000.0;
    particles[particle_index].y_coordinate += displacements[particle_index].y_component * 1000.0;
}

pub fn collide_two_particles(
    dt: f64,
    distance: f64,
    p1: &data::Particle,
    p2: &data::Particle,
    velocity_p1: &data::Vector,
    velocity_p2: &data::Vector,
    _properties_p1: &data::ParticleProperties,
    properties_p2: &data::ParticleProperties,
    previous_normal: f64,
    previous_tangent: f64,
    _forces_p2: &data::Vector,
) {
    // The Rust signature here mirrors the C function but cannot mutate
    // through immutable references. The math is computed for parity with
    // the C implementation, but the results cannot be propagated back.
    let normal_x = (p1.x_coordinate - p2.x_coordinate) / distance;
    let normal_y = (p1.y_coordinate - p2.y_coordinate) / distance;

    let velocity_x_diff = velocity_p2.x_component - velocity_p1.x_component;
    let velocity_y_diff = velocity_p2.y_component - velocity_p1.y_component;
    let normal_velocity = (normal_x * velocity_x_diff) + (normal_y * velocity_y_diff);
    let tangent_velocity = (normal_y * velocity_x_diff) - (normal_x * velocity_y_diff);

    let dfn = normal_velocity * properties_p2.kn * dt;
    let dfs = tangent_velocity * properties_p2.ks * dt;

    let mut fn_1_2 = previous_normal + dfn;
    let mut fs_1_2 = previous_tangent + dfs;

    if fn_1_2 < 0.0 {
        fn_1_2 = 0.0;
        fs_1_2 = 0.0;
    }

    let fs_1_2_max = fn_1_2 * TAN_30_PI_180;
    if fs_1_2.abs() > fs_1_2_max {
        fs_1_2 = (fs_1_2_max.abs() * fs_1_2.abs()) / fs_1_2;
    }

    // Discard computed updates because the signature does not allow mutation.
    let _ = fn_1_2;
    let _ = fs_1_2;
}
