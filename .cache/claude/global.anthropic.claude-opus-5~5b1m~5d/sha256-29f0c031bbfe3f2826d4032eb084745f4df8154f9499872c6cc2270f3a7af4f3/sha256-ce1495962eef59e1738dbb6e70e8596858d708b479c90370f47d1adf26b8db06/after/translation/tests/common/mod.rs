//! Differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes their exported
//! symbols behind an identical `Api` struct, so every test calls the C library
//! and the Rust library through the *same* FFI path. Nothing in the crate is
//! ever called directly — this exercises the `#[no_mangle]` wrappers and the
//! real ABI, exactly as an external consumer would.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::{c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Types mirroring the C ones
// ---------------------------------------------------------------------------

pub type C2_TYPE = c_uint;
pub const C2_TYPE_CIRCLE: C2_TYPE = 0;
pub const C2_TYPE_AABB: C2_TYPE = 1;

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
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

pub fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}
pub fn circle(x: f32, y: f32, r: f32) -> C2Circle {
    C2Circle { p: v(x, y), r }
}
pub fn aabb(minx: f32, miny: f32, maxx: f32, maxy: f32) -> C2AABB {
    C2AABB {
        min: v(minx, miny),
        max: v(maxx, maxy),
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit pattern of an `f32`; the tests compare these (never `==` on floats) so
/// that `NaN` payloads and the sign of zero are part of the contract.
pub fn fb(x: f32) -> u32 {
    x.to_bits()
}
pub fn vb(a: C2v) -> (u32, u32) {
    (a.x.to_bits(), a.y.to_bits())
}
pub fn show_v(a: C2v) -> String {
    format!("{{x: {} (0x{:08x}), y: {} (0x{:08x})}}", a.x, a.x.to_bits(), a.y, a.y.to_bits())
}
pub fn show_c(c: C2Circle) -> String {
    format!("Circle{{p: {}, r: {} (0x{:08x})}}", show_v(c.p), c.r, c.r.to_bits())
}
pub fn show_b(b: C2AABB) -> String {
    format!("AABB{{min: {}, max: {}}}", show_v(b.min), show_v(b.max))
}

// ---------------------------------------------------------------------------
// Loading the two libraries
// ---------------------------------------------------------------------------

type FnV = unsafe extern "C" fn(f32, f32) -> C2v;
type FnVV = unsafe extern "C" fn(C2v, C2v) -> C2v;
type FnClamp = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type FnDot = unsafe extern "C" fn(C2v, C2v) -> f32;
type FnCC = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type FnCA = unsafe extern "C" fn(C2Circle, C2AABB) -> c_int;
type FnAA = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
type FnCollided = unsafe extern "C" fn(*const c_void, C2_TYPE, *const c_void, C2_TYPE) -> c_int;

/// All ten exported symbols of one library.
pub struct Api {
    pub which: &'static str,
    pub path: PathBuf,
    pub c2V: FnV,
    pub c2Maxv: FnVV,
    pub c2Minv: FnVV,
    pub c2Clampv: FnClamp,
    pub c2Sub: FnVV,
    pub c2Dot: FnDot,
    pub c2CircletoCircle: FnCC,
    pub c2CircletoAABB: FnCA,
    pub c2AABBtoAABB: FnAA,
    pub collided: FnCollided,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = .../translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// The C `.so`. Its file name is derived from the parent directory name by
/// `CMakeLists.txt`, so glob instead of hard-coding it.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust `cdylib`, taken from the same profile directory as the running test
/// binary (`target/<profile>/deps/<test>` ⇒ `target/<profile>/`), so
/// `cargo test` checks the debug artifact and `cargo test --release` the
/// release one.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| if p.ends_with("deps") { p.parent() } else { Some(p) })
        .expect("profile dir")
        .to_path_buf();
    let p = profile_dir.join("libcollided_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} — run `cargo build` for this profile first",
        p.display()
    );
    assert_not_stale(&p);
    p
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library (no
/// test target links it), so without this guard the suite would happily test a
/// stale `.so` and report a false pass after every edit to `src/`. Compare the
/// artifact's mtime against the sources and fail loudly instead.
fn assert_not_stale(so: &std::path::Path) {
    if std::env::var_os("ALLOW_STALE_SO").is_some() {
        return;
    }
    let so_mtime = std::fs::metadata(so).and_then(|m| m.modified()).expect("so mtime");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![manifest.join("src")];
    stack.push(manifest.join("Cargo.toml"));
    while let Some(path) = stack.pop() {
        let Ok(md) = std::fs::metadata(&path) else { continue };
        if md.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&path) {
                stack.extend(rd.filter_map(|e| e.ok().map(|e| e.path())));
            }
        } else if md.is_file() {
            let t = md.modified().expect("src mtime");
            if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                newest = Some((path, t));
            }
        }
    }
    if let Some((newest_path, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE ARTIFACT: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib-only library — run\n\
             `cargo build{}` first (or set ALLOW_STALE_SO=1 to override).",
            so.display(),
            newest_path.display(),
            if so.to_string_lossy().contains("/release/") { " --release" } else { "" }
        );
    }
}

unsafe fn load(which: &'static str, path: PathBuf) -> Api {
    let lib = libloading::Library::new(&path)
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    // Leak so the symbol pointers stay valid for the whole test process.
    let lib: &'static libloading::Library = Box::leak(Box::new(lib));
    macro_rules! sym {
        ($t:ty, $name:literal) => {
            *lib.get::<$t>(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("{} lacks symbol {}: {e}", which, $name))
        };
    }
    Api {
        which,
        path,
        c2V: sym!(FnV, "c2V"),
        c2Maxv: sym!(FnVV, "c2Maxv"),
        c2Minv: sym!(FnVV, "c2Minv"),
        c2Clampv: sym!(FnClamp, "c2Clampv"),
        c2Sub: sym!(FnVV, "c2Sub"),
        c2Dot: sym!(FnDot, "c2Dot"),
        c2CircletoCircle: sym!(FnCC, "c2CircletoCircle"),
        c2CircletoAABB: sym!(FnCA, "c2CircletoAABB"),
        c2AABBtoAABB: sym!(FnAA, "c2AABBtoAABB"),
        collided: sym!(FnCollided, "collided"),
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static R_API: OnceLock<Api> = OnceLock::new();

pub fn c() -> &'static Api {
    C_API.get_or_init(|| unsafe { load("C", c_so_path()) })
}
pub fn r() -> &'static Api {
    R_API.get_or_init(|| unsafe { load("Rust", rust_so_path()) })
}

/// `(c(), r())`, for the usual `let (c, r) = both();`
pub fn both() -> (&'static Api, &'static Api) {
    (c(), r())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed ⇒ reproducible) + interesting-float generator
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64-seeded; never zero
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[-range, range]`, quantised to 1/16 so that exact ties,
    /// exact touches and equal coordinates occur often.
    pub fn coord(&mut self, range: f32) -> f32 {
        let steps = (range * 16.0) as i32;
        let k = (self.next_u32() % ((2 * steps + 1) as u32)) as i32 - steps;
        k as f32 / 16.0
    }
    pub fn small(&mut self) -> f32 {
        self.coord(8.0)
    }
    /// A float drawn from the "interesting" classes the C branches on:
    /// small quantised values, ±0, subnormals, huge finite, ±inf, qNaN/sNaN
    /// with distinct payloads, and fully random bit patterns.
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::from_bits(0x7fc0_0000),            // default qNaN
            5 => f32::from_bits(0x7f80_0001 | (self.below(0x40_0000)) ), // sNaN, random payload
            6 => f32::from_bits(0xffc0_0000 | (self.below(0x40_0000))),  // negative qNaN
            7 => f32::from_bits(0x0000_0001 + self.below(16)),           // subnormal
            8 => f32::from_bits(0x8000_0001 + self.below(16)),           // -subnormal
            9 => f32::MAX,
            10 => f32::MIN,
            11 => f32::MIN_POSITIVE,
            12 => f32::from_bits(self.next_u32()), // anything at all
            _ => self.coord(64.0),
        }
    }
    pub fn v_small(&mut self) -> C2v {
        v(self.small(), self.small())
    }
    pub fn v_wild(&mut self) -> C2v {
        v(self.wild(), self.wild())
    }
    pub fn c_small(&mut self) -> C2Circle {
        C2Circle {
            p: self.v_small(),
            r: self.coord(6.0),
        }
    }
    pub fn c_wild(&mut self) -> C2Circle {
        C2Circle {
            p: self.v_wild(),
            r: self.wild(),
        }
    }
    /// Well-formed box (`min <= max` on both axes).
    pub fn b_small(&mut self) -> C2AABB {
        let (x0, x1) = (self.small(), self.small());
        let (y0, y1) = (self.small(), self.small());
        aabb(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1))
    }
    /// Arbitrary box: may be inverted, degenerate or contain non-finite edges.
    pub fn b_wild(&mut self) -> C2AABB {
        C2AABB {
            min: self.v_wild(),
            max: self.v_wild(),
        }
    }
}

/// Iterations per randomized row.
pub const ITERS: usize = 4000;

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_int(row: &str, ctx: &str, cv: c_int, rv: c_int) {
    assert_eq!(
        cv, rv,
        "[{row}] int mismatch: C returned {cv}, Rust returned {rv}\n  inputs: {ctx}"
    );
}

#[track_caller]
pub fn eq_f32(row: &str, ctx: &str, cv: f32, rv: f32) {
    assert_eq!(
        fb(cv),
        fb(rv),
        "[{row}] f32 mismatch: C returned {cv} (0x{:08x}), Rust returned {rv} (0x{:08x})\n  inputs: {ctx}",
        fb(cv),
        fb(rv)
    );
}

#[track_caller]
pub fn eq_v(row: &str, ctx: &str, cv: C2v, rv: C2v) {
    assert_eq!(
        vb(cv),
        vb(rv),
        "[{row}] c2v mismatch: C returned {}, Rust returned {}\n  inputs: {ctx}",
        show_v(cv),
        show_v(rv)
    );
}

/// The `int` predicates must be exactly the 0/1 that C's relational operators
/// produce — a "truthy" 2 would be a real ABI difference.
#[track_caller]
pub fn assert_bool_like(row: &str, ctx: &str, value: c_int) {
    assert!(
        value == 0 || value == 1,
        "[{row}] predicate returned {value}, expected 0 or 1\n  inputs: {ctx}"
    );
}
