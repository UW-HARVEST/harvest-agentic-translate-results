use crate::config;

/// Initializes the size of the simulation by computing the number of
/// particles, mirroring the corresponding C helper which returns the
/// total particle count given the configuration.
pub fn initialize(cfg: &config::Config) -> usize {
    cfg.initialize()
}
