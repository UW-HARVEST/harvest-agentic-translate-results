#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Backend {
    Haraka,
    Sha2,
    Shake,
    Blake,
}

#[derive(Copy, Clone)]
pub struct ParamSet {
    pub n: usize,
    pub full_height: usize,
    pub d: usize,
    pub fors_height: usize,
    pub fors_trees: usize,
    pub wots_w: usize,
    pub sha2_512: bool,
    pub blake512: bool,
}

pub const BACKEND: Backend = if cfg!(feature = "haraka") {
    Backend::Haraka
} else if cfg!(feature = "blake") {
    Backend::Blake
} else if cfg!(any(feature = "shake", feature = "shake256")) {
    Backend::Shake
} else {
    Backend::Sha2
};

pub const THASH_SIMPLE: bool = cfg!(feature = "simple");
pub const THASH_ROBUST: bool = !THASH_SIMPLE;

pub const PARAMS: ParamSet = if cfg!(feature = "256f") {
    ParamSet { n: 32, full_height: 68, d: 17, fors_height: 9, fors_trees: 35, wots_w: 16, sha2_512: true, blake512: true }
} else if cfg!(feature = "256s") {
    ParamSet { n: 32, full_height: 64, d: 8, fors_height: 14, fors_trees: 22, wots_w: 16, sha2_512: true, blake512: true }
} else if cfg!(feature = "192f") {
    ParamSet { n: 24, full_height: 66, d: 22, fors_height: 8, fors_trees: 33, wots_w: 16, sha2_512: true, blake512: true }
} else if cfg!(feature = "192s") {
    ParamSet { n: 24, full_height: 63, d: 7, fors_height: 14, fors_trees: 17, wots_w: 16, sha2_512: true, blake512: true }
} else if cfg!(feature = "128f") {
    ParamSet { n: 16, full_height: 66, d: 22, fors_height: 6, fors_trees: 33, wots_w: 16, sha2_512: false, blake512: false }
} else {
    ParamSet { n: 16, full_height: 63, d: 7, fors_height: 12, fors_trees: 14, wots_w: 16, sha2_512: false, blake512: false }
};

pub const SPX_N: usize = PARAMS.n;
pub const SPX_FULL_HEIGHT: usize = PARAMS.full_height;
pub const SPX_D: usize = PARAMS.d;
pub const SPX_FORS_HEIGHT: usize = PARAMS.fors_height;
pub const SPX_FORS_TREES: usize = PARAMS.fors_trees;
pub const SPX_WOTS_W: usize = PARAMS.wots_w;
pub const SPX_SHA512: bool = PARAMS.sha2_512;
pub const SPX_BLAKE512: bool = PARAMS.blake512;

pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LOGW: usize = if SPX_WOTS_W == 16 { 4 } else { 8 };
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: usize = if SPX_WOTS_W == 256 {
    if SPX_N <= 1 {
        1
    } else {
        2
    }
} else if SPX_N <= 8 {
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
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES).div_ceil(8);
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;
pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

pub const CRYPTO_ALGNAME: &str = "SPHINCS+";
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

pub const SPX_OFFSET_LAYER: usize = if matches!(BACKEND, Backend::Sha2) { 0 } else { 3 };
pub const SPX_OFFSET_TREE: usize = if matches!(BACKEND, Backend::Sha2) { 1 } else { 8 };
pub const SPX_OFFSET_TYPE: usize = if matches!(BACKEND, Backend::Sha2) { 9 } else { 19 };
pub const SPX_OFFSET_KP_ADDR: usize = if matches!(BACKEND, Backend::Sha2) { 10 } else { 20 };
pub const SPX_OFFSET_CHAIN_ADDR: usize = if matches!(BACKEND, Backend::Sha2) { 17 } else { 27 };
pub const SPX_OFFSET_HASH_ADDR: usize = if matches!(BACKEND, Backend::Sha2) { 21 } else { 31 };
pub const SPX_OFFSET_TREE_HGT: usize = if matches!(BACKEND, Backend::Sha2) { 17 } else { 27 };
pub const SPX_OFFSET_TREE_INDEX: usize = if matches!(BACKEND, Backend::Sha2) { 18 } else { 28 };

pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
pub const SPX_SHA256_ADDR_BYTES: usize = 22;
