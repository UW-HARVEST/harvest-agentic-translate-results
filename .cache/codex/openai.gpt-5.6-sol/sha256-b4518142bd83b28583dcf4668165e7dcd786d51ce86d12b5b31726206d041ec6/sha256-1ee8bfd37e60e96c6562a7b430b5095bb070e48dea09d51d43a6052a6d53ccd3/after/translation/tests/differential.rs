use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::ptr::{null, null_mut};

const CAPSULE: c_int = 0;
const CIRCLE: c_int = 1;
const AABB: c_int = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct V {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct R {
    c: c_float,
    s: c_float,
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
    r: c_float,
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
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
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

impl Default for Proxy {
    fn default() -> Self {
        unsafe { zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Sv {
    s_a: V,
    s_b: V,
    p: V,
    u: c_float,
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
    div: c_float,
    count: c_int,
}

type GjkFn = unsafe extern "C" fn(
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

type OmniFn = unsafe extern "C" fn(
    c_int,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
    c_int,
    c_float,
    c_float,
    c_float,
    c_float,
    c_float,
) -> c_int;

#[allow(dead_code)]
struct Api {
    _lib: Library,
    c2_v: unsafe extern "C" fn(c_float, c_float) -> V,
    c2_mulvs: unsafe extern "C" fn(V, c_float) -> V,
    c2_maxv: unsafe extern "C" fn(V, V) -> V,
    c2_minv: unsafe extern "C" fn(V, V) -> V,
    c2_clampv: unsafe extern "C" fn(V, V, V) -> V,
    c2_sub: unsafe extern "C" fn(V, V) -> V,
    c2_dot: unsafe extern "C" fn(V, V) -> c_float,
    c2_rot_identity: unsafe extern "C" fn() -> R,
    c2x_identity: unsafe extern "C" fn() -> X,
    c2_bb_verts: unsafe extern "C" fn(*mut V, *mut Aabb),
    c2_make_proxy: unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
    c2_len: unsafe extern "C" fn(V) -> c_float,
    c2_det2: unsafe extern "C" fn(V, V) -> c_float,
    c2_gjk_simplex_metric: unsafe extern "C" fn(*mut Simplex) -> c_float,
    c2_mulrv: unsafe extern "C" fn(R, V) -> V,
    c2_add: unsafe extern "C" fn(V, V) -> V,
    c2_mulxv: unsafe extern "C" fn(X, V) -> V,
    c22: unsafe extern "C" fn(*mut Simplex),
    c23: unsafe extern "C" fn(*mut Simplex),
    c2_neg: unsafe extern "C" fn(V) -> V,
    c2_skew: unsafe extern "C" fn(V) -> V,
    c2_ccw90: unsafe extern "C" fn(V) -> V,
    c2_d: unsafe extern "C" fn(*mut Simplex) -> V,
    c2_support: unsafe extern "C" fn(*const V, c_int, V) -> c_int,
    c2_witness: unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
    c2_div: unsafe extern "C" fn(V, c_float) -> V,
    c2_norm: unsafe extern "C" fn(V) -> V,
    c2_l: unsafe extern "C" fn(*mut Simplex) -> V,
    c2_mulrv_t: unsafe extern "C" fn(R, V) -> V,
    c2_gjk: GjkFn,
    c2_aabb_to_aabb: unsafe extern "C" fn(Aabb, Aabb) -> c_int,
    c2_aabb_to_capsule: unsafe extern "C" fn(Aabb, Capsule) -> c_int,
    c2_capsule_to_capsule: unsafe extern "C" fn(Capsule, Capsule) -> c_int,
    c2_circle_to_circle: unsafe extern "C" fn(Circle, Circle) -> c_int,
    c2_circle_to_aabb: unsafe extern "C" fn(Circle, Aabb) -> c_int,
    c2_circle_to_capsule: unsafe extern "C" fn(Circle, Capsule) -> c_int,
    c2_collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    ptr_from_parts:
        unsafe extern "C" fn(c_int, c_float, c_float, c_float, c_float, c_float) -> *mut c_void,
    omni_collide: OmniFn,
}

unsafe fn load_symbol<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe { *lib.get::<T>(name).unwrap() }
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let lib = unsafe { Library::new(path).unwrap() };
        macro_rules! s {
            ($name:literal, $ty:ty) => {
                unsafe { load_symbol::<$ty>(&lib, concat!($name, "\0").as_bytes()) }
            };
        }
        Self {
            c2_v: s!("c2V", unsafe extern "C" fn(c_float, c_float) -> V),
            c2_mulvs: s!("c2Mulvs", unsafe extern "C" fn(V, c_float) -> V),
            c2_maxv: s!("c2Maxv", unsafe extern "C" fn(V, V) -> V),
            c2_minv: s!("c2Minv", unsafe extern "C" fn(V, V) -> V),
            c2_clampv: s!("c2Clampv", unsafe extern "C" fn(V, V, V) -> V),
            c2_sub: s!("c2Sub", unsafe extern "C" fn(V, V) -> V),
            c2_dot: s!("c2Dot", unsafe extern "C" fn(V, V) -> c_float),
            c2_rot_identity: s!("c2RotIdentity", unsafe extern "C" fn() -> R),
            c2x_identity: s!("c2xIdentity", unsafe extern "C" fn() -> X),
            c2_bb_verts: s!("c2BBVerts", unsafe extern "C" fn(*mut V, *mut Aabb)),
            c2_make_proxy: s!(
                "c2MakeProxy",
                unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)
            ),
            c2_len: s!("c2Len", unsafe extern "C" fn(V) -> c_float),
            c2_det2: s!("c2Det2", unsafe extern "C" fn(V, V) -> c_float),
            c2_gjk_simplex_metric: s!(
                "c2GJKSimplexMetric",
                unsafe extern "C" fn(*mut Simplex) -> c_float
            ),
            c2_mulrv: s!("c2Mulrv", unsafe extern "C" fn(R, V) -> V),
            c2_add: s!("c2Add", unsafe extern "C" fn(V, V) -> V),
            c2_mulxv: s!("c2Mulxv", unsafe extern "C" fn(X, V) -> V),
            c22: s!("c22", unsafe extern "C" fn(*mut Simplex)),
            c23: s!("c23", unsafe extern "C" fn(*mut Simplex)),
            c2_neg: s!("c2Neg", unsafe extern "C" fn(V) -> V),
            c2_skew: s!("c2Skew", unsafe extern "C" fn(V) -> V),
            c2_ccw90: s!("c2CCW90", unsafe extern "C" fn(V) -> V),
            c2_d: s!("c2D", unsafe extern "C" fn(*mut Simplex) -> V),
            c2_support: s!(
                "c2Support",
                unsafe extern "C" fn(*const V, c_int, V) -> c_int
            ),
            c2_witness: s!(
                "c2Witness",
                unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)
            ),
            c2_div: s!("c2Div", unsafe extern "C" fn(V, c_float) -> V),
            c2_norm: s!("c2Norm", unsafe extern "C" fn(V) -> V),
            c2_l: s!("c2L", unsafe extern "C" fn(*mut Simplex) -> V),
            c2_mulrv_t: s!("c2MulrvT", unsafe extern "C" fn(R, V) -> V),
            c2_gjk: s!("c2GJK", GjkFn),
            c2_aabb_to_aabb: s!("c2AABBtoAABB", unsafe extern "C" fn(Aabb, Aabb) -> c_int),
            c2_aabb_to_capsule: s!(
                "c2AABBtoCapsule",
                unsafe extern "C" fn(Aabb, Capsule) -> c_int
            ),
            c2_capsule_to_capsule: s!(
                "c2CapsuletoCapsule",
                unsafe extern "C" fn(Capsule, Capsule) -> c_int
            ),
            c2_circle_to_circle: s!(
                "c2CircletoCircle",
                unsafe extern "C" fn(Circle, Circle) -> c_int
            ),
            c2_circle_to_aabb: s!(
                "c2CircletoAABB",
                unsafe extern "C" fn(Circle, Aabb) -> c_int
            ),
            c2_circle_to_capsule: s!(
                "c2CircletoCapsule",
                unsafe extern "C" fn(Circle, Capsule) -> c_int
            ),
            c2_collided: s!(
                "c2Collided",
                unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int
            ),
            ptr_from_parts: s!(
                "ptr_from_parts",
                unsafe extern "C" fn(
                    c_int,
                    c_float,
                    c_float,
                    c_float,
                    c_float,
                    c_float,
                ) -> *mut c_void
            ),
            omni_collide: s!("omni_collide", OmniFn),
            _lib: lib,
        }
    }
}

struct Pair {
    c: Api,
    r: Api,
}

impl Pair {
    fn load() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_dir
            .parent()
            .unwrap()
            .join("c_src/build/libharvest-work-18164a.so");
        let r_path = crate_dir.join("target/release/libomni_collide_lib.so");
        assert!(
            c_path.exists(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            r_path.exists(),
            "missing Rust shared library: {}",
            r_path.display()
        );
        unsafe {
            Self {
                c: Api::load(c_path),
                r: Api::load(r_path),
            }
        }
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>()) }
}

fn same<T>(label: &str, c: &T, r: &T) {
    assert_eq!(
        bytes(c),
        bytes(r),
        "{label}: C={:?} Rust={:?}",
        bytes(c),
        bytes(r)
    );
}

fn same_call<T>(label: &str, c: T, r: T) -> T {
    same(label, &c, &r);
    c
}

struct Rng(u32);

impl Rng {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn f(&mut self) -> f32 {
        ((self.u32() % 4001) as i32 - 2000) as f32 / 16.0
    }

    fn positive(&mut self) -> f32 {
        (self.u32() % 255 + 1) as f32 / 32.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.f(),
            y: self.f(),
        }
    }

    fn radius(&mut self) -> f32 {
        (self.u32() % 200) as f32 / 16.0
    }
}

fn shape_ptr<'a>(
    typ: c_int,
    circle: &'a Circle,
    aabb: &'a Aabb,
    capsule: &'a Capsule,
) -> *const c_void {
    match typ {
        CIRCLE => (circle as *const Circle).cast(),
        AABB => (aabb as *const Aabb).cast(),
        CAPSULE => (capsule as *const Capsule).cast(),
        _ => unreachable!(),
    }
}

fn shape_size(typ: c_int) -> usize {
    match typ {
        CIRCLE => size_of::<Circle>(),
        AABB => size_of::<Aabb>(),
        CAPSULE => size_of::<Capsule>(),
        _ => unreachable!(),
    }
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

fn filled_proxy(byte: u8) -> Proxy {
    let mut p = std::mem::MaybeUninit::<Proxy>::uninit();
    unsafe {
        std::ptr::write_bytes(p.as_mut_ptr().cast::<u8>(), byte, size_of::<Proxy>());
        p.assume_init()
    }
}

#[test]
fn differential_low_level_vectors_proxies_and_allocators() {
    let p = Pair::load();
    let mut rng = Rng::new(0x51f1_5e5d);

    unsafe {
        same_call(
            "c2RotIdentity",
            (p.c.c2_rot_identity)(),
            (p.r.c2_rot_identity)(),
        );
        same_call("c2xIdentity", (p.c.c2x_identity)(), (p.r.c2x_identity)());
    }

    for n in 0..10_000 {
        let a = rng.v();
        let b = rng.v();
        let x = rng.f();
        let y = rng.f();
        let scalar = rng.positive();
        let lo = V {
            x: rng.f().min(rng.f()),
            y: rng.f().min(rng.f()),
        };
        let hi = V {
            x: lo.x + rng.positive(),
            y: lo.y + rng.positive(),
        };
        let below_inside_above = match n % 3 {
            0 => V {
                x: lo.x - scalar,
                y: lo.y - scalar,
            },
            1 => V {
                x: (lo.x + hi.x) * 0.5,
                y: (lo.y + hi.y) * 0.5,
            },
            _ => V {
                x: hi.x + scalar,
                y: hi.y + scalar,
            },
        };
        let rot = R {
            c: rng.f() / 128.0,
            s: rng.f() / 128.0,
        };
        let transform = X { p: rng.v(), r: rot };

        unsafe {
            same_call("c2V", (p.c.c2_v)(x, y), (p.r.c2_v)(x, y));
            same_call(
                "c2Mulvs",
                (p.c.c2_mulvs)(a, scalar),
                (p.r.c2_mulvs)(a, scalar),
            );
            same_call("c2Maxv", (p.c.c2_maxv)(a, b), (p.r.c2_maxv)(a, b));
            same_call("c2Maxv-equal", (p.c.c2_maxv)(a, a), (p.r.c2_maxv)(a, a));
            same_call("c2Minv", (p.c.c2_minv)(a, b), (p.r.c2_minv)(a, b));
            same_call("c2Minv-equal", (p.c.c2_minv)(a, a), (p.r.c2_minv)(a, a));
            same_call(
                "c2Clampv",
                (p.c.c2_clampv)(below_inside_above, lo, hi),
                (p.r.c2_clampv)(below_inside_above, lo, hi),
            );
            same_call("c2Sub", (p.c.c2_sub)(a, b), (p.r.c2_sub)(a, b));
            same_call("c2Add", (p.c.c2_add)(a, b), (p.r.c2_add)(a, b));
            same_call("c2Dot", (p.c.c2_dot)(a, b), (p.r.c2_dot)(a, b));
            same_call("c2Det2", (p.c.c2_det2)(a, b), (p.r.c2_det2)(a, b));
            same_call(
                "c2Len-zero",
                (p.c.c2_len)(V::default()),
                (p.r.c2_len)(V::default()),
            );
            same_call("c2Len", (p.c.c2_len)(a), (p.r.c2_len)(a));
            same_call("c2Mulrv", (p.c.c2_mulrv)(rot, a), (p.r.c2_mulrv)(rot, a));
            same_call(
                "c2MulrvT",
                (p.c.c2_mulrv_t)(rot, a),
                (p.r.c2_mulrv_t)(rot, a),
            );
            same_call(
                "c2Mulxv",
                (p.c.c2_mulxv)(transform, a),
                (p.r.c2_mulxv)(transform, a),
            );
            same_call("c2Neg", (p.c.c2_neg)(a), (p.r.c2_neg)(a));
            same_call("c2Skew", (p.c.c2_skew)(a), (p.r.c2_skew)(a));
            same_call("c2CCW90", (p.c.c2_ccw90)(a), (p.r.c2_ccw90)(a));
            same_call("c2Div", (p.c.c2_div)(a, scalar), (p.r.c2_div)(a, scalar));
            if a.x != 0.0 || a.y != 0.0 {
                same_call("c2Norm", (p.c.c2_norm)(a), (p.r.c2_norm)(a));
            }
        }

        let mut box_ = Aabb {
            min: rng.v(),
            max: rng.v(),
        };
        let mut c_verts = [V::default(); 4];
        let mut r_verts = [V::default(); 4];
        unsafe {
            (p.c.c2_bb_verts)(c_verts.as_mut_ptr(), &mut box_);
            (p.r.c2_bb_verts)(r_verts.as_mut_ptr(), &mut box_);
        }
        same("c2BBVerts", &c_verts, &r_verts);

        let circle = Circle {
            p: rng.v(),
            r: rng.radius(),
        };
        let capsule = Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.radius(),
        };
        for typ in [CAPSULE, CIRCLE, AABB] {
            let shape = shape_ptr(typ, &circle, &box_, &capsule);
            let mut cp = filled_proxy(0xa5);
            let mut rp = filled_proxy(0xa5);
            unsafe {
                (p.c.c2_make_proxy)(shape, typ, &mut cp);
                (p.r.c2_make_proxy)(shape, typ, &mut rp);
            }
            same("c2MakeProxy", &cp, &rp);
        }

        for typ in [CAPSULE, CIRCLE, AABB] {
            let cp = unsafe { (p.c.ptr_from_parts)(typ, x, y, a.x, a.y, scalar) };
            let rp = unsafe { (p.r.ptr_from_parts)(typ, x, y, a.x, a.y, scalar) };
            assert!(!cp.is_null() && !rp.is_null());
            let len = shape_size(typ);
            let cb = unsafe { std::slice::from_raw_parts(cp.cast::<u8>(), len) };
            let rb = unsafe { std::slice::from_raw_parts(rp.cast::<u8>(), len) };
            assert_eq!(cb, rb, "ptr_from_parts type {typ}");
            unsafe {
                free(cp);
                free(rp);
            }
        }
    }
}

fn dot(a: V, b: V) -> f32 {
    a.x * b.x + a.y * b.y
}

fn sub(a: V, b: V) -> V {
    V {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn det(a: V, b: V) -> f32 {
    a.x * b.y - a.y * b.x
}

fn classify_c22(s: &Simplex) -> usize {
    let a = s.a.p;
    let b = s.b.p;
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

fn classify_c23(s: &Simplex) -> usize {
    let a = s.a.p;
    let b = s.b.p;
    let c = s.c.p;
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

fn random_sv(rng: &mut Rng) -> Sv {
    Sv {
        s_a: rng.v(),
        s_b: rng.v(),
        p: rng.v(),
        u: rng.positive(),
        i_a: (rng.u32() % 4) as c_int,
        i_b: (rng.u32() % 4) as c_int,
    }
}

#[test]
fn differential_simplex_and_support_branches() {
    let p = Pair::load();
    let mut rng = Rng::new(0x7623_33a1);
    let mut c22_hits = [0usize; 3];
    let mut c23_hits = [0usize; 7];

    for n in 0..50_000 {
        let base = Simplex {
            a: random_sv(&mut rng),
            b: random_sv(&mut rng),
            c: random_sv(&mut rng),
            d: random_sv(&mut rng),
            div: rng.positive(),
            count: (n % 4) as c_int,
        };

        for count in [1, 2, 3, 4] {
            let mut cs = base;
            let mut rs = base;
            cs.count = count;
            rs.count = count;
            unsafe {
                same_call(
                    "c2GJKSimplexMetric",
                    (p.c.c2_gjk_simplex_metric)(&mut cs),
                    (p.r.c2_gjk_simplex_metric)(&mut rs),
                );
                same_call("c2D", (p.c.c2_d)(&mut cs), (p.r.c2_d)(&mut rs));
                same_call("c2L", (p.c.c2_l)(&mut cs), (p.r.c2_l)(&mut rs));

                let mut cwa = V::default();
                let mut cwb = V::default();
                let mut rwa = V::default();
                let mut rwb = V::default();
                (p.c.c2_witness)(&mut cs, &mut cwa, &mut cwb);
                (p.r.c2_witness)(&mut rs, &mut rwa, &mut rwb);
                same("c2Witness-a", &cwa, &rwa);
                same("c2Witness-b", &cwb, &rwb);
            }
        }

        let mut cs = base;
        cs.count = 2;
        let mut rs = cs;
        c22_hits[classify_c22(&cs)] += 1;
        unsafe {
            (p.c.c22)(&mut cs);
            (p.r.c22)(&mut rs);
        }
        same("c22", &cs, &rs);

        let mut cs = base;
        cs.count = 3;
        let mut rs = cs;
        c23_hits[classify_c23(&cs)] += 1;
        unsafe {
            (p.c.c23)(&mut cs);
            (p.r.c23)(&mut rs);
        }
        same("c23", &cs, &rs);

        let verts = [
            rng.v(),
            rng.v(),
            rng.v(),
            rng.v(),
            rng.v(),
            rng.v(),
            rng.v(),
            rng.v(),
        ];
        let direction = rng.v();
        unsafe {
            same_call(
                "c2Support-one",
                (p.c.c2_support)(verts.as_ptr(), 1, direction),
                (p.r.c2_support)(verts.as_ptr(), 1, direction),
            );
            same_call(
                "c2Support-many",
                (p.c.c2_support)(verts.as_ptr(), verts.len() as c_int, direction),
                (p.r.c2_support)(verts.as_ptr(), verts.len() as c_int, direction),
            );
        }

        let tied = [V { x: 2.0, y: 1.0 }, V { x: 2.0, y: -3.0 }];
        let ci = unsafe { (p.c.c2_support)(tied.as_ptr(), 2, V { x: 1.0, y: 0.0 }) };
        let ri = unsafe { (p.r.c2_support)(tied.as_ptr(), 2, V { x: 1.0, y: 0.0 }) };
        same_call("c2Support-tie", ci, ri);
        assert_eq!(ci, 0);
    }

    assert!(
        c22_hits.iter().all(|&count| count >= 100),
        "insufficient c22 branch coverage: {c22_hits:?}"
    );
    assert!(
        c23_hits.iter().all(|&count| count >= 100),
        "insufficient c23 branch coverage: {c23_hits:?}"
    );
}

#[derive(Clone, Copy)]
struct GjkResult {
    distance: f32,
    out_a: V,
    out_b: V,
    iterations: c_int,
    cache: Option<Cache>,
}

#[allow(clippy::too_many_arguments)]
fn compare_gjk(
    p: &Pair,
    a: *const c_void,
    type_a: c_int,
    ax: Option<&X>,
    b: *const c_void,
    type_b: c_int,
    bx: Option<&X>,
    use_radius: c_int,
    cache_seed: Option<Cache>,
    outputs: [bool; 3],
) -> GjkResult {
    let mut coa = V {
        x: 12345.0,
        y: -12345.0,
    };
    let mut cob = V {
        x: -23456.0,
        y: 23456.0,
    };
    let mut roa = coa;
    let mut rob = cob;
    let mut ci = -777;
    let mut ri = ci;
    let mut cc = cache_seed.unwrap_or_default();
    let mut rc = cc;
    let cache_present = cache_seed.is_some();

    let cdist = unsafe {
        (p.c.c2_gjk)(
            a,
            type_a,
            ax.map_or(null(), |v| v as *const X),
            b,
            type_b,
            bx.map_or(null(), |v| v as *const X),
            if outputs[0] { &mut coa } else { null_mut() },
            if outputs[1] { &mut cob } else { null_mut() },
            use_radius,
            if outputs[2] { &mut ci } else { null_mut() },
            if cache_present { &mut cc } else { null_mut() },
        )
    };
    let rdist = unsafe {
        (p.r.c2_gjk)(
            a,
            type_a,
            ax.map_or(null(), |v| v as *const X),
            b,
            type_b,
            bx.map_or(null(), |v| v as *const X),
            if outputs[0] { &mut roa } else { null_mut() },
            if outputs[1] { &mut rob } else { null_mut() },
            use_radius,
            if outputs[2] { &mut ri } else { null_mut() },
            if cache_present { &mut rc } else { null_mut() },
        )
    };
    same("c2GJK-distance", &cdist, &rdist);
    if outputs[0] {
        same("c2GJK-outA", &coa, &roa);
    }
    if outputs[1] {
        same("c2GJK-outB", &cob, &rob);
    }
    if outputs[2] {
        same("c2GJK-iterations", &ci, &ri);
    }
    if cache_present {
        same("c2GJK-cache", &cc, &rc);
    }
    GjkResult {
        distance: cdist,
        out_a: coa,
        out_b: cob,
        iterations: ci,
        cache: cache_present.then_some(cc),
    }
}

#[test]
fn differential_gjk_configuration_matrix() {
    let p = Pair::load();
    let mut rng = Rng::new(0x8c77_1ac3);

    for n in 0..2_000 {
        let circle_a = Circle {
            p: rng.v(),
            r: rng.radius(),
        };
        let circle_b = Circle {
            p: rng.v(),
            r: rng.radius(),
        };
        let aabb_a = Aabb {
            min: rng.v(),
            max: rng.v(),
        };
        let aabb_b = Aabb {
            min: rng.v(),
            max: rng.v(),
        };
        let capsule_a = Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.radius(),
        };
        let capsule_b = Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.radius(),
        };
        let ax = X {
            p: rng.v(),
            r: R {
                c: rng.f() / 128.0,
                s: rng.f() / 128.0,
            },
        };
        let bx = X {
            p: rng.v(),
            r: R {
                c: rng.f() / 128.0,
                s: rng.f() / 128.0,
            },
        };

        for type_a in [CAPSULE, CIRCLE, AABB] {
            for type_b in [CAPSULE, CIRCLE, AABB] {
                let sa = shape_ptr(type_a, &circle_a, &aabb_a, &capsule_a);
                let sb = shape_ptr(type_b, &circle_b, &aabb_b, &capsule_b);

                compare_gjk(
                    &p,
                    sa,
                    type_a,
                    None,
                    sb,
                    type_b,
                    None,
                    0,
                    None,
                    [true, true, true],
                );
                let first = compare_gjk(
                    &p,
                    sa,
                    type_a,
                    None,
                    sb,
                    type_b,
                    None,
                    1,
                    Some(Cache::default()),
                    [true, true, true],
                );
                compare_gjk(
                    &p,
                    sa,
                    type_a,
                    None,
                    sb,
                    type_b,
                    None,
                    0,
                    first.cache,
                    [true, true, true],
                );
                compare_gjk(
                    &p,
                    sa,
                    type_a,
                    Some(&ax),
                    sb,
                    type_b,
                    Some(&bx),
                    (n & 1) as c_int,
                    None,
                    [true, true, true],
                );
                if n % 17 == 0 {
                    compare_gjk(
                        &p,
                        sa,
                        type_a,
                        Some(&ax),
                        sb,
                        type_b,
                        None,
                        0,
                        None,
                        [true, true, true],
                    );
                    compare_gjk(
                        &p,
                        sa,
                        type_a,
                        None,
                        sb,
                        type_b,
                        Some(&bx),
                        0,
                        None,
                        [true, true, true],
                    );
                }

                if n % 17 == 0 {
                    for outputs in [
                        [false, true, true],
                        [true, false, true],
                        [true, true, false],
                        [false, false, false],
                    ] {
                        compare_gjk(&p, sa, type_a, None, sb, type_b, None, 1, None, outputs);
                    }
                }
            }
        }
    }

    let large = Aabb {
        min: V { x: 0.0, y: 0.0 },
        max: V {
            x: 100_000.0,
            y: 100_000.0,
        },
    };
    let point = Circle {
        p: V::default(),
        r: 0.0,
    };
    let rejecting_cache = Cache {
        metric: -100_000_000.0,
        count: 3,
        i_a: [0, 2, 1],
        i_b: [0, 0, 0],
        div: 1.0,
    };
    compare_gjk(
        &p,
        (&large as *const Aabb).cast(),
        AABB,
        None,
        (&point as *const Circle).cast(),
        CIRCLE,
        None,
        0,
        Some(rejecting_cache),
        [true, true, true],
    );

    let overlap_a = Circle {
        p: V { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    let overlap_b = Circle {
        p: V { x: 1.0, y: 0.0 },
        r: 2.0,
    };
    let touching_b = Circle {
        p: V { x: 4.0, y: 0.0 },
        r: 2.0,
    };
    let separated_b = Circle {
        p: V { x: 10.0, y: 0.0 },
        r: 2.0,
    };
    for b in [&overlap_b, &touching_b, &separated_b] {
        let result = compare_gjk(
            &p,
            (&overlap_a as *const Circle).cast(),
            CIRCLE,
            None,
            (b as *const Circle).cast(),
            CIRCLE,
            None,
            1,
            Some(Cache::default()),
            [true, true, true],
        );
        let _ = (
            result.distance,
            result.out_a,
            result.out_b,
            result.iterations,
        );
    }

    let same_center = Circle {
        p: V { x: 3.0, y: -7.0 },
        r: 1.0,
    };
    compare_gjk(
        &p,
        (&same_center as *const Circle).cast(),
        CIRCLE,
        None,
        (&same_center as *const Circle).cast(),
        CIRCLE,
        None,
        0,
        Some(Cache::default()),
        [true, true, true],
    );

    let overlapping_box_a = Aabb {
        min: V { x: -3.0, y: -3.0 },
        max: V { x: 3.0, y: 3.0 },
    };
    let overlapping_box_b = Aabb {
        min: V { x: -1.0, y: -1.0 },
        max: V { x: 5.0, y: 5.0 },
    };
    compare_gjk(
        &p,
        (&overlapping_box_a as *const Aabb).cast(),
        AABB,
        None,
        (&overlapping_box_b as *const Aabb).cast(),
        AABB,
        None,
        0,
        Some(Cache::default()),
        [true, true, true],
    );
}

#[test]
fn differential_collision_dispatch_and_boundaries() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1337_4f91);

    for _ in 0..20_000 {
        let circle_a = Circle {
            p: rng.v(),
            r: rng.radius(),
        };
        let circle_b = Circle {
            p: rng.v(),
            r: rng.radius(),
        };
        let aabb_a = Aabb {
            min: rng.v(),
            max: rng.v(),
        };
        let aabb_b = Aabb {
            min: rng.v(),
            max: rng.v(),
        };
        let capsule_a = Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.radius(),
        };
        let capsule_b = Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.radius(),
        };

        unsafe {
            same_call(
                "c2AABBtoAABB",
                (p.c.c2_aabb_to_aabb)(aabb_a, aabb_b),
                (p.r.c2_aabb_to_aabb)(aabb_a, aabb_b),
            );
            same_call(
                "c2AABBtoCapsule",
                (p.c.c2_aabb_to_capsule)(aabb_a, capsule_b),
                (p.r.c2_aabb_to_capsule)(aabb_a, capsule_b),
            );
            same_call(
                "c2CapsuletoCapsule",
                (p.c.c2_capsule_to_capsule)(capsule_a, capsule_b),
                (p.r.c2_capsule_to_capsule)(capsule_a, capsule_b),
            );
            same_call(
                "c2CircletoCircle",
                (p.c.c2_circle_to_circle)(circle_a, circle_b),
                (p.r.c2_circle_to_circle)(circle_a, circle_b),
            );
            same_call(
                "c2CircletoAABB",
                (p.c.c2_circle_to_aabb)(circle_a, aabb_b),
                (p.r.c2_circle_to_aabb)(circle_a, aabb_b),
            );
            same_call(
                "c2CircletoCapsule",
                (p.c.c2_circle_to_capsule)(circle_a, capsule_b),
                (p.r.c2_circle_to_capsule)(circle_a, capsule_b),
            );
        }

        for type_a in [CAPSULE, CIRCLE, AABB] {
            for type_b in [CAPSULE, CIRCLE, AABB] {
                let sa = shape_ptr(type_a, &circle_a, &aabb_a, &capsule_a);
                let sb = shape_ptr(type_b, &circle_b, &aabb_b, &capsule_b);
                unsafe {
                    same_call(
                        "c2Collided",
                        (p.c.c2_collided)(sa, type_a, sb, type_b),
                        (p.r.c2_collided)(sa, type_a, sb, type_b),
                    );
                    same_call(
                        "omni_collide",
                        (p.c.omni_collide)(
                            type_a,
                            circle_a.p.x,
                            circle_a.p.y,
                            capsule_a.b.x,
                            capsule_a.b.y,
                            capsule_a.r,
                            type_b,
                            circle_b.p.x,
                            circle_b.p.y,
                            capsule_b.b.x,
                            capsule_b.b.y,
                            capsule_b.r,
                        ),
                        (p.r.omni_collide)(
                            type_a,
                            circle_a.p.x,
                            circle_a.p.y,
                            capsule_a.b.x,
                            capsule_a.b.y,
                            capsule_a.r,
                            type_b,
                            circle_b.p.x,
                            circle_b.p.y,
                            capsule_b.b.x,
                            capsule_b.b.y,
                            capsule_b.r,
                        ),
                    );
                }
            }
        }
    }

    let box_ = Aabb {
        min: V { x: -1.0, y: -1.0 },
        max: V { x: 1.0, y: 1.0 },
    };
    let capsule = Capsule {
        a: V { x: 0.0, y: 0.0 },
        b: V { x: 10.0, y: 0.0 },
        r: 1.0,
    };
    let branch_circles = [
        Circle {
            p: V { x: -2.0, y: 0.0 },
            r: 1.0,
        },
        Circle {
            p: V { x: 5.0, y: 1.0 },
            r: 1.0,
        },
        Circle {
            p: V { x: 12.0, y: 0.0 },
            r: 1.0,
        },
    ];
    for circle in branch_circles {
        unsafe {
            same_call(
                "c2CircletoCapsule-branch",
                (p.c.c2_circle_to_capsule)(circle, capsule),
                (p.r.c2_circle_to_capsule)(circle, capsule),
            );
            same_call(
                "c2CircletoAABB-boundary",
                (p.c.c2_circle_to_aabb)(circle, box_),
                (p.r.c2_circle_to_aabb)(circle, box_),
            );
        }
    }

    let tangent_a = Circle {
        p: V::default(),
        r: 1.0,
    };
    let tangent_b = Circle {
        p: V { x: 2.0, y: 0.0 },
        r: 1.0,
    };
    unsafe {
        same_call(
            "c2CircletoCircle-tangent",
            (p.c.c2_circle_to_circle)(tangent_a, tangent_b),
            (p.r.c2_circle_to_circle)(tangent_a, tangent_b),
        );
    }
}

#[test]
fn differential_defined_error_surface_and_enum_boundaries() {
    let p = Pair::load();
    let invalid_types = [-1, 3, 4, c_int::MAX, c_int::MIN];

    for type_a in [CAPSULE, CIRCLE, AABB] {
        for invalid_b in invalid_types {
            let c = unsafe { (p.c.c2_collided)(null(), type_a, null(), invalid_b) };
            let r = unsafe { (p.r.c2_collided)(null(), type_a, null(), invalid_b) };
            same_call("c2Collided-invalid-B", c, r);
            assert_eq!(c, 0);
        }
    }
    for invalid_a in invalid_types {
        for type_b in [CAPSULE, CIRCLE, AABB, -1, 3] {
            let c = unsafe { (p.c.c2_collided)(null(), invalid_a, null(), type_b) };
            let r = unsafe { (p.r.c2_collided)(null(), invalid_a, null(), type_b) };
            same_call("c2Collided-invalid-A", c, r);
            assert_eq!(c, 0);
        }
    }

    for invalid in invalid_types {
        let mut cp = filled_proxy(0x5a);
        let mut rp = filled_proxy(0x5a);
        let before = bytes(&cp).to_vec();
        unsafe {
            (p.c.c2_make_proxy)(null(), invalid, &mut cp);
            (p.r.c2_make_proxy)(null(), invalid, &mut rp);
            (p.c.c2_make_proxy)(null(), invalid, null_mut());
            (p.r.c2_make_proxy)(null(), invalid, null_mut());
        }
        same("c2MakeProxy-invalid", &cp, &rp);
        assert_eq!(bytes(&cp), before);

        for valid in [CAPSULE, CIRCLE, AABB] {
            unsafe {
                same_call(
                    "omni_collide-invalid-A",
                    (p.c.omni_collide)(
                        invalid, 1.0, 2.0, 3.0, 4.0, 5.0, valid, 6.0, 7.0, 8.0, 9.0, 10.0,
                    ),
                    (p.r.omni_collide)(
                        invalid, 1.0, 2.0, 3.0, 4.0, 5.0, valid, 6.0, 7.0, 8.0, 9.0, 10.0,
                    ),
                );
                same_call(
                    "omni_collide-invalid-B",
                    (p.c.omni_collide)(
                        valid, 1.0, 2.0, 3.0, 4.0, 5.0, invalid, 6.0, 7.0, 8.0, 9.0, 10.0,
                    ),
                    (p.r.omni_collide)(
                        valid, 1.0, 2.0, 3.0, 4.0, 5.0, invalid, 6.0, 7.0, 8.0, 9.0, 10.0,
                    ),
                );
            }
        }
    }
}
