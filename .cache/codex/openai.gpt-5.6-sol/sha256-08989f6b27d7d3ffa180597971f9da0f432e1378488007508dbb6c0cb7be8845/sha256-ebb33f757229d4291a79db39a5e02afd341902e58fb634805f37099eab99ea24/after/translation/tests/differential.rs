#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr::{null, null_mut};

const CIRCLE: c_int = 0;
const AABB: c_int = 1;
const CAPSULE: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct V {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct R {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct X {
    p: V,
    r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Circle {
    p: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Capsule {
    a: V,
    b: V,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Cache {
    metric: c_float,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Proxy {
    radius: c_float,
    count: c_int,
    verts: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Sv {
    s_a: V,
    s_b: V,
    p: V,
    u: c_float,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: c_float,
    count: c_int,
}

type FnV = unsafe extern "C" fn(c_float, c_float) -> V;
type FnVfV = unsafe extern "C" fn(V, c_float) -> V;
type FnVvV = unsafe extern "C" fn(V, V) -> V;
type FnVvvV = unsafe extern "C" fn(V, V, V) -> V;
type FnVvF = unsafe extern "C" fn(V, V) -> c_float;
type FnVR = unsafe extern "C" fn() -> R;
type FnVX = unsafe extern "C" fn() -> X;
type FnBbVerts = unsafe extern "C" fn(*mut V, *mut Aabb);
type FnMakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
type FnVF = unsafe extern "C" fn(V) -> c_float;
type FnSimplexF = unsafe extern "C" fn(*mut Simplex) -> c_float;
type FnRvV = unsafe extern "C" fn(R, V) -> V;
type FnXvV = unsafe extern "C" fn(X, V) -> V;
type FnSimplexVoid = unsafe extern "C" fn(*mut Simplex);
type FnVV = unsafe extern "C" fn(V) -> V;
type FnSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
type FnSupport = unsafe extern "C" fn(*const V, c_int, V) -> c_int;
type FnWitness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);
type FnGjk = unsafe extern "C" fn(
    *const c_void,
    c_int,
    *const X,
    *const c_void,
    c_int,
    *const X,
    *mut V,
    *mut V,
    c_int,
    *mut c_int,
    *mut Cache,
) -> c_float;
type FnAabbAabb = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
type FnAabbCapsule = unsafe extern "C" fn(Aabb, Capsule) -> c_int;
type FnCapsuleCapsule = unsafe extern "C" fn(Capsule, Capsule) -> c_int;
type FnCircleCircle = unsafe extern "C" fn(Circle, Circle) -> c_int;
type FnCircleAabb = unsafe extern "C" fn(Circle, Aabb) -> c_int;
type FnCircleCapsule = unsafe extern "C" fn(Circle, Capsule) -> c_int;
type FnCollided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
type FnCapsule = unsafe extern "C" fn(c_float, c_float, c_float, c_float, c_float) -> c_int;

#[derive(Clone, Copy)]
struct Api {
    c22: FnSimplexVoid,
    c23: FnSimplexVoid,
    aabb_aabb: FnAabbAabb,
    aabb_capsule: FnAabbCapsule,
    add: FnVvV,
    bb_verts: FnBbVerts,
    ccw90: FnVV,
    capsule_capsule: FnCapsuleCapsule,
    circle_aabb: FnCircleAabb,
    circle_capsule: FnCircleCapsule,
    circle_circle: FnCircleCircle,
    clamp: FnVvvV,
    collided: FnCollided,
    direction: FnSimplexV,
    det: FnVvF,
    div: FnVfV,
    dot: FnVvF,
    gjk: FnGjk,
    metric: FnSimplexF,
    closest: FnSimplexV,
    len: FnVF,
    make_proxy: FnMakeProxy,
    max: FnVvV,
    min: FnVvV,
    mul_rv: FnRvV,
    mul_rv_t: FnRvV,
    mul_vs: FnVfV,
    mul_xv: FnXvV,
    neg: FnVV,
    norm: FnVV,
    rot_identity: FnVR,
    skew: FnVV,
    sub: FnVvV,
    support: FnSupport,
    v: FnV,
    witness: FnWitness,
    x_identity: FnVX,
    capsule: FnCapsule,
}

struct Harness {
    _c_lib: Library,
    _r_lib: Library,
    c: Api,
    r: Api,
}

unsafe fn load<T: Copy>(lib: &Library, name: &[u8]) -> T {
    *lib.get::<T>(name).unwrap()
}

unsafe fn load_api(lib: &Library) -> Api {
    Api {
        c22: load(lib, b"c22\0"),
        c23: load(lib, b"c23\0"),
        aabb_aabb: load(lib, b"c2AABBtoAABB\0"),
        aabb_capsule: load(lib, b"c2AABBtoCapsule\0"),
        add: load(lib, b"c2Add\0"),
        bb_verts: load(lib, b"c2BBVerts\0"),
        ccw90: load(lib, b"c2CCW90\0"),
        capsule_capsule: load(lib, b"c2CapsuletoCapsule\0"),
        circle_aabb: load(lib, b"c2CircletoAABB\0"),
        circle_capsule: load(lib, b"c2CircletoCapsule\0"),
        circle_circle: load(lib, b"c2CircletoCircle\0"),
        clamp: load(lib, b"c2Clampv\0"),
        collided: load(lib, b"c2Collided\0"),
        direction: load(lib, b"c2D\0"),
        det: load(lib, b"c2Det2\0"),
        div: load(lib, b"c2Div\0"),
        dot: load(lib, b"c2Dot\0"),
        gjk: load(lib, b"c2GJK\0"),
        metric: load(lib, b"c2GJKSimplexMetric\0"),
        closest: load(lib, b"c2L\0"),
        len: load(lib, b"c2Len\0"),
        make_proxy: load(lib, b"c2MakeProxy\0"),
        max: load(lib, b"c2Maxv\0"),
        min: load(lib, b"c2Minv\0"),
        mul_rv: load(lib, b"c2Mulrv\0"),
        mul_rv_t: load(lib, b"c2MulrvT\0"),
        mul_vs: load(lib, b"c2Mulvs\0"),
        mul_xv: load(lib, b"c2Mulxv\0"),
        neg: load(lib, b"c2Neg\0"),
        norm: load(lib, b"c2Norm\0"),
        rot_identity: load(lib, b"c2RotIdentity\0"),
        skew: load(lib, b"c2Skew\0"),
        sub: load(lib, b"c2Sub\0"),
        support: load(lib, b"c2Support\0"),
        v: load(lib, b"c2V\0"),
        witness: load(lib, b"c2Witness\0"),
        x_identity: load(lib, b"c2xIdentity\0"),
        capsule: load(lib, b"capsule\0"),
    }
}

impl Harness {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-eNZ0uT.so");
        let r_path = root.join("target/release/libcapsule_lib.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            r_path.is_file(),
            "missing Rust library: {}",
            r_path.display()
        );
        unsafe {
            let c_lib = Library::new(c_path).unwrap();
            let r_lib = Library::new(r_path).unwrap();
            let c = load_api(&c_lib);
            let r = load_api(&r_lib);
            Self {
                _c_lib: c_lib,
                _r_lib: r_lib,
                c,
                r,
            }
        }
    }
}

fn v(x: f32, y: f32) -> V {
    V { x, y }
}

fn zero_sv() -> Sv {
    Sv {
        s_a: v(0.0, 0.0),
        s_b: v(0.0, 0.0),
        p: v(0.0, 0.0),
        u: 0.0,
        i_a: 0,
        i_b: 0,
    }
}

fn simplex() -> Simplex {
    Simplex {
        a: zero_sv(),
        b: zero_sv(),
        c: zero_sv(),
        d: zero_sv(),
        div: 1.0,
        count: 0,
    }
}

fn cache() -> Cache {
    Cache {
        metric: 0.0,
        count: 0,
        i_a: [0; 3],
        i_b: [0; 3],
        div: 0.0,
    }
}

fn proxy_sentinel() -> Proxy {
    Proxy {
        radius: f32::from_bits(0x7fc0_1234),
        count: 0x1234_5678,
        verts: [v(123.25, -456.5); 8],
    }
}

unsafe fn bytes<T>(value: &T) -> &[u8] {
    std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>())
}

fn eq_f(c: f32, r: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{context}: C={c:?} ({:#010x}) Rust={r:?} ({:#010x})",
        c.to_bits(),
        r.to_bits()
    );
}

fn eq_v(c: V, r: V, context: &str) {
    eq_f(c.x, r.x, &format!("{context}.x"));
    eq_f(c.y, r.y, &format!("{context}.y"));
}

fn eq_bytes<T>(c: &T, r: &T, context: &str) {
    unsafe { assert_eq!(bytes(c), bytes(r), "{context}") }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn f(&mut self) -> f32 {
        let n = (self.next_u32() % 8193) as i32 - 4096;
        n as f32 / 16.0
    }

    fn positive(&mut self) -> f32 {
        (self.next_u32() % 1024) as f32 / 32.0 + 0.03125
    }

    fn vec(&mut self) -> V {
        v(self.f(), self.f())
    }
}

#[test]
fn all_symbols_load_from_both_shared_objects() {
    let h = Harness::new();
    let _ = (h.c.capsule, h.r.capsule);
}

#[test]
fn vector_transform_and_proxy_surface() {
    let h = Harness::new();
    let mut rng = Rng::new(0x8f31_2a77_d00d_beef);
    unsafe {
        eq_bytes(
            &(h.c.rot_identity)(),
            &(h.r.rot_identity)(),
            "c2RotIdentity",
        );
        eq_bytes(&(h.c.x_identity)(), &(h.r.x_identity)(), "c2xIdentity");

        for i in 0..512 {
            let a = rng.vec();
            let b = rng.vec();
            let scalar = rng.f();
            let lo = v(a.x.min(b.x), a.y.min(b.y));
            let hi = v(a.x.max(b.x), a.y.max(b.y));
            let x = X {
                p: rng.vec(),
                r: R {
                    c: rng.f(),
                    s: rng.f(),
                },
            };
            eq_v((h.c.v)(a.x, a.y), (h.r.v)(a.x, a.y), &format!("c2V {i}"));
            eq_v(
                (h.c.mul_vs)(a, scalar),
                (h.r.mul_vs)(a, scalar),
                &format!("c2Mulvs {i}"),
            );
            eq_v((h.c.max)(a, b), (h.r.max)(a, b), &format!("c2Maxv {i}"));
            eq_v((h.c.min)(a, b), (h.r.min)(a, b), &format!("c2Minv {i}"));
            eq_v(
                (h.c.clamp)(a, lo, hi),
                (h.r.clamp)(a, lo, hi),
                &format!("c2Clampv {i}"),
            );
            eq_v((h.c.sub)(a, b), (h.r.sub)(a, b), &format!("c2Sub {i}"));
            eq_f((h.c.dot)(a, b), (h.r.dot)(a, b), &format!("c2Dot {i}"));
            eq_v((h.c.add)(a, b), (h.r.add)(a, b), &format!("c2Add {i}"));
            eq_v((h.c.neg)(a), (h.r.neg)(a), &format!("c2Neg {i}"));
            eq_v((h.c.skew)(a), (h.r.skew)(a), &format!("c2Skew {i}"));
            eq_v((h.c.ccw90)(a), (h.r.ccw90)(a), &format!("c2CCW90 {i}"));
            eq_f((h.c.det)(a, b), (h.r.det)(a, b), &format!("c2Det2 {i}"));
            eq_f((h.c.len)(a), (h.r.len)(a), &format!("c2Len {i}"));
            eq_v(
                (h.c.mul_rv)(x.r, a),
                (h.r.mul_rv)(x.r, a),
                &format!("c2Mulrv {i}"),
            );
            eq_v(
                (h.c.mul_rv_t)(x.r, a),
                (h.r.mul_rv_t)(x.r, a),
                &format!("c2MulrvT {i}"),
            );
            eq_v(
                (h.c.mul_xv)(x, a),
                (h.r.mul_xv)(x, a),
                &format!("c2Mulxv {i}"),
            );
            if scalar != 0.0 {
                eq_v(
                    (h.c.div)(a, scalar),
                    (h.r.div)(a, scalar),
                    &format!("c2Div {i}"),
                );
            }
            if a.x != 0.0 || a.y != 0.0 {
                eq_v((h.c.norm)(a), (h.r.norm)(a), &format!("c2Norm {i}"));
            }

            let mut c_bb = Aabb { min: a, max: b };
            let mut r_bb = c_bb;
            let mut c_out = [v(0.0, 0.0); 4];
            let mut r_out = c_out;
            (h.c.bb_verts)(c_out.as_mut_ptr(), &mut c_bb);
            (h.r.bb_verts)(r_out.as_mut_ptr(), &mut r_bb);
            eq_bytes(&c_out, &r_out, &format!("c2BBVerts {i}"));
        }

        let nan = f32::from_bits(0x7fc0_4321);
        for (a, b) in [(v(nan, 1.0), v(2.0, nan)), (v(0.0, -0.0), v(-0.0, 0.0))] {
            eq_v((h.c.max)(a, b), (h.r.max)(a, b), "c2Maxv special");
            eq_v((h.c.min)(a, b), (h.r.min)(a, b), "c2Minv special");
        }
        for point in [v(-3.0, 8.0), v(3.0, 5.0), v(12.0, -4.0)] {
            eq_v(
                (h.c.clamp)(point, v(0.0, 0.0), v(10.0, 10.0)),
                (h.r.clamp)(point, v(0.0, 0.0), v(10.0, 10.0)),
                "c2Clampv below/inside/above",
            );
        }
        for (a, b) in [(v(0.0, 0.0), v(12.0, -9.0)), (v(1.0, 0.0), v(0.0, 1.0))] {
            eq_f((h.c.dot)(a, b), (h.r.dot)(a, b), "c2Dot boundary");
        }
        eq_f((h.c.len)(v(0.0, 0.0)), (h.r.len)(v(0.0, 0.0)), "c2Len zero");
        for (a, b) in [
            (v(1.0, 0.0), v(0.0, 1.0)),
            (v(0.0, 1.0), v(1.0, 0.0)),
            (v(1.0, 1.0), v(2.0, 2.0)),
        ] {
            eq_f((h.c.det)(a, b), (h.r.det)(a, b), "c2Det2 sign");
        }
        eq_v(
            (h.c.div)(v(1.0, 0.0), 0.0),
            (h.r.div)(v(1.0, 0.0), 0.0),
            "c2Div zero",
        );
        eq_v(
            (h.c.norm)(v(0.0, 0.0)),
            (h.r.norm)(v(0.0, 0.0)),
            "c2Norm zero",
        );
        for mut bb in [
            Aabb {
                min: v(2.0, 2.0),
                max: v(2.0, 2.0),
            },
            Aabb {
                min: v(4.0, 7.0),
                max: v(-3.0, -8.0),
            },
        ] {
            let mut rb = bb;
            let mut co = [v(0.0, 0.0); 4];
            let mut ro = co;
            (h.c.bb_verts)(co.as_mut_ptr(), &mut bb);
            (h.r.bb_verts)(ro.as_mut_ptr(), &mut rb);
            eq_bytes(&co, &ro, "c2BBVerts zero/reversed");
        }

        for ty in [CIRCLE, AABB, CAPSULE] {
            for i in 0..256 {
                let circle = Circle {
                    p: rng.vec(),
                    r: rng.f(),
                };
                let a = rng.vec();
                let b = rng.vec();
                let aabb = Aabb { min: a, max: b };
                let capsule = Capsule { a, b, r: rng.f() };
                let shape = match ty {
                    CIRCLE => (&circle as *const Circle).cast(),
                    AABB => (&aabb as *const Aabb).cast(),
                    _ => (&capsule as *const Capsule).cast(),
                };
                let mut cp = proxy_sentinel();
                let mut rp = cp;
                (h.c.make_proxy)(shape, ty, &mut cp);
                (h.r.make_proxy)(shape, ty, &mut rp);
                eq_bytes(&cp, &rp, &format!("c2MakeProxy type={ty} sample={i}"));
            }
        }

        let mut cp = proxy_sentinel();
        let mut rp = cp;
        (h.c.make_proxy)(null(), 99, &mut cp);
        (h.r.make_proxy)(null(), 99, &mut rp);
        eq_bytes(&cp, &rp, "c2MakeProxy invalid enum");
        eq_bytes(&cp, &proxy_sentinel(), "c2MakeProxy invalid enum untouched");
    }
}

#[test]
fn simplex_support_and_witness_surface() {
    let h = Harness::new();
    let mut rng = Rng::new(0xa991_b2c3_d4e5_f607);
    unsafe {
        for count in [0, 1, 2, 3, 4, -1] {
            for i in 0..128 {
                let mut cs = simplex();
                cs.count = count;
                cs.a.p = rng.vec();
                cs.b.p = rng.vec();
                cs.c.p = rng.vec();
                let mut rs = cs;
                eq_f(
                    (h.c.metric)(&mut cs),
                    (h.r.metric)(&mut rs),
                    &format!("metric count={count} {i}"),
                );
            }
        }

        let c22_cases = [
            (v(0.0, 0.0), v(1.0, 0.0)),
            (v(1.0, 0.0), v(0.0, 0.0)),
            (v(-1.0, 0.0), v(1.0, 0.0)),
        ];
        for (i, (a, b)) in c22_cases.into_iter().enumerate() {
            for sample in 0..128 {
                let scale = rng.positive();
                let mut cs = simplex();
                cs.count = 2;
                cs.a.p = v(a.x * scale, a.y * scale);
                cs.b.p = v(b.x * scale, b.y * scale);
                cs.a.i_a = 11;
                cs.b.i_a = 22;
                let mut rs = cs;
                (h.c.c22)(&mut cs);
                (h.r.c22)(&mut rs);
                eq_bytes(&cs, &rs, &format!("c22 branch {i} sample={sample}"));
            }
        }

        let c23_cases = [
            ([v(1.0, 0.0), v(2.0, 1.0), v(2.0, -1.0)], (1, 10, 0)),
            ([v(2.0, 1.0), v(1.0, 0.0), v(2.0, -1.0)], (1, 20, 0)),
            ([v(2.0, -1.0), v(2.0, 1.0), v(1.0, 0.0)], (1, 30, 0)),
            ([v(-1.0, 1.0), v(1.0, 1.0), v(0.0, 2.0)], (2, 10, 20)),
            ([v(0.0, 2.0), v(-1.0, 1.0), v(1.0, 1.0)], (2, 20, 30)),
            ([v(1.0, 1.0), v(0.0, 2.0), v(-1.0, 1.0)], (2, 30, 10)),
            ([v(-1.0, -1.0), v(1.0, -1.0), v(0.0, 1.0)], (3, 10, 20)),
        ];
        for (i, (points, expected)) in c23_cases.into_iter().enumerate() {
            for sample in 0..128 {
                let scale = rng.positive();
                let mut cs = simplex();
                cs.count = 3;
                cs.a.p = v(points[0].x * scale, points[0].y * scale);
                cs.b.p = v(points[1].x * scale, points[1].y * scale);
                cs.c.p = v(points[2].x * scale, points[2].y * scale);
                cs.a.i_a = 10;
                cs.b.i_a = 20;
                cs.c.i_a = 30;
                let mut rs = cs;
                (h.c.c23)(&mut cs);
                (h.r.c23)(&mut rs);
                eq_bytes(&cs, &rs, &format!("c23 branch {i} sample={sample}"));
                assert_eq!(
                    (cs.count, cs.a.i_a, if cs.count >= 2 { cs.b.i_a } else { 0 }),
                    expected,
                    "c23 case {i} sample={sample} did not reach the intended C branch"
                );
            }
        }

        for (count, a, b) in [
            (1, v(2.0, -3.0), v(0.0, 0.0)),
            (2, v(1.0, 0.0), v(1.0, 1.0)),
            (2, v(1.0, 0.0), v(1.0, -1.0)),
            (3, v(2.0, 3.0), v(4.0, 5.0)),
            (0, v(2.0, 3.0), v(4.0, 5.0)),
        ] {
            let mut cs = simplex();
            cs.count = count;
            cs.a.p = a;
            cs.b.p = b;
            let mut rs = cs;
            eq_v((h.c.direction)(&mut cs), (h.r.direction)(&mut rs), "c2D");
        }

        let one = [v(3.0, 4.0)];
        assert_eq!(
            (h.c.support)(one.as_ptr(), 1, v(1.0, 1.0)),
            (h.r.support)(one.as_ptr(), 1, v(1.0, 1.0))
        );
        assert_eq!(
            (h.c.support)(one.as_ptr(), 0, v(1.0, 1.0)),
            (h.r.support)(one.as_ptr(), 0, v(1.0, 1.0))
        );
        let tied = [v(1.0, 0.0), v(1.0, 2.0), v(0.0, 8.0)];
        let ci = (h.c.support)(tied.as_ptr(), tied.len() as c_int, v(1.0, 0.0));
        let ri = (h.r.support)(tied.as_ptr(), tied.len() as c_int, v(1.0, 0.0));
        assert_eq!((ci, ri), (0, 0));
        let unique = [v(-3.0, 1.0), v(9.0, 2.0), v(4.0, 8.0)];
        assert_eq!(
            (h.c.support)(unique.as_ptr(), 3, v(1.0, 0.0)),
            (h.r.support)(unique.as_ptr(), 3, v(1.0, 0.0))
        );
        for count in [2usize, 3, 4, 9] {
            for i in 0..256 {
                let mut verts = [v(0.0, 0.0); 9];
                for item in &mut verts[..count] {
                    *item = rng.vec();
                }
                let d = rng.vec();
                assert_eq!(
                    (h.c.support)(verts.as_ptr(), count as c_int, d),
                    (h.r.support)(verts.as_ptr(), count as c_int, d),
                    "c2Support count={count} sample={i}"
                );
            }
        }

        for count in [0, 1, 2, 3, 4] {
            for i in 0..256 {
                let mut cs = simplex();
                cs.count = count;
                cs.div = rng.positive();
                for sv in [&mut cs.a, &mut cs.b, &mut cs.c] {
                    sv.s_a = rng.vec();
                    sv.s_b = rng.vec();
                    sv.p = rng.vec();
                    sv.u = rng.positive();
                }
                let mut rs = cs;
                let mut ca = v(91.0, 92.0);
                let mut cb = v(93.0, 94.0);
                let mut ra = ca;
                let mut rb = cb;
                (h.c.witness)(&mut cs, &mut ca, &mut cb);
                (h.r.witness)(&mut rs, &mut ra, &mut rb);
                eq_v(ca, ra, &format!("c2Witness A count={count} {i}"));
                eq_v(cb, rb, &format!("c2Witness B count={count} {i}"));
                eq_v(
                    (h.c.closest)(&mut cs),
                    (h.r.closest)(&mut rs),
                    &format!("c2L count={count} {i}"),
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Shapes {
    circle: Circle,
    aabb: Aabb,
    capsule: Capsule,
}

impl Shapes {
    fn pointer(&self, ty: c_int) -> *const c_void {
        match ty {
            CIRCLE => (&self.circle as *const Circle).cast(),
            AABB => (&self.aabb as *const Aabb).cast(),
            CAPSULE => (&self.capsule as *const Capsule).cast(),
            _ => null(),
        }
    }
}

fn random_shapes(rng: &mut Rng) -> Shapes {
    let center = rng.vec();
    let half = v(rng.positive(), rng.positive());
    Shapes {
        circle: Circle {
            p: center,
            r: rng.positive(),
        },
        aabb: Aabb {
            min: v(center.x - half.x, center.y - half.y),
            max: v(center.x + half.x, center.y + half.y),
        },
        capsule: Capsule {
            a: rng.vec(),
            b: rng.vec(),
            r: rng.positive(),
        },
    }
}

unsafe fn compare_gjk_call(
    h: &Harness,
    a: &Shapes,
    type_a: c_int,
    ax: Option<X>,
    b: &Shapes,
    type_b: c_int,
    bx: Option<X>,
    use_radius: c_int,
    cache_mode: c_int,
    outputs: bool,
    context: &str,
) {
    let mut cc = cache();
    let mut rc = cache();
    let ax_storage = ax.unwrap_or(X {
        p: v(0.0, 0.0),
        r: R { c: 1.0, s: 0.0 },
    });
    let bx_storage = bx.unwrap_or(X {
        p: v(0.0, 0.0),
        r: R { c: 1.0, s: 0.0 },
    });
    let axp = if ax.is_some() { &ax_storage } else { null() };
    let bxp = if bx.is_some() { &bx_storage } else { null() };

    if cache_mode == 2 {
        (h.c.gjk)(
            a.pointer(type_a),
            type_a,
            axp,
            b.pointer(type_b),
            type_b,
            bxp,
            null_mut(),
            null_mut(),
            use_radius,
            null_mut(),
            &mut cc,
        );
        (h.r.gjk)(
            a.pointer(type_a),
            type_a,
            axp,
            b.pointer(type_b),
            type_b,
            bxp,
            null_mut(),
            null_mut(),
            use_radius,
            null_mut(),
            &mut rc,
        );
        eq_bytes(&cc, &rc, &format!("{context} warmup cache"));
    }

    let ccp = if cache_mode == 0 { null_mut() } else { &mut cc };
    let rcp = if cache_mode == 0 { null_mut() } else { &mut rc };
    let mut ca = v(77.0, 78.0);
    let mut cb = v(79.0, 80.0);
    let mut ra = ca;
    let mut rb = cb;
    let mut ci = -99;
    let mut ri = -99;
    let (cap, cbp, cip) = if outputs {
        (&mut ca as *mut V, &mut cb as *mut V, &mut ci as *mut c_int)
    } else {
        (null_mut(), null_mut(), null_mut())
    };
    let (rap, rbp, rip) = if outputs {
        (&mut ra as *mut V, &mut rb as *mut V, &mut ri as *mut c_int)
    } else {
        (null_mut(), null_mut(), null_mut())
    };
    let cd = (h.c.gjk)(
        a.pointer(type_a),
        type_a,
        axp,
        b.pointer(type_b),
        type_b,
        bxp,
        cap,
        cbp,
        use_radius,
        cip,
        ccp,
    );
    let rd = (h.r.gjk)(
        a.pointer(type_a),
        type_a,
        axp,
        b.pointer(type_b),
        type_b,
        bxp,
        rap,
        rbp,
        use_radius,
        rip,
        rcp,
    );
    eq_f(cd, rd, &format!("{context} distance"));
    if outputs {
        eq_v(ca, ra, &format!("{context} outA"));
        eq_v(cb, rb, &format!("{context} outB"));
        assert_eq!(ci, ri, "{context} iterations");
    }
    if cache_mode != 0 {
        eq_bytes(&cc, &rc, &format!("{context} cache"));
    }
}

#[test]
fn gjk_full_option_and_shape_cross_product() {
    let h = Harness::new();
    let mut rng = Rng::new(0x1ee7_c0de_55aa_9031);
    unsafe {
        for type_a in [CIRCLE, AABB, CAPSULE] {
            for type_b in [CIRCLE, AABB, CAPSULE] {
                for sample in 0..64 {
                    let a = random_shapes(&mut rng);
                    let b = random_shapes(&mut rng);
                    for use_radius in [0, 1, -7] {
                        for transformed in [false, true] {
                            let ax = transformed.then(|| X {
                                p: rng.vec(),
                                r: R { c: 0.6, s: 0.8 },
                            });
                            let bx = transformed.then(|| X {
                                p: rng.vec(),
                                r: R { c: -0.8, s: 0.6 },
                            });
                            for cache_mode in [0, 1, 2] {
                                for outputs in [false, true] {
                                    compare_gjk_call(
                                        &h,
                                        &a,
                                        type_a,
                                        ax,
                                        &b,
                                        type_b,
                                        bx,
                                        use_radius,
                                        cache_mode,
                                        outputs,
                                        &format!(
                                            "types={type_a}/{type_b} sample={sample} radius={use_radius} transform={transformed} cache={cache_mode} outputs={outputs}"
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        let large = Shapes {
            circle: Circle {
                p: v(0.0, 0.0),
                r: 1.0,
            },
            aabb: Aabb {
                min: v(-10_000.0, -10_000.0),
                max: v(10_000.0, 10_000.0),
            },
            capsule: Capsule {
                a: v(-1.0, 0.0),
                b: v(1.0, 0.0),
                r: 1.0,
            },
        };
        let mut bad_c = Cache {
            metric: 1.0,
            count: 3,
            i_a: [0, 0, 0],
            i_b: [0, 2, 1],
            div: 1.0,
        };
        let mut bad_r = bad_c;
        let cd = (h.c.gjk)(
            large.pointer(AABB),
            AABB,
            null(),
            large.pointer(AABB),
            AABB,
            null(),
            null_mut(),
            null_mut(),
            0,
            null_mut(),
            &mut bad_c,
        );
        let rd = (h.r.gjk)(
            large.pointer(AABB),
            AABB,
            null(),
            large.pointer(AABB),
            AABB,
            null(),
            null_mut(),
            null_mut(),
            0,
            null_mut(),
            &mut bad_r,
        );
        eq_f(cd, rd, "rejected-cache distance");
        eq_bytes(&bad_c, &bad_r, "rejected-cache output");

        let focused = [
            (
                Shapes {
                    circle: Circle {
                        p: v(0.0, 0.0),
                        r: 1.0,
                    },
                    aabb: Aabb {
                        min: v(-1.0, -1.0),
                        max: v(1.0, 1.0),
                    },
                    capsule: Capsule {
                        a: v(-2.0, 0.0),
                        b: v(2.0, 0.0),
                        r: 1.0,
                    },
                },
                Shapes {
                    circle: Circle {
                        p: v(0.0, 0.0),
                        r: 1.0,
                    },
                    aabb: Aabb {
                        min: v(-1.0, -1.0),
                        max: v(1.0, 1.0),
                    },
                    capsule: Capsule {
                        a: v(-2.0, 0.0),
                        b: v(2.0, 0.0),
                        r: 1.0,
                    },
                },
            ),
            (
                Shapes {
                    circle: Circle {
                        p: v(-20.0, 0.0),
                        r: 2.0,
                    },
                    aabb: Aabb {
                        min: v(-22.0, -2.0),
                        max: v(-18.0, 2.0),
                    },
                    capsule: Capsule {
                        a: v(-22.0, 0.0),
                        b: v(-18.0, 0.0),
                        r: 2.0,
                    },
                },
                Shapes {
                    circle: Circle {
                        p: v(20.0, 0.0),
                        r: 2.0,
                    },
                    aabb: Aabb {
                        min: v(18.0, -2.0),
                        max: v(22.0, 2.0),
                    },
                    capsule: Capsule {
                        a: v(18.0, 0.0),
                        b: v(22.0, 0.0),
                        r: 2.0,
                    },
                },
            ),
        ];
        for (i, (a, b)) in focused.iter().enumerate() {
            compare_gjk_call(
                &h,
                a,
                CAPSULE,
                None,
                b,
                CAPSULE,
                None,
                1,
                1,
                true,
                &format!("focused {i}"),
            );
        }
        let touching_a = Shapes {
            circle: Circle {
                p: v(0.0, 0.0),
                r: 1.0,
            },
            aabb: focused[0].0.aabb,
            capsule: focused[0].0.capsule,
        };
        let touching_b = Shapes {
            circle: Circle {
                p: v(2.0, 0.0),
                r: 1.0,
            },
            aabb: focused[0].1.aabb,
            capsule: focused[0].1.capsule,
        };
        compare_gjk_call(
            &h,
            &touching_a,
            CIRCLE,
            None,
            &touching_b,
            CIRCLE,
            None,
            1,
            1,
            true,
            "focused touching circles",
        );
    }
}

#[test]
fn collision_dispatch_and_capsule_wrapper_surface() {
    let h = Harness::new();
    let mut rng = Rng::new(0x7654_3210_fedc_ba98);
    unsafe {
        let aabb_cases = [
            (
                Aabb {
                    min: v(0.0, 0.0),
                    max: v(2.0, 2.0),
                },
                Aabb {
                    min: v(1.0, 1.0),
                    max: v(3.0, 3.0),
                },
            ),
            (
                Aabb {
                    min: v(0.0, 0.0),
                    max: v(2.0, 2.0),
                },
                Aabb {
                    min: v(2.0, 0.0),
                    max: v(3.0, 1.0),
                },
            ),
            (
                Aabb {
                    min: v(0.0, 0.0),
                    max: v(2.0, 2.0),
                },
                Aabb {
                    min: v(3.0, 0.0),
                    max: v(4.0, 1.0),
                },
            ),
            (
                Aabb {
                    min: v(0.0, 0.0),
                    max: v(2.0, 2.0),
                },
                Aabb {
                    min: v(-4.0, 0.0),
                    max: v(-1.0, 1.0),
                },
            ),
            (
                Aabb {
                    min: v(0.0, 0.0),
                    max: v(2.0, 2.0),
                },
                Aabb {
                    min: v(0.0, 3.0),
                    max: v(1.0, 4.0),
                },
            ),
            (
                Aabb {
                    min: v(0.0, 0.0),
                    max: v(2.0, 2.0),
                },
                Aabb {
                    min: v(0.0, -4.0),
                    max: v(1.0, -1.0),
                },
            ),
            (
                Aabb {
                    min: v(0.0, 0.0),
                    max: v(10.0, 10.0),
                },
                Aabb {
                    min: v(2.0, 2.0),
                    max: v(3.0, 3.0),
                },
            ),
        ];
        for (a, b) in aabb_cases {
            assert_eq!((h.c.aabb_aabb)(a, b), (h.r.aabb_aabb)(a, b));
        }

        let circle_pairs = [
            (
                Circle {
                    p: v(0.0, 0.0),
                    r: 2.0,
                },
                Circle {
                    p: v(3.0, 0.0),
                    r: 2.0,
                },
            ),
            (
                Circle {
                    p: v(0.0, 0.0),
                    r: 2.0,
                },
                Circle {
                    p: v(4.0, 0.0),
                    r: 2.0,
                },
            ),
            (
                Circle {
                    p: v(0.0, 0.0),
                    r: 2.0,
                },
                Circle {
                    p: v(5.0, 0.0),
                    r: 2.0,
                },
            ),
        ];
        for (a, b) in circle_pairs {
            assert_eq!((h.c.circle_circle)(a, b), (h.r.circle_circle)(a, b));
        }

        let box0 = Aabb {
            min: v(-1.0, -1.0),
            max: v(1.0, 1.0),
        };
        for circle in [
            Circle {
                p: v(0.0, 0.0),
                r: 0.25,
            },
            Circle {
                p: v(1.5, 0.0),
                r: 1.0,
            },
            Circle {
                p: v(2.0, 0.0),
                r: 1.0,
            },
            Circle {
                p: v(1.5, 1.5),
                r: 1.0,
            },
            Circle {
                p: v(3.0, 3.0),
                r: 1.0,
            },
        ] {
            assert_eq!(
                (h.c.circle_aabb)(circle, box0),
                (h.r.circle_aabb)(circle, box0)
            );
        }

        let segment = Capsule {
            a: v(0.0, 0.0),
            b: v(10.0, 0.0),
            r: 1.0,
        };
        for circle in [
            Circle {
                p: v(-2.0, 0.0),
                r: 1.5,
            },
            Circle {
                p: v(5.0, 1.0),
                r: 0.5,
            },
            Circle {
                p: v(12.0, 0.0),
                r: 1.5,
            },
        ] {
            assert_eq!(
                (h.c.circle_capsule)(circle, segment),
                (h.r.circle_capsule)(circle, segment)
            );
        }

        for (aabb, capsule) in [
            (
                Aabb {
                    min: v(-1.0, -1.0),
                    max: v(1.0, 1.0),
                },
                Capsule {
                    a: v(-2.0, 0.0),
                    b: v(2.0, 0.0),
                    r: 0.5,
                },
            ),
            (
                Aabb {
                    min: v(-1.0, -1.0),
                    max: v(1.0, 1.0),
                },
                Capsule {
                    a: v(20.0, 0.0),
                    b: v(30.0, 0.0),
                    r: 0.5,
                },
            ),
        ] {
            assert_eq!(
                (h.c.aabb_capsule)(aabb, capsule),
                (h.r.aabb_capsule)(aabb, capsule)
            );
        }
        for (a, b) in [
            (
                Capsule {
                    a: v(-2.0, 0.0),
                    b: v(2.0, 0.0),
                    r: 1.0,
                },
                Capsule {
                    a: v(0.0, -2.0),
                    b: v(0.0, 2.0),
                    r: 1.0,
                },
            ),
            (
                Capsule {
                    a: v(-2.0, 0.0),
                    b: v(2.0, 0.0),
                    r: 1.0,
                },
                Capsule {
                    a: v(20.0, -2.0),
                    b: v(20.0, 2.0),
                    r: 1.0,
                },
            ),
        ] {
            assert_eq!((h.c.capsule_capsule)(a, b), (h.r.capsule_capsule)(a, b));
        }

        for sample in 0..1024 {
            let a = random_shapes(&mut rng);
            let b = random_shapes(&mut rng);
            assert_eq!(
                (h.c.aabb_capsule)(a.aabb, b.capsule),
                (h.r.aabb_capsule)(a.aabb, b.capsule),
                "aabb/capsule {sample}"
            );
            assert_eq!(
                (h.c.capsule_capsule)(a.capsule, b.capsule),
                (h.r.capsule_capsule)(a.capsule, b.capsule),
                "capsule/capsule {sample}"
            );
            assert_eq!(
                (h.c.circle_circle)(a.circle, b.circle),
                (h.r.circle_circle)(a.circle, b.circle),
                "circle/circle {sample}"
            );
            assert_eq!(
                (h.c.circle_aabb)(a.circle, b.aabb),
                (h.r.circle_aabb)(a.circle, b.aabb),
                "circle/aabb {sample}"
            );
            assert_eq!(
                (h.c.circle_capsule)(a.circle, b.capsule),
                (h.r.circle_capsule)(a.circle, b.capsule),
                "circle/capsule {sample}"
            );

            for type_a in [CIRCLE, AABB, CAPSULE] {
                for type_b in [CIRCLE, AABB, CAPSULE] {
                    assert_eq!(
                        (h.c.collided)(a.pointer(type_a), type_a, b.pointer(type_b), type_b),
                        (h.r.collided)(a.pointer(type_a), type_a, b.pointer(type_b), type_b),
                        "c2Collided types={type_a}/{type_b} sample={sample}"
                    );
                }
            }

            assert_eq!(
                (h.c.capsule)(
                    a.capsule.a.x,
                    a.capsule.a.y,
                    a.capsule.b.x,
                    a.capsule.b.y,
                    a.capsule.r
                ),
                (h.r.capsule)(
                    a.capsule.a.x,
                    a.capsule.a.y,
                    a.capsule.b.x,
                    a.capsule.b.y,
                    a.capsule.r
                ),
                "capsule wrapper {sample}"
            );
        }

        let wrapper_cases = [
            [139.01619, 72.01207, 49.53636, -50.459583, 19.098978],
            [-111.84172, 27.234076, 101.166855, -24.547712, 14.551576],
            [91.24735, 33.176983, -95.06487, -96.79142, 34.573826],
            [-57.920383, -27.742004, -67.20522, -16.05002, 22.48084],
            [-66.217995, 124.539406, 129.58119, -87.645676, 12.371104],
            [-32.67715, 103.32543, -115.92492, -67.14575, 40.12823],
            [8.92985, -24.737625, -69.34136, 76.394775, 16.76948],
            [-75.257935, -54.5561, 5.241578, 53.851994, 35.85483],
        ];
        let mut observed = [false; 8];
        for values in wrapper_cases {
            let c = (h.c.capsule)(values[0], values[1], values[2], values[3], values[4]);
            let r = (h.r.capsule)(values[0], values[1], values[2], values[3], values[4]);
            assert_eq!(c, r);
            observed[c as usize] = true;
        }
        assert!(observed.into_iter().all(|value| value));
    }
}

#[test]
fn explicit_error_surface_invalid_enums() {
    let h = Harness::new();
    unsafe {
        for invalid in [-1, 3, 99, c_int::MIN, c_int::MAX] {
            assert_eq!((h.c.collided)(null(), invalid, null(), CIRCLE), 0);
            assert_eq!((h.r.collided)(null(), invalid, null(), CIRCLE), 0);
            for valid_a in [CIRCLE, AABB, CAPSULE] {
                assert_eq!((h.c.collided)(null(), valid_a, null(), invalid), 0);
                assert_eq!((h.r.collided)(null(), valid_a, null(), invalid), 0);
            }
        }
    }
}
