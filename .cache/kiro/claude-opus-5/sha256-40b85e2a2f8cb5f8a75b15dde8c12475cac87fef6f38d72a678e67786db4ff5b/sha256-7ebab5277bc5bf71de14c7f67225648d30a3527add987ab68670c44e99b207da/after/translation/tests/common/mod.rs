//! Shared test scaffolding: loads the C and Rust shared objects through `libloading`
//! and exposes the `c_src` type layouts.
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Types (must match c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

pub type C2_TYPE = c_uint;
pub const C2_TYPE_CAPSULE: C2_TYPE = 0;
pub const C2_TYPE_CIRCLE: C2_TYPE = 1;
pub const C2_TYPE_AABB: C2_TYPE = 2;
pub const C2_TYPE_POLY: C2_TYPE = 3;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
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
#[derive(Copy, Clone, Default, Debug)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with cmake first",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Prefer the cdylib built for the same profile as this test binary
    // (target/<profile>/deps/<test> -> target/<profile>/libomni_manifold_lib.so).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            let p = profile_dir.join("libomni_manifold_lib.so");
            if p.exists() {
                return p;
            }
        }
    }
    let t = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = t.join(profile).join("libomni_manifold_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust cdylib found; run `cargo build --release` first");
}

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

/// Paths of the two shared objects under comparison.
pub fn so_paths() -> (PathBuf, PathBuf) {
    (find_c_so(), find_rust_so())
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            Libs {
                c: Library::new(find_c_so()).expect("load C .so"),
                r: Library::new(find_rust_so()).expect("load Rust .so"),
            }
        }
    }

    /// Fetch the same symbol from both libraries.
    pub fn pair<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cn = format!("{name}\0");
        unsafe {
            let a: Symbol<T> = self
                .c
                .get(cn.as_bytes())
                .unwrap_or_else(|e| panic!("C symbol {name}: {e}"));
            let b: Symbol<T> = self
                .r
                .get(cn.as_bytes())
                .unwrap_or_else(|e| panic!("Rust symbol {name}: {e}"));
            (a, b)
        }
    }

    /// Exercises both `omni_manifold`s a few hundred times before comparisons start.
    ///
    /// `ptr_from_parts` never frees, so the first calls in a process make glibc
    /// create and extend its arena. Those slow paths (`sysmalloc`, `brk`) use far
    /// more stack than the steady state and write into the region `c2GJK` later
    /// reads its uninitialised `c2Proxy pB` from, so the very first calls can
    /// disagree with *any* deterministic implementation. Once the arena is warm the
    /// C is stable — measured at 1 divergence in 300 000 calls, at call index 2,
    /// with the identical input agreeing on every replay.
    pub fn warm_up(&self) {
        type OmniFn = unsafe extern "C" fn(
            *mut c2Manifold,
            C2_TYPE,
            f32,
            f32,
            f32,
            f32,
            f32,
            C2_TYPE,
            f32,
            f32,
            f32,
            f32,
            f32,
        );
        let (cf, rf) = self.pair::<OmniFn>("omni_manifold");
        for i in 0..256u32 {
            let ta = i % 4;
            let tb = (i / 4) % 4;
            let f = i as f32 * 0.03125;
            let mut m = c2Manifold::default();
            unsafe {
                cf(&mut m, ta, -1.0 - f, -0.5, 1.0, 0.5 + f, 0.75, tb, -0.25, 0.25, 1.25, 1.5, 0.5);
                rf(&mut m, ta, -1.0 - f, -0.5, 1.0, 0.5 + f, 0.75, tb, -0.25, 0.25, 1.25, 1.5, 0.5);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Bitwise comparison helpers
// ---------------------------------------------------------------------------

pub fn bytes_of<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Byte-for-byte equality of two values of the same POD type.
pub fn same<T>(a: &T, b: &T) -> bool {
    bytes_of(a) == bytes_of(b)
}

#[track_caller]
pub fn assert_same<T: std::fmt::Debug>(what: &str, c: &T, r: &T) {
    if !same(c, r) {
        panic!(
            "mismatch in {what}\n  C   = {:?} [{}]\n  Rust= {:?} [{}]",
            c,
            hex(bytes_of(c)),
            r,
            hex(bytes_of(r))
        );
    }
}

#[track_caller]
pub fn assert_f32(what: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "mismatch in {what}\n  C   = {c} (0x{:08x})\n  Rust= {r} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

/// Like [`assert_same`], but the message is only built when the values differ.
///
/// This matters for more than speed: `omni_manifold` -> `ptr_from_parts` calls
/// `malloc`, and how much stack glibc's `malloc` uses depends on the process-wide
/// heap state. Allocating in the comparison loop pushes `malloc` onto slower,
/// deeper paths that write into the region `c2GJK`'s uninitialised
/// `c2Proxy pB` is later read from — so a loop that allocates makes the C
/// disagree with *any* deterministic implementation. With no allocation in the
/// loop the C is stable (see `diag`-style measurement: 0 mismatches in 200k
/// capsule/AABB calls).
#[track_caller]
pub fn assert_same_lazy<T: std::fmt::Debug, F: FnOnce() -> String>(c: &T, r: &T, what: F) {
    if !same(c, r) {
        assert_same(&what(), c, r);
    }
}

#[track_caller]
pub fn assert_f32_lazy<F: FnOnce() -> String>(c: f32, r: f32, what: F) {
    if c.to_bits() != r.to_bits() {
        assert_f32(&what(), c, r);
    }
}

// ---------------------------------------------------------------------------
// Deterministic input generation
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
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
    /// A "tame" finite float in a physically plausible range, biased towards
    /// small integers and halves so that degenerate/equal cases are hit often.
    pub fn tame(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => (self.below(9) as f32) - 4.0,
            3 => ((self.below(41) as f32) - 20.0) * 0.5,
            4 => (self.below(3) as f32) - 1.0,
            5 => {
                let v = (self.next_u32() as f64 / u32::MAX as f64) as f32;
                (v - 0.5) * 20.0
            }
            6 => {
                let v = (self.next_u32() as f64 / u32::MAX as f64) as f32;
                (v - 0.5) * 2.0e-6
            }
            7 => {
                let v = (self.next_u32() as f64 / u32::MAX as f64) as f32;
                (v - 0.5) * 2.0e6
            }
            8 => (self.below(5) as f32) * 0.25,
            _ => {
                let v = (self.next_u32() as f64 / u32::MAX as f64) as f32;
                (v - 0.5) * 4.0
            }
        }
    }
    /// A non-negative radius-like value.
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => 1.0,
            2 => 0.5,
            3 => 2.0,
            4 => 1.0e-7,
            5 => 1.0e6,
            6 => (self.below(9) as f32) * 0.5,
            _ => (self.next_u32() as f64 / u32::MAX as f64) as f32 * 5.0,
        }
    }
    /// Anything at all, including inf / NaN / subnormals.
    pub fn wild(&mut self) -> f32 {
        match self.below(16) {
            0 => f32::INFINITY,
            1 => f32::NEG_INFINITY,
            2 => f32::NAN,
            3 => f32::from_bits(0xffc0_0000), // -NaN
            4 => f32::from_bits(0x7f80_0001), // signalling NaN
            5 => f32::from_bits(0x7fab_cdef), // NaN with payload
            6 => f32::from_bits(0xffab_cdef),
            7 => f32::from_bits(0x0000_0001), // smallest subnormal
            8 => f32::from_bits(0x8000_0001),
            9 => f32::MAX,
            10 => f32::MIN,
            11 => f32::from_bits(self.next_u32()),
            _ => self.tame(),
        }
    }
    pub fn vec_tame(&mut self) -> c2v {
        c2v {
            x: self.tame(),
            y: self.tame(),
        }
    }
    pub fn vec_wild(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
}

/// A fixed set of notable float values used for exhaustive small sweeps.
pub const NOTABLE: &[f32] = &[    0.0,
    -0.0,
    1.0,
    -1.0,
    2.0,
    -2.0,
    0.5,
    -0.5,
    3.0,
    1.0e-8,
    -1.0e-8,
    1.19209289550781250000000000000000000e-7,
    1.0e-6,
    1.0e6,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

// ---------------------------------------------------------------------------
// Stack scrubbing
// ---------------------------------------------------------------------------

/// Zeroes the stack region that the next call's frames will occupy.
///
/// `c2MakeProxy` has no `C2_TYPE_POLY` case, so for a polygon operand the C's
/// `c2Proxy pA/pB` inside `c2GJK` stay uninitialised and the result depends on
/// whatever bytes happen to sit at `rbp-0x100` / `rbp-0x150`. The only
/// well-defined reading of that is a stack page that has never been written,
/// i.e. all zeros — which is exactly what the Rust translation models, and what
/// the C itself does when measured on a fresh thread stack (see `probe2.rs`).
///
/// Calling this immediately before each C entry point reproduces that state, so
/// the comparison measures the translation instead of the test harness's own
/// stack litter. Nothing may be called between this and the C call.
/// Serialises the comparison loops inside a test binary.
///
/// Two things make concurrency observable through the C's undefined behaviour:
/// `ptr_from_parts` never frees, and `c2GJK` reads an uninitialised `c2Proxy`
/// for a `C2_TYPE_POLY` operand. Concurrent allocation pushes glibc onto
/// contended/slow paths that use much more stack, which lands in the region that
/// proxy is read from — and a garbage `c2Proxy::count` makes `c2Support` walk off
/// the end of the array, so the C can crash outright. Running the loops one at a
/// time keeps the C in its deterministic regime.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn serialize() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

#[inline(never)]
pub fn scrub_stack() {
    // 8 KiB comfortably covers the deepest C chain
    // (`omni_manifold` -> `c2Collide` -> `c2AABBtoCapsuleManifold` ->
    //  `c2CapsuletoPolyManifold` -> `c2GJK` -> `c2Witness` is under 2 KiB).
    //
    // The buffer lives in this function's own frame, which sits directly below the
    // caller's stack pointer — exactly where the C call's frames will go. Volatile
    // writes are required: LLVM will happily drop the initialisation of a buffer
    // whose address merely escapes through `black_box`.
    const WORDS: usize = 1024;
    let mut buf = [0u64; WORDS];
    let p = buf.as_mut_ptr();
    let mut i = 0;
    while i < WORDS {
        unsafe { std::ptr::write_volatile(p.add(i), 0u64) };
        i += 1;
    }
    std::hint::black_box(p);
}
