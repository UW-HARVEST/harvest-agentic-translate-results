// Compile-time SPHINCS+ parameter constants, selected via Cargo features.
//
// Each (HASH_BACKEND, SECPAR) combination corresponds to one of the
// params-sphincs-<backend>-<secpar>.h headers in the C source.

// ---------------------------------------------------------------------------
// SPX_N, SPX_FULL_HEIGHT, SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES
// ---------------------------------------------------------------------------

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
pub const SPX_ADDR_BYTES: usize = 32;

// WOTS parameters
pub const SPX_WOTS_LOGW: usize = 4; // log2(16)
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

// SPX_WOTS_LEN2 is precomputed based on SPX_N (and SPX_WOTS_W = 16).
//   N <= 8   -> 2
//   N <= 136 -> 3
//   N <= 256 -> 4
const fn calc_wots_len2(n: usize) -> usize {
    if n <= 8 {
        2
    } else if n <= 136 {
        3
    } else {
        4
    }
}
pub const SPX_WOTS_LEN2: usize = calc_wots_len2(SPX_N);
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;

// Subtree height
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

// FORS parameters
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;

// Resulting SPX sizes
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// CRYPTO_* aliases
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// SPX_BLAKE512: 0 for blake-128*, 1 for blake-192* / blake-256*
#[cfg(all(feature = "blake", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_BLAKE512: bool = true;
#[cfg(all(feature = "blake", any(feature = "128s", feature = "128f")))]
pub const SPX_BLAKE512: bool = false;

// SPX_SHA512: 0 for sha2-128*, 1 for sha2-192* / sha2-256*
#[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
pub const SPX_SHA512: bool = true;
#[cfg(all(feature = "sha2", any(feature = "128s", feature = "128f")))]
pub const SPX_SHA512: bool = false;

// CRYPTO_ALGNAME from api.h. The C header defines it as "SPHINCS+" if not
// already defined, and nothing else in the build defines it, so it is always
// the literal string "SPHINCS+".
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";

// ---------------------------------------------------------------------------
// Address-format offsets (selected by the hash backend).
// These come from haraka_offsets.h / sha2_offsets.h / shake_offsets.h /
// blake_offsets.h. SHA2 has a different (compact) address layout; the others
// share the same offsets.
// ---------------------------------------------------------------------------

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

// Address types
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;
