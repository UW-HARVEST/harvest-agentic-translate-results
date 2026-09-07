//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` through `libloading` and calls
//! every function only through its exported symbol, exactly as an external
//! consumer would.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};

const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI types (mirrors of the C declarations)
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
// Function-pointer types
// ---------------------------------------------------------------------------

pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVf = unsafe extern "C" fn(c2v) -> f32;
pub type FnVfV = unsafe extern "C" fn(c2v, f32) -> c2v;

/// One loaded shared library plus lazily-resolved symbols.
pub struct Lib {
    pub lib: Library,
    pub name: &'static str,
}

impl Lib {
    pub fn get<T: Copy>(&self, sym: &str) -> T {
        unsafe {
            let s: Symbol<T> = self
                .lib
                .get(sym.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", self.name, sym, e));
            *s
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                cands.push(p);
            }
        }
    }
    cands.sort();
    cands.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {:?} — build it with: \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libomni_manifold_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libomni_manifold_lib.so not found; run `cargo build --release` first");
}

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

/// Loads both libraries. Leaked so symbol pointers stay valid for the process.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<&'static Pair> = OnceLock::new();
    P.get_or_init(|| {
        if std::env::var_os("SEGV_REPORT").is_some() {
            install_segv_reporter();
        }
        if std::env::var_os("CHECK_SCRUB").is_some() {
            CHECK_SCRUB.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let cp = find_c_so();
        let rp = find_rust_so();
        unsafe {
            // RTLD_NOW, *not* the default RTLD_LAZY.
            //
            // With lazy binding, the first call to each of the C library's own
            // PLT entries runs `_dl_runtime_resolve_xsavec`, which spills the
            // whole vector register file several hundred bytes down the stack —
            // exactly where `c2GJK` puts the `c2Proxy` it never initialises for
            // `C2_TYPE_POLY` (ERRORS.md row 53). That turned the very first
            // AABB-vs-capsule call of a process into a different (and
            // irreproducible) result from every later one. Resolving eagerly at
            // load time removes the resolver from the call path entirely.
            let c = Library::from(
                libloading::os::unix::Library::open(Some(&cp), RTLD_NOW | RTLD_LOCAL)
                    .expect("load C .so"),
            );
            let r = Library::from(
                libloading::os::unix::Library::open(Some(&rp), RTLD_NOW | RTLD_LOCAL)
                    .expect("load Rust .so"),
            );
            Box::leak(Box::new(Pair {
                c: Lib { lib: c, name: "C" },
                r: Lib {
                    lib: r,
                    name: "Rust",
                },
            }))
        }
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// Bit-for-bit float equality (NaN payload and sign of zero included).
pub fn bits_eq_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

pub fn fmt_f32(x: f32) -> String {
    format!("{:e}(0x{:08x})", x, x.to_bits())
}

pub fn fmt_v(v: c2v) -> String {
    format!("({}, {})", fmt_f32(v.x), fmt_f32(v.y))
}

pub fn v_eq(a: c2v, b: c2v) -> bool {
    bits_eq_f32(a.x, b.x) && bits_eq_f32(a.y, b.y)
}

pub fn h_eq(a: c2h, b: c2h) -> bool {
    v_eq(a.n, b.n) && bits_eq_f32(a.d, b.d)
}

pub fn x_eq(a: c2x, b: c2x) -> bool {
    v_eq(a.p, b.p) && bits_eq_f32(a.r.c, b.r.c) && bits_eq_f32(a.r.s, b.r.s)
}

pub fn man_eq(a: &c2Manifold, b: &c2Manifold) -> bool {
    a.count == b.count
        && a.depths.iter().zip(b.depths.iter()).all(|(x, y)| bits_eq_f32(*x, *y))
        && a.contact_points
            .iter()
            .zip(b.contact_points.iter())
            .all(|(x, y)| v_eq(*x, *y))
        && v_eq(a.n, b.n)
}

pub fn fmt_man(m: &c2Manifold) -> String {
    format!(
        "count={} depths=[{}, {}] cp=[{}, {}] n={}",
        m.count,
        fmt_f32(m.depths[0]),
        fmt_f32(m.depths[1]),
        fmt_v(m.contact_points[0]),
        fmt_v(m.contact_points[1]),
        fmt_v(m.n)
    )
}

pub fn proxy_eq(a: &c2Proxy, b: &c2Proxy) -> bool {
    bits_eq_f32(a.radius, b.radius)
        && a.count == b.count
        && a.verts.iter().zip(b.verts.iter()).all(|(x, y)| v_eq(*x, *y))
}

pub fn sv_eq(a: &c2sv, b: &c2sv) -> bool {
    v_eq(a.sA, b.sA)
        && v_eq(a.sB, b.sB)
        && v_eq(a.p, b.p)
        && bits_eq_f32(a.u, b.u)
        && a.iA == b.iA
        && a.iB == b.iB
}

pub fn simplex_eq(a: &c2Simplex, b: &c2Simplex) -> bool {
    sv_eq(&a.a, &b.a)
        && sv_eq(&a.b, &b.b)
        && sv_eq(&a.c, &b.c)
        && sv_eq(&a.d, &b.d)
        && bits_eq_f32(a.div, b.div)
        && a.count == b.count
}

pub fn cache_eq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    bits_eq_f32(a.metric, b.metric)
        && a.count == b.count
        && a.iA == b.iA
        && a.iB == b.iB
        && bits_eq_f32(a.div, b.div)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

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
    /// Uniform in [0,1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo,hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// "Interesting" float: mostly small, sometimes special.
    pub fn nice(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => self.range(-1.0, 1.0),
            3 => self.range(-1e6, 1e6),
            4 => self.range(-1e-6, 1e-6),
            5 => (self.below(21) as i32 - 10) as f32,
            6 => (self.below(21) as i32 - 10) as f32 * 0.5,
            _ => self.range(-20.0, 20.0),
        }
    }
    /// Includes non-finite values.
    pub fn wild(&mut self) -> f32 {
        match self.below(24) {
            0 => f32::INFINITY,
            1 => f32::NEG_INFINITY,
            2 => f32::NAN,
            3 => -f32::NAN,
            4 => f32::from_bits(0x7fc0_1234), // qNaN, distinct payload
            5 => f32::from_bits(0xffab_cdef), // negative NaN, distinct payload
            6 => f32::MAX,
            7 => f32::MIN,
            8 => f32::MIN_POSITIVE,
            9 => f32::from_bits(1), // denormal
            10 => 1e30,
            11 => -1e30,
            _ => self.nice(),
        }
    }
    pub fn v(&mut self) -> c2v {
        c2v {
            x: self.nice(),
            y: self.nice(),
        }
    }
    pub fn wild_v(&mut self) -> c2v {
        c2v {
            x: self.wild(),
            y: self.wild(),
        }
    }
    /// Unit-ish rotation.
    pub fn rot(&mut self) -> c2r {
        let t = self.range(-3.2, 3.2);
        c2r {
            c: t.cos(),
            s: t.sin(),
        }
    }
    pub fn xform(&mut self) -> c2x {
        c2x {
            p: self.v(),
            r: self.rot(),
        }
    }
}

pub const NULL_V: *const c_void = std::ptr::null();

/// Zeroes ~32 KiB of stack *below* the caller's frame.
///
/// `c2MakeProxy` has no `C2_TYPE_POLY` case, so `c2GJK` leaves its `c2Proxy`
/// local completely uninitialised for a poly operand (see ERRORS.md row 53).
/// The C library therefore reads whatever the previous call at that stack depth
/// left behind, which makes it non-deterministic *against itself*: calling
/// `c2AABBtoCapsuleManifold` twice with identical arguments can return two
/// different manifolds.
///
/// To get a well-defined differential comparison we pin that indeterminate
/// memory to zero before every call. The Rust translation models the same local
/// with `std::mem::zeroed()`, so both libraries then observe the same proxy and
/// the comparison is meaningful and reproducible. (Zero is also what the C
/// library sees on a freshly-grown stack.)
/// The writes MUST be volatile: `std::hint::black_box` on a raw pointer does not
/// clobber memory, so LLVM happily deletes stores into an otherwise-dead local
/// buffer (verified — a `write_bytes` + `black_box(ptr)` version was a complete
/// no-op). `write_volatile` cannot be elided.
///
/// 16 KiB is far more than the ~1.5 KiB of stack the deepest C call chain
/// (`omni_manifold` → `c2Collide` → `c2AABBtoCapsuleManifold` →
/// `c2CapsuletoPolyManifold` → `c2GJK`) actually uses.
#[inline(never)]
pub fn scrub_stack() {
    const N: usize = 2048; // u64 units => 16 KiB
    let mut buf = [0u64; N];
    let p = buf.as_mut_ptr();
    for i in 0..N {
        unsafe { std::ptr::write_volatile(p.add(i), 0) };
    }
    SCRUB_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    SCRUB_LO.store(p as usize, std::sync::atomic::Ordering::Relaxed);
    SCRUB_HI.store(unsafe { p.add(N) } as usize, std::sync::atomic::Ordering::Relaxed);
    std::hint::black_box(p);
}

pub static SCRUB_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static SCRUB_LO: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub static SCRUB_HI: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Runs `f` on a freshly spawned thread.
///
/// A brand-new thread stack is freshly `mmap`ed and therefore zero-filled by the
/// kernel, which pins the C library's uninitialised `c2Proxy` (ERRORS.md row 53)
/// to zero — the same value the Rust translation models with `std::mem::zeroed()`.
/// Without this, results on the POLY path depend on whatever the *previous* test
/// left on the shared stack, and the C library is then non-deterministic against
/// itself (and can even fault, because a garbage `c2Proxy::count` makes
/// `c2Support` walk off the end of the stack).
#[track_caller]
pub fn fresh<F: FnOnce() + Send + 'static>(f: F) {
    let loc = std::panic::Location::caller();
    let name = format!("{}:{}", loc.file(), loc.line());
    // A unique, monotonically increasing stack size prevents glibc from handing
    // back a *cached* (already dirty) thread stack: a brand-new `mmap` is always
    // zero-filled, which pins the C library's uninitialised `c2Proxy`
    // (ERRORS.md row 53) to zero for the whole test.
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let extra = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) * 64 * 1024;
    let h = std::thread::Builder::new()
        .name(name)
        .stack_size(32 * 1024 * 1024 + extra)
        .spawn(move || {
            deep_scrub();
            f()
        })
        .expect("spawn test thread");
    if let Err(e) = h.join() {
        std::panic::resume_unwind(e);
    }
}

/// Counts non-zero bytes in the `n` bytes of stack immediately below the
/// caller's frame. Diagnostic companion to `scrub_stack`.
#[inline(never)]
pub fn stack_nonzero(n: usize) -> usize {
    let mut anchor = [0u64; 1];
    unsafe { std::ptr::write_volatile(anchor.as_mut_ptr(), 0) };
    let p = anchor.as_ptr() as *const u8;
    let mut nz = 0usize;
    let mut first = 0usize;
    let mut last = 0usize;
    for i in 1..n {
        unsafe {
            if std::ptr::read_volatile(p.sub(i)) != 0 {
                if nz == 0 {
                    first = i;
                }
                last = i;
                nz += 1;
            }
        }
    }
    let _ = (first, last);
    nz
}

/// Zeroes the stack the callee is about to use, then runs `f`.
///
/// `scrub_stack` and the FFI call must share the same frame base for the scrub
/// to be guaranteed to cover the callee's frames; doing both inside one
/// `#[inline(never)]` function is what guarantees that. (Calling `scrub_stack()`
/// and the FFI function as two separate statements in a larger function was
/// observed NOT to be reliable.)
#[inline(never)]
pub fn with_clean_stack<R>(f: impl FnOnce() -> R) -> R {
    scrub_stack();
    if CHECK_SCRUB.load(std::sync::atomic::Ordering::Relaxed) {
        let nz = stack_nonzero(8192);
        assert!(nz < 32, "scrub ineffective: {} nonzero bytes below frame", nz);
    }
    let r = f();
    std::hint::black_box(&r as *const R);
    r
}

pub static CHECK_SCRUB: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

// ---------------------------------------------------------------------------
// SIGSEGV forensics
// ---------------------------------------------------------------------------

#[repr(C)]
struct SigActionT {
    handler: usize,
    mask: [u64; 16],
    flags: c_int,
    _pad: c_int,
    restorer: usize,
}

extern "C" {
    fn sigaction(sig: c_int, act: *const SigActionT, old: *mut SigActionT) -> c_int;
}

extern "C" fn on_segv(_sig: c_int, info: *const u8, uctx: *const u8) {
    unsafe {
        let si_addr = *(info.add(16) as *const usize);
        let rip = *(uctx.add(168) as *const usize);
        let rsp = *(uctx.add(160) as *const usize);
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
        let attribute = |a: usize| -> String {
            for line in maps.lines() {
                if let Some((range, rest)) = line.split_once(' ') {
                    if let Some((lo, hi)) = range.split_once('-') {
                        let lo = usize::from_str_radix(lo, 16).unwrap_or(0);
                        let hi = usize::from_str_radix(hi, 16).unwrap_or(0);
                        if a >= lo && a < hi {
                            return format!("{} +0x{:x}", rest.trim(), a - lo);
                        }
                    }
                }
            }
            "unmapped".into()
        };
        eprintln!(
            "\n*** SIGSEGV si_addr=0x{:x} [{}]\n    rip=0x{:x} [{}]\n    rsp=0x{:x} [{}]",
            si_addr,
            attribute(si_addr),
            rip,
            attribute(rip),
            rsp,
            attribute(rsp)
        );
        eprintln!("    thread = {:?}", std::thread::current().name());
        // gregs[]: R8..R15, RDI, RSI, RBP(10), RBX, RDX, RAX(13), RCX, RSP(15), RIP(16)
        let rbp = *(uctx.add(40 + 10 * 8) as *const usize);
        let rax = *(uctx.add(40 + 13 * 8) as *const usize);
        eprintln!("    rbp=0x{:x} rax=0x{:x}", rbp, rax);
        // c2Support frame: -0x18 verts, -0x1c count, -0xc i, -0x8 dmax, -0x4 imax
        let verts = *((rbp - 0x18) as *const usize);
        let count = *((rbp - 0x1c) as *const i32);
        let i = *((rbp - 0xc) as *const i32);
        eprintln!(
            "    c2Support(verts=0x{:x} [{}], count={}, i={})  verts-rbp={}",
            verts,
            attribute(verts),
            count,
            i,
            (verts as isize) - (rbp as isize)
        );
        eprintln!(
            "    last scrub range = [0x{:x}, 0x{:x})  verts inside = {}",
            SCRUB_LO.load(std::sync::atomic::Ordering::Relaxed),
            SCRUB_HI.load(std::sync::atomic::Ordering::Relaxed),
            verts >= SCRUB_LO.load(std::sync::atomic::Ordering::Relaxed)
                && verts < SCRUB_HI.load(std::sync::atomic::Ordering::Relaxed)
        );
        eprint!("    proxy dump (verts-8 .. verts+72):");
        for k in 0..20i64 {
            let a = (verts as i64 - 8 + k * 4) as *const u32;
            let w = *a;
            eprint!(" {:08x}({:e})", w, f32::from_bits(w));
        }
        eprintln!();
        eprintln!("    scrub_count={}", SCRUB_COUNT.load(std::sync::atomic::Ordering::Relaxed));
    }
    std::process::exit(42);
}

/// Installs a SIGSEGV reporter (diagnostics only).
pub fn install_segv_reporter() {
    let act = SigActionT {
        handler: on_segv as *const () as usize,
        mask: [0; 16],
        flags: 4 | 0x0800_0000, // SA_SIGINFO | SA_ONSTACK
        _pad: 0,
        restorer: 0,
    };
    unsafe {
        sigaction(11, &act, std::ptr::null_mut());
    }
}

/// One-off deep scrub at thread entry: zeroes 1 MiB, far more than any C call
/// chain in this library reaches, so the whole test runs against a known-zero
/// stack even if the OS handed us recycled pages.
#[inline(never)]
fn deep_scrub() {
    const N: usize = 128 * 1024; // u64 units => 1 MiB
    let mut buf = vec![0u64; N];
    let p = buf.as_mut_ptr();
    for i in 0..N {
        unsafe { std::ptr::write_volatile(p.add(i), 0) };
    }
    std::hint::black_box(p);
    // and the actual stack region
    stack_scrub_1m();
}

#[inline(never)]
fn stack_scrub_1m() {
    const N: usize = 128 * 1024; // u64 units => 1 MiB on the stack
    let mut buf = [0u64; N];
    let p = buf.as_mut_ptr();
    for i in 0..N {
        unsafe { std::ptr::write_volatile(p.add(i), 0) };
    }
    std::hint::black_box(p);
}
