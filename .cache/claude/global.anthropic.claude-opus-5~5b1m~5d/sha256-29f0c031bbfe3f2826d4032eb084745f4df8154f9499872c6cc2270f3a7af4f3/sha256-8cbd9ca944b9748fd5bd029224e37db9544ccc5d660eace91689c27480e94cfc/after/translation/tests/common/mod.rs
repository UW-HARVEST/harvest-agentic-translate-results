//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes one `Lib` handle per
//! implementation. Nothing here ever calls a Rust function directly — every
//! call goes through the `.so`'s exported C symbol, so the `#[no_mangle]`
//! wrappers and the C ABI are what actually gets tested.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// FFI types — byte-identical to the C definitions in c_src/src/lib.c
// ---------------------------------------------------------------------------

pub type C2_TYPE = c_uint;
pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

pub fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}
pub fn circle(x: f32, y: f32, r: f32) -> c2Circle {
    c2Circle { p: v(x, y), r }
}
pub fn aabb(minx: f32, miny: f32, maxx: f32, maxy: f32) -> c2AABB {
    c2AABB {
        min: v(minx, miny),
        max: v(maxx, maxy),
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Raw bits of an `f32`. Used for ALL float comparisons so that NaN payloads
/// and the sign of zero are part of the assertion.
pub fn bits(f: f32) -> u32 {
    f.to_bits()
}

pub fn vbits(a: c2v) -> (u32, u32) {
    (a.x.to_bits(), a.y.to_bits())
}

pub fn show(f: f32) -> String {
    format!("{:e} (0x{:08x})", f, f.to_bits())
}

pub fn showv(a: c2v) -> String {
    format!("({}, {})", show(a.x), show(a.y))
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}. Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    assert!(
        !candidates.is_empty(),
        "no lib*.so found in {}",
        build.display()
    );
    candidates.remove(0)
}

/// `cargo test` does not build `cdylib` artifacts (nothing in the test graph
/// depends on them), so the `.so` may legitimately be absent on a clean tree.
/// Build it on demand, exactly once per test binary, for the profile this test
/// binary was itself compiled with.
fn ensure_rust_so_built(profile_dir: &std::path::Path) {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let release = profile_dir
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.arg("build")
            .arg("--offline")
            .arg("--lib")
            .current_dir(crate_root());
        if release {
            cmd.arg("--release");
        }
        // Best-effort: if this fails, the caller's `panic!` reports the real
        // problem with a clearer message.
        let _ = cmd.status();
    });
}

fn find_rust_so() -> PathBuf {
    // tests/ binaries live in target/<profile>/deps/, so the cdylib is one
    // directory up from the test executable.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile, deps] {
        let p = dir.join("libcollided_lib.so");
        if p.exists() {
            return p;
        }
    }
    ensure_rust_so_built(profile);
    for dir in [profile, deps] {
        let p = dir.join("libcollided_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libcollided_lib.so not found next to {}; run `cargo build` first",
        exe.display()
    );
}

pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

/// The ten exported entry points, resolved by name out of a `.so`.
impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Lib { name, lib }
    }

    fn sym<T>(&self, s: &str) -> Symbol<'_, T> {
        unsafe { self.lib.get(s.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{s}`: {e}", self.name))
    }

    pub fn c2V(&self, x: f32, y: f32) -> c2v {
        let f: Symbol<extern "C" fn(f32, f32) -> c2v> = self.sym("c2V");
        f(x, y)
    }
    pub fn c2Maxv(&self, a: c2v, b: c2v) -> c2v {
        let f: Symbol<extern "C" fn(c2v, c2v) -> c2v> = self.sym("c2Maxv");
        f(a, b)
    }
    pub fn c2Minv(&self, a: c2v, b: c2v) -> c2v {
        let f: Symbol<extern "C" fn(c2v, c2v) -> c2v> = self.sym("c2Minv");
        f(a, b)
    }
    pub fn c2Clampv(&self, a: c2v, lo: c2v, hi: c2v) -> c2v {
        let f: Symbol<extern "C" fn(c2v, c2v, c2v) -> c2v> = self.sym("c2Clampv");
        f(a, lo, hi)
    }
    pub fn c2Sub(&self, a: c2v, b: c2v) -> c2v {
        let f: Symbol<extern "C" fn(c2v, c2v) -> c2v> = self.sym("c2Sub");
        f(a, b)
    }
    pub fn c2Dot(&self, a: c2v, b: c2v) -> f32 {
        let f: Symbol<extern "C" fn(c2v, c2v) -> f32> = self.sym("c2Dot");
        f(a, b)
    }
    pub fn c2CircletoCircle(&self, a: c2Circle, b: c2Circle) -> c_int {
        let f: Symbol<extern "C" fn(c2Circle, c2Circle) -> c_int> = self.sym("c2CircletoCircle");
        f(a, b)
    }
    pub fn c2CircletoAABB(&self, a: c2Circle, b: c2AABB) -> c_int {
        let f: Symbol<extern "C" fn(c2Circle, c2AABB) -> c_int> = self.sym("c2CircletoAABB");
        f(a, b)
    }
    pub fn c2AABBtoAABB(&self, a: c2AABB, b: c2AABB) -> c_int {
        let f: Symbol<extern "C" fn(c2AABB, c2AABB) -> c_int> = self.sym("c2AABBtoAABB");
        f(a, b)
    }
    /// Raw `collided`: takes whatever pointers/tags the caller supplies, so the
    /// error-path tests can pass NULL and out-of-range enum values.
    pub fn collided_raw(
        &self,
        a: *const c_void,
        ta: C2_TYPE,
        b: *const c_void,
        tb: C2_TYPE,
    ) -> c_int {
        let f: Symbol<
            unsafe extern "C" fn(*const c_void, C2_TYPE, *const c_void, C2_TYPE) -> c_int,
        > = self.sym("collided");
        unsafe { f(a, ta, b, tb) }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

/// Open both libraries. Called once per test.
pub fn libs() -> Pair {
    Pair {
        c: Lib::open("C", find_c_so()),
        rs: Lib::open("Rust", find_rust_so()),
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! assert_int_eq {
    ($row:expr, $c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv) = ($c, $r);
        assert_eq!(
            cv, rv,
            "[{}] C returned {} but Rust returned {} for {}",
            $row, cv, rv, format_args!($($ctx)*)
        );
    }};
}

#[macro_export]
macro_rules! assert_f32_bits_eq {
    ($row:expr, $c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv) = ($c, $r);
        assert_eq!(
            cv.to_bits(), rv.to_bits(),
            "[{}] C returned {} but Rust returned {} for {}",
            $row, $crate::common::show(cv), $crate::common::show(rv),
            format_args!($($ctx)*)
        );
    }};
}

#[macro_export]
macro_rules! assert_v_bits_eq {
    ($row:expr, $c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv) = ($c, $r);
        assert_eq!(
            $crate::common::vbits(cv), $crate::common::vbits(rv),
            "[{}] C returned {} but Rust returned {} for {}",
            $row, $crate::common::showv(cv), $crate::common::showv(rv),
            format_args!($($ctx)*)
        );
    }};
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed ⇒ reproducible property tests)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
        (self.next_u64() >> 32) as u32
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `[-1, 1)`, scaled by `scale`.
    pub fn signed(&mut self, scale: f32) -> f32 {
        let u = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
        (u * 2.0 - 1.0) * scale
    }

    /// A float with a fully random bit pattern — hits every IEEE class
    /// (normals, subnormals, zeros, infinities, quiet and signalling NaNs).
    pub fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A float drawn from a "nasty" distribution: mostly ordinary values but
    /// with special values injected often enough to exercise them densely.
    pub fn nasty_f32(&mut self) -> f32 {
        const SPECIALS: [u32; 16] = [
            0x0000_0000, // +0.0
            0x8000_0000, // -0.0
            0x7F80_0000, // +inf
            0xFF80_0000, // -inf
            0x7FC0_0000, // default qNaN
            0x7FC0_0001, // qNaN, payload 1
            0xFFC0_0BAD, // negative qNaN, payload
            0x7F80_0001, // sNaN, payload 1
            0xFF80_0002, // negative sNaN
            0x0000_0001, // smallest subnormal
            0x0080_0000, // smallest normal
            0x7F7F_FFFF, // FLT_MAX
            0xFF7F_FFFF, // -FLT_MAX
            0x3F80_0000, // 1.0
            0xBF80_0000, // -1.0
            0x0000_0002, // subnormal
        ];
        match self.below(10) {
            0 | 1 | 2 => f32::from_bits(SPECIALS[self.below(16) as usize]),
            3 => self.raw_f32(),
            4 => self.signed(1.0e30),
            5 => self.signed(1.0e-30),
            6 => (self.below(21) as f32) - 10.0, // small integers, forces ties
            _ => self.signed(10.0),
        }
    }

    pub fn nasty_v(&mut self) -> c2v {
        v(self.nasty_f32(), self.nasty_f32())
    }
    pub fn raw_v(&mut self) -> c2v {
        v(self.raw_f32(), self.raw_f32())
    }
    pub fn nasty_circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.nasty_v(),
            r: self.nasty_f32(),
        }
    }
    pub fn raw_circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.raw_v(),
            r: self.raw_f32(),
        }
    }
    pub fn nasty_aabb(&mut self) -> c2AABB {
        c2AABB {
            min: self.nasty_v(),
            max: self.nasty_v(),
        }
    }
    pub fn raw_aabb(&mut self) -> c2AABB {
        c2AABB {
            min: self.raw_v(),
            max: self.raw_v(),
        }
    }
}

/// The 16 interesting `f32` bit patterns, as a fixed list for exhaustive
/// small-cross-product tests.
pub const SPECIAL_F32: [f32; 14] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    3.0,
    -7.25,
];

/// NaN patterns worth injecting, including signalling ones.
pub const NAN_F32_BITS: [u32; 6] = [
    0x7FC0_0000, // default qNaN
    0xFFC0_0000, // -qNaN
    0x7FC0_1234, // qNaN with payload
    0x7F80_0001, // sNaN
    0xFF80_4321, // -sNaN with payload
    0x7FFF_FFFF, // qNaN, all-ones payload
];

pub fn nan(i: usize) -> f32 {
    f32::from_bits(NAN_F32_BITS[i % NAN_F32_BITS.len()])
}
