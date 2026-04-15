#[cfg(feature = "secpar-128f")]
pub const SPX_N: usize = 16;
#[cfg(feature = "secpar-128f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "secpar-128f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "secpar-128f")]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(feature = "secpar-128f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "secpar-128f")]
pub const SPX_WOTS_W: usize = 16;

#[cfg(feature = "secpar-128s")]
pub const SPX_N: usize = 16;
#[cfg(feature = "secpar-128s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "secpar-128s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "secpar-128s")]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(feature = "secpar-128s")]
pub const SPX_FORS_TREES: usize = 14;
#[cfg(feature = "secpar-128s")]
pub const SPX_WOTS_W: usize = 16;

#[cfg(feature = "secpar-192f")]
pub const SPX_N: usize = 24;
#[cfg(feature = "secpar-192f")]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(feature = "secpar-192f")]
pub const SPX_D: usize = 22;
#[cfg(feature = "secpar-192f")]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(feature = "secpar-192f")]
pub const SPX_FORS_TREES: usize = 33;
#[cfg(feature = "secpar-192f")]
pub const SPX_WOTS_W: usize = 16;

#[cfg(feature = "secpar-192s")]
pub const SPX_N: usize = 24;
#[cfg(feature = "secpar-192s")]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(feature = "secpar-192s")]
pub const SPX_D: usize = 7;
#[cfg(feature = "secpar-192s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "secpar-192s")]
pub const SPX_FORS_TREES: usize = 17;
#[cfg(feature = "secpar-192s")]
pub const SPX_WOTS_W: usize = 16;

#[cfg(feature = "secpar-256f")]
pub const SPX_N: usize = 32;
#[cfg(feature = "secpar-256f")]
pub const SPX_FULL_HEIGHT: usize = 68;
#[cfg(feature = "secpar-256f")]
pub const SPX_D: usize = 17;
#[cfg(feature = "secpar-256f")]
pub const SPX_FORS_HEIGHT: usize = 9;
#[cfg(feature = "secpar-256f")]
pub const SPX_FORS_TREES: usize = 35;
#[cfg(feature = "secpar-256f")]
pub const SPX_WOTS_W: usize = 16;

#[cfg(feature = "secpar-256s")]
pub const SPX_N: usize = 32;
#[cfg(feature = "secpar-256s")]
pub const SPX_FULL_HEIGHT: usize = 64;
#[cfg(feature = "secpar-256s")]
pub const SPX_D: usize = 8;
#[cfg(feature = "secpar-256s")]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(feature = "secpar-256s")]
pub const SPX_FORS_TREES: usize = 22;
#[cfg(feature = "secpar-256s")]
pub const SPX_WOTS_W: usize = 16;

pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LOGW: usize = if SPX_WOTS_W == 256 { 8 } else { 4 };
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: usize = if SPX_WOTS_W == 256 {
    if SPX_N <= 1 { 1 } else { 2 }
} else {
    if SPX_N <= 8 { 2 } else if SPX_N <= 136 { 3 } else { 4 }
};
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

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_LAYER: usize = 0;
#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_TREE: usize = 1;
#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_TYPE: usize = 9;
#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_KP_ADDR: usize = 10;
#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 17;
#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_HASH_ADDR: usize = 21;
#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_TREE_HGT: usize = 17;
#[cfg(feature = "hash-sha2")]
pub const SPX_OFFSET_TREE_INDEX: usize = 18;

#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_LAYER: usize = 3;
#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_TREE: usize = 8;
#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_TYPE: usize = 19;
#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_KP_ADDR: usize = 20;
#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_HASH_ADDR: usize = 31;
#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_TREE_HGT: usize = 27;
#[cfg(any(feature = "hash-blake", feature = "hash-shake"))]
pub const SPX_OFFSET_TREE_INDEX: usize = 28;
