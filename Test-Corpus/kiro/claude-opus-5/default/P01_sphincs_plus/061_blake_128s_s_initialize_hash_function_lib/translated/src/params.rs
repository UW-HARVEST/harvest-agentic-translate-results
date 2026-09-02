//! Translation of `app/include/params.h` together with the selected
//! `app/params/params-sphincs-<backend>-<secpar>.h` and the backend's
//! `<backend>_offsets.h`.

// ---------------------------------------------------------------------------
// SECPAR dependent values.  These are identical for every hash backend.
// ---------------------------------------------------------------------------

/// Hash output length in bytes.
#[cfg(secpar_128s)]
pub const SPX_N: usize = 16;
/// Height of the hypertree.
#[cfg(secpar_128s)]
pub const SPX_FULL_HEIGHT: usize = 63;
/// Number of subtree layer.
#[cfg(secpar_128s)]
pub const SPX_D: usize = 7;
#[cfg(secpar_128s)]
pub const SPX_FORS_HEIGHT: usize = 12;
#[cfg(secpar_128s)]
pub const SPX_FORS_TREES: usize = 14;

#[cfg(secpar_128f)]
pub const SPX_N: usize = 16;
#[cfg(secpar_128f)]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(secpar_128f)]
pub const SPX_D: usize = 22;
#[cfg(secpar_128f)]
pub const SPX_FORS_HEIGHT: usize = 6;
#[cfg(secpar_128f)]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(secpar_192s)]
pub const SPX_N: usize = 24;
#[cfg(secpar_192s)]
pub const SPX_FULL_HEIGHT: usize = 63;
#[cfg(secpar_192s)]
pub const SPX_D: usize = 7;
#[cfg(secpar_192s)]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(secpar_192s)]
pub const SPX_FORS_TREES: usize = 17;

#[cfg(secpar_192f)]
pub const SPX_N: usize = 24;
#[cfg(secpar_192f)]
pub const SPX_FULL_HEIGHT: usize = 66;
#[cfg(secpar_192f)]
pub const SPX_D: usize = 22;
#[cfg(secpar_192f)]
pub const SPX_FORS_HEIGHT: usize = 8;
#[cfg(secpar_192f)]
pub const SPX_FORS_TREES: usize = 33;

#[cfg(secpar_256s)]
pub const SPX_N: usize = 32;
#[cfg(secpar_256s)]
pub const SPX_FULL_HEIGHT: usize = 64;
#[cfg(secpar_256s)]
pub const SPX_D: usize = 8;
#[cfg(secpar_256s)]
pub const SPX_FORS_HEIGHT: usize = 14;
#[cfg(secpar_256s)]
pub const SPX_FORS_TREES: usize = 22;

#[cfg(secpar_256f)]
pub const SPX_N: usize = 32;
#[cfg(secpar_256f)]
pub const SPX_FULL_HEIGHT: usize = 68;
#[cfg(secpar_256f)]
pub const SPX_D: usize = 17;
#[cfg(secpar_256f)]
pub const SPX_FORS_HEIGHT: usize = 9;
#[cfg(secpar_256f)]
pub const SPX_FORS_TREES: usize = 35;

/// `SPX_SHA512` / `SPX_BLAKE512`: use the wide primitive for H and T_l, l >= 2.
pub const SPX_WIDE: bool = cfg!(spx_n_ge_24);

// ---------------------------------------------------------------------------
// Winternitz parameter and derived values.
// ---------------------------------------------------------------------------

pub const SPX_WOTS_W: usize = 16;

/// For clarity.
pub const SPX_ADDR_BYTES: usize = 32;

pub const SPX_WOTS_LOGW: usize = 4;

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

/// `SPX_WOTS_LEN2` is floor(log(len_1 * (w - 1)) / log(w)) + 1; precomputed.
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

/// Subtree size.
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

// FORS parameters.
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

// Resulting SPX sizes.
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// ---------------------------------------------------------------------------
// Address structure field offsets (`<backend>_offsets.h`).
// ---------------------------------------------------------------------------

/// The byte used to specify the Merkle tree layer.
#[cfg(backend_sha2)]
pub const SPX_OFFSET_LAYER: usize = 0;
/// The start of the 8 byte field used to specify the tree.
#[cfg(backend_sha2)]
pub const SPX_OFFSET_TREE: usize = 1;
/// The byte used to specify the hash type (reason).
#[cfg(backend_sha2)]
pub const SPX_OFFSET_TYPE: usize = 9;
/// The start of the 4 byte field used to specify the key pair address.
#[cfg(backend_sha2)]
pub const SPX_OFFSET_KP_ADDR: usize = 10;
/// The byte used to specify the chain address (which Winternitz chain).
#[cfg(backend_sha2)]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 17;
/// The byte used to specify the hash address (where in the Winternitz chain).
#[cfg(backend_sha2)]
pub const SPX_OFFSET_HASH_ADDR: usize = 21;
/// The byte used to specify the height of this node in the FORS/Merkle tree.
#[cfg(backend_sha2)]
pub const SPX_OFFSET_TREE_HGT: usize = 17;
/// The start of the 4 byte field used to specify the node in the tree.
#[cfg(backend_sha2)]
pub const SPX_OFFSET_TREE_INDEX: usize = 18;

#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_LAYER: usize = 3;
#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_TREE: usize = 8;
#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_TYPE: usize = 19;
#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_KP_ADDR: usize = 20;
#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_CHAIN_ADDR: usize = 27;
#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_HASH_ADDR: usize = 31;
#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_TREE_HGT: usize = 27;
#[cfg(not(backend_sha2))]
pub const SPX_OFFSET_TREE_INDEX: usize = 28;

// ---------------------------------------------------------------------------
// Message hash split, defined identically in every `hash_*.c`.
// ---------------------------------------------------------------------------

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// ---------------------------------------------------------------------------
// `app/include/api.h`
// ---------------------------------------------------------------------------

pub const CRYPTO_ALGNAME: &str = "SPHINCS+";
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// ---------------------------------------------------------------------------
// Upper bound on the `inblocks` argument of `thash`, used to size the buffers
// that are variable length arrays in the C sources.
// ---------------------------------------------------------------------------

pub const SPX_THASH_MAX_INBLOCKS: usize = if SPX_WOTS_LEN > SPX_FORS_TREES {
    SPX_WOTS_LEN
} else {
    SPX_FORS_TREES
};
