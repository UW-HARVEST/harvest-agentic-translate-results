//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls every function through
//! its exported symbol, exactly as an external C consumer would.  Nothing in the
//! Rust crate is called directly.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI-identical type definitions (transcribed from c_src)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CAPSULE: c_int = 0;
pub const C2_TYPE_CIRCLE: c_int = 1;
pub const C2_TYPE_AABB: c_int = 2;
pub const C2_TYPE_POLY: c_int = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Manifold {
    pub count: c_int,
    pub depths: [f32; 2],
    pub contact_points: [c2v; 2],
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2h {
    pub n: c2v,
    pub d: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2x {
    pub p: c2v,
    pub r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
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
#[derive(Copy, Clone, Debug, Default)]
pub struct c2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [c2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2sv {
    pub sA: c2v,
    pub sB: c2v,
    pub p: c2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct c2Simplex {
    pub a: c2sv,
    pub b: c2sv,
    pub c: c2sv,
    pub d: c2sv,
    pub div: f32,
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-exact equality of every float and int in a value.
pub trait BitEq {
    fn bit_eq(&self, other: &Self) -> bool;
    fn show(&self) -> String;
}

impl BitEq for f32 {
    fn bit_eq(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
    fn show(&self) -> String {
        format!("{:?}[{:#010x}]", self, self.to_bits())
    }
}

impl BitEq for c_int {
    fn bit_eq(&self, other: &Self) -> bool {
        self == other
    }
    fn show(&self) -> String {
        format!("{}", self)
    }
}

impl BitEq for c2v {
    fn bit_eq(&self, other: &Self) -> bool {
        self.x.bit_eq(&other.x) && self.y.bit_eq(&other.y)
    }
    fn show(&self) -> String {
        format!("({}, {})", self.x.show(), self.y.show())
    }
}

impl BitEq for c2r {
    fn bit_eq(&self, o: &Self) -> bool {
        self.c.bit_eq(&o.c) && self.s.bit_eq(&o.s)
    }
    fn show(&self) -> String {
        format!("c2r{{{}, {}}}", self.c.show(), self.s.show())
    }
}

impl BitEq for c2x {
    fn bit_eq(&self, o: &Self) -> bool {
        self.p.bit_eq(&o.p) && self.r.bit_eq(&o.r)
    }
    fn show(&self) -> String {
        format!("c2x{{p={}, r={}}}", self.p.show(), self.r.show())
    }
}

impl BitEq for c2h {
    fn bit_eq(&self, o: &Self) -> bool {
        self.n.bit_eq(&o.n) && self.d.bit_eq(&o.d)
    }
    fn show(&self) -> String {
        format!("c2h{{n={}, d={}}}", self.n.show(), self.d.show())
    }
}

impl BitEq for c2Manifold {
    fn bit_eq(&self, o: &Self) -> bool {
        self.count == o.count
            && self.depths[0].bit_eq(&o.depths[0])
            && self.depths[1].bit_eq(&o.depths[1])
            && self.contact_points[0].bit_eq(&o.contact_points[0])
            && self.contact_points[1].bit_eq(&o.contact_points[1])
            && self.n.bit_eq(&o.n)
    }
    fn show(&self) -> String {
        format!(
            "c2Manifold{{count={}, depths=[{}, {}], cp=[{}, {}], n={}}}",
            self.count,
            self.depths[0].show(),
            self.depths[1].show(),
            self.contact_points[0].show(),
            self.contact_points[1].show(),
            self.n.show()
        )
    }
}

impl BitEq for c2Proxy {
    fn bit_eq(&self, o: &Self) -> bool {
        self.radius.bit_eq(&o.radius)
            && self.count == o.count
            && (0..8).all(|i| self.verts[i].bit_eq(&o.verts[i]))
    }
    fn show(&self) -> String {
        let v: Vec<String> = self.verts.iter().map(|v| v.show()).collect();
        format!(
            "c2Proxy{{radius={}, count={}, verts=[{}]}}",
            self.radius.show(),
            self.count,
            v.join(", ")
        )
    }
}

impl BitEq for c2sv {
    fn bit_eq(&self, o: &Self) -> bool {
        self.sA.bit_eq(&o.sA)
            && self.sB.bit_eq(&o.sB)
            && self.p.bit_eq(&o.p)
            && self.u.bit_eq(&o.u)
            && self.iA == o.iA
            && self.iB == o.iB
    }
    fn show(&self) -> String {
        format!(
            "c2sv{{sA={}, sB={}, p={}, u={}, iA={}, iB={}}}",
            self.sA.show(),
            self.sB.show(),
            self.p.show(),
            self.u.show(),
            self.iA,
            self.iB
        )
    }
}

impl BitEq for c2Simplex {
    fn bit_eq(&self, o: &Self) -> bool {
        self.a.bit_eq(&o.a)
            && self.b.bit_eq(&o.b)
            && self.c.bit_eq(&o.c)
            && self.d.bit_eq(&o.d)
            && self.div.bit_eq(&o.div)
            && self.count == o.count
    }
    fn show(&self) -> String {
        format!(
            "c2Simplex{{\n  a={}\n  b={}\n  c={}\n  d={}\n  div={}, count={}}}",
            self.a.show(),
            self.b.show(),
            self.c.show(),
            self.d.show(),
            self.div.show(),
            self.count
        )
    }
}

impl BitEq for c2GJKCache {
    fn bit_eq(&self, o: &Self) -> bool {
        self.metric.bit_eq(&o.metric)
            && self.count == o.count
            && self.iA == o.iA
            && self.iB == o.iB
            && self.div.bit_eq(&o.div)
    }
    fn show(&self) -> String {
        format!(
            "c2GJKCache{{metric={}, count={}, iA={:?}, iB={:?}, div={}}}",
            self.metric.show(),
            self.count,
            self.iA,
            self.iB,
            self.div.show()
        )
    }
}

impl BitEq for c2AABB {
    fn bit_eq(&self, o: &Self) -> bool {
        self.min.bit_eq(&o.min) && self.max.bit_eq(&o.max)
    }
    fn show(&self) -> String {
        format!("c2AABB{{min={}, max={}}}", self.min.show(), self.max.show())
    }
}

impl BitEq for c2Circle {
    fn bit_eq(&self, o: &Self) -> bool {
        self.p.bit_eq(&o.p) && self.r.bit_eq(&o.r)
    }
    fn show(&self) -> String {
        format!("c2Circle{{p={}, r={}}}", self.p.show(), self.r.show())
    }
}

impl BitEq for c2Capsule {
    fn bit_eq(&self, o: &Self) -> bool {
        self.a.bit_eq(&o.a) && self.b.bit_eq(&o.b) && self.r.bit_eq(&o.r)
    }
    fn show(&self) -> String {
        format!(
            "c2Capsule{{a={}, b={}, r={}}}",
            self.a.show(),
            self.b.show(),
            self.r.show()
        )
    }
}

impl BitEq for u32 {
    fn bit_eq(&self, o: &Self) -> bool {
        self == o
    }
    fn show(&self) -> String {
        format!("{:#010x}", self)
    }
}

impl BitEq for bool {
    fn bit_eq(&self, o: &Self) -> bool {
        self == o
    }
    fn show(&self) -> String {
        format!("{}", self)
    }
}

impl<T: BitEq> BitEq for Option<T> {
    fn bit_eq(&self, o: &Self) -> bool {
        match (self, o) {
            (None, None) => true,
            (Some(a), Some(b)) => a.bit_eq(b),
            _ => false,
        }
    }
    fn show(&self) -> String {
        match self {
            None => "None".to_string(),
            Some(v) => format!("Some({})", v.show()),
        }
    }
}

impl<A: BitEq, B: BitEq, C: BitEq, D: BitEq, E: BitEq> BitEq for (A, B, C, D, E) {
    fn bit_eq(&self, o: &Self) -> bool {
        self.0.bit_eq(&o.0)
            && self.1.bit_eq(&o.1)
            && self.2.bit_eq(&o.2)
            && self.3.bit_eq(&o.3)
            && self.4.bit_eq(&o.4)
    }
    fn show(&self) -> String {
        format!(
            "({}, {}, {}, {}, {})",
            self.0.show(),
            self.1.show(),
            self.2.show(),
            self.3.show(),
            self.4.show()
        )
    }
}

impl BitEq for c2Poly {
    fn bit_eq(&self, o: &Self) -> bool {
        self.count == o.count
            && (0..8).all(|i| self.verts[i].bit_eq(&o.verts[i]))
            && (0..8).all(|i| self.norms[i].bit_eq(&o.norms[i]))
    }
    fn show(&self) -> String {
        let v: Vec<String> = self.verts.iter().map(|v| v.show()).collect();
        let n: Vec<String> = self.norms.iter().map(|v| v.show()).collect();
        format!(
            "c2Poly{{count={}, verts=[{}], norms=[{}]}}",
            self.count,
            v.join(", "),
            n.join(", ")
        )
    }
}

impl<T: BitEq> BitEq for Vec<T> {
    fn bit_eq(&self, o: &Self) -> bool {
        self.len() == o.len() && self.iter().zip(o.iter()).all(|(a, b)| a.bit_eq(b))
    }
    fn show(&self) -> String {
        let v: Vec<String> = self.iter().map(|x| x.show()).collect();
        format!("[{}]", v.join(", "))
    }
}

impl<A: BitEq, B: BitEq> BitEq for (A, B) {
    fn bit_eq(&self, o: &Self) -> bool {
        self.0.bit_eq(&o.0) && self.1.bit_eq(&o.1)
    }
    fn show(&self) -> String {
        format!("({}, {})", self.0.show(), self.1.show())
    }
}

impl<A: BitEq, B: BitEq, C: BitEq> BitEq for (A, B, C) {
    fn bit_eq(&self, o: &Self) -> bool {
        self.0.bit_eq(&o.0) && self.1.bit_eq(&o.1) && self.2.bit_eq(&o.2)
    }
    fn show(&self) -> String {
        format!("({}, {}, {})", self.0.show(), self.1.show(), self.2.show())
    }
}

impl<A: BitEq, B: BitEq, C: BitEq, D: BitEq> BitEq for (A, B, C, D) {
    fn bit_eq(&self, o: &Self) -> bool {
        self.0.bit_eq(&o.0) && self.1.bit_eq(&o.1) && self.2.bit_eq(&o.2) && self.3.bit_eq(&o.3)
    }
    fn show(&self) -> String {
        format!(
            "({}, {}, {}, {})",
            self.0.show(),
            self.1.show(),
            self.2.show(),
            self.3.show()
        )
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation has a parent dir")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let dir = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("lib") && s.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so in {}", dir.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libomni_manifold_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libomni_manifold_lib.so not found under {} — run `cargo build --release --offline`",
        base.display()
    );
}

/// `dlopen(path, RTLD_NOW | RTLD_LOCAL)` — see [`Libs::load`] for why eager
/// binding matters here.
unsafe fn open_eager(path: &std::path::Path) -> Library {
    use libloading::os::unix::{Library as UnixLibrary, RTLD_LOCAL, RTLD_NOW};
    let l = UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL)
        .unwrap_or_else(|e| panic!("dlopen(RTLD_NOW) {}: {e}", path.display()));
    Library::from(l)
}

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

impl Libs {
    fn load() -> Libs {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        // RTLD_NOW (eager binding) is REQUIRED, not just nice to have.
        //
        // With the default RTLD_LAZY the first call to `malloc@plt` (from
        // `ptr_from_parts`) or `sqrtf@plt` (from `c2Len`) traps into
        // `_dl_runtime_resolve`, which spills the whole SSE register file and
        // then runs `_dl_fixup`/`_dl_lookup_symbol_x` — hundreds of bytes of
        // stack, far more than the ~660 bytes between `ptr_from_parts` and
        // `c2GJK`'s uninitialised `c2Proxy` locals.  That one-time event
        // re-dirties the region the harness just scrubbed, *inside* the C
        // library where we cannot intervene, and the uninitialised-proxy read
        // then returns linker junk instead of zero.  Symptom before this fix:
        // exactly 1 divergence in 80 000 cases, always on the first
        // `omni_manifold` call of a test thread.
        unsafe {
            let c = open_eager(&c_path);
            let r = open_eager(&r_path);
            Libs { c, r, c_path, r_path }
        }
    }

    pub fn sym<T>(&self, side: Side, name: &str) -> Symbol<'_, T> {
        let lib = match side {
            Side::C => &self.c,
            Side::Rust => &self.r,
        };
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        unsafe {
            lib.get::<T>(&bytes)
                .unwrap_or_else(|e| panic!("symbol {name} missing from {side:?} .so: {e}"))
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    C,
    Rust,
}

static LIBS: std::sync::OnceLock<Libs> = std::sync::OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(Libs::load)
}

// ---------------------------------------------------------------------------
// Stack normalisation
// ---------------------------------------------------------------------------

/// Zeroes ~64 KiB of stack *below* the current frame.
///
/// `c2MakeProxy` in the C has no `C2_TYPE_POLY` case, so `c2GJK`'s two
/// `c2Proxy` locals stay **uninitialised** when a poly (or an out-of-range
/// type) is passed.  In a normal C program those stack bytes are still the
/// kernel's fresh zero pages, and the library then behaves as if the proxy were
/// `{radius: 0, count: 0, verts: all-zero}` — which is exactly what the Rust
/// translation materialises.  Verified against a standalone C `main()`:
/// `omni_manifold(AABB, CAPSULE)` there produces the Rust's bytes exactly.
///
/// Inside a `libloading` test harness the stack at that depth is *dirty* (it
/// holds leftover pointers, whose values also move with ASLR), so the C's UB
/// read would be non-deterministic run to run.  Scrubbing the stack back to
/// zero before every call restores the pristine-stack condition and makes the
/// differential comparison well defined.
///
/// Deepest observed C call chain is ~1.5 KiB
/// (`omni_manifold`→`c2Collide`→`c2AABBtoCapsuleManifold`→`c2CapsuletoPolyManifold`
/// →`c2GJK`→`c2MakeProxy`), so 64 KiB is a very wide margin.
/// IMPORTANT: this must be the **last** thing that runs before the FFI call —
/// anything in between (notably `dlsym`, which uses several KiB of stack) would
/// re-dirty the region.  That is why every wrapper below resolves its symbol
/// from a cache first, primes the allocator, and only then scrubs.
#[inline(never)]
fn scrub_stack_only() {
    const SCRUB: usize = 16 * 1024;
    let mut buf = [0u8; SCRUB];
    let p = buf.as_mut_ptr();
    unsafe {
        std::ptr::write_bytes(p, 0, SCRUB);
    }
    std::hint::black_box(p);
}

/// Populates glibc's tcache bin for 32-byte chunks.
///
/// `omni_manifold` → `ptr_from_parts` calls `malloc` **before** `c2Collide`
/// eventually reaches `c2GJK`, and `malloc`'s slow paths (`tcache_init`,
/// `sysmalloc`/`brk` heap growth) use far more stack than the ~660 bytes that
/// separate `ptr_from_parts` from `c2GJK`'s `c2Proxy` locals.  When that
/// happens the freshly scrubbed region is dirtied again *inside the C library*,
/// where the harness cannot intervene, and the uninitialised-proxy read stops
/// being all-zero.  (Observed exactly once in 20 000 mixed calls before this
/// mitigation.)
///
/// `c2Circle` (12 B), `c2AABB` (16 B) and `c2Capsule` (20 B) all round up to a
/// single 32-byte glibc chunk, i.e. one tcache bin, so pre-filling that bin (and
/// doing it *before* the scrub) guarantees both of the C's `malloc` calls take
/// the inlined `tcache_get` fast path, which touches almost no stack at all.
#[inline(never)]
fn prime_malloc_tcache() {
    // tcache holds 7 entries per bin; 4 covers the two allocations with margin.
    let mut keep: [Vec<u8>; 4] = [
        Vec::with_capacity(20),
        Vec::with_capacity(20),
        Vec::with_capacity(20),
        Vec::with_capacity(20),
    ];
    for v in keep.iter_mut() {
        v.push(0);
    }
    std::hint::black_box(keep.as_ptr());
    drop(keep); // → tcache
}

/// Normalises everything the C could observe besides its actual arguments.
/// Call immediately before an FFI call and nothing in between.
#[inline(always)]
pub fn scrub_stack() {
    prime_malloc_tcache();
    scrub_stack_only();
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Runs `f` against the C `.so` and the Rust `.so` and asserts bit-equality.
/// `label` identifies the case in the failure message.
pub fn diff<R, F>(label: &str, mut f: F) -> R
where
    R: BitEq,
    F: FnMut(Side) -> R,
{
    let c = f(Side::C);
    let r = f(Side::Rust);
    if !c.bit_eq(&r) {
        panic!(
            "DIVERGENCE [{label}]\n    C    = {}\n    Rust = {}",
            c.show(),
            r.show()
        );
    }
    c
}

/// Like [`diff`] but collects up to `max_report` failures over an iteration and
/// reports them all at once.
pub struct DiffAccum {
    name: &'static str,
    failures: Vec<String>,
    checked: usize,
}

impl DiffAccum {
    pub fn new(name: &'static str) -> Self {
        DiffAccum {
            name,
            failures: Vec::new(),
            checked: 0,
        }
    }

    pub fn check<R, F>(&mut self, label: impl std::fmt::Display, mut f: F)
    where
        R: BitEq,
        F: FnMut(Side) -> R,
    {
        self.checked += 1;
        let c = f(Side::C);
        let r = f(Side::Rust);
        if !c.bit_eq(&r) && self.failures.len() < 12 {
            self.failures.push(format!(
                "  [{label}]\n     C    = {}\n     Rust = {}",
                c.show(),
                r.show()
            ));
        } else if !c.bit_eq(&r) {
            // still count it
            self.failures.push(String::new());
        }
    }

    pub fn finish(self) {
        let real: Vec<&String> = self.failures.iter().filter(|s| !s.is_empty()).collect();
        if !self.failures.is_empty() {
            let mut msg = format!(
                "{}: {} of {} cases diverged\n",
                self.name,
                self.failures.len(),
                self.checked
            );
            for f in real {
                msg.push_str(f);
                msg.push('\n');
            }
            panic!("{}", msg);
        }
        assert!(self.checked > 0, "{}: no cases were checked", self.name);
        eprintln!("{}: {} cases OK", self.name, self.checked);
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xoshiro-ish; fixed seed per test)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0,1).
    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [-mag, mag].
    #[inline]
    pub fn sym(&mut self, mag: f32) -> f32 {
        (self.unit() * 2.0 - 1.0) * mag
    }
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    #[inline]
    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// A "nice" coordinate: small magnitude, sometimes exactly an integer or a
    /// half-integer so that `==`/`<=` boundary branches are actually hit.
    pub fn coord(&mut self) -> f32 {
        match self.below(8) {
            0 => self.below(9) as f32 - 4.0,          // integer in [-4,4]
            1 => (self.below(17) as f32 - 8.0) * 0.5, // half integer
            2 => 0.0,
            3 => self.sym(1.0),
            4 => self.sym(10.0),
            5 => self.sym(100.0),
            6 => self.sym(0.001),
            _ => self.sym(3.0),
        }
    }

    /// A radius: mostly small positive, sometimes exactly 0 or negative.
    pub fn radius(&mut self) -> f32 {
        match self.below(10) {
            0 => 0.0,
            1 => -0.0,
            2 => self.sym(2.0), // may be negative — C never validates
            3 => self.below(5) as f32,
            4 => 0.5,
            _ => self.unit() * 4.0,
        }
    }

    /// Any f32 bit pattern (NaN, inf, subnormal included).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A float drawn from a table of interesting special values, or a random
    /// coordinate.
    pub fn special(&mut self) -> f32 {
        const SPECIALS: [f32; 18] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::MAX,
            f32::MIN,
            f32::MIN_POSITIVE,
            f32::EPSILON,
            -f32::EPSILON,
            1.0e-6,
            -1.0e-6,
            1.0e8,
            -1.0e8,
            0.5,
            -0.5,
        ];
        match self.below(3) {
            0 => SPECIALS[self.below(SPECIALS.len() as u32) as usize],
            1 => f32::from_bits(0x7f80_0000 | self.next_u32() & 0x807f_ffff), // NaN family
            _ => self.coord(),
        }
    }

    pub fn vec(&mut self) -> c2v {
        c2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn any_vec(&mut self) -> c2v {
        c2v {
            x: self.any_f32(),
            y: self.any_f32(),
        }
    }
    pub fn special_vec(&mut self) -> c2v {
        c2v {
            x: self.special(),
            y: self.special(),
        }
    }
    pub fn circle(&mut self) -> c2Circle {
        c2Circle {
            p: self.vec(),
            r: self.radius(),
        }
    }
    pub fn aabb(&mut self) -> c2AABB {
        let a = self.vec();
        let b = self.vec();
        match self.below(8) {
            0 => c2AABB { min: a, max: a },  // degenerate
            1 => c2AABB { min: b, max: a },  // possibly inverted
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
        let a = self.vec();
        let b = if self.below(8) == 0 { a } else { self.vec() };
        c2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
    pub fn rot(&mut self) -> c2r {
        match self.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 1.0 },
            2 => c2r { c: -1.0, s: 0.0 },
            3 => {
                let t = self.sym(std::f32::consts::PI);
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
            4 => c2r {
                c: self.coord(),
                s: self.coord(),
            },
            _ => {
                let t = self.unit() * std::f32::consts::TAU;
                c2r {
                    c: t.cos(),
                    s: t.sin(),
                }
            }
        }
    }
    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.vec(),
            r: self.rot(),
        }
    }
    /// A random convex CCW polygon with `count` vertices (normals computed with
    /// the library's own `c2Norms`, per side).
    pub fn convex_poly_verts(&mut self, count: usize) -> [c2v; 8] {
        let mut out = [c2v::default(); 8];
        let cx = self.sym(3.0);
        let cy = self.sym(3.0);
        let rad = 0.5 + self.unit() * 3.0;
        // Sorted angles ⇒ convex.
        let mut angles: Vec<f32> = (0..count)
            .map(|i| {
                i as f32 / count as f32 * std::f32::consts::TAU + self.unit() * 0.3
            })
            .collect();
        angles.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (i, t) in angles.iter().enumerate() {
            out[i] = c2v {
                x: cx + rad * t.cos(),
                y: cy + rad * t.sin(),
            };
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Typed wrappers — one per exported symbol, dispatched per side.
// ---------------------------------------------------------------------------

macro_rules! fn1 {
    ($name:ident, $sym:literal, ($($an:ident : $at:ty),*) -> $rt:ty) => {
        pub fn $name(side: Side, $($an: $at),*) -> $rt {
            type Raw = unsafe extern "C" fn($($at),*) -> $rt;
            static CACHE: std::sync::OnceLock<[Raw; 2]> = std::sync::OnceLock::new();
            let tbl = CACHE.get_or_init(|| {
                let c: Symbol<Raw> = libs().sym(Side::C, $sym);
                let r: Symbol<Raw> = libs().sym(Side::Rust, $sym);
                [*c, *r]
            });
            let f = tbl[side as usize];
            scrub_stack();
            unsafe { f($($an),*) }
        }
    };
}

fn1!(c2V, "c2V", (x: f32, y: f32) -> c2v);
fn1!(c2Mulvs, "c2Mulvs", (a: c2v, b: f32) -> c2v);
fn1!(c2Maxv, "c2Maxv", (a: c2v, b: c2v) -> c2v);
fn1!(c2Minv, "c2Minv", (a: c2v, b: c2v) -> c2v);
fn1!(c2Clampv, "c2Clampv", (a: c2v, lo: c2v, hi: c2v) -> c2v);
fn1!(c2Sub, "c2Sub", (a: c2v, b: c2v) -> c2v);
fn1!(c2Dot, "c2Dot", (a: c2v, b: c2v) -> f32);
fn1!(c2Dist, "c2Dist", (h: c2h, p: c2v) -> f32);
fn1!(c2PlaneAt, "c2PlaneAt", (p: *const c2Poly, i: c_int) -> c2h);
fn1!(c2RotIdentity, "c2RotIdentity", () -> c2r);
fn1!(c2xIdentity, "c2xIdentity", () -> c2x);
fn1!(c2BBVerts, "c2BBVerts", (out: *mut c2v, bb: *mut c2AABB) -> ());
fn1!(c2MakeProxy, "c2MakeProxy", (shape: *const c_void, t: c_int, p: *mut c2Proxy) -> ());
fn1!(c2Len, "c2Len", (a: c2v) -> f32);
fn1!(c2Det2, "c2Det2", (a: c2v, b: c2v) -> f32);
fn1!(c2GJKSimplexMetric, "c2GJKSimplexMetric", (s: *mut c2Simplex) -> f32);
fn1!(c2Mulrv, "c2Mulrv", (a: c2r, b: c2v) -> c2v);
fn1!(c2MulrvT, "c2MulrvT", (a: c2r, b: c2v) -> c2v);
fn1!(c2Add, "c2Add", (a: c2v, b: c2v) -> c2v);
fn1!(c2Mulxv, "c2Mulxv", (a: c2x, b: c2v) -> c2v);
fn1!(c2MulxvT, "c2MulxvT", (a: c2x, b: c2v) -> c2v);
fn1!(c2Intersect, "c2Intersect", (a: c2v, b: c2v, da: f32, db: f32) -> c2v);
fn1!(c2Div, "c2Div", (a: c2v, b: f32) -> c2v);
fn1!(c2Norm, "c2Norm", (a: c2v) -> c2v);
fn1!(c2Neg, "c2Neg", (a: c2v) -> c2v);
fn1!(c2CCW90, "c2CCW90", (a: c2v) -> c2v);
fn1!(c22, "c22", (s: *mut c2Simplex) -> ());
fn1!(c23, "c23", (s: *mut c2Simplex) -> ());
fn1!(c2Skew, "c2Skew", (a: c2v) -> c2v);
fn1!(c2D, "c2D", (s: *mut c2Simplex) -> c2v);
fn1!(c2Support, "c2Support", (verts: *const c2v, count: c_int, d: c2v) -> c_int);
fn1!(c2Witness, "c2Witness", (s: *mut c2Simplex, a: *mut c2v, b: *mut c2v) -> ());
fn1!(c2L, "c2L", (s: *mut c2Simplex) -> c2v);
fn1!(c2Absv, "c2Absv", (a: c2v) -> c2v);
fn1!(c2Norms, "c2Norms", (verts: *mut c2v, norms: *mut c2v, count: c_int) -> ());
fn1!(c2Collide, "c2Collide", (a: *const c_void, ta: c_int, b: *const c_void, tb: c_int, m: *mut c2Manifold) -> ());
fn1!(ptr_from_parts, "ptr_from_parts", (t: c_int, a: f32, b: f32, c: f32, d: f32, e: f32) -> *mut c_void);

#[allow(clippy::too_many_arguments)]
pub fn c2GJK(
    side: Side,
    A: *const c_void,
    typeA: c_int,
    ax: *const c2x,
    B: *const c_void,
    typeB: c_int,
    bx: *const c2x,
    outA: *mut c2v,
    outB: *mut c2v,
    use_radius: c_int,
    iterations: *mut c_int,
    cache: *mut c2GJKCache,
) -> f32 {
    type F = unsafe extern "C" fn(
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
    static CACHE: std::sync::OnceLock<[F; 2]> = std::sync::OnceLock::new();
    let tbl = CACHE.get_or_init(|| {
        let c: Symbol<F> = libs().sym(Side::C, "c2GJK");
        let r: Symbol<F> = libs().sym(Side::Rust, "c2GJK");
        [*c, *r]
    });
    let f = tbl[side as usize];
    scrub_stack();
    unsafe {
        f(
            A, typeA, ax, B, typeB, bx, outA, outB, use_radius, iterations, cache,
        )
    }
}

fn1!(c2CircletoCircleManifold, "c2CircletoCircleManifold", (a: c2Circle, b: c2Circle, m: *mut c2Manifold) -> ());
fn1!(c2CircletoAABBManifold, "c2CircletoAABBManifold", (a: c2Circle, b: c2AABB, m: *mut c2Manifold) -> ());
fn1!(c2CircletoCapsuleManifold, "c2CircletoCapsuleManifold", (a: c2Circle, b: c2Capsule, m: *mut c2Manifold) -> ());
fn1!(c2AABBtoAABBManifold, "c2AABBtoAABBManifold", (a: c2AABB, b: c2AABB, m: *mut c2Manifold) -> ());
fn1!(c2CapsuletoPolyManifold, "c2CapsuletoPolyManifold", (a: c2Capsule, b: *const c2Poly, bx: *const c2x, m: *mut c2Manifold) -> ());
fn1!(c2AABBtoCapsuleManifold, "c2AABBtoCapsuleManifold", (a: c2AABB, b: c2Capsule, m: *mut c2Manifold) -> ());
fn1!(c2CapsuletoCapsuleManifold, "c2CapsuletoCapsuleManifold", (a: c2Capsule, b: c2Capsule, m: *mut c2Manifold) -> ());

#[allow(clippy::too_many_arguments)]
pub fn omni_manifold(
    side: Side,
    m: *mut c2Manifold,
    type_a: c_int,
    a1: f32,
    a2: f32,
    a3: f32,
    a4: f32,
    a5: f32,
    type_b: c_int,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
) {
    type F = unsafe extern "C" fn(
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
    static CACHE: std::sync::OnceLock<[F; 2]> = std::sync::OnceLock::new();
    let tbl = CACHE.get_or_init(|| {
        let c: Symbol<F> = libs().sym(Side::C, "omni_manifold");
        let r: Symbol<F> = libs().sym(Side::Rust, "omni_manifold");
        [*c, *r]
    });
    let f = tbl[side as usize];
    scrub_stack();
    unsafe {
        f(m, type_a, a1, a2, a3, a4, a5, type_b, b1, b2, b3, b4, b5);
    }
}

// ---------------------------------------------------------------------------
// Convenience: sentinel-seeded manifold so that "untouched" fields are visible
// ---------------------------------------------------------------------------

pub const SENTINEL_MANIFOLD: c2Manifold = c2Manifold {
    count: -12345,
    depths: [-7.5, 42.25],
    contact_points: [
        c2v { x: 11.0, y: -13.0 },
        c2v { x: -17.0, y: 19.0 },
    ],
    n: c2v { x: 0.25, y: -0.75 },
};

/// Calls a manifold producer on a sentinel-seeded manifold and returns it.
pub fn with_sentinel<F: FnOnce(*mut c2Manifold)>(f: F) -> c2Manifold {
    let mut m = SENTINEL_MANIFOLD;
    f(&mut m as *mut c2Manifold);
    m
}

// ---------------------------------------------------------------------------
// Shape abstraction for driving c2GJK / c2Collide generically
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Ci(c2Circle),
    Bb(c2AABB),
    Ca(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> c_int {
        match self {
            Shape::Ci(_) => C2_TYPE_CIRCLE,
            Shape::Bb(_) => C2_TYPE_AABB,
            Shape::Ca(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn as_ptr(&self) -> *const c_void {
        match self {
            Shape::Ci(c) => c as *const c2Circle as *const c_void,
            Shape::Bb(b) => b as *const c2AABB as *const c_void,
            Shape::Ca(c) => c as *const c2Capsule as *const c_void,
        }
    }
    /// The five scalars `ptr_from_parts` would use for this shape.
    pub fn parts(&self) -> (f32, f32, f32, f32, f32) {
        match self {
            Shape::Ci(c) => (c.p.x, c.p.y, c.r, 0.0, 0.0),
            Shape::Bb(b) => (b.min.x, b.min.y, b.max.x, b.max.y, 0.0),
            Shape::Ca(c) => (c.a.x, c.a.y, c.b.x, c.b.y, c.r),
        }
    }
    pub fn translate(&self, dx: f32, dy: f32) -> Shape {
        match *self {
            Shape::Ci(c) => Shape::Ci(c2Circle {
                p: c2v {
                    x: c.p.x + dx,
                    y: c.p.y + dy,
                },
                r: c.r,
            }),
            Shape::Bb(b) => Shape::Bb(c2AABB {
                min: c2v {
                    x: b.min.x + dx,
                    y: b.min.y + dy,
                },
                max: c2v {
                    x: b.max.x + dx,
                    y: b.max.y + dy,
                },
            }),
            Shape::Ca(c) => Shape::Ca(c2Capsule {
                a: c2v {
                    x: c.a.x + dx,
                    y: c.a.y + dy,
                },
                b: c2v {
                    x: c.b.x + dx,
                    y: c.b.y + dy,
                },
                r: c.r,
            }),
        }
    }
}

impl BitEq for Shape {
    fn bit_eq(&self, o: &Self) -> bool {
        match (self, o) {
            (Shape::Ci(a), Shape::Ci(b)) => a.bit_eq(b),
            (Shape::Bb(a), Shape::Bb(b)) => a.bit_eq(b),
            (Shape::Ca(a), Shape::Ca(b)) => a.bit_eq(b),
            _ => false,
        }
    }
    fn show(&self) -> String {
        match self {
            Shape::Ci(c) => c.show(),
            Shape::Bb(b) => b.show(),
            Shape::Ca(c) => c.show(),
        }
    }
}

impl Rng {
    pub fn shape(&mut self, kind: u32) -> Shape {
        match kind {
            0 => Shape::Ci(self.circle()),
            1 => Shape::Bb(self.aabb()),
            _ => Shape::Ca(self.capsule()),
        }
    }
    /// A "well-formed" shape: finite, non-degenerate, positive radius.
    pub fn nice_shape(&mut self, kind: u32) -> Shape {
        match kind {
            0 => Shape::Ci(c2Circle {
                p: c2v {
                    x: self.sym(2.0),
                    y: self.sym(2.0),
                },
                r: 0.25 + self.unit() * 1.5,
            }),
            1 => {
                let cx = self.sym(2.0);
                let cy = self.sym(2.0);
                let ex = 0.25 + self.unit() * 1.5;
                let ey = 0.25 + self.unit() * 1.5;
                Shape::Bb(c2AABB {
                    min: c2v {
                        x: cx - ex,
                        y: cy - ey,
                    },
                    max: c2v {
                        x: cx + ex,
                        y: cy + ey,
                    },
                })
            }
            _ => Shape::Ca(c2Capsule {
                a: c2v {
                    x: self.sym(2.0),
                    y: self.sym(2.0),
                },
                b: c2v {
                    x: self.sym(2.0),
                    y: self.sym(2.0),
                },
                r: 0.25 + self.unit() * 1.0,
            }),
        }
    }
    /// A shape with only special/non-finite coordinates.
    pub fn special_shape(&mut self, kind: u32) -> Shape {
        match kind {
            0 => Shape::Ci(c2Circle {
                p: self.special_vec(),
                r: self.special(),
            }),
            1 => Shape::Bb(c2AABB {
                min: self.special_vec(),
                max: self.special_vec(),
            }),
            _ => Shape::Ca(c2Capsule {
                a: self.special_vec(),
                b: self.special_vec(),
                r: self.special(),
            }),
        }
    }
}

/// Full observable result of one `c2GJK` invocation.
#[derive(Copy, Clone)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iter: c_int,
    pub cache: c2GJKCache,
}

impl BitEq for GjkOut {
    fn bit_eq(&self, o: &Self) -> bool {
        self.dist.bit_eq(&o.dist)
            && self.a.bit_eq(&o.a)
            && self.b.bit_eq(&o.b)
            && self.iter == o.iter
            && self.cache.bit_eq(&o.cache)
    }
    fn show(&self) -> String {
        format!(
            "GjkOut{{dist={}, a={}, b={}, iter={}, cache={}}}",
            self.dist.show(),
            self.a.show(),
            self.b.show(),
            self.iter,
            self.cache.show()
        )
    }
}

/// Sentinel values so that "not written" is distinguishable from "written 0".
pub const OUT_SENTINEL_A: c2v = c2v { x: 1234.5, y: -6789.5 };
pub const OUT_SENTINEL_B: c2v = c2v { x: -2468.25, y: 1357.75 };
pub const ITER_SENTINEL: c_int = -777;

pub struct GjkArgs {
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub use_radius: c_int,
    pub want_a: bool,
    pub want_b: bool,
    pub want_iter: bool,
    pub cache: Option<c2GJKCache>,
}

impl Default for GjkArgs {
    fn default() -> Self {
        GjkArgs {
            ax: None,
            bx: None,
            use_radius: 0,
            want_a: true,
            want_b: true,
            want_iter: true,
            cache: None,
        }
    }
}

/// Drives `c2GJK` through the `.so` on one side and collects every output.
/// `pa`/`pb` are raw shape pointers so that `C2_TYPE_POLY` can be passed too.
#[allow(clippy::too_many_arguments)]
pub fn run_gjk_raw(
    side: Side,
    pa: *const c_void,
    ta: c_int,
    pb: *const c_void,
    tb: c_int,
    args: &GjkArgs,
) -> GjkOut {
    let mut a = OUT_SENTINEL_A;
    let mut b = OUT_SENTINEL_B;
    let mut it: c_int = ITER_SENTINEL;
    let mut cache = args.cache.unwrap_or_default();
    let axp = match &args.ax {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let bxp = match &args.bx {
        Some(x) => x as *const c2x,
        None => std::ptr::null(),
    };
    let dist = c2GJK(
        side,
        pa,
        ta,
        axp,
        pb,
        tb,
        bxp,
        if args.want_a { &mut a } else { std::ptr::null_mut() },
        if args.want_b { &mut b } else { std::ptr::null_mut() },
        args.use_radius,
        if args.want_iter {
            &mut it
        } else {
            std::ptr::null_mut()
        },
        match args.cache {
            Some(_) => &mut cache,
            None => std::ptr::null_mut(),
        },
    );
    GjkOut {
        dist,
        a,
        b,
        iter: it,
        cache,
    }
}

pub fn run_gjk(side: Side, sa: &Shape, sb: &Shape, args: &GjkArgs) -> GjkOut {
    run_gjk_raw(side, sa.as_ptr(), sa.ty(), sb.as_ptr(), sb.ty(), args)
}

/// Builds a `c2Poly` with `count` verts and library-computed normals (using the
/// given side's own `c2Norms`, so both sides get their own normals — used only
/// where we want an *identical* poly, so we always take the C side's).
pub fn make_poly(verts: &[c2v; 8], count: c_int) -> c2Poly {
    let mut p = c2Poly {
        count,
        verts: *verts,
        norms: [c2v::default(); 8],
    };
    let mut v = p.verts;
    let mut n = [c2v::default(); 8];
    c2Norms(Side::C, v.as_mut_ptr(), n.as_mut_ptr(), count);
    p.norms = n;
    p
}
