// SPHINCS+ blake-256f parameters
pub const SPX_N: usize = 32;
pub const SPX_FULL_HEIGHT: usize = 68;
pub const SPX_D: usize = 17;
pub const SPX_FORS_HEIGHT: usize = 9;
pub const SPX_FORS_TREES: usize = 35;
pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 64
pub const SPX_WOTS_LEN2: usize = 3; // precomputed for W=16, N=32
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 67
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 4

pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;
pub const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES; // SPX_N >= 24

// Address offsets for BLAKE
pub const SPX_OFFSET_LAYER: usize = 3;
pub const SPX_OFFSET_TREE: usize = 8;
pub const SPX_OFFSET_TYPE: usize = 19;
pub const SPX_OFFSET_KP_ADDR: usize = 20;
pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
pub const SPX_OFFSET_HASH_ADDR: usize = 31;
pub const SPX_OFFSET_TREE_HGT: usize = 27;
pub const SPX_OFFSET_TREE_INDEX: usize = 28;

// Address types
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// Derived constants for hash_message
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 64
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 8
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 4
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 1
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
