//! Translation of `app/include/params.h` + `app/params/params-sphincs-<backend>-<secpar>.h`
//! + `lib/<backend>/include/<backend>_offsets.h`.
//!
//! The CMake build selected one parameter header via `-DPARAMS=sphincs-${HASH_BACKEND}-${SECPAR}`.
//! Here the same selection is done with cargo features:
//!   * backend: `haraka` (default), `sha2`, `shake`, `blake`
//!   * secpar : `128s` (default), `128f`, `192s`, `192f`, `256s`, `256f`
//!
//! To guarantee that *every* feature combination compiles, the features are
//! resolved with a fixed priority order (first one wins).

// ---------------------------------------------------------------------------
// Security-parameter dependent values (SPX_N, heights, FORS dimensions).
// Priority: 128s > 128f > 192s > 192f > 256s > 256f, default 128s.
// ---------------------------------------------------------------------------

#[cfg(any(
    feature = "128s",
    not(any(
        feature = "128f",
        feature = "192s",
        feature = "192f",
        feature = "256s",
        feature = "256f"
    ))
))]
mod secpar {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: usize = 63;
    pub const SPX_D: usize = 7;
    pub const SPX_FORS_HEIGHT: usize = 12;
    pub const SPX_FORS_TREES: usize = 14;
}

#[cfg(all(feature = "128f", not(feature = "128s")))]
mod secpar {
    pub const SPX_N: usize = 16;
    pub const SPX_FULL_HEIGHT: usize = 66;
    pub const SPX_D: usize = 22;
    pub const SPX_FORS_HEIGHT: usize = 6;
    pub const SPX_FORS_TREES: usize = 33;
}

#[cfg(all(feature = "192s", not(any(feature = "128s", feature = "128f"))))]
mod secpar {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: usize = 63;
    pub const SPX_D: usize = 7;
    pub const SPX_FORS_HEIGHT: usize = 14;
    pub const SPX_FORS_TREES: usize = 17;
}

#[cfg(all(
    feature = "192f",
    not(any(feature = "128s", feature = "128f", feature = "192s"))
))]
mod secpar {
    pub const SPX_N: usize = 24;
    pub const SPX_FULL_HEIGHT: usize = 66;
    pub const SPX_D: usize = 22;
    pub const SPX_FORS_HEIGHT: usize = 8;
    pub const SPX_FORS_TREES: usize = 33;
}

#[cfg(all(
    feature = "256s",
    not(any(feature = "128s", feature = "128f", feature = "192s", feature = "192f"))
))]
mod secpar {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: usize = 64;
    pub const SPX_D: usize = 8;
    pub const SPX_FORS_HEIGHT: usize = 14;
    pub const SPX_FORS_TREES: usize = 22;
}

#[cfg(all(
    feature = "256f",
    not(any(
        feature = "128s",
        feature = "128f",
        feature = "192s",
        feature = "192f",
        feature = "256s"
    ))
))]
mod secpar {
    pub const SPX_N: usize = 32;
    pub const SPX_FULL_HEIGHT: usize = 68;
    pub const SPX_D: usize = 17;
    pub const SPX_FORS_HEIGHT: usize = 9;
    pub const SPX_FORS_TREES: usize = 35;
}

pub use secpar::{SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES, SPX_FULL_HEIGHT, SPX_N};

/* Winternitz parameter */
pub const SPX_WOTS_W: usize = 16;

/* For clarity */
pub const SPX_ADDR_BYTES: usize = 32;

/* WOTS parameters. */
pub const SPX_WOTS_LOGW: usize = if SPX_WOTS_W == 256 { 8 } else { 4 };

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

/* SPX_WOTS_LEN2 is floor(log(len_1 * (w - 1)) / log(w)) + 1; precomputed as in C */
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

/* api.h */
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

/// `SPX_SHA512` from the sha2 parameter sets (0 for 128{s,f}, 1 otherwise).
/// Also used as `SPX_BLAKE512` for the blake parameter sets (identical rule).
pub const SPX_SHA512: bool = SPX_N >= 24;
/// `SPX_BLAKE512` from the blake parameter sets (0 for 128{s,f}, 1 otherwise).
pub const SPX_BLAKE512: bool = SPX_N >= 24;

// ---------------------------------------------------------------------------
// Address-field offsets: `lib/<backend>/include/<backend>_offsets.h`
// sha2 uses a compressed address layout, haraka/shake/blake share the plain one.
// Priority: haraka > sha2 > shake > blake, default haraka.
// ---------------------------------------------------------------------------

#[cfg(all(feature = "sha2", not(feature = "haraka")))]
mod offsets {
    /* The byte used to specify the Merkle tree layer */
    pub const SPX_OFFSET_LAYER: usize = 0;
    /* The start of the 8 byte field used to specify the tree */
    pub const SPX_OFFSET_TREE: usize = 1;
    /* The byte used to specify the hash type (reason) */
    pub const SPX_OFFSET_TYPE: usize = 9;
    /* The start of the 4 byte field used to specify the key pair address */
    pub const SPX_OFFSET_KP_ADDR: usize = 10;
    /* The byte used to specify the chain address (which Winternitz chain) */
    pub const SPX_OFFSET_CHAIN_ADDR: usize = 17;
    /* The byte used to specify the hash address (where in the Winternitz chain) */
    pub const SPX_OFFSET_HASH_ADDR: usize = 21;
    /* The byte used to specify the height of this node in the FORS or Merkle tree */
    pub const SPX_OFFSET_TREE_HGT: usize = 17;
    /* The start of the 4 byte field used to specify the node in the FORS or Merkle tree */
    pub const SPX_OFFSET_TREE_INDEX: usize = 18;
}

#[cfg(not(all(feature = "sha2", not(feature = "haraka"))))]
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

pub use offsets::{
    SPX_OFFSET_CHAIN_ADDR, SPX_OFFSET_HASH_ADDR, SPX_OFFSET_KP_ADDR, SPX_OFFSET_LAYER,
    SPX_OFFSET_TREE, SPX_OFFSET_TREE_HGT, SPX_OFFSET_TREE_INDEX, SPX_OFFSET_TYPE,
};

/* The hash types that are passed to set_type (address.h) */
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;
