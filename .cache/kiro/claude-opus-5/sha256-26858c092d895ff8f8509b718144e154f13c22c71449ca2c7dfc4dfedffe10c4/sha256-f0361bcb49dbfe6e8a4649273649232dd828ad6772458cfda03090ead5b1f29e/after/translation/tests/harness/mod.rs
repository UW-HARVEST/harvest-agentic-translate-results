//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes them behind one struct so every test can call the
//! two implementations with identical arguments.
//!
//! The Rust implementation is **never** called directly — only through the
//! `.so`'s exported symbols, so the `#[no_mangle]` / `extern "C"` wrappers and
//! the struct-passing ABI are part of what is under test.

#![allow(non_snake_case, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// C types, mirrored exactly (see c_src/src/lib.c and c_src/include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

type FnC2V = unsafe extern "C" fn(f32, f32) -> C2v;
type FnVv = unsafe extern "C" fn(C2v, C2v) -> C2v;
type FnVvv = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type FnDot = unsafe extern "C" fn(C2v, C2v) -> f32;
type FnCc = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type FnCa = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type FnAa = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;

/// One loaded shared object with all ten exports resolved.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub c2V: FnC2V,
    pub c2Maxv: FnVv,
    pub c2Minv: FnVv,
    pub c2Clampv: FnVvv,
    pub c2Sub: FnVv,
    pub c2Dot: FnDot,
    pub c2CircletoCircle: FnCc,
    pub c2CircletoAABB: FnCa,
    pub c2AABBtoAABB: FnAa,
    pub collided: FnCollided,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: Symbol<$t> = lib
                        .get($s)
                        .unwrap_or_else(|e| panic!("{name}: missing symbol {:?}: {e}", String::from_utf8_lossy($s)));
                    *s
                }};
            }
            let me = Impl {
                name,
                c2V: sym!(FnC2V, b"c2V\0"),
                c2Maxv: sym!(FnVv, b"c2Maxv\0"),
                c2Minv: sym!(FnVv, b"c2Minv\0"),
                c2Clampv: sym!(FnVvv, b"c2Clampv\0"),
                c2Sub: sym!(FnVv, b"c2Sub\0"),
                c2Dot: sym!(FnDot, b"c2Dot\0"),
                c2CircletoCircle: sym!(FnCc, b"c2CircletoCircle\0"),
                c2CircletoAABB: sym!(FnCa, b"c2CircletoAABB\0"),
                c2AABBtoAABB: sym!(FnAa, b"c2AABBtoAABB\0"),
                collided: sym!(FnCollided, b"collided\0"),
                _lib: lib,
            };
            me
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so` — the project name is derived by CMake from
/// the parent directory name, so the file is found by scanning for any `.so`.
fn c_so_path() -> PathBuf {
    let dir = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        dir.display()
    );
    found.pop().unwrap()
}

/// `target/<profile>/libcollided_lib.so`, located relative to the running test
/// binary (`target/<profile>/deps/<test>`), so it always matches the profile
/// and feature set the tests were compiled with.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let direct = profile_dir.join("libcollided_lib.so");
    if direct.exists() {
        assert_fresh(&direct);
        return direct;
    }
    // Fallback: scan target/{debug,release}.
    for p in ["debug", "release"] {
        let cand = manifest_dir().join("target").join(p).join("libcollided_lib.so");
        if cand.exists() {
            assert_fresh(&cand);
            return cand;
        }
    }
    panic!(
        "libcollided_lib.so not found next to {} — run `cargo build` first",
        exe.display()
    );
}

/// `cargo test` does **not** rebuild the `cdylib` target (integration tests do
/// not link it), so a plain `cargo test` after editing `src/lib.rs` would
/// happily differential-test a *stale* `.so` and report success. Refuse to run
/// unless the `.so` is newer than every crate input.
fn assert_fresh(so: &Path) {
    let so_time = std::fs::metadata(so).and_then(|m| m.modified()).expect("so mtime");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut inputs = vec![manifest_dir().join("Cargo.toml")];
    let src = manifest_dir().join("src");
    let mut stack = vec![src];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    inputs.push(p);
                }
            }
        }
    }
    for p in inputs {
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                newest = Some((p, t));
            }
        }
    }
    if let Some((path, t)) = newest {
        assert!(
            so_time >= t,
            "STALE Rust .so: {} is older than {}.\n`cargo test` does not rebuild the cdylib — run \
             `cargo build{}` (or ./run-tests.sh) first, otherwise the differential tests would \
             compare against an out-of-date library.",
            so.display(),
            path.display(),
            if so.to_string_lossy().contains("/release/") { " --release" } else { "" }
        );
    }
}

/// The (C, Rust) pair, loaded once per test process.
pub fn pair() -> &'static (Impl, Impl) {
    use std::sync::OnceLock;
    static PAIR: OnceLock<(Impl, Impl)> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c = Impl::load("C", &c_so_path());
        let r = Impl::load("Rust", &rust_so_path());
        (c, r)
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn bits_v(v: C2v) -> (u32, u32) {
    (v.x.to_bits(), v.y.to_bits())
}

#[track_caller]
pub fn assert_v_eq(func: &str, args: &str, c: C2v, r: C2v) {
    if bits_v(c) != bits_v(r) {
        panic!(
            "{func} diverged for {args}\n  C   : x=0x{:08x} ({}) y=0x{:08x} ({})\n  Rust: \
             x=0x{:08x} ({}) y=0x{:08x} ({})",
            c.x.to_bits(),
            c.x,
            c.y.to_bits(),
            c.y,
            r.x.to_bits(),
            r.x,
            r.y.to_bits(),
            r.y
        );
    }
}

#[track_caller]
pub fn assert_f_eq(func: &str, args: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{func} diverged for {args}\n  C   : 0x{:08x} ({})\n  Rust: 0x{:08x} ({})",
            c.to_bits(),
            c,
            r.to_bits(),
            r
        );
    }
}

#[track_caller]
pub fn assert_i_eq(func: &str, args: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "{func} diverged for {args}: C={c} Rust={r}");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Any 32-bit pattern: reaches every float class including NaN payloads.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "reasonable" finite value in roughly [-1000, 1000].
    pub fn small_f32(&mut self) -> f32 {
        let n = (self.next_u32() % 2_000_001) as f32 / 1000.0 - 1000.0;
        n
    }
    /// A finite value spanning the whole exponent range (no inf/NaN).
    pub fn finite_f32(&mut self) -> f32 {
        loop {
            let v = f32::from_bits(self.next_u32());
            if v.is_finite() {
                return v;
            }
        }
    }
    /// Draws from a hand-built pool of interesting values, mixed with random
    /// finite and fully-random patterns.
    pub fn interesting_f32(&mut self) -> f32 {
        match self.below(10) {
            0..=3 => SPECIAL[(self.next_u32() as usize) % SPECIAL.len()],
            4..=6 => self.small_f32(),
            7..=8 => self.finite_f32(),
            _ => self.any_f32(),
        }
    }
    pub fn v_small(&mut self) -> C2v {
        C2v { x: self.small_f32(), y: self.small_f32() }
    }
    pub fn v_interesting(&mut self) -> C2v {
        C2v { x: self.interesting_f32(), y: self.interesting_f32() }
    }
    pub fn v_any(&mut self) -> C2v {
        C2v { x: self.any_f32(), y: self.any_f32() }
    }
    pub fn circle_interesting(&mut self) -> C2Circle {
        C2Circle { p: self.v_interesting(), r: self.interesting_f32() }
    }
    pub fn aabb_interesting(&mut self) -> C2Aabb {
        C2Aabb { min: self.v_interesting(), max: self.v_interesting() }
    }
}

/// Interesting float values: signed zeros, subnormals, boundaries, infinities,
/// and several distinct NaN payloads (both quiet and signalling, both signs).
pub const SPECIAL: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    -3.0,
    f32::MIN_POSITIVE,                 // smallest normal
    -f32::MIN_POSITIVE,
    f32::from_bits(0x0000_0001), // smallest subnormal
    f32::from_bits(0x8000_0001),
    f32::from_bits(0x007F_FFFF), // largest subnormal
    f32::MAX,
    f32::MIN,
    f32::EPSILON,
    16_777_216.0,  // 2^24, first integer with a gap
    16_777_217.0,
    1e30,
    -1e30,
    1e-30,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::from_bits(0x7FC0_0000), // +QNaN, empty payload
    f32::from_bits(0xFFC0_0000), // -QNaN indefinite
    f32::from_bits(0x7FC0_1234), // +QNaN, payload
    f32::from_bits(0xFFDE_AD00), // -QNaN, payload
    f32::from_bits(0x7F80_0001), // +SNaN
    f32::from_bits(0xFF80_0001), // -SNaN
    f32::from_bits(0x7FBF_FFFF), // +SNaN, max payload
];

/// The 8 raw bytes of a `c2v`.
pub fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}
pub fn circle(x: f32, y: f32, r: f32) -> C2Circle {
    C2Circle { p: v(x, y), r }
}
pub fn aabb(minx: f32, miny: f32, maxx: f32, maxy: f32) -> C2Aabb {
    C2Aabb { min: v(minx, miny), max: v(maxx, maxy) }
}
