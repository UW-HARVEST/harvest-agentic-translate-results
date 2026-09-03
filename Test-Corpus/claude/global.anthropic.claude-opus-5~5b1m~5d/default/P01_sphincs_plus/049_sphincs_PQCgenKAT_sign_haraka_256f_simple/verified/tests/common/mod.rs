//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH implementations are reached only through `dlopen`/`dlsym` on their
//! shared objects — the Rust side is *never* called through the linked rlib, so
//! the `#[no_mangle] extern "C"` export wrappers are part of what is under test.
//!
//! * C side   : `../cbuild/<combo>/libcsphincs_all.so`
//!   (all the translation units CMake puts into `libsphincs_core_det.so` +
//!   `lib<backend>.so`, linked into one object so the two libraries' circular
//!   symbol references do not have to be resolved through `RTLD_GLOBAL`;
//!   `build_c_all.sh` asserts the exported symbol set is identical.)
//! * Rust side: `../rbuild/<combo>/libsphincs_core_det.so`
//!
//! Both are opened `RTLD_NOW | RTLD_LOCAL` so that the two definitions of e.g.
//! `crypto_sign` can coexist in one process without interposing on each other.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]

use libloading::os::unix::{Library, Symbol};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

/// `DRBG_ctx` is a *process-global* in both libraries, so any test that seeds it
/// or consumes randomness from it must not run concurrently with another such
/// test.  Cargo runs `#[test]`s on multiple threads by default, hence this lock.
static DRBG_LOCK: Mutex<()> = Mutex::new(());

#[must_use]
pub fn drbg_guard() -> MutexGuard<'static, ()> {
    DRBG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Which configuration are we compiled for?  Mirrors the precedence in
// src/backend/mod.rs, src/backend/thash_*.rs and src/params.rs exactly.
// ---------------------------------------------------------------------------

pub const BACKEND: &str = if cfg!(feature = "sha2") {
    "sha2"
} else if cfg!(feature = "shake") {
    "shake"
} else if cfg!(feature = "blake") {
    "blake"
} else {
    "haraka"
};

pub const THASH: &str = if cfg!(feature = "simple") {
    "simple"
} else {
    "robust"
};

pub const SECPAR: &str = if cfg!(feature = "256f") {
    "256f"
} else if cfg!(feature = "256s") {
    "256s"
} else if cfg!(feature = "192f") {
    "192f"
} else if cfg!(feature = "192s") {
    "192s"
} else if cfg!(feature = "128f") {
    "128f"
} else {
    "128s"
};

// ---------------------------------------------------------------------------
// Parameters.  Taken from the crate's own `params` module (which is compiled
// with the very same features) and cross-checked at run time against the four
// `crypto_sign_*bytes()` entry points of BOTH shared objects, so a wrong
// constant here cannot silently weaken a test.
// ---------------------------------------------------------------------------

pub use sphincs_core_det::params::{
    CRYPTO_SEEDBYTES, SPX_ADDR_BYTES, SPX_BYTES, SPX_D, SPX_FORS_BYTES, SPX_FORS_HEIGHT,
    SPX_FORS_MSG_BYTES, SPX_FORS_TREES, SPX_FULL_HEIGHT, SPX_N, SPX_PK_BYTES, SPX_SK_BYTES,
    SPX_TREE_HEIGHT, SPX_WOTS_BYTES, SPX_WOTS_LEN, SPX_WOTS_W,
};

/// `SPX_BLAKE512` / `SPX_SHA512` — 1 for the 192/256-bit parameter sets.
pub const BIG_HASH: bool = cfg!(spx_big_hash);

/// `sizeof(spx_ctx)`, recomputed from `context.h` independently of the Rust
/// struct so that a layout regression in `SpxCtx` is caught rather than hidden.
pub const CTX_BYTES: usize = if cfg!(feature = "sha2") {
    2 * SPX_N + 40 + if BIG_HASH { 72 } else { 0 }
} else if cfg!(feature = "shake") || cfg!(feature = "blake") {
    2 * SPX_N
} else {
    // haraka: uint64_t[10][8] + uint32_t[10][8]; 2*SPX_N is already 8-aligned
    2 * SPX_N + 640 + 320
};

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BYTES: usize = (SPX_TREE_HEIGHT + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, so every run is reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.byte();
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
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
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

// ---------------------------------------------------------------------------
// C struct mirrors
// ---------------------------------------------------------------------------

/// `leaf_info_x1` from `app/include/wotsx1.h`.
#[repr(C)]
#[derive(Clone, Copy)]
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
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: core::ptr::null_mut(),
            leaf_addr: [0; 8],
            pk_addr: [0; 8],
        }
    }
    /// The parts of the struct the callee may mutate (`leaf_addr`, `pk_addr`).
    pub fn addrs(&self) -> ([u32; 8], [u32; 8]) {
        (self.leaf_addr, self.pk_addr)
    }
}

/// `fors_gen_leaf_info` from `app/include/fors.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

/// `AES_XOF_struct` from `app/include/rng.h`.
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
}

/// `AES256_CTR_DRBG_struct` from `app/include/rng.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Drbg {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

/// `blakestate256` from `lib/blake/include/blake.h`.
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
}

/// `blakestate512` from `lib/blake/include/blake.h`.
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
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub fn combo() -> String {
    format!("{}-{}-{}", BACKEND, THASH, SECPAR)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

fn open(path: &PathBuf) -> Library {
    assert!(
        path.exists(),
        "shared object {} does not exist.\n\
         Run ./build_c_all.sh and ./build_rust_all.sh from the repository root \
         (or use ./run_all.sh, which does both) before running the tests.",
        path.display()
    );
    // RTLD_NOW so a missing symbol fails loudly here rather than at first call;
    // RTLD_LOCAL so the C and the Rust definitions of identically-named symbols
    // stay in separate namespaces.
    unsafe { Library::open(Some(path), libc_RTLD_NOW | libc_RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
}

// libloading does not re-export the flags; these are the glibc/x86-64 values.
const libc_RTLD_NOW: i32 = 0x2;
const libc_RTLD_LOCAL: i32 = 0x0;

/// One loaded implementation (either the C one or the Rust one).
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    pub fn sym<T>(&self, name: &str) -> Symbol<T> {
        unsafe { self.lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: dlsym({name}) failed: {e}", self.name))
    }
    /// Address of a data symbol (e.g. `DRBG_ctx`, `cst`).
    ///
    /// `libloading`'s unix `Symbol<T>` reinterprets its stored `dlsym` result as
    /// `T`, so `Symbol<*mut T>` dereferences to the address of the object.
    pub fn data<T>(&self, name: &str) -> *mut T {
        let s: Symbol<*mut T> = unsafe { self.lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: dlsym({name}) failed: {e}", self.name));
        *s
    }
}

/// The C implementation and the Rust implementation, side by side.
pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

pub fn load() -> Pair {
    let root = repo_root();
    let combo = combo();

    let c_path = std::env::var("SPX_C_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("cbuild").join(&combo).join("libcsphincs_all.so"));
    let r_path = std::env::var("SPX_RUST_LIB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            root.join("rbuild")
                .join(&combo)
                .join("libsphincs_core_det.so")
        });

    let pair = Pair {
        c: Impl {
            name: "C",
            lib: open(&c_path),
        },
        r: Impl {
            name: "Rust",
            lib: open(&r_path),
        },
    };

    // Sanity gate: the four size constants must agree between the two shared
    // objects AND with the constants this harness sizes its buffers with.
    for imp in [&pair.c, &pair.r] {
        let f: Symbol<unsafe extern "C" fn() -> u64> = imp.sym("crypto_sign_secretkeybytes");
        assert_eq!(
            unsafe { f() } as usize,
            SPX_SK_BYTES,
            "{}: crypto_sign_secretkeybytes mismatch for {combo}",
            imp.name
        );
        let f: Symbol<unsafe extern "C" fn() -> u64> = imp.sym("crypto_sign_publickeybytes");
        assert_eq!(unsafe { f() } as usize, SPX_PK_BYTES, "{}", imp.name);
        let f: Symbol<unsafe extern "C" fn() -> u64> = imp.sym("crypto_sign_bytes");
        assert_eq!(unsafe { f() } as usize, SPX_BYTES, "{}", imp.name);
        let f: Symbol<unsafe extern "C" fn() -> u64> = imp.sym("crypto_sign_seedbytes");
        assert_eq!(unsafe { f() } as usize, CRYPTO_SEEDBYTES, "{}", imp.name);
    }

    pair
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(2 * b.len().min(64));
    for (i, x) in b.iter().enumerate() {
        if i == 64 {
            s.push_str("...");
            break;
        }
        s.push_str(&format!("{x:02x}"));
    }
    s
}

#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.len().min(r.len()));
        panic!(
            "{what} differs (config {}) at byte {first}\n  C   : {}\n  Rust: {}",
            combo(),
            hex(c),
            hex(r)
        );
    }
}

#[track_caller]
pub fn eq_u32s(what: &str, c: &[u32], r: &[u32]) {
    assert_eq!(c, r, "{what} differs (config {})", combo());
}

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(c, r, "{what} differs (config {})", combo());
}

/// A freshly zeroed `spx_ctx`-sized byte buffer, 8-byte aligned (the haraka
/// variant contains `uint64_t` arrays).
pub fn new_ctx_buf() -> CtxBuf {
    CtxBuf {
        words: vec![0u64; (CTX_BYTES + 7) / 8],
    }
}

pub struct CtxBuf {
    words: Vec<u64>,
}

impl CtxBuf {
    pub fn as_ptr(&self) -> *const u8 {
        self.words.as_ptr() as *const u8
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), CTX_BYTES) }
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), CTX_BYTES) }
    }
    /// Write `pub_seed` and `sk_seed` (the only fields a caller sets in C).
    pub fn set_seeds(&mut self, pub_seed: &[u8], sk_seed: &[u8]) {
        self.bytes_mut()[..SPX_N].copy_from_slice(&pub_seed[..SPX_N]);
        self.bytes_mut()[SPX_N..2 * SPX_N].copy_from_slice(&sk_seed[..SPX_N]);
    }
}

impl Clone for CtxBuf {
    fn clone(&self) -> Self {
        CtxBuf {
            words: self.words.clone(),
        }
    }
}

/// Build a `spx_ctx` in BOTH libraries from the same seeds, running each
/// library's own `SPX_initialize_hash_function`, and assert the two resulting
/// context images are byte-identical.  Returns the (c_ctx, r_ctx) pair.
pub fn init_ctx_pair(p: &Pair, rng: &mut Rng) -> (CtxBuf, CtxBuf) {
    let pub_seed = rng.bytes(SPX_N);
    let sk_seed = rng.bytes(SPX_N);
    init_ctx_pair_from(p, &pub_seed, &sk_seed)
}

pub fn init_ctx_pair_from(p: &Pair, pub_seed: &[u8], sk_seed: &[u8]) -> (CtxBuf, CtxBuf) {
    let mut cc = new_ctx_buf();
    let mut rc = new_ctx_buf();
    cc.set_seeds(pub_seed, sk_seed);
    rc.set_seeds(pub_seed, sk_seed);
    type InitFn = unsafe extern "C" fn(*mut u8);
    let f: Symbol<InitFn> = p.c.sym("SPX_initialize_hash_function");
    unsafe { f(cc.as_mut_ptr()) };
    let f: Symbol<InitFn> = p.r.sym("SPX_initialize_hash_function");
    unsafe { f(rc.as_mut_ptr()) };
    eq_bytes("spx_ctx after initialize_hash_function", cc.bytes(), rc.bytes());
    (cc, rc)
}

/// Seed both libraries' DRBG (`randombytes_init`) with the same input so that
/// subsequent `randombytes()`-consuming entry points are comparable.
pub fn seed_drbg(p: &Pair, entropy: &[u8; 48], pers: Option<&[u8; 48]>) {
    type InitFn = unsafe extern "C" fn(*mut u8, *mut u8);
    let mut e = *entropy;
    let mut ps = pers.copied().unwrap_or([0u8; 48]);
    let ps_ptr = if pers.is_some() {
        ps.as_mut_ptr()
    } else {
        core::ptr::null_mut()
    };
    for imp in [&p.c, &p.r] {
        let f: Symbol<InitFn> = imp.sym("randombytes_init");
        unsafe { f(e.as_mut_ptr(), ps_ptr) };
    }
    let _ = &mut e;
    let _ = &mut ps;
}

/// Read the `DRBG_ctx` global out of one implementation.
pub fn read_drbg(imp: &Impl) -> Drbg {
    unsafe { *imp.data::<Drbg>("DRBG_ctx") }
}

pub fn eq_drbg(p: &Pair, what: &str) {
    eq(what, read_drbg(&p.c), read_drbg(&p.r));
}
