//! Shared infrastructure for the C-vs-Rust differential tests.
//!
//! IMPORTANT — this module deliberately does **not** reference the `sphincsplus`
//! crate.  If the crate were linked into the test executable, the executable
//! would itself define `SPX_thash`, `crypto_sign_verify`, … and — because the
//! main executable sits at the front of the dynamic linker's *global* search
//! scope — the C shared libraries we `dlopen` would bind **their** calls to the
//! Rust implementations, silently making every comparison a tautology.
//!
//! For the same reason the load order in [`Libs::get`] matters and is enforced:
//!
//! 1. `libsphincsplus.so`  — `RTLD_NOW | RTLD_LOCAL`.  At this point nothing in
//!    the process defines any `SPX_*` / `crypto_sign*` / `blake*` / … symbol, so
//!    every one of its relocations is resolved against **itself**.  Because it
//!    is `RTLD_LOCAL` its symbols never enter the global scope, so the C
//!    libraries loaded afterwards can never bind to it.
//! 2. `lib<backend>.so`    — `RTLD_NOW | RTLD_GLOBAL` (provides `SPX_thash`,
//!    `SPX_prf_addr`, … to the core library).
//! 3. `libsphincs_core_det.so` — `RTLD_NOW | RTLD_GLOBAL`.
//!
//! Everything is reached through `dlsym`; no Rust function is ever called
//! directly, so the `#[no_mangle]` export wrappers are part of what is tested.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::too_many_arguments)]

use libloading::os::unix::{Library, Symbol, RTLD_GLOBAL, RTLD_LAZY, RTLD_LOCAL, RTLD_NOW};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ===========================================================================
// Build configuration mirrored from src/params.rs (kept independent on purpose)
// ===========================================================================

pub const IS_SHA2: bool = cfg!(feature = "sha2");
pub const IS_SHAKE: bool = cfg!(all(feature = "shake", not(feature = "sha2")));
pub const IS_BLAKE: bool = cfg!(all(
    feature = "blake",
    not(feature = "sha2"),
    not(feature = "shake")
));
pub const IS_HARAKA: bool = !IS_SHA2 && !IS_SHAKE && !IS_BLAKE;

pub const BACKEND: &str = if IS_SHA2 {
    "sha2"
} else if IS_SHAKE {
    "shake"
} else if IS_BLAKE {
    "blake"
} else {
    "haraka"
};

pub const THASH: &str = if cfg!(feature = "simple") {
    "simple"
} else {
    "robust"
};

pub const SECPAR: &str = if cfg!(feature = "128s") {
    "128s"
} else if cfg!(feature = "128f") {
    "128f"
} else if cfg!(feature = "192s") {
    "192s"
} else if cfg!(feature = "192f") {
    "192f"
} else if cfg!(feature = "256s") {
    "256s"
} else if cfg!(feature = "256f") {
    "256f"
} else {
    "128s"
};

const N_IS_256: bool = cfg!(feature = "256s") || cfg!(feature = "256f");
const N_IS_192: bool = cfg!(feature = "192s") || cfg!(feature = "192f");
const IS_FAST: bool =
    cfg!(feature = "128f") || cfg!(feature = "192f") || cfg!(feature = "256f");

pub const SPX_N: usize = if N_IS_256 {
    32
} else if N_IS_192 {
    24
} else {
    16
};
pub const SPX_FULL_HEIGHT: usize = if N_IS_256 {
    if IS_FAST {
        68
    } else {
        64
    }
} else if IS_FAST {
    66
} else {
    63
};
pub const SPX_D: usize = if N_IS_256 {
    if IS_FAST {
        17
    } else {
        8
    }
} else if IS_FAST {
    22
} else {
    7
};
pub const SPX_FORS_HEIGHT: usize = if N_IS_256 {
    if IS_FAST {
        9
    } else {
        14
    }
} else if N_IS_192 {
    if IS_FAST {
        8
    } else {
        14
    }
} else if IS_FAST {
    6
} else {
    12
};
pub const SPX_FORS_TREES: usize = if N_IS_256 {
    if IS_FAST {
        35
    } else {
        22
    }
} else if N_IS_192 {
    if IS_FAST {
        33
    } else {
        17
    }
} else if IS_FAST {
    33
} else {
    14
};

pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: usize = if SPX_N <= 8 {
    2
} else if SPX_N <= 136 {
    3
} else {
    4
};
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BYTES: usize = (SPX_TREE_HEIGHT + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

pub const SPX_SHA512: bool = SPX_N >= 24;
pub const SPX_BLAKE512: bool = SPX_N >= 24;

pub const SPX_OFFSET_LAYER: usize = if IS_SHA2 { 0 } else { 3 };
pub const SPX_OFFSET_TREE: usize = if IS_SHA2 { 1 } else { 8 };
pub const SPX_OFFSET_TYPE: usize = if IS_SHA2 { 9 } else { 19 };
pub const SPX_OFFSET_KP_ADDR: usize = if IS_SHA2 { 10 } else { 20 };
pub const SPX_OFFSET_CHAIN_ADDR: usize = if IS_SHA2 { 17 } else { 27 };
pub const SPX_OFFSET_HASH_ADDR: usize = if IS_SHA2 { 21 } else { 31 };
pub const SPX_OFFSET_TREE_HGT: usize = if IS_SHA2 { 17 } else { 27 };
pub const SPX_OFFSET_TREE_INDEX: usize = if IS_SHA2 { 18 } else { 28 };
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

/// Number of context bytes that are actually *defined* by the C `spx_ctx` for
/// the active backend — the window that may be compared byte for byte.
pub const CTX_LIVE_BYTES: usize = if IS_SHA2 {
    2 * SPX_N + 40 + if SPX_SHA512 { 72 } else { 0 }
} else if IS_HARAKA {
    2 * SPX_N + 10 * 8 * 8 + 10 * 8 * 4
} else {
    2 * SPX_N
};

// ===========================================================================
// Over-sized, over-aligned scratch buffer used as an `spx_ctx`
// ===========================================================================

/// The C `spx_ctx` is at most `2*32 + 640 + 320 = 1024` bytes (haraka/256*).
/// The Rust `SpxCtx` may be *larger* for `sha2` with `SPX_N < 24` (it always
/// carries `state_seeded_512`).  Both agree on the offsets of every field that
/// is actually used, so a shared over-sized, 16-byte-aligned buffer can back
/// either.
#[repr(C, align(16))]
pub struct CtxBuf(pub [u8; 2048]);

impl CtxBuf {
    pub fn new() -> Box<CtxBuf> {
        Box::new(CtxBuf([0u8; 2048]))
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
    pub fn set_seeds(&mut self, pub_seed: &[u8], sk_seed: &[u8]) {
        self.0[..SPX_N].copy_from_slice(&pub_seed[..SPX_N]);
        self.0[SPX_N..2 * SPX_N].copy_from_slice(&sk_seed[..SPX_N]);
    }
    pub fn live(&self) -> &[u8] {
        &self.0[..CTX_LIVE_BYTES]
    }
}

// ===========================================================================
// Structs shared across the FFI boundary
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    pub fn zeroed() -> Self {
        LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: std::ptr::null_mut(),
            leaf_addr: [0; 8],
            pk_addr: [0; 8],
        }
    }
    /// The observable part (the two pointers are per-call scratch).
    pub fn observable(&self) -> ([u32; 8], [u32; 8], u32) {
        (self.leaf_addr, self.pk_addr, self.wots_sign_leaf)
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

impl AesXofStruct {
    pub fn zeroed() -> Self {
        AesXofStruct {
            buffer: [0; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0; 32],
            ctr: [0; 16],
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        let p = self as *const AesXofStruct as *const u8;
        unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<AesXofStruct>()) }.to_vec()
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl BlakeState256 {
    pub fn zeroed() -> Self {
        BlakeState256 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        let p = self as *const _ as *const u8;
        unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<Self>()) }.to_vec()
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl BlakeState512 {
    pub fn zeroed() -> Self {
        BlakeState512 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        let p = self as *const _ as *const u8;
        unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<Self>()) }.to_vec()
    }
}

// ===========================================================================
// Function-pointer types
// ===========================================================================

pub type GenLeafFn =
    unsafe extern "C" fn(leaf: *mut u8, ctx: *const u8, addr_idx: u32, tree_addr: *const u32);

macro_rules! fnty {
    ($name:ident : $($t:tt)*) => { pub type $name = unsafe extern "C" $($t)*; };
}

fnty!(FnAddrU32:  fn(*mut u32, u32));
fnty!(FnAddrU64:  fn(*mut u32, u64));
fnty!(FnAddrCopy: fn(*mut u32, *const u32));
fnty!(FnUllToBytes: fn(*mut u8, u32, u64));
fnty!(FnU32ToBytes: fn(*mut u8, u32));
fnty!(FnBytesToUll: fn(*const u8, u32) -> u64);
fnty!(FnComputeRoot: fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32));
fnty!(FnTreehash: fn(*mut u8, *mut u8, *const u8, u32, u32, u32, GenLeafFn, *mut u32));
fnty!(FnThash: fn(*mut u8, *const u8, u32, *const u8, *mut u32));
fnty!(FnCtx: fn(*mut u8));
fnty!(FnPrfAddr: fn(*mut u8, *const u8, *const u32));
fnty!(FnGenMsgRandom: fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8));
fnty!(FnHashMessage: fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8));
fnty!(FnChainLengths: fn(*mut u32, *const u8));
fnty!(FnWotsPkFromSig: fn(*mut u8, *const u8, *const u8, *const u8, *mut u32));
fnty!(FnWotsGenLeafx1: fn(*mut u8, *const u8, u32, *mut LeafInfoX1));
fnty!(FnForsGenLeafx1: fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo));
fnty!(FnForsSign: fn(*mut u8, *mut u8, *const u8, *const u8, *const u32));
fnty!(FnForsPkFromSig: fn(*mut u8, *const u8, *const u8, *const u8, *const u32));
fnty!(FnWotsTreehashx1: fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfoX1));
fnty!(FnForsTreehashx1: fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut ForsGenLeafInfo));
fnty!(FnMerkleSign: fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32));
fnty!(FnMerkleGenRoot: fn(*mut u8, *const u8));
fnty!(FnSizes: fn() -> u64);
fnty!(FnSeedKeypair: fn(*mut u8, *mut u8, *const u8) -> i32);
fnty!(FnKeypair: fn(*mut u8, *mut u8) -> i32);
fnty!(FnSignature: fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32);
fnty!(FnVerify: fn(*const u8, usize, *const u8, usize, *const u8) -> i32);
fnty!(FnSign: fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32);
fnty!(FnSignOpen: fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32);
fnty!(FnDrbgUpdate: fn(*mut u8, *mut u8, *mut u8));
fnty!(FnAes256Ecb: fn(*mut u8, *mut u8, *mut u8));
fnty!(FnSeedexpanderInit: fn(*mut AesXofStruct, *mut u8, *mut u8, u64) -> i32);
fnty!(FnSeedexpander: fn(*mut AesXofStruct, *mut u8, u64) -> i32);
fnty!(FnRandombytesInit: fn(*mut u8, *mut u8));
fnty!(FnRandombytes: fn(*mut u8, u64) -> i32);
// blake
fnty!(FnBlake256: fn(*mut u8, *const u8, u64) -> i32);
fnty!(FnBlake256Init: fn(*mut BlakeState256));
fnty!(FnBlake256Update: fn(*mut BlakeState256, *const u8, u64));
fnty!(FnBlake256Final: fn(*mut BlakeState256, *mut u8));
fnty!(FnBlake256Compress: fn(*mut BlakeState256, *const u8));
fnty!(FnBlake512Init: fn(*mut BlakeState512));
fnty!(FnBlake512Update: fn(*mut BlakeState512, *const u8, u64));
fnty!(FnBlake512Final: fn(*mut BlakeState512, *mut u8));
fnty!(FnBlake512Compress: fn(*mut BlakeState512, *const u8));
fnty!(FnMgf1: fn(*mut u8, u64, *const u8, u64));
// sha2
fnty!(FnSha: fn(*mut u8, *const u8, usize));
fnty!(FnShaIncInit: fn(*mut u8));
fnty!(FnShaIncBlocks: fn(*mut u8, *const u8, usize));
fnty!(FnShaIncFinalize: fn(*mut u8, *mut u8, *const u8, usize));
// shake
fnty!(FnShake256: fn(*mut u8, usize, *const u8, usize));
fnty!(FnShakeAbsorb: fn(*mut u64, *const u8, usize));
fnty!(FnShakeSqueezeblocks: fn(*mut u8, usize, *mut u64));
fnty!(FnShakeIncInit: fn(*mut u64));
fnty!(FnShakeIncAbsorb: fn(*mut u64, *const u8, usize));
fnty!(FnShakeIncFinalize: fn(*mut u64));
fnty!(FnShakeIncSqueeze: fn(*mut u8, usize, *mut u64));
// haraka
fnty!(FnHarakaPerm: fn(*mut u8, *const u8, *const u8));
fnty!(FnHarakaS: fn(*mut u8, u64, *const u8, u64, *const u8));
fnty!(FnHarakaSIncInit: fn(*mut u8));
fnty!(FnHarakaSIncAbsorb: fn(*mut u8, *const u8, usize, *const u8));
fnty!(FnHarakaSIncFinalize: fn(*mut u8));
fnty!(FnHarakaSIncSqueeze: fn(*mut u8, usize, *mut u8, *const u8));

// ===========================================================================
// The two implementations
// ===========================================================================

/// One side of the comparison: a `dlopen`ed implementation.
pub struct Impl {
    pub name: &'static str,
    libs: Vec<Library>,
}

impl Impl {
    fn sym<T: Copy>(&self, name: &str) -> T {
        for l in &self.libs {
            let bytes = format!("{name}\0").into_bytes();
            if let Ok(s) = unsafe { l.get::<T>(&bytes) } {
                let s: Symbol<T> = s;
                // `Deref` for `os::unix::Symbol<T>` reinterprets the stored
                // `dlsym` result as a `T`, which for a function-pointer `T` is
                // exactly the function pointer.
                return *s;
            }
        }
        panic!("{}: symbol `{}` not found", self.name, name);
    }

    /// Address of a data symbol.
    pub fn data(&self, name: &str) -> *mut u8 {
        for l in &self.libs {
            let bytes = format!("{name}\0").into_bytes();
            if let Ok(s) = unsafe { l.get::<*mut u8>(&bytes) } {
                return s.into_raw() as *mut u8;
            }
        }
        panic!("{}: data symbol `{}` not found", self.name, name);
    }

    pub fn has(&self, name: &str) -> bool {
        for l in &self.libs {
            let bytes = format!("{name}\0").into_bytes();
            if unsafe { l.get::<*mut u8>(&bytes) }.is_ok() {
                return true;
            }
        }
        false
    }

    // --- address.c ---
    pub fn set_layer_addr(&self) -> FnAddrU32 { self.sym("SPX_set_layer_addr") }
    pub fn set_tree_addr(&self) -> FnAddrU64 { self.sym("SPX_set_tree_addr") }
    pub fn set_type(&self) -> FnAddrU32 { self.sym("SPX_set_type") }
    pub fn copy_subtree_addr(&self) -> FnAddrCopy { self.sym("SPX_copy_subtree_addr") }
    pub fn set_keypair_addr(&self) -> FnAddrU32 { self.sym("SPX_set_keypair_addr") }
    pub fn copy_keypair_addr(&self) -> FnAddrCopy { self.sym("SPX_copy_keypair_addr") }
    pub fn set_chain_addr(&self) -> FnAddrU32 { self.sym("SPX_set_chain_addr") }
    pub fn set_hash_addr(&self) -> FnAddrU32 { self.sym("SPX_set_hash_addr") }
    pub fn set_tree_height(&self) -> FnAddrU32 { self.sym("SPX_set_tree_height") }
    pub fn set_tree_index(&self) -> FnAddrU32 { self.sym("SPX_set_tree_index") }
    // --- utils.c ---
    pub fn ull_to_bytes(&self) -> FnUllToBytes { self.sym("SPX_ull_to_bytes") }
    pub fn u32_to_bytes(&self) -> FnU32ToBytes { self.sym("SPX_u32_to_bytes") }
    pub fn bytes_to_ull(&self) -> FnBytesToUll { self.sym("SPX_bytes_to_ull") }
    pub fn compute_root(&self) -> FnComputeRoot { self.sym("SPX_compute_root") }
    pub fn treehash(&self) -> FnTreehash { self.sym("SPX_treehash") }
    // --- thash / hash ---
    pub fn thash(&self) -> FnThash { self.sym("SPX_thash") }
    pub fn initialize_hash_function(&self) -> FnCtx { self.sym("SPX_initialize_hash_function") }
    pub fn prf_addr(&self) -> FnPrfAddr { self.sym("SPX_prf_addr") }
    pub fn gen_message_random(&self) -> FnGenMsgRandom { self.sym("SPX_gen_message_random") }
    pub fn hash_message(&self) -> FnHashMessage { self.sym("SPX_hash_message") }
    // --- wots ---
    pub fn chain_lengths(&self) -> FnChainLengths { self.sym("SPX_chain_lengths") }
    pub fn wots_pk_from_sig(&self) -> FnWotsPkFromSig { self.sym("SPX_wots_pk_from_sig") }
    pub fn wots_gen_leafx1(&self) -> FnWotsGenLeafx1 { self.sym("SPX_wots_gen_leafx1") }
    // --- fors ---
    pub fn fors_gen_leafx1(&self) -> FnForsGenLeafx1 { self.sym("SPX_fors_gen_leafx1") }
    pub fn fors_sign(&self) -> FnForsSign { self.sym("SPX_fors_sign") }
    pub fn fors_pk_from_sig(&self) -> FnForsPkFromSig { self.sym("SPX_fors_pk_from_sig") }
    // --- utilsx1 ---
    pub fn wots_treehashx1(&self) -> FnWotsTreehashx1 { self.sym("SPX_wots_treehashx1") }
    pub fn fors_treehashx1(&self) -> FnForsTreehashx1 { self.sym("SPX_fors_treehashx1") }
    // --- merkle ---
    pub fn merkle_sign(&self) -> FnMerkleSign { self.sym("SPX_merkle_sign") }
    pub fn merkle_gen_root(&self) -> FnMerkleGenRoot { self.sym("SPX_merkle_gen_root") }
    // --- api ---
    pub fn crypto_sign_secretkeybytes(&self) -> FnSizes { self.sym("crypto_sign_secretkeybytes") }
    pub fn crypto_sign_publickeybytes(&self) -> FnSizes { self.sym("crypto_sign_publickeybytes") }
    pub fn crypto_sign_bytes(&self) -> FnSizes { self.sym("crypto_sign_bytes") }
    pub fn crypto_sign_seedbytes(&self) -> FnSizes { self.sym("crypto_sign_seedbytes") }
    pub fn crypto_sign_seed_keypair(&self) -> FnSeedKeypair { self.sym("crypto_sign_seed_keypair") }
    pub fn crypto_sign_keypair(&self) -> FnKeypair { self.sym("crypto_sign_keypair") }
    pub fn crypto_sign_signature(&self) -> FnSignature { self.sym("crypto_sign_signature") }
    pub fn crypto_sign_verify(&self) -> FnVerify { self.sym("crypto_sign_verify") }
    pub fn crypto_sign(&self) -> FnSign { self.sym("crypto_sign") }
    pub fn crypto_sign_open(&self) -> FnSignOpen { self.sym("crypto_sign_open") }
    // --- rng ---
    pub fn drbg_update(&self) -> FnDrbgUpdate { self.sym("AES256_CTR_DRBG_Update") }
    pub fn aes256_ecb(&self) -> FnAes256Ecb { self.sym("AES256_ECB") }
    pub fn seedexpander_init(&self) -> FnSeedexpanderInit { self.sym("seedexpander_init") }
    pub fn seedexpander(&self) -> FnSeedexpander { self.sym("seedexpander") }
    pub fn randombytes_init(&self) -> FnRandombytesInit { self.sym("randombytes_init") }
    pub fn randombytes(&self) -> FnRandombytes { self.sym("randombytes") }
    pub fn drbg_ctx(&self) -> *mut DrbgStruct { self.data("DRBG_ctx") as *mut DrbgStruct }
    // --- blake ---
    pub fn blake256(&self) -> FnBlake256 { self.sym("blake256") }
    pub fn blake256_init(&self) -> FnBlake256Init { self.sym("blake256_init") }
    pub fn blake256_update(&self) -> FnBlake256Update { self.sym("blake256_update") }
    pub fn blake256_final(&self) -> FnBlake256Final { self.sym("blake256_final") }
    pub fn blake256_compress(&self) -> FnBlake256Compress { self.sym("blake256_compress") }
    pub fn blake512(&self) -> FnBlake256 { self.sym("blake512") }
    pub fn blake512_init(&self) -> FnBlake512Init { self.sym("blake512_init") }
    pub fn blake512_update(&self) -> FnBlake512Update { self.sym("blake512_update") }
    pub fn blake512_final(&self) -> FnBlake512Final { self.sym("blake512_final") }
    pub fn blake512_compress(&self) -> FnBlake512Compress { self.sym("blake512_compress") }
    pub fn blake256_mgf1(&self) -> FnMgf1 { self.sym("SPX_blake256_mgf1") }
    pub fn blake512_mgf1(&self) -> FnMgf1 { self.sym("SPX_blake512_mgf1") }
    pub fn cst(&self) -> *const u64 { self.data("cst") as *const u64 }
    // --- sha2 ---
    pub fn sha256(&self) -> FnSha { self.sym("sha256") }
    pub fn sha512(&self) -> FnSha { self.sym("sha512") }
    pub fn sha256_inc_init(&self) -> FnShaIncInit { self.sym("sha256_inc_init") }
    pub fn sha512_inc_init(&self) -> FnShaIncInit { self.sym("sha512_inc_init") }
    pub fn sha256_inc_blocks(&self) -> FnShaIncBlocks { self.sym("sha256_inc_blocks") }
    pub fn sha512_inc_blocks(&self) -> FnShaIncBlocks { self.sym("sha512_inc_blocks") }
    pub fn sha256_inc_finalize(&self) -> FnShaIncFinalize { self.sym("sha256_inc_finalize") }
    pub fn sha512_inc_finalize(&self) -> FnShaIncFinalize { self.sym("sha512_inc_finalize") }
    pub fn mgf1_256(&self) -> FnMgf1 { self.sym("SPX_mgf1_256") }
    pub fn mgf1_512(&self) -> FnMgf1 { self.sym("SPX_mgf1_512") }
    pub fn seed_state(&self) -> FnCtx { self.sym("SPX_seed_state") }
    // --- shake ---
    pub fn shake256(&self) -> FnShake256 { self.sym("shake256") }
    pub fn shake256_absorb(&self) -> FnShakeAbsorb { self.sym("shake256_absorb") }
    pub fn shake256_squeezeblocks(&self) -> FnShakeSqueezeblocks { self.sym("shake256_squeezeblocks") }
    pub fn shake256_inc_init(&self) -> FnShakeIncInit { self.sym("shake256_inc_init") }
    pub fn shake256_inc_absorb(&self) -> FnShakeIncAbsorb { self.sym("shake256_inc_absorb") }
    pub fn shake256_inc_finalize(&self) -> FnShakeIncFinalize { self.sym("shake256_inc_finalize") }
    pub fn shake256_inc_squeeze(&self) -> FnShakeIncSqueeze { self.sym("shake256_inc_squeeze") }
    // --- haraka ---
    pub fn tweak_constants(&self) -> FnCtx { self.sym("SPX_tweak_constants") }
    pub fn haraka256(&self) -> FnHarakaPerm { self.sym("SPX_haraka256") }
    pub fn haraka512(&self) -> FnHarakaPerm { self.sym("SPX_haraka512") }
    pub fn haraka512_perm(&self) -> FnHarakaPerm { self.sym("SPX_haraka512_perm") }
    pub fn haraka_S(&self) -> FnHarakaS { self.sym("SPX_haraka_S") }
    pub fn haraka_S_inc_init(&self) -> FnHarakaSIncInit { self.sym("SPX_haraka_S_inc_init") }
    pub fn haraka_S_inc_absorb(&self) -> FnHarakaSIncAbsorb { self.sym("SPX_haraka_S_inc_absorb") }
    pub fn haraka_S_inc_finalize(&self) -> FnHarakaSIncFinalize { self.sym("SPX_haraka_S_inc_finalize") }
    pub fn haraka_S_inc_squeeze(&self) -> FnHarakaSIncSqueeze { self.sym("SPX_haraka_S_inc_squeeze") }
}

pub struct Libs {
    pub c: Impl,
    pub r: Impl,
    /// Kept alive so OpenSSL stays loaded for the whole test run.
    _ossl: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();
static SERIAL: Mutex<()> = Mutex::new(());

/// Serialises the tests: `DRBG_ctx` is process-global mutable state in both
/// implementations, so tests must not interleave.
pub fn serial() -> MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("SPHINCS_RUST_SO") {
        return PathBuf::from(p);
    }
    let r = crate_root();
    for c in ["release", "debug"] {
        let p = r.join("target").join(c).join("libsphincsplus.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libsphincsplus.so not found; run `cargo build --release` first");
}

fn c_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SPHINCS_C_DIR") {
        return PathBuf::from(p);
    }
    crate_root()
        .join("cbuild")
        .join(format!("{BACKEND}-{THASH}-{SECPAR}"))
}

fn open(path: &Path, flags: i32) -> Library {
    unsafe { Library::open(Some(path), flags) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
}

impl Libs {
    pub fn get() -> &'static Libs {
        LIBS.get_or_init(|| {
            // (1) Rust FIRST, RTLD_LOCAL: all of its relocations bind to itself
            //     because nothing else in the process defines these symbols yet,
            //     and it never becomes visible to the C libraries.
            let rust = open(&rust_so(), RTLD_NOW | RTLD_LOCAL);

            // (2) the C backend, then the C core, both RTLD_GLOBAL so that
            //     libsphincs_core_det.so can resolve SPX_thash/SPX_prf_addr/...
            //     and lib<backend>.so can resolve SPX_set_tree_index/... from
            //     the core.  The dependency is *circular*, so RTLD_LAZY is
            //     required: both objects are in the global scope before any C
            //     function is called, so every lazy binding resolves inside the
            //     C pair (never into the RTLD_LOCAL Rust object).
            let d = c_dir();
            let backend = d.join("lib").join(BACKEND).join(format!("lib{BACKEND}.so"));
            let core = d.join("app").join("libsphincs_core_det.so");
            assert!(
                backend.exists(),
                "missing {} — run ./build_c_all.sh",
                backend.display()
            );
            assert!(
                core.exists(),
                "missing {} — run ./build_c_all.sh",
                core.display()
            );
            let cb = open(&backend, RTLD_LAZY | RTLD_GLOBAL);
            let cc = open(&core, RTLD_LAZY | RTLD_GLOBAL);

            // (3) OpenSSL, needed by rng.c's AES256_ECB.  CMake links `crypto`
            //     only into the `driver` executable, so libsphincs_core_det.so
            //     has no DT_NEEDED for it and we must provide it ourselves.
            //     Loaded *last* so it can never shadow a SPHINCS+ symbol.
            let mut ossl = None;
            for cand in ["libcrypto.so.3", "libcrypto.so.1.1", "libcrypto.so"] {
                if let Ok(l) = unsafe { Library::open(Some(cand), RTLD_LAZY | RTLD_GLOBAL) } {
                    ossl = Some(l);
                    break;
                }
            }
            let ossl = ossl.expect("libcrypto not found; rng.c's AES256_ECB needs OpenSSL");

            let libs = Libs {
                // core first: for the symbols present in both C libraries
                // (utils.c is compiled into both) either copy is the same code.
                c: Impl { name: "C", libs: vec![cc, cb] },
                r: Impl { name: "Rust", libs: vec![rust] },
                _ossl: ossl,
            };

            // Sanity: the two implementations must be *distinct* objects.
            let a = libs.c.thash() as usize;
            let b = libs.r.thash() as usize;
            assert_ne!(a, b, "C and Rust SPX_thash resolved to the same address — \
                              symbol interposition happened, the test would be vacuous");
            let a = libs.c.crypto_sign_verify() as usize;
            let b = libs.r.crypto_sign_verify() as usize;
            assert_ne!(a, b, "C and Rust crypto_sign_verify resolved to the same address");
            // Sanity: both libraries really are the configuration we think
            // (this catches a stale `target/release/libsphincsplus.so`, which
            // `cargo test` does NOT rebuild — use ./run_all.sh).
            let cb_ = unsafe { (libs.c.crypto_sign_bytes())() };
            assert_eq!(
                cb_, SPX_BYTES as u64,
                "the C .so in {} is not the {BACKEND}/{THASH}/{SECPAR} build",
                c_dir().display()
            );
            let rb_ = unsafe { (libs.r.crypto_sign_bytes())() };
            assert_eq!(
                rb_, SPX_BYTES as u64,
                "{} is not the {BACKEND}/{THASH}/{SECPAR} build \
                 (run `cargo build --release --no-default-features --features \"{BACKEND} {THASH} {SECPAR}\"` \
                 or use ./run_all.sh)",
                rust_so().display()
            );
            // Sanity: the backend actually compiled into the Rust .so must
            // match the feature set (a stale .so from another backend would
            // otherwise silently skip the backend-specific rows).
            let marker = if IS_BLAKE { "blake256" }
                    else if IS_SHA2 { "sha256" }
                    else if IS_SHAKE { "shake256" }
                    else { "SPX_haraka512" };
            assert!(libs.r.has(marker),
                "{} does not export `{marker}`: it is not a {BACKEND} build",
                rust_so().display());
            libs
        })
    }
}

pub fn libs() -> &'static Libs {
    Libs::get()
}

// ===========================================================================
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ===========================================================================

pub const TEST_SEED: u64 = 0x5150_4849_4E43_5321;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        let mut i = 0;
        while i < out.len() {
            let v = self.next_u64().to_le_bytes();
            let n = std::cmp::min(8, out.len() - i);
            out[i..i + n].copy_from_slice(&v[..n]);
            i += n;
        }
    }
    pub fn vec(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for x in a.iter_mut() {
            *x = self.next_u32();
        }
        a
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
}

// ===========================================================================
// Assertion helpers
// ===========================================================================

#[track_caller]
pub fn eqb(what: &str, ctx: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(std::cmp::min(c.len(), r.len()));
        panic!(
            "[{BACKEND}/{THASH}/{SECPAR}] {what} MISMATCH ({ctx})\n  \
             len C={} R={} first differing byte at {}\n  C: {}\n  R: {}",
            c.len(),
            r.len(),
            first,
            hex_around(c, first),
            hex_around(r, first)
        );
    }
}

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, ctx: &str, c: T, r: T) {
    if c != r {
        panic!("[{BACKEND}/{THASH}/{SECPAR}] {what} MISMATCH ({ctx})\n  C: {c:?}\n  R: {r:?}");
    }
}

fn hex_around(b: &[u8], at: usize) -> String {
    let lo = at.saturating_sub(4);
    let hi = std::cmp::min(b.len(), at + 12);
    let mut s = String::new();
    if lo > 0 {
        s.push_str("…");
    }
    for x in &b[lo..hi] {
        s.push_str(&format!("{x:02x}"));
    }
    if hi < b.len() {
        s.push_str("…");
    }
    s
}

/// Builds two freshly initialised contexts (one per implementation) from the
/// same seeds, going through each implementation's own
/// `SPX_initialize_hash_function`, and asserts the observable images agree.
pub fn make_ctx(pub_seed: &[u8], sk_seed: &[u8]) -> (Box<CtxBuf>, Box<CtxBuf>) {
    let l = libs();
    let mut cc = CtxBuf::new();
    let mut rc = CtxBuf::new();
    cc.set_seeds(pub_seed, sk_seed);
    rc.set_seeds(pub_seed, sk_seed);
    unsafe {
        (l.c.initialize_hash_function())(cc.as_mut_ptr());
        (l.r.initialize_hash_function())(rc.as_mut_ptr());
    }
    eqb("initialize_hash_function(ctx)", "ctx image", cc.live(), rc.live());
    (cc, rc)
}

/// Number of randomized iterations per row; the "small" parameter sets are much
/// slower for the whole-tree routines so the heavy tests scale themselves.
pub fn iters(default: usize) -> usize {
    if let Ok(v) = std::env::var("SPHINCS_ITERS") {
        if let Ok(n) = v.parse::<usize>() {
            return n;
        }
    }
    default
}

/// `true` for the parameter sets whose full-tree operations are expensive.
pub const SLOW_PARAMS: bool = SPX_TREE_HEIGHT >= 8 || SPX_FORS_HEIGHT >= 12;
