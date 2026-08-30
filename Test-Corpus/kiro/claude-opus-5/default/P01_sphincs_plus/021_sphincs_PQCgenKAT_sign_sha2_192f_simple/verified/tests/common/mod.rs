//! Shared harness for the C-vs-Rust FFI differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! every call goes through `dlsym`, so the tests exercise exactly the same
//! entry points an external C caller would use — including the `#[no_mangle]`
//! export wrappers of the Rust crate.
//!
//! * C   : `cbuild/<backend>_<secpar>_<thash>/lib/<backend>/lib<backend>.so`
//!         plus `.../app/libsphincs_core_det.so`
//! * Rust: `translation/target/{debug,release}/libsphincsplus.so`
//!
//! The parameter constants below are transcribed independently from
//! `c_src/app/params/params-sphincs-*.h` so that a mistake in the crate's
//! `src/params.rs` shows up as a test failure rather than being papered over.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::os::unix::{Library, Symbol, RTLD_LOCAL, RTLD_NOW};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Build-time configuration, mirrored from the Cargo features.
// ---------------------------------------------------------------------------

/// `HASH_BACKEND`.  The precedence matches `src/tree.rs`.
pub const BACKEND: &str = if cfg!(feature = "blake") {
    "blake"
} else if cfg!(feature = "shake") {
    "shake"
} else if cfg!(feature = "sha2") {
    "sha2"
} else {
    "haraka"
};

/// `THASH`.
pub const THASH: &str = if cfg!(feature = "simple") {
    "simple"
} else {
    "robust"
};

/// `SECPAR`.  The precedence matches `src/params.rs`.
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
// Parameters, transcribed from c_src/app/params/params-sphincs-<b>-<secpar>.h.
// The five primary values are identical across all four hash backends.
// ---------------------------------------------------------------------------

/// `(SPX_N, SPX_FULL_HEIGHT, SPX_D, SPX_FORS_HEIGHT, SPX_FORS_TREES)`
const PRIMARY: (usize, usize, usize, usize, usize) = match SECPAR.as_bytes() {
    b"128s" => (16, 63, 7, 12, 14),
    b"128f" => (16, 66, 22, 6, 33),
    b"192s" => (24, 63, 7, 14, 17),
    b"192f" => (24, 66, 22, 8, 33),
    b"256s" => (32, 64, 8, 14, 22),
    b"256f" => (32, 68, 17, 9, 35),
    _ => panic!("unknown SECPAR"),
};

pub const SPX_N: usize = PRIMARY.0;
pub const SPX_FULL_HEIGHT: usize = PRIMARY.1;
pub const SPX_D: usize = PRIMARY.2;
pub const SPX_FORS_HEIGHT: usize = PRIMARY.3;
pub const SPX_FORS_TREES: usize = PRIMARY.4;

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
pub const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

pub const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

pub const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
pub const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
pub const SPX_FORS_PK_BYTES: usize = SPX_N;

pub const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: usize = 2 * SPX_N;
pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

pub const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
pub const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
pub const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
pub const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

/// `SPX_SHA512` / `SPX_BLAKE512` from the parameter headers.
pub const USES_512: bool = SPX_N >= 24;

/* Address offsets, transcribed from lib/<backend>/include/<backend>_offsets.h. */
/// `(LAYER, TREE, TYPE, KP_ADDR, CHAIN_ADDR, HASH_ADDR, TREE_HGT, TREE_INDEX)`
const OFFSETS: (usize, usize, usize, usize, usize, usize, usize, usize) =
    if matches!(BACKEND.as_bytes(), b"sha2") {
        (0, 1, 9, 10, 17, 21, 17, 18)
    } else {
        (3, 8, 19, 20, 27, 31, 27, 28)
    };

pub const SPX_OFFSET_LAYER: usize = OFFSETS.0;
pub const SPX_OFFSET_TREE: usize = OFFSETS.1;
pub const SPX_OFFSET_TYPE: usize = OFFSETS.2;
pub const SPX_OFFSET_KP_ADDR: usize = OFFSETS.3;
pub const SPX_OFFSET_CHAIN_ADDR: usize = OFFSETS.4;
pub const SPX_OFFSET_HASH_ADDR: usize = OFFSETS.5;
pub const SPX_OFFSET_TREE_HGT: usize = OFFSETS.6;
pub const SPX_OFFSET_TREE_INDEX: usize = OFFSETS.7;

/* Address type constants (app/include/address.h). */
pub const SPX_ADDR_TYPE_WOTS: u32 = 0;
pub const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
pub const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
pub const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
pub const SPX_ADDR_TYPE_FORSPK: u32 = 4;
pub const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
pub const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// ---------------------------------------------------------------------------
// `spx_ctx` (app/include/context.h) as an opaque, correctly sized byte buffer.
// ---------------------------------------------------------------------------

/// Size of the backend-specific tail of `spx_ctx`.
pub const CTX_TAIL_BYTES: usize = match BACKEND.as_bytes() {
    // uint8_t state_seeded[40] (+ uint8_t state_seeded_512[72] if SPX_SHA512)
    b"sha2" => 40 + if USES_512 { 72 } else { 0 },
    // uint64_t tweaked512_rc64[10][8]; uint32_t tweaked256_rc32[10][8];
    b"haraka" => 10 * 8 * 8 + 10 * 8 * 4,
    _ => 0,
};

pub const CTX_BYTES: usize = 2 * SPX_N + CTX_TAIL_BYTES;

/// A heap allocation holding an `spx_ctx`, aligned to 8 bytes (the strictest
/// alignment any member of the C struct requires).
#[repr(C, align(8))]
pub struct Ctx {
    pub bytes: [u8; CTX_BYTES],
}

impl Ctx {
    pub fn new() -> Box<Ctx> {
        Box::new(Ctx {
            bytes: [0u8; CTX_BYTES],
        })
    }

    /// Builds a context with `pub_seed`/`sk_seed` filled from a seed byte.
    pub fn seeded(tag: u8) -> Box<Ctx> {
        let mut c = Ctx::new();
        for i in 0..SPX_N {
            c.bytes[i] = tag ^ (0x40 + i as u8);
            c.bytes[SPX_N + i] = tag ^ (0x90 + i as u8);
        }
        c
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.bytes.as_mut_ptr()
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

/// `fors_gen_leaf_info` from `app/include/fors.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

/// `AES_XOF_struct` from `app/include/rng.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: std::os::raw::c_ulong,
    pub length_remaining: std::os::raw::c_ulong,
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
#[derive(Clone, Copy)]
pub struct Drbg {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: std::os::raw::c_int,
}

// ---------------------------------------------------------------------------
// Library loading.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at translation/.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_build_dir() -> PathBuf {
    workspace_root()
        .join("cbuild")
        .join(format!("{BACKEND}_{SECPAR}_{THASH}"))
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPHINCS_RUST_SO") {
        return PathBuf::from(p);
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["debug", "release"] {
        let p = target.join(profile).join("libsphincsplus.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libsphincsplus.so not found under {}; run `cargo build` with the same \
         features before `cargo test`",
        target.display()
    );
}

/// Holds the two implementations.  `Symbol`s borrow from these libraries, so
/// the struct is leaked into a `'static` reference by [`libs`].
pub struct Libs {
    /// `libsphincs_all.so`: an empty stub whose `DT_NEEDED` list pulls in
    /// `libsphincs_core_det.so`, `lib<backend>.so` and `libcrypto.so.3`.  It is
    /// opened `RTLD_LOCAL`, so the C definitions never reach the global scope
    /// and cannot interpose on the Rust cdylib's own exported globals.
    pub c_all: Library,
    /// The Rust cdylib, also `RTLD_LOCAL` for the same reason.
    pub rust: Library,
    /// `libsphincs_all_urandom.so`: the same stub built around
    /// `libsphincs_core.so` (`randombytes.c`) instead of the deterministic
    /// core.  Only used by the `urandom` tests.
    pub c_urandom: Library,
}

impl Libs {
    /// Looks a C symbol up through the combined stub.  `dlsym` searches the
    /// stub's dependencies in `DT_NEEDED` order, i.e. `libsphincs_core_det.so`
    /// before `lib<backend>.so`, which is the order the CMake `driver` target
    /// links them in.
    pub unsafe fn c<T>(&self, name: &str) -> Symbol<T> {
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            self.c_all
                .get::<T>(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("C symbol {name} not found: {e}"))
        }
    }

    /// Backend-only symbols resolve through the same handle.
    pub unsafe fn c_backend<T>(&self, name: &str) -> Symbol<T> {
        unsafe { self.c(name) }
    }

    pub unsafe fn r<T>(&self, name: &str) -> Symbol<T> {
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            self.rust
                .get::<T>(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("Rust symbol {name} not found: {e}"))
        }
    }

    /// Looks a symbol up in the C library built around `randombytes.c`.
    pub unsafe fn c_urandom<T>(&self, name: &str) -> Symbol<T> {
        let cname = std::ffi::CString::new(name).unwrap();
        unsafe {
            self.c_urandom
                .get::<T>(cname.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("C (urandom) symbol {name} not found: {e}"))
        }
    }
}

static LIBS: std::sync::OnceLock<&'static Libs> = std::sync::OnceLock::new();

/// Loads (once) and returns both implementations.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let cdir = c_build_dir();
        let all_so = cdir.join("libsphincs_all.so");
        assert!(
            all_so.exists(),
            "missing {}; run ./build_c.sh {BACKEND} {SECPAR} {THASH}",
            all_so.display()
        );
        let c_all = unsafe { Library::open(Some(&all_so), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", all_so.display()));
        let ur_so = cdir.join("libsphincs_all_urandom.so");
        let c_urandom = unsafe { Library::open(Some(&ur_so), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", ur_so.display()));
        let rso = rust_so_path();
        let rust = unsafe { Library::open(Some(&rso), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rso.display()));
        Box::leak(Box::new(Libs {
            c_all,
            rust,
            c_urandom,
        }))
    })
}

/// `urandom`: the exported `randombytes` comes from `randombytes.c`
/// (`/dev/urandom`) rather than the `rng.c` DRBG.  On the C side that is the
/// difference between the `sphincs_core` and `sphincs_core_det` libraries.
pub const URANDOM: bool = cfg!(feature = "urandom");

// ---------------------------------------------------------------------------
// DRBG helpers.
//
// `crypto_sign_signature` and `crypto_sign_keypair` draw from the global
// `randombytes` DRBG, so the C and the Rust library only agree when their
// `DRBG_ctx` is in the same state at the moment of the call.  Tests that touch
// the DRBG take `drbg_lock()` (the harness runs tests in parallel threads
// inside one process) and re-seed both libraries with `reseed_drbgs` before
// each call.
// ---------------------------------------------------------------------------

static DRBG_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serialises access to the two libraries' global DRBG state.
pub fn drbg_lock() -> std::sync::MutexGuard<'static, ()> {
    DRBG_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

type FnRandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);

/// Puts both libraries' `DRBG_ctx` into the same state.
pub fn reseed_drbgs(entropy: &[u8; 48]) {
    let l = libs();
    let c = unsafe { l.c::<FnRandombytesInit>("randombytes_init") };
    let r = unsafe { l.r::<FnRandombytesInit>("randombytes_init") };
    let mut e = *entropy;
    unsafe {
        c(e.as_mut_ptr(), core::ptr::null_mut());
        r(e.as_mut_ptr(), core::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random test inputs (xorshift64*, no crate needed).
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
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
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = (self.next_u64() >> 24) as u8;
        }
    }
    pub fn vec(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
}

/// Asserts two byte buffers are identical, printing a short diff on failure.
#[track_caller]
pub fn assert_bytes_eq(what: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let first = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(c.len().min(r.len()));
    panic!(
        "{what} mismatch ({BACKEND}/{THASH}/{SECPAR}): lengths {}/{}, first difference at byte {first}\n  C   : {:02x?}\n  Rust: {:02x?}",
        c.len(),
        r.len(),
        &c[first.saturating_sub(4)..(first + 12).min(c.len())],
        &r[first.saturating_sub(4)..(first + 12).min(r.len())],
    );
}
