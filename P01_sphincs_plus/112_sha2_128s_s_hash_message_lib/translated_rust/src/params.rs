// SPHINCS+ SHA2-128s parameters

pub const SPX_N: usize = 16;
pub const SPX_FULL_HEIGHT: usize = 63;
pub const SPX_D: usize = 7;
pub const SPX_FORS_HEIGHT: usize = 12;
pub const SPX_FORS_TREES: usize = 14;
pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_SHA512: bool = false;
pub const SPX_ADDR_BYTES: usize = 32;

// WOTS parameters
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 32
pub const SPX_WOTS_LEN2: usize = 3; // precomputed for N=16, W=16
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 35
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

// Subtree size
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 9

// FORS parameters
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8; // 21
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

// Resulting SPX sizes
pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// SHA2 address offsets
pub const SPX_OFFSET_LAYER: usize = 0;
pub const SPX_OFFSET_TREE: usize = 1;
pub const SPX_OFFSET_TYPE: usize = 9;
pub const SPX_OFFSET_KP_ADDR: usize = 10;
pub const SPX_OFFSET_CHAIN_ADDR: usize = 17;
pub const SPX_OFFSET_HASH_ADDR: usize = 21;
pub const SPX_OFFSET_TREE_HGT: usize = 17;
pub const SPX_OFFSET_TREE_INDEX: usize = 18;

// SHA2 constants
pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// Address types
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// For hash_message
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 54
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 7
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 9
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 2
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// Since SPX_N < 24, we use SHA-256 for shaX
pub const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
pub const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

// SPX_INBLOCKS = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & -(SPX_SHAX_BLOCK_BYTES as isize)) / SPX_SHAX_BLOCK_BYTES
// = ((16 + 32 + 63) & !63) / 64 = (111 & !63) / 64 = 64 / 64 = 1
pub const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) / SPX_SHAX_BLOCK_BYTES);
