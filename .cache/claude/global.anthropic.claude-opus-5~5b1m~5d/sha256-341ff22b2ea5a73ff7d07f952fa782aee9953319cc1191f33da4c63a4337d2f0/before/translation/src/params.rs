//! Translation of app/include/params.h + app/params/params-sphincs-<backend>-<secpar>.h
//! plus the backend-specific *_offsets.h headers.

// ---------------------------------------------------------------------------
// Per-security-parameter values (identical across backends)
// ---------------------------------------------------------------------------

#[cfg(feature = "256f")]
mod secpar {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: usize = 68;
    pub const SPX_D: usize = 17;
    pub const SPX_FORS_HEIGHT: usize = 9;
    pub const SPX_FORS_TREES: usize = 35;
    pub const SPX_BIG_HASH: u32 = 1;
}

#[cfg(all(feature = "256s", not(any(feature = "256f"))))]
mod secpar {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: usize = 64;
    pub const SPX_D: usize = 8;
    pub const SPX_FORS_HEIGHT: usize = 14;
    pub const SPX_FORS_TREES: usize = 22;
    pub const SPX_BIG_HASH: u32 = 1;
}

#[cfg(all(feature = "192f", not(any(feature = "256f", feature = "256s"))))]
mod secpar {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: usize = 66;
    pub const SPX_D: usize = 22;
    pub const SPX_FORS_HEIGHT: usize = 8;
    pub const SPX_FORS_TREES: usize = 33;
    pub const SPX_BIG_HASH: u32 = 1;
}

#[cfg(all(feature = "192s", not(any(feature = "256f", feature = "256s", feature = "192f"))))]
mod secpar {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: usize = 63;
    pub const SPX_D: usize = 7;
    pub const SPX_FORS_HEIGHT: usize = 14;
    pub const SPX_FORS_TREES: usize = 17;
    pub const SPX_BIG_HASH: u32 = 1;
}

#[cfg(all(feature = "128f", not(any(feature = "256f", feature = "256s", feature = "192f", feature = "192s"))))]
mod secpar {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: usize = 66;
    pub const SPX_D: usize = 22;
    pub const SPX_FORS_HEIGHT: usize = 6;
    pub const SPX_FORS_TREES: usize = 33;
    pub const SPX_BIG_HASH: u32 = 0;
}

#[cfg(all(feature = "128s", not(any(feature = "256f", feature = "256s", feature = "192f", feature = "192s", feature = "128f"))))]
mod secpar {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: usize = 63;
    pub const SPX_D: usize = 7;
    pub const SPX_FORS_HEIGHT: usize = 12;
    pub const SPX_FORS_TREES: usize = 14;
    pub const SPX_BIG_HASH: u32 = 0;
}

// Fallback (no secpar feature selected): CMake default is 128s.
#[cfg(not(any(feature = "256f", feature = "256s", feature = "192f", feature = "192s", feature = "128f", feature = "128s")))]
mod secpar {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: usize = 63;
    pub const SPX_D: usize = 7;
    pub const SPX_FORS_HEIGHT: usize = 12;
    pub const SPX_FORS_TREES: usize = 14;
    pub const SPX_BIG_HASH: u32 = 0;
}

pub use secpar::{SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES, SPX_FULL_HEIGHT, SPX_N};

/// `SPX_SHA512` from the sha2 parameter sets (1 for 192/256, 0 for 128).
pub const SPX_SHA512: u32 = secpar::SPX_BIG_HASH;
/// `SPX_BLAKE512` from the blake parameter sets (1 for 192/256, 0 for 128).
pub const SPX_BLAKE512: u32 = secpar::SPX_BIG_HASH;

pub const SPX_WOTS_W: usize = 16;

/* For clarity */
pub const SPX_ADDR_BYTES: usize = 32;

/* WOTS parameters. */
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: usize = if SPX_N <= 8 {
    2
} else if SPX_N <= 136 {
    3
} else {
    4
};
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

/* Subtree size. */
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

/* FORS parameters. */
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

/* Resulting SPX sizes. */
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// ---------------------------------------------------------------------------
// api.h
// ---------------------------------------------------------------------------

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";

// ---------------------------------------------------------------------------
// Address field offsets (backend specific)
// ---------------------------------------------------------------------------

#[cfg(feature = "sha2")]
mod offsets {
    pub const SPX_OFFSET_LAYER: usize = 0;
    pub const SPX_OFFSET_TREE: usize = 1;
    pub const SPX_OFFSET_TYPE: usize = 9;
    pub const SPX_OFFSET_KP_ADDR: usize = 10;
    pub const SPX_OFFSET_CHAIN_ADDR: usize = 17;
    pub const SPX_OFFSET_HASH_ADDR: usize = 21;
    pub const SPX_OFFSET_TREE_HGT: usize = 17;
    pub const SPX_OFFSET_TREE_INDEX: usize = 18;
}

#[cfg(not(feature = "sha2"))]
mod offsets {
    pub const SPX_OFFSET_LAYER: usize = 3;
    pub const SPX_OFFSET_TREE: usize = 8;
    pub const SPX_OFFSET_TYPE: usize = 19;
    pub const SPX_OFFSET_KP_ADDR: usize = 20;
    pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
    pub const SPX_OFFSET_HASH_ADDR: usize = 31;
    pub const SPX_OFFSET_TREE_HGT: usize = 27;
    pub const SPX_OFFSET_TREE_INDEX: usize = 28;
}

pub use offsets::*;
