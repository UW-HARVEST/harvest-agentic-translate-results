//! Shared differential-test harness.
//!
//! Loads BOTH the C shared object and the Rust cdylib with `libloading` and
//! exposes each exported symbol as a pair of function pointers.  Nothing in
//! this crate's Rust code is ever called directly: every Rust call goes through
//! the `.so`'s `#[no_mangle]` export, exactly as an external C consumer would.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI types (re-declared here so the tests never link against the Rust crate)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `struct { c2sv a, b, c, d; float div; int count; }`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-for-bit float comparison (so `-0.0 != 0.0` and NaN payloads matter).
pub fn feq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn veq(a: c2v, b: c2v) -> bool {
    feq(a.x, b.x) && feq(a.y, b.y)
}

pub fn req(a: c2r, b: c2r) -> bool {
    feq(a.c, b.c) && feq(a.s, b.s)
}

pub fn xeq(a: c2x, b: c2x) -> bool {
    veq(a.p, b.p) && req(a.r, b.r)
}

pub fn proxyeq(a: &c2Proxy, b: &c2Proxy) -> bool {
    feq(a.radius, b.radius)
        && a.count == b.count
        && (0..8).all(|i| veq(a.verts[i], b.verts[i]))
}

pub fn svq(a: &c2sv, b: &c2sv) -> bool {
    veq(a.sA, b.sA) && veq(a.sB, b.sB) && veq(a.p, b.p) && feq(a.u, b.u) && a.iA == b.iA && a.iB == b.iB
}

pub fn simplexeq(a: &c2Simplex, b: &c2Simplex) -> bool {
    (0..4).all(|i| svq(&a.verts[i], &b.verts[i])) && feq(a.div, b.div) && a.count == b.count
}

pub fn cacheeq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    feq(a.metric, b.metric) && a.count == b.count && a.iA == b.iA && a.iB == b.iB && feq(a.div, b.div)
}

/// Raw byte image of a value — used to prove *every* byte matches, padding
/// included where the C writes it.
pub fn bytes_of<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub fn fmt_f(v: f32) -> String {
    format!("{:?}(0x{:08x})", v, v.to_bits())
}

pub fn fmt_v(v: c2v) -> String {
    format!("({}, {})", fmt_f(v.x), fmt_f(v.y))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(std::cell::Cell::new(seed ^ 0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&self) -> u64 {
        self.0.set(self.0.get().wrapping_add(0x9E37_79B9_7F4A_7C15));
        let mut z = self.0.get();
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-a, a]`.
    pub fn sym(&self, a: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * a
    }
    /// A float drawn from a wide spread of "interesting" magnitudes.
    pub fn wild(&self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => self.sym(1.0e-30),
            3 => self.sym(1.0e-6),
            4 => self.sym(1.0),
            5 => self.sym(100.0),
            6 => self.sym(1.0e6),
            7 => self.sym(1.0e18),
            8 => (self.below(41) as i32 - 20) as f32, // exact small integers
            _ => self.sym(50.0),
        }
    }
    /// A float from a "geometric" range that actually produces interesting
    /// collision configurations.
    pub fn geo(&self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => (self.below(41) as i32 - 20) as f32,
            2 => self.sym(5.0),
            3 => self.sym(0.001),
            _ => self.sym(120.0),
        }
    }
    pub fn geo_v(&self) -> c2v {
        c2v { x: self.geo(), y: self.geo() }
    }
    pub fn wild_v(&self) -> c2v {
        c2v { x: self.wild(), y: self.wild() }
    }
    /// Non-negative radius, sometimes exactly zero.
    pub fn radius(&self) -> f32 {
        match self.below(6) {
            0 => 0.0,
            1 => self.unit() * 0.001,
            2 => (self.below(21)) as f32,
            _ => self.unit() * 40.0,
        }
    }
    pub fn circle(&self) -> c2Circle {
        c2Circle { p: self.geo_v(), r: self.radius() }
    }
    /// AABB with `min <= max` most of the time, occasionally degenerate.
    pub fn aabb(&self) -> c2AABB {
        let a = self.geo_v();
        let w = self.unit() * 60.0;
        let h = self.unit() * 60.0;
        match self.below(8) {
            0 => c2AABB { min: a, max: a },           // degenerate point box
            1 => c2AABB { min: a, max: c2v { x: a.x + w, y: a.y } }, // flat
            _ => c2AABB { min: a, max: c2v { x: a.x + w, y: a.y + h } },
        }
    }
    pub fn capsule(&self) -> c2Capsule {
        let a = self.geo_v();
        match self.below(8) {
            0 => c2Capsule { a, b: a, r: self.radius() }, // point capsule
            _ => c2Capsule { a, b: c2v { x: a.x + self.sym(60.0), y: a.y + self.sym(60.0) }, r: self.radius() },
        }
    }
    /// A normalized rotation.
    pub fn rot(&self) -> c2r {
        let t = self.unit() * std::f32::consts::TAU;
        c2r { c: t.cos(), s: t.sin() }
    }
    pub fn xform(&self) -> c2x {
        c2x { p: self.geo_v(), r: self.rot() }
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                best = Some(p);
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "no C .so found in {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the profile the harness was asked for, then release, then debug.
    let want = std::env::var("DIFFTEST_RUST_PROFILE").unwrap_or_else(|_| "release".into());
    for prof in [want.as_str(), "release", "debug"] {
        let p = base.join(prof).join("libaabb_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libaabb_lib.so not found under {} — run `cargo build --release`", base.display());
}

macro_rules! decl_syms {
    ( $( $name:ident : $ty:ty ; )* ) => {
        pub struct Api {
            _c: Library,
            _r: Library,
            $( pub $name: ($ty, $ty), )*
        }

        impl Api {
            fn load() -> Api {
                let cp = find_c_so();
                let rp = find_rust_so();
                unsafe {
                    let c = Library::new(&cp)
                        .unwrap_or_else(|e| panic!("loading {}: {e}", cp.display()));
                    let r = Library::new(&rp)
                        .unwrap_or_else(|e| panic!("loading {}: {e}", rp.display()));
                    $(
                        let cs: Symbol<$ty> = c.get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("C .so missing {}: {e}", stringify!($name)));
                        let rs: Symbol<$ty> = r.get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!("Rust .so missing {}: {e}", stringify!($name)));
                        let $name = (*cs, *rs);
                    )*
                    Api { $( $name, )* _c: c, _r: r }
                }
            }
        }
    };
}

decl_syms! {
    c2V:                 extern "C" fn(f32, f32) -> c2v;
    c2Mulvs:             extern "C" fn(c2v, f32) -> c2v;
    c2Maxv:              extern "C" fn(c2v, c2v) -> c2v;
    c2Minv:              extern "C" fn(c2v, c2v) -> c2v;
    c2Clampv:            extern "C" fn(c2v, c2v, c2v) -> c2v;
    c2Sub:               extern "C" fn(c2v, c2v) -> c2v;
    c2Dot:               extern "C" fn(c2v, c2v) -> f32;
    c2RotIdentity:       extern "C" fn() -> c2r;
    c2xIdentity:         extern "C" fn() -> c2x;
    c2BBVerts:           unsafe extern "C" fn(*mut c2v, *mut c2AABB);
    c2MakeProxy:         unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
    c2Len:               extern "C" fn(c2v) -> f32;
    c2Det2:              extern "C" fn(c2v, c2v) -> f32;
    c2GJKSimplexMetric:  unsafe extern "C" fn(*mut c2Simplex) -> f32;
    c2Mulrv:             extern "C" fn(c2r, c2v) -> c2v;
    c2Add:               extern "C" fn(c2v, c2v) -> c2v;
    c2Mulxv:             extern "C" fn(c2x, c2v) -> c2v;
    c22:                 unsafe extern "C" fn(*mut c2Simplex);
    c23:                 unsafe extern "C" fn(*mut c2Simplex);
    c2Neg:               extern "C" fn(c2v) -> c2v;
    c2Skew:              extern "C" fn(c2v) -> c2v;
    c2CCW90:             extern "C" fn(c2v) -> c2v;
    c2D:                 unsafe extern "C" fn(*mut c2Simplex) -> c2v;
    c2Support:           unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
    c2Witness:           unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
    c2Div:               extern "C" fn(c2v, f32) -> c2v;
    c2Norm:              extern "C" fn(c2v) -> c2v;
    c2L:                 unsafe extern "C" fn(*mut c2Simplex) -> c2v;
    c2MulrvT:            extern "C" fn(c2r, c2v) -> c2v;
    c2GJK:               unsafe extern "C" fn(*const c_void, c_int, *const c2x, *const c_void, c_int, *const c2x, *mut c2v, *mut c2v, c_int, *mut c_int, *mut c2GJKCache) -> f32;
    c2AABBtoAABB:        extern "C" fn(c2AABB, c2AABB) -> c_int;
    c2AABBtoCapsule:     extern "C" fn(c2AABB, c2Capsule) -> c_int;
    c2CapsuletoCapsule:  extern "C" fn(c2Capsule, c2Capsule) -> c_int;
    c2CircletoCircle:    extern "C" fn(c2Circle, c2Circle) -> c_int;
    c2CircletoAABB:      extern "C" fn(c2Circle, c2AABB) -> c_int;
    c2CircletoCapsule:   extern "C" fn(c2Circle, c2Capsule) -> c_int;
    c2Collided:          unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
    aabb:                extern "C" fn(f32, f32, f32, f32) -> c_int;
}

/// The 38 exported symbol names, in the order `nm -D` sorts them.
pub const ALL_SYMBOLS: &[&str] = &[
    "aabb", "c22", "c23", "c2AABBtoAABB", "c2AABBtoCapsule", "c2Add", "c2BBVerts", "c2CCW90",
    "c2CapsuletoCapsule", "c2CircletoAABB", "c2CircletoCapsule", "c2CircletoCircle", "c2Clampv",
    "c2Collided", "c2D", "c2Det2", "c2Div", "c2Dot", "c2GJK", "c2GJKSimplexMetric", "c2L", "c2Len",
    "c2MakeProxy", "c2Maxv", "c2Minv", "c2Mulrv", "c2MulrvT", "c2Mulvs", "c2Mulxv", "c2Neg",
    "c2Norm", "c2RotIdentity", "c2Skew", "c2Sub", "c2Support", "c2V", "c2Witness", "c2xIdentity",
];

pub fn api() -> &'static Api {
    use std::sync::OnceLock;
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(Api::load)
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

// ---------------------------------------------------------------------------
// Failure accounting: collect every divergence in a row instead of aborting on
// the first one, so a single run reports the whole picture.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Diff {
    pub row: String,
    pub checked: usize,
    pub fails: Vec<String>,
}

impl Diff {
    pub fn new(row: &str) -> Diff {
        Diff { row: row.to_string(), checked: 0, fails: Vec::new() }
    }
    pub fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.checked += 1;
        if !ok && self.fails.len() < 12 {
            self.fails.push(msg());
        } else if !ok {
            self.fails.push("...".into());
        }
    }
    pub fn finish(self) {
        assert!(self.checked > 0, "[{}] no cases were exercised", self.row);
        if !self.fails.is_empty() {
            panic!(
                "[{}] {} / {} cases DIVERGED between C and Rust:\n{}",
                self.row,
                self.fails.len(),
                self.checked,
                self.fails.join("\n")
            );
        }
        eprintln!("[{}] ok ({} cases)", self.row, self.checked);
    }
}
