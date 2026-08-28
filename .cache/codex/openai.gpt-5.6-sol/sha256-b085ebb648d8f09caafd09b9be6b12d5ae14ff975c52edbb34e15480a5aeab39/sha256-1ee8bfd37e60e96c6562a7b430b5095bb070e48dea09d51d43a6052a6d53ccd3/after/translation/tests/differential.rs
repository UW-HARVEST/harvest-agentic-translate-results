use libloading::Library;
use std::ffi::{c_int, c_void};
use std::mem::{MaybeUninit, size_of};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

const CAPSULE: c_int = 0;
const CIRCLE: c_int = 1;
const AABB: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct V {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct R {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct X {
    p: V,
    r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Circle {
    p: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Capsule {
    a: V,
    b: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct GjkCache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Proxy {
    radius: f32,
    count: c_int,
    verts: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Sv {
    s_a: V,
    s_b: V,
    p: V,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: f32,
    count: c_int,
}

type FV = unsafe extern "C" fn(f32, f32) -> V;
type FVS = unsafe extern "C" fn(V, f32) -> V;
type FVV = unsafe extern "C" fn(V, V) -> V;
type FVVV = unsafe extern "C" fn(V, V, V) -> V;
type FVVF = unsafe extern "C" fn(V, V) -> f32;
type FVF = unsafe extern "C" fn(V) -> f32;
type FVR = unsafe extern "C" fn(V) -> V;
type FRV = unsafe extern "C" fn(R, V) -> V;
type FXV = unsafe extern "C" fn(X, V) -> V;
type FSimplexF = unsafe extern "C" fn(*mut Simplex) -> f32;
type FSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
type FSimplexVoid = unsafe extern "C" fn(*mut Simplex);
type FGjk = unsafe extern "C" fn(
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
    *mut GjkCache,
) -> f32;

struct Api {
    _lib: Library,
    c2_v: FV,
    c2_mulvs: FVS,
    c2_maxv: FVV,
    c2_minv: FVV,
    c2_clampv: FVVV,
    c2_sub: FVV,
    c2_dot: FVVF,
    c2_rot_identity: unsafe extern "C" fn() -> R,
    c2x_identity: unsafe extern "C" fn() -> X,
    c2_bb_verts: unsafe extern "C" fn(*mut V, *mut Aabb),
    c2_make_proxy: unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
    c2_len: FVF,
    c2_det2: FVVF,
    c2_gjk_simplex_metric: FSimplexF,
    c2_mulrv: FRV,
    c2_add: FVV,
    c2_mulxv: FXV,
    c22: FSimplexVoid,
    c23: FSimplexVoid,
    c2_neg: FVR,
    c2_skew: FVR,
    c2_ccw90: FVR,
    c2_d: FSimplexV,
    c2_support: unsafe extern "C" fn(*const V, c_int, V) -> c_int,
    c2_witness: unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
    c2_div: FVS,
    c2_norm: FVR,
    c2_l: FSimplexV,
    c2_mulrv_t: FRV,
    c2_gjk: FGjk,
    c2_aabb_to_aabb: unsafe extern "C" fn(Aabb, Aabb) -> c_int,
    c2_aabb_to_capsule: unsafe extern "C" fn(Aabb, Capsule) -> c_int,
    c2_capsule_to_capsule: unsafe extern "C" fn(Capsule, Capsule) -> c_int,
    c2_circle_to_circle: unsafe extern "C" fn(Circle, Circle) -> c_int,
    c2_circle_to_aabb: unsafe extern "C" fn(Circle, Aabb) -> c_int,
    c2_circle_to_capsule: unsafe extern "C" fn(Circle, Capsule) -> c_int,
    c2_collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    ptr_from_parts: unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void,
    omni_collide: unsafe extern "C" fn(
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
    ) -> c_int,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let value = unsafe { lib.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *value
            }};
        }
        Self {
            c2_v: symbol!("c2V", FV),
            c2_mulvs: symbol!("c2Mulvs", FVS),
            c2_maxv: symbol!("c2Maxv", FVV),
            c2_minv: symbol!("c2Minv", FVV),
            c2_clampv: symbol!("c2Clampv", FVVV),
            c2_sub: symbol!("c2Sub", FVV),
            c2_dot: symbol!("c2Dot", FVVF),
            c2_rot_identity: symbol!("c2RotIdentity", unsafe extern "C" fn() -> R),
            c2x_identity: symbol!("c2xIdentity", unsafe extern "C" fn() -> X),
            c2_bb_verts: symbol!("c2BBVerts", unsafe extern "C" fn(*mut V, *mut Aabb)),
            c2_make_proxy: symbol!(
                "c2MakeProxy",
                unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)
            ),
            c2_len: symbol!("c2Len", FVF),
            c2_det2: symbol!("c2Det2", FVVF),
            c2_gjk_simplex_metric: symbol!("c2GJKSimplexMetric", FSimplexF),
            c2_mulrv: symbol!("c2Mulrv", FRV),
            c2_add: symbol!("c2Add", FVV),
            c2_mulxv: symbol!("c2Mulxv", FXV),
            c22: symbol!("c22", FSimplexVoid),
            c23: symbol!("c23", FSimplexVoid),
            c2_neg: symbol!("c2Neg", FVR),
            c2_skew: symbol!("c2Skew", FVR),
            c2_ccw90: symbol!("c2CCW90", FVR),
            c2_d: symbol!("c2D", FSimplexV),
            c2_support: symbol!(
                "c2Support",
                unsafe extern "C" fn(*const V, c_int, V) -> c_int
            ),
            c2_witness: symbol!(
                "c2Witness",
                unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)
            ),
            c2_div: symbol!("c2Div", FVS),
            c2_norm: symbol!("c2Norm", FVR),
            c2_l: symbol!("c2L", FSimplexV),
            c2_mulrv_t: symbol!("c2MulrvT", FRV),
            c2_gjk: symbol!("c2GJK", FGjk),
            c2_aabb_to_aabb: symbol!("c2AABBtoAABB", unsafe extern "C" fn(Aabb, Aabb) -> c_int),
            c2_aabb_to_capsule: symbol!(
                "c2AABBtoCapsule",
                unsafe extern "C" fn(Aabb, Capsule) -> c_int
            ),
            c2_capsule_to_capsule: symbol!(
                "c2CapsuletoCapsule",
                unsafe extern "C" fn(Capsule, Capsule) -> c_int
            ),
            c2_circle_to_circle: symbol!(
                "c2CircletoCircle",
                unsafe extern "C" fn(Circle, Circle) -> c_int
            ),
            c2_circle_to_aabb: symbol!(
                "c2CircletoAABB",
                unsafe extern "C" fn(Circle, Aabb) -> c_int
            ),
            c2_circle_to_capsule: symbol!(
                "c2CircletoCapsule",
                unsafe extern "C" fn(Circle, Capsule) -> c_int
            ),
            c2_collided: symbol!(
                "c2Collided",
                unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int
            ),
            ptr_from_parts: symbol!(
                "ptr_from_parts",
                unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void
            ),
            omni_collide: symbol!(
                "omni_collide",
                unsafe extern "C" fn(
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
                ) -> c_int
            ),
            _lib: lib,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-ssxMbU.so");
        let rust_path = root.join("target/release/libomni_collide_lib.so");
        assert!(
            c_path.is_file(),
            "missing C shared object: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared object: {}",
            rust_path.display()
        );
        Self {
            c: unsafe { Api::load(&c_path) },
            rust: unsafe { Api::load(&rust_path) },
        }
    }
}

#[derive(Clone)]
struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn f32(&mut self) -> f32 {
        ((self.u32() % 4001) as i32 - 2000) as f32 / 16.0
    }

    fn nonzero(&mut self) -> f32 {
        let value = (self.u32() % 255 + 1) as f32 / 32.0;
        if self.u32() & 1 == 0 { value } else { -value }
    }

    fn v(&mut self) -> V {
        V {
            x: self.f32(),
            y: self.f32(),
        }
    }

    fn radius(&mut self) -> f32 {
        (self.u32() % 200) as f32 / 16.0
    }

    fn circle(&mut self) -> Circle {
        Circle {
            p: self.v(),
            r: self.radius(),
        }
    }

    fn aabb(&mut self) -> Aabb {
        let a = self.v();
        let b = self.v();
        Aabb {
            min: V {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: V {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }

    fn capsule(&mut self) -> Capsule {
        Capsule {
            a: self.v(),
            b: self.v(),
            r: self.radius(),
        }
    }

    fn transform(&mut self) -> X {
        X {
            p: self.v(),
            r: R {
                c: (self.u32() % 33) as f32 / 16.0 - 1.0,
                s: (self.u32() % 33) as f32 / 16.0 - 1.0,
            },
        }
    }
}

#[track_caller]
fn same<T: Copy>(name: &str, c: T, rust: T) {
    let c_bytes =
        unsafe { std::slice::from_raw_parts((&c as *const T).cast::<u8>(), size_of::<T>()) };
    let rust_bytes =
        unsafe { std::slice::from_raw_parts((&rust as *const T).cast::<u8>(), size_of::<T>()) };
    assert_eq!(c_bytes, rust_bytes, "{name}");
}

fn initialized<T: Copy>(byte: u8) -> T {
    let mut value = MaybeUninit::<T>::uninit();
    unsafe {
        value
            .as_mut_ptr()
            .cast::<u8>()
            .write_bytes(byte, size_of::<T>());
        value.assume_init()
    }
}

fn simplex(rng: &mut Rng) -> Simplex {
    let mut result = Simplex::default();
    let vertices = [&mut result.a, &mut result.b, &mut result.c, &mut result.d];
    for vertex in vertices {
        vertex.s_a = rng.v();
        vertex.s_b = rng.v();
        vertex.p = rng.v();
        vertex.u = (rng.u32() % 100 + 1) as f32 / 16.0;
        vertex.i_a = (rng.u32() % 4) as c_int;
        vertex.i_b = (rng.u32() % 4) as c_int;
    }
    result.div = (rng.u32() % 100 + 1) as f32 / 16.0;
    result
}

#[repr(C)]
union Shape {
    circle: Circle,
    aabb: Aabb,
    capsule: Capsule,
}

impl Copy for Shape {}
impl Clone for Shape {
    fn clone(&self) -> Self {
        *self
    }
}

fn random_shape(rng: &mut Rng, shape_type: c_int) -> Shape {
    match shape_type {
        CAPSULE => Shape {
            capsule: rng.capsule(),
        },
        CIRCLE => Shape {
            circle: rng.circle(),
        },
        AABB => Shape { aabb: rng.aabb() },
        _ => unreachable!(),
    }
}

fn shape_ptr(shape: &Shape) -> *const c_void {
    (shape as *const Shape).cast()
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

#[test]
fn scalar_vector_transform_and_proxy_surface() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0x51f1_5e5d);
    let boundaries = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
    ];

    unsafe {
        same(
            "c2RotIdentity",
            (pair.c.c2_rot_identity)(),
            (pair.rust.c2_rot_identity)(),
        );
        same(
            "c2xIdentity",
            (pair.c.c2x_identity)(),
            (pair.rust.c2x_identity)(),
        );
        let comparisons = [
            (V { x: 0.0, y: 0.0 }, V { x: 0.0, y: 0.0 }),
            (V { x: -1.0, y: 1.0 }, V { x: 1.0, y: -1.0 }),
            (V { x: 1.0, y: -1.0 }, V { x: -1.0, y: 1.0 }),
            (V { x: -1.0, y: -1.0 }, V { x: 1.0, y: 1.0 }),
            (V { x: 1.0, y: 1.0 }, V { x: -1.0, y: -1.0 }),
        ];
        for (a, b) in comparisons {
            same(
                "c2Maxv exact comparison",
                (pair.c.c2_maxv)(a, b),
                (pair.rust.c2_maxv)(a, b),
            );
            same(
                "c2Minv exact comparison",
                (pair.c.c2_minv)(a, b),
                (pair.rust.c2_minv)(a, b),
            );
        }
        for x_region in -1..=1 {
            for y_region in -1..=1 {
                let value = V {
                    x: x_region as f32,
                    y: y_region as f32,
                };
                let lo = V { x: -0.5, y: -0.5 };
                let hi = V { x: 0.5, y: 0.5 };
                same(
                    "c2Clampv 9 regions",
                    (pair.c.c2_clampv)(value, lo, hi),
                    (pair.rust.c2_clampv)(value, lo, hi),
                );
            }
        }
        for value in [V::default(), V { x: 3.0, y: 4.0 }] {
            let identity = X {
                p: V::default(),
                r: R { c: 1.0, s: 0.0 },
            };
            same(
                "c2Mulxv identity",
                (pair.c.c2_mulxv)(identity, value),
                (pair.rust.c2_mulxv)(identity, value),
            );
            same(
                "c2Norm zero/nonzero",
                (pair.c.c2_norm)(value),
                (pair.rust.c2_norm)(value),
            );
        }

        for iteration in 0..10_000 {
            let a = if iteration < boundaries.len() {
                V {
                    x: boundaries[iteration],
                    y: boundaries[boundaries.len() - 1 - iteration],
                }
            } else {
                rng.v()
            };
            let b = rng.v();
            let lo = rng.v();
            let hi = rng.v();
            let scalar = if iteration % 127 == 0 {
                0.0
            } else {
                rng.nonzero()
            };
            let r = R {
                c: rng.f32(),
                s: rng.f32(),
            };
            let x = X { p: rng.v(), r };

            same("c2V", (pair.c.c2_v)(a.x, a.y), (pair.rust.c2_v)(a.x, a.y));
            same(
                "c2Mulvs",
                (pair.c.c2_mulvs)(a, scalar),
                (pair.rust.c2_mulvs)(a, scalar),
            );
            same("c2Maxv", (pair.c.c2_maxv)(a, b), (pair.rust.c2_maxv)(a, b));
            same("c2Minv", (pair.c.c2_minv)(a, b), (pair.rust.c2_minv)(a, b));
            same(
                "c2Clampv",
                (pair.c.c2_clampv)(a, lo, hi),
                (pair.rust.c2_clampv)(a, lo, hi),
            );
            same("c2Sub", (pair.c.c2_sub)(a, b), (pair.rust.c2_sub)(a, b));
            same("c2Dot", (pair.c.c2_dot)(a, b), (pair.rust.c2_dot)(a, b));
            same("c2Len", (pair.c.c2_len)(a), (pair.rust.c2_len)(a));
            same("c2Det2", (pair.c.c2_det2)(a, b), (pair.rust.c2_det2)(a, b));
            same(
                "c2Mulrv",
                (pair.c.c2_mulrv)(r, a),
                (pair.rust.c2_mulrv)(r, a),
            );
            same("c2Add", (pair.c.c2_add)(a, b), (pair.rust.c2_add)(a, b));
            same(
                "c2Mulxv",
                (pair.c.c2_mulxv)(x, a),
                (pair.rust.c2_mulxv)(x, a),
            );
            same("c2Neg", (pair.c.c2_neg)(a), (pair.rust.c2_neg)(a));
            same("c2Skew", (pair.c.c2_skew)(a), (pair.rust.c2_skew)(a));
            same("c2CCW90", (pair.c.c2_ccw90)(a), (pair.rust.c2_ccw90)(a));
            same(
                "c2Div",
                (pair.c.c2_div)(a, scalar),
                (pair.rust.c2_div)(a, scalar),
            );
            same("c2Norm", (pair.c.c2_norm)(a), (pair.rust.c2_norm)(a));
            same(
                "c2MulrvT",
                (pair.c.c2_mulrv_t)(r, a),
                (pair.rust.c2_mulrv_t)(r, a),
            );

            let mut box_c = if iteration % 3 == 0 {
                Aabb { min: b, max: a }
            } else {
                rng.aabb()
            };
            let mut box_r = box_c;
            let mut verts_c = [V::default(); 4];
            let mut verts_r = [V::default(); 4];
            (pair.c.c2_bb_verts)(verts_c.as_mut_ptr(), &mut box_c);
            (pair.rust.c2_bb_verts)(verts_r.as_mut_ptr(), &mut box_r);
            same("c2BBVerts", verts_c, verts_r);

            let circle = rng.circle();
            let capsule = rng.capsule();
            for shape_type in CAPSULE..=AABB {
                let shape = match shape_type {
                    CAPSULE => (&capsule as *const Capsule).cast(),
                    CIRCLE => (&circle as *const Circle).cast(),
                    AABB => (&box_c as *const Aabb).cast(),
                    _ => unreachable!(),
                };
                let mut proxy_c: Proxy = initialized(0xa5);
                let mut proxy_r = proxy_c;
                (pair.c.c2_make_proxy)(shape, shape_type, &mut proxy_c);
                (pair.rust.c2_make_proxy)(shape, shape_type, &mut proxy_r);
                same("c2MakeProxy", proxy_c, proxy_r);
            }
        }
    }
}

#[test]
fn simplex_surface_and_branches() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0xc001_cafe);
    let mut c22_regions = [false; 3];
    let mut c23_regions = [false; 7];

    unsafe {
        let tied = [V { x: 2.0, y: 0.0 }, V { x: 2.0, y: 0.0 }];
        same(
            "c2Support tied maximum",
            (pair.c.c2_support)(tied.as_ptr(), tied.len() as c_int, V { x: 1.0, y: 0.0 }),
            (pair.rust.c2_support)(tied.as_ptr(), tied.len() as c_int, V { x: 1.0, y: 0.0 }),
        );
        for _ in 0..100_000 {
            let base = simplex(&mut rng);
            for count in 1..=3 {
                let mut c = base;
                let mut r = base;
                c.count = count;
                r.count = count;
                same(
                    "c2GJKSimplexMetric",
                    (pair.c.c2_gjk_simplex_metric)(&mut c),
                    (pair.rust.c2_gjk_simplex_metric)(&mut r),
                );
                same("c2D", (pair.c.c2_d)(&mut c), (pair.rust.c2_d)(&mut r));
                same("c2L", (pair.c.c2_l)(&mut c), (pair.rust.c2_l)(&mut r));
                let mut ca = initialized(0x5a);
                let mut cb = initialized(0x5a);
                let mut ra = initialized(0x5a);
                let mut rb = initialized(0x5a);
                (pair.c.c2_witness)(&mut c, &mut ca, &mut cb);
                (pair.rust.c2_witness)(&mut r, &mut ra, &mut rb);
                same("c2Witness-a", ca, ra);
                same("c2Witness-b", cb, rb);
            }

            let mut c = base;
            let mut r = base;
            c.count = 2;
            r.count = 2;
            let a = base.a.p;
            let b = base.b.p;
            let u = b.x * (b.x - a.x) + b.y * (b.y - a.y);
            let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
            let c22_region = if v <= 0.0 {
                0
            } else if u <= 0.0 {
                1
            } else {
                2
            };
            c22_regions[c22_region] = true;
            (pair.c.c22)(&mut c);
            (pair.rust.c22)(&mut r);
            same("c22", c, r);

            let c_point = base.c.p;
            let dot = |left: V, right: V| left.x * right.x + left.y * right.y;
            let sub = |left: V, right: V| V {
                x: left.x - right.x,
                y: left.y - right.y,
            };
            let det = |left: V, right: V| left.x * right.y - left.y * right.x;
            let u_ab = dot(b, sub(b, a));
            let v_ab = dot(a, sub(a, b));
            let u_bc = dot(c_point, sub(c_point, b));
            let v_bc = dot(b, sub(b, c_point));
            let u_ca = dot(a, sub(a, c_point));
            let v_ca = dot(c_point, sub(c_point, a));
            let area = det(sub(b, a), sub(c_point, a));
            let u_abc = det(b, c_point) * area;
            let v_abc = det(c_point, a) * area;
            let w_abc = det(a, b) * area;
            let region = if v_ab <= 0.0 && u_ca <= 0.0 {
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
            };
            c23_regions[region] = true;

            let mut c = base;
            let mut r = base;
            c.count = 3;
            r.count = 3;
            (pair.c.c23)(&mut c);
            (pair.rust.c23)(&mut r);
            same("c23", c, r);

            let count = (rng.u32() % 8 + 1) as usize;
            let mut vertices = [V::default(); 8];
            for vertex in &mut vertices[..count] {
                *vertex = rng.v();
            }
            let direction = rng.v();
            same(
                "c2Support",
                (pair.c.c2_support)(vertices.as_ptr(), count as c_int, direction),
                (pair.rust.c2_support)(vertices.as_ptr(), count as c_int, direction),
            );
        }
    }

    assert!(
        c22_regions.into_iter().all(|covered| covered),
        "not all c22 regions covered"
    );
    assert!(
        c23_regions.into_iter().all(|covered| covered),
        "not all c23 regions covered"
    );
}

#[test]
fn collision_boundary_fixtures() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0xb0ad_4e55);

    unsafe {
        for _ in 0..512 {
            let offset = rng.v();
            let width = (rng.u32() % 32 + 1) as f32;
            let height = (rng.u32() % 32 + 1) as f32;
            let box_a = Aabb {
                min: offset,
                max: V {
                    x: offset.x + width,
                    y: offset.y + height,
                },
            };
            let boxes = [
                box_a,
                Aabb {
                    min: V {
                        x: box_a.max.x,
                        y: box_a.min.y,
                    },
                    max: V {
                        x: box_a.max.x + width,
                        y: box_a.max.y,
                    },
                },
                Aabb {
                    min: V {
                        x: box_a.max.x + 1.0,
                        y: box_a.min.y,
                    },
                    max: V {
                        x: box_a.max.x + width + 1.0,
                        y: box_a.max.y,
                    },
                },
                Aabb {
                    min: V {
                        x: box_a.min.x - width - 1.0,
                        y: box_a.min.y,
                    },
                    max: V {
                        x: box_a.min.x - 1.0,
                        y: box_a.max.y,
                    },
                },
                Aabb {
                    min: V {
                        x: box_a.min.x,
                        y: box_a.max.y + 1.0,
                    },
                    max: V {
                        x: box_a.max.x,
                        y: box_a.max.y + height + 1.0,
                    },
                },
                Aabb {
                    min: V {
                        x: box_a.min.x,
                        y: box_a.min.y - height - 1.0,
                    },
                    max: V {
                        x: box_a.max.x,
                        y: box_a.min.y - 1.0,
                    },
                },
            ];
            for box_b in boxes {
                same(
                    "c2AABBtoAABB boundary",
                    (pair.c.c2_aabb_to_aabb)(box_a, box_b),
                    (pair.rust.c2_aabb_to_aabb)(box_a, box_b),
                );
            }

            let radius_a = (rng.u32() % 16 + 1) as f32;
            let radius_b = (rng.u32() % 16 + 1) as f32;
            let total_radius = radius_a + radius_b;
            for delta in [-1.0, 0.0, 1.0] {
                let circle_a = Circle {
                    p: offset,
                    r: radius_a,
                };
                let circle_b = Circle {
                    p: V {
                        x: offset.x + total_radius + delta,
                        y: offset.y,
                    },
                    r: radius_b,
                };
                same(
                    "c2CircletoCircle boundary",
                    (pair.c.c2_circle_to_circle)(circle_a, circle_b),
                    (pair.rust.c2_circle_to_circle)(circle_a, circle_b),
                );
            }

            let unit_box = Aabb {
                min: offset,
                max: V {
                    x: offset.x + 8.0,
                    y: offset.y + 8.0,
                },
            };
            for x_region in -1..=1 {
                for y_region in -1..=1 {
                    let center = V {
                        x: match x_region {
                            -1 => offset.x - 2.0,
                            0 => offset.x + 4.0,
                            1 => offset.x + 10.0,
                            _ => unreachable!(),
                        },
                        y: match y_region {
                            -1 => offset.y - 2.0,
                            0 => offset.y + 4.0,
                            1 => offset.y + 10.0,
                            _ => unreachable!(),
                        },
                    };
                    for radius in [1.0, 2.0, 4.0] {
                        let circle = Circle {
                            p: center,
                            r: radius,
                        };
                        same(
                            "c2CircletoAABB region/boundary",
                            (pair.c.c2_circle_to_aabb)(circle, unit_box),
                            (pair.rust.c2_circle_to_aabb)(circle, unit_box),
                        );
                    }
                }
            }

            let capsule = Capsule {
                a: offset,
                b: V {
                    x: offset.x + 10.0,
                    y: offset.y,
                },
                r: 1.0,
            };
            for (region_x, direction) in [(-1.0, -1.0), (5.0, 0.0), (11.0, 1.0)] {
                for delta in [-0.5, 0.0, 0.5] {
                    let circle = Circle {
                        p: if direction == 0.0 {
                            V {
                                x: offset.x + region_x,
                                y: offset.y + 2.0 + delta,
                            }
                        } else {
                            V {
                                x: offset.x + region_x + delta * direction,
                                y: offset.y,
                            }
                        },
                        r: 1.0,
                    };
                    same(
                        "c2CircletoCapsule region/boundary",
                        (pair.c.c2_circle_to_capsule)(circle, capsule),
                        (pair.rust.c2_circle_to_capsule)(circle, capsule),
                    );
                }
            }

            for delta in [-0.5, 0.0, 0.5] {
                let box_capsule = Capsule {
                    a: V {
                        x: offset.x + 2.0,
                        y: unit_box.max.y + 1.0 + delta,
                    },
                    b: V {
                        x: offset.x + 6.0,
                        y: unit_box.max.y + 1.0 + delta,
                    },
                    r: 1.0,
                };
                same(
                    "c2AABBtoCapsule boundary",
                    (pair.c.c2_aabb_to_capsule)(unit_box, box_capsule),
                    (pair.rust.c2_aabb_to_capsule)(unit_box, box_capsule),
                );
                let other_capsule = Capsule {
                    a: V {
                        x: capsule.a.x,
                        y: capsule.a.y + 2.0 + delta,
                    },
                    b: V {
                        x: capsule.b.x,
                        y: capsule.b.y + 2.0 + delta,
                    },
                    r: 1.0,
                };
                same(
                    "c2CapsuletoCapsule boundary",
                    (pair.c.c2_capsule_to_capsule)(capsule, other_capsule),
                    (pair.rust.c2_capsule_to_capsule)(capsule, other_capsule),
                );
            }
            let point_capsule = Capsule {
                a: offset,
                b: offset,
                r: 1.0,
            };
            same(
                "c2AABBtoCapsule degenerate",
                (pair.c.c2_aabb_to_capsule)(unit_box, point_capsule),
                (pair.rust.c2_aabb_to_capsule)(unit_box, point_capsule),
            );
            same(
                "c2CapsuletoCapsule degenerate",
                (pair.c.c2_capsule_to_capsule)(capsule, point_capsule),
                (pair.rust.c2_capsule_to_capsule)(capsule, point_capsule),
            );
        }
    }
}

#[test]
fn collision_dispatch_construction_and_packed_api() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0x1234_abcd);

    unsafe {
        for _ in 0..20_000 {
            let circle_a = rng.circle();
            let circle_b = rng.circle();
            let aabb_a = rng.aabb();
            let aabb_b = rng.aabb();
            let capsule_a = rng.capsule();
            let capsule_b = rng.capsule();

            same(
                "c2AABBtoAABB",
                (pair.c.c2_aabb_to_aabb)(aabb_a, aabb_b),
                (pair.rust.c2_aabb_to_aabb)(aabb_a, aabb_b),
            );
            same(
                "c2AABBtoCapsule",
                (pair.c.c2_aabb_to_capsule)(aabb_a, capsule_b),
                (pair.rust.c2_aabb_to_capsule)(aabb_a, capsule_b),
            );
            same(
                "c2CapsuletoCapsule",
                (pair.c.c2_capsule_to_capsule)(capsule_a, capsule_b),
                (pair.rust.c2_capsule_to_capsule)(capsule_a, capsule_b),
            );
            same(
                "c2CircletoCircle",
                (pair.c.c2_circle_to_circle)(circle_a, circle_b),
                (pair.rust.c2_circle_to_circle)(circle_a, circle_b),
            );
            same(
                "c2CircletoAABB",
                (pair.c.c2_circle_to_aabb)(circle_a, aabb_b),
                (pair.rust.c2_circle_to_aabb)(circle_a, aabb_b),
            );
            same(
                "c2CircletoCapsule",
                (pair.c.c2_circle_to_capsule)(circle_a, capsule_b),
                (pair.rust.c2_circle_to_capsule)(circle_a, capsule_b),
            );

            let shapes_a = [
                Shape { capsule: capsule_a },
                Shape { circle: circle_a },
                Shape { aabb: aabb_a },
            ];
            let shapes_b = [
                Shape { capsule: capsule_b },
                Shape { circle: circle_b },
                Shape { aabb: aabb_b },
            ];
            for type_a in CAPSULE..=AABB {
                for type_b in CAPSULE..=AABB {
                    let a = &shapes_a[type_a as usize];
                    let b = &shapes_b[type_b as usize];
                    same(
                        "c2Collided",
                        (pair.c.c2_collided)(shape_ptr(a), type_a, shape_ptr(b), type_b),
                        (pair.rust.c2_collided)(shape_ptr(a), type_a, shape_ptr(b), type_b),
                    );
                    let values_a = [rng.f32(), rng.f32(), rng.f32(), rng.f32(), rng.f32()];
                    let values_b = [rng.f32(), rng.f32(), rng.f32(), rng.f32(), rng.f32()];
                    same(
                        "omni_collide",
                        (pair.c.omni_collide)(
                            type_a,
                            values_a[0],
                            values_a[1],
                            values_a[2],
                            values_a[3],
                            values_a[4],
                            type_b,
                            values_b[0],
                            values_b[1],
                            values_b[2],
                            values_b[3],
                            values_b[4],
                        ),
                        (pair.rust.omni_collide)(
                            type_a,
                            values_a[0],
                            values_a[1],
                            values_a[2],
                            values_a[3],
                            values_a[4],
                            type_b,
                            values_b[0],
                            values_b[1],
                            values_b[2],
                            values_b[3],
                            values_b[4],
                        ),
                    );
                }
            }

            for shape_type in CAPSULE..=AABB {
                let values = [rng.f32(), rng.f32(), rng.f32(), rng.f32(), rng.f32()];
                let c_pointer = (pair.c.ptr_from_parts)(
                    shape_type, values[0], values[1], values[2], values[3], values[4],
                );
                let rust_pointer = (pair.rust.ptr_from_parts)(
                    shape_type, values[0], values[1], values[2], values[3], values[4],
                );
                assert!(!c_pointer.is_null() && !rust_pointer.is_null());
                let bytes = match shape_type {
                    CAPSULE => size_of::<Capsule>(),
                    CIRCLE => size_of::<Circle>(),
                    AABB => size_of::<Aabb>(),
                    _ => unreachable!(),
                };
                let c_value = std::slice::from_raw_parts(c_pointer.cast::<u8>(), bytes);
                let rust_value = std::slice::from_raw_parts(rust_pointer.cast::<u8>(), bytes);
                assert_eq!(c_value, rust_value, "ptr_from_parts");
                free(c_pointer);
                free(rust_pointer);
            }
        }
    }
}

unsafe fn compare_gjk_call(
    pair: &Pair,
    shape_a: &Shape,
    type_a: c_int,
    transform_a: Option<&X>,
    shape_b: &Shape,
    type_b: c_int,
    transform_b: Option<&X>,
    use_radius: c_int,
    output_mask: u8,
    cache_mode: u8,
) {
    let ax = transform_a.map_or(null(), |value| value);
    let bx = transform_b.map_or(null(), |value| value);
    let mut cache_c = GjkCache::default();
    let mut cache_r = GjkCache::default();
    if cache_mode == 2 {
        unsafe {
            (pair.c.c2_gjk)(
                shape_ptr(shape_a),
                type_a,
                ax,
                shape_ptr(shape_b),
                type_b,
                bx,
                null_mut(),
                null_mut(),
                use_radius,
                null_mut(),
                &mut cache_c,
            );
            (pair.rust.c2_gjk)(
                shape_ptr(shape_a),
                type_a,
                ax,
                shape_ptr(shape_b),
                type_b,
                bx,
                null_mut(),
                null_mut(),
                use_radius,
                null_mut(),
                &mut cache_r,
            );
        }
        same("c2GJK-warm-cache", cache_c, cache_r);
    }

    let mut out_a_c: V = initialized(0xa5);
    let mut out_a_r = out_a_c;
    let mut out_b_c: V = initialized(0x5a);
    let mut out_b_r = out_b_c;
    let mut iterations_c: c_int = -1;
    let mut iterations_r: c_int = -1;
    let out_a_c_ptr = if output_mask & 1 != 0 {
        &mut out_a_c
    } else {
        null_mut()
    };
    let out_a_r_ptr = if output_mask & 1 != 0 {
        &mut out_a_r
    } else {
        null_mut()
    };
    let out_b_c_ptr = if output_mask & 2 != 0 {
        &mut out_b_c
    } else {
        null_mut()
    };
    let out_b_r_ptr = if output_mask & 2 != 0 {
        &mut out_b_r
    } else {
        null_mut()
    };
    let iter_c_ptr = if output_mask & 4 != 0 {
        &mut iterations_c
    } else {
        null_mut()
    };
    let iter_r_ptr = if output_mask & 4 != 0 {
        &mut iterations_r
    } else {
        null_mut()
    };
    let cache_c_ptr = if cache_mode == 0 {
        null_mut()
    } else {
        &mut cache_c
    };
    let cache_r_ptr = if cache_mode == 0 {
        null_mut()
    } else {
        &mut cache_r
    };

    let distance_c = unsafe {
        (pair.c.c2_gjk)(
            shape_ptr(shape_a),
            type_a,
            ax,
            shape_ptr(shape_b),
            type_b,
            bx,
            out_a_c_ptr,
            out_b_c_ptr,
            use_radius,
            iter_c_ptr,
            cache_c_ptr,
        )
    };
    let distance_r = unsafe {
        (pair.rust.c2_gjk)(
            shape_ptr(shape_a),
            type_a,
            ax,
            shape_ptr(shape_b),
            type_b,
            bx,
            out_a_r_ptr,
            out_b_r_ptr,
            use_radius,
            iter_r_ptr,
            cache_r_ptr,
        )
    };
    same("c2GJK-distance", distance_c, distance_r);
    if output_mask & 1 != 0 {
        same("c2GJK-outA", out_a_c, out_a_r);
    }
    if output_mask & 2 != 0 {
        same("c2GJK-outB", out_b_c, out_b_r);
    }
    if output_mask & 4 != 0 {
        same("c2GJK-iterations", iterations_c, iterations_r);
    }
    if cache_mode != 0 {
        same("c2GJK-cache", cache_c, cache_r);
    }
}

#[test]
fn gjk_complete_option_cross_product() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0x0ddc_0ffe);

    for type_a in CAPSULE..=AABB {
        for type_b in CAPSULE..=AABB {
            for _ in 0..64 {
                let shape_a = random_shape(&mut rng, type_a);
                let shape_b = random_shape(&mut rng, type_b);
                let transform_a = rng.transform();
                let transform_b = rng.transform();
                for transform_mode in 0..4 {
                    let ax = (transform_mode & 1 != 0).then_some(&transform_a);
                    let bx = (transform_mode & 2 != 0).then_some(&transform_b);
                    for use_radius in 0..=1 {
                        for output_mask in 0..8 {
                            for cache_mode in 0..3 {
                                unsafe {
                                    compare_gjk_call(
                                        &pair,
                                        &shape_a,
                                        type_a,
                                        ax,
                                        &shape_b,
                                        type_b,
                                        bx,
                                        use_radius,
                                        output_mask,
                                        cache_mode,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let shape_a = Shape {
        aabb: Aabb {
            min: V::default(),
            max: V::default(),
        },
    };
    let shape_b = Shape {
        aabb: Aabb {
            min: V::default(),
            max: V {
                x: 20_000.0,
                y: 20_000.0,
            },
        },
    };
    let rejected = GjkCache {
        metric: 1.0,
        count: 3,
        i_a: [0, 0, 0],
        i_b: [0, 2, 1],
        div: 1.0,
    };
    unsafe {
        let mut cache_c = rejected;
        let mut cache_r = rejected;
        let mut out_a_c = V::default();
        let mut out_a_r = V::default();
        let distance_c = (pair.c.c2_gjk)(
            shape_ptr(&shape_a),
            AABB,
            null(),
            shape_ptr(&shape_b),
            AABB,
            null(),
            &mut out_a_c,
            null_mut(),
            1,
            null_mut(),
            &mut cache_c,
        );
        let distance_r = (pair.rust.c2_gjk)(
            shape_ptr(&shape_a),
            AABB,
            null(),
            shape_ptr(&shape_b),
            AABB,
            null(),
            &mut out_a_r,
            null_mut(),
            1,
            null_mut(),
            &mut cache_r,
        );
        same("c2GJK-rejected-cache-distance", distance_c, distance_r);
        same("c2GJK-rejected-cache-out", out_a_c, out_a_r);
        same("c2GJK-rejected-cache", cache_c, cache_r);
    }
}

#[test]
fn defined_error_and_boundary_surface() {
    let pair = unsafe { Pair::load() };
    let mut rng = Rng::new(0xfeed_beef);

    unsafe {
        let shape = rng.circle();
        for invalid_type in [-1, 3, c_int::MAX] {
            let mut proxy_c: Proxy = initialized(0xa5);
            let mut proxy_r = proxy_c;
            (pair.c.c2_make_proxy)((&shape as *const Circle).cast(), invalid_type, &mut proxy_c);
            (pair.rust.c2_make_proxy)((&shape as *const Circle).cast(), invalid_type, &mut proxy_r);
            same("invalid c2MakeProxy", proxy_c, proxy_r);

            same(
                "invalid outer c2Collided",
                (pair.c.c2_collided)(null(), invalid_type, null(), invalid_type),
                (pair.rust.c2_collided)(null(), invalid_type, null(), invalid_type),
            );
            for type_a in CAPSULE..=AABB {
                let valid_shape = random_shape(&mut rng, type_a);
                same(
                    "invalid inner c2Collided",
                    (pair.c.c2_collided)(shape_ptr(&valid_shape), type_a, null(), invalid_type),
                    (pair.rust.c2_collided)(shape_ptr(&valid_shape), type_a, null(), invalid_type),
                );
                same(
                    "invalid omni_collide B",
                    (pair.c.omni_collide)(
                        type_a,
                        1.0,
                        2.0,
                        3.0,
                        4.0,
                        5.0,
                        invalid_type,
                        6.0,
                        7.0,
                        8.0,
                        9.0,
                        10.0,
                    ),
                    (pair.rust.omni_collide)(
                        type_a,
                        1.0,
                        2.0,
                        3.0,
                        4.0,
                        5.0,
                        invalid_type,
                        6.0,
                        7.0,
                        8.0,
                        9.0,
                        10.0,
                    ),
                );
            }
            same(
                "invalid omni_collide A",
                (pair.c.omni_collide)(
                    invalid_type,
                    1.0,
                    2.0,
                    3.0,
                    4.0,
                    5.0,
                    CIRCLE,
                    6.0,
                    7.0,
                    8.0,
                    9.0,
                    10.0,
                ),
                (pair.rust.omni_collide)(
                    invalid_type,
                    1.0,
                    2.0,
                    3.0,
                    4.0,
                    5.0,
                    CIRCLE,
                    6.0,
                    7.0,
                    8.0,
                    9.0,
                    10.0,
                ),
            );
        }

        for invalid_count in [-1, 0, 4, c_int::MAX] {
            let mut c = simplex(&mut rng);
            let mut r = c;
            c.count = invalid_count;
            r.count = invalid_count;
            same(
                "invalid c2GJKSimplexMetric",
                (pair.c.c2_gjk_simplex_metric)(&mut c),
                (pair.rust.c2_gjk_simplex_metric)(&mut r),
            );
            same(
                "invalid c2D",
                (pair.c.c2_d)(&mut c),
                (pair.rust.c2_d)(&mut r),
            );
            same(
                "invalid c2L",
                (pair.c.c2_l)(&mut c),
                (pair.rust.c2_l)(&mut r),
            );
            let mut ca: V = initialized(0xa5);
            let mut cb: V = initialized(0xa5);
            let mut ra = ca;
            let mut rb = cb;
            (pair.c.c2_witness)(&mut c, &mut ca, &mut cb);
            (pair.rust.c2_witness)(&mut r, &mut ra, &mut rb);
            same("invalid c2Witness-a", ca, ra);
            same("invalid c2Witness-b", cb, rb);
        }

        let first = V { x: 2.0, y: -3.0 };
        for count in [c_int::MIN, -1, 0] {
            same(
                "zero/negative c2Support",
                (pair.c.c2_support)(&first, count, V { x: 1.0, y: 1.0 }),
                (pair.rust.c2_support)(&first, count, V { x: 1.0, y: 1.0 }),
            );
        }
        let mut oversized = [V::default(); 257];
        for vertex in &mut oversized {
            *vertex = rng.v();
        }
        same(
            "oversized c2Support",
            (pair.c.c2_support)(oversized.as_ptr(), oversized.len() as c_int, first),
            (pair.rust.c2_support)(oversized.as_ptr(), oversized.len() as c_int, first),
        );
    }
}
