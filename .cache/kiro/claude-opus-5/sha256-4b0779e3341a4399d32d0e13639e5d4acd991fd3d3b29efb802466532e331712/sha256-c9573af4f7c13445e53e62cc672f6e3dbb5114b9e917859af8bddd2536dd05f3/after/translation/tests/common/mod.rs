//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C `.so` built by CMake and the Rust
//! `cdylib`) with `libloading` and exposes matched symbol pairs. Nothing in the
//! Rust crate is ever called directly — every call goes through `dlsym`, so the
//! `#[no_mangle]` export wrappers and the C ABI are exercised too.

#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Layout-compatible mirrors of the C types (see c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub const C2_TYPE_CIRCLE: i32 = 0;
pub const C2_TYPE_AABB: i32 = 1;
pub const C2_TYPE_CAPSULE: i32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2GJKCache {
    pub metric: f32,
    pub count: i32,
    pub iA: [i32; 3],
    pub iB: [i32; 3],
    pub div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2Proxy {
    pub radius: f32,
    pub count: i32,
    pub verts: [C2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2sv {
    pub sA: C2v,
    pub sB: C2v,
    pub p: C2v,
    pub u: f32,
    pub iA: i32,
    pub iB: i32,
}

/// `typedef struct { c2sv a, b, c, d; float div; int count; } c2Simplex;`
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct C2Simplex {
    pub v: [C2sv; 4],
    pub div: f32,
    pub count: i32,
}

pub const FLT_MAX: f32 = f32::MAX;
pub const FLT_EPSILON: f32 = 1.192_092_895_507_812_5e-7;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

/// `dlopen("libm.so.6", RTLD_NOW | RTLD_GLOBAL)`, leaked on purpose so the
/// symbols stay visible for the whole process lifetime.
fn preload_libm() {
    use libloading::os::unix as ux;
    for name in ["libm.so.6", "libm.so"] {
        let flags = ux::RTLD_NOW | ux::RTLD_GLOBAL;
        if let Ok(lib) = unsafe { ux::Library::open(Some(name), flags) } {
            std::mem::forget(lib);
            return;
        }
    }
    // Fall through: on some glibc builds libm is folded into libc and the C
    // library's `sqrtf` resolves without help.
}

fn find_rust_so() -> PathBuf {
    const NAME: &str = "libreverse_collide_lib.so";
    let md = manifest_dir();
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| md.join("target"));
    // Prefer the release cdylib (that is what an external consumer links).
    for sub in ["release", "debug"] {
        let p = target.join(sub).join(NAME);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no Rust {NAME} under {}; build it with `cargo build --release`",
        target.display()
    );
}

impl Libs {
    pub fn load() -> Libs {
        // The C .so is linked without -lm, so `sqrtf` (used by c2Len) is left
        // undefined and is NOT resolvable from the test executable unless libm
        // happens to be in its dependency list. Pull libm into the GLOBAL
        // namespace first so the C library's lazy binding can find it.
        // (c_src/ must not be modified, so this has to be done here.)
        preload_libm();

        let c_path = find_c_so();
        let r_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let r = unsafe { Library::new(&r_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
        Libs {
            c,
            r,
            c_path,
            r_path,
        }
    }

    /// Resolve `name` in BOTH libraries, returning `(c_fn, rust_fn)`.
    pub fn pair<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let cs: Symbol<T> = unsafe { self.c.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("C .so is missing symbol `{name}`: {e}"));
        let rs: Symbol<T> = unsafe { self.r.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("Rust .so is missing symbol `{name}`: {e}"));
        (cs, rs)
    }
}

/// One process-wide pair of handles (dlopen is cheap but we only need one).
pub fn libs() -> &'static Libs {
    use std::sync::OnceLock;
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(Libs::load)
}

pub fn so_path_c() -> PathBuf {
    find_c_so()
}
pub fn so_path_rust() -> PathBuf {
    find_rust_so()
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// One 4-byte field of a C struct, tagged with its real type.
#[derive(Debug, Clone, Copy)]
pub enum Lane {
    F(f32),
    I(i32),
}

impl Lane {
    fn agrees(self, other: Lane) -> bool {
        match (self, other) {
            // Bit-exact for every value class EXCEPT the NaN payload/sign.
            //
            // NaN payload and sign propagation is explicitly unspecified by
            // IEEE 754 and by the C standard, and the two toolchains select
            // different (equally valid) instruction sequences for commutative
            // expressions: gcc -O0 compiles `a.x += b.x` to
            // `addss %xmm1(a.x),%xmm0(b.x)` so the *second* operand's NaN wins,
            // while LLVM keeps `a.x` in the destination so the *first* wins.
            // Likewise LLVM folds `-a.s*b.x + a.c*b.y` into an `fsub`, which
            // does not flip the NaN sign the C's explicit negation does.
            // No amount of source-level rewriting pins this down across two
            // independent optimisers, and no NaN payload is observable through
            // any comparison the library performs. Everything else --
            // +0 vs -0, denormals, infinities, ordinary values -- is compared
            // bit-for-bit.
            (Lane::F(a), Lane::F(b)) => {
                if a.is_nan() || b.is_nan() {
                    a.is_nan() && b.is_nan()
                } else {
                    a.to_bits() == b.to_bits()
                }
            }
            (Lane::I(a), Lane::I(b)) => a == b,
            _ => false,
        }
    }
    fn show(self) -> String {
        match self {
            Lane::F(v) => format!("{v:?}/0x{:08x}", v.to_bits()),
            Lane::I(v) => format!("{v}"),
        }
    }
}

/// Field-wise decomposition of a C struct into typed 4-byte lanes.
pub trait Lanes {
    fn lanes(&self) -> Vec<Lane>;
}

impl Lanes for f32 {
    fn lanes(&self) -> Vec<Lane> {
        vec![Lane::F(*self)]
    }
}
impl Lanes for i32 {
    fn lanes(&self) -> Vec<Lane> {
        vec![Lane::I(*self)]
    }
}
impl Lanes for C2v {
    fn lanes(&self) -> Vec<Lane> {
        vec![Lane::F(self.x), Lane::F(self.y)]
    }
}
impl Lanes for C2r {
    fn lanes(&self) -> Vec<Lane> {
        vec![Lane::F(self.c), Lane::F(self.s)]
    }
}
impl Lanes for C2x {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = self.p.lanes();
        v.extend(self.r.lanes());
        v
    }
}
impl Lanes for C2Circle {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = self.p.lanes();
        v.push(Lane::F(self.r));
        v
    }
}
impl Lanes for C2AABB {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = self.min.lanes();
        v.extend(self.max.lanes());
        v
    }
}
impl Lanes for C2Capsule {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = self.a.lanes();
        v.extend(self.b.lanes());
        v.push(Lane::F(self.r));
        v
    }
}
impl Lanes for C2Proxy {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = vec![Lane::F(self.radius), Lane::I(self.count)];
        for p in &self.verts {
            v.extend(p.lanes());
        }
        v
    }
}
impl Lanes for C2sv {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = self.sA.lanes();
        v.extend(self.sB.lanes());
        v.extend(self.p.lanes());
        v.push(Lane::F(self.u));
        v.push(Lane::I(self.iA));
        v.push(Lane::I(self.iB));
        v
    }
}
impl Lanes for C2Simplex {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = Vec::new();
        for s in &self.v {
            v.extend(s.lanes());
        }
        v.push(Lane::F(self.div));
        v.push(Lane::I(self.count));
        v
    }
}
impl Lanes for C2GJKCache {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = vec![Lane::F(self.metric), Lane::I(self.count)];
        v.extend(self.iA.iter().map(|x| Lane::I(*x)));
        v.extend(self.iB.iter().map(|x| Lane::I(*x)));
        v.push(Lane::F(self.div));
        v
    }
}
impl<T: Lanes> Lanes for Option<T> {
    fn lanes(&self) -> Vec<Lane> {
        match self {
            None => vec![Lane::I(i32::MIN + 7)], // distinct "absent" marker
            Some(v) => v.lanes(),
        }
    }
}
impl<T: Lanes> Lanes for [T] {
    fn lanes(&self) -> Vec<Lane> {
        self.iter().flat_map(|x| x.lanes()).collect()
    }
}
impl<T: Lanes, const N: usize> Lanes for [T; N] {
    fn lanes(&self) -> Vec<Lane> {
        self.iter().flat_map(|x| x.lanes()).collect()
    }
}

pub fn lanes_agree(c: &[Lane], r: &[Lane]) -> Option<usize> {
    if c.len() != r.len() {
        return Some(usize::MAX);
    }
    (0..c.len()).find(|&i| !c[i].agrees(r[i]))
}

/// Raw object representation (used for diagnostics only).
pub fn raw<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

pub fn show_lanes(l: &[Lane]) -> String {
    l.iter().map(|x| x.show()).collect::<Vec<_>>().join(" ")
}

/// Assert two values agree field-by-field, with a readable failure message.
#[track_caller]
pub fn same<T: Lanes + std::fmt::Debug>(
    what: &str,
    ctx: &dyn std::fmt::Debug,
    c: &T,
    r: &T,
) {
    let (cl, rl) = (c.lanes(), r.lanes());
    if let Some(i) = lanes_agree(&cl, &rl) {
        panic!(
            "DIVERGENCE in {what} (field #{i})\n  input : {ctx:?}\n  C     : {}\n  Rust  : {}",
            show_lanes(&cl),
            show_lanes(&rl)
        );
    }
}

#[track_caller]
pub fn same_f32(what: &str, ctx: &dyn std::fmt::Debug, c: f32, r: f32) {
    same(what, ctx, &c, &r);
}

#[track_caller]
pub fn same_i32(what: &str, ctx: &dyn std::fmt::Debug, c: i32, r: i32) {
    assert_eq!(c, r, "DIVERGENCE in {what}\n  input : {ctx:?}");
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) + input pools
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    /// Seeded RNG. Set `DIFF_SEED_OFFSET` in the environment to re-run the whole
    /// suite with a different input corpus and confirm the results are not an
    /// artefact of one lucky seed.
    pub fn new(seed: u64) -> Rng {
        let off: u64 = std::env::var("DIFF_SEED_OFFSET")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(off.wrapping_mul(0xD1B5_4A32_D192_ED03)) | 1)
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
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next_u32() as f64) / (u32::MAX as f64);
        (lo as f64 + t * (hi as f64 - lo as f64)) as f32
    }
    /// Completely arbitrary bit pattern: normals, denormals, +-0, +-inf, NaN.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// "Interesting" float: mostly tame play-area values, sometimes an extreme.
    pub fn f32_mixed(&mut self) -> f32 {
        match self.below(20) {
            0 => SPECIAL[self.below(SPECIAL.len() as u32) as usize],
            1 => self.any_f32(),
            2 => self.range(-1.0e30, 1.0e30),
            3 => self.range(-1.0, 1.0) * f32::from_bits(1),
            _ => self.range(-200.0, 200.0),
        }
    }
    /// Tame float only (keeps geometry meaningful).
    pub fn f32_tame(&mut self) -> f32 {
        self.range(-200.0, 200.0)
    }
    pub fn v_mixed(&mut self) -> C2v {
        C2v {
            x: self.f32_mixed(),
            y: self.f32_mixed(),
        }
    }
    pub fn v_tame(&mut self) -> C2v {
        C2v {
            x: self.f32_tame(),
            y: self.f32_tame(),
        }
    }
    pub fn v_any(&mut self) -> C2v {
        C2v {
            x: self.any_f32(),
            y: self.any_f32(),
        }
    }
    pub fn circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.v_tame(),
            r: self.range(0.0, 60.0),
        }
    }
    pub fn aabb(&mut self) -> C2AABB {
        let a = self.v_tame();
        let w = self.range(0.0, 80.0);
        let h = self.range(0.0, 80.0);
        C2AABB {
            min: a,
            max: C2v {
                x: a.x + w,
                y: a.y + h,
            },
        }
    }
    pub fn capsule(&mut self) -> C2Capsule {
        C2Capsule {
            a: self.v_tame(),
            b: self.v_tame(),
            r: self.range(0.0, 40.0),
        }
    }
    /// Random transform: unit rotation + translation.
    pub fn xform_unit(&mut self) -> C2x {
        let th = self.range(-6.2831855, 6.2831855);
        C2x {
            p: self.v_tame(),
            r: C2r {
                c: th.cos(),
                s: th.sin(),
            },
        }
    }
    /// Random transform with a NON-unit `c2r` (the C never normalises it).
    pub fn xform_nonunit(&mut self) -> C2x {
        C2x {
            p: self.v_tame(),
            r: C2r {
                c: self.range(-3.0, 3.0),
                s: self.range(-3.0, 3.0),
            },
        }
    }
}

pub const SPECIAL: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::from_bits(1),            // smallest denormal
    f32::from_bits(0x8000_0001),  // -smallest denormal
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::from_bits(0x7f80_0001), // signalling NaN
    FLT_EPSILON,
    -FLT_EPSILON,
    1.0e30,
    -1.0e30,
    1.0e-30,
    16777216.0, // 2^24, first integer gap
    -16777216.0,
];

/// Shapes covering the interesting degenerate cases, used alongside random ones.
pub fn degenerate_circles() -> Vec<C2Circle> {
    vec![
        C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 0.0 },
        C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: -5.0 },
        C2Circle { p: C2v { x: 1.0e30, y: -1.0e30 }, r: 1.0e30 },
        C2Circle { p: C2v { x: f32::NAN, y: 0.0 }, r: 1.0 },
        C2Circle { p: C2v { x: f32::INFINITY, y: 0.0 }, r: 1.0 },
        C2Circle { p: C2v { x: -0.0, y: -0.0 }, r: f32::MIN_POSITIVE },
    ]
}

pub fn degenerate_aabbs() -> Vec<C2AABB> {
    vec![
        C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 0.0, y: 0.0 } },
        C2AABB { min: C2v { x: 5.0, y: 5.0 }, max: C2v { x: -5.0, y: -5.0 } }, // inverted
        C2AABB { min: C2v { x: -1.0e30, y: -1.0e30 }, max: C2v { x: 1.0e30, y: 1.0e30 } },
        C2AABB { min: C2v { x: f32::NAN, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
        C2AABB {
            min: C2v { x: f32::NEG_INFINITY, y: f32::NEG_INFINITY },
            max: C2v { x: f32::INFINITY, y: f32::INFINITY },
        },
        C2AABB { min: C2v { x: -0.0, y: -0.0 }, max: C2v { x: 0.0, y: 0.0 } },
    ]
}

pub fn degenerate_capsules() -> Vec<C2Capsule> {
    vec![
        C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 0.0, y: 0.0 }, r: 0.0 },
        C2Capsule { a: C2v { x: 3.0, y: 3.0 }, b: C2v { x: 3.0, y: 3.0 }, r: 7.0 },
        C2Capsule { a: C2v { x: -1.0e30, y: 0.0 }, b: C2v { x: 1.0e30, y: 0.0 }, r: 1.0 },
        C2Capsule { a: C2v { x: f32::NAN, y: 0.0 }, b: C2v { x: 1.0, y: 1.0 }, r: 1.0 },
        C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 10.0, y: 0.0 }, r: -3.0 },
        C2Capsule {
            a: C2v { x: f32::INFINITY, y: 0.0 },
            b: C2v { x: f32::NEG_INFINITY, y: 0.0 },
            r: 2.0,
        },
    ]
}

/// The 9 ordered `(typeA, typeB)` combinations.
pub const TYPE_PAIRS: [(i32, i32); 9] = [
    (C2_TYPE_CIRCLE, C2_TYPE_CIRCLE),
    (C2_TYPE_CIRCLE, C2_TYPE_AABB),
    (C2_TYPE_CIRCLE, C2_TYPE_CAPSULE),
    (C2_TYPE_AABB, C2_TYPE_CIRCLE),
    (C2_TYPE_AABB, C2_TYPE_AABB),
    (C2_TYPE_AABB, C2_TYPE_CAPSULE),
    (C2_TYPE_CAPSULE, C2_TYPE_CIRCLE),
    (C2_TYPE_CAPSULE, C2_TYPE_AABB),
    (C2_TYPE_CAPSULE, C2_TYPE_CAPSULE),
];

/// A type-erased shape blob, big enough for any of the three shapes.
#[repr(C, align(8))]
#[derive(Copy, Clone, Debug)]
pub struct ShapeBlob {
    pub bytes: [u8; 24],
    pub kind: i32,
}

impl ShapeBlob {
    pub fn circle(c: C2Circle) -> ShapeBlob {
        let mut b = ShapeBlob { bytes: [0; 24], kind: C2_TYPE_CIRCLE };
        b.bytes[..12].copy_from_slice(raw(&c));
        b
    }
    pub fn aabb(a: C2AABB) -> ShapeBlob {
        let mut b = ShapeBlob { bytes: [0; 24], kind: C2_TYPE_AABB };
        b.bytes[..16].copy_from_slice(raw(&a));
        b
    }
    pub fn capsule(c: C2Capsule) -> ShapeBlob {
        let mut b = ShapeBlob { bytes: [0; 24], kind: C2_TYPE_CAPSULE };
        b.bytes[..20].copy_from_slice(raw(&c));
        b
    }
    pub fn ptr(&self) -> *const std::ffi::c_void {
        self.bytes.as_ptr() as *const std::ffi::c_void
    }
    pub fn random(rng: &mut Rng, kind: i32) -> ShapeBlob {
        match kind {
            C2_TYPE_CIRCLE => ShapeBlob::circle(rng.circle()),
            C2_TYPE_AABB => ShapeBlob::aabb(rng.aabb()),
            _ => ShapeBlob::capsule(rng.capsule()),
        }
    }
    /// A shape of `kind` roughly centred on `at` with roughly `size` extent.
    pub fn near(rng: &mut Rng, kind: i32, at: C2v, size: f32) -> ShapeBlob {
        match kind {
            C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle { p: at, r: size }),
            C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
                min: C2v { x: at.x - size, y: at.y - size },
                max: C2v { x: at.x + size, y: at.y + size },
            }),
            _ => ShapeBlob::capsule(C2Capsule {
                a: C2v { x: at.x - size, y: at.y },
                b: C2v { x: at.x + size, y: at.y },
                r: size * 0.5 + rng.range(0.0, 0.001),
            }),
        }
    }
    pub fn degenerate(kind: i32, i: usize) -> ShapeBlob {
        match kind {
            C2_TYPE_CIRCLE => ShapeBlob::circle(degenerate_circles()[i % 6]),
            C2_TYPE_AABB => ShapeBlob::aabb(degenerate_aabbs()[i % 6]),
            _ => ShapeBlob::capsule(degenerate_capsules()[i % 6]),
        }
    }
}

pub fn type_name(t: i32) -> &'static str {
    match t {
        0 => "CIRCLE",
        1 => "AABB",
        2 => "CAPSULE",
        _ => "INVALID",
    }
}

// ---------------------------------------------------------------------------
// c2GJK signature + a differential driver for it
// ---------------------------------------------------------------------------

pub type FnGJK = unsafe extern "C" fn(
    *const std::ffi::c_void,
    i32,
    *const C2x,
    *const std::ffi::c_void,
    i32,
    *const C2x,
    *mut C2v,
    *mut C2v,
    i32,
    *mut i32,
    *mut C2GJKCache,
) -> f32;

/// Everything `c2GJK` can observably produce.
#[derive(Debug, Clone, Copy)]
pub struct GjkOut {
    pub dist: f32,
    pub a: C2v,
    pub b: C2v,
    pub iters: i32,
    pub cache: Option<C2GJKCache>,
}

impl Lanes for GjkOut {
    fn lanes(&self) -> Vec<Lane> {
        let mut v = vec![Lane::F(self.dist)];
        v.extend(self.a.lanes());
        v.extend(self.b.lanes());
        v.push(Lane::I(self.iters));
        v.extend(self.cache.lanes());
        v
    }
}

/// Options for one `c2GJK` invocation.
#[derive(Debug, Clone, Copy)]
pub struct GjkOpts {
    pub use_radius: i32,
    pub ax: Option<C2x>,
    pub bx: Option<C2x>,
    pub want_a: bool,
    pub want_b: bool,
    pub want_iters: bool,
    pub cache: Option<C2GJKCache>,
}

impl Default for GjkOpts {
    fn default() -> Self {
        GjkOpts {
            use_radius: 0,
            ax: None,
            bx: None,
            want_a: true,
            want_b: true,
            want_iters: true,
            cache: None,
        }
    }
}

/// Invoke `c2GJK` with the full 11-argument signature and capture all outputs.
pub unsafe fn gjk(
    f: &FnGJK,
    a: &ShapeBlob,
    ta: i32,
    b: &ShapeBlob,
    tb: i32,
    opts: &GjkOpts,
) -> GjkOut {
    let mut oa = C2v { x: f32::from_bits(0xdead_beef), y: f32::from_bits(0xdead_beef) };
    let mut ob = oa;
    let mut it: i32 = -12345;
    let mut cache = opts.cache;

    let axp = opts.ax.as_ref().map_or(std::ptr::null(), |x| x as *const C2x);
    let bxp = opts.bx.as_ref().map_or(std::ptr::null(), |x| x as *const C2x);
    let oap = if opts.want_a { &mut oa as *mut C2v } else { std::ptr::null_mut() };
    let obp = if opts.want_b { &mut ob as *mut C2v } else { std::ptr::null_mut() };
    let itp = if opts.want_iters { &mut it as *mut i32 } else { std::ptr::null_mut() };
    let cp = cache
        .as_mut()
        .map_or(std::ptr::null_mut(), |c| c as *mut C2GJKCache);

    let dist = unsafe {
        f(
            a.ptr(),
            ta,
            axp,
            b.ptr(),
            tb,
            bxp,
            oap,
            obp,
            opts.use_radius,
            itp,
            cp,
        )
    };

    GjkOut {
        dist,
        a: oa,
        b: ob,
        iters: it,
        cache,
    }
}

/// Run `c2GJK` in both libraries and assert bit-identical observable output.
#[track_caller]
pub fn gjk_diff(
    ctx: &str,
    a: &ShapeBlob,
    ta: i32,
    b: &ShapeBlob,
    tb: i32,
    opts: &GjkOpts,
) -> GjkOut {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let co = unsafe { gjk(&cf, a, ta, b, tb, opts) };
    let ro = unsafe { gjk(&rf, a, ta, b, tb, opts) };
    let (cl, rl) = (co.lanes(), ro.lanes());
    if let Some(i) = lanes_agree(&cl, &rl) {
        panic!(
            "DIVERGENCE in c2GJK (field #{i}) [{ctx}]\n  typeA={} typeB={} opts={:?}\n  A={:?}\n  B={:?}\n  C   ={:?}\n  Rust={:?}",
            type_name(ta),
            type_name(tb),
            opts,
            a,
            b,
            co,
            ro
        );
    }
    co
}

// ---------------------------------------------------------------------------
// GJK break-reason classifier
// ---------------------------------------------------------------------------
//
// `c2GJK` has five mutually exclusive ways to leave its loop and several ways to
// finish the `use_radius` block, and none of them is directly observable from
// the return value. To get real evidence that each one was exercised, the loop
// is re-assembled here out of the **C library's own exported primitives**
// (`c2MakeProxy`, `c22`, `c23`, `c2L`, `c2D`, `c2Support`, ... all called through
// `dlsym` on the C `.so`). Nothing is re-implemented: this is used only to
// classify an input for coverage bookkeeping. The correctness assertion is
// always the C-vs-Rust comparison in `gjk_diff`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Brk {
    /// 3-simplex enclosed the origin (`hit = 1`).
    Hit,
    /// `d1 > d0` — the iteration made no progress.
    NoProgress,
    /// `c2Dot(d,d) < FLT_EPSILON * FLT_EPSILON` — degenerate search direction.
    DegenerateDir,
    /// the new support point duplicated a saved one.
    Dup,
    /// the loop ran out at `iter == 20`.
    IterCap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RadiusArm {
    /// `hit` was set, so the `use_radius` block is skipped.
    SkippedByHit,
    /// `use_radius == 0`.
    Disabled,
    /// `dist > rA+rB && dist > FLT_EPSILON` — witness points pulled in.
    Shrink,
    /// same, but the shrink made `a == b` exactly, so `dist` was forced to 0.
    ShrinkCollapsed,
    /// the `else` arm: `a = b = midpoint`, `dist = 0`.
    Midpoint,
}

#[derive(Debug, Clone, Copy)]
pub struct Classification {
    pub brk: Brk,
    pub radius: RadiusArm,
    pub iters: i32,
    pub final_count: i32,
    /// `cache_was_read` (only meaningful when a cache was supplied).
    pub cache_read: bool,
}

struct Prim<'a> {
    make_proxy: Symbol<'a, unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut C2Proxy)>,
    mulxv: Symbol<'a, unsafe extern "C" fn(C2x, C2v) -> C2v>,
    mulrvt: Symbol<'a, unsafe extern "C" fn(C2r, C2v) -> C2v>,
    sub: Symbol<'a, unsafe extern "C" fn(C2v, C2v) -> C2v>,
    add: Symbol<'a, unsafe extern "C" fn(C2v, C2v) -> C2v>,
    mulvs: Symbol<'a, unsafe extern "C" fn(C2v, f32) -> C2v>,
    neg: Symbol<'a, unsafe extern "C" fn(C2v) -> C2v>,
    dot: Symbol<'a, unsafe extern "C" fn(C2v, C2v) -> f32>,
    len: Symbol<'a, unsafe extern "C" fn(C2v) -> f32>,
    norm: Symbol<'a, unsafe extern "C" fn(C2v) -> C2v>,
    xident: Symbol<'a, unsafe extern "C" fn() -> C2x>,
    c22: Symbol<'a, unsafe extern "C" fn(*mut C2Simplex)>,
    c23: Symbol<'a, unsafe extern "C" fn(*mut C2Simplex)>,
    cl: Symbol<'a, unsafe extern "C" fn(*mut C2Simplex) -> C2v>,
    cd: Symbol<'a, unsafe extern "C" fn(*mut C2Simplex) -> C2v>,
    support: Symbol<'a, unsafe extern "C" fn(*const C2v, i32, C2v) -> i32>,
    witness: Symbol<'a, unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v)>,
    metric: Symbol<'a, unsafe extern "C" fn(*mut C2Simplex) -> f32>,
}

fn prims() -> Prim<'static> {
    let l = libs();
    Prim {
        make_proxy: l.pair("c2MakeProxy").0,
        mulxv: l.pair("c2Mulxv").0,
        mulrvt: l.pair("c2MulrvT").0,
        sub: l.pair("c2Sub").0,
        add: l.pair("c2Add").0,
        mulvs: l.pair("c2Mulvs").0,
        neg: l.pair("c2Neg").0,
        dot: l.pair("c2Dot").0,
        len: l.pair("c2Len").0,
        norm: l.pair("c2Norm").0,
        xident: l.pair("c2xIdentity").0,
        c22: l.pair("c22").0,
        c23: l.pair("c23").0,
        cl: l.pair("c2L").0,
        cd: l.pair("c2D").0,
        support: l.pair("c2Support").0,
        witness: l.pair("c2Witness").0,
        metric: l.pair("c2GJKSimplexMetric").0,
    }
}

const C2_FLT_MAX_LOCAL: f32 = 3.402_823_466_385_288_598_117_041_834_845_169_25e+38;

/// Re-trace the C's `c2GJK` control flow with the C's own primitives and report
/// which branches it took.
pub fn classify(a: &ShapeBlob, ta: i32, b: &ShapeBlob, tb: i32, opts: &GjkOpts) -> Classification {
    let p = prims();
    unsafe {
        let ax = opts.ax.unwrap_or_else(|| (p.xident)());
        let bx = opts.bx.unwrap_or_else(|| (p.xident)());

        let mut pa = C2Proxy::default();
        let mut pb = C2Proxy::default();
        (p.make_proxy)(a.ptr(), ta, &mut pa);
        (p.make_proxy)(b.ptr(), tb, &mut pb);

        let mut s = C2Simplex::default();
        let mut cache_read = false;
        if let Some(cache) = opts.cache {
            if cache.count != 0 {
                for i in 0..cache.count.clamp(0, 3) as usize {
                    let ia = cache.iA[i];
                    let ib = cache.iB[i];
                    let sa = (p.mulxv)(ax, pa.verts[ia.clamp(0, 7) as usize]);
                    let sb = (p.mulxv)(bx, pb.verts[ib.clamp(0, 7) as usize]);
                    s.v[i].iA = ia;
                    s.v[i].sA = sa;
                    s.v[i].iB = ib;
                    s.v[i].sB = sb;
                    s.v[i].p = (p.sub)(sb, sa);
                    s.v[i].u = 0.0;
                }
                s.count = cache.count;
                s.div = cache.div;
                let metric_old = cache.metric;
                let metric = (p.metric)(&mut s);
                let min_metric = if metric < metric_old { metric } else { metric_old };
                let max_metric = if metric > metric_old { metric } else { metric_old };
                if !(min_metric < max_metric * 2.0 && metric < -1.0e8) {
                    cache_read = true;
                }
            }
        }
        if !cache_read {
            s = C2Simplex::default();
            s.v[0].sA = (p.mulxv)(ax, pa.verts[0]);
            s.v[0].sB = (p.mulxv)(bx, pb.verts[0]);
            s.v[0].p = (p.sub)(s.v[0].sB, s.v[0].sA);
            s.v[0].u = 1.0;
            s.div = 1.0;
            s.count = 1;
        }

        let mut save_a = [0i32; 3];
        let mut save_b = [0i32; 3];
        let mut d0 = C2_FLT_MAX_LOCAL;
        let mut iter = 0i32;
        let mut brk = Brk::IterCap;
        let mut hit = false;
        while iter < 20 {
            let save_count = s.count;
            for i in 0..save_count.clamp(0, 3) as usize {
                save_a[i] = s.v[i].iA;
                save_b[i] = s.v[i].iB;
            }
            match s.count {
                2 => (p.c22)(&mut s),
                3 => (p.c23)(&mut s),
                _ => {}
            }
            if s.count == 3 {
                hit = true;
                brk = Brk::Hit;
                break;
            }
            let pt = (p.cl)(&mut s);
            let d1 = (p.dot)(pt, pt);
            if d1 > d0 {
                brk = Brk::NoProgress;
                break;
            }
            d0 = d1;
            let d = (p.cd)(&mut s);
            if (p.dot)(d, d) < FLT_EPSILON * FLT_EPSILON {
                brk = Brk::DegenerateDir;
                break;
            }
            let ia = (p.support)(pa.verts.as_ptr(), pa.count, (p.mulrvt)(ax.r, (p.neg)(d)));
            let sa = (p.mulxv)(ax, pa.verts[ia.clamp(0, 7) as usize]);
            let ib = (p.support)(pb.verts.as_ptr(), pb.count, (p.mulrvt)(bx.r, d));
            let sb = (p.mulxv)(bx, pb.verts[ib.clamp(0, 7) as usize]);
            let slot = s.count.clamp(0, 3) as usize;
            s.v[slot].iA = ia;
            s.v[slot].sA = sa;
            s.v[slot].iB = ib;
            s.v[slot].sB = sb;
            s.v[slot].p = (p.sub)(sb, sa);
            let mut dup = false;
            for i in 0..save_count.clamp(0, 3) as usize {
                if ia == save_a[i] && ib == save_b[i] {
                    dup = true;
                    break;
                }
            }
            if dup {
                brk = Brk::Dup;
                break;
            }
            s.count += 1;
            iter += 1;
        }

        let mut wa = C2v::default();
        let mut wb = C2v::default();
        (p.witness)(&mut s, &mut wa, &mut wb);
        let dist = (p.len)((p.sub)(wa, wb));

        let radius = if hit {
            RadiusArm::SkippedByHit
        } else if opts.use_radius == 0 {
            RadiusArm::Disabled
        } else {
            let ra = pa.radius;
            let rb = pb.radius;
            if dist > ra + rb && dist > FLT_EPSILON {
                let n = (p.norm)((p.sub)(wb, wa));
                let na = (p.add)(wa, (p.mulvs)(n, ra));
                let nb = (p.sub)(wb, (p.mulvs)(n, rb));
                if na.x == nb.x && na.y == nb.y {
                    RadiusArm::ShrinkCollapsed
                } else {
                    RadiusArm::Shrink
                }
            } else {
                RadiusArm::Midpoint
            }
        };

        Classification {
            brk,
            radius,
            iters: iter,
            final_count: s.count,
            cache_read,
        }
    }
}
