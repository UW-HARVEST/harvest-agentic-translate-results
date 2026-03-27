// params.rs - SPHINCS+ parameter constants, gated by Cargo features
//
// SPX_NAMESPACE(s) => SPX_##s  -- all params headers define this identically.
// The Rust equivalent is handled by #[unsafe(no_mangle)] with the SPX_ prefix.

// ============================================================
// Security-parameter-dependent constants
// ============================================================

// SPX_N: hash output length in bytes
#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_N: usize = 16;
#[cfg(any(feature = "192s", feature = "192f"))]
pub const SPX_N: usize = 24;
#[cfg(any(feature = "256s", feature = "256f"))]
pub const SPX_N: usize = 32;

// SPX_FULL_HEIGHT
#[cfg(feature = "128f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "128s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "192f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "192s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "256f")]
pub const SPX_FULL_HEIGHT: usize = 68;
#[cfg(feature = "256s")]
pub const SPX_FULL_HEIGHT: usize = 64;

// SPX_D
#[cfg(feature = "128f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "128s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "192f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "192s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "256f")]
pub const SPX_D: usize = 17;
#[cfg(feature = "256s")]
pub const SPX_D: usize = 8;

// SPX_FORS_HEIGHT
#[cfg(feature = "128f")]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(feature = "128s")]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(feature = "192f")]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(feature = "192s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "256f")]
pub const SPX_FORS_HEIGHT: usize = 9;
#[cfg(feature = "256s")]
pub const SPX_FORS_HEIGHT: usize = 14;

// SPX_FORS_TREES
#[cfg(feature = "128f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "128s")]
pub const SPX_FORS_TREES: usize = 14;
#[cfg(feature = "192f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "192s")]
pub const SPX_FORS_TREES: usize = 17;
#[cfg(feature = "256f")]
pub const SPX_FORS_TREES: usize = 35;
#[cfg(feature = "256s")]
pub const SPX_FORS_TREES: usize = 22;

// SPX_WOTS_W is always 16 for all parameter sets
pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;

pub const SPX_ADDR_BYTES: usize = 32;

// SPX_WOTS_LEN1 = 8 * SPX_N / SPX_WOTS_LOGW
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2: precomputed for W=16
// N <= 8 => 2, N <= 136 => 3, N <= 256 => 4
#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_WOTS_LEN2: usize = 3; // N=16, 16 <= 136
#[cfg(any(feature = "192s", feature = "192f"))]
pub const SPX_WOTS_LEN2: usize = 3; // N=24, 24 <= 136
#[cfg(any(feature = "256s", feature = "256f"))]
pub const SPX_WOTS_LEN2: usize = 3; // N=32, 32 <= 136

pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

// SPX_TREE_HEIGHT = SPX_FULL_HEIGHT / SPX_D
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

// FORS parameters
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

// Resulting SPX sizes
pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// CRYPTO_ aliases
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// ============================================================
// Address offsets - depend on hash backend
// ============================================================

// SHA2 uses compact offsets; blake/haraka/shake use the same wider offsets
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_LAYER: usize = 0;
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_TREE: usize = 1;
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_TYPE: usize = 9;
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_KP_ADDR: usize = 10;
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 17;
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_HASH_ADDR: usize = 21;
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_TREE_HGT: usize = 17;
#[cfg(feature = "sha2")]
pub const SPX_OFFSET_TREE_INDEX: usize = 18;

#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_LAYER: usize = 3;
#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_TREE: usize = 8;
#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_TYPE: usize = 19;
#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_KP_ADDR: usize = 20;
#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_HASH_ADDR: usize = 31;
#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_TREE_HGT: usize = 27;
#[cfg(not(feature = "sha2"))]
pub const SPX_OFFSET_TREE_INDEX: usize = 28;

// Backend-specific defines used by hash_blake.c / thash_blake_*.c
// SPX_BLAKE512: 0 for N<24, 1 for N>=24
#[cfg(feature = "blake")]
pub const SPX_BLAKE512: bool = SPX_N >= 24;

// SPX_SHA512: 0 for N<24, 1 for N>=24
#[cfg(feature = "sha2")]
pub const SPX_SHA512: bool = SPX_N >= 24;

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// SPX_TREE_BITS and related for hash_message
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// CRYPTO_ALGNAME
pub const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";
