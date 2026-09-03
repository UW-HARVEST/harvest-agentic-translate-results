#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    Blake,
    Sha2,
    Shake,
    Haraka,
}

#[cfg(feature = "blake")]
pub const BACKEND: Backend = Backend::Blake;
#[cfg(all(not(feature = "blake"), feature = "sha2"))]
pub const BACKEND: Backend = Backend::Sha2;
#[cfg(all(
    not(feature = "blake"),
    not(feature = "sha2"),
    feature = "shake"
))]
pub const BACKEND: Backend = Backend::Shake;
#[cfg(all(
    not(feature = "blake"),
    not(feature = "sha2"),
    not(feature = "shake")
))]
pub const BACKEND: Backend = Backend::Haraka;

pub const ROBUST: bool = cfg!(feature = "robust") || !cfg!(feature = "simple");

#[cfg(feature = "128s")]
mod selected {
    pub const N: usize = 16;
    pub const FULL_HEIGHT: usize = 63;
    pub const D: usize = 7;
    pub const FORS_HEIGHT: usize = 12;
    pub const FORS_TREES: usize = 14;
}
#[cfg(all(not(feature = "128s"), feature = "128f"))]
mod selected {
    pub const N: usize = 16;
    pub const FULL_HEIGHT: usize = 66;
    pub const D: usize = 22;
    pub const FORS_HEIGHT: usize = 6;
    pub const FORS_TREES: usize = 33;
}
#[cfg(all(
    not(feature = "128s"),
    not(feature = "128f"),
    feature = "192s"
))]
mod selected {
    pub const N: usize = 24;
    pub const FULL_HEIGHT: usize = 63;
    pub const D: usize = 7;
    pub const FORS_HEIGHT: usize = 14;
    pub const FORS_TREES: usize = 17;
}
#[cfg(all(
    not(feature = "128s"),
    not(feature = "128f"),
    not(feature = "192s"),
    feature = "192f"
))]
mod selected {
    pub const N: usize = 24;
    pub const FULL_HEIGHT: usize = 66;
    pub const D: usize = 22;
    pub const FORS_HEIGHT: usize = 8;
    pub const FORS_TREES: usize = 33;
}
#[cfg(all(
    not(feature = "128s"),
    not(feature = "128f"),
    not(feature = "192s"),
    not(feature = "192f"),
    feature = "256s"
))]
mod selected {
    pub const N: usize = 32;
    pub const FULL_HEIGHT: usize = 64;
    pub const D: usize = 8;
    pub const FORS_HEIGHT: usize = 14;
    pub const FORS_TREES: usize = 22;
}
#[cfg(all(
    not(feature = "128s"),
    not(feature = "128f"),
    not(feature = "192s"),
    not(feature = "192f"),
    not(feature = "256s")
))]
mod selected {
    pub const N: usize = 32;
    pub const FULL_HEIGHT: usize = 68;
    pub const D: usize = 17;
    pub const FORS_HEIGHT: usize = 9;
    pub const FORS_TREES: usize = 35;
}

pub use selected::{D, FORS_HEIGHT, FORS_TREES, FULL_HEIGHT, N};

pub const WOTS_W: usize = 16;
pub const WOTS_LOGW: usize = 4;
pub const WOTS_LEN1: usize = 8 * N / WOTS_LOGW;
pub const WOTS_LEN2: usize = 3;
pub const WOTS_LEN: usize = WOTS_LEN1 + WOTS_LEN2;
pub const WOTS_BYTES: usize = WOTS_LEN * N;
pub const TREE_HEIGHT: usize = FULL_HEIGHT / D;
pub const FORS_MSG_BYTES: usize = (FORS_HEIGHT * FORS_TREES + 7) / 8;
pub const FORS_BYTES: usize = (FORS_HEIGHT + 1) * FORS_TREES * N;
pub const BYTES: usize = N + FORS_BYTES + D * WOTS_BYTES + FULL_HEIGHT * N;
pub const PK_BYTES: usize = 2 * N;
pub const SK_BYTES: usize = 4 * N;
pub const SEED_BYTES: usize = 3 * N;

pub const ADDR_TYPE_WOTS: u32 = 0;
pub const ADDR_TYPE_WOTSPK: u32 = 1;
pub const ADDR_TYPE_HASHTREE: u32 = 2;
pub const ADDR_TYPE_FORSTREE: u32 = 3;
pub const ADDR_TYPE_FORSPK: u32 = 4;
pub const ADDR_TYPE_WOTSPRF: u32 = 5;
pub const ADDR_TYPE_FORSPRF: u32 = 6;

pub const SHA2_ADDR_BYTES: usize = 22;
pub const ADDR_BYTES: usize = 32;
pub const SHA512: bool = N >= 24;
pub const BLAKE512: bool = N >= 24;

pub const SPX_N: usize = N;
pub const SPX_FULL_HEIGHT: usize = FULL_HEIGHT;
pub const SPX_D: usize = D;
pub const SPX_FORS_HEIGHT: usize = FORS_HEIGHT;
pub const SPX_FORS_TREES: usize = FORS_TREES;
pub const SPX_WOTS_W: usize = WOTS_W;
pub const SPX_WOTS_LOGW: usize = WOTS_LOGW;
pub const SPX_WOTS_LEN1: usize = WOTS_LEN1;
pub const SPX_WOTS_LEN2: usize = WOTS_LEN2;
pub const SPX_WOTS_LEN: usize = WOTS_LEN;
pub const SPX_WOTS_BYTES: usize = WOTS_BYTES;
pub const SPX_TREE_HEIGHT: usize = TREE_HEIGHT;
pub const SPX_FORS_MSG_BYTES: usize = FORS_MSG_BYTES;
pub const SPX_FORS_BYTES: usize = FORS_BYTES;
pub const SPX_BYTES: usize = BYTES;
pub const SPX_PK_BYTES: usize = PK_BYTES;
pub const CRYPTO_SECRETKEYBYTES: usize = SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = PK_BYTES;
pub const CRYPTO_BYTES: usize = BYTES;
pub const CRYPTO_SEEDBYTES: usize = SEED_BYTES;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

pub const SPX_OFFSET_LAYER: usize = if matches!(BACKEND, Backend::Sha2) { 0 } else { 3 };
pub const SPX_OFFSET_TREE: usize = if matches!(BACKEND, Backend::Sha2) { 1 } else { 8 };
pub const SPX_OFFSET_TYPE: usize = if matches!(BACKEND, Backend::Sha2) { 9 } else { 19 };
pub const SPX_OFFSET_KP_ADDR: usize = if matches!(BACKEND, Backend::Sha2) { 10 } else { 20 };
pub const SPX_OFFSET_KP_ADDR2: usize = if matches!(BACKEND, Backend::Sha2) { 12 } else { 22 };
pub const SPX_OFFSET_KP_ADDR1: usize = if matches!(BACKEND, Backend::Sha2) { 13 } else { 23 };
pub const SPX_OFFSET_CHAIN_ADDR: usize = if matches!(BACKEND, Backend::Sha2) { 17 } else { 27 };
pub const SPX_OFFSET_HASH_ADDR: usize = if matches!(BACKEND, Backend::Sha2) { 21 } else { 31 };
pub const SPX_OFFSET_TREE_HGT: usize = if matches!(BACKEND, Backend::Sha2) { 17 } else { 27 };
pub const SPX_OFFSET_TREE_INDEX: usize = if matches!(BACKEND, Backend::Sha2) { 18 } else { 28 };
