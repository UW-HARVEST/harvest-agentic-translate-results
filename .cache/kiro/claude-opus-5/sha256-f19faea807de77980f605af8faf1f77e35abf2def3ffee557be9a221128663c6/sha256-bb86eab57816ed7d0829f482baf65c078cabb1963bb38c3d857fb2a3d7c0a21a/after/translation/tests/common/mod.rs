//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are reached **only** through their shared objects,
//! loaded with `libloading`, so the `#[no_mangle]` export wrappers are part of
//! what is under test.
//!
//! * Rust: `translation/target/release/libsphincsplus.so` (override with
//!   `SPHINCS_RUST_SO`).
//! * C:   `c_build/<backend>_<thash>_<secpar>/app/libsphincs_core_det.so` plus
//!   `c_build/<backend>_<thash>_<secpar>/lib/<backend>/lib<backend>.so`
//!   (override the directory with `SPHINCS_C_DIR`).  The backend `.so` is
//!   dlopen'ed with `RTLD_GLOBAL` first so that the core's undefined
//!   `SPX_thash` / `SPX_prf_addr` / … resolve; the Rust `.so` is loaded
//!   `RTLD_LOCAL` (it has zero undefined non-libc symbols, verified with
//!   `nm -D -u`, so it cannot be interposed by the C copies).

#![allow(dead_code)]

use libloading::os::unix::{Library as UnixLibrary, Symbol as UnixSymbol, RTLD_GLOBAL, RTLD_LAZY};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Build configuration, re-derived independently from build.rs's cfgs
// ---------------------------------------------------------------------------

pub const BACKEND: &str = if cfg!(backend_blake) {
    "blake"
} else if cfg!(backend_haraka) {
    "haraka"
} else if cfg!(backend_sha2) {
    "sha2"
} else {
    "shake"
};

pub const THASH: &str = if cfg!(thash_simple) { "simple" } else { "robust" };

pub const SECPAR: &str = if cfg!(secpar_128s) {
    "128s"
} else if cfg!(secpar_128f) {
    "128f"
} else if cfg!(secpar_192s) {
    "192s"
} else if cfg!(secpar_192f) {
    "192f"
} else if cfg!(secpar_256s) {
    "256s"
} else {
    "256f"
};

pub const IS_SHA2: bool = cfg!(backend_sha2);
pub const IS_HARAKA: bool = cfg!(backend_haraka);
pub const IS_BLAKE: bool = cfg!(backend_blake);
pub const IS_SHAKE: bool = cfg!(backend_shake);
pub const IS_ROBUST: bool = cfg!(thash_robust);
/// `urandom` Cargo feature: `randombytes()` comes from `randombytes.c`
/// (`libsphincs_core.so`) instead of `rng.c` (`libsphincs_core_det.so`).
pub const RAND_URANDOM: bool = cfg!(rand_urandom);

// --- params-sphincs-<backend>-<secpar>.h ------------------------------------

pub const SPX_N: usize = if cfg!(any(secpar_128s, secpar_128f)) {
    16
} else if cfg!(any(secpar_192s, secpar_192f)) {
    24
} else {
    32
};

pub const SPX_FULL_HEIGHT: usize = if cfg!(secpar_128s) {
    63
} else if cfg!(secpar_128f) {
    66
} else if cfg!(secpar_192s) {
    63
} else if cfg!(secpar_192f) {
    66
} else if cfg!(secpar_256s) {
    64
} else {
    68
};

pub const SPX_D: usize = if cfg!(secpar_128s) {
    7
} else if cfg!(secpar_128f) {
    22
} else if cfg!(secpar_192s) {
    7
} else if cfg!(secpar_192f) {
    22
} else if cfg!(secpar_256s) {
    8
} else {
    17
};

pub const SPX_FORS_HEIGHT: usize = if cfg!(secpar_128s) {
    12
} else if cfg!(secpar_128f) {
    6
} else if cfg!(secpar_192s) {
    14
} else if cfg!(secpar_192f) {
    8
} else if cfg!(secpar_256s) {
    14
} else {
    9
};

pub const SPX_FORS_TREES: usize = if cfg!(secpar_128s) {
    14
} else if cfg!(secpar_128f) {
    33
} else if cfg!(secpar_192s) {
    17
} else if cfg!(secpar_192f) {
    33
} else if cfg!(secpar_256s) {
    22
} else {
    35
};

pub const N_GE_24: bool = SPX_N >= 24;

pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
/// `SPX_N` is 16/24/32, all in `(8, 136]`, so `SPX_WOTS_LEN2` is always 3.
pub const SPX_WOTS_LEN2: usize = 3;
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BYTES: usize = (SPX_TREE_HEIGHT + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// --- <backend>_offsets.h ----------------------------------------------------

pub const OFF_LAYER: usize = if IS_SHA2 { 0 } else { 3 };
pub const OFF_TREE: usize = if IS_SHA2 { 1 } else { 8 };
pub const OFF_TYPE: usize = if IS_SHA2 { 9 } else { 19 };
pub const OFF_KP_ADDR: usize = if IS_SHA2 { 10 } else { 20 };
pub const OFF_CHAIN_ADDR: usize = if IS_SHA2 { 17 } else { 27 };
pub const OFF_HASH_ADDR: usize = if IS_SHA2 { 21 } else { 31 };
pub const OFF_TREE_HGT: usize = if IS_SHA2 { 17 } else { 27 };
pub const OFF_TREE_INDEX: usize = if IS_SHA2 { 18 } else { 28 };

/// `sizeof(spx_ctx)` for the active configuration (see `app/include/context.h`).
pub const CTX_BYTES: usize = 2 * SPX_N
    + if IS_SHA2 {
        40 + if N_GE_24 { 72 } else { 0 }
    } else if IS_HARAKA {
        10 * 8 * 8 + 10 * 8 * 4
    } else {
        0
    };

/// The backend's underlying hash block size, used to pick message lengths that
/// straddle the `mlen` branches in `hash_sha2.c`.
pub const SHAX_BLOCK_BYTES: usize = if N_GE_24 { 128 } else { 64 };
pub const SHAX_OUTPUT_BYTES: usize = if N_GE_24 { 64 } else { 32 };
/// `SPX_INBLOCKS` from `hash_sha2.c`.
pub const SHA2_INBLOCKS: usize =
    (SPX_N + SPX_PK_BYTES + SHAX_BLOCK_BYTES - 1) / SHAX_BLOCK_BYTES;

pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, so every failure is reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x5EED_1234_9E37_79B9)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
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
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        while v.len() < n {
            v.extend_from_slice(&self.next_u64().to_le_bytes());
        }
        v.truncate(n);
        v
    }
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for x in a.iter_mut() {
            *x = self.next_u32();
        }
        a
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub struct Libs {
    /// Rust cdylib, `RTLD_LOCAL`.
    rs: UnixLibrary,
    /// C `libsphincs_core_det.so` (rng.c / NIST DRBG), `RTLD_GLOBAL`.
    c_core: UnixLibrary,
    /// C `libsphincs_core.so` (randombytes.c / `/dev/urandom`), `RTLD_GLOBAL`.
    /// Only consulted first when the `urandom` feature is on, so that C and
    /// Rust use the *same* `randombytes()` provider.
    c_core_nd: UnixLibrary,
    /// C `lib<backend>.so`, `RTLD_GLOBAL`.
    c_be: UnixLibrary,
    /// `libcrypto.so.3`, kept alive: CMake links OpenSSL only into the `driver`
    /// executable, so `libsphincs_core_det.so` has undefined `EVP_*` symbols
    /// that must come from the global scope.
    _crypto: Option<UnixLibrary>,
}

impl Libs {
    pub fn load() -> Libs {
        let root = workspace_root();

        let c_dir = std::env::var("SPHINCS_C_DIR").map(PathBuf::from).unwrap_or_else(|_| {
            root.join("c_build")
                .join(format!("{}_{}_{}", BACKEND, THASH, SECPAR))
        });
        let be_path = c_dir.join("lib").join(BACKEND).join(format!("lib{}.so", BACKEND));
        let core_path = c_dir.join("app").join("libsphincs_core_det.so");
        let core_nd_path = c_dir.join("app").join("libsphincs_core.so");
        let rs_path = std::env::var("SPHINCS_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|_| root.join("translation/target/release/libsphincsplus.so"));

        assert!(be_path.exists(), "missing C backend .so: {:?}", be_path);
        assert!(core_path.exists(), "missing C core .so: {:?}", core_path);
        assert!(core_nd_path.exists(), "missing C core .so: {:?}", core_nd_path);
        assert!(rs_path.exists(), "missing Rust .so: {:?}", rs_path);

        unsafe {
            // Rust first, RTLD_LOCAL, so nothing of the C libraries can be
            // interposed into it (and vice versa).
            let rs = UnixLibrary::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", rs_path));
            // OpenSSL must be in the global scope before the C core is used.
            let crypto = ["libcrypto.so.3", "libcrypto.so"]
                .iter()
                .find_map(|n| UnixLibrary::open(Some(n), RTLD_LAZY | RTLD_GLOBAL).ok());
            assert!(crypto.is_some(), "could not dlopen libcrypto");
            // `lib<backend>.so` and `libsphincs_core_det.so` reference each
            // other (backend needs `SPX_set_tree_index`/`SPX_thash` callers,
            // core needs `SPX_thash`/`SPX_prf_addr`), exactly as in CMake's
            // `target_link_libraries(driver sphincs_core_det <backend> crypto)`.
            // Lazy + global binding reproduces that mutual resolution.
            let c_be = UnixLibrary::open(Some(&be_path), RTLD_LAZY | RTLD_GLOBAL)
                .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", be_path));
            // With `urandom`, load the `/dev/urandom` core FIRST so that its
            // `randombytes` wins global binding and the C `crypto_sign_*` use
            // the same provider the Rust build does.
            let (c_core, c_core_nd) = if RAND_URANDOM {
                let nd = UnixLibrary::open(Some(&core_nd_path), RTLD_LAZY | RTLD_GLOBAL)
                    .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", core_nd_path));
                let det = UnixLibrary::open(Some(&core_path), RTLD_LAZY | RTLD_GLOBAL)
                    .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", core_path));
                (det, nd)
            } else {
                let det = UnixLibrary::open(Some(&core_path), RTLD_LAZY | RTLD_GLOBAL)
                    .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", core_path));
                let nd = UnixLibrary::open(Some(&core_nd_path), RTLD_LAZY | RTLD_GLOBAL)
                    .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", core_nd_path));
                (det, nd)
            };
            Libs { rs, c_core, c_core_nd, c_be, _crypto: crypto }
        }
    }

    /// dlsym in the C libraries.
    ///
    /// With `urandom` the `/dev/urandom` core is searched first, so that
    /// `randombytes` (and everything that calls it) matches the Rust build;
    /// symbols only `rng.c` defines (`randombytes_init`, `seedexpander*`,
    /// `AES256_*`, `DRBG_ctx`) then fall through to `libsphincs_core_det.so`.
    pub fn c<T>(&self, name: &str) -> UnixSymbol<T> {
        let cname = std::ffi::CString::new(name).unwrap();
        let order: [&UnixLibrary; 3] = if RAND_URANDOM {
            [&self.c_core_nd, &self.c_core, &self.c_be]
        } else {
            [&self.c_core, &self.c_core_nd, &self.c_be]
        };
        unsafe {
            for lib in order {
                if let Ok(s) = lib.get::<T>(cname.as_bytes_with_nul()) {
                    return s;
                }
            }
            panic!("C symbol {name} not found");
        }
    }

    /// dlsym in the C backend library only (used where both define a symbol and
    /// we specifically want the backend's copy).
    pub fn c_backend<T>(&self, name: &str) -> UnixSymbol<T> {
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            self.c_be
                .get::<T>(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("C backend symbol {name} not found: {e}"))
        }
    }

    pub fn r<T>(&self, name: &str) -> UnixSymbol<T> {
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            self.rs
                .get::<T>(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("Rust symbol {name} not found: {e}"))
        }
    }
}

/// Fetch the same symbol from both sides with the same function type.
#[macro_export]
macro_rules! pair {
    ($libs:expr, $name:literal, $t:ty) => {{
        let c: libloading::os::unix::Symbol<$t> = $libs.c($name);
        let r: libloading::os::unix::Symbol<$t> = $libs.r($name);
        (c, r)
    }};
}

/// Same, but forcing the C side to come from `lib<backend>.so`.
#[macro_export]
macro_rules! pair_backend {
    ($libs:expr, $name:literal, $t:ty) => {{
        let c: libloading::os::unix::Symbol<$t> = $libs.c_backend($name);
        let r: libloading::os::unix::Symbol<$t> = $libs.r($name);
        (c, r)
    }};
}

// ---------------------------------------------------------------------------
// Comparison helper
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let first = c.iter().zip(r).position(|(a, b)| a != b).unwrap_or(c.len().min(r.len()));
        panic!(
            "{what}: C/Rust differ ({BACKEND}/{THASH}/{SECPAR})\n  \
             len C={} R={}, first difference at byte {first}\n  \
             C = {:02x?}\n  R = {:02x?}",
            c.len(),
            r.len(),
            &c[first.saturating_sub(4)..(first + 12).min(c.len())],
            &r[first.saturating_sub(4)..(first + 12).min(r.len())],
        );
    }
}

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(c, r, "{what}: C/Rust differ ({BACKEND}/{THASH}/{SECPAR})");
}

/// An 8-byte-aligned `spx_ctx` scratch buffer.
pub struct Ctx {
    buf: Vec<u64>,
}

impl Ctx {
    pub fn new() -> Ctx {
        Ctx { buf: vec![0u64; (CTX_BYTES + 7) / 8] }
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.buf.as_ptr() as *const u8
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr() as *mut u8
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), CTX_BYTES) }
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr(), CTX_BYTES) }
    }
    pub fn set_pub_seed(&mut self, s: &[u8]) {
        self.bytes_mut()[..SPX_N].copy_from_slice(&s[..SPX_N]);
    }
    pub fn set_sk_seed(&mut self, s: &[u8]) {
        self.bytes_mut()[SPX_N..2 * SPX_N].copy_from_slice(&s[..SPX_N]);
    }
}

/// Build a pair of initialized contexts (one per implementation) from the same
/// seeds, calling each side's own `SPX_initialize_hash_function`.
pub fn init_ctx_pair(libs: &Libs, pub_seed: &[u8], sk_seed: &[u8]) -> (Ctx, Ctx) {
    type InitFn = unsafe extern "C" fn(*mut u8);
    let (c_init, r_init) = pair_backend!(libs, "SPX_initialize_hash_function", InitFn);
    let mut cc = Ctx::new();
    let mut rc = Ctx::new();
    for ctx in [&mut cc, &mut rc] {
        ctx.set_pub_seed(pub_seed);
        ctx.set_sk_seed(sk_seed);
    }
    unsafe {
        c_init(cc.as_mut_ptr());
        r_init(rc.as_mut_ptr());
    }
    eq_bytes("initialize_hash_function(ctx)", cc.bytes(), rc.bytes());
    (cc, rc)
}

/// `leaf_info_x1` from `app/include/wotsx1.h`, x86-64 layout.
#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    _pad: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl LeafInfoX1 {
    pub fn zeroed() -> LeafInfoX1 {
        LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: 0,
            _pad: 0,
            wots_steps: std::ptr::null_mut(),
            leaf_addr: [0; 8],
            pk_addr: [0; 8],
        }
    }
}

/// `AES_XOF_struct` from `app/include/rng.h`, x86-64 layout (80 bytes).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

impl AesXofStruct {
    pub fn zeroed() -> AesXofStruct {
        AesXofStruct {
            buffer: [0; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0; 32],
            ctr: [0; 16],
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of::<Self>())
        }
    }
}

/// `AES256_CTR_DRBG_struct` from `app/include/rng.h` (56 bytes with padding).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

pub fn u32s_to_bytes(a: &[u32]) -> Vec<u8> {
    let mut v = Vec::with_capacity(a.len() * 4);
    for x in a {
        v.extend_from_slice(&x.to_ne_bytes());
    }
    v
}
