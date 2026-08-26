//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and every
//! call goes through `dlsym`, so the Rust `#[no_mangle] extern "C"` export
//! wrappers and their ABI are exercised exactly like an external C consumer
//! would exercise them. No Rust function is ever called directly.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// ABI types (mirror of the C typedefs in c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: c_int = 0;
pub const C2_TYPE_AABB: c_int = 1;
pub const C2_TYPE_CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Aabb {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2GJKCache {
    pub metric: f32,
    pub count: c_int,
    pub iA: [c_int; 3],
    pub iB: [c_int; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: c_int,
    pub verts: [C2v; 8],
}

impl Default for C2Proxy {
    fn default() -> Self {
        C2Proxy {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2sv {
    pub sA: C2v,
    pub sB: C2v,
    pub p: C2v,
    pub u: f32,
    pub iA: c_int,
    pub iB: c_int,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
/// The four `c2sv` are contiguous (the C walks them with `c2sv* v = &s->a;`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct C2Simplex {
    pub verts: [C2sv; 4],
    pub div: f32,
    pub count: c_int,
}

// Compile-time layout assertions: if any of these fire, the harness types do
// not match the C struct layout and every comparison would be meaningless.
const _: () = {
    assert!(size_of::<C2v>() == 8);
    assert!(size_of::<C2r>() == 8);
    assert!(size_of::<C2x>() == 16);
    assert!(size_of::<C2Circle>() == 12);
    assert!(size_of::<C2Aabb>() == 16);
    assert!(size_of::<C2Capsule>() == 20);
    assert!(size_of::<C2GJKCache>() == 36);
    assert!(size_of::<C2Proxy>() == 72);
    assert!(size_of::<C2sv>() == 36);
    assert!(size_of::<C2Simplex>() == 152);
};

// ---------------------------------------------------------------------------
// Symbol table
// ---------------------------------------------------------------------------

macro_rules! api {
    ( $( $name:ident : $ty:ty , )* ) => {
        /// Every one of the 38 symbols the C `.so` exports, resolved by `dlsym`.
        pub struct Api {
            pub tag: &'static str,
            pub path: PathBuf,
            _lib: Library,
            $( pub $name : $ty , )*
        }

        impl Api {
            pub fn load(tag: &'static str, path: &Path) -> Api {
                // RTLD_NOW so that any unresolved symbol in the object is a
                // hard load failure instead of a lazy surprise later on.
                let raw = unsafe {
                    libloading::os::unix::Library::open(
                        Some(path),
                        libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
                    )
                }
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
                let lib: Library = raw.into();
                $(
                    let $name : $ty = unsafe {
                        let s: Symbol<$ty> = lib
                            .get(concat!(stringify!($name), "\0").as_bytes())
                            .unwrap_or_else(|e| panic!(
                                "dlsym({}) missing in {}: {e}",
                                stringify!($name), path.display()));
                        *s
                    };
                )*
                Api { tag, path: path.to_path_buf(), _lib: lib, $( $name , )* }
            }
        }

        /// Names of all symbols the harness binds, for the parity test.
        pub const BOUND_SYMBOLS: &[&str] = &[ $( stringify!($name) , )* ];
    };
}

api! {
    c2V: extern "C" fn(f32, f32) -> C2v,
    c2Mulvs: extern "C" fn(C2v, f32) -> C2v,
    c2Maxv: extern "C" fn(C2v, C2v) -> C2v,
    c2Minv: extern "C" fn(C2v, C2v) -> C2v,
    c2Clampv: extern "C" fn(C2v, C2v, C2v) -> C2v,
    c2Sub: extern "C" fn(C2v, C2v) -> C2v,
    c2Dot: extern "C" fn(C2v, C2v) -> f32,
    c2RotIdentity: extern "C" fn() -> C2r,
    c2xIdentity: extern "C" fn() -> C2x,
    c2BBVerts: unsafe extern "C" fn(*mut C2v, *mut C2Aabb),
    c2MakeProxy: unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy),
    c2Len: extern "C" fn(C2v) -> f32,
    c2Det2: extern "C" fn(C2v, C2v) -> f32,
    c2GJKSimplexMetric: unsafe extern "C" fn(*mut C2Simplex) -> f32,
    c2Mulrv: extern "C" fn(C2r, C2v) -> C2v,
    c2Add: extern "C" fn(C2v, C2v) -> C2v,
    c2Mulxv: extern "C" fn(C2x, C2v) -> C2v,
    c22: unsafe extern "C" fn(*mut C2Simplex),
    c23: unsafe extern "C" fn(*mut C2Simplex),
    c2Neg: extern "C" fn(C2v) -> C2v,
    c2Skew: extern "C" fn(C2v) -> C2v,
    c2CCW90: extern "C" fn(C2v) -> C2v,
    c2D: unsafe extern "C" fn(*mut C2Simplex) -> C2v,
    c2Support: unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int,
    c2Witness: unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v),
    c2Div: extern "C" fn(C2v, f32) -> C2v,
    c2Norm: extern "C" fn(C2v) -> C2v,
    c2L: unsafe extern "C" fn(*mut C2Simplex) -> C2v,
    c2MulrvT: extern "C" fn(C2r, C2v) -> C2v,
    c2GJK: unsafe extern "C" fn(
        *const c_void,
        c_int,
        *const C2x,
        *const c_void,
        c_int,
        *const C2x,
        *mut C2v,
        *mut C2v,
        c_int,
        *mut c_int,
        *mut C2GJKCache,
    ) -> f32,
    c2AABBtoAABB: extern "C" fn(C2Aabb, C2Aabb) -> c_int,
    c2AABBtoCapsule: extern "C" fn(C2Aabb, C2Capsule) -> c_int,
    c2CapsuletoCapsule: extern "C" fn(C2Capsule, C2Capsule) -> c_int,
    c2CircletoCircle: extern "C" fn(C2Circle, C2Circle) -> c_int,
    c2CircletoAABB: extern "C" fn(C2Circle, C2Aabb) -> c_int,
    c2CircletoCapsule: extern "C" fn(C2Circle, C2Capsule) -> c_int,
    c2Collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    capsule: extern "C" fn(f32, f32, f32, f32, f32) -> c_int,
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    // Explicit override, used to run the very same differential suite against a
    // release-optimised build of the cdylib (different float codegen).
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HARVEST_RUST_SO={} is not a file", p.display());
        return p;
    }
    // The integration-test binary lives in <target>/<profile>/deps/. Cargo
    // writes the cdylib into deps/ and *uplifts* a copy to <profile>/ — but the
    // uplift only happens for `cargo build`, not for `cargo test`. So both
    // locations may exist with different contents, and the NEWEST one is the one
    // that corresponds to the current sources.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for cand in [
        deps.join("libcapsule_lib.so"),
        profile.join("libcapsule_lib.so"),
    ] {
        if let Ok(m) = cand.metadata() {
            if let Ok(t) = m.modified() {
                if best.as_ref().is_none_or(|(bt, _)| t > *bt) {
                    best = Some((t, cand));
                }
            }
        }
    }
    match best {
        Some((_, p)) => p,
        None => panic!(
            "libcapsule_lib.so not found next to the test binary ({})",
            exe.display()
        ),
    }
}

fn ensure_c_lib() -> PathBuf {
    let so = c_so_path();
    if so.is_file() {
        return so;
    }
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let st = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake");
    assert!(st.success(), "cmake configure failed");
    let st = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake --build");
    assert!(st.success(), "cmake build failed");
    assert!(so.is_file(), "C .so still missing after build");
    so
}

/// Guard against the single most dangerous failure mode of a `dlopen`-based
/// differential suite: silently testing a **stale** shared object and reporting
/// success. Both `.so`s must be at least as new as their sources.
fn assert_fresh(so: &Path, src: &Path, hint: &str) {
    let (Ok(so_m), Ok(src_m)) = (so.metadata(), src.metadata()) else {
        return;
    };
    let (Ok(so_t), Ok(src_t)) = (so_m.modified(), src_m.modified()) else {
        return;
    };
    assert!(
        so_t >= src_t,
        "STALE SHARED OBJECT: {} is older than {}.\n\
         The differential tests would be comparing against outdated code.\n\
         Rebuild it with: {hint}",
        so.display(),
        src.display()
    );
}

/// `(c_api, rust_api)` — both loaded through `dlopen`/`dlsym`.
pub fn load_pair() -> (Api, Api) {
    let c = ensure_c_lib();
    let r = rust_so_path();
    assert_fresh(
        &c,
        &manifest_dir().join("c_src/src/lib.c"),
        "cd c_src/build && cmake --build .",
    );
    assert_fresh(&r, &manifest_dir().join("src/lib.rs"), "cargo build");
    (Api::load("C", &c), Api::load("RUST", &r))
}

// ---------------------------------------------------------------------------
// Bit-exact difference accumulator
// ---------------------------------------------------------------------------

pub fn as_bytes<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, size_of::<T>()) }
}

pub fn as_bytes_mut<T>(v: &mut T) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v as *mut T as *mut u8, size_of::<T>()) }
}

/// Fill a struct with a reproducible non-zero byte pattern, so that "the C
/// never wrote this field" is observable instead of silently matching a zero.
///
/// Every 4-byte group is forced to a *finite* float bit pattern (the high byte
/// is masked to `0x3f`, so the exponent can never be `0xff`). That matters
/// because the only tolerated difference in `Diff::f32` is a NaN payload, and a
/// poison value must never be able to masquerade as a NaN.
pub fn poison<T: Default>(seed: u8) -> T {
    let mut v = T::default();
    for (i, b) in as_bytes_mut(&mut v).iter_mut().enumerate() {
        let mut x = seed.wrapping_mul(37).wrapping_add((i as u8).wrapping_mul(11)) | 0x21;
        if i % 4 == 3 {
            x &= 0x3f; // keep the float exponent well away from inf/NaN
            x |= 0x10;
        }
        *b = x;
    }
    v
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub struct Diff {
    pub row: String,
    pub checks: u64,
    pub fails: Vec<String>,
    /// Number of results where both libraries produced a NaN but with a
    /// different payload/sign bit. See `NAN_PAYLOAD_NOTE`.
    pub nan_payload_diffs: u64,
}

/// Why a NaN *payload* difference is not a translation defect:
///
/// `addss`/`mulss` are commutative, so a compiler is free to choose either
/// operand as the destination register — and x86 returns the **destination**
/// operand's NaN when both operands are NaN. GCC and LLVM make different
/// choices for the very same expression:
///
/// ```text
/// C   c2Mulvs: movss a.x,%xmm0 ; mulss b,%xmm0      -> dst = a.x  (LHS)
/// RS  c2Mulvs: movaps b,%xmm0  ; mulss a.x,%xmm0    -> dst = b    (RHS)
/// C   c2Dot  : mulss %xmm2,%xmm0 (dst = b.y)        -> dst = RHS
/// RS  c2Dot  : mulss -0xc(%rsp),%xmm1 (dst = a.y)   -> dst = LHS
/// ```
///
/// IEEE-754 §6.2.3 leaves the payload of a NaN result unspecified when an
/// operand is NaN, and explicitly states the sign of a NaN is not interpreted;
/// the C standard adds no constraint. Matching GCC here would mean replicating
/// GCC's register allocation, which no Rust source spelling can express (and
/// which flips again at a different `-O` level).
///
/// Therefore: NaN-**ness** is compared strictly (a NaN on one side and a number
/// on the other is a hard failure), every non-NaN result is compared
/// bit-for-bit, and only the payload bits of a mutually-NaN result are
/// tolerated — and each occurrence is counted and reported.
pub const NAN_PAYLOAD_NOTE: &str =
    "both libraries produced NaN with differing payload bits (unspecified by IEEE-754 §6.2.3)";

impl Diff {
    pub fn new(row: &str) -> Diff {
        Diff {
            row: row.to_string(),
            checks: 0,
            fails: Vec::new(),
            nan_payload_diffs: 0,
        }
    }

    fn record(&mut self, msg: String) {
        if self.fails.len() < 12 {
            self.fails.push(msg);
        } else if self.fails.len() == 12 {
            self.fails.push("... (further failures suppressed)".to_string());
        }
    }

    /// Bit-exact f32 comparison (`to_bits`), so `+0.0` != `-0.0`.
    /// The single tolerated difference is the payload of a mutually-NaN result
    /// (see `NAN_PAYLOAD_NOTE`); it is counted, and a NaN-vs-number mismatch is
    /// always a hard failure.
    pub fn f32(&mut self, ctx: &str, c: f32, r: f32) {
        self.checks += 1;
        if c.to_bits() == r.to_bits() {
            return;
        }
        if c.is_nan() && r.is_nan() {
            self.nan_payload_diffs += 1;
            return;
        }
        self.record(format!(
            "{ctx}: f32 C={c:?} (0x{:08x}) != RUST={r:?} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        ));
    }

    pub fn int(&mut self, ctx: &str, c: c_int, r: c_int) {
        self.checks += 1;
        if c != r {
            self.record(format!("{ctx}: int C={c} != RUST={r}"));
        }
    }

    pub fn v(&mut self, ctx: &str, c: C2v, r: C2v) {
        self.f32(&format!("{ctx}.x"), c.x, r.x);
        self.f32(&format!("{ctx}.y"), c.y, r.y);
    }

    /// Whole-struct byte comparison — catches fields the C leaves untouched.
    pub fn raw<T>(&mut self, ctx: &str, c: &T, r: &T) {
        self.checks += 1;
        let (cb, rb) = (as_bytes(c), as_bytes(r));
        if cb != rb {
            let mut first = 0usize;
            while first < cb.len() && cb[first] == rb[first] {
                first += 1;
            }
            self.record(format!(
                "{ctx}: {} bytes differ (first at offset {first})\n    C   ={}\n    RUST={}",
                cb.len(),
                hex(cb),
                hex(rb)
            ));
        }
    }

    // --- field-wise struct comparators -----------------------------------
    // These structs contain no padding, so a field-wise comparison covers
    // exactly the same bytes as a memcmp, while still applying the NaN policy.

    pub fn rot(&mut self, ctx: &str, c: C2r, r: C2r) {
        self.f32(&format!("{ctx}.c"), c.c, r.c);
        self.f32(&format!("{ctx}.s"), c.s, r.s);
    }

    pub fn xform(&mut self, ctx: &str, c: C2x, r: C2x) {
        self.v(&format!("{ctx}.p"), c.p, r.p);
        self.rot(&format!("{ctx}.r"), c.r, r.r);
    }

    pub fn aabb(&mut self, ctx: &str, c: &C2Aabb, r: &C2Aabb) {
        self.v(&format!("{ctx}.min"), c.min, r.min);
        self.v(&format!("{ctx}.max"), c.max, r.max);
    }

    pub fn varr(&mut self, ctx: &str, c: &[C2v], r: &[C2v]) {
        assert_eq!(c.len(), r.len());
        for k in 0..c.len() {
            self.v(&format!("{ctx}[{k}]"), c[k], r[k]);
        }
    }

    pub fn proxy(&mut self, ctx: &str, c: &C2Proxy, r: &C2Proxy) {
        self.f32(&format!("{ctx}.radius"), c.radius, r.radius);
        self.int(&format!("{ctx}.count"), c.count, r.count);
        self.varr(&format!("{ctx}.verts"), &c.verts, &r.verts);
    }

    pub fn sv(&mut self, ctx: &str, c: &C2sv, r: &C2sv) {
        self.v(&format!("{ctx}.sA"), c.sA, r.sA);
        self.v(&format!("{ctx}.sB"), c.sB, r.sB);
        self.v(&format!("{ctx}.p"), c.p, r.p);
        self.f32(&format!("{ctx}.u"), c.u, r.u);
        self.int(&format!("{ctx}.iA"), c.iA, r.iA);
        self.int(&format!("{ctx}.iB"), c.iB, r.iB);
    }

    pub fn simplex(&mut self, ctx: &str, c: &C2Simplex, r: &C2Simplex) {
        for k in 0..4 {
            self.sv(&format!("{ctx}.verts[{k}]"), &c.verts[k], &r.verts[k]);
        }
        self.f32(&format!("{ctx}.div"), c.div, r.div);
        self.int(&format!("{ctx}.count"), c.count, r.count);
    }

    pub fn cache(&mut self, ctx: &str, c: &C2GJKCache, r: &C2GJKCache) {
        self.f32(&format!("{ctx}.metric"), c.metric, r.metric);
        self.int(&format!("{ctx}.count"), c.count, r.count);
        for k in 0..3 {
            self.int(&format!("{ctx}.iA[{k}]"), c.iA[k], r.iA[k]);
            self.int(&format!("{ctx}.iB[{k}]"), c.iB[k], r.iB[k]);
        }
        self.f32(&format!("{ctx}.div"), c.div, r.div);
    }

    pub fn finish(self) {
        assert!(self.checks > 0, "[{}] performed no checks at all", self.row);
        if !self.fails.is_empty() {
            panic!(
                "[{}] {} divergence(s) out of {} checks:\n  {}",
                self.row,
                self.fails.len(),
                self.checks,
                self.fails.join("\n  ")
            );
        }
        if self.nan_payload_diffs > 0 {
            eprintln!(
                "[{}] OK ({} bit-exact checks, {} tolerated NaN-payload-only diffs: {})",
                self.row, self.checks, self.nan_payload_diffs, NAN_PAYLOAD_NOTE
            );
        } else {
            eprintln!("[{}] OK ({} bit-exact checks)", self.row, self.checks);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) and input generators
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C2C2_A11C_E000;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        (self.u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.u32() % n
    }
    pub fn chance(&mut self, one_in: u32) -> bool {
        self.below(one_in) == 0
    }
    pub fn unit(&mut self) -> f32 {
        (self.u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    pub fn f32_in(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    /// "Ordinary" coordinate: ±200, often snapped to a whole/half number so
    /// that exact-equality boundaries (touching shapes) are actually hit.
    pub fn coord(&mut self) -> f32 {
        let v = self.f32_in(-200.0, 200.0);
        match self.below(4) {
            0 => v.round(),
            1 => (v * 2.0).round() / 2.0,
            _ => v,
        }
    }
    pub fn small_coord(&mut self) -> f32 {
        let v = self.f32_in(-8.0, 8.0);
        if self.chance(3) { v.round() } else { v }
    }
    pub fn radius(&mut self) -> f32 {
        match self.below(8) {
            0 => 0.0,
            1 => self.f32_in(-30.0, 0.0), // negative radii are accepted by the C
            2 => self.f32_in(0.0, 200.0),
            _ => self.f32_in(0.0, 30.0),
        }
    }
    pub fn huge(&mut self) -> f32 {
        let m = match self.below(4) {
            0 => 1e18f32,
            1 => 1e30f32,
            2 => f32::MAX,
            _ => 1e38f32,
        };
        if self.chance(2) { -m } else { m }
    }
    pub fn tiny(&mut self) -> f32 {
        let m = match self.below(4) {
            0 => 1e-40f32, // denormal
            1 => f32::MIN_POSITIVE,
            2 => f32::EPSILON,
            _ => 1.192_092_9e-7f32,
        };
        if self.chance(2) { -m } else { m }
    }
    pub fn special(&mut self) -> f32 {
        SPECIALS[self.below(SPECIALS.len() as u32) as usize]
    }
    /// Mixture used for the "special values" rows.
    pub fn any(&mut self) -> f32 {
        match self.below(10) {
            0 | 1 | 2 | 3 | 4 => self.coord(),
            5 | 6 => self.special(),
            7 => self.huge(),
            8 => self.tiny(),
            _ => 0.0,
        }
    }

    pub fn v_coord(&mut self) -> C2v {
        C2v {
            x: self.coord(),
            y: self.coord(),
        }
    }
    pub fn v_small(&mut self) -> C2v {
        C2v {
            x: self.small_coord(),
            y: self.small_coord(),
        }
    }
    pub fn v_any(&mut self) -> C2v {
        C2v {
            x: self.any(),
            y: self.any(),
        }
    }
    pub fn v_huge(&mut self) -> C2v {
        C2v {
            x: self.huge(),
            y: self.huge(),
        }
    }
    pub fn v_tiny(&mut self) -> C2v {
        C2v {
            x: self.tiny(),
            y: self.tiny(),
        }
    }
    pub fn v_special(&mut self) -> C2v {
        C2v {
            x: self.special(),
            y: self.special(),
        }
    }

    pub fn rot(&mut self) -> C2r {
        // Normalised rotation from a random angle, sometimes an exact quarter
        // turn, sometimes a completely arbitrary (non-normalised) pair.
        match self.below(6) {
            0 => C2r { c: 1.0, s: 0.0 },
            1 => C2r { c: 0.0, s: 1.0 },
            2 => C2r { c: -1.0, s: 0.0 },
            3 => C2r { c: 0.0, s: -1.0 },
            _ => {
                let a = self.f32_in(-3.141_592_7, 3.141_592_7);
                C2r {
                    c: a.cos(),
                    s: a.sin(),
                }
            }
        }
    }
    pub fn rot_weird(&mut self) -> C2r {
        C2r {
            c: self.f32_in(-4.0, 4.0),
            s: self.f32_in(-4.0, 4.0),
        }
    }

    pub fn circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.v_coord(),
            r: self.radius(),
        }
    }
    pub fn aabb(&mut self) -> C2Aabb {
        let a = self.v_coord();
        let b = self.v_coord();
        match self.below(8) {
            0 => C2Aabb { min: a, max: a },              // degenerate point
            1 => C2Aabb { min: b, max: a },              // possibly inverted
            _ => C2Aabb {
                min: C2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: C2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            },
        }
    }
    pub fn capsule(&mut self) -> C2Capsule {
        let a = self.v_coord();
        let b = if self.chance(8) { a } else { self.v_coord() };
        C2Capsule {
            a,
            b,
            r: self.radius(),
        }
    }
    pub fn simplex_vert(&mut self) -> C2sv {
        C2sv {
            sA: self.v_coord(),
            sB: self.v_coord(),
            p: self.v_small(),
            u: self.f32_in(-4.0, 4.0),
            iA: self.below(4) as c_int,
            iB: self.below(4) as c_int,
        }
    }
}

/// A fully randomised `c2Simplex` with the requested `count`. All four vertex
/// slots are filled (so the "the C only touches the first `count`" behaviour is
/// observable) and `div` covers 0 / negative / huge.
pub fn rnd_simplex(rng: &mut Rng, count: c_int) -> C2Simplex {
    let mut s = C2Simplex::default();
    for k in 0..4 {
        s.verts[k] = rng.simplex_vert();
    }
    s.div = match rng.below(8) {
        0 => 0.0,
        1 => -0.0,
        2 => rng.f32_in(-4.0, 0.0),
        3 => rng.huge(),
        _ => rng.f32_in(0.001, 8.0),
    };
    s.count = count;
    s
}

pub const SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::MAX,
    f32::MIN,
    f32::MIN_POSITIVE,
    f32::EPSILON,
    1.192_092_9e-7,
    1e-40,
    -1e-40,
    1.0,
    -1.0,
    0.5,
    -0.5,
    3.402_823_5e38,
];

// ---------------------------------------------------------------------------
// Shape bag: lets a test hand `c2GJK` / `c2Collided` an opaque pointer
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
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
            Shape::Circle(c) => c as *const C2Circle as *const c_void,
            Shape::Aabb(a) => a as *const C2Aabb as *const c_void,
            Shape::Capsule(c) => c as *const C2Capsule as *const c_void,
        }
    }
    /// Translate the shape (used by the "moving shapes" warm-cache rows).
    pub fn translated(&self, d: C2v) -> Shape {
        let t = |v: C2v| C2v {
            x: v.x + d.x,
            y: v.y + d.y,
        };
        match *self {
            Shape::Circle(c) => Shape::Circle(C2Circle { p: t(c.p), r: c.r }),
            Shape::Aabb(a) => Shape::Aabb(C2Aabb {
                min: t(a.min),
                max: t(a.max),
            }),
            Shape::Capsule(c) => Shape::Capsule(C2Capsule {
                a: t(c.a),
                b: t(c.b),
                r: c.r,
            }),
        }
    }
}

pub const TYPE_NAMES: [&str; 3] = ["circle", "aabb", "capsule"];

/// A random shape of the requested type index (0=circle, 1=aabb, 2=capsule).
pub fn shape_of(rng: &mut Rng, ty: usize) -> Shape {
    match ty {
        0 => Shape::Circle(rng.circle()),
        1 => Shape::Aabb(rng.aabb()),
        _ => Shape::Capsule(rng.capsule()),
    }
}

/// A random shape of the requested type placed near the origin, so that shapes
/// generated in pairs actually overlap / touch / just-miss frequently.
pub fn shape_near(rng: &mut Rng, ty: usize, centre: C2v, spread: f32) -> Shape {
    let jitter = |rng: &mut Rng| C2v {
        x: centre.x + rng.f32_in(-spread, spread),
        y: centre.y + rng.f32_in(-spread, spread),
    };
    match ty {
        0 => Shape::Circle(C2Circle {
            p: jitter(rng),
            r: rng.f32_in(0.0, spread),
        }),
        1 => {
            let a = jitter(rng);
            let b = jitter(rng);
            Shape::Aabb(C2Aabb {
                min: C2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                },
                max: C2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                },
            })
        }
        _ => Shape::Capsule(C2Capsule {
            a: jitter(rng),
            b: jitter(rng),
            r: rng.f32_in(0.0, spread),
        }),
    }
}

// ---------------------------------------------------------------------------
// c2GJK differential driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct GjkOpts {
    pub ax: Option<C2x>,
    pub bx: Option<C2x>,
    pub use_radius: c_int,
    pub want_outa: bool,
    pub want_outb: bool,
    pub want_iters: bool,
}

impl Default for GjkOpts {
    fn default() -> Self {
        GjkOpts {
            ax: None,
            bx: None,
            use_radius: 1,
            want_outa: true,
            want_outb: true,
            want_iters: true,
        }
    }
}

/// Everything `c2GJK` can produce, so the caller can compare all of it.
#[derive(Clone, Copy, Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub outa: C2v,
    pub outb: C2v,
    pub iters: c_int,
    pub cache: C2GJKCache,
    pub cache_used: bool,
}

/// Calls `c2GJK` on one library. `cache` is `Some(..)` to pass a cache pointer
/// (and is updated in place, exactly like a real incremental consumer).
pub fn call_gjk(
    api: &Api,
    a: &Shape,
    b: &Shape,
    opts: &GjkOpts,
    cache: Option<&mut C2GJKCache>,
) -> GjkOut {
    // Poison the out-params so a "not written" case is visible.
    let mut outa = C2v {
        x: f32::from_bits(0xDEAD_BEEF),
        y: f32::from_bits(0xDEAD_BEEE),
    };
    let mut outb = C2v {
        x: f32::from_bits(0xCAFE_BABE),
        y: f32::from_bits(0xCAFE_BABD),
    };
    let mut iters: c_int = -12345;

    let ax = opts.ax;
    let bx = opts.bx;
    let axp = ax.as_ref().map_or(std::ptr::null(), |v| v as *const C2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |v| v as *const C2x);

    let cache_used = cache.is_some();
    let mut local_cache = C2GJKCache::default();

    let dist = match cache {
        Some(cref) => {
            local_cache = *cref;
            let d = unsafe {
                (api.c2GJK)(
                    a.as_ptr(),
                    a.ty(),
                    axp,
                    b.as_ptr(),
                    b.ty(),
                    bxp,
                    if opts.want_outa {
                        &mut outa
                    } else {
                        std::ptr::null_mut()
                    },
                    if opts.want_outb {
                        &mut outb
                    } else {
                        std::ptr::null_mut()
                    },
                    opts.use_radius,
                    if opts.want_iters {
                        &mut iters
                    } else {
                        std::ptr::null_mut()
                    },
                    &mut local_cache,
                )
            };
            *cref = local_cache;
            d
        }
        None => unsafe {
            (api.c2GJK)(
                a.as_ptr(),
                a.ty(),
                axp,
                b.as_ptr(),
                b.ty(),
                bxp,
                if opts.want_outa {
                    &mut outa
                } else {
                    std::ptr::null_mut()
                },
                if opts.want_outb {
                    &mut outb
                } else {
                    std::ptr::null_mut()
                },
                opts.use_radius,
                if opts.want_iters {
                    &mut iters
                } else {
                    std::ptr::null_mut()
                },
                std::ptr::null_mut(),
            )
        },
    };

    GjkOut {
        dist,
        outa,
        outb,
        iters,
        cache: local_cache,
        cache_used,
    }
}

/// Compare two `c2GJK` results field by field, bit-exactly.
pub fn cmp_gjk(d: &mut Diff, ctx: &str, c: &GjkOut, r: &GjkOut) {
    d.f32(&format!("{ctx}/dist"), c.dist, r.dist);
    d.v(&format!("{ctx}/outA"), c.outa, r.outa);
    d.v(&format!("{ctx}/outB"), c.outb, r.outb);
    d.int(&format!("{ctx}/iters"), c.iters, r.iters);
    if c.cache_used || r.cache_used {
        d.cache(&format!("{ctx}/cache"), &c.cache, &r.cache);
    }
}

/// Run one `c2GJK` configuration on both libraries and compare everything.
pub fn gjk_case(d: &mut Diff, capi: &Api, rapi: &Api, ctx: &str, a: &Shape, b: &Shape, o: &GjkOpts) {
    let c = call_gjk(capi, a, b, o, None);
    let r = call_gjk(rapi, a, b, o, None);
    cmp_gjk(d, ctx, &c, &r);
}

/// Same, but with a cache that both libraries start from and that is compared
/// after the call.
pub fn gjk_case_cached(
    d: &mut Diff,
    capi: &Api,
    rapi: &Api,
    ctx: &str,
    a: &Shape,
    b: &Shape,
    o: &GjkOpts,
    start: &C2GJKCache,
) -> (C2GJKCache, C2GJKCache) {
    let mut cc = *start;
    let mut rc = *start;
    let c = call_gjk(capi, a, b, o, Some(&mut cc));
    let r = call_gjk(rapi, a, b, o, Some(&mut rc));
    cmp_gjk(d, ctx, &c, &r);
    d.cache(&format!("{ctx}/cache_after"), &cc, &rc);
    (cc, rc)
}
