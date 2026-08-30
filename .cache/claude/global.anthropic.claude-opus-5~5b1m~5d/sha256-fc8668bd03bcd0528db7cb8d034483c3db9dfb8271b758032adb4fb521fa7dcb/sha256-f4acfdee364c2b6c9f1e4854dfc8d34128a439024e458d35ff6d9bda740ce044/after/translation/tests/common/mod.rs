//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and every call
//! goes through `dlsym`, so the `#[no_mangle]`/`extern "C"` export wrappers are
//! part of what is under test. No Rust function of the crate is ever called
//! directly.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

// ------------------------------------------------------------------
// Parameters, re-derived independently from the C headers (NOT imported
// from the crate, so a wrong constant in src/params.rs cannot hide here).
// ------------------------------------------------------------------

#[cfg(spx_secpar = "128s")]
pub const BASE: (usize, usize, usize, usize, usize) = (16, 63, 7, 12, 14);
#[cfg(spx_secpar = "128f")]
pub const BASE: (usize, usize, usize, usize, usize) = (16, 66, 22, 6, 33);
#[cfg(spx_secpar = "192s")]
pub const BASE: (usize, usize, usize, usize, usize) = (24, 63, 7, 14, 17);
#[cfg(spx_secpar = "192f")]
pub const BASE: (usize, usize, usize, usize, usize) = (24, 66, 22, 8, 33);
#[cfg(spx_secpar = "256s")]
pub const BASE: (usize, usize, usize, usize, usize) = (32, 64, 8, 14, 22);
#[cfg(spx_secpar = "256f")]
pub const BASE: (usize, usize, usize, usize, usize) = (32, 68, 17, 9, 35);

pub const N: usize = BASE.0;
pub const FULL_HEIGHT: usize = BASE.1;
pub const D: usize = BASE.2;
pub const FORS_HEIGHT: usize = BASE.3;
pub const FORS_TREES: usize = BASE.4;

pub const WOTS_W: usize = 16;
pub const WOTS_LOGW: usize = 4;
pub const WOTS_LEN1: usize = 8 * N / WOTS_LOGW;
pub const WOTS_LEN2: usize = if N <= 8 {
    2
} else if N <= 136 {
    3
} else {
    4
};
pub const WOTS_LEN: usize = WOTS_LEN1 + WOTS_LEN2;
pub const WOTS_BYTES: usize = WOTS_LEN * N;
pub const TREE_HEIGHT: usize = FULL_HEIGHT / D;
pub const FORS_MSG_BYTES: usize = (FORS_HEIGHT * FORS_TREES + 7) / 8;
pub const FORS_BYTES: usize = (FORS_HEIGHT + 1) * FORS_TREES * N;
pub const SPX_BYTES: usize = N + FORS_BYTES + D * WOTS_BYTES + FULL_HEIGHT * N;
pub const PK_BYTES: usize = 2 * N;
pub const SK_BYTES: usize = 4 * N;
pub const SEED_BYTES: usize = 3 * N;
pub const ADDR_BYTES: usize = 32;

pub const TREE_BITS: usize = TREE_HEIGHT * (D - 1);
pub const TREE_BYTES: usize = (TREE_BITS + 7) / 8;
pub const LEAF_BITS: usize = TREE_HEIGHT;
pub const LEAF_BYTES: usize = (LEAF_BITS + 7) / 8;
pub const DGST_BYTES: usize = FORS_MSG_BYTES + TREE_BYTES + LEAF_BYTES;

// `spx_ctx` size (context.h), used to allocate byte-identical C structs.
#[cfg(spx_backend = "sha2")]
pub const CTX_BYTES: usize = 2 * N + 40 + if N >= 24 { 72 } else { 0 };
#[cfg(spx_backend = "haraka")]
pub const CTX_BYTES: usize = 2 * N + 10 * 8 * 8 + 10 * 8 * 4;
#[cfg(any(spx_backend = "blake", spx_backend = "shake"))]
pub const CTX_BYTES: usize = 2 * N;

pub const BACKEND: &str = if cfg!(spx_backend = "haraka") {
    "haraka"
} else if cfg!(spx_backend = "sha2") {
    "sha2"
} else if cfg!(spx_backend = "shake") {
    "shake"
} else {
    "blake"
};
pub const THASH: &str = if cfg!(spx_thash = "robust") {
    "robust"
} else {
    "simple"
};
pub const SECPAR: &str = if cfg!(spx_secpar = "128s") {
    "128s"
} else if cfg!(spx_secpar = "128f") {
    "128f"
} else if cfg!(spx_secpar = "192s") {
    "192s"
} else if cfg!(spx_secpar = "192f") {
    "192f"
} else if cfg!(spx_secpar = "256s") {
    "256s"
} else {
    "256f"
};

pub fn cfg_name() -> String {
    format!("{}_{}_{}", BACKEND, SECPAR, THASH)
}

// ------------------------------------------------------------------
// Library pair
// ------------------------------------------------------------------

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

fn root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <work>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPX_C_LIB") {
        return PathBuf::from(p);
    }
    root().join(format!("cbuild/{}/libc_sphincs.so", cfg_name()))
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPX_RUST_LIB") {
        return PathBuf::from(p);
    }
    // The freshly built cdylib for the feature set this test was compiled with.
    let m = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["release", "debug"] {
        let c = m
            .join("target")
            .join(profile)
            .join("lib005_sphincs_PQCgenKAT_sign_blake_128f_simple.so");
        if c.exists() {
            return c;
        }
    }
    root().join(format!("rustlibs/librust_{}.so", cfg_name()))
}

impl Pair {
    pub fn load() -> Pair {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        assert!(cp.exists(), "C library not built: {}", cp.display());
        assert!(rp.exists(), "Rust library not built: {}", rp.display());
        let p = unsafe {
            Pair {
                c: Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {}: {e}", cp.display())),
                r: Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {}: {e}", rp.display())),
            }
        };
        // Guard against a stale artifact from a different feature set.
        p.check_config();
        p
    }

    /// Sanity check: the C library really is this configuration.
    pub fn check_config(&self) {
        unsafe {
            let cb: Symbol<unsafe extern "C" fn() -> u64> = self.c.get(b"crypto_sign_bytes\0").unwrap();
            let rb: Symbol<unsafe extern "C" fn() -> u64> = self.r.get(b"crypto_sign_bytes\0").unwrap();
            assert_eq!(cb() as usize, SPX_BYTES, "C SPX_BYTES mismatch (wrong .so for {})", cfg_name());
            assert_eq!(rb() as usize, SPX_BYTES, "Rust SPX_BYTES mismatch (wrong .so for {})", cfg_name());
        }
    }
}

/// The library pair is loaded exactly ONCE per test process and never
/// `dlclose`d. Repeatedly `dlopen`/`dlclose`ing the same objects from parallel
/// test threads is not safe: both libcrypto (inside the C `.so`) and the Rust
/// `cdylib` register `__cxa_thread_atexit_impl` destructors, and unmapping an
/// object those still point into segfaults at thread exit.
static PAIR: std::sync::OnceLock<Pair> = std::sync::OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(Pair::load)
}

/// `let f = sym!(lib, b"name\0", unsafe extern "C" fn(...))`
#[macro_export]
macro_rules! sym {
    ($lib:expr, $name:expr, $t:ty) => {{
        let s: libloading::Symbol<$t> = $lib.get($name).unwrap_or_else(|e| {
            panic!("missing symbol {}: {e}", String::from_utf8_lossy($name))
        });
        s
    }};
}

// ------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility.
// ------------------------------------------------------------------

pub struct Rng(pub u64);

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
        self.next_u64() as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = self.next_u64() as u8;
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for w in a.iter_mut() {
            *w = self.next_u32();
        }
        a
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ------------------------------------------------------------------
// spx_ctx helpers
// ------------------------------------------------------------------

/// Builds a raw `spx_ctx` byte image with `pub_seed`/`sk_seed` filled in and
/// the backend-specific tail zeroed, then runs the library's own
/// `SPX_initialize_hash_function` on it.
pub unsafe fn make_ctx(lib: &Library, pub_seed: &[u8], sk_seed: &[u8]) -> Vec<u8> {
    let mut ctx = vec![0u8; CTX_BYTES];
    ctx[..N].copy_from_slice(&pub_seed[..N]);
    ctx[N..2 * N].copy_from_slice(&sk_seed[..N]);
    let f = sym!(lib, b"SPX_initialize_hash_function\0", unsafe extern "C" fn(*mut u8));
    f(ctx.as_mut_ptr());
    ctx
}

/// Asserts two byte buffers are identical, printing a compact diff.
pub fn eqb(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(usize::MAX);
        panic!(
            "[{}] {} mismatch (len C={} R={}) first differing byte at {}\n C={:02x?}\n R={:02x?}",
            cfg_name(),
            what,
            c.len(),
            r.len(),
            first,
            &c[..c.len().min(96)],
            &r[..r.len().min(96)]
        );
    }
}

pub fn eqv<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(c, r, "[{}] {} mismatch", cfg_name(), what);
}

/// Output buffers are always over-allocated with a canary tail so that a
/// library writing MORE bytes than expected cannot corrupt the heap — and so
/// that the over-write itself shows up as a diff (the C `hash_blake.c`
/// `gen_message_random` really does write the full 32/64-byte digest to `R`).
pub const SLACK: usize = 96;

pub fn obuf(n: usize) -> Vec<u8> {
    vec![0xA5u8; n + SLACK]
}

/// `dlopen` of the same path returns the same mapping, so every test in a
/// binary shares the libraries' process-wide `DRBG_ctx`. Tests that touch the
/// DRBG must therefore hold this lock (cargo runs tests in parallel threads).
static DRBG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn drbg_lock() -> std::sync::MutexGuard<'static, ()> {
    match DRBG_LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Address of an exported *data* symbol.
pub unsafe fn data_ptr(lib: &Library, name: &[u8]) -> *mut u8 {
    // libloading transmutes the raw `dlsym` address to `T`, so with
    // `T = *mut u8` the dereferenced symbol *is* the object's address.
    let s: Symbol<*mut u8> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing data symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

/// `AES256_CTR_DRBG_struct` is `{ u8 Key[32]; u8 V[16]; int reseed_counter; }`.
pub const DRBG_BYTES: usize = 52;

pub unsafe fn drbg_image(lib: &Library) -> Vec<u8> {
    let p = data_ptr(lib, b"DRBG_ctx\0");
    core::slice::from_raw_parts(p, DRBG_BYTES).to_vec()
}

pub fn addr_to_bytes(a: &[u32; 8]) -> [u8; 32] {
    let mut o = [0u8; 32];
    for i in 0..8 {
        o[4 * i..4 * i + 4].copy_from_slice(&a[i].to_ne_bytes());
    }
    o
}
