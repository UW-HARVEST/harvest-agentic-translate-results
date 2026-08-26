//! Translation of `app/include/context.h`.

use crate::params::SPX_N;

/// `spx_ctx` from context.h.
///
/// The sha2 / haraka specific members are gated on the corresponding cargo
/// feature, exactly like the `#ifdef SPX_SHA2` / `#ifdef SPX_HARAKA` blocks in
/// the C header. (The `SPX_SHA512` sub-gate for `state_seeded_512` is handled
/// with the compile-time constant `params::SPX_SHA512` in the code instead, so
/// the field is always present for the sha2 backend.)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    // sha256 state that absorbed pub_seed
    #[cfg(feature = "sha2")]
    pub state_seeded: [u8; 40],
    // sha512 state that absorbed pub_seed
    #[cfg(feature = "sha2")]
    pub state_seeded_512: [u8; 72],

    // The haraka backend is also the fallback when no backend feature is given
    // (mirrors the CMake default `HASH_BACKEND=haraka`), so the gate has to
    // match the module gate used in lib.rs.
    #[cfg(any(
        feature = "haraka",
        not(any(feature = "sha2", feature = "shake", feature = "blake"))
    ))]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(any(
        feature = "haraka",
        not(any(feature = "sha2", feature = "shake", feature = "blake"))
    ))]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    /// A freshly zeroed context (C code declares `spx_ctx ctx = {0};` or fills
    /// every member before use).
    pub fn new() -> Self {
        // All-zero is a valid bit pattern for every member.
        unsafe { core::mem::zeroed() }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
