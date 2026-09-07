use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2GjkCache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
}

const ZERO_V: C2v = C2v { x: 0.0, y: 0.0 };
const ZERO_SV: C2sv = C2sv {
    s_a: ZERO_V,
    s_b: ZERO_V,
    p: ZERO_V,
    u: 0.0,
    i_a: 0,
    i_b: 0,
};

fn zero_simplex() -> C2Simplex {
    C2Simplex {
        a: ZERO_SV,
        b: ZERO_SV,
        c: ZERO_SV,
        d: ZERO_SV,
        div: 0.0,
        count: 0,
    }
}

struct Api {
    library: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            library: unsafe { Library::new(path) }.unwrap(),
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.library.get::<T>(name) }.unwrap()
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libharvest-work-rzwpTA.so"),
        manifest.join("target/release/libgjk_cache_lib.so"),
    )
}

unsafe fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust release library: {}",
        rust_path.display()
    );
    (unsafe { Api::open(&c_path) }, unsafe {
        Api::open(&rust_path)
    })
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn assert_bytes<T>(left: &T, right: &T, context: &str) {
    assert_eq!(bytes(left), bytes(right), "{context}");
}

fn assert_f32(left: f32, right: f32, context: &str) {
    assert_eq!(left.to_bits(), right.to_bits(), "{context}");
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 16) as u32
    }

    fn f32(&mut self, scale: f32) -> f32 {
        let unit = (self.u32() as f64 / u32::MAX as f64) as f32;
        (unit * 2.0 - 1.0) * scale
    }

    fn positive(&mut self, max: f32) -> f32 {
        (self.f32(max).abs() + 0.01).min(max + 0.01)
    }

    fn vector(&mut self, scale: f32) -> C2v {
        C2v {
            x: self.f32(scale),
            y: self.f32(scale),
        }
    }
}

type Vec2Fn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type VecScalarFn = unsafe extern "C" fn(C2v, f32) -> C2v;
type Scalar2Fn = unsafe extern "C" fn(C2v, C2v) -> f32;

#[test]
fn all_c_exports_are_loadable_from_both_shared_libraries() {
    let names = [
        "c22",
        "c23",
        "c2Add",
        "c2BBVerts",
        "c2CCW90",
        "c2Clampv",
        "c2D",
        "c2Det2",
        "c2Div",
        "c2Dot",
        "c2GJK",
        "c2GJKSimplexMetric",
        "c2L",
        "c2Len",
        "c2MakeProxy",
        "c2Maxv",
        "c2Minv",
        "c2Mulrv",
        "c2MulrvT",
        "c2Mulvs",
        "c2Mulxv",
        "c2Neg",
        "c2Norm",
        "c2RotIdentity",
        "c2Skew",
        "c2Sub",
        "c2Support",
        "c2V",
        "c2Witness",
        "c2xIdentity",
        "gjk_cache",
    ];
    unsafe {
        let (c_api, rust_api) = apis();
        for name in names {
            let mut nul = name.as_bytes().to_vec();
            nul.push(0);
            let _: *mut c_void = c_api.symbol(&nul);
            let _: *mut c_void = rust_api.symbol(&nul);
        }
    }
}

#[test]
fn vector_transform_and_proxy_surface_matches() {
    unsafe {
        let (c, r) = apis();
        let c_v: unsafe extern "C" fn(f32, f32) -> C2v = c.symbol(b"c2V\0");
        let r_v: unsafe extern "C" fn(f32, f32) -> C2v = r.symbol(b"c2V\0");
        let c_mul: VecScalarFn = c.symbol(b"c2Mulvs\0");
        let r_mul: VecScalarFn = r.symbol(b"c2Mulvs\0");
        let c_max: Vec2Fn = c.symbol(b"c2Maxv\0");
        let r_max: Vec2Fn = r.symbol(b"c2Maxv\0");
        let c_min: Vec2Fn = c.symbol(b"c2Minv\0");
        let r_min: Vec2Fn = r.symbol(b"c2Minv\0");
        let c_clamp: unsafe extern "C" fn(C2v, C2v, C2v) -> C2v = c.symbol(b"c2Clampv\0");
        let r_clamp: unsafe extern "C" fn(C2v, C2v, C2v) -> C2v = r.symbol(b"c2Clampv\0");
        let c_sub: Vec2Fn = c.symbol(b"c2Sub\0");
        let r_sub: Vec2Fn = r.symbol(b"c2Sub\0");
        let c_add: Vec2Fn = c.symbol(b"c2Add\0");
        let r_add: Vec2Fn = r.symbol(b"c2Add\0");
        let c_dot: Scalar2Fn = c.symbol(b"c2Dot\0");
        let r_dot: Scalar2Fn = r.symbol(b"c2Dot\0");
        let c_det: Scalar2Fn = c.symbol(b"c2Det2\0");
        let r_det: Scalar2Fn = r.symbol(b"c2Det2\0");
        let c_neg: unsafe extern "C" fn(C2v) -> C2v = c.symbol(b"c2Neg\0");
        let r_neg: unsafe extern "C" fn(C2v) -> C2v = r.symbol(b"c2Neg\0");
        let c_skew: unsafe extern "C" fn(C2v) -> C2v = c.symbol(b"c2Skew\0");
        let r_skew: unsafe extern "C" fn(C2v) -> C2v = r.symbol(b"c2Skew\0");
        let c_ccw: unsafe extern "C" fn(C2v) -> C2v = c.symbol(b"c2CCW90\0");
        let r_ccw: unsafe extern "C" fn(C2v) -> C2v = r.symbol(b"c2CCW90\0");
        let c_len: unsafe extern "C" fn(C2v) -> f32 = c.symbol(b"c2Len\0");
        let r_len: unsafe extern "C" fn(C2v) -> f32 = r.symbol(b"c2Len\0");
        let c_div: VecScalarFn = c.symbol(b"c2Div\0");
        let r_div: VecScalarFn = r.symbol(b"c2Div\0");
        let c_norm: unsafe extern "C" fn(C2v) -> C2v = c.symbol(b"c2Norm\0");
        let r_norm: unsafe extern "C" fn(C2v) -> C2v = r.symbol(b"c2Norm\0");
        let c_rot_id: unsafe extern "C" fn() -> C2r = c.symbol(b"c2RotIdentity\0");
        let r_rot_id: unsafe extern "C" fn() -> C2r = r.symbol(b"c2RotIdentity\0");
        let c_x_id: unsafe extern "C" fn() -> C2x = c.symbol(b"c2xIdentity\0");
        let r_x_id: unsafe extern "C" fn() -> C2x = r.symbol(b"c2xIdentity\0");
        let c_mulrv: unsafe extern "C" fn(C2r, C2v) -> C2v = c.symbol(b"c2Mulrv\0");
        let r_mulrv: unsafe extern "C" fn(C2r, C2v) -> C2v = r.symbol(b"c2Mulrv\0");
        let c_mulrvt: unsafe extern "C" fn(C2r, C2v) -> C2v = c.symbol(b"c2MulrvT\0");
        let r_mulrvt: unsafe extern "C" fn(C2r, C2v) -> C2v = r.symbol(b"c2MulrvT\0");
        let c_mulxv: unsafe extern "C" fn(C2x, C2v) -> C2v = c.symbol(b"c2Mulxv\0");
        let r_mulxv: unsafe extern "C" fn(C2x, C2v) -> C2v = r.symbol(b"c2Mulxv\0");
        let c_bb: unsafe extern "C" fn(*mut C2v, *mut C2Aabb) = c.symbol(b"c2BBVerts\0");
        let r_bb: unsafe extern "C" fn(*mut C2v, *mut C2Aabb) = r.symbol(b"c2BBVerts\0");
        let c_proxy: unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy) =
            c.symbol(b"c2MakeProxy\0");
        let r_proxy: unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy) =
            r.symbol(b"c2MakeProxy\0");

        assert_bytes(&c_rot_id(), &r_rot_id(), "rotation identity");
        assert_bytes(&c_x_id(), &r_x_id(), "transform identity");

        let mut rng = Rng::new(0x91d5_75a7_8c31_2b09);
        for iteration in 0..512 {
            let a = rng.vector(100.0);
            let b = rng.vector(100.0);
            let lo = rng.vector(50.0);
            let hi = rng.vector(50.0);
            let scalar = rng.f32(10.0);
            let divisor = rng.positive(10.0);
            let rotation = C2r {
                c: rng.f32(2.0),
                s: rng.f32(2.0),
            };
            let transform = C2x {
                p: rng.vector(100.0),
                r: rotation,
            };

            assert_bytes(&c_v(a.x, a.y), &r_v(a.x, a.y), "c2V");
            assert_bytes(&c_mul(a, scalar), &r_mul(a, scalar), "c2Mulvs");
            assert_bytes(&c_max(a, b), &r_max(a, b), "c2Maxv");
            assert_bytes(&c_min(a, b), &r_min(a, b), "c2Minv");
            assert_bytes(&c_clamp(a, lo, hi), &r_clamp(a, lo, hi), "c2Clampv");
            assert_bytes(&c_sub(a, b), &r_sub(a, b), "c2Sub");
            assert_bytes(&c_add(a, b), &r_add(a, b), "c2Add");
            assert_f32(c_dot(a, b), r_dot(a, b), "c2Dot");
            assert_f32(c_det(a, b), r_det(a, b), "c2Det2");
            assert_bytes(&c_neg(a), &r_neg(a), "c2Neg");
            assert_bytes(&c_skew(a), &r_skew(a), "c2Skew");
            assert_bytes(&c_ccw(a), &r_ccw(a), "c2CCW90");
            assert_f32(c_len(a), r_len(a), "c2Len");
            assert_bytes(&c_div(a, divisor), &r_div(a, divisor), "c2Div");
            if a.x != 0.0 || a.y != 0.0 {
                assert_bytes(&c_norm(a), &r_norm(a), "c2Norm");
            }
            assert_bytes(&c_mulrv(rotation, a), &r_mulrv(rotation, a), "c2Mulrv");
            assert_bytes(&c_mulrvt(rotation, a), &r_mulrvt(rotation, a), "c2MulrvT");
            assert_bytes(&c_mulxv(transform, a), &r_mulxv(transform, a), "c2Mulxv");

            let mut c_box = C2Aabb { min: a, max: b };
            let mut r_box = c_box;
            let mut c_verts = [C2v {
                x: f32::from_bits(0x7fc0_0101),
                y: f32::from_bits(0x7fc0_0202),
            }; 4];
            let mut r_verts = c_verts;
            c_bb(c_verts.as_mut_ptr(), &mut c_box);
            r_bb(r_verts.as_mut_ptr(), &mut r_box);
            assert_bytes(&c_verts, &r_verts, "c2BBVerts");

            let sentinel = C2Proxy {
                radius: f32::from_bits(0x7fc0_0303),
                count: -7788,
                verts: [C2v {
                    x: f32::from_bits(0x7fc0_0404),
                    y: f32::from_bits(0x7fc0_0505),
                }; 8],
            };
            let circle = C2Circle {
                p: a,
                r: rng.positive(20.0),
            };
            let capsule = C2Capsule {
                a,
                b,
                r: rng.positive(20.0),
            };
            for (shape, kind) in [
                ((&circle as *const C2Circle).cast::<c_void>(), 0),
                ((&c_box as *const C2Aabb).cast::<c_void>(), 1),
                ((&capsule as *const C2Capsule).cast::<c_void>(), 2),
            ] {
                let mut cp = sentinel;
                let mut rp = sentinel;
                c_proxy(shape, kind, &mut cp);
                r_proxy(shape, kind, &mut rp);
                assert_bytes(
                    &cp,
                    &rp,
                    &format!("c2MakeProxy kind {kind} iter {iteration}"),
                );
            }
        }

        let specials = [
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc1_2345),
        ];
        for &x in &specials {
            for &y in &specials {
                let a = C2v { x, y };
                let b = C2v { x: y, y: x };
                assert_bytes(&c_add(a, b), &r_add(a, b), "special c2Add");
                assert_bytes(&c_sub(a, b), &r_sub(a, b), "special c2Sub");
                assert_bytes(&c_mul(a, y), &r_mul(a, y), "special c2Mulvs");
                assert_f32(c_dot(a, b), r_dot(a, b), "special c2Dot");
            }
        }
    }
}

fn dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}

fn sub(a: C2v, b: C2v) -> C2v {
    C2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn det(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

fn c22_region(a: C2v, b: C2v) -> usize {
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

fn c23_region(a: C2v, b: C2v, c: C2v) -> usize {
    let u_ab = dot(b, sub(b, a));
    let v_ab = dot(a, sub(a, b));
    let u_bc = dot(c, sub(c, b));
    let v_bc = dot(b, sub(b, c));
    let u_ca = dot(a, sub(a, c));
    let v_ca = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let u_abc = det(b, c) * area;
    let v_abc = det(c, a) * area;
    let w_abc = det(a, b) * area;
    if v_ab <= 0.0 && u_ca <= 0.0 {
        0
    } else if u_ab <= 0.0 && v_bc <= 0.0 {
        1
    } else if u_bc <= 0.0 && v_ca <= 0.0 {
        2
    } else if u_ab > 0.0 && v_ab > 0.0 && w_abc <= 0.0 {
        3
    } else if u_bc > 0.0 && v_bc > 0.0 && u_abc <= 0.0 {
        4
    } else if u_ca > 0.0 && v_ca > 0.0 && v_abc <= 0.0 {
        5
    } else {
        6
    }
}

fn random_sv(rng: &mut Rng) -> C2sv {
    C2sv {
        s_a: rng.vector(20.0),
        s_b: rng.vector(20.0),
        p: rng.vector(20.0),
        u: rng.positive(10.0),
        i_a: (rng.u32() % 8) as c_int,
        i_b: (rng.u32() % 8) as c_int,
    }
}

#[test]
fn simplex_support_and_witness_surface_matches() {
    unsafe {
        let (c, r) = apis();
        let c_metric: unsafe extern "C" fn(*mut C2Simplex) -> f32 =
            c.symbol(b"c2GJKSimplexMetric\0");
        let r_metric: unsafe extern "C" fn(*mut C2Simplex) -> f32 =
            r.symbol(b"c2GJKSimplexMetric\0");
        let c22: unsafe extern "C" fn(*mut C2Simplex) = c.symbol(b"c22\0");
        let r22: unsafe extern "C" fn(*mut C2Simplex) = r.symbol(b"c22\0");
        let c23: unsafe extern "C" fn(*mut C2Simplex) = c.symbol(b"c23\0");
        let r23: unsafe extern "C" fn(*mut C2Simplex) = r.symbol(b"c23\0");
        let c_d: unsafe extern "C" fn(*mut C2Simplex) -> C2v = c.symbol(b"c2D\0");
        let r_d: unsafe extern "C" fn(*mut C2Simplex) -> C2v = r.symbol(b"c2D\0");
        let c_support: unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int =
            c.symbol(b"c2Support\0");
        let r_support: unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int =
            r.symbol(b"c2Support\0");
        let c_witness: unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v) =
            c.symbol(b"c2Witness\0");
        let r_witness: unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v) =
            r.symbol(b"c2Witness\0");
        let c_l: unsafe extern "C" fn(*mut C2Simplex) -> C2v = c.symbol(b"c2L\0");
        let r_l: unsafe extern "C" fn(*mut C2Simplex) -> C2v = r.symbol(b"c2L\0");

        let mut rng = Rng::new(0xdda4_2f80_c9b2_1771);
        let mut seen22 = [false; 3];
        let mut seen23 = [false; 7];
        for _ in 0..40_000 {
            let base = C2Simplex {
                a: random_sv(&mut rng),
                b: random_sv(&mut rng),
                c: random_sv(&mut rng),
                d: random_sv(&mut rng),
                div: rng.positive(20.0),
                count: 2,
            };
            seen22[c22_region(base.a.p, base.b.p)] = true;
            let mut cs = base;
            let mut rs = base;
            c22(&mut cs);
            r22(&mut rs);
            assert_bytes(&cs, &rs, "c22");

            let mut triangle = base;
            triangle.count = 3;
            seen23[c23_region(triangle.a.p, triangle.b.p, triangle.c.p)] = true;
            let mut cs = triangle;
            let mut rs = triangle;
            c23(&mut cs);
            r23(&mut rs);
            assert_bytes(&cs, &rs, "c23");
        }
        assert!(
            seen22.iter().all(|seen| *seen),
            "not all c22 regions reached"
        );
        assert!(
            seen23.iter().all(|seen| *seen),
            "not all c23 regions reached: {seen23:?}"
        );

        for count in [1, 2, 3] {
            for _ in 0..512 {
                let simplex = C2Simplex {
                    a: random_sv(&mut rng),
                    b: random_sv(&mut rng),
                    c: random_sv(&mut rng),
                    d: random_sv(&mut rng),
                    div: rng.positive(20.0),
                    count,
                };
                let mut cs = simplex;
                let mut rs = simplex;
                assert_f32(c_metric(&mut cs), r_metric(&mut rs), "simplex metric");
                assert_bytes(&c_d(&mut cs), &r_d(&mut rs), "simplex direction");
                assert_bytes(&c_l(&mut cs), &r_l(&mut rs), "simplex closest point");
                let mut ca = C2v { x: 999.0, y: 998.0 };
                let mut cb = C2v { x: 997.0, y: 996.0 };
                let mut ra = ca;
                let mut rb = cb;
                c_witness(&mut cs, &mut ca, &mut cb);
                r_witness(&mut rs, &mut ra, &mut rb);
                assert_bytes(&ca, &ra, "witness A");
                assert_bytes(&cb, &rb, "witness B");
            }
        }

        for count in [1usize, 2, 8] {
            for _ in 0..512 {
                let mut verts = [ZERO_V; 9];
                for vertex in &mut verts[..count] {
                    *vertex = rng.vector(100.0);
                }
                let direction = rng.vector(10.0);
                assert_eq!(
                    c_support(verts.as_ptr(), count as c_int, direction),
                    r_support(verts.as_ptr(), count as c_int, direction),
                    "support count {count}"
                );
            }
        }
        let tied = [
            C2v { x: 4.0, y: 0.0 },
            C2v { x: 4.0, y: 7.0 },
            C2v { x: 3.0, y: 100.0 },
        ];
        assert_eq!(
            c_support(tied.as_ptr(), 3, C2v { x: 1.0, y: 0.0 }),
            r_support(tied.as_ptr(), 3, C2v { x: 1.0, y: 0.0 })
        );
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(C2Circle),
    Aabb(C2Aabb),
    Capsule(C2Capsule),
}

impl Shape {
    fn kind(&self) -> c_int {
        match self {
            Shape::Circle(_) => 0,
            Shape::Aabb(_) => 1,
            Shape::Capsule(_) => 2,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(value) => (value as *const C2Circle).cast(),
            Shape::Aabb(value) => (value as *const C2Aabb).cast(),
            Shape::Capsule(value) => (value as *const C2Capsule).cast(),
        }
    }
}

fn random_shape(rng: &mut Rng, kind: c_int) -> Shape {
    match kind {
        0 => Shape::Circle(C2Circle {
            p: rng.vector(80.0),
            r: rng.positive(20.0),
        }),
        1 => {
            let center = rng.vector(80.0);
            let half = C2v {
                x: rng.positive(20.0),
                y: rng.positive(20.0),
            };
            Shape::Aabb(C2Aabb {
                min: C2v {
                    x: center.x - half.x,
                    y: center.y - half.y,
                },
                max: C2v {
                    x: center.x + half.x,
                    y: center.y + half.y,
                },
            })
        }
        2 => Shape::Capsule(C2Capsule {
            a: rng.vector(80.0),
            b: rng.vector(80.0),
            r: rng.positive(20.0),
        }),
        _ => unreachable!(),
    }
}

type GjkFn = unsafe extern "C" fn(
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
    *mut C2GjkCache,
) -> f32;

#[derive(Clone, Copy)]
struct GjkResult {
    distance: f32,
    out_a: C2v,
    out_b: C2v,
    iterations: c_int,
    cache: C2GjkCache,
}

unsafe fn invoke_gjk(
    api: &Api,
    a: &Shape,
    ax: Option<&C2x>,
    b: &Shape,
    bx: Option<&C2x>,
    use_radius: c_int,
    output_mask: u8,
    cache_input: Option<C2GjkCache>,
) -> GjkResult {
    let function: GjkFn = unsafe { api.symbol(b"c2GJK\0") };
    let mut out_a = C2v {
        x: f32::from_bits(0x7fc0_1111),
        y: f32::from_bits(0x7fc0_2222),
    };
    let mut out_b = C2v {
        x: f32::from_bits(0x7fc0_3333),
        y: f32::from_bits(0x7fc0_4444),
    };
    let mut iterations = -1234567;
    let mut cache = cache_input.unwrap_or(C2GjkCache {
        metric: f32::from_bits(0x7fc0_5555),
        count: -7654321,
        i_a: [-11, -12, -13],
        i_b: [-21, -22, -23],
        div: f32::from_bits(0x7fc0_6666),
    });
    let distance = unsafe {
        function(
            a.ptr(),
            a.kind(),
            ax.map_or(std::ptr::null(), |value| value),
            b.ptr(),
            b.kind(),
            bx.map_or(std::ptr::null(), |value| value),
            if output_mask & 1 != 0 {
                &mut out_a
            } else {
                std::ptr::null_mut()
            },
            if output_mask & 2 != 0 {
                &mut out_b
            } else {
                std::ptr::null_mut()
            },
            use_radius,
            if output_mask & 4 != 0 {
                &mut iterations
            } else {
                std::ptr::null_mut()
            },
            if cache_input.is_some() {
                &mut cache
            } else {
                std::ptr::null_mut()
            },
        )
    };
    GjkResult {
        distance,
        out_a,
        out_b,
        iterations,
        cache,
    }
}

fn assert_gjk(left: &GjkResult, right: &GjkResult, cache_present: bool, context: &str) {
    assert_f32(left.distance, right.distance, context);
    assert_bytes(&left.out_a, &right.out_a, context);
    assert_bytes(&left.out_b, &right.out_b, context);
    assert_eq!(left.iterations, right.iterations, "{context}");
    if cache_present {
        assert_bytes(&left.cache, &right.cache, context);
    }
}

#[test]
fn gjk_all_shape_options_cache_and_output_modes_match() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(0x52cd_93b1_f800_4a77);

        for kind_a in 0..3 {
            for kind_b in 0..3 {
                for use_radius in [0, 1] {
                    for sample in 0..96 {
                        let a = random_shape(&mut rng, kind_a);
                        let b = random_shape(&mut rng, kind_b);
                        let cr = invoke_gjk(&c, &a, None, &b, None, use_radius, 7, None);
                        let rr = invoke_gjk(&r, &a, None, &b, None, use_radius, 7, None);
                        assert_gjk(
                            &cr,
                            &rr,
                            false,
                            &format!(
                                "shape pair {kind_a}/{kind_b} radius {use_radius} sample {sample}"
                            ),
                        );
                    }
                }
            }
        }

        for sample in 0..512 {
            let kind_a = (rng.u32() % 3) as c_int;
            let kind_b = (rng.u32() % 3) as c_int;
            let a = random_shape(&mut rng, kind_a);
            let b = random_shape(&mut rng, kind_b);
            let angle_a = rng.f32(3.0);
            let angle_b = rng.f32(3.0);
            let ax = C2x {
                p: rng.vector(100.0),
                r: C2r {
                    c: angle_a.cos(),
                    s: angle_a.sin(),
                },
            };
            let bx = C2x {
                p: rng.vector(100.0),
                r: C2r {
                    c: angle_b.cos(),
                    s: angle_b.sin(),
                },
            };
            for (a_transform, b_transform) in
                [(Some(&ax), Some(&bx)), (Some(&ax), None), (None, Some(&bx))]
            {
                let cr = invoke_gjk(&c, &a, a_transform, &b, b_transform, 1, 7, None);
                let rr = invoke_gjk(&r, &a, a_transform, &b, b_transform, 1, 7, None);
                assert_gjk(&cr, &rr, false, &format!("transform sample {sample}"));
            }
        }

        let cold = C2GjkCache {
            metric: 1234.5,
            count: 0,
            i_a: [71, 72, 73],
            i_b: [81, 82, 83],
            div: -998.0,
        };
        for sample in 0..512 {
            let kind_a = (rng.u32() % 3) as c_int;
            let kind_b = (rng.u32() % 3) as c_int;
            let a = random_shape(&mut rng, kind_a);
            let b = random_shape(&mut rng, kind_b);
            let c_first = invoke_gjk(&c, &a, None, &b, None, 1, 7, Some(cold));
            let r_first = invoke_gjk(&r, &a, None, &b, None, 1, 7, Some(cold));
            assert_gjk(&c_first, &r_first, true, "cold cache");
            let c_second = invoke_gjk(&c, &a, None, &b, None, 1, 7, Some(c_first.cache));
            let r_second = invoke_gjk(&r, &a, None, &b, None, 1, 7, Some(r_first.cache));
            assert_gjk(&c_second, &r_second, true, &format!("warm cache {sample}"));
        }

        let big_box = Shape::Aabb(C2Aabb {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v {
                x: 20_000.0,
                y: 20_000.0,
            },
        });
        let point = Shape::Circle(C2Circle {
            p: C2v {
                x: 50_000.0,
                y: 50_000.0,
            },
            r: 1.0,
        });
        let rejected_cache = C2GjkCache {
            metric: -1.0e9,
            count: 3,
            i_a: [0, 2, 1],
            i_b: [0, 0, 0],
            div: 1.0,
        };
        let cr = invoke_gjk(&c, &big_box, None, &point, None, 1, 7, Some(rejected_cache));
        let rr = invoke_gjk(&r, &big_box, None, &point, None, 1, 7, Some(rejected_cache));
        assert_gjk(&cr, &rr, true, "rejected cache");

        let separated_a = Shape::Circle(C2Circle {
            p: C2v { x: -100.0, y: 0.0 },
            r: 5.0,
        });
        let separated_b = Shape::Capsule(C2Capsule {
            a: C2v { x: 100.0, y: -5.0 },
            b: C2v { x: 100.0, y: 5.0 },
            r: 7.0,
        });
        let overlapping_a = Shape::Circle(C2Circle { p: ZERO_V, r: 10.0 });
        let overlapping_b = Shape::Circle(C2Circle {
            p: C2v { x: 1.0, y: 1.0 },
            r: 10.0,
        });
        for (a, b, label) in [
            (separated_a, separated_b, "separated"),
            (overlapping_a, overlapping_b, "overlapping"),
        ] {
            for use_radius in [0, 1, 2, -1] {
                for mask in [0, 1, 2, 4, 7] {
                    let cr = invoke_gjk(&c, &a, None, &b, None, use_radius, mask, None);
                    let rr = invoke_gjk(&r, &a, None, &b, None, use_radius, mask, None);
                    assert_gjk(
                        &cr,
                        &rr,
                        false,
                        &format!("{label} radius {use_radius} mask {mask}"),
                    );
                }
            }
        }
    }
}

#[test]
fn boundary_and_default_results_match() {
    unsafe {
        let (c, r) = apis();
        let c_proxy: unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy) =
            c.symbol(b"c2MakeProxy\0");
        let r_proxy: unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy) =
            r.symbol(b"c2MakeProxy\0");
        let c_support: unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int =
            c.symbol(b"c2Support\0");
        let r_support: unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int =
            r.symbol(b"c2Support\0");
        let c_metric: unsafe extern "C" fn(*mut C2Simplex) -> f32 =
            c.symbol(b"c2GJKSimplexMetric\0");
        let r_metric: unsafe extern "C" fn(*mut C2Simplex) -> f32 =
            r.symbol(b"c2GJKSimplexMetric\0");
        let c_d: unsafe extern "C" fn(*mut C2Simplex) -> C2v = c.symbol(b"c2D\0");
        let r_d: unsafe extern "C" fn(*mut C2Simplex) -> C2v = r.symbol(b"c2D\0");
        let c_l: unsafe extern "C" fn(*mut C2Simplex) -> C2v = c.symbol(b"c2L\0");
        let r_l: unsafe extern "C" fn(*mut C2Simplex) -> C2v = r.symbol(b"c2L\0");
        let c_witness: unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v) =
            c.symbol(b"c2Witness\0");
        let r_witness: unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v) =
            r.symbol(b"c2Witness\0");
        let c_div: VecScalarFn = c.symbol(b"c2Div\0");
        let r_div: VecScalarFn = r.symbol(b"c2Div\0");
        let c_norm: unsafe extern "C" fn(C2v) -> C2v = c.symbol(b"c2Norm\0");
        let r_norm: unsafe extern "C" fn(C2v) -> C2v = r.symbol(b"c2Norm\0");
        let c_wrapper: unsafe extern "C" fn(
            c_char,
            *mut C2v,
            *mut C2v,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
        ) = c.symbol(b"gjk_cache\0");
        let r_wrapper: unsafe extern "C" fn(
            c_char,
            *mut C2v,
            *mut C2v,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
        ) = r.symbol(b"gjk_cache\0");

        let shape = C2Circle {
            p: C2v { x: 1.0, y: 2.0 },
            r: 3.0,
        };
        let sentinel = C2Proxy {
            radius: f32::from_bits(0x7fc0_abcd),
            count: -44,
            verts: [C2v { x: 71.0, y: 72.0 }; 8],
        };
        for invalid in [-100, -1, 3, 4, c_int::MAX] {
            let mut cp = sentinel;
            let mut rp = sentinel;
            c_proxy((&shape as *const C2Circle).cast(), invalid, &mut cp);
            r_proxy((&shape as *const C2Circle).cast(), invalid, &mut rp);
            assert_bytes(&cp, &rp, "invalid proxy enum");
            assert_bytes(&cp, &sentinel, "C invalid proxy enum must be no-op");
        }

        let mut verts = [ZERO_V; 9];
        for (index, vertex) in verts.iter_mut().enumerate() {
            *vertex = C2v {
                x: index as f32,
                y: -(index as f32),
            };
        }
        for count in [0, -1, c_int::MIN, 9] {
            assert_eq!(
                c_support(verts.as_ptr(), count, C2v { x: 1.0, y: 0.0 }),
                r_support(verts.as_ptr(), count, C2v { x: 1.0, y: 0.0 }),
                "boundary support count {count}"
            );
        }

        for count in [0, -1, 4, 99] {
            let mut cs = zero_simplex();
            cs.count = count;
            cs.div = 0.0;
            let mut rs = cs;
            assert_f32(c_metric(&mut cs), r_metric(&mut rs), "default metric");
            assert_bytes(&c_d(&mut cs), &r_d(&mut rs), "default direction");
            assert_bytes(&c_l(&mut cs), &r_l(&mut rs), "default closest point");
            let mut ca = C2v { x: 1.0, y: 2.0 };
            let mut cb = C2v { x: 3.0, y: 4.0 };
            let mut ra = ca;
            let mut rb = cb;
            c_witness(&mut cs, &mut ca, &mut cb);
            r_witness(&mut rs, &mut ra, &mut rb);
            assert_bytes(&ca, &ra, "default witness A");
            assert_bytes(&cb, &rb, "default witness B");
        }

        for zero in [0.0, -0.0] {
            for value in [C2v { x: 1.0, y: -2.0 }, C2v { x: 0.0, y: -0.0 }] {
                assert_bytes(&c_div(value, zero), &r_div(value, zero), "zero divisor");
            }
        }
        assert_bytes(
            &c_norm(C2v { x: 0.0, y: 0.0 }),
            &r_norm(C2v { x: 0.0, y: 0.0 }),
            "zero norm",
        );

        let mut rng = Rng::new(0x1a0e_33c4_77b9_e025);
        for _ in 0..1024 {
            let args = [
                rng.f32(100.0),
                rng.f32(100.0),
                rng.f32(100.0),
                rng.f32(100.0),
                rng.f32(100.0),
                rng.f32(100.0),
                rng.f32(100.0),
                rng.f32(100.0),
                rng.f32(30.0),
            ];
            for reverse in [0, 1, -1, 2, 127] {
                let mut ca = C2v { x: 11.0, y: 12.0 };
                let mut cb = C2v { x: 13.0, y: 14.0 };
                let mut ra = ca;
                let mut rb = cb;
                c_wrapper(
                    reverse, &mut ca, &mut cb, args[0], args[1], args[2], args[3], args[4],
                    args[5], args[6], args[7], args[8],
                );
                r_wrapper(
                    reverse, &mut ra, &mut rb, args[0], args[1], args[2], args[3], args[4],
                    args[5], args[6], args[7], args[8],
                );
                assert_bytes(&ca, &ra, "gjk_cache ignored output A");
                assert_bytes(&cb, &rb, "gjk_cache ignored output B");
                c_wrapper(
                    reverse,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7],
                    args[8],
                );
                r_wrapper(
                    reverse,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7],
                    args[8],
                );
            }
        }
    }
}

#[test]
fn required_null_child() {
    let Ok(which_library) = std::env::var("DIFF_NULL_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_NULL_CASE").unwrap();
    let (c_path, rust_path) = library_paths();
    let path = if which_library == "c" {
        c_path
    } else {
        rust_path
    };
    unsafe {
        let api = Api::open(&path);
        match case.as_str() {
            "proxy_shape" => {
                let function: unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy) =
                    api.symbol(b"c2MakeProxy\0");
                let mut proxy = C2Proxy {
                    radius: 0.0,
                    count: 0,
                    verts: [ZERO_V; 8],
                };
                function(std::ptr::null(), 0, &mut proxy);
            }
            "bb_input" => {
                let function: unsafe extern "C" fn(*mut C2v, *mut C2Aabb) =
                    api.symbol(b"c2BBVerts\0");
                let mut out = [ZERO_V; 4];
                function(out.as_mut_ptr(), std::ptr::null_mut());
            }
            "support_input" => {
                let function: unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int =
                    api.symbol(b"c2Support\0");
                std::hint::black_box(function(std::ptr::null(), 1, ZERO_V));
            }
            "simplex_input" => {
                let function: unsafe extern "C" fn(*mut C2Simplex) -> f32 =
                    api.symbol(b"c2GJKSimplexMetric\0");
                std::hint::black_box(function(std::ptr::null_mut()));
            }
            _ => panic!("unknown null case"),
        }
    }
}

#[test]
fn required_null_pointer_termination_matches() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    for case in ["proxy_shape", "bb_input", "support_input", "simplex_input"] {
        let run = |library: &str| {
            Command::new(&executable)
                .arg("--exact")
                .arg("required_null_child")
                .arg("--nocapture")
                .env("DIFF_NULL_LIBRARY", library)
                .env("DIFF_NULL_CASE", case)
                .status()
                .unwrap()
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(
            !c_status.success(),
            "C unexpectedly survived null case {case}"
        );
        assert!(
            !rust_status.success(),
            "Rust unexpectedly survived null case {case}"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "termination signal differs for null case {case}: C={c_status:?} Rust={rust_status:?}"
        );
    }
}
