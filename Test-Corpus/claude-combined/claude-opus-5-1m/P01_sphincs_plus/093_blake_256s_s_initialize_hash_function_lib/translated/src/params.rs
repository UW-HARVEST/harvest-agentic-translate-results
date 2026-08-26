// Feature-gated parameter constants for SPHINCS+
//
// Each combination of (hash backend, secpar) selects a distinct parameter set.
// The values match the corresponding C header in c_src/app/params/.

#![allow(dead_code)]

// ---- Hash output length (SPX_N) ----
#[cfg(any(feature = "128s", feature = "128f"))]
pub const SPX_N: usize = 16;
#[cfg(any(feature = "192s", feature = "192f"))]
pub const SPX_N: usize = 24;
#[cfg(any(feature = "256s", feature = "256f"))]
pub const SPX_N: usize = 32;

// ---- Hypertree height/depth, FORS dims ----
// 128s: H=63, D=7, FH=12, FT=14
// 128f: H=66, D=22, FH=6, FT=33
// 192s: H=63, D=7, FH=14, FT=17
// 192f: H=66, D=22, FH=8, FT=33
// 256s: H=64, D=8, FH=14, FT=22
// 256f: H=68, D=17, FH=9, FT=35

#[cfg(feature = "128s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "128s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "128s")]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(feature = "128s")]
pub const SPX_FORS_TREES: usize = 14;

#[cfg(feature = "128f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "128f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "128f")]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(feature = "128f")]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(feature = "192s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "192s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "192s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "192s")]
pub const SPX_FORS_TREES: usize = 17;

#[cfg(feature = "192f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "192f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "192f")]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(feature = "192f")]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(feature = "256s")]
pub const SPX_FULL_HEIGHT: usize = 64;
#[cfg(feature = "256s")]
pub const SPX_D: usize = 8;
#[cfg(feature = "256s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "256s")]
pub const SPX_FORS_TREES: usize = 22;

#[cfg(feature = "256f")]
pub const SPX_FULL_HEIGHT: usize = 68;
#[cfg(feature = "256f")]
pub const SPX_D: usize = 17;
#[cfg(feature = "256f")]
pub const SPX_FORS_HEIGHT: usize = 9;
#[cfg(feature = "256f")]
pub const SPX_FORS_TREES: usize = 35;

pub const SPX_WOTS_W: usize = 16;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LOGW: usize = 4; // log2(16)

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2 = floor(log(len1*(w-1))/log(w)) + 1; precomputed for w=16
// N <= 8 -> 2; N <= 136 -> 3
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

// CRYPTO_* aliases
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// ---- Address offsets (depend on hash backend) ----
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub const SPX_OFFSET_LAYER: usize = 3;
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub const SPX_OFFSET_TREE: usize = 8;
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub const SPX_OFFSET_TYPE: usize = 19;
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub const SPX_OFFSET_KP_ADDR: usize = 20;
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub const SPX_OFFSET_HASH_ADDR: usize = 31;
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub const SPX_OFFSET_TREE_HGT: usize = 27;
#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
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

// SHA2 specific: SHA512 used for N >= 24
#[cfg(feature = "sha2")]
pub const SPX_SHA512: bool = SPX_N >= 24;

// BLAKE specific: BLAKE512 used for N >= 24
#[cfg(feature = "blake")]
pub const SPX_BLAKE512: bool = SPX_N >= 24;

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// SHA2 specific
pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// BLAKE specific
pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;
