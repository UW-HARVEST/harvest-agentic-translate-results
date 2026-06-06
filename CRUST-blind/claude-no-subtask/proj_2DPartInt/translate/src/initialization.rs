use crate::config;

/// Initialize all simulation data structures, returning the number of
/// initialized particles. Mirrors the C `initialize` function and uses
/// the same total particle count formula as `Config::initialize`.
pub fn initialize(config: &config::Config) -> usize {
    config.initialize()
}
