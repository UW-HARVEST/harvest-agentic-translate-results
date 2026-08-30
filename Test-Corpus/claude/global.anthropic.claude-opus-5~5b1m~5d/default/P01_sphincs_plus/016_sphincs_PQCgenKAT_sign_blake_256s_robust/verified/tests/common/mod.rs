//! Shared harness for the C-vs-Rust differential tests.
//!
//! IMPORTANT: this module deliberately does **not** reference the
//! `sphincs_plus` crate.  Both implementations are reached exclusively through
//! `dlopen`/`dlsym` on their shared objects, so the `#[no_mangle]` export
//! wrappers are exercised exactly like an external C consumer would, and no
//! Rust symbol from the crate ends up in the test executable where it could
//! interpose on the C library's own relocations.

#![allow(dead_code)]

use libloading::os::unix::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

// ------------------------------------------------------------------
// Build configuration, re-derived independently from the C headers.
// ------------------------------------------------------------------

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

// params-sphincs-<backend>-<secpar>.h
pub const N: usize = if cfg!(any(spx_secpar = "128s", spx_secpar = "128f")) {
    16
} else if cfg!(any(spx_secpar = "192s", spx_secpar = "192f")) {
    24
} else {
    32
};
pub const FULL_HEIGHT: usize = if cfg!(any(spx_secpar = "128s", spx_secpar = "192s")) {
    63
} else if cfg!(any(spx_secpar = "128f", spx_secpar = "192f")) {
    66
} else if cfg!(spx_secpar = "256s") {
    64
} else {
    68
};
pub const D: usize = if cfg!(any(spx_secpar = "128s", spx_secpar = "192s")) {
    7
} else if cfg!(any(spx_secpar = "128f", spx_secpar = "192f")) {
    22
} else if cfg!(spx_secpar = "256s") {
    8
} else {
    17
};
pub const FORS_HEIGHT: usize = if cfg!(spx_secpar = "128s") {
    12
} else if cfg!(spx_secpar = "128f") {
    6
} else if cfg!(spx_secpar = "192s") {
    14
} else if cfg!(spx_secpar = "192f") {
    8
} else if cfg!(spx_secpar = "256s") {
    14
} else {
    9
};
pub const FORS_TREES: usize = if cfg!(spx_secpar = "128s") {
    14
} else if cfg!(spx_secpar = "128f") {
    33
} else if cfg!(spx_secpar = "192s") {
    17
} else if cfg!(spx_secpar = "192f") {
    33
} else if cfg!(spx_secpar = "256s") {
    22
} else {
    35
};

pub const WOTS_W: usize = 16;
pub const WOTS_LOGW: usize = 4;
pub const WOTS_LEN1: usize = 8 * N / WOTS_LOGW;
pub const WOTS_LEN2: usize = 3; // N in {16,24,32} => 8 < N <= 136 => 3
pub const WOTS_LEN: usize = WOTS_LEN1 + WOTS_LEN2;
pub const WOTS_BYTES: usize = WOTS_LEN * N;
pub const ADDR_BYTES: usize = 32;
pub const TREE_HEIGHT: usize = FULL_HEIGHT / D;
pub const FORS_MSG_BYTES: usize = (FORS_HEIGHT * FORS_TREES + 7) / 8;
pub const FORS_BYTES: usize = (FORS_HEIGHT + 1) * FORS_TREES * N;
pub const SPX_BYTES: usize = N + FORS_BYTES + D * WOTS_BYTES + FULL_HEIGHT * N;
pub const PK_BYTES: usize = 2 * N;
pub const SK_BYTES: usize = 2 * N + PK_BYTES;
pub const SEED_BYTES: usize = 3 * N;

pub const TREE_BITS: usize = TREE_HEIGHT * (D - 1);
pub const TREE_BYTES: usize = (TREE_BITS + 7) / 8;
pub const LEAF_BITS: usize = TREE_HEIGHT;
pub const LEAF_BYTES: usize = (LEAF_BITS + 7) / 8;
pub const DGST_BYTES: usize = FORS_MSG_BYTES + TREE_BYTES + LEAF_BYTES;

// <backend>_offsets.h
pub const OFFSET_LAYER: usize = if cfg!(spx_backend = "sha2") { 0 } else { 3 };
pub const OFFSET_TREE: usize = if cfg!(spx_backend = "sha2") { 1 } else { 8 };
pub const OFFSET_TYPE: usize = if cfg!(spx_backend = "sha2") { 9 } else { 19 };
pub const OFFSET_KP_ADDR: usize = if cfg!(spx_backend = "sha2") { 10 } else { 20 };
pub const OFFSET_CHAIN_ADDR: usize = if cfg!(spx_backend = "sha2") { 17 } else { 27 };
pub const OFFSET_HASH_ADDR: usize = if cfg!(spx_backend = "sha2") { 21 } else { 31 };
pub const OFFSET_TREE_HGT: usize = if cfg!(spx_backend = "sha2") { 17 } else { 27 };
pub const OFFSET_TREE_INDEX: usize = if cfg!(spx_backend = "sha2") { 18 } else { 28 };

pub const ADDR_TYPE_WOTS: u32 = 0;
pub const ADDR_TYPE_WOTSPK: u32 = 1;
pub const ADDR_TYPE_HASHTREE: u32 = 2;
pub const ADDR_TYPE_FORSTREE: u32 = 3;
pub const ADDR_TYPE_FORSPK: u32 = 4;
pub const ADDR_TYPE_WOTSPRF: u32 = 5;
pub const ADDR_TYPE_FORSPRF: u32 = 6;

pub const HAS_SHA512: bool = cfg!(spx_sha512);
pub const HAS_BLAKE512: bool = cfg!(spx_blake512);

/// `sizeof(spx_ctx)`, derived from `app/include/context.h`.
pub const CTX_SIZE: usize = if cfg!(spx_backend = "sha2") {
    2 * N + 40 + if cfg!(spx_sha512) { 72 } else { 0 }
} else if cfg!(spx_backend = "haraka") {
    2 * N + 10 * 8 * 8 + 10 * 8 * 4
} else {
    2 * N
};

// ------------------------------------------------------------------
// Library loading
// ------------------------------------------------------------------

const RTLD_LAZY: i32 = 0x1;
const RTLD_NOW: i32 = 0x2;
const RTLD_GLOBAL: i32 = 0x100;
const RTLD_LOCAL: i32 = 0;

pub struct Libs {
    /// `lib<backend>.so` — keeps the backend loaded (RTLD_GLOBAL) so that the
    /// undefined `SPX_thash`/`SPX_prf_addr`/... relocations inside
    /// `libsphincs_core.so` resolve to the C implementation.
    backend: Library,
    /// `libsphincs_core.so`
    core: Library,
    /// `libsphincs_plus.so` (the Rust cdylib)
    rust: Library,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn tag() -> String {
    format!("{}-{}-{}", BACKEND, SECPAR, THASH)
}

impl Libs {
    pub fn load() -> Libs {
        let root = repo_root();
        let tag = tag();
        let cdir = root.join(format!("c_src/build-{}", tag));
        let backend_path = cdir.join(format!("lib/{}/lib{}.so", BACKEND, BACKEND));
        // The *deterministic* core (rng.c, the NIST AES-256-CTR-DRBG) is used:
        // it is what `driver` links, and it makes `crypto_sign_keypair` /
        // `crypto_sign_signature` reproducible so they can be compared
        // byte-for-byte against the Rust cdylib (whose exported `randombytes`
        // is the same DRBG).
        let core_path = cdir.join("app/libsphincs_core_det.so");

        // The Rust cdylib for this exact feature combination.  Built by
        // verif/run_tests.sh into target/<tag>/release/.
        let rust_path = match std::env::var("SPHINCS_RUST_SO") {
            Ok(p) => PathBuf::from(p),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("target/{}/release/libsphincs_plus.so", tag)),
        };

        for p in [&backend_path, &core_path, &rust_path] {
            assert!(p.exists(), "missing shared object: {}", p.display());
        }

        // ORDER MATTERS.
        //
        // The Rust cdylib is opened FIRST, with RTLD_NOW | RTLD_LOCAL, so that
        // all of its relocations — in particular the GOT entry for its own
        // exported `static mut DRBG_ctx` — bind to its own definitions.  If the
        // C libraries were loaded first with RTLD_GLOBAL they would *interpose*
        // that data symbol and the Rust code would end up mutating the C
        // library's DRBG state, which would silently invalidate the comparison.
        let rust = unsafe { Library::open(Some(&rust_path), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("dlopen {}: {}", rust_path.display(), e));

        // `libsphincs_core_det.so` and `lib<backend>.so` reference each other's
        // symbols (core needs SPX_thash/SPX_prf_addr..., the backend needs
        // SPX_set_tree_index/...), and neither records the other as a
        // DT_NEEDED.  They therefore have to be loaded RTLD_GLOBAL with lazy
        // binding so they can satisfy each other.  `core` is loaded first so
        // that, for the symbols both define (utils.c is compiled into both the
        // sha2 and blake backends), the core copy wins — exactly what the C
        // link would do.
        let core = unsafe { Library::open(Some(&core_path), RTLD_LAZY | RTLD_GLOBAL) }
            .unwrap_or_else(|e| panic!("dlopen {}: {}", core_path.display(), e));
        let backend = unsafe { Library::open(Some(&backend_path), RTLD_LAZY | RTLD_GLOBAL) }
            .unwrap_or_else(|e| panic!("dlopen {}: {}", backend_path.display(), e));

        Libs {
            backend,
            core,
            rust,
        }
    }

    /// Look up `name` in the C libraries: `libsphincs_core.so` first (it owns
    /// the copy the C code itself binds to), then the backend library.
    pub fn c<T>(&self, name: &str) -> Symbol<T> {
        unsafe {
            if let Ok(s) = self.core.get::<T>(name.as_bytes()) {
                return s;
            }
            self.backend
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|e| panic!("C symbol {} not found: {}", name, e))
        }
    }

    /// Look up `name` in the Rust cdylib.
    pub fn r<T>(&self, name: &str) -> Symbol<T> {
        unsafe {
            self.rust
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|e| panic!("Rust symbol {} not found: {}", name, e))
        }
    }

    /// Both implementations of `name`, as `(c, rust)`.
    pub fn pair<T>(&self, name: &str) -> (Symbol<T>, Symbol<T>) {
        (self.c::<T>(name), self.r::<T>(name))
    }

    /// Address of an exported *data* symbol (e.g. `DRBG_ctx`, `cst`).
    ///
    /// `Symbol<T>` derefs to the *value* stored at the symbol, so a data
    /// symbol has to be fetched via the raw `dlsym` result instead.
    pub fn c_data(&self, name: &str) -> *mut u8 {
        self.c::<*mut c_void>(name).into_raw() as *mut u8
    }
    pub fn r_data(&self, name: &str) -> *mut u8 {
        self.r::<*mut c_void>(name).into_raw() as *mut u8
    }
}

/// Both libraries keep the CTR-DRBG in a single process-global
/// (`DRBG_ctx`), and `dlopen` on the same path returns the same handle, so
/// every test that reseeds or draws from it must hold this lock.
pub static DRBG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Acquire `DRBG_LOCK`, tolerating poisoning from an unrelated failing test.
pub fn drbg_lock() -> std::sync::MutexGuard<'static, ()> {
    DRBG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep runs reproducible.
// ------------------------------------------------------------------

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
        self.next_u64() as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u64() as u8;
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    /// A random byte string whose length is uniform in `[0, max)`.
    pub fn bytes_upto(&mut self, max: u32) -> Vec<u8> {
        let l = self.below(max) as usize;
        self.bytes(l)
    }
    /// `n` random lengths, each uniform in `[0, max)`.
    pub fn lens(&mut self, n: usize, max: u32) -> Vec<usize> {
        (0..n).map(|_| self.below(max) as usize).collect()
    }
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for x in a.iter_mut() {
            *x = self.next_u32();
        }
        a
    }
}

// ------------------------------------------------------------------
// spx_ctx helper: an 8-byte-aligned, over-sized buffer holding the C-layout
// structure.  Extra slack is kept so both implementations write inside it.
// ------------------------------------------------------------------

pub const CTX_BUF_WORDS: usize = 256; // 2048 bytes

#[repr(align(8))]
pub struct Ctx {
    pub raw: [u64; CTX_BUF_WORDS],
}

impl Ctx {
    pub fn new(pub_seed: &[u8], sk_seed: &[u8]) -> Ctx {
        let mut c = Ctx {
            raw: [0u64; CTX_BUF_WORDS],
        };
        c.bytes_mut()[..N].copy_from_slice(&pub_seed[..N]);
        c.bytes_mut()[N..2 * N].copy_from_slice(&sk_seed[..N]);
        c
    }
    pub fn zeroed() -> Ctx {
        Ctx {
            raw: [0u64; CTX_BUF_WORDS],
        }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.raw.as_ptr() as *const u8, CTX_BUF_WORDS * 8) }
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.raw.as_mut_ptr() as *mut u8, CTX_BUF_WORDS * 8)
        }
    }
    pub fn as_ptr(&self) -> *const c_void {
        self.raw.as_ptr() as *const c_void
    }
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.raw.as_mut_ptr() as *mut c_void
    }
    /// Only the bytes that actually belong to `spx_ctx`.
    pub fn live(&self) -> &[u8] {
        &self.bytes()[..CTX_SIZE]
    }
}

/// Builds a pair of contexts (one for C, one for Rust) with the same seeds and
/// runs each library's `initialize_hash_function` on it.
pub fn init_ctx_pair(libs: &Libs, pub_seed: &[u8], sk_seed: &[u8]) -> (Ctx, Ctx) {
    type InitFn = unsafe extern "C" fn(*mut c_void);
    let (ci, ri) = libs.pair::<InitFn>("SPX_initialize_hash_function");
    let mut cc = Ctx::new(pub_seed, sk_seed);
    let mut rc = Ctx::new(pub_seed, sk_seed);
    unsafe {
        ci(cc.as_mut_ptr());
        ri(rc.as_mut_ptr());
    }
    assert_eq!(
        cc.live(),
        rc.live(),
        "initialize_hash_function diverged ({}), ctx bytes differ",
        tag()
    );
    (cc, rc)
}

/// Pretty hex for failure messages.
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

pub fn assert_bytes_eq(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let at = c.iter().zip(r.iter()).position(|(a, b)| a != b);
        panic!(
            "[{}] {}: C != Rust (first diff at {:?})\n  C   = {}\n  Rust= {}",
            tag(),
            what,
            at,
            hex(c),
            hex(r)
        );
    }
}
