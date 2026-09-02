//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! driven exclusively through their exported C symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.
//!
//! Load order matters: the C `libsphincs_core*.so` has *undefined* references to
//! the backend (`SPX_thash`, `SPX_prf_addr`, …) and, for `rng.c`, to OpenSSL's
//! `EVP_*`.  So `libcrypto` and `lib<backend>.so` are opened `RTLD_GLOBAL`
//! first, then the C core, and only then the Rust `cdylib` — which is opened
//! `RTLD_LOCAL` so that it can never satisfy (or be satisfied by) a C symbol.

#![allow(dead_code)]

use libloading::os::unix::{Library as UnixLibrary, Symbol as UnixSymbol, RTLD_GLOBAL, RTLD_LAZY, RTLD_LOCAL};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Build configuration, resolved exactly the way translation/build.rs resolves it
// ---------------------------------------------------------------------------

pub const BACKEND: &str = if cfg!(feature = "blake") {
    "blake"
} else if cfg!(feature = "shake") {
    "shake"
} else if cfg!(feature = "sha2") {
    "sha2"
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

pub const URANDOM: bool = cfg!(feature = "urandom");

pub const IS_BLAKE: bool = cfg!(feature = "blake");
pub const IS_SHAKE: bool = !IS_BLAKE && cfg!(feature = "shake");
pub const IS_SHA2: bool = !IS_BLAKE && !cfg!(feature = "shake") && cfg!(feature = "sha2");
pub const IS_HARAKA: bool =
    !IS_BLAKE && !cfg!(feature = "shake") && !cfg!(feature = "sha2");

// ---------------------------------------------------------------------------
// Parameters, re-derived from app/params/params-sphincs-<backend>-<SECPAR>.h
// ---------------------------------------------------------------------------

const fn secpar_params() -> (usize, usize, usize, usize, usize) {
    // (SPX_N, SPX_FULL_HEIGHT, SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES)
    if cfg!(feature = "256f") {
        (32, 68, 17, 9, 35)
    } else if cfg!(feature = "256s") {
        (32, 64, 8, 14, 22)
    } else if cfg!(feature = "192f") {
        (24, 66, 22, 8, 33)
    } else if cfg!(feature = "192s") {
        (24, 63, 7, 14, 17)
    } else if cfg!(feature = "128f") {
        (16, 66, 22, 6, 33)
    } else {
        (16, 63, 7, 12, 14)
    }
}

pub const SPX_N: usize = secpar_params().0;
pub const SPX_FULL_HEIGHT: usize = secpar_params().1;
pub const SPX_D: usize = secpar_params().2;
pub const SPX_FORS_HEIGHT: usize = secpar_params().3;
pub const SPX_FORS_TREES: usize = secpar_params().4;

pub const SPX_ADDR_BYTES: usize = 32;
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
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 4 * SPX_N;
pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

/// `SPX_BLAKE512` / `SPX_SHA512`
pub const WIDE: bool = SPX_N >= 24;

// `<backend>_offsets.h`
pub const OFF_LAYER: usize = if IS_SHA2 { 0 } else { 3 };
pub const OFF_TREE: usize = if IS_SHA2 { 1 } else { 8 };
pub const OFF_TYPE: usize = if IS_SHA2 { 9 } else { 19 };
pub const OFF_KP_ADDR: usize = if IS_SHA2 { 10 } else { 20 };
pub const OFF_CHAIN_ADDR: usize = if IS_SHA2 { 17 } else { 27 };
pub const OFF_HASH_ADDR: usize = if IS_SHA2 { 21 } else { 31 };
pub const OFF_TREE_HGT: usize = if IS_SHA2 { 17 } else { 27 };
pub const OFF_TREE_INDEX: usize = if IS_SHA2 { 18 } else { 28 };

/// Size of the `spx_ctx` struct as laid out by `app/include/context.h`.
pub const CTX_BYTES: usize = if IS_SHA2 {
    2 * SPX_N + 40 + if WIDE { 72 } else { 0 }
} else if IS_HARAKA {
    // uint64_t member forces 8-byte alignment/padding after the two seeds.
    let head = 2 * SPX_N;
    let pad = (8 - head % 8) % 8;
    head + pad + 10 * 8 * 8 + 10 * 8 * 4
} else {
    2 * SPX_N
};

/// Blake writes its full digest into `R` from `gen_message_random`.
pub const GEN_MSG_RANDOM_OUT: usize = if IS_BLAKE {
    if WIDE {
        64
    } else {
        32
    }
} else {
    SPX_N
};

// ---------------------------------------------------------------------------
// spx_ctx as an opaque, correctly aligned byte blob
// ---------------------------------------------------------------------------

/// `spx_ctx` seen as raw bytes.  Over-aligned to 8 so that the haraka layout is
/// valid for both implementations.
#[repr(C, align(8))]
#[derive(Clone)]
pub struct Ctx(pub [u8; CTX_BYTES]);

impl Ctx {
    pub fn zeroed() -> Self {
        Ctx([0u8; CTX_BYTES])
    }
    /// Fills `pub_seed` and `sk_seed` (the first `2*SPX_N` bytes) and leaves the
    /// rest zero, matching a freshly declared `spx_ctx` whose seeds were
    /// `memcpy`d in by `sign.c`.
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
#[derive(Clone)]
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
}

/// `fors_gen_leaf_info` from `app/include/fors.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

/// `AES_XOF_struct` from `app/include/rng.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AesXof {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

impl AesXof {
    pub fn zeroed() -> Self {
        AesXof {
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
pub struct DrbgCtx {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

/// `blakestate256`
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

/// `blakestate512`
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
// Deterministic PRNG (xoshiro256**), so every row is reproducible
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64 expansion
        let mut x = seed;
        let mut s = [0u64; 4];
        for slot in s.iter_mut() {
            x = x.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            *slot = z ^ (z >> 31);
        }
        Rng { s }
    }
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            let n = chunk.len();
            chunk.copy_from_slice(&v[..n]);
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
}

/// The fixed seed used by every row, so failures reproduce exactly.
pub const SEED: u64 = 0x5150_5058_2b2b_0001;

/// How many randomized inputs a cheap row uses.
pub fn iters_cheap() -> usize {
    env_usize("SPX_ITERS_CHEAP", 32)
}
/// How many randomized inputs a per-WOTS-key row uses.
pub fn iters_mid() -> usize {
    env_usize("SPX_ITERS_MID", 4)
}
/// How many randomized inputs a full-signature row uses.
pub fn iters_heavy() -> usize {
    env_usize(
        "SPX_ITERS_HEAVY",
        if SECPAR.ends_with('s') { 1 } else { 2 },
    )
}

fn env_usize(name: &str, dflt: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(dflt)
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> ..
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

pub struct Libs {
    pub c: UnixLibrary,
    pub rs: UnixLibrary,
    // Kept alive for the lifetime of the process.
    _crypto: Option<UnixLibrary>,
    _backend: UnixLibrary,
    /// Under the `urandom` feature the active C core is `libsphincs_core.so`,
    /// which links `randombytes.c` and therefore does not contain `rng.c` at
    /// all.  `rng.c`'s symbols (`seedexpander*`, `AES256_*`, `DRBG_ctx`) are
    /// identical in both CMake targets, so they are looked up in
    /// `libsphincs_core_det.so` opened as an auxiliary, non-global handle.
    _rng: Option<UnixLibrary>,
}

unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

impl Libs {
    fn open() -> Libs {
        let root = workspace_root();
        let cdir = root.join("cbuild").join(format!(
            "{}_{}_{}",
            BACKEND, THASH, SECPAR
        ));
        assert!(
            cdir.is_dir(),
            "C reference build missing: {} — run ./build_c_all.sh",
            cdir.display()
        );

        // OpenSSL must be resolvable before libsphincs_core_det.so is opened.
        let crypto = unsafe {
            UnixLibrary::open(Some("libcrypto.so.3"), RTLD_LAZY | RTLD_GLOBAL)
                .or_else(|_| UnixLibrary::open(Some("libcrypto.so"), RTLD_LAZY | RTLD_GLOBAL))
                .ok()
        };

        let backend = unsafe {
            UnixLibrary::open(
                Some(cdir.join(format!("lib{}.so", BACKEND))),
                RTLD_LAZY | RTLD_GLOBAL,
            )
        }
        .expect("failed to dlopen the C backend library");

        let core_name = if URANDOM {
            "libsphincs_core.so"
        } else {
            "libsphincs_core_det.so"
        };
        let c = unsafe { UnixLibrary::open(Some(cdir.join(core_name)), RTLD_LAZY | RTLD_GLOBAL) }
            .expect("failed to dlopen the C core library");

        // `rng.c` lives only in `libsphincs_core_det.so`.  When the active core
        // is the `/dev/urandom` one, open the deterministic core as a *local*
        // handle purely as a source of the `rng.c` symbols; RTLD_LOCAL keeps its
        // duplicate `randombytes` out of the global scope.
        let rng_aux = if URANDOM {
            Some(
                unsafe {
                    UnixLibrary::open(
                        Some(cdir.join("libsphincs_core_det.so")),
                        RTLD_LAZY | RTLD_LOCAL,
                    )
                }
                .expect("failed to dlopen libsphincs_core_det.so for the rng.c symbols"),
            )
        } else {
            None
        };

        // The Rust cdylib is self-contained; open it RTLD_LOCAL so the two
        // implementations can never resolve into each other.
        let rs_path = {
            let a = root.join("translation/target/release/libsphincsplus.so");
            let b = root.join("translation/target/debug/libsphincsplus.so");
            if a.is_file() && (!b.is_file() || newer(&a, &b)) {
                a
            } else {
                b
            }
        };
        assert!(
            rs_path.is_file(),
            "Rust cdylib missing: {}",
            rs_path.display()
        );
        // The Rust cdylib is self-contained; open it RTLD_LOCAL so the two
        // implementations can never resolve into each other.  RTLD_DEEPBIND is
        // essential: the C libraries are in the global scope and define the very
        // same names (`SPX_thash`, `DRBG_ctx`, …) with default visibility, so
        // without it the Rust library's *own* GOT/PLT entries would be
        // interposed by the C definitions and the "differential" test would
        // silently compare C against C.
        const RTLD_DEEPBIND: std::ffi::c_int = 0x0000_8;
        let rs = unsafe {
            UnixLibrary::open(Some(&rs_path), RTLD_LAZY | RTLD_LOCAL | RTLD_DEEPBIND)
        }
        .expect("failed to dlopen the Rust cdylib");

        // Guard against a stale cdylib: `cargo test` does not necessarily
        // rebuild a `crate-type = ["cdylib"]` artifact, so a leftover .so from a
        // different feature combination would silently produce nonsense (and
        // heap corruption, since every buffer size would be wrong).
        for (which, lib) in [("C", &c), ("Rust", &rs)] {
            type F = unsafe extern "C" fn() -> u64;
            let f = unsafe { lib.get::<F>(b"crypto_sign_bytes\0") }
                .expect("crypto_sign_bytes must be exported");
            let got = unsafe { f() } as usize;
            assert_eq!(
                got, SPX_BYTES,
                "{which} library was built for a different configuration \
                 (crypto_sign_bytes() = {got}, expected {SPX_BYTES} for \
                 {BACKEND}/{THASH}/{SECPAR}). Run `cargo build --release \
                 --no-default-features --features \"{BACKEND},{THASH},{SECPAR}\"` \
                 and `./build_c_all.sh` first."
            );
        }

        let l = Libs {
            c,
            rs,
            _crypto: crypto,
            _backend: backend,
            _rng: rng_aux,
        };

        // Isolation self-check: the two implementations must be distinct
        // objects.  If RTLD_DEEPBIND were ineffective these addresses would
        // coincide and every "differential" assertion would be vacuous.
        {
            type F = unsafe extern "C" fn() -> u64;
            let (a, b) = l.pair::<F>("crypto_sign_bytes");
            assert_ne!(
                a.into_raw(),
                b.into_raw(),
                "C and Rust resolved crypto_sign_bytes to the same address; \
                 the two libraries are not isolated"
            );
            let (a, b) = l.pair::<F>("SPX_thash");
            assert_ne!(
                a.into_raw(),
                b.into_raw(),
                "C and Rust resolved SPX_thash to the same address"
            );
        }

        l
    }

    /// `dlsym` on both libraries, returning `(c_fn, rust_fn)`.
    /// The C core (`libsphincs_core*.so`) has no `DT_NEEDED` entry for the
    /// backend, so backend symbols (`SPX_thash`, `SPX_prf_addr`, `blake256`, …)
    /// are not reachable through the core handle; fall back to the backend
    /// handle for those.
    pub fn pair<T>(&self, name: &str) -> (UnixSymbol<T>, UnixSymbol<T>) {
        let key = cname(name);
        let cs = unsafe { self.c.get::<T>(key.as_bytes()) }
            .or_else(|_| unsafe { self._backend.get::<T>(key.as_bytes()) })
            .or_else(|e| match &self._rng {
                Some(l) => unsafe { l.get::<T>(key.as_bytes()) },
                None => Err(e),
            })
            .unwrap_or_else(|e| panic!("C lib is missing {name}: {e}"));
        let rss = unsafe { self.rs.get::<T>(key.as_bytes()) }
            .unwrap_or_else(|e| panic!("Rust lib is missing {name}: {e}"));
        (cs, rss)
    }
}

fn cname(name: &str) -> String {
    format!("{name}\0")
}

/// Address of a *data* symbol in both libraries.
pub fn data_pair<T>(name: &str) -> (*mut T, *mut T) {
    let l = libs();
    let key = cname(name);
    let c = unsafe { l.c.get::<*mut T>(key.as_bytes()) }
        .or_else(|e| match &l._rng {
            Some(x) => unsafe { x.get::<*mut T>(key.as_bytes()) },
            None => Err(e),
        })
        .unwrap_or_else(|e| panic!("C lib is missing data symbol {name}: {e}"))
        .into_raw() as *mut T;
    let r = unsafe { l.rs.get::<*mut T>(key.as_bytes()) }
        .unwrap_or_else(|e| panic!("Rust lib is missing data symbol {name}: {e}"))
        .into_raw() as *mut T;
    (c, r)
}

fn newer(a: &std::path::Path, b: &std::path::Path) -> bool {
    let ma = a.metadata().and_then(|m| m.modified()).ok();
    let mb = b.metadata().and_then(|m| m.modified()).ok();
    match (ma, mb) {
        (Some(x), Some(y)) => x >= y,
        _ => true,
    }
}

static LIBS: std::sync::OnceLock<Libs> = std::sync::OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(Libs::open)
}

/// Byte-for-byte comparison with a helpful message.
#[track_caller]
pub fn same(what: &str, c: &[u8], r: &[u8]) {
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
            "{what}: C and Rust differ ({BACKEND}/{THASH}/{SECPAR}, urandom={URANDOM})\n\
             first difference at byte {first} (lens {} vs {})\n  C = {}\n  R = {}",
            c.len(),
            r.len(),
            hex(&c[first.saturating_sub(4)..(first + 12).min(c.len())]),
            hex(&r[first.saturating_sub(4)..(first + 12).min(r.len())]),
        );
    }
}

#[track_caller]
pub fn same_val<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(
        c, r,
        "{what}: C and Rust differ ({BACKEND}/{THASH}/{SECPAR}, urandom={URANDOM})"
    );
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A random, plausible 32-byte hash address.
pub fn rand_addr(rng: &mut Rng) -> [u8; 32] {
    let mut a = [0u8; 32];
    rng.fill(&mut a);
    a
}

/// Seeds the deterministic DRBG in *both* libraries identically.  No-op under
/// the `urandom` feature, where `randombytes()` has no seedable state.
pub fn seed_both_drbgs(entropy: &[u8; 48], personalization: Option<&[u8; 48]>) {
    if URANDOM {
        return;
    }
    type Init = unsafe extern "C" fn(*mut u8, *mut u8);
    let (cf, rf) = libs().pair::<Init>("randombytes_init");
    let mut e1 = *entropy;
    let mut e2 = *entropy;
    let mut p1 = personalization.copied();
    let mut p2 = personalization.copied();
    unsafe {
        cf(
            e1.as_mut_ptr(),
            p1.as_mut().map_or(std::ptr::null_mut(), |p| p.as_mut_ptr()),
        );
        rf(
            e2.as_mut_ptr(),
            p2.as_mut().map_or(std::ptr::null_mut(), |p| p.as_mut_ptr()),
        );
    }
}
