//! Translation of `app/include/params.h` together with the
//! `app/params/params-sphincs-<backend>-<secpar>.h` headers and the
//! `lib/<backend>/include/<backend>_offsets.h` headers.
//!
//! In the C build CMake concatenates `HASH_BACKEND` and `SECPAR` into the
//! `PARAMS` macro which selects exactly one parameter header.  Here the
//! `SECPAR` half is chosen by the `128s`/`128f`/... features and the
//! backend-dependent half (the address offsets) by the backend features.

macro_rules! secpar_params {
    ($n:expr, $full_height:expr, $d:expr, $fors_height:expr, $fors_trees:expr) => {
        /// Hash output length in bytes.
        pub const SPX_N: usize = $n;
        /// Height of the hypertree.
        pub const SPX_FULL_HEIGHT: usize = $full_height;
        /// Number of subtree layers.
        pub const SPX_D: usize = $d;
        /// FORS tree height.
        pub const SPX_FORS_HEIGHT: usize = $fors_height;
        /// Number of FORS trees.
        pub const SPX_FORS_TREES: usize = $fors_trees;
    };
}

// The five values below are identical across all four hash backends for a
// given SECPAR, so only the SECPAR features need to be inspected.  The arms
// are ordered so that naming a non-default SECPAR feature wins over the
// default `128s` that stays enabled unless `--no-default-features` is used.
#[cfg(feature = "256f")]
secpar_params!(32, 68, 17, 9, 35);

#[cfg(all(feature = "256s", not(feature = "256f")))]
secpar_params!(32, 64, 8, 14, 22);

#[cfg(all(feature = "192f", not(any(feature = "256f", feature = "256s"))))]
secpar_params!(24, 66, 22, 8, 33);

#[cfg(all(
    feature = "192s",
    not(any(feature = "256f", feature = "256s", feature = "192f"))
))]
secpar_params!(24, 63, 7, 14, 17);

#[cfg(all(
    feature = "128f",
    not(any(
        feature = "256f",
        feature = "256s",
        feature = "192f",
        feature = "192s"
    ))
))]
secpar_params!(16, 66, 22, 6, 33);

// Fallback: `128s`, which is also the CMake default for SECPAR.
#[cfg(not(any(
    feature = "256f",
    feature = "256s",
    feature = "192f",
    feature = "192s",
    feature = "128f"
)))]
secpar_params!(16, 63, 7, 12, 14);

/// Winternitz parameter.
pub const SPX_WOTS_W: usize = 16;

/// For clarity (`SPX_ADDR_BYTES`).
pub const SPX_ADDR_BYTES: usize = 32;

macro_rules! address_offsets {
    ($layer:expr, $tree:expr, $ty:expr, $kp:expr, $chain:expr, $hash:expr, $hgt:expr, $index:expr) => {
        /// The byte used to specify the Merkle tree layer.
        pub const SPX_OFFSET_LAYER: usize = $layer;
        /// The start of the 8 byte field used to specify the tree.
        pub const SPX_OFFSET_TREE: usize = $tree;
        /// The byte used to specify the hash type (reason).
        pub const SPX_OFFSET_TYPE: usize = $ty;
        /// The start of the 4 byte field used to specify the key pair address.
        pub const SPX_OFFSET_KP_ADDR: usize = $kp;
        /// The byte used to specify the chain address.
        pub const SPX_OFFSET_CHAIN_ADDR: usize = $chain;
        /// The byte used to specify the hash address.
        pub const SPX_OFFSET_HASH_ADDR: usize = $hash;
        /// The byte used to specify the height of this node in the tree.
        pub const SPX_OFFSET_TREE_HGT: usize = $hgt;
        /// The start of the 4 byte field used to specify the node in the tree.
        pub const SPX_OFFSET_TREE_INDEX: usize = $index;
    };
}

// `lib/sha2/include/sha2_offsets.h`
#[cfg(all(
    feature = "sha2",
    not(any(feature = "blake", feature = "shake"))
))]
address_offsets!(0, 1, 9, 10, 17, 21, 17, 18);

// `haraka_offsets.h`, `shake_offsets.h` and `blake_offsets.h` all agree.
#[cfg(not(all(
    feature = "sha2",
    not(any(feature = "blake", feature = "shake"))
)))]
address_offsets!(3, 8, 19, 20, 27, 31, 27, 28);

/* WOTS parameters. */

/// `SPX_WOTS_LOGW`; `SPX_WOTS_W` is always 16 in the shipped parameter sets.
pub const SPX_WOTS_LOGW: usize = 4;

pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;

/// `floor(log(len_1 * (w - 1)) / log(w)) + 1`, precomputed as in the headers.
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

const _: () = assert!(
    SPX_TREE_HEIGHT * SPX_D == SPX_FULL_HEIGHT,
    "SPX_D should always divide SPX_FULL_HEIGHT"
);

/* FORS parameters. */
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

/* Resulting SPX sizes. */
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

/* Message digest splitting, from the `hash_*.c` back ends. */
pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

const _: () = assert!(
    SPX_TREE_BITS <= 64,
    "For given height and depth, 64 bits cannot represent all subtrees"
);

/* `app/include/api.h` */
pub const CRYPTO_ALGNAME: &str = "SPHINCS+";
pub const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
pub const CRYPTO_BYTES: usize = SPX_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

/* Helpers replacing the C variable length arrays with fixed upper bounds. */

/// Largest `inblocks` argument that ever reaches `thash`: `SPX_WOTS_LEN`
/// (WOTS public key compression) or `SPX_FORS_TREES` (FORS root compression).
pub const SPX_MAX_INBLOCKS: usize = if SPX_WOTS_LEN > SPX_FORS_TREES {
    SPX_WOTS_LEN
} else {
    SPX_FORS_TREES
};

/// Largest `tree_height` argument that ever reaches a tree hash routine.
pub const SPX_MAX_TREE_HEIGHT: usize = if SPX_TREE_HEIGHT > SPX_FORS_HEIGHT {
    SPX_TREE_HEIGHT
} else {
    SPX_FORS_HEIGHT
};
