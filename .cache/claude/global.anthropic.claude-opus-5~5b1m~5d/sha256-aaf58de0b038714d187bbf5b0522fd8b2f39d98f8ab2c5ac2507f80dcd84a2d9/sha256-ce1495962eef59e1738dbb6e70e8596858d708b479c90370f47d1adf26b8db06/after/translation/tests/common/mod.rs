//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects through `libloading` and exposes one typed getter
//! per exported symbol, so every test calls the Rust code exactly the way an
//! external C consumer would (through the `#[no_mangle]` export wrappers).

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// POD types — layouts must match the C exactly.
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// Mirrors `struct { c2sv a, b, c, d; float div; int count; }`.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Raw bytes of any POD value — the ultimate "byte-identical" comparison.
pub fn raw<T: Copy>(v: &T) -> Vec<u8> {
    let p = v as *const T as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<T>()) }.to_vec()
}

pub fn f32_hex(v: f32) -> String {
    format!("{:#010x} ({})", v.to_bits(), v)
}

pub fn v_hex(v: &c2v) -> String {
    format!("({}, {})", f32_hex(v.x), f32_hex(v.y))
}

pub fn simplex_hex(s: &c2Simplex) -> String {
    let mut out = format!("count={} div={}", s.count, f32_hex(s.div));
    for (i, v) in s.verts.iter().enumerate() {
        out += &format!(
            "\n  v[{i}] sA={} sB={} p={} u={} iA={} iB={}",
            v_hex(&v.sA),
            v_hex(&v.sB),
            v_hex(&v.p),
            f32_hex(v.u),
            v.iA,
            v.iB
        );
    }
    out
}

pub fn proxy_hex(p: &c2Proxy) -> String {
    let mut out = format!("radius={} count={}", f32_hex(p.radius), p.count);
    for (i, v) in p.verts.iter().enumerate() {
        out += &format!("\n  verts[{i}]={}", v_hex(v));
    }
    out
}

pub fn cache_hex(c: &c2GJKCache) -> String {
    format!(
        "metric={} count={} iA={:?} iB={:?} div={}",
        f32_hex(c.metric),
        c.count,
        c.iA,
        c.iB,
        f32_hex(c.div)
    )
}

#[macro_export]
macro_rules! assert_bits_eq {
    ($c:expr, $r:expr, $($ctx:tt)+) => {{
        let cv = $c;
        let rv = $r;
        if $crate::common::raw(&cv) != $crate::common::raw(&rv) {
            panic!(
                "DIVERGENCE\n  context: {}\n  C   bytes: {:02x?}\n  Rust bytes: {:02x?}",
                format!($($ctx)+),
                $crate::common::raw(&cv),
                $crate::common::raw(&rv),
            );
        }
    }};
}

/// Bit-exact `f32` equality (so `NaN` payloads and `-0.0` are compared too).
#[macro_export]
macro_rules! assert_f32_bits_eq {
    ($c:expr, $r:expr, $($ctx:tt)+) => {{
        let cv: f32 = $c;
        let rv: f32 = $r;
        if cv.to_bits() != rv.to_bits() {
            panic!(
                "DIVERGENCE\n  context: {}\n  C   = {}\n  Rust= {}",
                format!($($ctx)+),
                $crate::common::f32_hex(cv),
                $crate::common::f32_hex(rv),
            );
        }
    }};
}

// ---------------------------------------------------------------------------
// Function-pointer typedefs, one per exported symbol
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnMulvs = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVf = unsafe extern "C" fn(c2v) -> f32;
pub type FnV1 = unsafe extern "C" fn(c2v) -> c2v;
pub type FnRotIdentity = unsafe extern "C" fn() -> c2r;
pub type FnxIdentity = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexf = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexv = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
pub type FnMulrv = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnMulxv = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnDiv = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnGJK = unsafe extern "C" fn(
    *const c_void,
    c_int,
    *const c2x,
    *const c_void,
    c_int,
    *const c2x,
    *mut c2v,
    *mut c2v,
    c_int,
    *mut c_int,
    *mut c2GJKCache,
) -> f32;
pub type FnAABBtoAABB = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
pub type FnAABBtoCapsule = unsafe extern "C" fn(c2AABB, c2Capsule) -> c_int;
pub type FnCapsuletoCapsule = unsafe extern "C" fn(c2Capsule, c2Capsule) -> c_int;
pub type FnCircletoCircle = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
pub type FnCircletoAABB = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
pub type FnCircletoCapsule = unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
pub type FnCapsule = unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int;

// ---------------------------------------------------------------------------
// The loaded library pair
// ---------------------------------------------------------------------------

/// One dynamically loaded implementation of the library.
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        Impl { name, lib }
    }

    pub fn sym<T: Copy>(&self, name: &str) -> T {
        let s: Symbol<T> = unsafe { self.lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{name}`: {e}", self.name));
        *s
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} (did you run cmake?): {e}",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration-test binary lives in <target>/<profile>/deps/, and cargo
    // puts the cdylib in <target>/<profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("…/<profile>/deps/<test>")
        .to_path_buf();
    let direct = profile_dir.join("libcapsule_lib.so");
    if direct.exists() {
        return direct;
    }
    // `cargo test` alone does NOT emit the cdylib artifact for a
    // `crate-type = ["cdylib"]` library (it only builds the lib as a unit-test
    // binary), so a bare `cargo test` on a clean tree would find nothing here.
    // Build it on demand into a SIDE target directory: cargo already holds a
    // lock on `target/`, so reusing it would dead-lock.
    build_cdylib(&profile_dir)
}

/// Builds the cdylib into `target/harness-cdylib/<profile>/` and returns its path.
fn build_cdylib(profile_dir: &Path) -> PathBuf {
    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let side = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("harness-cdylib");
    // `release` is the only non-`debug` profile this crate defines; anything
    // else (e.g. a custom profile dir) is built with the same flag mapping.
    let out = side.join(&profile).join("libcapsule_lib.so");
    if out.exists() {
        return out;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--lib")
        .arg("--target-dir")
        .arg(&side);
    if profile == "release" {
        cmd.arg("--release");
    }
    // Don't let the parent cargo invocation's env redirect the child.
    cmd.env_remove("CARGO_TARGET_DIR");
    cmd.env_remove("RUSTC_WRAPPER");
    let status = cmd.status().unwrap_or_else(|e| {
        panic!(
            "libcapsule_lib.so was not found in {} and `cargo build` could not be \
             spawned to produce it: {e}\n\
             Run `cargo build --release` (or set RUST_SO=/path/to/libcapsule_lib.so) first.",
            profile_dir.display()
        )
    });
    assert!(
        status.success(),
        "on-demand `cargo build --lib` for the cdylib failed ({status})"
    );
    assert!(
        out.exists(),
        "on-demand build succeeded but {} is still missing",
        out.display()
    );
    out
}

/// Loads both `.so`s.  Panics with a helpful message if either is missing.
pub fn load() -> Pair {
    let c_path = find_c_so();
    let r_path = find_rust_so();
    Pair {
        c: Impl::open("C", &c_path),
        rs: Impl::open("Rust", &r_path),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[lo, hi)`.
    pub fn uniform(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() as f32) / 4_294_967_296.0f32;
        lo + (hi - lo) * t
    }
    /// A "nice" coordinate in roughly `[-128, 128)`, quantised so exact ties,
    /// exact touching and integer geometry occur often.
    pub fn coord(&mut self) -> f32 {
        match self.below(8) {
            0 => (self.next_u32() % 65 ) as f32 - 32.0,          // small integer
            1 => ((self.next_u32() % 513) as f32 - 256.0) * 0.5, // half-integer
            2 => 0.0,
            3 => -0.0,
            _ => self.uniform(-128.0, 128.0),
        }
    }
    /// A radius: mostly small positive, sometimes 0 or negative.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => -0.0,
            2 => (self.next_u32() % 33) as f32,
            3 => -self.uniform(0.0, 20.0),
            _ => self.uniform(0.0, 40.0),
        }
    }
    /// A value drawn from the *whole* float space, weighted towards the
    /// interesting corners (specials, denormals, huge, exact powers of two).
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::from_bits(0x7fc0_0000),           // canonical qNaN
            6 => f32::from_bits(0xffc0_0000),           // negative qNaN
            7 => f32::from_bits(0x7f80_0001),           // sNaN
            8 => f32::from_bits(0xff80_0001),           // negative sNaN
            9 => f32::from_bits(0x7fab_cdef),           // qNaN, odd payload
            10 => f32::from_bits(0x0000_0001),          // smallest denormal
            11 => f32::from_bits(0x8000_0007),          // negative denormal
            12 => f32::MAX,
            13 => f32::MIN,
            14 => f32::from_bits(self.next_u32()),      // fully random bits
            _ => self.coord(),
        }
    }
    pub fn v(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn v_wild(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
    /// A rotation: mostly a real unit rotation, sometimes degenerate.
    pub fn rot(&mut self) -> c2r {
        match self.below(8) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 },
            2 => c2r { c: -1.0, s: 0.0 },
            3 => c2r {
                // deliberately NOT normalised — the C never normalises either
                c: self.uniform(-3.0, 3.0),
                s: self.uniform(-3.0, 3.0),
            },
            _ => {
                let a = self.uniform(-std::f32::consts::PI, std::f32::consts::PI);
                c2r { c: a.cos(), s: a.sin() }
            }
        }
    }
    pub fn x(&mut self) -> c2x {
        c2x {
            p: self.v(),
            r: self.rot(),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.v(),
            r: self.radius(),
        }
    }
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.v();
        let b = self.v();
        match self.below(8) {
            // inverted / degenerate shapes on purpose
            0 => c2AABB { min: a, max: a },
            1 => c2AABB { min: b, max: a },
            _ => c2AABB {
                min: c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            },
        }
    }
    pub fn capsule(&mut self) -> c2Capsule {
        let a = self.v();
        let b = if self.below(8) == 0 { a } else { self.v() };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
    pub fn sv(&mut self) -> c2sv {
        c2sv {
            sA: self.v(),
            sB: self.v(),
            p: self.v(),
            u: self.uniform(-4.0, 4.0),
            iA: self.below(8) as c_int,
            iB: self.below(8) as c_int,
        }
    }
    /// A fully randomised simplex with the given `count` and a `div` that is
    /// usually positive but occasionally `±0` / `NaN`.
    pub fn simplex(&mut self, count: c_int) -> c2Simplex {
        let mut s = c2Simplex::default();
        for i in 0..4 {
            s.verts[i] = self.sv();
        }
        s.div = match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::NAN,
            3 => 1.0,
            _ => self.uniform(0.05, 8.0),
        };
        s.count = count;
        s
    }
}

/// Small fixed menagerie used where an exhaustive special-value sweep is wanted.
pub const SPECIALS: &[u32] = &[
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // +denormal min
    0x8000_0001, // -denormal min
    0x007f_ffff, // largest denormal
    0x3f80_0000, // 1.0
    0xbf80_0000, // -1.0
    0x3380_0000, // FLT_EPSILON / 2
    0x3400_0000, // FLT_EPSILON
    0x7f7f_ffff, // FLT_MAX
    0xff7f_ffff, // -FLT_MAX
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x7fc0_0000, // qNaN
    0xffc0_0000, // -qNaN
    0x7f80_0001, // sNaN
    0xff80_0001, // -sNaN
    0x7fab_cdef, // qNaN odd payload
    0x4348_0000, // 200.0
    0xc348_0000, // -200.0
];

pub fn specials() -> impl Iterator<Item = f32> {
    SPECIALS.iter().copied().map(f32::from_bits)
}
