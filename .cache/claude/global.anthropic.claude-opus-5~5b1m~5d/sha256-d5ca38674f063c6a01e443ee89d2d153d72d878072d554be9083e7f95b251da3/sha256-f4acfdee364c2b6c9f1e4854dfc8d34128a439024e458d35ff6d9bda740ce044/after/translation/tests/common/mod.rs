//! Shared differential-test harness.
//!
//! Both the C reference `.so`s and the Rust cdylib are loaded with `libloading`
//! and driven purely through their exported C symbols -- no Rust function is
//! ever called directly, so the `#[no_mangle]` wrappers are under test too.
//!
//! Environment variables (set by `../run_tests.sh`):
//!   * `SPX_C_BUILD`  -- directory of the matching C cmake build tree
//!                       (default: `../cbuild/<backend>-<secpar>-<thash>`)
//!   * `SPX_RUST_SO`  -- path of the Rust cdylib built with the same features
//!                       (default: `target/release/libsphincs_core_det.so`)

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_LAZY, RTLD_LOCAL};
use std::ffi::CString;

// ===========================================================================
// Parameters -- an INDEPENDENT re-derivation from the C headers, so a wrong
// constant in src/params.rs cannot hide itself.
// ===========================================================================

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

// app/params/params-sphincs-<backend>-<secpar>.h
#[cfg(feature = "256f")]
pub const SECPAR: &str = "256f";
#[cfg(all(feature = "256s", not(feature = "256f")))]
pub const SECPAR: &str = "256s";
#[cfg(all(feature = "192f", not(any(feature = "256f", feature = "256s"))))]
pub const SECPAR: &str = "192f";
#[cfg(all(
    feature = "192s",
    not(any(feature = "256f", feature = "256s", feature = "192f"))
))]
pub const SECPAR: &str = "192s";
#[cfg(all(
    feature = "128f",
    not(any(
        feature = "256f",
        feature = "256s",
        feature = "192f",
        feature = "192s"
    ))
))]
pub const SECPAR: &str = "128f";
#[cfg(all(
    not(feature = "128f"),
    not(any(
        feature = "256f",
        feature = "256s",
        feature = "192f",
        feature = "192s"
    ))
))]
pub const SECPAR: &str = "128s";

pub const SPX_N: usize = match SECPAR.as_bytes() {
    b"128s" | b"128f" => 16,
    b"192s" | b"192f" => 24,
    _ => 32,
};

pub const SPX_FULL_HEIGHT: usize = match SECPAR.as_bytes() {
    b"128s" => 63,
    b"128f" => 66,
    b"192s" => 63,
    b"192f" => 66,
    b"256s" => 64,
    _ => 68,
};

pub const SPX_D: usize = match SECPAR.as_bytes() {
    b"128s" => 7,
    b"128f" => 22,
    b"192s" => 7,
    b"192f" => 22,
    b"256s" => 8,
    _ => 17,
};

pub const SPX_FORS_HEIGHT: usize = match SECPAR.as_bytes() {
    b"128s" => 12,
    b"128f" => 6,
    b"192s" => 14,
    b"192f" => 8,
    b"256s" => 14,
    _ => 9,
};

pub const SPX_FORS_TREES: usize = match SECPAR.as_bytes() {
    b"128s" => 14,
    b"128f" => 33,
    b"192s" => 17,
    b"192f" => 33,
    b"256s" => 22,
    _ => 35,
};

/// `SPX_SHA512` / `SPX_BLAKE512`: 1 for the 192/256-bit parameter sets.
pub const BIG_HASH: bool = SPX_N >= 24;

pub const SPX_WOTS_W: usize = 16;
pub const SPX_WOTS_LOGW: usize = 4;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: usize = 3; // SPX_N is 16/24/32 -> 8 < N <= 136 -> 3
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

// Address field offsets: sha2 uses its own layout (sha2_offsets.h).
pub const SPX_OFFSET_LAYER: usize = if cfg!(feature = "sha2") { 0 } else { 3 };
pub const SPX_OFFSET_TREE: usize = if cfg!(feature = "sha2") { 1 } else { 8 };
pub const SPX_OFFSET_TYPE: usize = if cfg!(feature = "sha2") { 9 } else { 19 };
pub const SPX_OFFSET_KP_ADDR: usize = if cfg!(feature = "sha2") { 10 } else { 20 };
pub const SPX_OFFSET_CHAIN_ADDR: usize = if cfg!(feature = "sha2") { 17 } else { 27 };
pub const SPX_OFFSET_HASH_ADDR: usize = if cfg!(feature = "sha2") { 21 } else { 31 };
pub const SPX_OFFSET_TREE_HGT: usize = if cfg!(feature = "sha2") { 17 } else { 27 };
pub const SPX_OFFSET_TREE_INDEX: usize = if cfg!(feature = "sha2") { 18 } else { 28 };

// ---------------------------------------------------------------------------
// spx_ctx layout (context.h).  Every test allocates CTX_SIZE bytes; the Rust
// struct for sha2 always carries `state_seeded_512` while the C one omits it
// when SPX_SHA512 == 0, so the buffer is sized for the larger of the two.
// ---------------------------------------------------------------------------

pub const CTX_OFF_PUB_SEED: usize = 0;
pub const CTX_OFF_SK_SEED: usize = SPX_N;
/// sha2 only
pub const CTX_OFF_STATE_SEEDED: usize = 2 * SPX_N;
/// sha2 only
pub const CTX_OFF_STATE_SEEDED_512: usize = 2 * SPX_N + 40;
/// haraka only (`uint64_t[10][8]`, 8-byte aligned; 2*SPX_N is a multiple of 8)
pub const CTX_OFF_TWEAKED512: usize = 2 * SPX_N;
/// haraka only (`uint32_t[10][8]`)
pub const CTX_OFF_TWEAKED256: usize = 2 * SPX_N + 640;

/// Number of bytes of `spx_ctx` that BOTH implementations actually use, i.e.
/// the part that can be compared after `initialize_hash_function`.
pub const CTX_LIVE: usize = if cfg!(feature = "sha2") {
    if BIG_HASH {
        2 * SPX_N + 40 + 72
    } else {
        2 * SPX_N + 40
    }
} else if cfg!(any(feature = "shake", feature = "blake")) {
    2 * SPX_N
} else {
    2 * SPX_N + 640 + 320
};

/// Allocation size (generous, identical for both sides).
pub const CTX_SIZE: usize = 2 * SPX_N + 1024 + 64;

// ===========================================================================
// Reproducible PRNG: xorshift64* with a fixed seed.
// ===========================================================================

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
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
}

/// The fixed seed used by every test in this suite.
pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

// ===========================================================================
// Loading both sides
// ===========================================================================

pub struct Side {
    libs: Vec<Library>,
    pub name: &'static str,
}

impl Side {
    /// Address of an exported symbol, searched over all handles of this side.
    pub fn addr(&self, sym: &str) -> usize {
        let c = CString::new(sym).unwrap();
        for l in &self.libs {
            unsafe {
                if let Ok(s) = l.get::<usize>(c.as_bytes_with_nul()) {
                    return *s;
                }
            }
        }
        panic!("{}: symbol `{}` not found", self.name, sym);
    }

    pub fn has(&self, sym: &str) -> bool {
        let c = CString::new(sym).unwrap();
        for l in &self.libs {
            unsafe {
                if l.get::<usize>(c.as_bytes_with_nul()).is_ok() {
                    return true;
                }
            }
        }
        false
    }

    pub fn data(&self, sym: &str) -> *mut u8 {
        self.addr(sym) as *mut u8
    }
}

pub struct Pair {
    pub c: Side,
    pub rust: Side,
}

fn repo_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let md = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    std::path::Path::new(&md).parent().unwrap().to_path_buf()
}

fn c_build_dir() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("SPX_C_BUILD") {
        return std::path::PathBuf::from(d);
    }
    repo_root()
        .join("cbuild")
        .join(format!("{}-{}-{}", BACKEND, SECPAR, THASH))
}

fn rust_so() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("SPX_RUST_SO") {
        return std::path::PathBuf::from(d);
    }
    repo_root()
        .join("translation/target/release/libsphincs_core_det.so")
}

fn open(path: &std::path::Path, flags: i32) -> Library {
    unsafe {
        Library::open(Some(path), flags)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {}", path.display(), e))
    }
}

/// Loads (1) libcrypto, (2) the C hash-backend lib, (3) the C deterministic
/// core lib -- all `RTLD_GLOBAL|RTLD_LAZY` because the two SPHINCS+ objects
/// reference each other -- and (4) the Rust cdylib, `RTLD_LOCAL` so that it
/// cannot be interposed by (or interpose on) the C symbols.
pub fn load() -> Pair {
    let cb = c_build_dir();
    let core = cb.join("app/libsphincs_core_det.so");
    let back = cb.join(format!("lib/{0}/lib{0}.so", BACKEND));
    assert!(
        core.exists(),
        "missing C build {} -- run ../build_c.sh {} {} {}",
        core.display(),
        BACKEND,
        SECPAR,
        THASH
    );

    let mut cl = Vec::new();
    // libcrypto provides the EVP_* symbols rng.c needs.
    unsafe {
        if let Ok(l) = Library::open(
            Some(std::path::Path::new("libcrypto.so.3")),
            RTLD_LAZY | RTLD_GLOBAL,
        ) {
            cl.push(l);
        }
    }
    cl.push(open(&back, RTLD_LAZY | RTLD_GLOBAL));
    cl.push(open(&core, RTLD_LAZY | RTLD_GLOBAL));
    // Search the core first (it owns crypto_sign*/rng), then the backend.
    cl.reverse();

    // RTLD_DEEPBIND (glibc, 0x8) is essential: the two C objects had to be
    // brought in RTLD_GLOBAL (they reference each other), which would otherwise
    // let them interpose on the Rust cdylib's *data* symbols -- e.g. the Rust
    // `DRBG_ctx` reference would bind to the C `DRBG_ctx`.  DEEPBIND makes the
    // Rust object prefer its own definitions, exactly as a standalone consumer
    // linking only against it would see.
    const RTLD_DEEPBIND: i32 = 0x8;
    let rl = vec![open(&rust_so(), RTLD_LAZY | RTLD_LOCAL | RTLD_DEEPBIND)];

    let pair = Pair {
        c: Side {
            libs: cl,
            name: "C",
        },
        rust: Side {
            libs: rl,
            name: "Rust",
        },
    };
    check_config(&pair);
    pair
}

/// Guards against the classic footgun of this setup: a `.so` left over from a
/// *different* feature/parameter combination.  Both sides must agree with the
/// constants this test binary was compiled with.
fn check_config(p: &Pair) {
    type FLen = unsafe extern "C" fn() -> u64;
    type FSetType = unsafe extern "C" fn(*mut u32, u32);
    for side in [&p.c, &p.rust] {
        for (sym, want) in [
            ("crypto_sign_bytes", SPX_BYTES as u64),
            ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
            ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
            ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
        ] {
            let g: FLen = unsafe { std::mem::transmute::<usize, FLen>(side.addr(sym)) };
            let got = unsafe { g() };
            assert_eq!(
                got, want,
                "{} .so was built for a DIFFERENT parameter set: {}() = {} but this test \
                 expects {} for {}/{}/{}.  Rebuild with the same cargo features / CMake \
                 cache variables (use ../run_tests.sh).",
                side.name, sym, got, want, BACKEND, THASH, SECPAR
            );
        }
        // The address-field layout differs between sha2 and the rest.
        let st: FSetType = unsafe { std::mem::transmute::<usize, FSetType>(side.addr("SPX_set_type")) };
        let mut addr = [0u32; 8];
        unsafe { st(addr.as_mut_ptr(), 0xAB) };
        let bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        assert_eq!(
            bytes[SPX_OFFSET_TYPE], 0xAB,
            "{} .so uses a different SPX_OFFSET_TYPE than {} expects ({}); \
             the .so was built for another backend.",
            side.name, BACKEND, SPX_OFFSET_TYPE
        );
    }
}

/// One shared instance for the whole test binary (dlopen is not cheap and the
/// C `DRBG_ctx` is process-global state we want to keep in one place).
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(load)
}

unsafe impl Send for Side {}
unsafe impl Sync for Side {}

// ===========================================================================
// Comparison helpers
// ===========================================================================

#[track_caller]
pub fn eq_bytes(what: &str, a: &[u8], b: &[u8]) {
    if a != b {
        let n = a.len().min(b.len());
        let mut first = n;
        for i in 0..n {
            if a[i] != b[i] {
                first = i;
                break;
            }
        }
        panic!(
            "{}/{}/{}/{}: {} differs (len C={} Rust={}, first diff at {})\n  C   = {}\n  Rust= {}",
            BACKEND,
            THASH,
            SECPAR,
            what,
            what,
            a.len(),
            b.len(),
            first,
            hex(&a[..a.len().min(first + 32)]),
            hex(&b[..b.len().min(first + 32)]),
        );
    }
}

#[track_caller]
pub fn eq<T: PartialEq + std::fmt::Debug>(what: &str, a: T, b: T) {
    assert!(
        a == b,
        "{}/{}/{}/{}: C={:?} Rust={:?}",
        BACKEND,
        THASH,
        SECPAR,
        what,
        a,
        b
    );
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

/// A fresh, zeroed `spx_ctx`-sized buffer.
pub fn new_ctx() -> Vec<u8> {
    vec![0u8; CTX_SIZE]
}

/// Fill the `pub_seed`/`sk_seed` fields of a ctx buffer.
pub fn seed_ctx(ctx: &mut [u8], rng: &mut Rng) {
    let ps = rng.bytes(SPX_N);
    let ss = rng.bytes(SPX_N);
    ctx[CTX_OFF_PUB_SEED..CTX_OFF_PUB_SEED + SPX_N].copy_from_slice(&ps);
    ctx[CTX_OFF_SK_SEED..CTX_OFF_SK_SEED + SPX_N].copy_from_slice(&ss);
}
