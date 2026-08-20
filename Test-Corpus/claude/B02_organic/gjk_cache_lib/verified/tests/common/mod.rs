//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes every exported symbol as a raw `extern "C"` fn
//! pointer. Nothing in the tests ever calls the Rust crate directly, so the
//! `#[no_mangle]` export wrappers are part of what is under test.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI types (must be layout-identical to c_src/src/lib.c)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
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
        c2Proxy { radius: 0.0, count: 0, verts: [c2v::default(); 8] }
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

/// C: `struct { c2sv a, b, c, d; float div; int count; }`
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct c2Simplex {
    pub verts: [c2sv; 4],
    pub div: f32,
    pub count: c_int,
}

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

/// Pins the layout assumptions the whole differential suite rests on.
/// Values verified against gcc with `offsetof` (see SYMBOLS.md).
pub fn assert_layout() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<c2v>(), 8, "c2v size");
    assert_eq!(size_of::<c2r>(), 8, "c2r size");
    assert_eq!(size_of::<c2x>(), 16, "c2x size");
    assert_eq!(size_of::<c2Circle>(), 12, "c2Circle size");
    assert_eq!(size_of::<c2AABB>(), 16, "c2AABB size");
    assert_eq!(size_of::<c2Capsule>(), 20, "c2Capsule size");
    assert_eq!(size_of::<c2GJKCache>(), 36, "c2GJKCache size");
    assert_eq!(size_of::<c2Proxy>(), 72, "c2Proxy size");
    assert_eq!(size_of::<c2sv>(), 36, "c2sv size");
    assert_eq!(size_of::<c2Simplex>(), 152, "c2Simplex size");
    for a in [
        align_of::<c2v>(),
        align_of::<c2GJKCache>(),
        align_of::<c2Proxy>(),
        align_of::<c2sv>(),
        align_of::<c2Simplex>(),
    ] {
        assert_eq!(a, 4, "alignment");
    }

    // Field offsets, mirroring the gcc `offsetof` dump.
    macro_rules! off {
        ($t:ty, $f:ident) => {{
            let u = std::mem::MaybeUninit::<$t>::uninit();
            let base = u.as_ptr() as usize;
            unsafe { (&raw const (*u.as_ptr()).$f) as usize - base }
        }};
    }
    assert_eq!(off!(c2GJKCache, metric), 0);
    assert_eq!(off!(c2GJKCache, count), 4);
    assert_eq!(off!(c2GJKCache, iA), 8);
    assert_eq!(off!(c2GJKCache, iB), 20);
    assert_eq!(off!(c2GJKCache, div), 32);
    assert_eq!(off!(c2Proxy, radius), 0);
    assert_eq!(off!(c2Proxy, count), 4);
    assert_eq!(off!(c2Proxy, verts), 8);
    assert_eq!(off!(c2sv, sA), 0);
    assert_eq!(off!(c2sv, sB), 8);
    assert_eq!(off!(c2sv, p), 16);
    assert_eq!(off!(c2sv, u), 24);
    assert_eq!(off!(c2sv, iA), 28);
    assert_eq!(off!(c2sv, iB), 32);
    // C members a/b/c/d live at 0/36/72/108 -> the `verts[4]` array must start
    // at 0, with `div` at 144 and `count` at 148.
    assert_eq!(off!(c2Simplex, verts), 0);
    assert_eq!(off!(c2Simplex, div), 144);
    assert_eq!(off!(c2Simplex, count), 148);
}

// ---------------------------------------------------------------------------
// Function-pointer table
// ---------------------------------------------------------------------------

pub type FnV = unsafe extern "C" fn(f32, f32) -> c2v;
pub type FnVsV = unsafe extern "C" fn(c2v, f32) -> c2v;
pub type FnVVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
pub type FnVVVV = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
pub type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
pub type FnVf = unsafe extern "C" fn(c2v) -> f32;
pub type FnVV = unsafe extern "C" fn(c2v) -> c2v;
pub type FnR = unsafe extern "C" fn() -> c2r;
pub type FnX = unsafe extern "C" fn() -> c2x;
pub type FnRVV = unsafe extern "C" fn(c2r, c2v) -> c2v;
pub type FnXVV = unsafe extern "C" fn(c2x, c2v) -> c2v;
pub type FnBBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
pub type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
pub type FnSimplexF = unsafe extern "C" fn(*mut c2Simplex) -> f32;
pub type FnSimplexV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
pub type FnSimplex = unsafe extern "C" fn(*mut c2Simplex);
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
pub type FnGjkCache = unsafe extern "C" fn(
    c_char,
    *mut c2v,
    *mut c2v,
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

/// Every symbol exported by the shared object, resolved once.
pub struct Api {
    pub name: &'static str,
    _lib: &'static libloading::Library,
    pub c2V: FnV,
    pub c2Mulvs: FnVsV,
    pub c2Maxv: FnVVV,
    pub c2Minv: FnVVV,
    pub c2Clampv: FnVVVV,
    pub c2Sub: FnVVV,
    pub c2Add: FnVVV,
    pub c2Dot: FnVVf,
    pub c2Det2: FnVVf,
    pub c2Len: FnVf,
    pub c2Neg: FnVV,
    pub c2Skew: FnVV,
    pub c2CCW90: FnVV,
    pub c2Div: FnVsV,
    pub c2Norm: FnVV,
    pub c2RotIdentity: FnR,
    pub c2xIdentity: FnX,
    pub c2Mulrv: FnRVV,
    pub c2MulrvT: FnRVV,
    pub c2Mulxv: FnXVV,
    pub c2BBVerts: FnBBVerts,
    pub c2MakeProxy: FnMakeProxy,
    pub c2GJKSimplexMetric: FnSimplexF,
    pub c22: FnSimplex,
    pub c23: FnSimplex,
    pub c2D: FnSimplexV,
    pub c2L: FnSimplexV,
    pub c2Support: FnSupport,
    pub c2Witness: FnWitness,
    pub c2GJK: FnGJK,
    pub gjk_cache: FnGjkCache,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_C_SO") {
        return PathBuf::from(p);
    }
    repo_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("GJK_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer whatever the current `cargo test` invocation just built. The test
    // binary lives in target/<profile>/deps/, so walk up to <profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe.parent().and_then(|p| p.parent()).map(PathBuf::from);
    if let Some(dir) = profile_dir {
        let cand = dir.join("libgjk_cache_lib.so");
        if cand.exists() {
            return cand;
        }
    }
    for p in ["target/debug/libgjk_cache_lib.so", "target/release/libgjk_cache_lib.so"] {
        let cand = repo_root().join(p);
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust .so not found; run `cargo build` or set GJK_RUST_SO");
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
    let lib: &'static libloading::Library = Box::leak(Box::new(
        libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {} ({:?}) failed: {e}", name, path)),
    ));
    macro_rules! sym {
        ($n:literal) => {{
            let s: libloading::Symbol<_> = lib
                .get($n)
                .unwrap_or_else(|e| {
                    panic!(
                        "{} is missing symbol {}: {e}",
                        name,
                        String::from_utf8_lossy(&$n[..$n.len() - 1])
                    )
                });
            *s
        }};
    }
    Api {
        name,
        _lib: lib,
        c2V: sym!(b"c2V\0"),
        c2Mulvs: sym!(b"c2Mulvs\0"),
        c2Maxv: sym!(b"c2Maxv\0"),
        c2Minv: sym!(b"c2Minv\0"),
        c2Clampv: sym!(b"c2Clampv\0"),
        c2Sub: sym!(b"c2Sub\0"),
        c2Add: sym!(b"c2Add\0"),
        c2Dot: sym!(b"c2Dot\0"),
        c2Det2: sym!(b"c2Det2\0"),
        c2Len: sym!(b"c2Len\0"),
        c2Neg: sym!(b"c2Neg\0"),
        c2Skew: sym!(b"c2Skew\0"),
        c2CCW90: sym!(b"c2CCW90\0"),
        c2Div: sym!(b"c2Div\0"),
        c2Norm: sym!(b"c2Norm\0"),
        c2RotIdentity: sym!(b"c2RotIdentity\0"),
        c2xIdentity: sym!(b"c2xIdentity\0"),
        c2Mulrv: sym!(b"c2Mulrv\0"),
        c2MulrvT: sym!(b"c2MulrvT\0"),
        c2Mulxv: sym!(b"c2Mulxv\0"),
        c2BBVerts: sym!(b"c2BBVerts\0"),
        c2MakeProxy: sym!(b"c2MakeProxy\0"),
        c2GJKSimplexMetric: sym!(b"c2GJKSimplexMetric\0"),
        c22: sym!(b"c22\0"),
        c23: sym!(b"c23\0"),
        c2D: sym!(b"c2D\0"),
        c2L: sym!(b"c2L\0"),
        c2Support: sym!(b"c2Support\0"),
        c2Witness: sym!(b"c2Witness\0"),
        c2GJK: sym!(b"c2GJK\0"),
        gjk_cache: sym!(b"gjk_cache\0"),
    }
}

/// The (C, Rust) pair under differential test.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

/// Loads both libraries. Both are leaked and cached for the process lifetime.
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| {
        assert_layout();
        unsafe {
            let c = load("C", &c_so_path());
            let r = load("Rust", &rust_so_path());
            Pair { c, r }
        }
    })
}

// ---------------------------------------------------------------------------
// Comparison policy
// ---------------------------------------------------------------------------
//
// STRICT (default, used for every NaN-free input): results must be
// bit-identical, including `+0.0` vs `-0.0`, subnormals, `±inf`, and
// hardware-generated NaNs. A hardware invalid-operation (`inf*0`, `0/0`,
// `inf-inf`) yields the x86 "indefinite" `0xffc00000` in *both* languages,
// because both emit the same SSE instruction, so generated NaNs are covered by
// STRICT too.
//
// SOFT (used only when an *input* already contains a NaN): two NaNs compare
// equal regardless of payload/sign. Rationale, established from the
// disassembly: `addss dst, src` returns the *destination* operand when both
// operands are NaN. Which of the two products lands in the destination register
// is a register-allocation choice, and it differs between gcc -O0 and LLVM:
//
//   C   (gcc -O0) c2Dot: mulss->%xmm1 (a.x*b.x), mulss->%xmm0 (a.y*b.y),
//                        addss %xmm1,%xmm0   => dest = a.y*b.y term
//   Rust (LLVM)   c2Dot: mulss->%xmm3 (a.y*b.y), mulss->%xmm0 (a.x*b.x),
//                        addss %xmm3,%xmm0   => dest = a.x*b.x term
//
// So with a.x*b.x = 0xffc00000 and a.y*b.y = 0x7fc00000 the two disagree on the
// payload while agreeing that the result is NaN. This is not a translation
// defect: recompiling the *C* at -O2 changes its payload too, so the payload is
// outside the ABI contract. Everything else stays bit-exact.

/// True when both sides are NaN (payload/sign ignored).
pub fn both_nan(c: f32, r: f32) -> bool {
    c.is_nan() && r.is_nan()
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Bit-exact float comparison (so `NaN == NaN`, `+0.0 != -0.0`).
#[track_caller]
pub fn eq_f32(ctx: &str, c: f32, r: f32) {
    if c.to_bits() != r.to_bits() {
        panic!(
            "{ctx}: f32 mismatch\n  C    = {c:?} (bits 0x{:08x})\n  Rust = {r:?} (bits 0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_v(ctx: &str, c: c2v, r: c2v) {
    if c.x.to_bits() != r.x.to_bits() || c.y.to_bits() != r.y.to_bits() {
        panic!(
            "{ctx}: c2v mismatch\n  C    = ({:?}, {:?}) [0x{:08x} 0x{:08x}]\n  \
             Rust = ({:?}, {:?}) [0x{:08x} 0x{:08x}]",
            c.x,
            c.y,
            c.x.to_bits(),
            c.y.to_bits(),
            r.x,
            r.y,
            r.x.to_bits(),
            r.y.to_bits()
        );
    }
}

#[track_caller]
pub fn eq_r(ctx: &str, c: c2r, r: c2r) {
    eq_f32(&format!("{ctx}.c"), c.c, r.c);
    eq_f32(&format!("{ctx}.s"), c.s, r.s);
}

#[track_caller]
pub fn eq_x(ctx: &str, c: c2x, r: c2x) {
    eq_v(&format!("{ctx}.p"), c.p, r.p);
    eq_r(&format!("{ctx}.r"), c.r, r.r);
}

#[track_caller]
pub fn eq_i(ctx: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "{ctx}: int mismatch (C={c}, Rust={r})");
}

/// Raw byte comparison — catches "wrote a field it should not have" bugs and
/// padding differences.
#[track_caller]
pub fn eq_bytes<T>(ctx: &str, c: &T, r: &T) {
    let n = std::mem::size_of::<T>();
    let cb = unsafe { std::slice::from_raw_parts(c as *const T as *const u8, n) };
    let rb = unsafe { std::slice::from_raw_parts(r as *const T as *const u8, n) };
    if cb != rb {
        let diff: Vec<usize> = (0..n).filter(|&i| cb[i] != rb[i]).collect();
        panic!(
            "{ctx}: {} of {} bytes differ at offsets {:?}\n  C    = {:02x?}\n  Rust = {:02x?}",
            diff.len(),
            n,
            diff,
            cb,
            rb
        );
    }
}

#[track_caller]
pub fn eq_simplex(ctx: &str, c: &c2Simplex, r: &c2Simplex) {
    eq_bytes(ctx, c, r);
}

#[track_caller]
pub fn eq_cache(ctx: &str, c: &c2GJKCache, r: &c2GJKCache) {
    eq_bytes(ctx, c, r);
}

#[track_caller]
pub fn eq_proxy(ctx: &str, c: &c2Proxy, r: &c2Proxy) {
    eq_bytes(ctx, c, r);
}

// --- SOFT variants: identical to the above except that NaN == NaN ------------

#[track_caller]
pub fn eq_f32_soft(ctx: &str, c: f32, r: f32) {
    if both_nan(c, r) {
        return;
    }
    eq_f32(ctx, c, r);
}

#[track_caller]
pub fn eq_v_soft(ctx: &str, c: c2v, r: c2v) {
    eq_f32_soft(&format!("{ctx}.x"), c.x, r.x);
    eq_f32_soft(&format!("{ctx}.y"), c.y, r.y);
}

#[track_caller]
pub fn eq_r_soft(ctx: &str, c: c2r, r: c2r) {
    eq_f32_soft(&format!("{ctx}.c"), c.c, r.c);
    eq_f32_soft(&format!("{ctx}.s"), c.s, r.s);
}

#[track_caller]
pub fn eq_proxy_soft(ctx: &str, c: &c2Proxy, r: &c2Proxy) {
    eq_f32_soft(&format!("{ctx}.radius"), c.radius, r.radius);
    eq_i(&format!("{ctx}.count"), c.count, r.count);
    for k in 0..8 {
        eq_v_soft(&format!("{ctx}.verts[{k}]"), c.verts[k], r.verts[k]);
    }
}

#[track_caller]
pub fn eq_sv_soft(ctx: &str, c: &c2sv, r: &c2sv) {
    eq_v_soft(&format!("{ctx}.sA"), c.sA, r.sA);
    eq_v_soft(&format!("{ctx}.sB"), c.sB, r.sB);
    eq_v_soft(&format!("{ctx}.p"), c.p, r.p);
    eq_f32_soft(&format!("{ctx}.u"), c.u, r.u);
    eq_i(&format!("{ctx}.iA"), c.iA, r.iA);
    eq_i(&format!("{ctx}.iB"), c.iB, r.iB);
}

#[track_caller]
pub fn eq_simplex_soft(ctx: &str, c: &c2Simplex, r: &c2Simplex) {
    for k in 0..4 {
        eq_sv_soft(&format!("{ctx}.verts[{k}]"), &c.verts[k], &r.verts[k]);
    }
    eq_f32_soft(&format!("{ctx}.div"), c.div, r.div);
    eq_i(&format!("{ctx}.count"), c.count, r.count);
}

#[track_caller]
pub fn eq_cache_soft(ctx: &str, c: &c2GJKCache, r: &c2GJKCache) {
    eq_f32_soft(&format!("{ctx}.metric"), c.metric, r.metric);
    eq_i(&format!("{ctx}.count"), c.count, r.count);
    for k in 0..3 {
        eq_i(&format!("{ctx}.iA[{k}]"), c.iA[k], r.iA[k]);
        eq_i(&format!("{ctx}.iB[{k}]"), c.iB[k], r.iB[k]);
    }
    eq_f32_soft(&format!("{ctx}.div"), c.div, r.div);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

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
    /// Uniform in [-1, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    /// Uniform in [-scale, scale).
    pub fn scaled(&mut self, scale: f32) -> f32 {
        self.unit() * scale
    }
    /// A "nasty" float: mostly ordinary values, sometimes a special one.
    pub fn nasty(&mut self) -> f32 {
        match self.below(16) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::NAN,
            5 => f32::MAX,
            6 => f32::MIN,
            7 => f32::MIN_POSITIVE,
            8 => -f32::MIN_POSITIVE,
            9 => f32::from_bits(1),          // subnormal
            10 => f32::from_bits(0x8000_0001), // -subnormal
            11 => 1.0,
            12 => -1.0,
            13 => self.scaled(1e30),
            14 => self.scaled(1e-30),
            _ => self.scaled(100.0),
        }
    }
    /// Like `nasty()` but **never** produces a NaN. Every extreme value that can
    /// be compared bit-exactly lives here: `±0`, `±inf`, subnormals, `±FLT_MAX`,
    /// `±FLT_MIN`. Used with STRICT comparison.
    pub fn nasty_no_nan(&mut self) -> f32 {
        match self.below(15) {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::NEG_INFINITY,
            4 => f32::MAX,
            5 => f32::MIN,
            6 => f32::MIN_POSITIVE,
            7 => -f32::MIN_POSITIVE,
            8 => f32::from_bits(1),
            9 => f32::from_bits(0x8000_0001),
            10 => 1.0,
            11 => -1.0,
            12 => self.scaled(1e30),
            13 => self.scaled(1e-30),
            _ => self.scaled(100.0),
        }
    }
    /// A random bit pattern reinterpreted as f32 (includes signalling NaNs).
    pub fn bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A random NON-NaN bit pattern reinterpreted as f32.
    pub fn bits_f32_no_nan(&mut self) -> f32 {
        loop {
            let f = f32::from_bits(self.next_u32());
            if !f.is_nan() {
                return f;
            }
        }
    }
    pub fn vec_scaled(&mut self, scale: f32) -> c2v {
        c2v { x: self.scaled(scale), y: self.scaled(scale) }
    }
    pub fn vec_nasty(&mut self) -> c2v {
        c2v { x: self.nasty(), y: self.nasty() }
    }
    pub fn vec_nasty_no_nan(&mut self) -> c2v {
        c2v { x: self.nasty_no_nan(), y: self.nasty_no_nan() }
    }
    /// One of the coordinate scales `c2GJK` behaves differently at.
    pub fn scale_choice(&mut self) -> f32 {
        match self.below(6) {
            0 => 1e-6,
            1 => 1e-3,
            2 => 1.0,
            3 => 1e3,
            4 => 1e5,
            _ => 1e7,
        }
    }
}

// ---------------------------------------------------------------------------
// Shape helpers
// ---------------------------------------------------------------------------

/// A shape plus its `C2_TYPE`, kept alive so its address can be handed to FFI.
#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    pub fn ty(&self) -> c_int {
        match self {
            Shape::Circle(_) => C2_TYPE_CIRCLE,
            Shape::Aabb(_) => C2_TYPE_AABB,
            Shape::Capsule(_) => C2_TYPE_CAPSULE,
        }
    }
    pub fn as_ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const c2Circle as *const c_void,
            Shape::Aabb(b) => b as *const c2AABB as *const c_void,
            Shape::Capsule(c) => c as *const c2Capsule as *const c_void,
        }
    }
    pub fn vert_count(&self) -> c_int {
        match self {
            Shape::Circle(_) => 1,
            Shape::Aabb(_) => 4,
            Shape::Capsule(_) => 2,
        }
    }
}

pub fn rand_circle(rng: &mut Rng, scale: f32) -> c2Circle {
    c2Circle { p: rng.vec_scaled(scale), r: rng.scaled(scale * 0.25).abs() }
}

pub fn rand_aabb(rng: &mut Rng, scale: f32) -> c2AABB {
    let a = rng.vec_scaled(scale);
    let b = rng.vec_scaled(scale);
    c2AABB {
        min: c2v { x: a.x.min(b.x), y: a.y.min(b.y) },
        max: c2v { x: a.x.max(b.x), y: a.y.max(b.y) },
    }
}

pub fn rand_capsule(rng: &mut Rng, scale: f32) -> c2Capsule {
    c2Capsule {
        a: rng.vec_scaled(scale),
        b: rng.vec_scaled(scale),
        r: rng.scaled(scale * 0.25).abs(),
    }
}

/// Borrow-friendly wrappers (`rand_x(&mut rng, rng.scale_choice())` cannot be
/// written directly because it would borrow `rng` twice).
impl Rng {
    pub fn vec_any_scale(&mut self) -> c2v {
        let s = self.scale_choice();
        self.vec_scaled(s)
    }
    pub fn circle_any(&mut self) -> c2Circle {
        let s = self.scale_choice();
        rand_circle(self, s)
    }
    pub fn aabb_any(&mut self) -> c2AABB {
        let s = self.scale_choice();
        rand_aabb(self, s)
    }
    pub fn capsule_any(&mut self) -> c2Capsule {
        let s = self.scale_choice();
        rand_capsule(self, s)
    }
    pub fn shape_any(&mut self) -> Shape {
        let t = self.below(3) as c_int;
        let s = self.scale_choice();
        rand_shape_of(self, t, s)
    }
    pub fn shape_of_any_scale(&mut self, ty: c_int) -> Shape {
        let s = self.scale_choice();
        rand_shape_of(self, ty, s)
    }
    pub fn simplex_any(&mut self, count: c_int) -> c2Simplex {
        let sc = self.scale_choice();
        rand_simplex(self, count, sc)
    }
    pub fn transform_any(&mut self) -> c2x {
        let s = self.scale_choice();
        rand_transform(self, s)
    }
}

pub fn rand_shape_of(rng: &mut Rng, ty: c_int, scale: f32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(rand_circle(rng, scale)),
        C2_TYPE_AABB => Shape::Aabb(rand_aabb(rng, scale)),
        _ => Shape::Capsule(rand_capsule(rng, scale)),
    }
}

/// Random `c2x` with a properly normalised rotation.
pub fn rand_transform(rng: &mut Rng, scale: f32) -> c2x {
    let ang = rng.unit() * std::f32::consts::PI;
    c2x { p: rng.vec_scaled(scale), r: c2r { c: ang.cos(), s: ang.sin() } }
}

/// Random `c2x` whose rotation is NOT normalised (scales/skews).
pub fn rand_transform_unnorm(rng: &mut Rng, scale: f32) -> c2x {
    c2x { p: rng.vec_scaled(scale), r: c2r { c: rng.scaled(2.0), s: rng.scaled(2.0) } }
}

/// Random simplex with the given `count`, `p`/`sA`/`sB`/`u`/`div` all filled.
pub fn rand_simplex(rng: &mut Rng, count: c_int, scale: f32) -> c2Simplex {
    let mut s = c2Simplex::default();
    for i in 0..4 {
        s.verts[i].sA = rng.vec_scaled(scale);
        s.verts[i].sB = rng.vec_scaled(scale);
        s.verts[i].p = rng.vec_scaled(scale);
        s.verts[i].u = rng.scaled(scale);
        s.verts[i].iA = rng.below(4) as c_int;
        s.verts[i].iB = rng.below(4) as c_int;
    }
    s.div = rng.scaled(scale);
    s.count = count;
    s
}

// ---------------------------------------------------------------------------
// c2GJK differential driver
// ---------------------------------------------------------------------------

/// Everything `c2GJK` can observably produce.
#[derive(Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: c2GJKCache,
}

pub struct GjkIn<'a> {
    pub a: &'a Shape,
    pub b: &'a Shape,
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub use_radius: c_int,
    /// `None` = pass a null cache pointer.
    pub cache: Option<c2GJKCache>,
    pub want_out_a: bool,
    pub want_out_b: bool,
    pub want_iters: bool,
    /// Override the type discriminant sent across FFI (for invalid-enum tests).
    pub type_a_override: Option<c_int>,
    pub type_b_override: Option<c_int>,
}

impl<'a> GjkIn<'a> {
    pub fn new(a: &'a Shape, b: &'a Shape) -> Self {
        GjkIn {
            a,
            b,
            ax: None,
            bx: None,
            use_radius: 1,
            cache: None,
            want_out_a: true,
            want_out_b: true,
            want_iters: true,
            type_a_override: None,
            type_b_override: None,
        }
    }
}

/// Calls `c2GJK` on one library, poisoning every out-parameter first so that
/// "did not write" is distinguishable from "wrote zero".
pub fn call_gjk(api: &Api, inp: &GjkIn) -> GjkOut {
    const POISON: c2v = c2v { x: -12345.678, y: 98765.43 };
    let mut a = POISON;
    let mut b = POISON;
    let mut iters: c_int = -777;
    let mut cache = inp.cache.unwrap_or(c2GJKCache {
        metric: -424242.0,
        count: -99,
        iA: [-7; 3],
        iB: [-8; 3],
        div: -1.5,
    });

    let ax_ptr = inp.ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bx_ptr = inp.bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let dist = unsafe {
        (api.c2GJK)(
            inp.a.as_ptr(),
            inp.type_a_override.unwrap_or_else(|| inp.a.ty()),
            ax_ptr,
            inp.b.as_ptr(),
            inp.type_b_override.unwrap_or_else(|| inp.b.ty()),
            bx_ptr,
            if inp.want_out_a { &mut a } else { std::ptr::null_mut() },
            if inp.want_out_b { &mut b } else { std::ptr::null_mut() },
            inp.use_radius,
            if inp.want_iters { &mut iters } else { std::ptr::null_mut() },
            if inp.cache.is_some() { &mut cache } else { std::ptr::null_mut() },
        )
    };
    GjkOut { dist, a, b, iters, cache }
}

/// Runs `c2GJK` on both libraries and asserts every output is bit-identical
/// (STRICT — use with NaN-free inputs).
#[track_caller]
pub fn diff_gjk(ctx: &str, p: &Pair, inp: &GjkIn) -> GjkOut {
    let oc = call_gjk(&p.c, inp);
    let or = call_gjk(&p.r, inp);
    eq_f32(&format!("{ctx}: dist"), oc.dist, or.dist);
    eq_v(&format!("{ctx}: outA"), oc.a, or.a);
    eq_v(&format!("{ctx}: outB"), oc.b, or.b);
    eq_i(&format!("{ctx}: iterations"), oc.iters, or.iters);
    eq_cache(&format!("{ctx}: cache"), &oc.cache, &or.cache);
    oc
}

/// SOFT variant (NaN == NaN) — use when an input already contains a NaN.
#[track_caller]
pub fn diff_gjk_soft(ctx: &str, p: &Pair, inp: &GjkIn) -> GjkOut {
    let oc = call_gjk(&p.c, inp);
    let or = call_gjk(&p.r, inp);
    eq_f32_soft(&format!("{ctx}: dist"), oc.dist, or.dist);
    eq_v_soft(&format!("{ctx}: outA"), oc.a, or.a);
    eq_v_soft(&format!("{ctx}: outB"), oc.b, or.b);
    eq_i(&format!("{ctx}: iterations"), oc.iters, or.iters);
    eq_cache_soft(&format!("{ctx}: cache"), &oc.cache, &or.cache);
    oc
}
