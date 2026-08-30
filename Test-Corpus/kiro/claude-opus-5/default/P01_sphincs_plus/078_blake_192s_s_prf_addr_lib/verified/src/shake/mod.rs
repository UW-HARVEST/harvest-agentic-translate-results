//! The SHAKE-256 hash backend (`lib/shake/`).

pub mod fips202;
pub mod hash;

#[cfg(feature = "simple")]
pub mod thash_simple;
#[cfg(not(feature = "simple"))]
pub mod thash_robust;

pub use hash::*;

#[cfg(feature = "simple")]
pub use thash_simple::thash;
#[cfg(not(feature = "simple"))]
pub use thash_robust::thash;

/// `spx_ctx` has no backend-specific tail for SHAKE.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BackendState;

impl BackendState {
    pub const fn new() -> Self {
        BackendState
    }
}

impl Default for BackendState {
    fn default() -> Self {
        Self::new()
    }
}
