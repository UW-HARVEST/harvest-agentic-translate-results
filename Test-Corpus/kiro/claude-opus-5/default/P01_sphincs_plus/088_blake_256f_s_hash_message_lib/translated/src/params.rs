//! Translation of `app/include/params.h` + `app/params/params-sphincs-*.h`
//! + `lib/*/include/*_offsets.h`.
//!
//! The CMake build selects a parameter header at configure time via the
//! `HASH_BACKEND` and `SECPAR` cache variables.  Here the same choice is made
//! with Cargo features of the same (lowercase) names.
//!
//! Because Cargo features are additive, an explicit selection has to be able
//! to override the `default` feature set (`haraka`, `robust`, `128s`).  The
//! resolution order below therefore gives the defaults the *lowest* priority.

/* ------------------------------------------------------------------ */
/* SECPAR selection: 256f > 256s > 192f > 192s > 128f > 128s(default)  */
/* ------------------------------------------------------------------ */

#[cfg(feature = "256f")]
mod secpar {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: u32 = 68;
    pub const SPX_D: u32 = 17;
    pub const SPX_FORS_HEIGHT: u32 = 9;
    pub const SPX_FORS_TREES: u32 = 35;
}

#[cfg(all(not(feature = "256f"), feature = "256s"))]
mod secpar {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: u32 = 64;
    pub const SPX_D: u32 = 8;
    pub const SPX_FORS_HEIGHT: u32 = 14;
    pub const SPX_FORS_TREES: u32 = 22;
}

#[cfg(all(not(feature = "256f"), not(feature = "256s"), feature = "192f"))]
mod secpar {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: u32 = 66;
    pub const SPX_D: u32 = 22;
    pub const SPX_FORS_HEIGHT: u32 = 8;
    pub const SPX_FORS_TREES: u32 = 33;
}

#[cfg(all(
    not(feature = "256f"),
    not(feature = "256s"),
    not(feature = "192f"),
    feature = "192s"
))]
mod secpar {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: u32 = 63;
    pub const SPX_D: u32 = 7;
    pub const SPX_FORS_HEIGHT: u32 = 14;
    pub const SPX_FORS_TREES: u32 = 17;
}

#[cfg(all(
    not(feature = "256f"),
    not(feature = "256s"),
    not(feature = "192f"),
    not(feature = "192s"),
    feature = "128f"
))]
mod secpar {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: u32 = 66;
    pub const SPX_D: u32 = 22;
    pub const SPX_FORS_HEIGHT: u32 = 6;
    pub const SPX_FORS_TREES: u32 = 33;
}

/* `128s` (the CMake default) and the "nothing selected" case. */
#[cfg(all(
    not(feature = "256f"),
    not(feature = "256s"),
    not(feature = "192f"),
    not(feature = "192s"),
    not(feature = "128f")
))]
mod secpar {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: u32 = 63;
    pub const SPX_D: u32 = 7;
    pub const SPX_FORS_HEIGHT: u32 = 12;
    pub const SPX_FORS_TREES: u32 = 14;
}

pub use secpar::*;

/* ------------------------------------------------------------------ */
/* Address field offsets (from lib/<backend>/include/<backend>_offsets.h) */
/* SHA-2 uses a compressed 22-byte address, the others the full 32-byte  */
/* ------------------------------------------------------------------ */

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

/* ------------------------------------------------------------------ */
/* Winternitz parameter (fixed at 16 in every shipped parameter set)   */
/* ------------------------------------------------------------------ */

pub const SPX_WOTS_W: u32 = 16;

/* For clarity */
pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LOGW: u32 = if SPX_WOTS_W == 256 { 8 } else { 4 };

pub const SPX_WOTS_LEN1: usize = (8 * SPX_N) / SPX_WOTS_LOGW as usize;

/* SPX_WOTS_LEN2 is floor(log(len_1 * (w - 1)) / log(w)) + 1; precomputed */
pub const SPX_WOTS_LEN2: usize = if SPX_WOTS_W == 256 {
    if SPX_N <= 1 {
        1
    } else {
        2
    }
} else if SPX_N <= 8 {
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
pub const SPX_TREE_HEIGHT: u32 = SPX_FULL_HEIGHT / SPX_D;

/* FORS parameters. */
pub const SPX_FORS_MSG_BYTES: usize =
    ((SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8) as usize;
pub const SPX_FORS_BYTES: usize =
    ((SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES) as usize * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

/* Resulting SPX sizes. */
pub const SPX_BYTES: usize = SPX_N
    + SPX_FORS_BYTES
    + SPX_D as usize * SPX_WOTS_BYTES
    + SPX_FULL_HEIGHT as usize * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

/* ------------------------------------------------------------------ */
/* Backend-specific "use the wide hash" switches                       */
/* ------------------------------------------------------------------ */

/// `SPX_SHA512` from the sha2 parameter headers (1 for N >= 24).
pub const SPX_SHA512: u32 = if SPX_N >= 24 { 1 } else { 0 };

/// `SPX_BLAKE512` from the blake parameter headers (1 for N >= 24).
pub const SPX_BLAKE512: u32 = if SPX_N >= 24 { 1 } else { 0 };

/* ------------------------------------------------------------------ */
/* api.h                                                              */
/* ------------------------------------------------------------------ */

pub const CRYPTO_ALGNAME: &str = "SPHINCS+";
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

/* ------------------------------------------------------------------ */
/* address.h hash types                                               */
/* ------------------------------------------------------------------ */

pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;
