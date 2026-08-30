//! The Haraka hash backend (`lib/haraka/`).

pub mod haraka;
pub mod hash;

#[cfg(feature = "simple")]
pub mod thash_simple;
#[cfg(not(feature = "simple"))]
pub mod thash_robust;

pub use haraka::*;
pub use hash::*;

#[cfg(feature = "simple")]
pub use thash_simple::thash;
#[cfg(not(feature = "simple"))]
pub use thash_robust::thash;

/// The `#ifdef SPX_HARAKA` tail of `spx_ctx` in `app/include/context.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BackendState {
    pub tweaked512_rc64: [[u64; 8]; 10],
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl BackendState {
    pub const fn new() -> Self {
        BackendState {
            tweaked512_rc64: [[0u64; 8]; 10],
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}

impl Default for BackendState {
    fn default() -> Self {
        Self::new()
    }
}
