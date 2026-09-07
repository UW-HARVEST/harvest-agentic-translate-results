//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C reference and the Rust translation) with
//! `libloading` and exposes helpers to call the same exported symbol in both
//! and compare the results bit-for-bit.
//!
//! Nothing here calls a Rust function directly — every call goes through the
//! `.so` export table, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.

#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Mirror of the C types (layouts asserted below)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;

pub const ALL_TYPES: [c_int; 3] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB];

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [C2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2sv {
    pub sA: C2v,
    pub sB: C2v,
    pub p: C2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `struct { c2sv a, b, c, d; float div; int count; }` — the four `c2sv`
/// members are contiguous and treated as an array by the C code.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Simplex {
    pub verts: [C2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub fn assert_layouts() {
    use std::mem::size_of;
    assert_eq!(size_of::<C2v>(), 8);
    assert_eq!(size_of::<C2r>(), 8);
    assert_eq!(size_of::<C2x>(), 16);
    assert_eq!(size_of::<C2Circle>(), 12);
    assert_eq!(size_of::<C2AABB>(), 16);
    assert_eq!(size_of::<C2Capsule>(), 20);
    assert_eq!(size_of::<C2GJKCache>(), 36);
    assert_eq!(size_of::<C2Proxy>(), 72);
    assert_eq!(size_of::<C2sv>(), 36);
    assert_eq!(size_of::<C2Simplex>(), 152);
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<manifest>/target/<profile>` — derived from the running test binary's own
/// path (`.../target/<profile>/deps/<test>`), so it always matches the profile
/// `cargo test` was invoked with.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("profile dir")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|s| s == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no .so found in {build:?} — build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    found.remove(0)
}

fn newest_mtime(dir: &std::path::Path) -> Option<std::time::SystemTime> {
    let mut newest = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|s| s == "rs").unwrap_or(false) {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    if newest.map_or(true, |n| m > n) {
                        newest = Some(m);
                    }
                }
            }
        }
    }
    newest
}

fn rust_so_path() -> PathBuf {
    let p = profile_dir().join("libomni_collide_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?} — run ./run_tests.sh (or `cargo build --release`) first."
    );

    // ---------------------------------------------------------------------
    // FRESHNESS GUARD — do not remove.
    //
    // This crate's only lib target is a `cdylib`. `cargo test` does NOT build
    // a cdylib-only lib target (integration tests have nothing to link it
    // against), so `cargo test` on its own happily runs against a STALE `.so`
    // left over from an earlier `cargo build`. That silently turns every
    // differential assertion into a no-op: the tests would keep passing while
    // testing code that no longer exists on disk.
    //
    // Verified experimentally: touching `src/lib.rs` and running
    // `cargo test --release` leaves `target/release/libomni_collide_lib.so`
    // untouched.
    //
    // So: refuse to run if the `.so` is older than any `.rs` source file.
    // ---------------------------------------------------------------------
    let so_mtime = std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .expect("stat cdylib");
    if let Some(src_mtime) = newest_mtime(&manifest_dir().join("src")) {
        assert!(
            so_mtime >= src_mtime,
            "STALE Rust cdylib: {p:?} is OLDER than src/*.rs.\n\
             `cargo test` does not rebuild a cdylib-only lib target, so this run \
             would have silently tested outdated code.\n\
             Run `./run_tests.sh` (which does `cargo build` first) instead of bare `cargo test`."
        );
    }
    p
}

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

impl Libs {
    pub fn load() -> Libs {
        assert_layouts();
        unsafe {
            Libs {
                c: Library::new(c_so_path()).expect("load C .so"),
                r: Library::new(rust_so_path()).expect("load Rust .so"),
            }
        }
    }

    /// Fetch the same symbol from both libraries.
    pub unsafe fn pair<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let bytes = name.as_bytes();
        let cs: Symbol<T> = self
            .c
            .get(bytes)
            .unwrap_or_else(|e| panic!("C .so is missing symbol `{name}`: {e}"));
        let rs: Symbol<T> = self
            .r
            .get(bytes)
            .unwrap_or_else(|e| panic!("Rust .so is missing symbol `{name}`: {e}"));
        (cs, rs)
    }
}

pub fn libs() -> Libs {
    Libs::load()
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

pub trait BitEq {
    fn bits(&self) -> String;
    fn beq(&self, other: &Self) -> bool;
}

impl BitEq for f32 {
    fn bits(&self) -> String {
        format!("{self:?}(0x{:08x})", self.to_bits())
    }
    /// Bit-exact, with ONE deliberate relaxation: two NaNs compare equal even
    /// if their sign bit / payload differs.
    ///
    /// Rationale: when both operands of an SSE arithmetic instruction are NaN,
    /// the hardware propagates the *destination* operand's payload, so the
    /// resulting NaN's sign bit depends purely on which register the compiler
    /// happened to pick. gcc at `-O0`/`-O2` and rustc at `-O3` make different
    /// (equally valid) choices for the same source expression, e.g.
    /// `a.x*b.x + a.y*b.y`. This is not observable behaviour any caller can
    /// depend on and it is not a translation defect.
    ///
    /// Everything else stays strict: `+0.0` vs `-0.0`, `+inf` vs `-inf`, NaN
    /// vs non-NaN and every finite value are all compared bit-for-bit.
    fn beq(&self, o: &Self) -> bool {
        if self.is_nan() || o.is_nan() {
            return self.is_nan() && o.is_nan();
        }
        self.to_bits() == o.to_bits()
    }
}

impl BitEq for c_int {
    fn bits(&self) -> String {
        format!("{self}")
    }
    fn beq(&self, o: &Self) -> bool {
        self == o
    }
}

impl BitEq for C2v {
    fn bits(&self) -> String {
        format!("({}, {})", self.x.bits(), self.y.bits())
    }
    fn beq(&self, o: &Self) -> bool {
        self.x.beq(&o.x) && self.y.beq(&o.y)
    }
}

impl BitEq for C2r {
    fn bits(&self) -> String {
        format!("(c={}, s={})", self.c.bits(), self.s.bits())
    }
    fn beq(&self, o: &Self) -> bool {
        self.c.beq(&o.c) && self.s.beq(&o.s)
    }
}

impl BitEq for C2x {
    fn bits(&self) -> String {
        format!("(p={}, r={})", self.p.bits(), self.r.bits())
    }
    fn beq(&self, o: &Self) -> bool {
        self.p.beq(&o.p) && self.r.beq(&o.r)
    }
}

impl<T: BitEq, const N: usize> BitEq for [T; N] {
    fn bits(&self) -> String {
        let v: Vec<String> = self.iter().map(|x| x.bits()).collect();
        format!("[{}]", v.join(", "))
    }
    fn beq(&self, o: &Self) -> bool {
        self.iter().zip(o.iter()).all(|(a, b)| a.beq(b))
    }
}

impl BitEq for C2AABB {
    fn bits(&self) -> String {
        format!("AABB{{min={}, max={}}}", self.min.bits(), self.max.bits())
    }
    fn beq(&self, o: &Self) -> bool {
        self.min.beq(&o.min) && self.max.beq(&o.max)
    }
}

impl BitEq for C2Circle {
    fn bits(&self) -> String {
        format!("Circle{{p={}, r={}}}", self.p.bits(), self.r.bits())
    }
    fn beq(&self, o: &Self) -> bool {
        self.p.beq(&o.p) && self.r.beq(&o.r)
    }
}

impl BitEq for C2Capsule {
    fn bits(&self) -> String {
        format!(
            "Capsule{{a={}, b={}, r={}}}",
            self.a.bits(),
            self.b.bits(),
            self.r.bits()
        )
    }
    fn beq(&self, o: &Self) -> bool {
        self.a.beq(&o.a) && self.b.beq(&o.b) && self.r.beq(&o.r)
    }
}

impl BitEq for C2Proxy {
    fn bits(&self) -> String {
        format!(
            "Proxy{{radius={}, count={}, verts={}}}",
            self.radius.bits(),
            self.count,
            self.verts.bits()
        )
    }
    fn beq(&self, o: &Self) -> bool {
        // Compare the whole 72 bytes: the C code may leave parts untouched and
        // the Rust must leave exactly the same parts untouched.
        self.radius.beq(&o.radius) && self.count == o.count && self.verts.beq(&o.verts)
    }
}

impl BitEq for C2sv {
    fn bits(&self) -> String {
        format!(
            "sv{{sA={}, sB={}, p={}, u={}, iA={}, iB={}}}",
            self.sA.bits(),
            self.sB.bits(),
            self.p.bits(),
            self.u.bits(),
            self.iA,
            self.iB
        )
    }
    fn beq(&self, o: &Self) -> bool {
        self.sA.beq(&o.sA)
            && self.sB.beq(&o.sB)
            && self.p.beq(&o.p)
            && self.u.beq(&o.u)
            && self.iA == o.iA
            && self.iB == o.iB
    }
}

impl BitEq for C2Simplex {
    fn bits(&self) -> String {
        format!(
            "Simplex{{count={}, div={}, verts={}}}",
            self.count,
            self.div.bits(),
            self.verts.bits()
        )
    }
    fn beq(&self, o: &Self) -> bool {
        self.count == o.count && self.div.beq(&o.div) && self.verts.beq(&o.verts)
    }
}

impl BitEq for C2GJKCache {
    fn bits(&self) -> String {
        format!(
            "Cache{{metric={}, count={}, iA={:?}, iB={:?}, div={}}}",
            self.metric.bits(),
            self.count,
            self.iA,
            self.iB,
            self.div.bits()
        )
    }
    fn beq(&self, o: &Self) -> bool {
        self.metric.beq(&o.metric)
            && self.count == o.count
            && self.iA == o.iA
            && self.iB == o.iB
            && self.div.beq(&o.div)
    }
}

impl<A: BitEq, B: BitEq> BitEq for (A, B) {
    fn bits(&self) -> String {
        format!("({}, {})", self.0.bits(), self.1.bits())
    }
    fn beq(&self, o: &Self) -> bool {
        self.0.beq(&o.0) && self.1.beq(&o.1)
    }
}

impl<A: BitEq, B: BitEq, C: BitEq> BitEq for (A, B, C) {
    fn bits(&self) -> String {
        format!("({}, {}, {})", self.0.bits(), self.1.bits(), self.2.bits())
    }
    fn beq(&self, o: &Self) -> bool {
        self.0.beq(&o.0) && self.1.beq(&o.1) && self.2.beq(&o.2)
    }
}

impl<A: BitEq, B: BitEq, C: BitEq, D: BitEq> BitEq for (A, B, C, D) {
    fn bits(&self) -> String {
        format!(
            "({}, {}, {}, {})",
            self.0.bits(),
            self.1.bits(),
            self.2.bits(),
            self.3.bits()
        )
    }
    fn beq(&self, o: &Self) -> bool {
        self.0.beq(&o.0) && self.1.beq(&o.1) && self.2.beq(&o.2) && self.3.beq(&o.3)
    }
}

/// A per-row differential checker that accumulates failures so one test run
/// reports everything that diverges instead of only the first case.
pub struct Diff {
    pub row: String,
    pub cases: usize,
    pub failures: Vec<String>,
}

impl Diff {
    pub fn new(row: &str) -> Diff {
        Diff {
            row: row.to_string(),
            cases: 0,
            failures: Vec::new(),
        }
    }

    pub fn check<T: BitEq>(&mut self, input: impl std::fmt::Debug, c: T, r: T) {
        self.cases += 1;
        if !c.beq(&r) {
            if self.failures.len() < 12 {
                self.failures.push(format!(
                    "  input {input:?}\n    C   = {}\n    RUST= {}",
                    c.bits(),
                    r.bits()
                ));
            }
        }
    }

    pub fn finish(self) {
        assert!(self.cases > 0, "row `{}` ran zero cases", self.row);
        if !self.failures.is_empty() {
            panic!(
                "row `{}`: {} of {} cases DIVERGED:\n{}",
                self.row,
                self.failures.len(),
                self.cases,
                self.failures.join("\n")
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) + value generators
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
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-scale, scale]`.
    pub fn sym(&mut self, scale: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * scale
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n
    }
    pub fn boolean(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }
    /// A completely arbitrary 32-bit pattern reinterpreted as `f32`
    /// (covers NaNs with random payloads, denormals, huge exponents...).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Small-integer grid value in `[-4, 4]` step `0.5` — produces many exact
    /// ties / touching configurations.
    pub fn grid(&mut self) -> f32 {
        (self.below(17) as f32 - 8.0) * 0.5
    }
}

/// Nasty but *finite-ish* special values the branches are sensitive to.
pub const SPECIALS: [f32; 22] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    f32::EPSILON,
    -f32::EPSILON,
    1.1920929e-7,  // the FLT_EPSILON literal spelled out in the C source
    -1.1920929e-7,
    5.960465e-8, // FLT_EPSILON / 2
    1e-30,
    -1e-30,
    1e-45, // smallest denormal
    f32::MIN_POSITIVE,
    1e30,
    -1e30,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    3.0,
];

/// Mixed generator: mostly "reasonable" values, sometimes a special one, and
/// occasionally a totally arbitrary bit pattern.
pub fn mixed(rng: &mut Rng, scale: f32) -> f32 {
    match rng.below(16) {
        0..=9 => rng.sym(scale),
        10 | 11 => rng.grid(),
        12 | 13 => SPECIALS[rng.below(SPECIALS.len())],
        14 => rng.any_f32(),
        _ => rng.sym(scale * 1000.0),
    }
}

/// "Tame" generator: finite, moderate magnitudes only.
pub fn tame(rng: &mut Rng, scale: f32) -> f32 {
    if rng.below(4) == 0 {
        rng.grid()
    } else {
        rng.sym(scale)
    }
}

pub fn vec_mixed(rng: &mut Rng, scale: f32) -> C2v {
    C2v {
        x: mixed(rng, scale),
        y: mixed(rng, scale),
    }
}

pub fn vec_tame(rng: &mut Rng, scale: f32) -> C2v {
    C2v {
        x: tame(rng, scale),
        y: tame(rng, scale),
    }
}

pub fn rot_mixed(rng: &mut Rng) -> C2r {
    if rng.below(3) == 0 {
        // unnormalized / degenerate rotation
        C2r {
            c: mixed(rng, 4.0),
            s: mixed(rng, 4.0),
        }
    } else {
        let t = rng.unit() * std::f32::consts::TAU;
        C2r {
            c: t.cos(),
            s: t.sin(),
        }
    }
}

pub fn x_mixed(rng: &mut Rng, scale: f32) -> C2x {
    C2x {
        p: vec_tame(rng, scale),
        r: rot_mixed(rng),
    }
}

pub fn circle(rng: &mut Rng, scale: f32) -> C2Circle {
    C2Circle {
        p: vec_tame(rng, scale),
        r: if rng.below(8) == 0 {
            tame(rng, scale)
        } else {
            rng.unit() * scale
        },
    }
}

pub fn aabb(rng: &mut Rng, scale: f32) -> C2AABB {
    let a = vec_tame(rng, scale);
    let b = vec_tame(rng, scale);
    if rng.below(6) == 0 {
        // possibly inverted
        C2AABB { min: a, max: b }
    } else {
        C2AABB {
            min: C2v {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: C2v {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }
}

pub fn capsule(rng: &mut Rng, scale: f32) -> C2Capsule {
    let a = vec_tame(rng, scale);
    let b = if rng.below(8) == 0 {
        a // degenerate zero-length capsule
    } else {
        vec_tame(rng, scale)
    };
    C2Capsule {
        a,
        b,
        r: if rng.below(8) == 0 {
            tame(rng, scale)
        } else {
            rng.unit() * scale
        },
    }
}

/// Random shape of the requested `C2_TYPE`, serialised into a 20-byte buffer
/// (the largest shape) so it can be handed to the `const void *` APIs.
pub fn shape_bytes(rng: &mut Rng, ty: c_int, scale: f32) -> [u8; 20] {
    let mut buf = [0u8; 20];
    unsafe {
        match ty {
            C2_TYPE_CIRCLE => {
                let c = circle(rng, scale);
                std::ptr::copy_nonoverlapping(
                    &c as *const C2Circle as *const u8,
                    buf.as_mut_ptr(),
                    12,
                );
            }
            C2_TYPE_AABB => {
                let a = aabb(rng, scale);
                std::ptr::copy_nonoverlapping(
                    &a as *const C2AABB as *const u8,
                    buf.as_mut_ptr(),
                    16,
                );
            }
            _ => {
                let c = capsule(rng, scale);
                std::ptr::copy_nonoverlapping(
                    &c as *const C2Capsule as *const u8,
                    buf.as_mut_ptr(),
                    20,
                );
            }
        }
    }
    buf
}

pub fn ty_name(ty: c_int) -> &'static str {
    match ty {
        C2_TYPE_CAPSULE => "CAPSULE",
        C2_TYPE_CIRCLE => "CIRCLE",
        C2_TYPE_AABB => "AABB",
        _ => "BAD",
    }
}

/// Build a `c2Simplex` whose fields are random and whose `count` is exactly
/// `count` (which may deliberately be out of range).
pub fn simplex(rng: &mut Rng, count: c_int, scale: f32) -> C2Simplex {
    let mk = |rng: &mut Rng| C2sv {
        sA: vec_tame(rng, scale),
        sB: vec_tame(rng, scale),
        p: vec_tame(rng, scale),
        u: tame(rng, scale),
        iA: rng.below(4) as c_int,
        iB: rng.below(4) as c_int,
    };
    C2Simplex {
        verts: [mk(rng), mk(rng), mk(rng), mk(rng)],
        div: if rng.below(10) == 0 {
            0.0
        } else {
            tame(rng, scale).abs() + 0.25
        },
        count,
    }
}

// Function-pointer type aliases used by the test files.
pub type FnVV = unsafe extern "C" fn(C2v) -> C2v;
pub type FnVVV = unsafe extern "C" fn(C2v, C2v) -> C2v;
pub type FnVVF = unsafe extern "C" fn(C2v, C2v) -> f32;
pub type FnVF = unsafe extern "C" fn(C2v) -> f32;
pub type FnFFV = unsafe extern "C" fn(f32, f32) -> C2v;
pub type FnVfV = unsafe extern "C" fn(C2v, f32) -> C2v;
pub type FnVVVV = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
pub type FnRVV = unsafe extern "C" fn(C2r, C2v) -> C2v;
pub type FnXVV = unsafe extern "C" fn(C2x, C2v) -> C2v;
pub type FnR = unsafe extern "C" fn() -> C2r;
pub type FnX = unsafe extern "C" fn() -> C2x;
pub type FnBBVerts = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const std::ffi::c_void, c_int, *mut C2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut C2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
pub type FnSimplexVoid = unsafe extern "C" fn(*mut C2Simplex);
pub type FnSupport = unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int;
pub type FnWitness = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);
pub type FnGJK = unsafe extern "C" fn(
    *const std::ffi::c_void,
    c_int,
    *const C2x,
    *const std::ffi::c_void,
    c_int,
    *const C2x,
    *mut C2v,
    *mut C2v,
    c_int,
    *mut c_int,
    *mut C2GJKCache,
) -> f32;
pub type FnAABBtoAABB = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
pub type FnAABBtoCapsule = unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int;
pub type FnCapsuletoCapsule = unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int;
pub type FnCircletoCircle = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
pub type FnCircletoAABB = unsafe extern "C" fn(C2Circle, C2AABB) -> c_int;
pub type FnCircletoCapsule = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
pub type FnCollided = unsafe extern "C" fn(
    *const std::ffi::c_void,
    c_int,
    *const std::ffi::c_void,
    c_int,
) -> c_int;
pub type FnPtrFromParts =
    unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut std::ffi::c_void;
pub type FnOmniCollide = unsafe extern "C" fn(
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
) -> c_int;
