// Parameter definitions for SPHINCS+
// Selected via Cargo features matching CMake SECPAR + HASH_BACKEND combinations.

#![allow(dead_code)]

// =========================
// Per-secpar values
// =========================

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

pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2 derivation (SPX_WOTS_W == 16)
// Always equals 3 for N <= 136 (which covers all our cases up to 32).
#[cfg(any(
    feature = "128s",
    feature = "128f",
    feature = "192s",
    feature = "192f",
    feature = "256s",
    feature = "256f"
))]
pub const SPX_WOTS_LEN2: usize = 3;

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

// =========================
// Address offsets per backend
// =========================

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

// =========================
// SHA2 vs BLAKE 512 toggle
// =========================

// SHA512 is used when N >= 24 for sha2 backend.
#[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_SHA512: bool = true;
#[cfg(all(feature = "sha2", any(feature = "128s", feature = "128f")))]
pub const SPX_SHA512: bool = false;

#[cfg(all(feature = "blake", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_BLAKE512: bool = true;
#[cfg(all(feature = "blake", any(feature = "128s", feature = "128f")))]
pub const SPX_BLAKE512: bool = false;

// =========================
// CRYPTO_* constants
// =========================
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// CRYPTO_ALGNAME for each combination.
#[cfg(all(feature = "haraka", feature = "robust", feature = "128s"))]
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";
#[cfg(not(all(feature = "haraka", feature = "robust", feature = "128s")))]
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;
