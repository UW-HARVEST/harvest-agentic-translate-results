//! Translation of `app/include/context.h`.

use crate::params::SPX_N;

/// `spx_ctx`.
///
/// The backend-specific tail of the C struct (`state_seeded` for SHA-2, the
/// tweaked round constants for Haraka, nothing for SHAKE/BLAKE) lives in
/// `backend::BackendState`.  Both are `#[repr(C)]` and the backend state is
/// never more strictly aligned than the offset `2 * SPX_N` it starts at
/// (`SPX_N` is 16, 24 or 32), so the memory layout matches the C struct.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub backend: crate::backend::BackendState,
}

impl SpxCtx {
    pub const fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            backend: crate::backend::BackendState::new(),
        }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
