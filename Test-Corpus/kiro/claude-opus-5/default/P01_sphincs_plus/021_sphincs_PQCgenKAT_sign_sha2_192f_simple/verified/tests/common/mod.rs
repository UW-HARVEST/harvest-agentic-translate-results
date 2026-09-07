//! Shared harness for the C ⇄ Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading` and driven
//! only through their exported C symbols — the Rust implementation is never
//! called directly, so the `#[no_mangle]` wrappers are part of what is tested.
//!
//! Library layout for one feature combination `<be>,<th>,<sp>`:
//!
//! * `../cbuild/<be>_<th>_<sp>/lib/<be>/lib<be>.so`     — hash backend (+ `utils.c`)
//! * `../cbuild/<be>_<th>_<sp>/app/libsphincs_core_det.so` — core + `rng.c`
//! * `target/<profile>/libsphincs_plus.so`               — the Rust cdylib
//!
//! `libsphincs_core_det.so` has undefined `SPX_thash`/`SPX_prf_addr`/… which
//! the backend `.so` provides, so the backend is opened first with
//! `RTLD_GLOBAL`; `libcrypto.so.3` is opened first for the same reason
//! (`rng.c` uses OpenSSL EVP).  The Rust `.so` is opened `RTLD_LOCAL` so it
//! cannot be interposed by the C definitions of the same names.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::os::unix::{Library, Symbol, RTLD_LOCAL, RTLD_NOW};
use std::path::PathBuf;

/* ------------------------------------------------------------------ */
/* Parameters — re-derived here from the Cargo features, independently  */
/* of src/params.rs, exactly as app/params/params-sphincs-*.h do it.    */
/* ------------------------------------------------------------------ */

pub const SPX_N: usize = if cfg!(any(feature = "256s", feature = "256f")) {
    32
} else if cfg!(any(feature = "192s", feature = "192f")) {
    24
} else {
    16
};

pub const SPX_FULL_HEIGHT: u32 = if cfg!(feature = "256f") {
    68
} else if cfg!(feature = "256s") {
    64
} else if cfg!(feature = "192f") {
    66
} else if cfg!(feature = "192s") {
    63
} else if cfg!(feature = "128f") {
    66
} else {
    63
};

pub const SPX_D: u32 = if cfg!(feature = "256f") {
    17
} else if cfg!(feature = "256s") {
    8
} else if cfg!(any(feature = "192f", feature = "128f")) {
    22
} else {
    7
};

pub const SPX_FORS_HEIGHT: u32 = if cfg!(feature = "256f") {
    9
} else if cfg!(any(feature = "256s", feature = "192s")) {
    14
} else if cfg!(feature = "192f") {
    8
} else if cfg!(feature = "128f") {
    6
} else {
    12
};

pub const SPX_FORS_TREES: u32 = if cfg!(feature = "256f") {
    35
} else if cfg!(feature = "256s") {
    22
} else if cfg!(feature = "192s") {
    17
} else if cfg!(any(feature = "192f", feature = "128f")) {
    33
} else {
    14
};

pub const SPX_WOTS_W: u32 = 16;
pub const SPX_WOTS_LOGW: u32 = 4;
pub const SPX_ADDR_BYTES: usize = 32;
pub const SPX_WOTS_LEN1: usize = (8 * SPX_N) / SPX_WOTS_LOGW as usize;
pub const SPX_WOTS_LEN2: usize = 3; // SPX_N in 9..=136 for every shipped set
pub const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
pub const SPX_TREE_HEIGHT: u32 = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_FORS_MSG_BYTES: usize = ((SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8) as usize;
pub const SPX_FORS_BYTES: usize = ((SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES) as usize * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
pub const SPX_BYTES: usize = SPX_N
    + SPX_FORS_BYTES
    + SPX_D as usize * SPX_WOTS_BYTES
    + SPX_FULL_HEIGHT as usize * SPX_N;
pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

/// `SPX_SHA512` / `SPX_BLAKE512` from the parameter headers.
pub const WIDE: bool = SPX_N >= 24;

/* ADRS field offsets — sha2 uses the compressed address, others the full one. */
#[cfg(feature = "sha2")]
pub mod off {
    pub const LAYER: usize = 0;
    pub const TREE: usize = 1;
    pub const TYPE: usize = 9;
    pub const KP_ADDR: usize = 10;
    pub const CHAIN_ADDR: usize = 17;
    pub const HASH_ADDR: usize = 21;
    pub const TREE_HGT: usize = 17;
    pub const TREE_INDEX: usize = 18;
}
#[cfg(not(feature = "sha2"))]
pub mod off {
    pub const LAYER: usize = 3;
    pub const TREE: usize = 8;
    pub const TYPE: usize = 19;
    pub const KP_ADDR: usize = 20;
    pub const CHAIN_ADDR: usize = 27;
    pub const HASH_ADDR: usize = 31;
    pub const TREE_HGT: usize = 27;
    pub const TREE_INDEX: usize = 28;
}

pub const ADDR_TYPES: [u32; 7] = [0, 1, 2, 3, 4, 5, 6];

/// `sizeof(spx_ctx)` for the active configuration (see `app/include/context.h`).
pub const CTX_BYTES: usize = if cfg!(feature = "sha2") {
    if WIDE {
        2 * SPX_N + 40 + 72
    } else {
        2 * SPX_N + 40
    }
} else if cfg!(any(feature = "shake", feature = "shake256", feature = "blake")) {
    2 * SPX_N
} else {
    // haraka: uint64_t[10][8] + uint32_t[10][8]
    2 * SPX_N + 640 + 320
};

/// The backend's hash block / sponge rate, used to pick `mlen` boundaries.
pub const BACKEND_BLOCK: usize = if cfg!(feature = "sha2") {
    if WIDE {
        128
    } else {
        64
    }
} else if cfg!(any(feature = "shake", feature = "shake256")) {
    136
} else if cfg!(feature = "blake") {
    if WIDE {
        128
    } else {
        64
    }
} else {
    32 // haraka sponge rate
};

/// `SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES` from
/// `hash_sha2.c` — the `mlen` at which `hash_message` switches branch.
pub const HASH_MESSAGE_BOUNDARY: usize = {
    let b = if WIDE { 128 } else { 64 };
    let inblocks = (SPX_N + SPX_PK_BYTES + b - 1) / b;
    inblocks * b - SPX_N - SPX_PK_BYTES
};

/// `SPX_SHAX_BLOCK_BYTES - SPX_N` from `hash_sha2.c` — the `mlen` at which
/// `gen_message_random` switches branch.
pub const GEN_MSG_RANDOM_BOUNDARY: usize = (if WIDE { 128 } else { 64 }) - SPX_N;

/* ------------------------------------------------------------------ */
/* C structs that cross the FFI boundary                               */
/* ------------------------------------------------------------------ */

/// `wotsx1.h: struct leaf_info_x1`
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
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: std::ptr::null_mut(),
            leaf_addr: [0; 8],
            pk_addr: [0; 8],
        }
    }
}

/// `fors.h: struct fors_gen_leaf_info`
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

/// `rng.h: AES_XOF_struct`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

impl Default for AesXofStruct {
    fn default() -> Self {
        AesXofStruct {
            buffer: [0; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0; 32],
            ctr: [0; 16],
        }
    }
}

/// `rng.h: AES256_CTR_DRBG_struct`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Drbg {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

/// `blake.h: blakestate256`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl Default for BlakeState256 {
    fn default() -> Self {
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

/// `blake.h: blakestate512`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl Default for BlakeState512 {
    fn default() -> Self {
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

/* ------------------------------------------------------------------ */
/* Deterministic PRNG (fixed seed => reproducible property tests)      */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    pub fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn fill(&mut self, b: &mut [u8]) {
        for x in b.iter_mut() {
            *x = self.byte();
        }
    }
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for x in a.iter_mut() {
            *x = self.next_u32();
        }
        a
    }
}

/// The seed every property test starts from, so failures are reproducible.
pub const SEED: u64 = 0x5EED_C0FF_EEu64;
/// Randomized inputs per `CONFIGS.md` row.
pub const NUM_ITERS: usize = 32;

/* ------------------------------------------------------------------ */
/* Library loading                                                     */
/* ------------------------------------------------------------------ */

pub fn backend_name() -> &'static str {
    if cfg!(feature = "sha2") {
        "sha2"
    } else if cfg!(any(feature = "shake", feature = "shake256")) {
        "shake"
    } else if cfg!(feature = "blake") {
        "blake"
    } else {
        "haraka"
    }
}

pub fn thash_name() -> &'static str {
    if cfg!(feature = "simple") {
        "simple"
    } else {
        "robust"
    }
}

pub fn secpar_name() -> &'static str {
    if cfg!(feature = "256f") {
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
    }
}

pub fn combo() -> String {
    format!("{}_{}_{}", backend_name(), thash_name(), secpar_name())
}

/// Holds every `dlopen`ed handle; dropping it closes them.
pub struct Libs {
    c: Library,
    r: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/libsphincs_plus.so`, found relative to the test binary
/// (`target/<profile>/deps/<test>-<hash>`).  Overridable with `SPHINCS_RUST_SO`.
fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("SPHINCS_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    profile_dir.join("libsphincs_plus.so")
}

type SizeFn = unsafe extern "C" fn() -> u64;

/// Both `.so`s must be built for the parameter set this test binary was
/// compiled for.  A stale `target/<profile>/libsphincs_plus.so` from a previous
/// `--features` run would otherwise silently corrupt every buffer the tests
/// hand out (different `SPX_N`), so this is checked up front and loudly.
fn assert_param_set(l: &Libs, which: &str, get: impl Fn(&str) -> Symbol<SizeFn>) {
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let f = get(name);
        let got = unsafe { f() };
        assert_eq!(
            got, expect,
            "the {which} shared object is built for a DIFFERENT parameter set: \
             {name}() = {got}, expected {expect} for `{}`.\n\
             Rebuild it: cargo build --release --no-default-features --features {},{},{}  \
             (and ../build_c_flat.sh for the C side)",
            combo(),
            backend_name(),
            thash_name(),
            secpar_name(),
        );
    }
    let _ = l;
}

impl Libs {
    pub fn load() -> Libs {
        let root = manifest_dir().parent().unwrap().to_path_buf();
        let c_path = root
            .join("cbuild_flat")
            .join(format!("libspx_{}.so", combo()));
        let rust_path = rust_so();

        for p in [&c_path, &rust_path] {
            assert!(
                p.exists(),
                "missing shared object {}\n\
                 run ../build_c_flat.sh (C) and `cargo build --release` (Rust) first",
                p.display()
            );
        }

        // Both RTLD_LOCAL: the two libraries export the same names, and neither
        // has undefined project symbols, so nothing can be interposed and each
        // handle resolves strictly to its own implementation.
        let c = unsafe {
            Library::open(Some(&c_path), RTLD_NOW | RTLD_LOCAL)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()))
        };
        let r = unsafe {
            Library::open(Some(&rust_path), RTLD_NOW | RTLD_LOCAL)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()))
        };
        let l = Libs { c, r };
        // Guard against accidental interposition: the same name must resolve to
        // two different addresses.
        for name in ["SPX_thash", "crypto_sign_verify", "SPX_ull_to_bytes", "randombytes"] {
            let a = l.c::<unsafe extern "C" fn()>(name).into_raw();
            let b = l.rs::<unsafe extern "C" fn()>(name).into_raw();
            assert_ne!(
                a as usize, b as usize,
                "`{name}` resolved to the same address in both libraries — \
                 one is interposing the other, the differential test would be vacuous"
            );
        }
        assert_param_set(&l, "C", |n| l.c::<SizeFn>(n));
        assert_param_set(&l, "Rust", |n| l.rs::<SizeFn>(n));
        l
    }

    /// Look a symbol up in the C library.
    pub fn c<T>(&self, name: &str) -> Symbol<T> {
        unsafe {
            self.c
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|e| panic!("C symbol `{name}` not found: {e}"))
        }
    }

    /// Look a symbol up in the Rust cdylib.
    pub fn rs<T>(&self, name: &str) -> Symbol<T> {
        unsafe {
            self.r
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|e| panic!("Rust symbol `{name}` not found: {e}"))
        }
    }

    /// Both sides of one symbol, in `(c, rust)` order.
    pub fn pair<T>(&self, name: &str) -> (Symbol<T>, Symbol<T>) {
        (self.c(name), self.rs(name))
    }
}

/// One process-wide instance; `dlopen` is refcounted so this is just cheaper.
pub fn libs() -> &'static Libs {
    use std::sync::OnceLock;
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(Libs::load)
}

/// `rng.c` keeps its DRBG in the process-global `DRBG_ctx`, and `libtest` runs
/// tests in parallel threads.  Every test that seeds or draws from the DRBG
/// (directly, or indirectly via `crypto_sign_keypair` / `crypto_sign_signature`)
/// must hold this lock for its whole body, or the two libraries' DRBG streams
/// interleave differently and the comparison becomes meaningless.
pub fn drbg_lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

/* ------------------------------------------------------------------ */
/* spx_ctx helpers                                                     */
/* ------------------------------------------------------------------ */

/// A `spx_ctx` sized and aligned for the active configuration.  Backed by
/// `u64` so the haraka variant's `uint64_t[10][8]` is properly aligned.
pub struct Ctx {
    buf: Vec<u64>,
}

impl Ctx {
    pub fn new() -> Ctx {
        Ctx {
            buf: vec![0u64; (CTX_BYTES + 7) / 8],
        }
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
    pub fn set_seeds(&mut self, pub_seed: &[u8], sk_seed: &[u8]) {
        assert_eq!(pub_seed.len(), SPX_N);
        assert_eq!(sk_seed.len(), SPX_N);
        unsafe {
            std::ptr::copy_nonoverlapping(pub_seed.as_ptr(), self.as_mut_ptr(), SPX_N);
            std::ptr::copy_nonoverlapping(sk_seed.as_ptr(), self.as_mut_ptr().add(SPX_N), SPX_N);
        }
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Ctx::new()
    }
}

pub type InitHashFn = unsafe extern "C" fn(*mut u8);

/// Builds a pair of contexts with identical seeds, one initialised by the C
/// `SPX_initialize_hash_function` and one by the Rust one, and asserts the
/// resulting `spx_ctx` images are byte-identical.  Returns both.
pub fn init_ctx_pair(pub_seed: &[u8], sk_seed: &[u8]) -> (Ctx, Ctx) {
    let l = libs();
    let (cf, rf) = l.pair::<InitHashFn>("SPX_initialize_hash_function");
    let mut cc = Ctx::new();
    let mut rc = Ctx::new();
    cc.set_seeds(pub_seed, sk_seed);
    rc.set_seeds(pub_seed, sk_seed);
    unsafe {
        cf(cc.as_mut_ptr());
        rf(rc.as_mut_ptr());
    }
    assert_eq!(
        cc.bytes(),
        rc.bytes(),
        "SPX_initialize_hash_function produced different spx_ctx images"
    );
    (cc, rc)
}

/* ------------------------------------------------------------------ */
/* Assertion helpers                                                   */
/* ------------------------------------------------------------------ */

#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(usize::MAX);
        panic!(
            "{what}: C != Rust (len {} vs {}), first differing byte at index {}\n  C   : {}\n  Rust: {}",
            c.len(),
            r.len(),
            first as i64,
            hex(&c[..c.len().min(96)]),
            hex(&r[..r.len().min(96)]),
        );
    }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// The `mlen` values that cover every message-length branch in the backends.
pub fn mlen_cases() -> Vec<usize> {
    let mut v = vec![0usize, 1, 2, 31, 32, 33, 63, 64, 65];
    for b in [GEN_MSG_RANDOM_BOUNDARY, HASH_MESSAGE_BOUNDARY, BACKEND_BLOCK] {
        if b > 0 {
            v.push(b - 1);
        }
        v.push(b);
        v.push(b + 1);
        v.push(2 * b);
        v.push(2 * b + 1);
    }
    v.push(231);
    v.push(1000);
    v.sort_unstable();
    v.dedup();
    v
}
