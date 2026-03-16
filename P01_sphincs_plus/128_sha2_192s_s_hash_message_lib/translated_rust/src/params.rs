// SPHINCS+ SHA2-192s simple parameters

pub const SPX_N: usize = 24;
pub const SPX_FULL_HEIGHT: u32 = 63;
pub const SPX_D: u32 = 7;
pub const SPX_FORS_HEIGHT: u32 = 14;
pub const SPX_FORS_TREES: u32 = 17;
pub const SPX_WOTS_W: u32 = 16;
pub const SPX_WOTS_LOGW: u32 = 4;
pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LEN1: u32 = (8 * SPX_N as u32) / SPX_WOTS_LOGW; // 48
pub const SPX_WOTS_LEN2: u32 = 3; // precomputed for N=24, W=16
pub const SPX_WOTS_LEN: u32 = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 51
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN as usize * SPX_N; // 1224
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

pub const SPX_TREE_HEIGHT: u32 = SPX_FULL_HEIGHT / SPX_D; // 9

pub const SPX_FORS_MSG_BYTES: usize =
    ((SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8) as usize; // 30
pub const SPX_FORS_BYTES: usize =
    ((SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES) as usize * SPX_N; // 6120
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + (SPX_D as usize) * SPX_WOTS_BYTES + (SPX_FULL_HEIGHT as usize) * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
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

pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// Address types
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// For SHA2-192s: SPX_N >= 24, so use SHA-512 for H and T_l
// SPX_SHAX = SHA-512
pub const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
pub const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;
