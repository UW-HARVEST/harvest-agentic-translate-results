// Parameters - selected by feature flags

// Compute SPX_N
#[cfg(any(feature = "128f", feature = "128s"))]
pub const SPX_N: usize = 16;
#[cfg(any(feature = "192f", feature = "192s"))]
pub const SPX_N: usize = 24;
#[cfg(any(feature = "256f", feature = "256s"))]
pub const SPX_N: usize = 32;

// Compute SPX_FULL_HEIGHT, SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES
// "f" variants
#[cfg(feature = "128f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "128f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "128f")]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(feature = "128f")]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(feature = "192f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "192f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "192f")]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(feature = "192f")]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(feature = "256f")]
pub const SPX_FULL_HEIGHT: usize = 68;
#[cfg(feature = "256f")]
pub const SPX_D: usize = 17;
#[cfg(feature = "256f")]
pub const SPX_FORS_HEIGHT: usize = 9;
#[cfg(feature = "256f")]
pub const SPX_FORS_TREES: usize = 35;

#[cfg(feature = "128s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "128s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "128s")]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(feature = "128s")]
pub const SPX_FORS_TREES: usize = 14;

#[cfg(feature = "192s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "192s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "192s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "192s")]
pub const SPX_FORS_TREES: usize = 17;

#[cfg(feature = "256s")]
pub const SPX_FULL_HEIGHT: usize = 64;
#[cfg(feature = "256s")]
pub const SPX_D: usize = 8;
#[cfg(feature = "256s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "256s")]
pub const SPX_FORS_TREES: usize = 22;

pub const SPX_WOTS_W: usize = 16;

pub const SPX_ADDR_BYTES: usize = 32;

// WOTS parameters
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2 for SPX_WOTS_W == 16:
//   SPX_N <= 8 -> 2; SPX_N <= 136 -> 3; SPX_N <= 256 -> 4
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

// Address offsets - depend on backend
#[cfg(feature = "sha2")]
pub mod offsets {
    pub const SPX_OFFSET_LAYER: usize = 0;
    pub const SPX_OFFSET_TREE: usize = 1;
    pub const SPX_OFFSET_TYPE: usize = 9;
    pub const SPX_OFFSET_KP_ADDR: usize = 10;
    pub const SPX_OFFSET_CHAIN_ADDR: usize = 17;
    pub const SPX_OFFSET_HASH_ADDR: usize = 21;
    pub const SPX_OFFSET_TREE_HGT: usize = 17;
    pub const SPX_OFFSET_TREE_INDEX: usize = 18;
}

#[cfg(any(feature = "haraka", feature = "shake", feature = "blake"))]
pub mod offsets {
    pub const SPX_OFFSET_LAYER: usize = 3;
    pub const SPX_OFFSET_TREE: usize = 8;
    pub const SPX_OFFSET_TYPE: usize = 19;
    pub const SPX_OFFSET_KP_ADDR: usize = 20;
    pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
    pub const SPX_OFFSET_HASH_ADDR: usize = 31;
    pub const SPX_OFFSET_TREE_HGT: usize = 27;
    pub const SPX_OFFSET_TREE_INDEX: usize = 28;
}

// SHA512 mode for SHA2 backend
#[cfg(feature = "sha2")]
pub const SPX_SHA512: bool = SPX_N >= 24;

// BLAKE512 mode for BLAKE backend
#[cfg(feature = "blake")]
pub const SPX_BLAKE512: bool = SPX_N >= 24;

// CRYPTO sizes (for use in C-API)
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// CRYPTO_ALGNAME default
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";
