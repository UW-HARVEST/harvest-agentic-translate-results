// SPHINCS+ parameter sets

// --- N (security parameter) ---
#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_N: usize = 16;
#[cfg(any(feature = "192s", feature = "192f"))]
pub const SPX_N: usize = 24;
#[cfg(any(feature = "256s", feature = "256f"))]
pub const SPX_N: usize = 32;

// --- FULL_HEIGHT ---
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

// --- D ---
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

// --- FORS_HEIGHT ---
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

// --- FORS_TREES ---
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

// --- Fixed constants ---
pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_ADDR_BYTES: usize = 32;

// --- Derived WOTS constants ---
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_WOTS_LEN2: usize = 3;
#[cfg(any(feature = "192s", feature = "192f"))]
pub const SPX_WOTS_LEN2: usize = 3;
#[cfg(any(feature = "256s", feature = "256f"))]
pub const SPX_WOTS_LEN2: usize = 3;

pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

// --- Tree constants ---
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

// --- FORS derived constants ---
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

// --- Signature and key sizes ---
pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// --- Crypto API sizes ---
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// --- Tree/leaf bit and byte sizes ---
pub const SPX_TREE_BITS: usize = SPX_FULL_HEIGHT - SPX_TREE_HEIGHT;
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// --- Address offsets (sha2 backend) ---
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

// --- Address offsets (shake/blake/haraka backends) ---
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

// --- SHA2-specific constants ---
#[cfg(feature = "sha2")]
pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
#[cfg(feature = "sha2")]
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
#[cfg(feature = "sha2")]
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
#[cfg(feature = "sha2")]
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
#[cfg(feature = "sha2")]
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

#[cfg(all(feature = "sha2", any(feature = "128s", feature = "128f")))]
pub const SPX_SHA512: bool = false;
#[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_SHA512: bool = true;

// --- BLAKE-specific constants ---
#[cfg(feature = "blake")]
pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
#[cfg(feature = "blake")]
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

#[cfg(all(feature = "blake", any(feature = "128s", feature = "128f")))]
pub const SPX_BLAKE512: bool = false;
#[cfg(all(feature = "blake", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_BLAKE512: bool = true;
