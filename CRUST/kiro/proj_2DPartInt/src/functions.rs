use crate::data;

const TAN_30_PI_180: f64 = 0.5773502691896257;

pub fn compute_distance(p1: &data::Particle, p2: &data::Particle) -> f64 {
    let dx = p1.x_coordinate - p2.x_coordinate;
    let dy = p1.y_coordinate - p2.y_coordinate;
    (dx * dx + dy * dy).sqrt()
}

pub fn compute_overlap(p1: &data::Particle, p2: &data::Particle) -> f64 {
    (p1.radius + p2.radius) - compute_distance(p1, p2)
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

pub fn size_triangular_matrix(n: usize) -> usize {
    n * (n - 1) / 2
}

fn collide_inner(
    dt: f64,
    distance: f64,
    p1: &data::Particle,
    p2: &data::Particle,
    velocity_p1: &data::Vector,
    velocity_p2: &data::Vector,
    properties_p2: &data::ParticleProperties,
    previous_normal: &mut f64,
    previous_tangent: &mut f64,
    force_p2: &mut data::Vector,
) {
    let nx = (p1.x_coordinate - p2.x_coordinate) / distance;
    let ny = (p1.y_coordinate - p2.y_coordinate) / distance;

    let vx_diff = velocity_p2.x_component - velocity_p1.x_component;
    let vy_diff = velocity_p2.y_component - velocity_p1.y_component;
    let normal_velocity = nx * vx_diff + ny * vy_diff;
    let tangent_velocity = ny * vx_diff - nx * vy_diff;

    let dfn = normal_velocity * properties_p2.kn * dt;
    let dfs = tangent_velocity * properties_p2.ks * dt;

    let mut fn_val = *previous_normal + dfn;
    let mut fs_val = *previous_tangent + dfs;

    if fn_val < 0.0 {
        fn_val = 0.0;
        fs_val = 0.0;
    }

    let fs_max = fn_val * TAN_30_PI_180;
    if fs_val.abs() > fs_max {
        fs_val = (fs_max.abs() * fs_val.abs()) / fs_val;
    }

    force_p2.x_component += -nx * fn_val - ny * fs_val;
    force_p2.y_component += -ny * fn_val + nx * fs_val;

    *previous_normal = fn_val;
    *previous_tangent = fs_val;
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
        let p1 = &particles[p1_idx];
        let p2 = &particles[p2_idx];
        let distance = compute_distance(p1, p2);

        // P1 collides P2: update forces on p2
        let idx_p2_p1 = p1_idx * particles_size + p2_idx;
        collide_inner(
            dt, distance, p1, p2,
            &velocities[p1_idx], &velocities[p2_idx],
            &properties[p2_idx],
            &mut normal_forces[idx_p2_p1],
            &mut tangent_forces[idx_p2_p1],
            &mut forces[p2_idx],
        );

        // P2 collides P1: update forces on p1
        let idx_p1_p2 = p2_idx * particles_size + p1_idx;
        collide_inner(
            dt, distance, p2, p1,
            &velocities[p2_idx], &velocities[p1_idx],
            &properties[p1_idx],
            &mut normal_forces[idx_p1_p2],
            &mut tangent_forces[idx_p1_p2],
            &mut forces[p1_idx],
        );
    }
    apply_gravity(particles_size, properties, forces);
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

pub fn compute_velocity(
    dt: f64,
    particle_index: usize,
    accelerations: &[data::Vector],
    velocities: &mut [data::Vector],
) {
    velocities[particle_index].x_component += accelerations[particle_index].x_component * dt;
    velocities[particle_index].y_component += accelerations[particle_index].y_component * dt;
}

pub fn compute_displacement(
    dt: f64,
    particle_index: usize,
    velocities: &[data::Vector],
    displacements: &mut [data::Vector],
) {
    displacements[particle_index].x_component += velocities[particle_index].x_component * dt;
    displacements[particle_index].y_component += velocities[particle_index].y_component * dt;
}

pub fn displace_particle(
    particle_index: usize,
    displacements: &[data::Vector],
    particles: &mut [data::Particle],
) {
    particles[particle_index].x_coordinate += displacements[particle_index].x_component * 1000.0;
    particles[particle_index].y_coordinate += displacements[particle_index].y_component * 1000.0;
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

pub fn collide_two_particles(
    _dt: f64,
    _distance: f64,
    _p1: &data::Particle,
    _p2: &data::Particle,
    _velocity_p1: &data::Vector,
    _velocity_p2: &data::Vector,
    _properties_p1: &data::ParticleProperties,
    _properties_p2: &data::ParticleProperties,
    _previous_normal: f64,
    _previous_tangent: f64,
    _forces_p2: &data::Vector,
) {
    // This function's signature takes immutable values, so it cannot modify state.
    // The actual collision logic is inlined in compute_forces via collide_inner.
}
