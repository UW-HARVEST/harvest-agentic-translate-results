// SPHINCS+ context structure.
// The layout depends on the hash backend (haraka, sha2, shake, blake).
// We mimic the C `spx_ctx` struct so its size and field offsets match,
// allowing FFI sharing between callers and our hash backends.

use crate::params::SPX_N;

#[cfg(feature = "haraka")]
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub tweaked512_rc64: [[u64; 8]; 10],
    pub tweaked256_rc32: [[u32; 8]; 10],
}

#[cfg(feature = "sha2")]
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub state_seeded_512: [u8; 72],
}

#[cfg(any(feature = "shake", feature = "blake"))]
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

impl SpxCtx {
    pub fn new() -> Self {
        // SAFETY: SpxCtx is plain old data; zero-initialization is valid.
        unsafe { core::mem::zeroed() }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
