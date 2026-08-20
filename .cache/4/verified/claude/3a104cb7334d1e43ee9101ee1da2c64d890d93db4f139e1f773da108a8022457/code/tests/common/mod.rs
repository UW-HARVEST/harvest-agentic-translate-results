//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust crate is never
//! linked or called directly, so the `#[no_mangle] extern "C"` wrappers and the
//! `#[repr(C)]` struct layouts are part of what is under test.

#![allow(dead_code)]
#![allow(non_snake_case)]

use std::ffi::c_int;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::Library;

// ---------------------------------------------------------------------------
// Mirror types (independent re-declarations of the C structs)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

impl C2v {
    pub fn new(x: f32, y: f32) -> Self {
        C2v { x, y }
    }
    /// Raw bit pattern of both lanes — the comparison key used everywhere.
    pub fn bits(self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

// ---------------------------------------------------------------------------
// The loaded API surface: one struct per `.so`
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub c2V: extern "C" fn(f32, f32) -> C2v,
    pub c2Mulvs: extern "C" fn(C2v, f32) -> C2v,
    pub c2Maxv: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Minv: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Clampv: extern "C" fn(C2v, C2v, C2v) -> C2v,
    pub c2Sub: extern "C" fn(C2v, C2v) -> C2v,
    pub c2Dot: extern "C" fn(C2v, C2v) -> f32,
    pub c2CircletoCircle: extern "C" fn(C2Circle, C2Circle) -> c_int,
    pub c2CircletoAABB: extern "C" fn(C2Circle, C2Aabb) -> c_int,
    pub c2CircletoCapsule: extern "C" fn(C2Circle, C2Capsule) -> c_int,
    pub c2Collided: unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int,
    pub circle_collide: extern "C" fn(f32, f32, f32) -> c_int,
    // Keep the library mapped for the whole process lifetime. Declared last so
    // it is dropped after the function pointers (which is moot anyway: `Api`
    // instances are leaked into a `OnceLock`).
    _lib: Library,
}

/// The 12 symbols the C `.so` exports; every one must resolve in both objects.
pub const EXPECTED_SYMBOLS: [&str; 12] = [
    "c2V",
    "c2Mulvs",
    "c2Maxv",
    "c2Minv",
    "c2Clampv",
    "c2Sub",
    "c2Dot",
    "c2CircletoCircle",
    "c2CircletoAABB",
    "c2CircletoCapsule",
    "c2Collided",
    "circle_collide",
];

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: libloading::Symbol<$t> = unsafe { lib.get(concat!($n, "\0").as_bytes()) }
                    .unwrap_or_else(|e| {
                        panic!("{} ({}) is missing symbol {}: {e}", name, path.display(), $n)
                    });
                *s
            }};
        }
        Api {
            name,
            c2V: sym!(extern "C" fn(f32, f32) -> C2v, "c2V"),
            c2Mulvs: sym!(extern "C" fn(C2v, f32) -> C2v, "c2Mulvs"),
            c2Maxv: sym!(extern "C" fn(C2v, C2v) -> C2v, "c2Maxv"),
            c2Minv: sym!(extern "C" fn(C2v, C2v) -> C2v, "c2Minv"),
            c2Clampv: sym!(extern "C" fn(C2v, C2v, C2v) -> C2v, "c2Clampv"),
            c2Sub: sym!(extern "C" fn(C2v, C2v) -> C2v, "c2Sub"),
            c2Dot: sym!(extern "C" fn(C2v, C2v) -> f32, "c2Dot"),
            c2CircletoCircle: sym!(extern "C" fn(C2Circle, C2Circle) -> c_int, "c2CircletoCircle"),
            c2CircletoAABB: sym!(extern "C" fn(C2Circle, C2Aabb) -> c_int, "c2CircletoAABB"),
            c2CircletoCapsule: sym!(
                extern "C" fn(C2Circle, C2Capsule) -> c_int,
                "c2CircletoCapsule"
            ),
            c2Collided: sym!(
                unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int,
                "c2Collided"
            ),
            circle_collide: sym!(extern "C" fn(f32, f32, f32) -> c_int, "circle_collide"),
            path,
            _lib: lib,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the freshly built Rust `cdylib` (`target/<profile>/`).
fn rust_artifact_dir() -> PathBuf {
    // .../target/<profile>/deps/<test-binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("test binary should live in target/<profile>/deps/")
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    if let Some(p) = std::env::var_os("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

pub fn rust_lib_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    rust_artifact_dir().join("libcircle_collide_lib.so")
}

/// Newest mtime among the files matching `exts` under `dir` (recursively).
fn newest_mtime(dir: &std::path::Path, exts: &[&str]) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                // Never descend into build output.
                if p.file_name().is_some_and(|n| n == "build" || n == "target") {
                    continue;
                }
                stack.push(p);
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| exts.contains(&x))
            {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    if newest.is_none_or(|n| m > n) {
                        newest = Some(m);
                    }
                }
            }
        }
    }
    newest
}

/// `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` library (an
/// integration test cannot link it), so a stale `.so` could silently make these
/// differential tests pass. Refuse to run against stale artifacts.
#[track_caller]
fn assert_fresh(artifact: &std::path::Path, src_dir: &std::path::Path, exts: &[&str], how: &str) {
    let Ok(built) = std::fs::metadata(artifact).and_then(|m| m.modified()) else {
        return;
    };
    if let Some(src) = newest_mtime(src_dir, exts) {
        assert!(
            built >= src,
            "STALE ARTIFACT: {} is older than the newest source in {}.\nRebuild with:\n  {how}",
            artifact.display(),
            src_dir.display()
        );
    }
}

/// `(c, rust)` — both loaded once per test process.
pub fn libs() -> (&'static Api, &'static Api) {
    static C: OnceLock<Api> = OnceLock::new();
    static R: OnceLock<Api> = OnceLock::new();

    let c_path = c_lib_path();
    assert!(
        c_path.is_file(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        c_path.display()
    );
    let r_path = rust_lib_path();
    assert!(
        r_path.is_file(),
        "Rust cdylib not found at {} (run `cargo build` first)",
        r_path.display()
    );

    assert_fresh(
        &c_path,
        &manifest_dir().join("c_src"),
        &["c", "h"],
        "cd c_src/build && cmake --build .",
    );
    let profile_flag = if rust_artifact_dir().file_name().is_some_and(|n| n == "release") {
        " --release"
    } else {
        ""
    };
    assert_fresh(
        &r_path,
        &manifest_dir().join("src"),
        &["rs"],
        &format!("cargo build --no-default-features{profile_flag}"),
    );

    let c = C.get_or_init(|| Api::load("C", c_path));
    let r = R.get_or_init(|| Api::load("RUST", r_path));
    (c, r)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64 — no external crates, fixed seed)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C011_1DE;

pub struct Rng(u64);

impl Rng {
    pub fn new(stream: u64) -> Rng {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-mag, mag]`.
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }

    /// A float built from a fully random 32-bit pattern: hits NaNs (quiet and
    /// signalling, every payload), infinities, subnormals and huge magnitudes.
    pub fn bit_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Mixture generator: mostly "interesting" special values, otherwise a
    /// plain uniform value on the scale the library's fixed geometry uses.
    pub fn any_f32(&mut self) -> f32 {
        match self.below(10) {
            0 => SPECIALS[self.below(SPECIALS.len() as u32) as usize],
            1 => self.bit_f32(),
            2 => NANS[self.below(NANS.len() as u32) as usize],
            3 => self.sym(1.0e30),
            4 => self.sym(1.0e-30),
            _ => self.sym(200.0),
        }
    }

    /// Well-behaved finite value in `[-mag, mag]`, occasionally an exact
    /// integer / zero so exact-boundary cases (tangency) are reachable.
    pub fn tame_f32(&mut self, mag: f32) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -0.0,
            2 => (self.sym(mag)).trunc(),
            _ => self.sym(mag),
        }
    }

    pub fn tame_v(&mut self, mag: f32) -> C2v {
        C2v::new(self.tame_f32(mag), self.tame_f32(mag))
    }
    pub fn any_v(&mut self) -> C2v {
        C2v::new(self.any_f32(), self.any_f32())
    }
    pub fn bit_v(&mut self) -> C2v {
        C2v::new(self.bit_f32(), self.bit_f32())
    }
}

/// Interesting non-NaN float values.
pub const SPECIALS: [f32; 20] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.0e-45,  // smallest positive subnormal
    -1.0e-45, // smallest negative subnormal
    f32::EPSILON,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    16777216.0, // 2^24, first f32 with a 1-ulp gap > 1
    -16777216.0,
    1.0e-30,
    1.0e30,
    3.4028235e38,
];

/// Interesting NaN bit patterns: quiet/signalling, both signs, several payloads
/// (including the all-ones payload and payload==1, the classic SNaN).
pub const NAN_BITS: [u32; 10] = [
    0x7FC0_0000, // +qNaN, payload 0
    0xFFC0_0000, // -qNaN, payload 0
    0x7FC0_1234, // +qNaN, payload 0x1234
    0xFFC5_5555, // -qNaN
    0x7FFF_FFFF, // +qNaN, all-ones payload
    0xFFFF_FFFF, // -qNaN, all-ones payload
    0x7F80_0001, // +sNaN, payload 1
    0xFF80_0001, // -sNaN, payload 1
    0x7FBF_FFFF, // +sNaN, max payload
    0xFFAA_AAAA, // -sNaN
];

pub const NANS: [f32; 10] = [
    f32::from_bits(NAN_BITS[0]),
    f32::from_bits(NAN_BITS[1]),
    f32::from_bits(NAN_BITS[2]),
    f32::from_bits(NAN_BITS[3]),
    f32::from_bits(NAN_BITS[4]),
    f32::from_bits(NAN_BITS[5]),
    f32::from_bits(NAN_BITS[6]),
    f32::from_bits(NAN_BITS[7]),
    f32::from_bits(NAN_BITS[8]),
    f32::from_bits(NAN_BITS[9]),
];

// ---------------------------------------------------------------------------
// Bit-exact assertions
// ---------------------------------------------------------------------------

pub fn fmt_f32(v: f32) -> String {
    format!("{:?}[{:#010x}]", v, v.to_bits())
}

pub fn fmt_v(v: C2v) -> String {
    format!("({}, {})", fmt_f32(v.x), fmt_f32(v.y))
}

#[track_caller]
pub fn assert_f32_bits(c: f32, r: f32, ctx: &str) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "float mismatch: C={} RUST={}  [{}]",
        fmt_f32(c),
        fmt_f32(r),
        ctx
    );
}

#[track_caller]
pub fn assert_v_bits(c: C2v, r: C2v, ctx: &str) {
    assert_eq!(
        c.bits(),
        r.bits(),
        "c2v mismatch: C={} RUST={}  [{}]",
        fmt_v(c),
        fmt_v(r),
        ctx
    );
}

#[track_caller]
pub fn assert_int(c: c_int, r: c_int, ctx: &str) {
    assert_eq!(c, r, "int mismatch: C={c} RUST={r}  [{}]", ctx);
}

// ---------------------------------------------------------------------------
// Coverage helper: makes "did we actually reach every branch?" assertable.
// ---------------------------------------------------------------------------

pub struct Cover {
    label: &'static str,
    hits: Vec<(&'static str, u32)>,
}

impl Cover {
    pub fn new(label: &'static str, buckets: &[&'static str]) -> Cover {
        Cover {
            label,
            hits: buckets.iter().map(|b| (*b, 0)).collect(),
        }
    }
    pub fn hit(&mut self, bucket: &str) {
        for e in self.hits.iter_mut() {
            if e.0 == bucket {
                e.1 += 1;
                return;
            }
        }
        panic!("{}: unknown coverage bucket {bucket:?}", self.label);
    }
    #[track_caller]
    pub fn require_all(&self, min: u32) {
        let missing: Vec<_> = self.hits.iter().filter(|(_, n)| *n < min).collect();
        assert!(
            missing.is_empty(),
            "{}: buckets under-covered (need >= {min} each): {:?}; full histogram {:?}",
            self.label,
            missing,
            self.hits
        );
    }
}
