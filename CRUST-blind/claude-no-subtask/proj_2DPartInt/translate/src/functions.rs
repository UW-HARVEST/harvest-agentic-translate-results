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

        // P1 collides P2.
        let prev_normal = normal_forces[p2_p1_idx];
        let prev_tangent = tangent_forces[p2_p1_idx];
        let velocity_p1 = velocities[p1_idx];
        let velocity_p2 = velocities[p2_idx];
        let properties_p2 = properties[p2_idx];

        let (new_normal, new_tangent, force_dx, force_dy) = collide_two_particles_internal(
            dt,
            distance,
            p1,
            p2,
            &velocity_p1,
            &velocity_p2,
            &properties_p2,
            prev_normal,
            prev_tangent,
        );

        forces[p2_idx].x_component += force_dx;
        forces[p2_idx].y_component += force_dy;
        normal_forces[p2_p1_idx] = new_normal;
        tangent_forces[p2_p1_idx] = new_tangent;
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
    // A triangular matrix without the diagonal: n*(n-1)/2.
    if n == 0 {
        0
    } else {
        n * (n - 1) / 2
    }
}

pub fn displace_particle(
    particle_index: usize,
    displacements: &[data::Vector],
    particles: &mut [data::Particle],
) {
    particles[particle_index].x_coordinate += displacements[particle_index].x_component * 1000.0;
    particles[particle_index].y_coordinate += displacements[particle_index].y_component * 1000.0;
}

// Note: The signature here matches the provided signature, but since `forces_p2`
// is an immutable reference, this function cannot mutate it. This signature is
// suspect; we keep the computation logic but we cannot reflect the changes
// outside. Internal logic mirrors the C `collide_two_particles`.
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
    forces_p2: &data::Vector,
) {
    let _ = collide_two_particles_internal(
        dt,
        distance,
        p1,
        p2,
        velocity_p1,
        velocity_p2,
        properties_p2,
        previous_normal,
        previous_tangent,
    );
    let _ = forces_p2;
}

// Internal helper that returns (new_normal, new_tangent, force_delta_x, force_delta_y).
fn collide_two_particles_internal(
    dt: f64,
    distance: f64,
    p1: &data::Particle,
    p2: &data::Particle,
    velocity_p1: &data::Vector,
    velocity_p2: &data::Vector,
    properties_p2: &data::ParticleProperties,
    previous_normal: f64,
    previous_tangent: f64,
) -> (f64, f64, f64, f64) {
    let normal = data::Vector {
        x_component: (p1.x_coordinate - p2.x_coordinate) / distance,
        y_component: (p1.y_coordinate - p2.y_coordinate) / distance,
    };

    let velocity_x_diff = velocity_p2.x_component - velocity_p1.x_component;
    let velocity_y_diff = velocity_p2.y_component - velocity_p1.y_component;
    let normal_velocity =
        normal.x_component * velocity_x_diff + normal.y_component * velocity_y_diff;
    let tangent_velocity =
        normal.y_component * velocity_x_diff - normal.x_component * velocity_y_diff;

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
        // Mirrors C: Fs_1_2 = (fabs(Fs_1_2_max) * fabs(Fs_1_2)) / Fs_1_2;
        // This effectively sets |Fs_1_2| to |Fs_1_2_max| while preserving
        // the original sign (because |Fs|/Fs = sign(Fs)).
        if fs_1_2 != 0.0 {
            fs_1_2 = (fs_1_2_max.abs() * fs_1_2.abs()) / fs_1_2;
        }
    }

    let force_dx = (-normal.x_component * fn_1_2) - (normal.y_component * fs_1_2);
    let force_dy = (-normal.y_component * fn_1_2) + (normal.x_component * fs_1_2);

    (fn_1_2, fs_1_2, force_dx, force_dy)
}
