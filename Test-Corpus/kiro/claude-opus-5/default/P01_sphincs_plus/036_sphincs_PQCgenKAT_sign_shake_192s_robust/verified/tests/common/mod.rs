//! Shared differential-test harness.
//!
//! Loads BOTH the C shared libraries and the Rust `cdylib` with `libloading`
//! and exposes them behind identical `extern "C"` function pointers, so every
//! test calls the Rust code the same way an external C consumer would (through
//! the `#[no_mangle]` export wrappers) — never via a direct Rust call.
//!
//! C side (three artifacts per configuration, see `SYMBOLS.md`):
//!   * `c_build/<cfg>/lib/<backend>/lib<backend>.so`  — hash/thash backend
//!   * `c_build/<cfg>/app/libsphincs_core_det.so`     — core + rng.c
//! They are mutually recursive (the backend needs `SPX_bytes_to_ull` from
//! `utils.c`, the core needs `SPX_thash` from the backend), so both are opened
//! `RTLD_GLOBAL | RTLD_LAZY`.  The Rust `.so` is self-contained and is opened
//! `RTLD_LOCAL` so it cannot be interposed by (or interpose on) the C symbols.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::os::unix::{Library, Symbol, RTLD_GLOBAL, RTLD_LAZY, RTLD_LOCAL, RTLD_NOW};
use std::path::PathBuf;
use std::sync::OnceLock;

pub use sphincs_plus::params::*;

/// glibc `RTLD_DEEPBIND` (not re-exported by libloading).
///
/// This flag is **essential** for this harness: the C `.so`s must be opened
/// `RTLD_GLOBAL` (they are mutually recursive — the backend needs
/// `SPX_bytes_to_ull` from `utils.c`, the core needs `SPX_thash` from the
/// backend). But then the *global* scope already contains a definition of every
/// symbol the Rust `.so` exports, and because the global scope is searched
/// *before* a dlopened object's own scope, the Rust library's internal
/// references (`DRBG_ctx`, `AES256_ECB`, `blake256`, `SPX_thash`, …) would be
/// interposed by the C definitions — silently turning "Rust vs C" into
/// "C vs C". `RTLD_DEEPBIND` makes each object prefer its own definitions.
const RTLD_DEEPBIND: i32 = 0x00008;

/* ------------------------------------------------------------------ */
/* Configuration identification                                        */
/* ------------------------------------------------------------------ */

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

pub fn cfg_name() -> String {
    format!("{BACKEND}-{THASH}-{SECPAR}")
}

/* ------------------------------------------------------------------ */
/* C type mirrors                                                      */
/* ------------------------------------------------------------------ */

/// `sizeof(spx_ctx)` for the active configuration, derived from `context.h`:
/// `pub_seed[N] ‖ sk_seed[N]` plus the SHA-2 midstates or the Haraka tweaked
/// round constants, with the struct alignment the C compiler applies.
pub const CTX_BYTES: usize = core::mem::size_of::<sphincs_plus::context::SpxCtx>();

/// Opaque byte image of a C `spx_ctx`. 8-byte aligned, which covers the
/// `uint64_t tweaked512_rc64[10][8]` member of the Haraka layout.
#[repr(C, align(8))]
#[derive(Clone)]
pub struct Ctx(pub [u8; CTX_BYTES]);

impl Ctx {
    pub fn zeroed() -> Self {
        Ctx([0u8; CTX_BYTES])
    }
    /// Fill only `pub_seed` and `sk_seed`; the rest is left zero, exactly like
    /// a C caller that memcpy's the two seeds and then calls
    /// `initialize_hash_function`.
    pub fn with_seeds(pub_seed: &[u8], sk_seed: &[u8]) -> Self {
        let mut c = Self::zeroed();
        c.0[..SPX_N].copy_from_slice(&pub_seed[..SPX_N]);
        c.0[SPX_N..2 * SPX_N].copy_from_slice(&sk_seed[..SPX_N]);
        c
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

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

impl Default for LeafInfoX1 {
    fn default() -> Self {
        LeafInfoX1 {
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: core::ptr::null_mut(),
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

/// `fors_gen_leaf_info` from `app/include/fors.h`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

/// `AES_XOF_struct` from `app/include/rng.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: core::ffi::c_ulong,
    pub length_remaining: core::ffi::c_ulong,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

impl Default for AesXofStruct {
    fn default() -> Self {
        AesXofStruct {
            buffer: [0u8; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0u8; 32],
            ctr: [0u8; 16],
        }
    }
}

impl AesXofStruct {
    pub fn bytes(&self) -> Vec<u8> {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<AesXofStruct>(),
            )
            .to_vec()
        }
    }
}

/// `AES256_CTR_DRBG_struct` from `app/include/rng.h`.
pub const DRBG_CTX_BYTES: usize = 32 + 16 + 4 /* int reseed_counter */;

/* ------------------------------------------------------------------ */
/* Function-pointer type aliases (one per exported C prototype)        */
/* ------------------------------------------------------------------ */

pub type FnU64Void = unsafe extern "C" fn() -> u64;

// address.h
pub type FnSetU32 = unsafe extern "C" fn(*mut u32, u32);
pub type FnSetU64 = unsafe extern "C" fn(*mut u32, u64);
pub type FnCopyAddr = unsafe extern "C" fn(*mut u32, *const u32);

// utils.h
pub type FnUllToBytes = unsafe extern "C" fn(*mut u8, core::ffi::c_uint, u64);
pub type FnU32ToBytes = unsafe extern "C" fn(*mut u8, u32);
pub type FnBytesToUll = unsafe extern "C" fn(*const u8, core::ffi::c_uint) -> u64;
pub type FnComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
pub type GenLeafFn = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
pub type FnTreehash =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, GenLeafFn, *mut u32);

// utilsx1.h
pub type FnTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut LeafInfoX1);

// hash.h / thash.h
pub type FnInitHash = unsafe extern "C" fn(*mut u8);
pub type FnPrfAddr = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
pub type FnGenMsgRandom = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
pub type FnHashMessage =
    unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8);
pub type FnThash = unsafe extern "C" fn(*mut u8, *const u8, core::ffi::c_uint, *const u8, *mut u32);

// wots.h / wotsx1.h
pub type FnWotsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
pub type FnChainLengths = unsafe extern "C" fn(*mut core::ffi::c_uint, *const u8);
pub type FnWotsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut LeafInfoX1);

// fors.h / forsx1.h
pub type FnForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
pub type FnForsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
pub type FnForsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo);

// merkle.h
pub type FnMerkleSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
pub type FnMerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);

// api.h
pub type FnSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
pub type FnKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
pub type FnSignature =
    unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
pub type FnVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
pub type FnSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
pub type FnSignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;

// rng.h
pub type FnRandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
pub type FnRandombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
pub type FnAes256Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
pub type FnDrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
pub type FnSeedexpanderInit =
    unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, core::ffi::c_ulong) -> i32;
pub type FnSeedexpander =
    unsafe extern "C" fn(*mut AesXofStruct, *mut u8, core::ffi::c_ulong) -> i32;

// blake.h
pub type FnBlakeOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
pub type FnBlakeInit = unsafe extern "C" fn(*mut u8);
pub type FnBlakeUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64);
pub type FnBlakeFinal = unsafe extern "C" fn(*mut u8, *mut u8);
pub type FnBlakeCompress = unsafe extern "C" fn(*mut u8, *const u8);
pub type FnMgf1 =
    unsafe extern "C" fn(*mut u8, core::ffi::c_ulong, *const u8, core::ffi::c_ulong);

// sha2.h
pub type FnShaIncInit = unsafe extern "C" fn(*mut u8);
pub type FnShaIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
pub type FnShaIncFinalize = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
pub type FnSha = unsafe extern "C" fn(*mut u8, *const u8, usize);
pub type FnSeedState = unsafe extern "C" fn(*mut u8);

// fips202.h
pub type FnShakeIncInit = unsafe extern "C" fn(*mut u64);
pub type FnShakeIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
pub type FnShakeIncFinalize = unsafe extern "C" fn(*mut u64);
pub type FnShakeIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
pub type FnShakeAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
pub type FnShakeSqueezeblocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
pub type FnShake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);

// haraka.h
pub type FnTweakConstants = unsafe extern "C" fn(*mut u8);
pub type FnHarakaSIncInit = unsafe extern "C" fn(*mut u8);
pub type FnHarakaSIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
pub type FnHarakaSIncFinalize = unsafe extern "C" fn(*mut u8);
pub type FnHarakaSIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);
pub type FnHarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
pub type FnHaraka = unsafe extern "C" fn(*mut u8, *const u8, *const u8);

/* ------------------------------------------------------------------ */
/* Library loading                                                     */
/* ------------------------------------------------------------------ */

pub struct Impl {
    /// Human-readable tag, `"C"` or `"Rust"`.
    pub tag: &'static str,
    libs: Vec<Library>,
}

impl Impl {
    /// `dlsym` across every library handle belonging to this implementation.
    pub fn raw(&self, name: &str) -> *mut std::ffi::c_void {
        for l in &self.libs {
            unsafe {
                if let Ok(s) = l.get::<*mut std::ffi::c_void>(name.as_bytes()) {
                    let p: Symbol<*mut std::ffi::c_void> = s;
                    return p.into_raw() as *mut std::ffi::c_void;
                }
            }
        }
        panic!("{}: symbol `{}` not found", self.tag, name);
    }

    /// Fetch an exported function as a typed `extern "C"` pointer.
    pub fn f<T: Copy>(&self, name: &str) -> T {
        assert_eq!(
            core::mem::size_of::<T>(),
            core::mem::size_of::<*const ()>(),
            "T must be a plain fn pointer"
        );
        let p = self.raw(name);
        unsafe { *(&p as *const _ as *const T) }
    }

    /// Fetch an exported data symbol as a byte slice.
    pub unsafe fn data(&self, name: &str, len: usize) -> &[u8] {
        core::slice::from_raw_parts(self.raw(name) as *const u8, len)
    }

    pub fn has(&self, name: &str) -> bool {
        for l in &self.libs {
            unsafe {
                if l.get::<*mut std::ffi::c_void>(name.as_bytes()).is_ok() {
                    return true;
                }
            }
        }
        false
    }
}

pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_lib_paths() -> Vec<PathBuf> {
    let base = repo_root().join("c_build").join(cfg_name());
    vec![
        base.join("lib").join(BACKEND).join(format!("lib{BACKEND}.so")),
        base.join("app").join("libsphincs_core_det.so"),
    ]
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPX_RUST_SO") {
        return PathBuf::from(p);
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let p = target.join(profile).join("libsphincs_plus.so");
    if p.exists() {
        return p;
    }
    // Fall back to whichever profile directory actually has the cdylib.
    for alt in ["debug", "release"] {
        let q = target.join(alt).join("libsphincs_plus.so");
        if q.exists() {
            return q;
        }
    }
    p
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        let mut c_libs = Vec::new();
        // `app/CMakeLists.txt` only links `crypto` into the `driver`
        // executable, so `libsphincs_core_det.so` has *undefined* `EVP_*`
        // references and no DT_NEEDED entry for libcrypto. Load libcrypto
        // RTLD_GLOBAL first so `AES256_ECB` can resolve.
        let mut libcrypto_loaded = false;
        for cand in [
            "libcrypto.so.3",
            "libcrypto.so",
            "/usr/lib64/libcrypto.so.3",
            "/usr/lib/x86_64-linux-gnu/libcrypto.so.3",
        ] {
            if let Ok(l) = unsafe { Library::open(Some(cand), RTLD_LAZY | RTLD_GLOBAL) } {
                c_libs.push(l);
                libcrypto_loaded = true;
                break;
            }
        }
        assert!(
            libcrypto_loaded,
            "could not dlopen libcrypto; the C rng.c needs OpenSSL's EVP_* for AES256_ECB"
        );
        // Backend first, then the core: both RTLD_GLOBAL|RTLD_LAZY so their
        // mutually undefined symbols resolve against each other.
        for p in c_lib_paths() {
            assert!(
                p.exists(),
                "missing C shared library {}\nrun: ./scripts/build_c.sh {} {} {}",
                p.display(),
                BACKEND,
                THASH,
                SECPAR
            );
            let l = unsafe { Library::open(Some(&p), RTLD_LAZY | RTLD_GLOBAL | RTLD_DEEPBIND) }
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()));
            c_libs.push(l);
        }
        let rp = rust_lib_path();
        assert!(
            rp.exists(),
            "missing Rust cdylib {} (run cargo build with the same features)",
            rp.display()
        );
        let rl = unsafe { Library::open(Some(&rp), RTLD_NOW | RTLD_LOCAL | RTLD_DEEPBIND) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rp.display()));

        let pair = Pair {
            c: Impl {
                tag: "C",
                libs: c_libs,
            },
            r: Impl {
                tag: "Rust",
                libs: vec![rl],
            },
        };

        // Guard against a stale cdylib built with different features: the
        // `.so` must have been compiled for the same parameter set as this
        // test binary, otherwise every comparison below is meaningless.
        let cb: FnU64Void = pair.r.f("crypto_sign_bytes");
        let got = unsafe { cb() } as usize;
        assert_eq!(
            got,
            SPX_BYTES,
            "stale Rust cdylib at {}: it reports SPX_BYTES={} but this test binary was built \
             for {} (SPX_BYTES={}).\n`cargo test` does not rebuild the cdylib — run\n  \
             cargo build --release --no-default-features --features {},{},{}\nfirst, or set \
             SPX_RUST_SO.",
            rp.display(),
            got,
            cfg_name(),
            SPX_BYTES,
            BACKEND,
            THASH,
            SECPAR
        );
        let cb: FnU64Void = pair.c.f("crypto_sign_bytes");
        let got = unsafe { cb() } as usize;
        assert_eq!(
            got, SPX_BYTES,
            "C libraries in c_build/{} report SPX_BYTES={}, expected {}",
            cfg_name(),
            got,
            SPX_BYTES
        );

        pair
    })
}

/* ------------------------------------------------------------------ */
/* Deterministic pseudo-random inputs (SplitMix64, fixed seed)         */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    /// Fixed seed derived from the row number so every row is reproducible and
    /// independent.
    pub fn for_row(row: u64) -> Self {
        Rng(0x5F_1B_C5_2A_00_00_00_00 ^ row.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
}

/// Number of randomized vectors per `CONFIGS.md` row.
pub const ITERS: usize = 32;
/// Reduced iteration count for rows that run a whole keygen/sign/verify.
pub const HEAVY_ITERS: usize = 3;

/* ------------------------------------------------------------------ */
/* Assertion helpers                                                   */
/* ------------------------------------------------------------------ */

#[track_caller]
pub fn eq_bytes(what: &str, iter: usize, c: &[u8], r: &[u8]) {
    if c != r {
        let at = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.len().min(r.len()));
        panic!(
            "[{}] {} differs at iteration {} (len C={} R={}), first mismatch at byte {}\n  C: {}\n  R: {}",
            cfg_name(),
            what,
            iter,
            c.len(),
            r.len(),
            at,
            hex(&c[at.saturating_sub(4)..(at + 12).min(c.len())]),
            hex(&r[at.saturating_sub(4)..(at + 12).min(r.len())]),
        );
    }
}

#[track_caller]
pub fn eq_u32s(what: &str, iter: usize, c: &[u32], r: &[u32]) {
    assert_eq!(
        c,
        r,
        "[{}] {} differs at iteration {}",
        cfg_name(),
        what,
        iter
    );
}

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, iter: usize, c: T, r: T) {
    assert_eq!(
        c,
        r,
        "[{}] {} differs at iteration {}",
        cfg_name(),
        what,
        iter
    );
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/* ------------------------------------------------------------------ */
/* Derived per-configuration constants used by the tests               */
/* ------------------------------------------------------------------ */

/// `SPX_SHAX_BLOCK_BYTES` / the sponge rate that `gen_message_random` and
/// `hash_message` branch on for the active backend.
pub const SHAX_BLOCK_BYTES: usize = if SPX_N >= 24 { 128 } else { 64 };

/// `SPX_INBLOCKS` from `hash_sha2.c`.
pub const SHA2_INBLOCKS: usize =
    (SPX_N + SPX_PK_BYTES + SHAX_BLOCK_BYTES - 1) / SHAX_BLOCK_BYTES;

/// `SPX_TREE_BITS`, `SPX_TREE_BYTES`, `SPX_LEAF_BYTES`, `SPX_DGST_BYTES`
/// from the backend `hash_*.c` files.
pub const SPX_TREE_BITS: u32 = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = ((SPX_TREE_BITS + 7) / 8) as usize;
pub const SPX_LEAF_BYTES: usize = ((SPX_TREE_HEIGHT + 7) / 8) as usize;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

/// Buffer big enough for `merkle_sign`'s output
/// (`SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N`).
pub const MERKLE_SIG_BYTES: usize = SPX_WOTS_BYTES + SPX_TREE_HEIGHT as usize * SPX_N;

/* ------------------------------------------------------------------ */
/* A deterministic gen_leaf callback for `SPX_treehash`                */
/* ------------------------------------------------------------------ */

/// `void (*gen_leaf)(unsigned char *leaf, const spx_ctx *ctx,
///                   uint32_t addr_idx, const uint32_t tree_addr[8])`
///
/// Deterministic and side-effect free so both implementations see the exact
/// same leaves; this isolates `treehash`'s stack/auth-path logic from the
/// backend leaf generator.
pub unsafe extern "C" fn test_gen_leaf(
    leaf: *mut u8,
    ctx: *const u8,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let mut acc = 0x243F_6A88_85A3_08D3u64 ^ (addr_idx as u64);
    // Mix in the first ctx byte and the whole tree_addr so the callback is
    // sensitive to everything treehash passes it.
    acc ^= (*ctx) as u64;
    for i in 0..8 {
        acc = acc
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .rotate_left(17)
            ^ (*tree_addr.add(i)) as u64;
    }
    for i in 0..SPX_N {
        acc = acc
            .wrapping_mul(0xBF58_476D_1CE4_E5B9)
            .wrapping_add(0x94D0_49BB_1331_11EB);
        *leaf.add(i) = (acc >> 33) as u8;
    }
}
