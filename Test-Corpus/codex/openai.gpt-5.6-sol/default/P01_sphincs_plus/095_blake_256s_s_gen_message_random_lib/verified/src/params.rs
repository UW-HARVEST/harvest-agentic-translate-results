//! Compile-time parameters, mirroring `app/params/params-sphincs-*.h`.
//!
//! The active security level is chosen by the `spx_secpar` cfg emitted from
//! `build.rs`, and the address-field offsets depend on the active backend
//! (`spx_backend`).

// ------------------------------------------------------------------
// Base parameters per security level (from the params-*.h headers).
// ------------------------------------------------------------------

#[cfg(spx_secpar = "128s")]
mod base {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: usize = 63;
    pub const SPX_D: usize = 7;
    pub const SPX_FORS_HEIGHT: usize = 12;
    pub const SPX_FORS_TREES: usize = 14;
}
#[cfg(spx_secpar = "128f")]
mod base {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: usize = 66;
    pub const SPX_D: usize = 22;
    pub const SPX_FORS_HEIGHT: usize = 6;
    pub const SPX_FORS_TREES: usize = 33;
}
#[cfg(spx_secpar = "192s")]
mod base {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: usize = 63;
    pub const SPX_D: usize = 7;
    pub const SPX_FORS_HEIGHT: usize = 14;
    pub const SPX_FORS_TREES: usize = 17;
}
#[cfg(spx_secpar = "192f")]
mod base {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: usize = 66;
    pub const SPX_D: usize = 22;
    pub const SPX_FORS_HEIGHT: usize = 8;
    pub const SPX_FORS_TREES: usize = 33;
}
#[cfg(spx_secpar = "256s")]
mod base {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: usize = 64;
    pub const SPX_D: usize = 8;
    pub const SPX_FORS_HEIGHT: usize = 14;
    pub const SPX_FORS_TREES: usize = 22;
}
#[cfg(spx_secpar = "256f")]
mod base {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: usize = 68;
    pub const SPX_D: usize = 17;
    pub const SPX_FORS_HEIGHT: usize = 9;
    pub const SPX_FORS_TREES: usize = 35;
}

pub use base::*;

pub const SPX_WOTS_W: usize = 16;

/// Whether the SHA2 backend uses SHA-512 (only for the 192/256 bit levels).
pub const SPX_SHA512: bool = cfg!(spx_sha512);

// ------------------------------------------------------------------
// Derived parameters (identical formulae to the C headers).
// ------------------------------------------------------------------

pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LOGW: usize = 4; // SPX_WOTS_W == 16

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

const fn wots_len2() -> usize {
    // Matches the precomputed table for SPX_WOTS_W == 16.
    if SPX_N <= 8 {
        2
    } else if SPX_N <= 136 {
        3
    } else {
        4
    }
}
pub const SPX_WOTS_LEN2: usize = wots_len2();

pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// ------------------------------------------------------------------
// API-level sizes (from api.h).
// ------------------------------------------------------------------

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";

// ------------------------------------------------------------------
// Address field offsets (depend on the hash backend).
// SHA2 uses a compressed 22-byte address layout; the others use 32 bytes.
// ------------------------------------------------------------------

#[cfg(spx_backend = "sha2")]
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
#[cfg(not(spx_backend = "sha2"))]
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

// Address type constants (from address.h).
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// Derived message-hash split sizes (used by hash_message in every backend).
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
