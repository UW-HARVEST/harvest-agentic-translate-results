// SPHINCS+ shake-128s-simple parameters
pub const SPX_N: usize = 16;
pub const SPX_FULL_HEIGHT: usize = 63;
pub const SPX_D: usize = 7;
pub const SPX_FORS_HEIGHT: usize = 12;
pub const SPX_FORS_TREES: usize = 14;
pub const SPX_WOTS_W: usize = 16;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 32
pub const SPX_WOTS_LEN2: usize = 3;
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 35
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N; // 560
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 9

pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8; // 21
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N; // 2912
pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N; // 7856
pub const SPX_PK_BYTES: usize = 2 * SPX_N; // 32
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES; // 64
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N; // 48
pub const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

// SHAKE address offsets
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
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 54
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 7
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 9
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 2
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES; // 30
