//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls everything across
//! the FFI boundary, exactly as an external C consumer would:
//!
//!   * C   : `<repo>/cbuild/<backend>-<thash>-<secpar>/libcref.so`
//!           (the untouched CMake object files for `sphincs_obj` + `rng.c`,
//!            with `lib<backend>.so` pulled in via `DT_NEEDED`)
//!   * Rust: `<repo>/translation/target/release/libsphincs_core_det.so`
//!
//! No Rust library function is ever called directly -- every value compared in
//! these tests came out of a `dlsym`'d symbol.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Build configuration, derived from the *test crate's* features.
// The params below are transcribed independently from
// c_src/app/params/params-sphincs-<backend>-<secpar>.h so that a bug in the
// Rust crate's own params.rs cannot hide (see cfg_sizes_match, CONFIGS row 1).
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

pub const THASH: &str = if cfg!(feature = "robust") {
    "robust"
} else {
    "simple"
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

/// (SPX_N, SPX_FULL_HEIGHT, SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES, big_hash)
const fn secpar_tuple() -> (usize, usize, usize, usize, usize, u32) {
    if cfg!(feature = "256f") {
        (32, 68, 17, 9, 35, 1)
    } else if cfg!(feature = "256s") {
        (32, 64, 8, 14, 22, 1)
    } else if cfg!(feature = "192f") {
        (24, 66, 22, 8, 33, 1)
    } else if cfg!(feature = "192s") {
        (24, 63, 7, 14, 17, 1)
    } else if cfg!(feature = "128f") {
        (16, 66, 22, 6, 33, 0)
    } else {
        (16, 63, 7, 12, 14, 0)
    }
}

pub const SPX_N: usize = secpar_tuple().0;
pub const SPX_FULL_HEIGHT: usize = secpar_tuple().1;
pub const SPX_D: usize = secpar_tuple().2;
pub const SPX_FORS_HEIGHT: usize = secpar_tuple().3;
pub const SPX_FORS_TREES: usize = secpar_tuple().4;
/// `SPX_SHA512` / `SPX_BLAKE512` -- 1 for the 192/256 sets, 0 for 128.
pub const BIG_HASH: u32 = secpar_tuple().5;

pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
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
pub const SPX_SK_BYTES: usize = 4 * SPX_N;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
pub const SPX_ADDR_BYTES: usize = 32;

// Address field offsets: sha2 uses the compressed 22-byte layout, the other
// three backends the 32-byte layout (lib/<backend>/include/<backend>_offsets.h).
pub const SPX_OFFSET_LAYER: usize = if cfg!(feature = "sha2") { 0 } else { 3 };
pub const SPX_OFFSET_TREE: usize = if cfg!(feature = "sha2") { 1 } else { 8 };
pub const SPX_OFFSET_TYPE: usize = if cfg!(feature = "sha2") { 9 } else { 19 };
pub const SPX_OFFSET_KP_ADDR: usize = if cfg!(feature = "sha2") { 10 } else { 20 };
pub const SPX_OFFSET_CHAIN_ADDR: usize = if cfg!(feature = "sha2") { 17 } else { 27 };
pub const SPX_OFFSET_HASH_ADDR: usize = if cfg!(feature = "sha2") { 21 } else { 31 };
pub const SPX_OFFSET_TREE_HGT: usize = if cfg!(feature = "sha2") { 17 } else { 27 };
pub const SPX_OFFSET_TREE_INDEX: usize = if cfg!(feature = "sha2") { 18 } else { 28 };

/// `sizeof(spx_ctx)` for the active configuration, from `app/include/context.h`.
pub const CTX_BYTES: usize = if cfg!(feature = "sha2") {
    // pub_seed + sk_seed + state_seeded[40] (+ state_seeded_512[72] iff SPX_SHA512)
    2 * SPX_N + 40 + if BIG_HASH == 1 { 72 } else { 0 }
} else if cfg!(feature = "haraka") {
    // pub_seed + sk_seed (8-aligned) + u64[10][8] + u32[10][8]
    2 * SPX_N + 640 + 320
} else {
    2 * SPX_N
};

/// Address types passed to `set_type` (`app/include/address.h`).
pub const ADDR_TYPES: [u32; 7] = [0, 1, 2, 3, 4, 5, 6];

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    repo_root()
        .join("cbuild")
        .join(format!("{}-{}-{}", BACKEND, THASH, SECPAR))
        .join("libcref.so")
}

pub fn rust_lib_path() -> PathBuf {
    // `SPHINCS_RUST_SO` lets a runner point at a .so in a dedicated
    // CARGO_TARGET_DIR, so parallel runs for different feature sets cannot
    // clobber each other's artifact.
    if let Ok(p) = std::env::var("SPHINCS_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"));
    base.join("release").join("libsphincs_core_det.so")
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_lib_path();
        let r_path = rust_lib_path();
        assert!(
            c_path.exists(),
            "missing C reference .so at {c_path:?}\n\
             build it with: ./build_c.sh {BACKEND} {THASH} {SECPAR} && \
             ./link_cref.sh {BACKEND} {THASH} {SECPAR}"
        );
        assert!(
            r_path.exists(),
            "missing Rust .so at {r_path:?}\n\
             build it with: cargo build --release --no-default-features \
             --features {BACKEND},{THASH},{SECPAR}"
        );
        // SAFETY: both objects are plain C-ABI libraries with no constructors
        // that run arbitrary code. Loaded with the default RTLD_LOCAL so their
        // identically-named symbols do not collide.
        let c = unsafe { Library::new(&c_path) }.expect("dlopen C .so");
        let r = unsafe { Library::new(&r_path) }.expect("dlopen Rust .so");

        // Guard against a STALE target/release/libsphincs_core_det.so left over
        // from a build with different features -- otherwise every test would
        // compare apples to oranges and report bogus divergences.
        for (which, lib) in [("C", &c), ("Rust", &r)] {
            let f: Symbol<unsafe extern "C" fn() -> u64> = unsafe { lib.get(b"crypto_sign_bytes") }
                .expect("crypto_sign_bytes");
            let got = unsafe { f() };
            assert_eq!(
                got, SPX_BYTES as u64,
                "{which} .so was built for a DIFFERENT configuration than this test \
                 ({BACKEND}/{THASH}/{SECPAR}, SPX_BYTES={SPX_BYTES}): its \
                 crypto_sign_bytes() returned {got}. Rebuild it:\n  \
                 cargo build --release --no-default-features --features \
                 {BACKENDplaceholder}",
                BACKENDplaceholder = format!("{BACKEND},{THASH},{SECPAR}")
            );
        }

        Libs {
            c,
            r,
            c_path,
            r_path,
        }
    })
}

/// Look up `name` in both libraries and hand the two symbols to `f`.
///
/// Panics with a clear message if either library is missing the symbol -- which
/// is itself a Phase-D parity failure.
#[macro_export]
macro_rules! both {
    ($name:expr, $ty:ty) => {{
        let l = $crate::common::libs();
        let cs: libloading::Symbol<$ty> = unsafe { l.c.get($name.as_bytes()) }
            .unwrap_or_else(|e| panic!("C .so is missing `{}`: {e}", $name));
        let rs: libloading::Symbol<$ty> = unsafe { l.r.get($name.as_bytes()) }
            .unwrap_or_else(|e| panic!("Rust .so is missing `{}`: {e}", $name));
        (cs, rs)
    }};
}

/// Fetch a *data* symbol (e.g. `DRBG_ctx`, `cst`) from both libraries.
#[macro_export]
macro_rules! both_data {
    ($name:expr, $ty:ty) => {{
        let l = $crate::common::libs();
        let cs: libloading::Symbol<*mut $ty> = unsafe { l.c.get($name.as_bytes()) }
            .unwrap_or_else(|e| panic!("C .so is missing data `{}`: {e}", $name));
        let rs: libloading::Symbol<*mut $ty> = unsafe { l.r.get($name.as_bytes()) }
            .unwrap_or_else(|e| panic!("Rust .so is missing data `{}`: {e}", $name));
        (*cs, *rs)
    }};
}

pub fn sym<'a, T>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
    unsafe { lib.get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const RNG_SEED: u64 = 0x00C0_FFEE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for w in a.iter_mut() {
            *w = self.next_u32();
        }
        a
    }
}

/// Number of randomized iterations for a cheap row.
pub const N_ITER: usize = 8;
/// Number of randomized iterations for an expensive (whole-signature) row.
pub const N_ITER_SLOW: usize = 2;

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let n = c.len().min(r.len());
        let mut first = n;
        for i in 0..n {
            if c[i] != r[i] {
                first = i;
                break;
            }
        }
        panic!(
            "{what}: C/Rust differ (cfg {BACKEND}/{THASH}/{SECPAR})\n\
             lens: C={} R={}\n\
             first differing byte at {first}\n\
             C[{first}..]: {}\n\
             R[{first}..]: {}",
            c.len(),
            r.len(),
            hex(&c[first..(first + 32).min(c.len())]),
            hex(&r[first..(first + 32).min(r.len())]),
        );
    }
}

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(
        c, r,
        "{what}: C/Rust differ (cfg {BACKEND}/{THASH}/{SECPAR})"
    );
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}

/// A freshly zeroed `spx_ctx` byte image of the right size for this config.
pub fn new_ctx_buf() -> Vec<u8> {
    vec![0u8; CTX_BYTES]
}

/// Build an `spx_ctx` image with the given seeds, then run
/// `SPX_initialize_hash_function` in the given library so that the sha2
/// midstate / haraka tweaked constants are populated.
pub fn init_ctx(lib: &Library, pub_seed: &[u8], sk_seed: &[u8]) -> Vec<u8> {
    assert_eq!(pub_seed.len(), SPX_N);
    assert_eq!(sk_seed.len(), SPX_N);
    let mut buf = new_ctx_buf();
    buf[..SPX_N].copy_from_slice(pub_seed);
    buf[SPX_N..2 * SPX_N].copy_from_slice(sk_seed);
    let f: Symbol<unsafe extern "C" fn(*mut u8)> = sym(lib, "SPX_initialize_hash_function");
    unsafe { f(buf.as_mut_ptr()) };
    buf
}

/// Same seeds -> one initialised ctx image per library.
pub fn init_ctx_pair(pub_seed: &[u8], sk_seed: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let l = libs();
    (
        init_ctx(&l.c, pub_seed, sk_seed),
        init_ctx(&l.r, pub_seed, sk_seed),
    )
}

/// `DRBG_ctx` is a *process-global* mutable in each `.so` (exactly as in C), so
/// any test that seeds it or consumes from it -- directly via `randombytes`, or
/// indirectly via `crypto_sign_keypair` / `crypto_sign_signature` /
/// `crypto_sign` -- must hold this lock for the whole sequence. Without it the
/// libtest thread pool interleaves two tests' DRBG draws and both see garbage.
static DRBG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[must_use = "hold the guard for the whole DRBG-dependent sequence"]
pub fn drbg_guard() -> std::sync::MutexGuard<'static, ()> {
    DRBG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Seed both DRBGs identically (the NIST KAT entropy input 0x00..0x2F).
pub fn seed_both_drbgs(entropy: &[u8; 48]) {
    let (c, r) = both!(
        "randombytes_init",
        unsafe extern "C" fn(*mut u8, *mut u8)
    );
    let mut e1 = *entropy;
    let mut e2 = *entropy;
    unsafe {
        c(e1.as_mut_ptr(), std::ptr::null_mut());
        r(e2.as_mut_ptr(), std::ptr::null_mut());
    }
}

pub fn kat_entropy() -> [u8; 48] {
    let mut e = [0u8; 48];
    for (i, b) in e.iter_mut().enumerate() {
        *b = i as u8;
    }
    e
}

/// Message lengths that straddle every hash block/rate boundary the four
/// backends use (sha256/blake256: 64, sha512/blake512: 128, shake256 rate: 136,
/// haraka sponge rate: 32) plus the degenerate and multi-block cases.
pub const MLENS: &[usize] = &[
    0, 1, 31, 32, 33, 55, 56, 63, 64, 65, 71, 72, 111, 112, 127, 128, 129, 135, 136, 137, 167, 168,
    169, 1000,
];
