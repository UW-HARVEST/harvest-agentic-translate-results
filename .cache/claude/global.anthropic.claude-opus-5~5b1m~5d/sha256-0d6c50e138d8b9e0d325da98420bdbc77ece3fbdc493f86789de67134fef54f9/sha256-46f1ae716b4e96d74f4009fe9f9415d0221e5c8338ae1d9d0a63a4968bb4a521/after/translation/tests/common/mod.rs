//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded through `libloading` with `RTLD_NOW | RTLD_LOCAL`
//! so that the identically-named exports of the two `.so`s can never interpose
//! on one another. The Rust side is *always* reached through the `.so`'s
//! `#[no_mangle]` exports — never by calling the crate directly — so the export
//! wrappers themselves are under test.
#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::os::unix::{Library as UnixLibrary, Symbol as UnixSymbol, RTLD_LOCAL, RTLD_NOW};
use std::path::PathBuf;

pub use sphincs_plus::params::*;

// ---------------------------------------------------------------------------
// Configuration identity (mirrors build.rs's resolution of the Cargo features)
// ---------------------------------------------------------------------------

pub const BACKEND: &str = if cfg!(spx_backend = "haraka") {
    "haraka"
} else if cfg!(spx_backend = "sha2") {
    "sha2"
} else if cfg!(spx_backend = "shake") {
    "shake"
} else {
    "blake"
};

pub const THASH_VARIANT: &str = if cfg!(spx_thash = "robust") {
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

/// `true` when the backend's `thash` has a distinct 512-bit branch for
/// `inblocks > 1` (`SPX_SHA512` / `SPX_BLAKE512`).
pub const HAS_512_THASH_BRANCH: bool =
    (cfg!(spx_backend = "sha2") && cfg!(spx_sha512)) || (cfg!(spx_backend = "blake") && cfg!(spx_blake512));

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPX_C_LIB") {
        return PathBuf::from(p);
    }
    repo_root().join(".verify/clibs").join(format!(
        "libspx_c_{}_{}_{}.so",
        BACKEND, THASH_VARIANT, SECPAR
    ))
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("SPX_RUST_LIB") {
        return PathBuf::from(p);
    }
    // Prefer release (what the runner script builds), fall back to debug.
    let rel = repo_root().join("translation/target/release/libsphincs_plus.so");
    if rel.exists() {
        return rel;
    }
    repo_root().join("translation/target/debug/libsphincs_plus.so")
}

/// A loaded pair of libraries. `Libs::get::<F>(name)` returns the C and Rust
/// function pointers for the same symbol name.
pub struct Libs {
    pub c: UnixLibrary,
    pub r: UnixLibrary,
}

impl Libs {
    pub fn load() -> Libs {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        // RTLD_LOCAL keeps each library's symbols out of the global scope, so
        // the C `SPX_thash` can never satisfy a Rust relocation or vice versa.
        let c = unsafe { UnixLibrary::open(Some(&cp), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("failed to dlopen C lib {}: {e}", cp.display()));
        let r = unsafe { UnixLibrary::open(Some(&rp), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("failed to dlopen Rust lib {}: {e}", rp.display()));
        Libs { c, r }
    }

    pub fn sym<T>(&self, which: Which, name: &str) -> UnixSymbol<T> {
        let lib = match which {
            Which::C => &self.c,
            Which::R => &self.r,
        };
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        unsafe { lib.get::<T>(&b) }
            .unwrap_or_else(|e| panic!("{:?} lib is missing symbol `{name}`: {e}", which))
    }

    /// Both function pointers for one symbol.
    pub fn pair<T>(&self, name: &str) -> (UnixSymbol<T>, UnixSymbol<T>) {
        (self.sym::<T>(Which::C, name), self.sym::<T>(Which::R, name))
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Which {
    C,
    R,
}

/// One shared instance for the whole test binary.
pub fn libs() -> &'static Libs {
    use std::sync::OnceLock;
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(Libs::load)
}

/// The DRBG (`DRBG_ctx`) is a process-global in both libraries, so any test
/// that calls `randombytes*` / `crypto_sign_keypair` / `crypto_sign*` must hold
/// this lock to stay deterministic under `cargo test`'s thread pool.
pub fn drbg_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Human-readable config tag, used in assertion messages.
pub fn tag() -> String {
    format!("[{},{},{}]", BACKEND, THASH_VARIANT, SECPAR)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
        for w in a.iter_mut() {
            *w = self.next_u32();
        }
        a
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

// ---------------------------------------------------------------------------
// FFI struct mirrors (must match the C layouts EXACTLY)
// ---------------------------------------------------------------------------

/// Mirror of `spx_ctx` (`app/include/context.h`) for the ACTIVE backend.
#[repr(C)]
#[derive(Clone)]
pub struct SpxCtxFfi {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    #[cfg(spx_backend = "sha2")]
    pub state_seeded: [u8; 40],
    #[cfg(all(spx_backend = "sha2", spx_sha512))]
    pub state_seeded_512: [u8; 72],
    #[cfg(spx_backend = "haraka")]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(spx_backend = "haraka")]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtxFfi {
    pub fn zeroed() -> SpxCtxFfi {
        unsafe { core::mem::zeroed() }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<SpxCtxFfi>(),
            )
        }
    }
}

/// Mirror of `struct leaf_info_x1` (`app/include/wotsx1.h`).
#[repr(C)]
pub struct LeafInfoX1Ffi {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *const u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

/// Mirror of `struct fors_gen_leaf_info` (`app/include/fors.h`).
#[repr(C)]
#[derive(Clone)]
pub struct ForsGenLeafInfoFfi {
    pub leaf_addrx: [u32; 8],
}

/// Mirror of `AES_XOF_struct` (`app/include/rng.h`).
#[repr(C)]
#[derive(Clone)]
pub struct AesXofFfi {
    pub buffer: [u8; 16],
    pub buffer_pos: core::ffi::c_ulong,
    pub length_remaining: core::ffi::c_ulong,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

impl AesXofFfi {
    pub fn zeroed() -> AesXofFfi {
        unsafe { core::mem::zeroed() }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 80) }
    }
}

/// Mirror of `AES256_CTR_DRBG_struct` (`app/include/rng.h`).
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrbgCtxFfi {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: core::ffi::c_int,
}

/// Mirror of `blakestate256` (`lib/blake/include/blake.h`).
#[repr(C)]
#[derive(Clone)]
pub struct BlakeState256Ffi {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: core::ffi::c_int,
    pub nullt: core::ffi::c_int,
    pub buf: [u8; 64],
}

/// Mirror of `blakestate512` (`lib/blake/include/blake.h`).
#[repr(C)]
#[derive(Clone)]
pub struct BlakeState512Ffi {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: core::ffi::c_int,
    pub nullt: core::ffi::c_int,
    pub buf: [u8; 128],
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b.iter().take(96) {
        s.push_str(&format!("{:02x}", x));
    }
    if b.len() > 96 {
        s.push_str("...");
    }
    s
}

#[track_caller]
pub fn assert_bytes_eq(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.len().min(r.len()));
        panic!(
            "{} {}: C/Rust byte mismatch (len {} vs {}, first diff at {})\n  C = {}\n  R = {}",
            tag(),
            what,
            c.len(),
            r.len(),
            first,
            hex(c),
            hex(r)
        );
    }
}

#[track_caller]
pub fn assert_eq_dbg<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    if c != r {
        panic!("{} {}: C = {:?}, Rust = {:?}", tag(), what, c, r);
    }
}

/// Turn a `&[u32; 8]` address into the raw 32 bytes the C hashes.
pub fn addr_bytes(a: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(unsafe { core::slice::from_raw_parts(a.as_ptr() as *const u8, 32) });
    out
}
