//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls every function
//! through its exported C symbol. The Rust crate is never linked directly, so
//! the `#[no_mangle]`/`extern "C"` wrappers are what actually gets tested.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI types (must mirror the C layout exactly)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

pub const ALL_TYPES: [c_int; 4] = [
    C2_TYPE_CAPSULE,
    C2_TYPE_CIRCLE,
    C2_TYPE_AABB,
    C2_TYPE_POLY,
];

/// Out-of-range enum values. C enums accept any `int`.
pub const BAD_TYPES: [c_int; 8] = [-1, 4, 5, 99, -99, i32::MAX, i32::MIN, 1000];

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct c2Poly {
    pub count: c_int,
    pub verts: [c2v; 8],
    pub norms: [c2v; 8],
}

impl Default for c2Poly {
    fn default() -> Self {
        c2Poly {
            count: 0,
            verts: [c2v::default(); 8],
            norms: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
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

/// C: `struct c2Simplex { c2sv a, b, c, d; float div; int count; }`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Byte-exact comparison
// ---------------------------------------------------------------------------

/// Raw bytes of any `Copy` POD value. Used so `NaN` payloads and `-0.0`
/// compare exactly, which `PartialEq` on floats would not.
pub fn raw<T: Copy>(v: &T) -> Vec<u8> {
    let p = v as *const T as *const u8;
    unsafe { std::slice::from_raw_parts(p, std::mem::size_of::<T>()) }.to_vec()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Assert two POD values are byte-identical.
#[track_caller]
pub fn same<T: Copy + std::fmt::Debug>(what: &str, c: &T, r: &T) {
    let cb = raw(c);
    let rb = raw(r);
    assert!(
        cb == rb,
        "{what}\n  C   = {c:?}\n        {}\n  Rust= {r:?}\n        {}",
        hex(&cb),
        hex(&rb)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) - fixed seed, reproducible
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E3779B97F4A7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-r, r].
    pub fn sym(&mut self, r: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * r
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Value snapped to a coarse grid, to force exact ties / touching /
    /// coincidence, which uniform floats essentially never hit.
    pub fn grid(&mut self, step: f32, n: i32) -> f32 {
        let k = (self.next_u64() % (2 * n as u64 + 1)) as i64 - n as i64;
        k as f32 * step
    }
    /// A completely arbitrary bit pattern reinterpreted as `f32` (may be
    /// `NaN`, `inf`, denormal, `-0.0`).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "spicy" value drawn from the pathological set.
    pub fn spicy(&mut self) -> f32 {
        const SET: [f32; 16] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            2.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            1e-30,
            1e30,
            f32::EPSILON,
        ];
        SET[self.below(SET.len())]
    }
    pub fn vec_sym(&mut self, r: f32) -> c2v {
        c2v {
            x: self.sym(r),
            y: self.sym(r),
        }
    }
    pub fn vec_grid(&mut self, step: f32, n: i32) -> c2v {
        c2v {
            x: self.grid(step, n),
            y: self.grid(step, n),
        }
    }
    pub fn vec_any(&mut self) -> c2v {
        c2v {
            x: self.any_f32(),
            y: self.any_f32(),
        }
    }
    pub fn vec_spicy(&mut self) -> c2v {
        c2v {
            x: self.spicy(),
            y: self.spicy(),
        }
    }
    /// A unit-ish rotation (as a real caller would build it).
    pub fn rot_unit(&mut self) -> c2r {
        let a = self.unit() * std::f32::consts::TAU;
        c2r {
            c: a.cos(),
            s: a.sin(),
        }
    }
    /// A rotation with `c*c + s*s != 1` - the C never normalizes.
    pub fn rot_raw(&mut self, r: f32) -> c2r {
        c2r {
            c: self.sym(r),
            s: self.sym(r),
        }
    }
    pub fn xform(&mut self, tr: f32, unit_rot: bool) -> c2x {
        c2x {
            p: self.vec_sym(tr),
            r: if unit_rot {
                self.rot_unit()
            } else {
                self.rot_raw(2.0)
            },
        }
    }
}

// ---------------------------------------------------------------------------
// The two loaded libraries
// ---------------------------------------------------------------------------

fn root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <work>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_so_path() -> PathBuf {
    let dir = root().join("c_src").join("build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(e.path());
            }
        }
    }
    found.unwrap_or_else(|| panic!("no C .so found in {dir:?}; build c_src first"))
}

fn rust_so_path() -> PathBuf {
    // Allows the same suite to be run against a different build profile, e.g.
    //   RUST_SO=target/debug/libomni_manifold_lib.so cargo test --release
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO points at a missing file: {p:?}");
        return p;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libomni_manifold_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libomni_manifold_lib.so not found; run `cargo build --release`")
}

/// A pair of dynamically-loaded libraries: C on the left, Rust on the right.
pub struct Pair {
    pub c: Library,
    pub r: Library,
}

impl Pair {
    pub fn load() -> Self {
        unsafe {
            Pair {
                c: Library::new(c_so_path()).expect("load C .so"),
                r: Library::new(rust_so_path()).expect("load Rust .so"),
            }
        }
    }

    /// Fetch the same symbol from both libraries.
    pub fn get<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        unsafe {
            let cs: Symbol<T> = self
                .c
                .get(name)
                .unwrap_or_else(|e| panic!("C missing {}: {e}", String::from_utf8_lossy(name)));
            let rs: Symbol<T> = self
                .r
                .get(name)
                .unwrap_or_else(|e| panic!("Rust missing {}: {e}", String::from_utf8_lossy(name)));
            (cs, rs)
        }
    }
}

pub fn pair() -> Pair {
    Pair::load()
}

// ---------------------------------------------------------------------------
// Stack scrubbing
// ---------------------------------------------------------------------------

/// Zero-fill ~4 KiB of stack *below* the current frame.
///
/// Why this is needed: `c2MakeProxy` has no `C2_TYPE_POLY` case, and `c2GJK`
/// declares `c2Proxy pA, pB;` uninitialized, so on the poly path the C reads an
/// indeterminate local. Its value is whatever the *caller's* stack left behind,
/// which was verified experimentally (`probe5.rs`) to change the C's answer.
/// That makes the C's output on that path a function of our own test harness's
/// stack, not of the library's inputs.
///
/// Calling this immediately before a library call puts a zero-filled region
/// where the callee's frames will land, which pins the C's indeterminate proxy
/// to all-zeros — exactly the state the Rust translation initializes it to.
/// With this in place the two libraries agree bit-for-bit on that path.
#[inline(never)]
pub fn scrub_stack() {
    let mut buf = [0u8; 4096];
    std::hint::black_box(&mut buf[..]);
}

// ---------------------------------------------------------------------------
// Function-pointer type aliases for every exported symbol
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVF = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVF = unsafe extern "C" fn(c2v) -> f32;
pub type FnFFV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVFV = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnHVF = unsafe extern "C" fn(c2h, c2v) -> f32;
pub type FnPolyIH = unsafe extern "C" fn(*const c2Poly, c_int) -> c2h;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplex = unsafe extern "C" fn(*mut c2Simplex);
pub type FnRVV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXVV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnIntersect = unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v;
pub type FnSupport = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
pub type FnNorms = unsafe extern "C" fn(*mut c2v, *mut c2v, c_int);

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

pub type FnCC = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
pub type FnCA = unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold);
pub type FnCCap = unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold);
pub type FnAA = unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold);
pub type FnACap = unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold);
pub type FnCapCap = unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold);
pub type FnCapPoly =
    unsafe extern "C" fn(c2Capsule, *const c2Poly, *const c2x, *mut c2Manifold);
pub type FnCollide =
    unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
pub type FnPtrFromParts =
    unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void;
pub type FnOmni = unsafe extern "C" fn(
    *mut c2Manifold,
    c_int,
    f32,
    f32,
    f32,
    f32,
    f32,
    c_int,
    f32,
    f32,
    f32,
    f32,
    f32,
);

// ---------------------------------------------------------------------------
// Poisoned manifold: a recognisable non-zero bit pattern, so that a function
// which leaves part of `*m` untouched is caught (rather than silently agreeing
// on a zeroed struct).
// ---------------------------------------------------------------------------

pub fn poison_manifold(tag: u8) -> c2Manifold {
    let mut m = c2Manifold::default();
    let p = &mut m as *mut c2Manifold as *mut u8;
    let n = std::mem::size_of::<c2Manifold>();
    unsafe {
        for i in 0..n {
            *p.add(i) = tag.wrapping_add(i as u8).wrapping_mul(7) | 1;
        }
    }
    m
}

/// A random convex CCW polygon with `count` vertices, with `norms` computed
/// consistently (the way a real caller would, via the library's own `c2Norms`).
pub fn convex_poly(rng: &mut Rng, count: c_int, radius: f32, norms_fn: FnNorms) -> c2Poly {
    let mut p = c2Poly::default();
    p.count = count;
    let n = count.max(0).min(8);
    // Sorted random angles => convex, counter-clockwise.
    let mut angs: Vec<f32> = (0..n).map(|_| rng.unit() * std::f32::consts::TAU).collect();
    angs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for i in 0..n as usize {
        let r = radius * (0.6 + 0.4 * rng.unit());
        p.verts[i] = c2v {
            x: r * angs[i].cos(),
            y: r * angs[i].sin(),
        };
    }
    if n > 0 {
        unsafe { norms_fn(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), n) };
    }
    p
}
