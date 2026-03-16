use crate::params::*;

/// spx_ctx holds the public seed, secret seed, and precomputed SHA-256 state.
#[repr(C)]
#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
}

impl Default for SpxCtx {
    fn default() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            state_seeded: [0u8; 40],
        }
    }
}
