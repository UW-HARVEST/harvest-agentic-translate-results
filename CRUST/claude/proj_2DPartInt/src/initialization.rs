use crate::config;

/// Computes the total number of particles in the simulation.
/// This includes the grid of `x_particles * y_particles` particles plus
/// one additional particle that is dropped from above.
pub fn initialize(config: &config::Config) -> usize {
    (config.x_particles * config.y_particles + 1) as usize
}
