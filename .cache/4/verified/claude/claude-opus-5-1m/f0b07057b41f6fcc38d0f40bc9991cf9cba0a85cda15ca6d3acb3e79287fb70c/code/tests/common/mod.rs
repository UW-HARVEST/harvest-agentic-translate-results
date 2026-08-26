//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C one built by cmake and the Rust cdylib)
//! with `libloading` and calls every function through its exported symbol, so
//! the `#[unsafe(no_mangle)] extern "C"` wrappers and the struct-by-value ABI
//! are part of what is under test. No Rust function is ever called directly.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C types, mirrored exactly (verified padding-free, so raw byte compares work)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct V {
    pub x: f32,
    pub y: f32,
}

impl V {
    pub const fn new(x: f32, y: f32) -> Self {
        V { x, y }
    }
    pub fn bits(&self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct R {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct X {
    pub p: V,
    pub r: R,
}

impl X {
    pub const IDENTITY: X = X {
        p: V { x: 0.0, y: 0.0 },
        r: R { c: 1.0, s: 0.0 },
    };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Circle {
    pub p: V,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct AABB {
    pub min: V,
    pub max: V,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Capsule {
    pub a: V,
    pub b: V,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [V; 8],
}

impl Default for Proxy {
    fn default() -> Self {
        Proxy {
            radius: 0.0,
            count: 0,
            verts: [V::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Sv {
    pub sA: V,
    pub sB: V,
    pub p: V,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// Mirrors `c2Simplex { c2sv a, b, c, d; float div; int count; }`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Simplex {
    pub verts: [Sv; 4],
    pub div: f32,
    pub count: i32,
}

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

// ---------------------------------------------------------------------------
// Exported-function signature aliases
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> V;
pub type FnVsV = unsafe extern "C" fn(V, f32) -> V;
pub type FnVVV = unsafe extern "C" fn(V, V) -> V;
pub type FnVVVV = unsafe extern "C" fn(V, V, V) -> V;
pub type FnVV = unsafe extern "C" fn(V) -> V;
pub type FnVVf = unsafe extern "C" fn(V, V) -> f32;
pub type FnVf = unsafe extern "C" fn(V) -> f32;
pub type FnR = unsafe extern "C" fn() -> R;
pub type FnX = unsafe extern "C" fn() -> X;
pub type FnRVV = unsafe extern "C" fn(R, V) -> V;
pub type FnXVV = unsafe extern "C" fn(X, V) -> V;
pub type FnBBVerts = unsafe extern "C" fn(*mut V, *mut AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut Simplex) -> f32;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut Simplex);
pub type FnSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
pub type FnSupport = unsafe extern "C" fn(*const V, i32, V) -> i32;
pub type FnWitness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);
pub type FnGJK = unsafe extern "C" fn(
    *const std::ffi::c_void, // A
    i32,                     // typeA
    *const X,                // ax
    *const std::ffi::c_void, // B
    i32,                     // typeB
    *const X,                // bx
    *mut V,                  // outA
    *mut V,                  // outB
    i32,                     // use_radius
    *mut i32,                // iterations
    *mut GJKCache,           // cache
) -> f32;
pub type FnGjkWrapper = unsafe extern "C" fn(
    std::ffi::c_char,
    *mut V,
    *mut V,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
);

// ---------------------------------------------------------------------------
// Loading both libraries
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

impl Pair {
    /// `(c_symbol, rust_symbol)` for the same exported name.
    pub fn get<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cs: Symbol<T> = unsafe { self.c.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("C .so is missing symbol `{name}`: {e}"));
        let rs: Symbol<T> = unsafe { self.r.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("Rust .so is missing symbol `{name}`: {e}"));
        (cs, rs)
    }
}

static LIBS: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    LIBS.get_or_init(|| {
        let (c_so, r_so) = ensure_built();
        let c = unsafe { Library::new(&c_so) }
            .unwrap_or_else(|e| panic!("cannot dlopen C .so {}: {e}", c_so.display()));
        let r = unsafe { Library::new(&r_so) }
            .unwrap_or_else(|e| panic!("cannot dlopen Rust .so {}: {e}", r_so.display()));
        Pair { c, r }
    })
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Make sure both `.so`s exist, building them if necessary.
///
/// The nested `cargo build` deliberately uses a SEPARATE target directory so it
/// can never contend on the lock held by the outer `cargo test`.
fn ensure_built() -> (PathBuf, PathBuf) {
    let root = manifest_dir();

    let c_so = match std::env::var_os("GJK_C_SO") {
        Some(p) => PathBuf::from(p),
        None => {
            let p = root.join("c_src/build/libtranslated_rust.so");
            if !p.exists() {
                build_c(&root);
            }
            p
        }
    };
    assert!(c_so.exists(), "C .so not found at {}", c_so.display());

    let r_so = match std::env::var_os("GJK_RUST_SO") {
        Some(p) => PathBuf::from(p),
        None => {
            let alt_target = root.join("target/difftest");
            let p = alt_target.join("release/libgjk_lib.so");
            build_rust(&root, &alt_target);
            if !p.exists() {
                // Fall back to whatever a plain `cargo build` produced.
                for cand in ["target/release/libgjk_lib.so", "target/debug/libgjk_lib.so"] {
                    let c = root.join(cand);
                    if c.exists() {
                        return (c_so, c);
                    }
                }
            }
            p
        }
    };
    assert!(
        r_so.exists(),
        "Rust .so not found at {} — run `cargo build --release` first",
        r_so.display()
    );
    (c_so, r_so)
}

fn build_c(root: &Path) {
    let build_dir = root.join("c_src/build");
    let _ = std::fs::create_dir_all(&build_dir);
    let cfg = Command::new("cmake")
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&build_dir)
        .output();
    if let Err(e) = cfg {
        panic!("failed to run cmake: {e}");
    }
    let _ = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build_dir)
        .output();
}

fn build_rust(root: &Path, alt_target: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .args(["build", "--release", "--lib"])
        .env("CARGO_TARGET_DIR", alt_target)
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .current_dir(root)
        .output();
    match out {
        Ok(o) if !o.status.success() => {
            eprintln!(
                "nested `cargo build --release` failed:\n{}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
        Err(e) => eprintln!("could not spawn cargo: {e}"),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

/// Float bit patterns that historically break translations.
pub const EDGE_F32: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::EPSILON,
    -f32::EPSILON,
    1.1920929e-7,  // FLT_EPSILON, as spelled in the C source
    -1.1920929e-7,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1e-40, // subnormal
    -1e-40,
    f32::MAX,
    f32::MIN,
    3.4028235e38,
    1e30,
    -1e30,
    1e8,
    -1e8,
    1.0e8,
    -1.0e8,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    2.0,
    -2.0,
    100.0,
    -100.0,
];

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

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * t
    }

    /// A "geometry-plausible" coordinate.
    pub fn coord(&mut self) -> f32 {
        self.range(-100.0, 100.0)
    }

    /// Coordinate snapped to a coarse grid — maximises support-function ties
    /// and exact-touching configurations.
    pub fn grid(&mut self) -> f32 {
        (self.below(21) as f32) - 10.0
    }

    /// Any bit pattern at all: NaN payloads, Inf, subnormals, huge values.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Mostly plausible, sometimes adversarial. This is the default generator
    /// for the randomised sweeps.
    pub fn mixed(&mut self) -> f32 {
        match self.below(100) {
            0..=79 => self.coord(),
            80..=88 => {
                let i = self.below(EDGE_F32.len() as u32) as usize;
                EDGE_F32[i]
            }
            89..=94 => self.grid(),
            _ => self.any_f32(),
        }
    }

    pub fn v_mixed(&mut self) -> V {
        V::new(self.mixed(), self.mixed())
    }
    pub fn v_coord(&mut self) -> V {
        V::new(self.coord(), self.coord())
    }
    pub fn v_grid(&mut self) -> V {
        V::new(self.grid(), self.grid())
    }
    pub fn v_any(&mut self) -> V {
        V::new(self.any_f32(), self.any_f32())
    }

    /// A radius: usually small and non-negative, sometimes 0 / negative / huge.
    pub fn radius(&mut self) -> f32 {
        match self.below(100) {
            0..=69 => self.range(0.0, 10.0),
            70..=79 => 0.0,
            80..=87 => self.range(-10.0, 0.0),
            88..=93 => self.range(0.0, 1e6),
            _ => {
                let i = self.below(EDGE_F32.len() as u32) as usize;
                EDGE_F32[i]
            }
        }
    }

    /// Unit rotation, plus non-unit and degenerate `c2r` values.
    pub fn rot(&mut self) -> R {
        match self.below(100) {
            0..=59 => {
                let a = self.range(-7.0, 7.0);
                R { c: a.cos(), s: a.sin() }
            }
            60..=74 => R { c: 1.0, s: 0.0 },
            75..=89 => R {
                c: self.range(-2.0, 2.0),
                s: self.range(-2.0, 2.0),
            },
            _ => R {
                c: self.mixed(),
                s: self.mixed(),
            },
        }
    }

    pub fn xform(&mut self, mode: u32) -> X {
        match mode {
            0 => X::IDENTITY,
            1 => X {
                p: self.v_coord(),
                r: R { c: 1.0, s: 0.0 },
            },
            2 => X {
                p: V::new(0.0, 0.0),
                r: self.rot(),
            },
            _ => X {
                p: self.v_coord(),
                r: self.rot(),
            },
        }
    }

    pub fn simplex(&mut self, count: i32, vgen: fn(&mut Rng) -> V) -> Simplex {
        let mut s = Simplex::default();
        for i in 0..4 {
            s.verts[i].sA = vgen(self);
            s.verts[i].sB = vgen(self);
            s.verts[i].p = vgen(self);
            s.verts[i].u = self.mixed();
            s.verts[i].iA = self.below(8) as i32;
            s.verts[i].iB = self.below(8) as i32;
        }
        s.div = match self.below(10) {
            0 => 0.0,
            1 => self.mixed(),
            _ => self.range(0.01, 20.0),
        };
        s.count = count;
        s
    }
}

// ---------------------------------------------------------------------------
// Byte-exact comparison helpers
// ---------------------------------------------------------------------------

pub fn raw<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

/// Byte-for-byte struct comparison (all mirrored structs are padding-free).
#[track_caller]
pub fn same<T>(what: &str, ctx: &str, c: &T, r: &T) {
    let (a, b) = (raw(c), raw(r));
    if a != b {
        panic!(
            "{what} MISMATCH\n  ctx : {ctx}\n  C   : {a:02x?}\n  Rust: {b:02x?}",
        );
    }
}

/// Bit-exact float comparison: NaN payload and the sign of zero both matter.
#[track_caller]
pub fn same_f32(what: &str, ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{what} MISMATCH\n  ctx : {ctx}\n  C   : {c:?} (bits {:#010x})\n  Rust: {r:?} (bits {:#010x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn same_v(what: &str, ctx: &str, c: V, r: V) {
    if c.bits() != r.bits() {
        panic!(
            "{what} MISMATCH\n  ctx : {ctx}\n  C   : ({:?},{:?}) bits {:#010x},{:#010x}\n  Rust: ({:?},{:?}) bits {:#010x},{:#010x}",
            c.x, c.y, c.x.to_bits(), c.y.to_bits(),
            r.x, r.y, r.x.to_bits(), r.y.to_bits()
        );
    }
}

#[track_caller]
pub fn same_i32(what: &str, ctx: &str, c: i32, r: i32) {
    assert_eq!(c, r, "{what} MISMATCH\n  ctx : {ctx}");
}

// ---------------------------------------------------------------------------
// Shared `c2GJK` invocation helper
// ---------------------------------------------------------------------------

/// Everything `c2GJK` can be asked to produce, captured for comparison.
#[derive(Copy, Clone, Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: V,
    pub b: V,
    pub iters: i32,
    pub cache: Option<GJKCache>,
    /// Whether `outA`/`outB`/`iterations` were left at their poison values.
    pub a_untouched: bool,
    pub b_untouched: bool,
    pub it_untouched: bool,
}

pub const POISON_F32: f32 = f32::from_bits(0xA5A5_A5A5);
pub const POISON_I32: i32 = -1_234_567;

/// Full-control `c2GJK` call. Any out-parameter can be made NULL.
#[allow(clippy::too_many_arguments)]
pub unsafe fn gjk_call(
    f: &FnGJK,
    a: *const std::ffi::c_void,
    ta: i32,
    ax: Option<&X>,
    b: *const std::ffi::c_void,
    tb: i32,
    bx: Option<&X>,
    use_radius: i32,
    want_a: bool,
    want_b: bool,
    want_it: bool,
    cache: Option<GJKCache>,
) -> GjkOut {
    let mut oa = V::new(POISON_F32, POISON_F32);
    let mut ob = V::new(POISON_F32, POISON_F32);
    let mut it = POISON_I32;
    let mut cch = cache;

    let axp = ax.map_or(std::ptr::null(), |x| x as *const X);
    let bxp = bx.map_or(std::ptr::null(), |x| x as *const X);
    let oap = if want_a { &mut oa as *mut V } else { std::ptr::null_mut() };
    let obp = if want_b { &mut ob as *mut V } else { std::ptr::null_mut() };
    let itp = if want_it { &mut it as *mut i32 } else { std::ptr::null_mut() };
    let cp = cch.as_mut().map_or(std::ptr::null_mut(), |c| c as *mut GJKCache);

    let dist = unsafe { f(a, ta, axp, b, tb, bxp, oap, obp, use_radius, itp, cp) };

    GjkOut {
        dist,
        a: oa,
        b: ob,
        iters: it,
        cache: cch,
        a_untouched: oa.x.to_bits() == POISON_F32.to_bits()
            && oa.y.to_bits() == POISON_F32.to_bits(),
        b_untouched: ob.x.to_bits() == POISON_F32.to_bits()
            && ob.y.to_bits() == POISON_F32.to_bits(),
        it_untouched: it == POISON_I32,
    }
}

/// Assert two `GjkOut`s are bit-identical; panics with full detail otherwise.
#[track_caller]
pub fn gjk_same(what: &str, ctx: &str, c: &GjkOut, r: &GjkOut) {
    let bad = c.dist.to_bits() != r.dist.to_bits()
        || c.a.bits() != r.a.bits()
        || c.b.bits() != r.b.bits()
        || c.iters != r.iters
        || c.a_untouched != r.a_untouched
        || c.b_untouched != r.b_untouched
        || c.it_untouched != r.it_untouched
        || match (&c.cache, &r.cache) {
            (None, None) => false,
            (Some(x), Some(y)) => raw(x) != raw(y),
            _ => true,
        };
    if bad {
        panic!(
            "{what} MISMATCH\n  ctx : {ctx}\n\
             \n  C   : dist={:?} ({:#010x}) a={:?} b={:?} it={} cache={:?} untouched=({},{},{})\
             \n  Rust: dist={:?} ({:#010x}) a={:?} b={:?} it={} cache={:?} untouched=({},{},{})",
            c.dist, c.dist.to_bits(), c.a, c.b, c.iters, c.cache,
            c.a_untouched, c.b_untouched, c.it_untouched,
            r.dist, r.dist.to_bits(), r.a, r.b, r.iters, r.cache,
            r.a_untouched, r.b_untouched, r.it_untouched,
        );
    }
}

// ---------------------------------------------------------------------------
// Zero-cost-on-success check macros (formatting happens only on divergence)
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! ck_f32 {
    ($what:expr, $c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv): (f32, f32) = ($c, $r);
        if cv.to_bits() != rv.to_bits() {
            panic!(
                "{} MISMATCH\n  ctx : {}\n  C   : {:?} bits {:#010x}\n  Rust: {:?} bits {:#010x}",
                $what, format_args!($($ctx)*), cv, cv.to_bits(), rv, rv.to_bits()
            );
        }
    }};
}

#[macro_export]
macro_rules! ck_v {
    ($what:expr, $c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv) = ($c, $r);
        if cv.x.to_bits() != rv.x.to_bits() || cv.y.to_bits() != rv.y.to_bits() {
            panic!(
                "{} MISMATCH\n  ctx : {}\n  C   : ({:?}, {:?}) bits {:#010x},{:#010x}\n  Rust: ({:?}, {:?}) bits {:#010x},{:#010x}",
                $what, format_args!($($ctx)*),
                cv.x, cv.y, cv.x.to_bits(), cv.y.to_bits(),
                rv.x, rv.y, rv.x.to_bits(), rv.y.to_bits()
            );
        }
    }};
}

#[macro_export]
macro_rules! ck_i32 {
    ($what:expr, $c:expr, $r:expr, $($ctx:tt)*) => {{
        let (cv, rv): (i32, i32) = ($c, $r);
        if cv != rv {
            panic!(
                "{} MISMATCH\n  ctx : {}\n  C   : {}\n  Rust: {}",
                $what, format_args!($($ctx)*), cv, rv
            );
        }
    }};
}

/// Byte-for-byte comparison of any two mirrored (padding-free) structs.
#[macro_export]
macro_rules! ck_bytes {
    ($what:expr, $c:expr, $r:expr, $($ctx:tt)*) => {{
        let cb = $crate::common::raw(&$c);
        let rb = $crate::common::raw(&$r);
        if cb != rb {
            panic!(
                "{} MISMATCH\n  ctx : {}\n  C   : {:02x?}\n  Rust: {:02x?}\n  Cst : {:?}\n  Rst : {:?}",
                $what, format_args!($($ctx)*), cb, rb, $c, $r
            );
        }
    }};
}

/// Poison byte pattern used to prove a callee wrote exactly the bytes the C did.
pub const POISON: u8 = 0xA5;

pub fn poisoned<T: Copy>() -> T {
    let mut t = std::mem::MaybeUninit::<T>::uninit();
    unsafe {
        std::ptr::write_bytes(t.as_mut_ptr() as *mut u8, POISON, std::mem::size_of::<T>());
        t.assume_init()
    }
}

pub fn poisoned_verts(n: usize) -> Vec<V> {
    vec![V::new(f32::from_bits(0xA5A5_A5A5), f32::from_bits(0xA5A5_A5A5)); n]
}
