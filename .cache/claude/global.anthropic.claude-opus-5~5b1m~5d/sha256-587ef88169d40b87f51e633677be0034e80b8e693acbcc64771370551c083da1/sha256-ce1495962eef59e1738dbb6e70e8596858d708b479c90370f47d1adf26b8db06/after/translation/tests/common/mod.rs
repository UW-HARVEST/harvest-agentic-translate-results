//! Differential-test harness: loads BOTH the C `libsodium.so` and the Rust
//! `liblibsodium.so` with `libloading` and calls every function through the FFI
//! boundary, exactly as an external consumer would.
//!
//! Nothing in this crate is ever called directly — only via `dlopen`/`dlsym`.
#![allow(dead_code)]
#![allow(unused_macros)]

use libloading::Library;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

fn rust_so_path() -> PathBuf {
    // test exe lives at <target>/<profile>/deps/<name>-<hash>; the cdylib is at
    // <target>/<profile>/liblibsodium.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    let p = profile_dir.join("liblibsodium.so");
    if p.exists() {
        return p;
    }
    // fallbacks
    for prof in ["release", "debug"] {
        let q = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(prof)
            .join("liblibsodium.so");
        if q.exists() {
            return q;
        }
    }
    panic!("cannot locate Rust liblibsodium.so (looked at {:?})", p);
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SODIUM_SO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("c_src")
        .join("build")
        .join("libsodium.so")
}

static LIBS: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("dlopen C {:?}: {}", cp, e));
        let r = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("dlopen Rust {:?}: {}", rp, e));
        // Both libraries need sodium_init() before some primitives are usable
        // (runtime implementation selection). Call it on both.
        unsafe {
            let ci = c
                .get::<unsafe extern "C" fn() -> i32>(b"sodium_init\0")
                .unwrap();
            let ri = r
                .get::<unsafe extern "C" fn() -> i32>(b"sodium_init\0")
                .unwrap();
            assert_eq!(ci(), ri(), "sodium_init return mismatch");
        }
        Pair { c, r }
    })
}

/// Fetch a function pointer of type `$t` named `$name` from library `$lib`.
#[macro_export]
macro_rules! getsym {
    ($lib:expr, $name:expr, $t:ty) => {{
        let s: libloading::Symbol<$t> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e))
        };
        *s
    }};
}

/// Fetch the same symbol from both libraries: `let (c, r) = both!("name", ty);`
#[macro_export]
macro_rules! both {
    ($name:expr, $t:ty) => {{
        let l = $crate::common::libs();
        (
            $crate::getsym!(l.c, $name, $t),
            $crate::getsym!(l.r, $name, $t),
        )
    }};
}

/// Fetch a data symbol (pointer to exported object) from both libraries.
#[macro_export]
macro_rules! both_data {
    ($name:expr, $t:ty) => {{
        let l = $crate::common::libs();
        unsafe {
            let cs: libloading::Symbol<*const $t> =
                l.c.get(concat!($name, "\0").as_bytes()).unwrap();
            let rs: libloading::Symbol<*const $t> =
                l.r.get(concat!($name, "\0").as_bytes()).unwrap();
            (*cs, *rs)
        }
    }};
}

// ---------------------------------------------------------------- rng --------

/// Deterministic splitmix64-based PRNG (fixed seed ⇒ reproducible tests).
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.u8();
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
}

// ------------------------------------------------------------- helpers -------

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

/// Assert two byte buffers are identical, printing hex on failure.
pub fn eqb(ctx: &str, c: &[u8], r: &[u8]) {
    if c != r {
        panic!(
            "{}: byte mismatch\n  C   = {}\n  Rust= {}",
            ctx,
            hex(c),
            hex(r)
        );
    }
}

pub fn eqi(ctx: &str, c: i32, r: i32) {
    assert_eq!(c, r, "{}: return-value mismatch", ctx);
}
