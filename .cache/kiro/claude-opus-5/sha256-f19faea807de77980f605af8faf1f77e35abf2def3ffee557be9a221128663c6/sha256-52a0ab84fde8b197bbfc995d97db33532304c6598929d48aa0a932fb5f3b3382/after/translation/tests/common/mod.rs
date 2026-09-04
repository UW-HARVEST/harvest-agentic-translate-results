//! Differential-test harness.
//!
//! Both implementations are reached **only** through `dlopen`/`dlsym`, so the
//! `#[no_mangle] extern "C"` export wrappers of the Rust crate are part of what
//! is under test.  Nothing in these tests links the Rust library directly.
//!
//! * C side: `cbuild/<backend>_<thash>_<secpar>/app/libsphincs_core_det.so`
//!   together with `.../lib/<backend>/lib<backend>.so`.  The two objects
//!   reference each other's symbols (`libsphincs_core_det.so` has no
//!   `thash`/`prf_addr`, the backend has no `treehash` unless it happens to
//!   compile `utils.c`), so both are opened `RTLD_GLOBAL | RTLD_LAZY` and let
//!   the loader tie them together exactly as the `driver` link line does.
//! * Rust side: the `cdylib` cargo just built next to the test executable,
//!   opened `RTLD_LOCAL | RTLD_NOW` so that it can neither interpose on nor be
//!   interposed by the C objects.

#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::os::unix::{Library, Symbol};
use std::path::PathBuf;

const RTLD_LAZY: i32 = 0x1;
const RTLD_NOW: i32 = 0x2;
const RTLD_GLOBAL: i32 = 0x100;
const RTLD_LOCAL: i32 = 0x0;

// ---------------------------------------------------------------------------
// Build configuration, mirrored from c_src/app/params/params-*.h.
// ---------------------------------------------------------------------------

pub mod params {
    #[cfg(backend_blake)]
    pub const BACKEND: &str = "blake";
    #[cfg(backend_haraka)]
    pub const BACKEND: &str = "haraka";
    #[cfg(backend_sha2)]
    pub const BACKEND: &str = "sha2";
    #[cfg(backend_shake)]
    pub const BACKEND: &str = "shake";

    #[cfg(thash_robust)]
    pub const THASH: &str = "robust";
    #[cfg(thash_simple)]
    pub const THASH: &str = "simple";

    #[cfg(secpar_128s)]
    pub const SECPAR: &str = "128s";
    #[cfg(secpar_128f)]
    pub const SECPAR: &str = "128f";
    #[cfg(secpar_192s)]
    pub const SECPAR: &str = "192s";
    #[cfg(secpar_192f)]
    pub const SECPAR: &str = "192f";
    #[cfg(secpar_256s)]
    pub const SECPAR: &str = "256s";
    #[cfg(secpar_256f)]
    pub const SECPAR: &str = "256f";

    #[cfg(secpar_128s)]
    pub const SPX_N: usize = 16;
    #[cfg(secpar_128s)]
    pub const SPX_FULL_HEIGHT: usize = 63;
    #[cfg(secpar_128s)]
    pub const SPX_D: usize = 7;
    #[cfg(secpar_128s)]
    pub const SPX_FORS_HEIGHT: usize = 12;
    #[cfg(secpar_128s)]
    pub const SPX_FORS_TREES: usize = 14;

    #[cfg(secpar_128f)]
    pub const SPX_N: usize = 16;
    #[cfg(secpar_128f)]
    pub const SPX_FULL_HEIGHT: usize = 66;
    #[cfg(secpar_128f)]
    pub const SPX_D: usize = 22;
    #[cfg(secpar_128f)]
    pub const SPX_FORS_HEIGHT: usize = 6;
    #[cfg(secpar_128f)]
    pub const SPX_FORS_TREES: usize = 33;

    #[cfg(secpar_192s)]
    pub const SPX_N: usize = 24;
    #[cfg(secpar_192s)]
    pub const SPX_FULL_HEIGHT: usize = 63;
    #[cfg(secpar_192s)]
    pub const SPX_D: usize = 7;
    #[cfg(secpar_192s)]
    pub const SPX_FORS_HEIGHT: usize = 14;
    #[cfg(secpar_192s)]
    pub const SPX_FORS_TREES: usize = 17;

    #[cfg(secpar_192f)]
    pub const SPX_N: usize = 24;
    #[cfg(secpar_192f)]
    pub const SPX_FULL_HEIGHT: usize = 66;
    #[cfg(secpar_192f)]
    pub const SPX_D: usize = 22;
    #[cfg(secpar_192f)]
    pub const SPX_FORS_HEIGHT: usize = 8;
    #[cfg(secpar_192f)]
    pub const SPX_FORS_TREES: usize = 33;

    #[cfg(secpar_256s)]
    pub const SPX_N: usize = 32;
    #[cfg(secpar_256s)]
    pub const SPX_FULL_HEIGHT: usize = 64;
    #[cfg(secpar_256s)]
    pub const SPX_D: usize = 8;
    #[cfg(secpar_256s)]
    pub const SPX_FORS_HEIGHT: usize = 14;
    #[cfg(secpar_256s)]
    pub const SPX_FORS_TREES: usize = 22;

    #[cfg(secpar_256f)]
    pub const SPX_N: usize = 32;
    #[cfg(secpar_256f)]
    pub const SPX_FULL_HEIGHT: usize = 68;
    #[cfg(secpar_256f)]
    pub const SPX_D: usize = 17;
    #[cfg(secpar_256f)]
    pub const SPX_FORS_HEIGHT: usize = 9;
    #[cfg(secpar_256f)]
    pub const SPX_FORS_TREES: usize = 35;

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
    pub const SPX_BYTES: usize =
        SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
    pub const SPX_PK_BYTES: usize = 2 * SPX_N;
    pub const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
    pub const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

    /// `SPX_BLAKE512` / `SPX_SHA512`; 0 for the 128-bit sets, 1 otherwise,
    /// which is exactly `SPX_N >= 24`.
    pub const WIDE: bool = SPX_N >= 24;

    /// `sizeof(spx_ctx)` for the selected backend (`app/include/context.h`).
    pub const CTX_SIZE: usize = 2 * SPX_N
        + if cfg!(backend_sha2) {
            40 + if WIDE { 72 } else { 0 }
        } else {
            0
        }
        + if cfg!(backend_haraka) { 10 * 8 * 8 + 10 * 8 * 4 } else { 0 };

    /// Largest `inblocks` the library itself ever passes to `thash`.
    pub const THASH_MAX_INTERNAL: usize = if SPX_WOTS_LEN > SPX_FORS_TREES {
        SPX_WOTS_LEN
    } else {
        SPX_FORS_TREES
    };

    pub const SPX_SHA256_ADDR_BYTES: usize = 22;
}

pub fn tag() -> String {
    format!(
        "{}_{}_{}",
        params::BACKEND,
        params::THASH,
        params::SECPAR
    )
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c_core: Library,
    pub c_back: Library,
    pub rs: Library,
    pub rs_path: PathBuf,
    pub c_core_path: PathBuf,
    pub c_back_path: PathBuf,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn rust_so_path() -> PathBuf {
    // target/<profile>/deps/<test-exe>  ->  target/<profile>/libsphincsplus.so
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libsphincsplus.so")
}

pub fn c_core_path() -> PathBuf {
    workspace_root()
        .join("cbuild")
        .join(tag())
        .join("app/libsphincs_core_det.so")
}

pub fn c_back_path() -> PathBuf {
    workspace_root()
        .join("cbuild")
        .join(tag())
        .join(format!("lib/{0}/lib{0}.so", params::BACKEND))
}

pub fn load() -> Libs {
    let rs_path = rust_so_path();
    let c_core_path = c_core_path();
    let c_back_path = c_back_path();
    assert!(rs_path.exists(), "missing Rust cdylib at {rs_path:?}");
    assert!(
        c_core_path.exists(),
        "missing C core at {c_core_path:?}; run ./build_c_all.sh"
    );
    assert!(c_back_path.exists(), "missing C backend at {c_back_path:?}");

    unsafe {
        // Rust first, private and fully bound, so the C objects that follow
        // cannot interpose on it.
        let rs = Library::open(Some(&rs_path), RTLD_LOCAL | RTLD_NOW)
            .unwrap_or_else(|e| panic!("dlopen {rs_path:?}: {e}"));
        // The two C objects have mutually undefined symbols; lazy binding lets
        // both be mapped before anything is resolved.
        let c_back = Library::open(Some(&c_back_path), RTLD_GLOBAL | RTLD_LAZY)
            .unwrap_or_else(|e| panic!("dlopen {c_back_path:?}: {e}"));
        let c_core = Library::open(Some(&c_core_path), RTLD_GLOBAL | RTLD_LAZY)
            .unwrap_or_else(|e| panic!("dlopen {c_core_path:?}: {e}"));
        let libs = Libs {
            c_core,
            c_back,
            rs,
            rs_path,
            c_core_path,
            c_back_path,
        };
        libs.assert_configuration();
        libs
    }
}

type SizeFn = unsafe extern "C" fn() -> u64;

impl Libs {
    /// Guards against a stale `libsphincsplus.so` left behind by a build with
    /// different features: the loaded object must agree with the compile-time
    /// parameters of this test binary on both the backend and the key sizes.
    fn assert_configuration(&self) {
        // A symbol only the selected backend defines.
        let probe = match params::BACKEND {
            "blake" => "blake256",
            "haraka" => "SPX_haraka512",
            "sha2" => "sha256",
            _ => "shake256",
        };
        let mut n = probe.as_bytes().to_vec();
        n.push(0);
        unsafe {
            assert!(
                self.rs.get::<*const ()>(&n).is_ok(),
                "the Rust .so at {:?} does not export {probe}: it was built for a different \
                 HASH_BACKEND than this test binary. Run `cargo build --release --features ...` \
                 with the same features before `cargo test`.",
                self.rs_path
            );
        }
        for (name, expect) in [
            ("crypto_sign_secretkeybytes", params::SPX_SK_BYTES as u64),
            ("crypto_sign_publickeybytes", params::SPX_PK_BYTES as u64),
            ("crypto_sign_bytes", params::SPX_BYTES as u64),
            ("crypto_sign_seedbytes", params::CRYPTO_SEEDBYTES as u64),
        ] {
            let (fc, fr) = self.pair::<SizeFn>(name);
            unsafe {
                assert_eq!(fc(), expect, "C {name} disagrees with the test's params");
                assert_eq!(
                    fr(),
                    expect,
                    "Rust {name} disagrees with the test's params: stale .so at {:?}?",
                    self.rs_path
                );
            }
        }
    }
}

impl Libs {
    /// A symbol from the C pair (core first, then the backend object).
    pub fn c<T>(&self, name: &str) -> Symbol<T> {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe {
            match self.c_core.get::<T>(&n) {
                Ok(s) => s,
                Err(_) => self
                    .c_back
                    .get::<T>(&n)
                    .unwrap_or_else(|e| panic!("C symbol {name} not found: {e}")),
            }
        }
    }

    /// A symbol from the Rust `cdylib`.
    pub fn r<T>(&self, name: &str) -> Symbol<T> {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe {
            self.rs
                .get::<T>(&n)
                .unwrap_or_else(|e| panic!("Rust symbol {name} not found: {e}"))
        }
    }

    /// Both sides of the same symbol.
    pub fn pair<T>(&self, name: &str) -> (Symbol<T>, Symbol<T>) {
        (self.c(name), self.r(name))
    }
}

// ---------------------------------------------------------------------------
// Deterministic randomness for the property-style sweeps
// ---------------------------------------------------------------------------

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
    pub fn fill(&mut self, out: &mut [u8]) {
        let b = self.bytes(out.len());
        out.copy_from_slice(&b);
    }
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for x in a.iter_mut() {
            *x = self.next_u32();
        }
        a
    }
}

/// The message lengths that straddle every block/rate boundary any backend
/// branches on.  See CONFIGS.md.
pub const MLEN_SWEEP: &[usize] = &[
    0, 1, 2, 15, 16, 17, 31, 32, 33, 47, 48, 49, 55, 56, 57, 63, 64, 65, 71, 72, 73, 95, 96, 97,
    103, 104, 105, 127, 128, 129, 135, 136, 137, 167, 168, 169, 191, 192, 193, 255, 256, 257, 1000,
    4096,
];

/// Reduced sweep for the rows whose cost is a full SPHINCS+ operation.  The
/// message-length branches themselves are swept densely and cheaply by rows 10
/// and 11 (`gen_message_random` / `hash_message`), which are the only places
/// `mlen` is branched on; these values exist to confirm the composition.
pub const MLEN_SWEEP_SMALL: &[usize] = &[0, 1, 33, 137];

// ---------------------------------------------------------------------------
// spx_ctx
// ---------------------------------------------------------------------------

/// An 8-byte aligned `spx_ctx` sized buffer (`haraka`'s members are `uint64_t`).
pub struct Ctx {
    buf: Vec<u64>,
}

impl Ctx {
    pub fn new() -> Self {
        Ctx {
            buf: vec![0u64; (params::CTX_SIZE + 7) / 8],
        }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.buf.as_ptr() as *const u8, params::CTX_SIZE)
        }
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self.buf.as_mut_ptr() as *mut u8, params::CTX_SIZE)
        }
    }
    pub fn ptr(&self) -> *const u8 {
        self.buf.as_ptr() as *const u8
    }
    pub fn ptr_mut(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr() as *mut u8
    }
    pub fn set_seeds(&mut self, pub_seed: &[u8], sk_seed: &[u8]) {
        let n = params::SPX_N;
        self.bytes_mut()[..n].copy_from_slice(&pub_seed[..n]);
        self.bytes_mut()[n..2 * n].copy_from_slice(&sk_seed[..n]);
    }
}

pub type InitHashFn = unsafe extern "C" fn(*mut u8);

/// Builds one `spx_ctx` per side by calling that side's own
/// `SPX_initialize_hash_function`, and asserts the two byte images agree.
pub fn make_ctx_pair(libs: &Libs, pub_seed: &[u8], sk_seed: &[u8]) -> (Ctx, Ctx) {
    let (ic, ir) = libs.pair::<InitHashFn>("SPX_initialize_hash_function");
    let mut cc = Ctx::new();
    let mut cr = Ctx::new();
    cc.set_seeds(pub_seed, sk_seed);
    cr.set_seeds(pub_seed, sk_seed);
    unsafe {
        ic(cc.ptr_mut());
        ir(cr.ptr_mut());
    }
    assert_eq!(
        cc.bytes(),
        cr.bytes(),
        "SPX_initialize_hash_function produced different spx_ctx images"
    );
    (cc, cr)
}

// ---------------------------------------------------------------------------
// C struct mirrors
// ---------------------------------------------------------------------------

/// `app/include/wotsx1.h` `leaf_info_x1`
#[repr(C)]
#[derive(Clone, Copy)]
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
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: 0,
            wots_steps: core::ptr::null_mut(),
            leaf_addr: [0; 8],
            pk_addr: [0; 8],
        }
    }
}

/// `app/include/fors.h` `fors_gen_leaf_info`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

/// `app/include/rng.h` `AES_XOF_struct`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
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
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<AesXofStruct>(),
            )
        }
    }
}

/// `app/include/rng.h` `AES256_CTR_DRBG_struct`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Aes256CtrDrbgStruct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

impl Aes256CtrDrbgStruct {
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<Aes256CtrDrbgStruct>(),
            )
        }
    }
}

/// `lib/blake/include/blake.h` `blakestate256`
#[repr(C)]
#[derive(Clone, Copy)]
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
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<BlakeState256>(),
            )
        }
    }
}

/// `lib/blake/include/blake.h` `blakestate512`
#[repr(C)]
#[derive(Clone, Copy)]
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
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<BlakeState512>(),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison helper
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.len().min(r.len()));
        panic!(
            "{what}: C and Rust differ (len {} vs {}) at byte {}\n  C  = {}\n  RS = {}",
            c.len(),
            r.len(),
            first,
            hex(&c[first.saturating_sub(4)..(first + 12).min(c.len())]),
            hex(&r[first.saturating_sub(4)..(first + 12).min(r.len())]),
        );
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn u32s_as_bytes(a: &[u32]) -> Vec<u8> {
    let mut v = Vec::with_capacity(a.len() * 4);
    for x in a {
        v.extend_from_slice(&x.to_ne_bytes());
    }
    v
}
