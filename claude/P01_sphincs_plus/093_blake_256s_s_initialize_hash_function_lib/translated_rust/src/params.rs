// Parameters selected by feature flags, mirroring the C params headers.

// SPX_N: Hash output length in bytes
#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_N: usize = 16;
#[cfg(any(feature = "192s", feature = "192f"))]
pub const SPX_N: usize = 24;
#[cfg(any(feature = "256s", feature = "256f"))]
pub const SPX_N: usize = 32;

// SPX_FULL_HEIGHT: Height of the hypertree
#[cfg(feature = "128s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "128f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "192s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "192f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "256s")]
pub const SPX_FULL_HEIGHT: usize = 64;
#[cfg(feature = "256f")]
pub const SPX_FULL_HEIGHT: usize = 68;

// SPX_D: Number of subtree layers
#[cfg(feature = "128s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "128f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "192s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "192f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "256s")]
pub const SPX_D: usize = 8;
#[cfg(feature = "256f")]
pub const SPX_D: usize = 17;

// SPX_FORS_HEIGHT
#[cfg(feature = "128s")]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(feature = "128f")]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(feature = "192s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "192f")]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(feature = "256s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "256f")]
pub const SPX_FORS_HEIGHT: usize = 9;

// SPX_FORS_TREES
#[cfg(feature = "128s")]
pub const SPX_FORS_TREES: usize = 14;
#[cfg(feature = "128f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "192s")]
pub const SPX_FORS_TREES: usize = 17;
#[cfg(feature = "192f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "256s")]
pub const SPX_FORS_TREES: usize = 22;
#[cfg(feature = "256f")]
pub const SPX_FORS_TREES: usize = 35;

pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2: precomputed floor(log(len_1 * (w - 1)) / log(w)) + 1
#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_WOTS_LEN2: usize = 3; // SPX_N <= 136
#[cfg(any(feature = "192s", feature = "192f"))]
pub const SPX_WOTS_LEN2: usize = 3; // SPX_N <= 136
#[cfg(any(feature = "256s", feature = "256f"))]
pub const SPX_WOTS_LEN2: usize = 3; // SPX_N <= 136

pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;

pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;

pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";

// Address offsets depend on hash backend
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

// Backend-specific: SPX_SHA512 flag (SHA2 backend only)
// SPX_SHA512 = 1 when N >= 24 (192/256 param sets)
#[cfg(feature = "sha2")]
pub const SPX_SHA512: bool = SPX_N >= 24;

// Backend-specific: SPX_BLAKE512 flag (BLAKE backend only)
// SPX_BLAKE512 = 1 when N >= 24 (192/256 param sets)
#[cfg(feature = "blake")]
pub const SPX_BLAKE512: bool = SPX_N >= 24;

// SHA2 constants (always available since sha2_impl compiles unconditionally)
pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// SHA2 conditional: SHAX variants
#[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
#[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;

#[cfg(all(feature = "sha2", any(feature = "128s", feature = "128f")))]
pub const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
#[cfg(all(feature = "sha2", any(feature = "128s", feature = "128f")))]
pub const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

// BLAKE constants (always available since blake_impl compiles unconditionally)
pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

#[cfg(feature = "blake")]
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
pub const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
#[cfg(feature = "blake")]
#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// Derived message hash constants
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// SHAKE constants
#[cfg(feature = "shake")]
pub const SHAKE256_RATE: usize = 136;

// Haraka constants
#[cfg(feature = "haraka")]
pub const HARAKAS_RATE: usize = 32;
