//! Build-time parameters, derived from the CMake cache variables
//! `HASH_BACKEND`, `THASH`, and `SECPAR`, which are mapped to Cargo features.
//!
//! The values here mirror the `app/params/params-sphincs-*.h` headers together
//! with the per-backend `*_offsets.h` files of the C project.
//!
//! Backend selection priority (matches the module gating in `lib.rs`):
//! `sha2` > `shake` > `blake` > `haraka` (default when no backend feature set).
//! THASH: `simple` if set, otherwise `robust`.
//! SECPAR: resolved from the six `128s/128f/192s/192f/256s/256f` features,
//! defaulting to `128s` when none are set.

// --- Backend identification -------------------------------------------------

/// True when the SHA2 backend is the active hash backend.
pub const IS_SHA2: bool = cfg!(feature = "sha2");

// --- Security-parameter resolution ------------------------------------------

const N_IS_256: bool = cfg!(feature = "256s") || cfg!(feature = "256f");
const N_IS_192: bool = cfg!(feature = "192s") || cfg!(feature = "192f");
const IS_FAST: bool =
    cfg!(feature = "128f") || cfg!(feature = "192f") || cfg!(feature = "256f");

/// Hash output length in bytes.
pub const SPX_N: usize = if N_IS_256 {
    32
} else if N_IS_192 {
    24
} else {
    16
};

/// Height of the hypertree.
pub const SPX_FULL_HEIGHT: usize = if N_IS_256 {
    if IS_FAST {
        68
    } else {
        64
    }
} else if IS_FAST {
    66
} else {
    63
};

/// Number of subtree layers.
pub const SPX_D: usize = if N_IS_256 {
    if IS_FAST {
        17
    } else {
        8
    }
} else if IS_FAST {
    22
} else {
    7
};

/// FORS tree height.
pub const SPX_FORS_HEIGHT: usize = if N_IS_256 {
    if IS_FAST {
        9
    } else {
        14
    }
} else if N_IS_192 {
    if IS_FAST {
        8
    } else {
        14
    }
} else if IS_FAST {
    6
} else {
    12
};

/// Number of FORS trees.
pub const SPX_FORS_TREES: usize = if N_IS_256 {
    if IS_FAST {
        35
    } else {
        22
    }
} else if N_IS_192 {
    if IS_FAST {
        33
    } else {
        17
    }
} else if IS_FAST {
    33
} else {
    14
};

/// Winternitz parameter.
pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;

/// For clarity.
pub const SPX_ADDR_BYTES: usize = 32;

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

/// Subtree size.
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

// FORS parameters.
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

// Resulting SPX sizes.
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// API-level sizes.
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";

// Whether the SHA2/BLAKE backends use their 512-bit variant for the larger
// (n >= 24) parameter sets.
pub const SPX_SHA512: bool = SPX_N >= 24;
pub const SPX_BLAKE512: bool = SPX_N >= 24;

// --- Address field offsets --------------------------------------------------
// SHA2 uses a compressed 22-byte address; the other backends use the full
// 32-byte address layout.

pub const SPX_OFFSET_LAYER: usize = if IS_SHA2 { 0 } else { 3 };
pub const SPX_OFFSET_TREE: usize = if IS_SHA2 { 1 } else { 8 };
pub const SPX_OFFSET_TYPE: usize = if IS_SHA2 { 9 } else { 19 };
pub const SPX_OFFSET_KP_ADDR: usize = if IS_SHA2 { 10 } else { 20 };
pub const SPX_OFFSET_CHAIN_ADDR: usize = if IS_SHA2 { 17 } else { 27 };
pub const SPX_OFFSET_HASH_ADDR: usize = if IS_SHA2 { 21 } else { 31 };
pub const SPX_OFFSET_TREE_HGT: usize = if IS_SHA2 { 17 } else { 27 };
pub const SPX_OFFSET_TREE_INDEX: usize = if IS_SHA2 { 18 } else { 28 };

// The SHA2 backend hashes a compressed 22-byte representation of the address.
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// --- Address type constants (address.h) -------------------------------------

pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// --- Derived quantities used by hash_message --------------------------------

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
