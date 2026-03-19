// Parameter sets gated by features, matching the C params headers exactly.

// --- Address offsets (hash-backend dependent) ---

#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_LAYER: usize = 3;
#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_TREE: usize = 8;
#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_TYPE: usize = 19;
#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_KP_ADDR: usize = 20;
#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_HASH_ADDR: usize = 31;
#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_TREE_HGT: usize = 27;
#[cfg(any(feature = "blake", feature = "shake", feature = "haraka"))]
pub const SPX_OFFSET_TREE_INDEX: usize = 28;

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

// --- Core parameters (secpar dependent) ---

// blake-128f / sha2-128f / shake-128f / haraka-128f
#[cfg(feature = "128f")]
pub const SPX_N: usize = 16;
#[cfg(feature = "128f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "128f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "128f")]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(feature = "128f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "128f")]
pub const SPX_WOTS_W: usize = 16;

// 128s
#[cfg(feature = "128s")]
pub const SPX_N: usize = 16;
#[cfg(feature = "128s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "128s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "128s")]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(feature = "128s")]
pub const SPX_FORS_TREES: usize = 14;
#[cfg(feature = "128s")]
pub const SPX_WOTS_W: usize = 16;

// 192f
#[cfg(feature = "192f")]
pub const SPX_N: usize = 24;
#[cfg(feature = "192f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "192f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "192f")]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(feature = "192f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "192f")]
pub const SPX_WOTS_W: usize = 16;

// 192s
#[cfg(feature = "192s")]
pub const SPX_N: usize = 24;
#[cfg(feature = "192s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "192s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "192s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "192s")]
pub const SPX_FORS_TREES: usize = 17;
#[cfg(feature = "192s")]
pub const SPX_WOTS_W: usize = 16;

// 256f
#[cfg(feature = "256f")]
pub const SPX_N: usize = 32;
#[cfg(feature = "256f")]
pub const SPX_FULL_HEIGHT: usize = 68;
#[cfg(feature = "256f")]
pub const SPX_D: usize = 17;
#[cfg(feature = "256f")]
pub const SPX_FORS_HEIGHT: usize = 9;
#[cfg(feature = "256f")]
pub const SPX_FORS_TREES: usize = 35;
#[cfg(feature = "256f")]
pub const SPX_WOTS_W: usize = 16;

// 256s
#[cfg(feature = "256s")]
pub const SPX_N: usize = 32;
#[cfg(feature = "256s")]
pub const SPX_FULL_HEIGHT: usize = 64;
#[cfg(feature = "256s")]
pub const SPX_D: usize = 8;
#[cfg(feature = "256s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "256s")]
pub const SPX_FORS_TREES: usize = 22;
#[cfg(feature = "256s")]
pub const SPX_WOTS_W: usize = 16;

// --- Derived constants ---

pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LOGW: usize = 4; // W==16 always

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// Precomputed SPX_WOTS_LEN2 for W=16
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

pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
pub const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

// Whether to use 512-bit variant for blake/sha2
#[cfg(feature = "blake")]
pub const SPX_BLAKE512: bool = SPX_N >= 24;
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

// Tree bits for hash_message
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
