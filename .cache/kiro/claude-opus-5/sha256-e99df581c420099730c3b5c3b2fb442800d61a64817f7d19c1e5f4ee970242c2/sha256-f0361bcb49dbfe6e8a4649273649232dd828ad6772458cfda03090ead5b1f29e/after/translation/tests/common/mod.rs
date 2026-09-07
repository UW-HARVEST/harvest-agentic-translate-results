//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `hdr_compare`
//! only through the dynamic symbol, exactly as an external C consumer would.
//! The Rust crate is never linked directly, so the `#[no_mangle] extern "C"`
//! export wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> std::ffi::c_int;

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: HdrCompareFn,
    pub rs: HdrCompareFn,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(&build).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    });
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found.push(p);
        }
    }
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    let target = workspace_root().join("translation").join("target");
    // Prefer the profile the tests themselves were built with, then fall back.
    let profiles = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for prof in profiles {
        let p = target.join(prof).join("libhdr_compare_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libhdr_compare_lib.so not found under {}. Build it first: cargo build --release",
        target.display()
    );
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(find_c_so()).expect("failed to dlopen the C .so");
            let rust_lib = Library::new(find_rust_so()).expect("failed to dlopen the Rust .so");
            let c: Symbol<HdrCompareFn> = c_lib
                .get(b"hdr_compare\0")
                .expect("C .so does not export hdr_compare");
            let rs: Symbol<HdrCompareFn> = rust_lib
                .get(b"hdr_compare\0")
                .expect("Rust .so does not export hdr_compare");
            let c = *c;
            let rs = *rs;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rs,
            }
        }
    }

    /// Call both implementations and assert byte-identical `int` results.
    /// Also asserts the C return value stays in its documented `{0,1}` domain
    /// (ERRORS.md G6).
    #[inline]
    pub fn assert_same(&self, h1: &[u8; 3], h2: &[u8; 3], row: &str) -> i32 {
        let a = unsafe { (self.c)(h1.as_ptr(), h2.as_ptr()) };
        let b = unsafe { (self.rs)(h1.as_ptr(), h2.as_ptr()) };
        assert!(
            a == 0 || a == 1,
            "[{row}] C returned out-of-domain {a} for h1={h1:02x?} h2={h2:02x?}"
        );
        assert_eq!(
            a, b,
            "[{row}] DIVERGENCE h1={h1:02x?} h2={h2:02x?}: C={a} Rust={b}"
        );
        a
    }

    /// Raw-pointer variant, for NULL / guard-page / aliasing / offset tests.
    #[inline]
    pub fn assert_same_raw(&self, h1: *const u8, h2: *const u8, row: &str) -> i32 {
        let a = unsafe { (self.c)(h1, h2) };
        let b = unsafe { (self.rs)(h1, h2) };
        assert_eq!(
            a, b,
            "[{row}] DIVERGENCE h1={h1:p} h2={h2:p}: C={a} Rust={b}"
        );
        a
    }
}

/// SplitMix64 — deterministic, seeded, no external dependency.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn seeded() -> Rng {
        Rng(Self::SEED)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[inline]
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % u64::from(n)) as u32
    }

    #[inline]
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u32) as usize]
    }
}

// ---------------------------------------------------------------------------
// Shape helpers, all transcribed straight from `c_src/src/lib.c`.
// ---------------------------------------------------------------------------

/// Oracle-free structural predicates (used only to *construct* inputs and to
/// partition rows — never to decide the expected answer; that always comes
/// from the C `.so`).
pub fn class_f(b1: u8) -> bool {
    (b1 & 0xF0) == 0xF0
}
pub fn class_e(b1: u8) -> bool {
    (b1 & 0xFE) == 0xE2
}
pub fn layer(b1: u8) -> u8 {
    (b1 >> 1) & 3
}
pub fn nibble(b2: u8) -> u8 {
    b2 >> 4
}
pub fn sbits(b2: u8) -> u8 {
    (b2 >> 2) & 3
}

pub fn h2_is_valid(h: &[u8; 3]) -> bool {
    h[0] == 0xff
        && (class_f(h[1]) || class_e(h[1]))
        && layer(h[1]) != 0
        && nibble(h[2]) != 15
        && sbits(h[2]) != 3
}

/// All `h2[1]` values that pass `hdr_valid`'s byte-1 tests, by (class, layer).
pub fn valid_b1_values() -> Vec<u8> {
    (0u16..=255)
        .map(|v| v as u8)
        .filter(|&b| (class_f(b) || class_e(b)) && layer(b) != 0)
        .collect()
}

/// All `h2[2]` values that pass `hdr_valid`'s byte-2 tests.
pub fn valid_b2_values() -> Vec<u8> {
    (0u16..=255)
        .map(|v| v as u8)
        .filter(|&b| nibble(b) != 15 && sbits(b) != 3)
        .collect()
}

/// Build an `h1` that agrees with `h2` on every bit the C compares, with all
/// ignored bits (`h1[0]`, bit `0x01` of `h1[1]`, bits `0x03` of `h1[2]`, and the
/// high nibble when it only needs to be *non-zero*) randomized.
pub fn matching_h1(h2: &[u8; 3], rng: &mut Rng) -> [u8; 3] {
    let b1 = (h2[1] & 0xFE) | (rng.u8() & 0x01);
    let hi = if h2[2] & 0xF0 == 0 {
        0x00
    } else {
        // any non-zero high nibble, including 0xF0 which hdr_valid would reject
        ((rng.below(15) as u8) + 1) << 4
    };
    let b2 = hi | (h2[2] & 0x0C) | (rng.u8() & 0x03);
    [rng.u8(), b1, b2]
}
