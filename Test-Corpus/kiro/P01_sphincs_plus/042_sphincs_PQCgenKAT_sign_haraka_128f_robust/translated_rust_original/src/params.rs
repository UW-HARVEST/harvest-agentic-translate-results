// Parameter sets: (SPX_N, SPX_FULL_HEIGHT, SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES, SPX_WOTS_W)
// 128s: N=16, H=63, D=7, FH=12, FT=14, W=16
// 128f: N=16, H=66, D=22, FH=6, FT=33, W=16
// 192s: N=24, H=63, D=7, FH=14, FT=17, W=16
// 192f: N=24, H=66, D=22, FH=8, FT=33, W=16
// 256s: N=32, H=64, D=8, FH=14, FT=22, W=16
// 256f: N=32, H=68, D=17, FH=9, FT=35, W=16

#[cfg(any(
    feature = "sphincs-haraka-128s", feature = "sphincs-sha2-128s",
    feature = "sphincs-shake-128s", feature = "sphincs-blake-128s"
))]
pub const SPX_N: usize = 16;
#[cfg(any(
    feature = "sphincs-haraka-128s", feature = "sphincs-sha2-128s",
    feature = "sphincs-shake-128s", feature = "sphincs-blake-128s"
))]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(any(
    feature = "sphincs-haraka-128s", feature = "sphincs-sha2-128s",
    feature = "sphincs-shake-128s", feature = "sphincs-blake-128s"
))]
pub const SPX_D: usize = 7;
#[cfg(any(
    feature = "sphincs-haraka-128s", feature = "sphincs-sha2-128s",
    feature = "sphincs-shake-128s", feature = "sphincs-blake-128s"
))]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(any(
    feature = "sphincs-haraka-128s", feature = "sphincs-sha2-128s",
    feature = "sphincs-shake-128s", feature = "sphincs-blake-128s"
))]
pub const SPX_FORS_TREES: usize = 14;

#[cfg(any(
    feature = "sphincs-haraka-128f", feature = "sphincs-sha2-128f",
    feature = "sphincs-shake-128f", feature = "sphincs-blake-128f"
))]
pub const SPX_N: usize = 16;
#[cfg(any(
    feature = "sphincs-haraka-128f", feature = "sphincs-sha2-128f",
    feature = "sphincs-shake-128f", feature = "sphincs-blake-128f"
))]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(any(
    feature = "sphincs-haraka-128f", feature = "sphincs-sha2-128f",
    feature = "sphincs-shake-128f", feature = "sphincs-blake-128f"
))]
pub const SPX_D: usize = 22;
#[cfg(any(
    feature = "sphincs-haraka-128f", feature = "sphincs-sha2-128f",
    feature = "sphincs-shake-128f", feature = "sphincs-blake-128f"
))]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(any(
    feature = "sphincs-haraka-128f", feature = "sphincs-sha2-128f",
    feature = "sphincs-shake-128f", feature = "sphincs-blake-128f"
))]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(any(
    feature = "sphincs-haraka-192s", feature = "sphincs-sha2-192s",
    feature = "sphincs-shake-192s", feature = "sphincs-blake-192s"
))]
pub const SPX_N: usize = 24;
#[cfg(any(
    feature = "sphincs-haraka-192s", feature = "sphincs-sha2-192s",
    feature = "sphincs-shake-192s", feature = "sphincs-blake-192s"
))]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(any(
    feature = "sphincs-haraka-192s", feature = "sphincs-sha2-192s",
    feature = "sphincs-shake-192s", feature = "sphincs-blake-192s"
))]
pub const SPX_D: usize = 7;
#[cfg(any(
    feature = "sphincs-haraka-192s", feature = "sphincs-sha2-192s",
    feature = "sphincs-shake-192s", feature = "sphincs-blake-192s"
))]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(any(
    feature = "sphincs-haraka-192s", feature = "sphincs-sha2-192s",
    feature = "sphincs-shake-192s", feature = "sphincs-blake-192s"
))]
pub const SPX_FORS_TREES: usize = 17;

#[cfg(any(
    feature = "sphincs-haraka-192f", feature = "sphincs-sha2-192f",
    feature = "sphincs-shake-192f", feature = "sphincs-blake-192f"
))]
pub const SPX_N: usize = 24;
#[cfg(any(
    feature = "sphincs-haraka-192f", feature = "sphincs-sha2-192f",
    feature = "sphincs-shake-192f", feature = "sphincs-blake-192f"
))]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(any(
    feature = "sphincs-haraka-192f", feature = "sphincs-sha2-192f",
    feature = "sphincs-shake-192f", feature = "sphincs-blake-192f"
))]
pub const SPX_D: usize = 22;
#[cfg(any(
    feature = "sphincs-haraka-192f", feature = "sphincs-sha2-192f",
    feature = "sphincs-shake-192f", feature = "sphincs-blake-192f"
))]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(any(
    feature = "sphincs-haraka-192f", feature = "sphincs-sha2-192f",
    feature = "sphincs-shake-192f", feature = "sphincs-blake-192f"
))]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(any(
    feature = "sphincs-haraka-256s", feature = "sphincs-sha2-256s",
    feature = "sphincs-shake-256s", feature = "sphincs-blake-256s"
))]
pub const SPX_N: usize = 32;
#[cfg(any(
    feature = "sphincs-haraka-256s", feature = "sphincs-sha2-256s",
    feature = "sphincs-shake-256s", feature = "sphincs-blake-256s"
))]
pub const SPX_FULL_HEIGHT: usize = 64;
#[cfg(any(
    feature = "sphincs-haraka-256s", feature = "sphincs-sha2-256s",
    feature = "sphincs-shake-256s", feature = "sphincs-blake-256s"
))]
pub const SPX_D: usize = 8;
#[cfg(any(
    feature = "sphincs-haraka-256s", feature = "sphincs-sha2-256s",
    feature = "sphincs-shake-256s", feature = "sphincs-blake-256s"
))]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(any(
    feature = "sphincs-haraka-256s", feature = "sphincs-sha2-256s",
    feature = "sphincs-shake-256s", feature = "sphincs-blake-256s"
))]
pub const SPX_FORS_TREES: usize = 22;

#[cfg(any(
    feature = "sphincs-haraka-256f", feature = "sphincs-sha2-256f",
    feature = "sphincs-shake-256f", feature = "sphincs-blake-256f"
))]
pub const SPX_N: usize = 32;
#[cfg(any(
    feature = "sphincs-haraka-256f", feature = "sphincs-sha2-256f",
    feature = "sphincs-shake-256f", feature = "sphincs-blake-256f"
))]
pub const SPX_FULL_HEIGHT: usize = 68;
#[cfg(any(
    feature = "sphincs-haraka-256f", feature = "sphincs-sha2-256f",
    feature = "sphincs-shake-256f", feature = "sphincs-blake-256f"
))]
pub const SPX_D: usize = 17;
#[cfg(any(
    feature = "sphincs-haraka-256f", feature = "sphincs-sha2-256f",
    feature = "sphincs-shake-256f", feature = "sphincs-blake-256f"
))]
pub const SPX_FORS_HEIGHT: usize = 9;
#[cfg(any(
    feature = "sphincs-haraka-256f", feature = "sphincs-sha2-256f",
    feature = "sphincs-shake-256f", feature = "sphincs-blake-256f"
))]
pub const SPX_FORS_TREES: usize = 35;

pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2: precomputed for W=16
// N<=8 => 2, N<=136 => 3, N<=256 => 4
pub const SPX_WOTS_LEN2: usize = if SPX_N <= 8 { 2 } else if SPX_N <= 136 { 3 } else { 4 };

pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

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
pub const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

// Address type constants
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// Offset constants depend on hash backend
// SHA2 offsets
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

// SHAKE, BLAKE, HARAKA offsets (all the same)
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_LAYER: usize = 3;
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_TREE: usize = 8;
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_TYPE: usize = 19;
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_KP_ADDR: usize = 20;
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_HASH_ADDR: usize = 31;
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_TREE_HGT: usize = 27;
#[cfg(any(feature = "shake", feature = "blake", feature = "haraka"))]
pub const SPX_OFFSET_TREE_INDEX: usize = 28;

// SHA2-specific constants
#[cfg(feature = "sha2")]
pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
#[cfg(feature = "sha2")]
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
#[cfg(feature = "sha2")]
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
#[cfg(feature = "sha2")]
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
#[cfg(feature = "sha2")]
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// SPX_SHA512 flag: true when N >= 24 for sha2 backend
#[cfg(feature = "sha2")]
pub const SPX_SHA512: bool = SPX_N >= 24;

// SPX_SHAX constants for sha2 backend
#[cfg(feature = "sha2")]
pub const SPX_SHAX_OUTPUT_BYTES: usize = if SPX_N >= 24 { SPX_SHA512_OUTPUT_BYTES } else { SPX_SHA256_OUTPUT_BYTES };
#[cfg(feature = "sha2")]
pub const SPX_SHAX_BLOCK_BYTES: usize = if SPX_N >= 24 { SPX_SHA512_BLOCK_BYTES } else { SPX_SHA256_BLOCK_BYTES };

// BLAKE-specific constants
#[cfg(feature = "blake")]
pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
#[cfg(feature = "blake")]
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;
#[cfg(feature = "blake")]
pub const SPX_BLAKE512: bool = SPX_N >= 24;
#[cfg(feature = "blake")]
pub const SPX_BLAKEX_OUTPUT_BYTES: usize = if SPX_N >= 24 { SPX_BLAKE512_OUTPUT_BYTES } else { SPX_BLAKE256_OUTPUT_BYTES };

// Derived message hash constants
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// SHA2 inblocks constant
#[cfg(feature = "sha2")]
pub const SPX_INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) / SPX_SHAX_BLOCK_BYTES;
