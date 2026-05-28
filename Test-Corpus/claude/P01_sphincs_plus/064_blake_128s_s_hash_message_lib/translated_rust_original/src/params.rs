// Parameters derived from cargo features. They map to the C `SPX_*` macros.

// Defaults to haraka if no backend feature selected.

// Hash output length in bytes
#[cfg(feature = "128s")]
pub const SPX_N: usize = 16;
#[cfg(feature = "128f")]
pub const SPX_N: usize = 16;
#[cfg(feature = "192s")]
pub const SPX_N: usize = 24;
#[cfg(feature = "192f")]
pub const SPX_N: usize = 24;
#[cfg(feature = "256s")]
pub const SPX_N: usize = 32;
#[cfg(feature = "256f")]
pub const SPX_N: usize = 32;

// FULL_HEIGHT, D, FORS_HEIGHT, FORS_TREES depend on (secpar, backend)
// Looking at the params headers, the values for secpar look identical
// across haraka/sha2/shake/blake for given secpar. Let's just choose by
// secpar feature.

// 128s/192s/256s use specific params. From the C headers:
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
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2 logic: For W=16,
//   N <= 8 => 2
//   N <= 136 => 3
//   N <= 256 => 4
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

pub const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// Address offsets — depend on hash backend.
// SHA2 uses small 22-byte addr layout.
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

#[cfg(any(feature = "haraka", feature = "blake", feature = "shake"))]
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

// SHA512 used for N >= 24 in sha2 backend
#[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_SHA512: bool = true;
#[cfg(all(feature = "sha2", any(feature = "128s", feature = "128f")))]
pub const SPX_SHA512: bool = false;

#[cfg(all(feature = "blake", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_BLAKE512: bool = true;
#[cfg(all(feature = "blake", any(feature = "128s", feature = "128f")))]
pub const SPX_BLAKE512: bool = false;

// For non-sha2/non-blake backends, the constant is simply not referenced.
