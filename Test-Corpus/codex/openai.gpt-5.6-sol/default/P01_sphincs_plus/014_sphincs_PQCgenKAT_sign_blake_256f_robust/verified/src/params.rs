#[cfg(feature = "128f")]
mod f128;
#[cfg(feature = "192f")]
mod f192;
#[cfg(feature = "256f")]
mod f256;
#[cfg(feature = "128s")]
mod s128;
#[cfg(feature = "192s")]
mod s192;
#[cfg(feature = "256s")]
mod s256;

#[cfg(feature = "128f")]
pub use f128::*;
#[cfg(feature = "192f")]
pub use f192::*;
#[cfg(feature = "256f")]
pub use f256::*;
#[cfg(feature = "128s")]
pub use s128::*;
#[cfg(feature = "192s")]
pub use s192::*;
#[cfg(feature = "256s")]
pub use s256::*;

pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

pub const SPX_WOTS_W: usize = 16;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: usize = if SPX_N <= 8 { 2 } else if SPX_N <= 136 { 3 } else { 4 };
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;

pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;

pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 4 * SPX_N;

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize =
    SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

pub const HASH: &str = if cfg!(feature = "blake") {
    "blake"
} else if cfg!(feature = "sha2") {
    "sha2"
} else if cfg!(feature = "shake") {
    "shake"
} else {
    "haraka"
};
pub const MODE: &str = if cfg!(feature = "128f") {
    "128f"
} else if cfg!(feature = "128s") {
    "128s"
} else if cfg!(feature = "192f") {
    "192f"
} else if cfg!(feature = "192s") {
    "192s"
} else if cfg!(feature = "256f") {
    "256f"
} else {
    "256s"
};
pub const THASH: &str = if cfg!(feature = "simple") { "simple" } else { "robust" };
