use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::path::PathBuf;

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

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-nTsGE1.so");
        let rust_path = root.join("target/release/libcapsule_lib.so");
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Library::new(c_path).unwrap(),
                rust: Library::new(rust_path).unwrap(),
            }
        }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = *unsafe { self.c.get::<T>(name) }.unwrap();
        let rust = *unsafe { self.rust.get::<T>(name) }.unwrap();
        (c, rust)
    }
}

#[derive(Clone)]
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
        x as u32
    }

    fn finite(&mut self) -> f32 {
        let n = (self.u32() % 40001) as i32 - 20000;
        n as f32 / 37.0
    }

    fn small(&mut self) -> f32 {
        let n = (self.u32() % 2001) as i32 - 1000;
        n as f32 / 41.0
    }

    fn positive(&mut self) -> f32 {
        (self.u32() % 1000 + 1) as f32 / 29.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.finite(),
            y: self.finite(),
        }
    }
}

fn assert_f32(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_v(c: V, rust: V, context: &str) {
    assert_f32(c.x, rust.x, &format!("{context}.x"));
    assert_f32(c.y, rust.y, &format!("{context}.y"));
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn assert_bytes<T>(c: &T, rust: &T, context: &str) {
    assert_eq!(bytes(c), bytes(rust), "{context}");
}

fn sentinel_proxy() -> Proxy {
    Proxy {
        radius: f32::from_bits(0x42f6_e979),
        count: 0x1234_5678,
        verts: [V {
            x: f32::from_bits(0x4123_4567),
            y: f32::from_bits(0xc234_5678),
        }; 8],
    }
}

fn sv(p: V, id: c_int) -> Sv {
    Sv {
        s_a: V {
            x: p.x + 2.0,
            y: p.y - 3.0,
        },
        s_b: V {
            x: p.x - 5.0,
            y: p.y + 7.0,
        },
        p,
        u: id as f32 / 7.0,
        i_a: id,
        i_b: -id,
    }
}

fn simplex(points: [V; 4], count: c_int) -> Simplex {
    Simplex {
        a: sv(points[0], 11),
        b: sv(points[1], 22),
        c: sv(points[2], 33),
        d: sv(points[3], 44),
        div: 3.25,
        count,
    }
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle(Circle),
    Aabb(Aabb),
    Capsule(Capsule),
}

impl Shape {
    fn type_id(&self) -> c_int {
        match self {
            Self::Circle(_) => CIRCLE,
            Self::Aabb(_) => AABB,
            Self::Capsule(_) => CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Self::Circle(value) => (value as *const Circle).cast(),
            Self::Aabb(value) => (value as *const Aabb).cast(),
            Self::Capsule(value) => (value as *const Capsule).cast(),
        }
    }
}

type Gjk = unsafe extern "C" fn(
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
) -> f32;

unsafe fn compare_gjk(
    libs: &Libs,
    a: &Shape,
    ax: Option<X>,
    b: &Shape,
    bx: Option<X>,
    use_radius: c_int,
    output_mask: u8,
    mut c_cache: Option<Cache>,
    mut r_cache: Option<Cache>,
    context: &str,
) -> (Option<Cache>, Option<Cache>, c_int) {
    let (c_gjk, r_gjk) = unsafe { libs.pair::<Gjk>(b"c2GJK\0") };
    let mut c_out_a = V {
        x: f32::from_bits(0x42f1_2345),
        y: f32::from_bits(0xc2f2_3456),
    };
    let mut r_out_a = c_out_a;
    let mut c_out_b = V {
        x: f32::from_bits(0x42f3_4567),
        y: f32::from_bits(0xc2f4_5678),
    };
    let mut r_out_b = c_out_b;
    let mut c_iterations = 0x1234_5678;
    let mut r_iterations = c_iterations;

    let ax_ptr = ax.as_ref().map_or(std::ptr::null(), |x| x as *const X);
    let bx_ptr = bx.as_ref().map_or(std::ptr::null(), |x| x as *const X);
    let c_out_a_ptr = if output_mask & 1 != 0 {
        &mut c_out_a
    } else {
        std::ptr::null_mut()
    };
    let r_out_a_ptr = if output_mask & 1 != 0 {
        &mut r_out_a
    } else {
        std::ptr::null_mut()
    };
    let c_out_b_ptr = if output_mask & 2 != 0 {
        &mut c_out_b
    } else {
        std::ptr::null_mut()
    };
    let r_out_b_ptr = if output_mask & 2 != 0 {
        &mut r_out_b
    } else {
        std::ptr::null_mut()
    };
    let c_iterations_ptr = if output_mask & 4 != 0 {
        &mut c_iterations
    } else {
        std::ptr::null_mut()
    };
    let r_iterations_ptr = if output_mask & 4 != 0 {
        &mut r_iterations
    } else {
        std::ptr::null_mut()
    };
    let c_cache_ptr = c_cache
        .as_mut()
        .map_or(std::ptr::null_mut(), |cache| cache as *mut Cache);
    let r_cache_ptr = r_cache
        .as_mut()
        .map_or(std::ptr::null_mut(), |cache| cache as *mut Cache);

    let c_dist = unsafe {
        c_gjk(
            a.ptr(),
            a.type_id(),
            ax_ptr,
            b.ptr(),
            b.type_id(),
            bx_ptr,
            c_out_a_ptr,
            c_out_b_ptr,
            use_radius,
            c_iterations_ptr,
            c_cache_ptr,
        )
    };
    let r_dist = unsafe {
        r_gjk(
            a.ptr(),
            a.type_id(),
            ax_ptr,
            b.ptr(),
            b.type_id(),
            bx_ptr,
            r_out_a_ptr,
            r_out_b_ptr,
            use_radius,
            r_iterations_ptr,
            r_cache_ptr,
        )
    };
    assert_f32(c_dist, r_dist, &format!("{context} distance"));
    if output_mask & 1 != 0 {
        assert_v(c_out_a, r_out_a, &format!("{context} outA"));
    }
    if output_mask & 2 != 0 {
        assert_v(c_out_b, r_out_b, &format!("{context} outB"));
    }
    if output_mask & 4 != 0 {
        assert_eq!(c_iterations, r_iterations, "{context} iterations");
    }
    match (&c_cache, &r_cache) {
        (Some(c), Some(rust)) => assert_bytes(c, rust, &format!("{context} cache")),
        (None, None) => {}
        _ => panic!("{context}: cache presence differs"),
    }
    (c_cache, r_cache, c_iterations)
}

fn random_shape(rng: &mut Rng, type_id: c_int) -> Shape {
    match type_id {
        CIRCLE => Shape::Circle(Circle {
            p: rng.v(),
            r: rng.positive(),
        }),
        AABB => {
            let center = rng.v();
            let extent = V {
                x: rng.positive(),
                y: rng.positive(),
            };
            Shape::Aabb(Aabb {
                min: V {
                    x: center.x - extent.x,
                    y: center.y - extent.y,
                },
                max: V {
                    x: center.x + extent.x,
                    y: center.y + extent.y,
                },
            })
        }
        CAPSULE => Shape::Capsule(Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.positive(),
        }),
        _ => unreachable!(),
    }
}

fn shifted(shape: Shape, delta: V) -> Shape {
    match shape {
        Shape::Circle(mut value) => {
            value.p.x += delta.x;
            value.p.y += delta.y;
            Shape::Circle(value)
        }
        Shape::Aabb(mut value) => {
            value.min.x += delta.x;
            value.min.y += delta.y;
            value.max.x += delta.x;
            value.max.y += delta.y;
            Shape::Aabb(value)
        }
        Shape::Capsule(mut value) => {
            value.a.x += delta.x;
            value.a.y += delta.y;
            value.b.x += delta.x;
            value.b.y += delta.y;
            Shape::Capsule(value)
        }
    }
}

#[test]
fn all_c_symbols_are_loadable_from_both_libraries() {
    let libs = Libs::load();
    let names = [
        b"c22\0".as_slice(),
        b"c23\0",
        b"c2AABBtoAABB\0",
        b"c2AABBtoCapsule\0",
        b"c2Add\0",
        b"c2BBVerts\0",
        b"c2CCW90\0",
        b"c2CapsuletoCapsule\0",
        b"c2CircletoAABB\0",
        b"c2CircletoCapsule\0",
        b"c2CircletoCircle\0",
        b"c2Clampv\0",
        b"c2Collided\0",
        b"c2D\0",
        b"c2Det2\0",
        b"c2Div\0",
        b"c2Dot\0",
        b"c2GJK\0",
        b"c2GJKSimplexMetric\0",
        b"c2L\0",
        b"c2Len\0",
        b"c2MakeProxy\0",
        b"c2Maxv\0",
        b"c2Minv\0",
        b"c2Mulrv\0",
        b"c2MulrvT\0",
        b"c2Mulvs\0",
        b"c2Mulxv\0",
        b"c2Neg\0",
        b"c2Norm\0",
        b"c2RotIdentity\0",
        b"c2Skew\0",
        b"c2Sub\0",
        b"c2Support\0",
        b"c2V\0",
        b"c2Witness\0",
        b"c2xIdentity\0",
        b"capsule\0",
    ];
    for name in names {
        unsafe {
            libs.c.get::<*const c_void>(name).unwrap();
            libs.rust.get::<*const c_void>(name).unwrap();
        }
    }
}

#[test]
fn vector_scalar_and_transform_surface_matches() {
    type V2 = unsafe extern "C" fn(V, V) -> V;
    type VF = unsafe extern "C" fn(V, f32) -> V;
    type V1 = unsafe extern "C" fn(V) -> V;
    type F2 = unsafe extern "C" fn(V, V) -> f32;
    type F1 = unsafe extern "C" fn(V) -> f32;
    type Clamp = unsafe extern "C" fn(V, V, V) -> V;
    type MakeV = unsafe extern "C" fn(f32, f32) -> V;
    type RV = unsafe extern "C" fn(R, V) -> V;
    type XV = unsafe extern "C" fn(X, V) -> V;
    type MakeR = unsafe extern "C" fn() -> R;
    type MakeX = unsafe extern "C" fn() -> X;

    let libs = Libs::load();
    unsafe {
        let (c_v, r_v) = libs.pair::<MakeV>(b"c2V\0");
        let (c_mul, r_mul) = libs.pair::<VF>(b"c2Mulvs\0");
        let (c_add, r_add) = libs.pair::<V2>(b"c2Add\0");
        let (c_sub, r_sub) = libs.pair::<V2>(b"c2Sub\0");
        let (c_dot, r_dot) = libs.pair::<F2>(b"c2Dot\0");
        let (c_det, r_det) = libs.pair::<F2>(b"c2Det2\0");
        let (c_max, r_max) = libs.pair::<V2>(b"c2Maxv\0");
        let (c_min, r_min) = libs.pair::<V2>(b"c2Minv\0");
        let (c_clamp, r_clamp) = libs.pair::<Clamp>(b"c2Clampv\0");
        let (c_neg, r_neg) = libs.pair::<V1>(b"c2Neg\0");
        let (c_skew, r_skew) = libs.pair::<V1>(b"c2Skew\0");
        let (c_ccw, r_ccw) = libs.pair::<V1>(b"c2CCW90\0");
        let (c_len, r_len) = libs.pair::<F1>(b"c2Len\0");
        let (c_div, r_div) = libs.pair::<VF>(b"c2Div\0");
        let (c_norm, r_norm) = libs.pair::<V1>(b"c2Norm\0");
        let (c_rot, r_rot) = libs.pair::<MakeR>(b"c2RotIdentity\0");
        let (c_xid, r_xid) = libs.pair::<MakeX>(b"c2xIdentity\0");
        let (c_rv, r_rv) = libs.pair::<RV>(b"c2Mulrv\0");
        let (c_rvt, r_rvt) = libs.pair::<RV>(b"c2MulrvT\0");
        let (c_xv, r_xv) = libs.pair::<XV>(b"c2Mulxv\0");

        assert_bytes(&c_rot(), &r_rot(), "c2RotIdentity");
        assert_bytes(&c_xid(), &r_xid(), "c2xIdentity");

        let mut rng = Rng::new(0x43d2_e191_a053_1c77);
        for i in 0..512 {
            let a = rng.v();
            let b = rng.v();
            let scalar = match i % 8 {
                0 => 0.0,
                1 => -0.0,
                2 => 0.5,
                3 => -2.0,
                _ => rng.small(),
            };
            assert_v(c_v(a.x, a.y), r_v(a.x, a.y), "c2V");
            assert_v(c_mul(a, scalar), r_mul(a, scalar), "c2Mulvs");
            assert_v(c_add(a, b), r_add(a, b), "c2Add");
            assert_v(c_sub(a, b), r_sub(a, b), "c2Sub");
            assert_f32(c_dot(a, b), r_dot(a, b), "c2Dot");
            assert_f32(c_det(a, b), r_det(a, b), "c2Det2");
            assert_v(c_max(a, b), r_max(a, b), "c2Maxv");
            assert_v(c_min(a, b), r_min(a, b), "c2Minv");
            assert_v(c_neg(a), r_neg(a), "c2Neg");
            assert_v(c_skew(a), r_skew(a), "c2Skew");
            assert_v(c_ccw(a), r_ccw(a), "c2CCW90");
            assert_f32(c_len(a), r_len(a), "c2Len");

            let divisor = if i % 17 == 0 {
                if i % 34 == 0 { 0.0 } else { -0.0 }
            } else if scalar == 0.0 {
                1.0
            } else {
                scalar
            };
            assert_v(c_div(a, divisor), r_div(a, divisor), "c2Div");
            if a.x != 0.0 || a.y != 0.0 {
                assert_v(c_norm(a), r_norm(a), "c2Norm");
            }

            let lo = V {
                x: a.x.min(b.x) - 1.0,
                y: a.y.min(b.y) - 1.0,
            };
            let hi = V {
                x: a.x.max(b.x) + 1.0,
                y: a.y.max(b.y) + 1.0,
            };
            let q = match i % 4 {
                0 => V {
                    x: lo.x - rng.positive(),
                    y: lo.y - rng.positive(),
                },
                1 => V {
                    x: hi.x + rng.positive(),
                    y: hi.y + rng.positive(),
                },
                2 => V {
                    x: (lo.x + hi.x) * 0.5,
                    y: (lo.y + hi.y) * 0.5,
                },
                _ => V {
                    x: lo.x - rng.positive(),
                    y: hi.y + rng.positive(),
                },
            };
            assert_v(c_clamp(q, lo, hi), r_clamp(q, lo, hi), "c2Clampv");

            let angle = rng.small();
            let rot = R {
                c: angle.cos(),
                s: angle.sin(),
            };
            let x = X { p: b, r: rot };
            assert_v(c_rv(rot, a), r_rv(rot, a), "c2Mulrv");
            assert_v(c_rvt(rot, a), r_rvt(rot, a), "c2MulrvT");
            assert_v(c_xv(x, a), r_xv(x, a), "c2Mulxv");
        }

        let zero = V { x: 0.0, y: 0.0 };
        assert_f32(c_len(zero), r_len(zero), "c2Len zero");
        assert_v(c_norm(zero), r_norm(zero), "c2Norm zero");

        for _ in 0..2048 {
            let a = V {
                x: f32::from_bits(rng.u32()),
                y: f32::from_bits(rng.u32()),
            };
            let b = V {
                x: f32::from_bits(rng.u32()),
                y: f32::from_bits(rng.u32()),
            };
            let scalar = f32::from_bits(rng.u32());
            assert_v(c_v(a.x, a.y), r_v(a.x, a.y), "c2V raw bits");
            assert_v(c_mul(a, scalar), r_mul(a, scalar), "c2Mulvs raw bits");
            assert_v(c_add(a, b), r_add(a, b), "c2Add raw bits");
            assert_v(c_sub(a, b), r_sub(a, b), "c2Sub raw bits");
            assert_f32(c_dot(a, b), r_dot(a, b), "c2Dot raw bits");
            assert_f32(c_det(a, b), r_det(a, b), "c2Det2 raw bits");
            assert_v(c_max(a, b), r_max(a, b), "c2Maxv raw bits");
            assert_v(c_min(a, b), r_min(a, b), "c2Minv raw bits");
            assert_v(c_neg(a), r_neg(a), "c2Neg raw bits");
            assert_v(c_skew(a), r_skew(a), "c2Skew raw bits");
            assert_v(c_ccw(a), r_ccw(a), "c2CCW90 raw bits");
            assert_f32(c_len(a), r_len(a), "c2Len raw bits");
            assert_v(c_div(a, scalar), r_div(a, scalar), "c2Div raw bits");
            assert_v(c_norm(a), r_norm(a), "c2Norm raw bits");
        }
    }
}

#[test]
fn proxy_and_simplex_surface_matches() {
    type BbVerts = unsafe extern "C" fn(*mut V, *mut Aabb);
    type MakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
    type Metric = unsafe extern "C" fn(*mut Simplex) -> f32;
    type MutSimplex = unsafe extern "C" fn(*mut Simplex);
    type SimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
    type Support = unsafe extern "C" fn(*const V, c_int, V) -> c_int;
    type Witness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);

    let libs = Libs::load();
    unsafe {
        let (c_bb, r_bb) = libs.pair::<BbVerts>(b"c2BBVerts\0");
        let (c_proxy, r_proxy) = libs.pair::<MakeProxy>(b"c2MakeProxy\0");
        let (c_metric, r_metric) = libs.pair::<Metric>(b"c2GJKSimplexMetric\0");
        let (c_22, r_22) = libs.pair::<MutSimplex>(b"c22\0");
        let (c_23, r_23) = libs.pair::<MutSimplex>(b"c23\0");
        let (c_d, r_d) = libs.pair::<SimplexV>(b"c2D\0");
        let (c_support, r_support) = libs.pair::<Support>(b"c2Support\0");
        let (c_witness, r_witness) = libs.pair::<Witness>(b"c2Witness\0");
        let (c_l, r_l) = libs.pair::<SimplexV>(b"c2L\0");

        let mut rng = Rng::new(0xf005_baad_7340_7719);
        for i in 0..256 {
            let a = rng.v();
            let b = rng.v();
            let bb = match i % 4 {
                0 => Aabb {
                    min: V {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                    },
                    max: V {
                        x: a.x.max(b.x),
                        y: a.y.max(b.y),
                    },
                },
                1 => Aabb { min: a, max: a },
                2 => Aabb {
                    min: V { x: a.x, y: b.y },
                    max: V { x: a.x, y: a.y },
                },
                _ => Aabb { min: a, max: b },
            };
            let mut c_out = [V { x: 9.0, y: -9.0 }; 4];
            let mut r_out = c_out;
            let mut c_bb_arg = bb;
            let mut r_bb_arg = bb;
            c_bb(c_out.as_mut_ptr(), &mut c_bb_arg);
            r_bb(r_out.as_mut_ptr(), &mut r_bb_arg);
            assert_bytes(&c_out, &r_out, "c2BBVerts");

            let circle = Circle {
                p: a,
                r: rng.small(),
            };
            let capsule = Capsule {
                a,
                b,
                r: rng.small(),
            };
            for (type_, shape) in [
                (CIRCLE, (&raw const circle).cast::<c_void>()),
                (AABB, (&raw const bb).cast::<c_void>()),
                (CAPSULE, (&raw const capsule).cast::<c_void>()),
            ] {
                let mut cp = sentinel_proxy();
                let mut rp = sentinel_proxy();
                c_proxy(shape, type_, &mut cp);
                r_proxy(shape, type_, &mut rp);
                assert_bytes(&cp, &rp, "c2MakeProxy");
            }

            let points = [a, b, rng.v(), rng.v()];
            for count in [-3, 0, 1, 2, 3, 4, 19] {
                let mut cs = simplex(points, count);
                let mut rs = cs;
                assert_f32(c_metric(&mut cs), r_metric(&mut rs), "c2GJKSimplexMetric");
            }
        }

        let mut branches_22 = [0usize; 3];
        for _ in 0..20_000 {
            let points = [rng.v(), rng.v(), rng.v(), rng.v()];
            let mut cs = simplex(points, 2);
            let mut rs = cs;
            c_22(&mut cs);
            r_22(&mut rs);
            assert_bytes(&cs, &rs, "c22");
            let branch = match (cs.count, cs.a.i_a) {
                (1, 11) => 0,
                (1, 22) => 1,
                (2, 11) => 2,
                other => panic!("unexpected c22 result {other:?}"),
            };
            branches_22[branch] += 1;
            if branches_22.iter().all(|n| *n >= 64) {
                break;
            }
        }
        assert!(
            branches_22.iter().all(|n| *n >= 64),
            "c22 branch counts: {branches_22:?}"
        );

        let mut branches_23 = [0usize; 7];
        for _ in 0..250_000 {
            let points = [
                V {
                    x: ((rng.u32() % 81) as i32 - 40) as f32 / 4.0,
                    y: ((rng.u32() % 81) as i32 - 40) as f32 / 4.0,
                },
                V {
                    x: ((rng.u32() % 81) as i32 - 40) as f32 / 4.0,
                    y: ((rng.u32() % 81) as i32 - 40) as f32 / 4.0,
                },
                V {
                    x: ((rng.u32() % 81) as i32 - 40) as f32 / 4.0,
                    y: ((rng.u32() % 81) as i32 - 40) as f32 / 4.0,
                },
                rng.v(),
            ];
            let mut cs = simplex(points, 3);
            let mut rs = cs;
            c_23(&mut cs);
            r_23(&mut rs);
            assert_bytes(&cs, &rs, "c23");
            let branch = match (cs.count, cs.a.i_a, cs.b.i_a) {
                (1, 11, _) => 0,
                (1, 22, _) => 1,
                (1, 33, _) => 2,
                (2, 11, 22) => 3,
                (2, 22, 33) => 4,
                (2, 33, 11) => 5,
                (3, 11, 22) => 6,
                other => panic!("unexpected c23 result {other:?}"),
            };
            branches_23[branch] += 1;
            if branches_23.iter().all(|n| *n >= 32) {
                break;
            }
        }
        assert!(
            branches_23.iter().all(|n| *n >= 32),
            "c23 branch counts: {branches_23:?}"
        );

        for count in [1, 2, 3, 0, 4, -1] {
            for _ in 0..128 {
                let points = [rng.v(), rng.v(), rng.v(), rng.v()];
                let mut cs = simplex(points, count);
                let mut rs = cs;
                assert_v(c_d(&mut cs), r_d(&mut rs), "c2D");
                assert_v(c_l(&mut cs), r_l(&mut rs), "c2L");

                cs.div = rng.positive();
                rs.div = cs.div;
                cs.a.u = rng.small();
                cs.b.u = rng.small();
                cs.c.u = rng.small();
                rs.a.u = cs.a.u;
                rs.b.u = cs.b.u;
                rs.c.u = cs.c.u;
                let mut ca = V { x: 91.0, y: 92.0 };
                let mut cb = V { x: 93.0, y: 94.0 };
                let mut ra = ca;
                let mut rb = cb;
                c_witness(&mut cs, &mut ca, &mut cb);
                r_witness(&mut rs, &mut ra, &mut rb);
                assert_v(ca, ra, "c2Witness a");
                assert_v(cb, rb, "c2Witness b");
            }
        }

        for count in 1..=8 {
            for _ in 0..128 {
                let mut verts = [V { x: 0.0, y: 0.0 }; 8];
                for v in &mut verts[..count] {
                    *v = rng.v();
                }
                let d = rng.v();
                assert_eq!(
                    c_support(verts.as_ptr(), count as c_int, d),
                    r_support(verts.as_ptr(), count as c_int, d),
                    "c2Support count={count}"
                );
            }
        }
        let tied = [
            V { x: 2.0, y: 1.0 },
            V { x: 2.0, y: -8.0 },
            V { x: 1.0, y: 99.0 },
        ];
        let d = V { x: 1.0, y: 0.0 };
        assert_eq!(c_support(tied.as_ptr(), 3, d), 0);
        assert_eq!(r_support(tied.as_ptr(), 3, d), 0);

        let mut large = vec![V { x: 0.0, y: 0.0 }; 1024];
        for value in &mut large {
            *value = rng.v();
        }
        for count in [-19, -1, 0, 1, 1024] {
            assert_eq!(
                c_support(large.as_ptr(), count, d),
                r_support(large.as_ptr(), count, d),
                "c2Support boundary count={count}"
            );
        }
    }
}

#[test]
fn gjk_configuration_surface_matches() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x7719_83ac_0d42_fe61);
    let output_masks = [0, 1, 2, 4, 7];

    unsafe {
        for type_a in [CIRCLE, AABB, CAPSULE] {
            for type_b in [CIRCLE, AABB, CAPSULE] {
                for i in 0..96 {
                    let a = random_shape(&mut rng, type_a);
                    let b = random_shape(&mut rng, type_b);
                    let (ax, bx) = match i % 4 {
                        0 => (None, None),
                        1 => (
                            Some(X {
                                p: rng.v(),
                                r: R { c: 0.8, s: 0.6 },
                            }),
                            None,
                        ),
                        2 => (
                            None,
                            Some(X {
                                p: rng.v(),
                                r: R { c: -0.6, s: 0.8 },
                            }),
                        ),
                        _ => (
                            Some(X {
                                p: rng.v(),
                                r: R { c: 0.0, s: 1.0 },
                            }),
                            Some(X {
                                p: rng.v(),
                                r: R { c: -1.0, s: 0.0 },
                            }),
                        ),
                    };
                    let context = format!("pair {type_a}->{type_b} sample {i}");
                    compare_gjk(
                        &libs,
                        &a,
                        ax,
                        &b,
                        bx,
                        0,
                        output_masks[i % output_masks.len()],
                        None,
                        None,
                        &context,
                    );
                    compare_gjk(
                        &libs,
                        &a,
                        ax,
                        &b,
                        bx,
                        if i % 2 == 0 { 1 } else { -7 },
                        output_masks[(i + 1) % output_masks.len()],
                        None,
                        None,
                        &format!("{context} radius"),
                    );
                }
            }
        }

        for sample in 0..64 {
            let center = rng.v();
            let radius = rng.positive().min(10.0);
            let overlap_shapes = [
                Shape::Circle(Circle {
                    p: center,
                    r: radius,
                }),
                Shape::Aabb(Aabb {
                    min: V {
                        x: center.x - radius,
                        y: center.y - radius,
                    },
                    max: V {
                        x: center.x + radius,
                        y: center.y + radius,
                    },
                }),
                Shape::Capsule(Capsule {
                    a: V {
                        x: center.x - radius,
                        y: center.y,
                    },
                    b: V {
                        x: center.x + radius,
                        y: center.y,
                    },
                    r: radius,
                }),
            ];
            for a in overlap_shapes {
                for b in overlap_shapes {
                    compare_gjk(
                        &libs,
                        &a,
                        None,
                        &b,
                        None,
                        1,
                        7,
                        None,
                        None,
                        &format!(
                            "explicit overlap {}->{} #{sample}",
                            a.type_id(),
                            b.type_id()
                        ),
                    );
                    let far_b = shifted(
                        b,
                        V {
                            x: 1000.0 + sample as f32,
                            y: -800.0,
                        },
                    );
                    compare_gjk(
                        &libs,
                        &a,
                        None,
                        &far_b,
                        None,
                        1,
                        7,
                        None,
                        None,
                        &format!(
                            "explicit separation {}->{} #{sample}",
                            a.type_id(),
                            b.type_id()
                        ),
                    );
                }
            }
        }

        let zero_cache = Cache {
            metric: f32::from_bits(0x4212_3456),
            count: 0,
            i_a: [101, 102, 103],
            i_b: [201, 202, 203],
            div: f32::from_bits(0x42ab_cdef),
        };
        let mut observed_counts = [0usize; 4];
        for i in 0..20_000 {
            let type_a = (rng.u32() % 3) as c_int;
            let type_b = (rng.u32() % 3) as c_int;
            let a = random_shape(&mut rng, type_a);
            let b = random_shape(&mut rng, type_b);
            let (c_cache, r_cache, _) = compare_gjk(
                &libs,
                &a,
                None,
                &b,
                None,
                i & 1,
                7,
                Some(zero_cache),
                Some(zero_cache),
                "cold cache",
            );
            let count = c_cache.unwrap().count;
            if (1..=3).contains(&count) {
                observed_counts[count as usize] += 1;
                compare_gjk(
                    &libs,
                    &a,
                    None,
                    &b,
                    None,
                    i & 1,
                    7,
                    c_cache,
                    r_cache,
                    &format!("warm cache count {count}"),
                );
            }
            if observed_counts[1..].iter().all(|n| *n >= 32) {
                break;
            }
        }
        assert!(
            observed_counts[1..].iter().all(|n| *n >= 32),
            "GJK cache counts not all reached: {observed_counts:?}"
        );

        let huge_a = Shape::Aabb(Aabb {
            min: V {
                x: -100_000.0,
                y: -100_000.0,
            },
            max: V {
                x: 100_000.0,
                y: 100_000.0,
            },
        });
        let huge_b = Shape::Aabb(Aabb {
            min: V {
                x: -80_000.0,
                y: -70_000.0,
            },
            max: V {
                x: 90_000.0,
                y: 60_000.0,
            },
        });
        let unusual_cache = Cache {
            metric: 1.0,
            count: 3,
            i_a: [0, 2, 1],
            i_b: [0, 1, 2],
            div: 1.0,
        };
        for _ in 0..64 {
            let (c_cache, _, _) = compare_gjk(
                &libs,
                &huge_a,
                None,
                &huge_b,
                None,
                1,
                7,
                Some(unusual_cache),
                Some(unusual_cache),
                "negative-metric cache predicate",
            );
            let c_cache = c_cache.unwrap();
            assert!(
                c_cache.i_a != unusual_cache.i_a || c_cache.i_b != unusual_cache.i_b,
                "negative metric cache was not reinitialized"
            );
        }
    }
}

#[test]
fn collision_and_capsule_surface_matches() {
    type AabbAabb = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
    type AabbCapsule = unsafe extern "C" fn(Aabb, Capsule) -> c_int;
    type CapsuleCapsule = unsafe extern "C" fn(Capsule, Capsule) -> c_int;
    type CircleCircle = unsafe extern "C" fn(Circle, Circle) -> c_int;
    type CircleAabb = unsafe extern "C" fn(Circle, Aabb) -> c_int;
    type CircleCapsule = unsafe extern "C" fn(Circle, Capsule) -> c_int;
    type Collided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
    type CapsuleEntry = unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int;

    let libs = Libs::load();
    unsafe {
        let (c_aa, r_aa) = libs.pair::<AabbAabb>(b"c2AABBtoAABB\0");
        let (c_ac, r_ac) = libs.pair::<AabbCapsule>(b"c2AABBtoCapsule\0");
        let (c_ccap, r_ccap) = libs.pair::<CapsuleCapsule>(b"c2CapsuletoCapsule\0");
        let (c_cc, r_cc) = libs.pair::<CircleCircle>(b"c2CircletoCircle\0");
        let (c_ca, r_ca) = libs.pair::<CircleAabb>(b"c2CircletoAABB\0");
        let (c_ccirclecap, r_ccirclecap) = libs.pair::<CircleCapsule>(b"c2CircletoCapsule\0");
        let (c_collided, r_collided) = libs.pair::<Collided>(b"c2Collided\0");
        let (c_capsule, r_capsule) = libs.pair::<CapsuleEntry>(b"capsule\0");

        let mut rng = Rng::new(0xacce_5510_5eed_9021);
        for i in 0..1024 {
            let circle_a = match i % 4 {
                0 => Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                },
                1 => Circle {
                    p: V { x: 4.0, y: 0.0 },
                    r: 2.0,
                },
                _ => Circle {
                    p: rng.v(),
                    r: rng.positive(),
                },
            };
            let circle_b = match i % 4 {
                0 => Circle {
                    p: V { x: 1.0, y: 0.0 },
                    r: 2.0,
                },
                1 => Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                },
                _ => Circle {
                    p: rng.v(),
                    r: rng.positive(),
                },
            };
            let aabb_a = match i % 6 {
                0 => Aabb {
                    min: V { x: 0.0, y: 0.0 },
                    max: V { x: 2.0, y: 2.0 },
                },
                1 => Aabb {
                    min: V { x: 2.0, y: 0.0 },
                    max: V { x: 4.0, y: 2.0 },
                },
                _ => match random_shape(&mut rng, AABB) {
                    Shape::Aabb(value) => value,
                    _ => unreachable!(),
                },
            };
            let aabb_b = match i % 6 {
                0 => Aabb {
                    min: V { x: 3.0, y: 0.0 },
                    max: V { x: 5.0, y: 2.0 },
                },
                1 => Aabb {
                    min: V { x: 0.0, y: 0.0 },
                    max: V { x: 2.0, y: 2.0 },
                },
                _ => match random_shape(&mut rng, AABB) {
                    Shape::Aabb(value) => value,
                    _ => unreachable!(),
                },
            };
            let capsule_a = if i % 8 == 0 {
                Capsule {
                    a: V { x: 0.0, y: 0.0 },
                    b: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                }
            } else {
                match random_shape(&mut rng, CAPSULE) {
                    Shape::Capsule(value) => value,
                    _ => unreachable!(),
                }
            };
            let capsule_b = match random_shape(&mut rng, CAPSULE) {
                Shape::Capsule(value) => value,
                _ => unreachable!(),
            };

            assert_eq!(c_aa(aabb_a, aabb_b), r_aa(aabb_a, aabb_b), "AABB/AABB");
            assert_eq!(
                c_ac(aabb_a, capsule_b),
                r_ac(aabb_a, capsule_b),
                "AABB/capsule"
            );
            assert_eq!(
                c_ccap(capsule_a, capsule_b),
                r_ccap(capsule_a, capsule_b),
                "capsule/capsule"
            );
            assert_eq!(
                c_cc(circle_a, circle_b),
                r_cc(circle_a, circle_b),
                "circle/circle"
            );
            assert_eq!(
                c_ca(circle_a, aabb_b),
                r_ca(circle_a, aabb_b),
                "circle/AABB"
            );
            assert_eq!(
                c_ccirclecap(circle_a, capsule_b),
                r_ccirclecap(circle_a, capsule_b),
                "circle/capsule"
            );

            let shapes_a = [
                Shape::Circle(circle_a),
                Shape::Aabb(aabb_a),
                Shape::Capsule(capsule_a),
            ];
            let shapes_b = [
                Shape::Circle(circle_b),
                Shape::Aabb(aabb_b),
                Shape::Capsule(capsule_b),
            ];
            for a in &shapes_a {
                for b in &shapes_b {
                    assert_eq!(
                        c_collided(a.ptr(), a.type_id(), b.ptr(), b.type_id()),
                        r_collided(a.ptr(), a.type_id(), b.ptr(), b.type_id()),
                        "c2Collided {}->{}",
                        a.type_id(),
                        b.type_id()
                    );
                }
            }
        }

        let before_a = Capsule {
            a: V { x: 0.0, y: 0.0 },
            b: V { x: 10.0, y: 0.0 },
            r: 1.0,
        };
        for circle in [
            Circle {
                p: V { x: -2.0, y: 0.0 },
                r: 1.5,
            },
            Circle {
                p: V { x: 5.0, y: 1.0 },
                r: 0.5,
            },
            Circle {
                p: V { x: 12.0, y: 0.0 },
                r: 1.5,
            },
            Circle {
                p: V { x: 0.0, y: 0.0 },
                r: 0.0,
            },
        ] {
            assert_eq!(
                c_ccirclecap(circle, before_a),
                r_ccirclecap(circle, before_a),
                "circle/capsule projection region"
            );
        }

        let mut observed = [0usize; 8];
        for _ in 0..200_000 {
            let min_x = (rng.u32() % 241) as f32 - 120.0;
            let min_y = (rng.u32() % 241) as f32 - 120.0;
            let max_x = (rng.u32() % 241) as f32 - 120.0;
            let max_y = (rng.u32() % 241) as f32 - 120.0;
            let radius = (rng.u32() % 41) as f32;
            let c = c_capsule(min_x, min_y, max_x, max_y, radius);
            let rust = r_capsule(min_x, min_y, max_x, max_y, radius);
            assert_eq!(c, rust, "capsule wrapper");
            assert!((0..=7).contains(&c), "capsule result out of range: {c}");
            observed[c as usize] += 1;
            if observed.iter().all(|count| *count >= 32) {
                break;
            }
        }
        assert!(
            observed.iter().all(|count| *count >= 32),
            "not all capsule bit combinations reached: {observed:?}"
        );
    }
}

#[test]
fn explicit_error_surface_matches() {
    type MakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);
    type Collided = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;

    let libs = Libs::load();
    unsafe {
        let (c_proxy, r_proxy) = libs.pair::<MakeProxy>(b"c2MakeProxy\0");
        let (c_collided, r_collided) = libs.pair::<Collided>(b"c2Collided\0");

        for invalid in [-1, 3, 4, 127, c_int::MIN, c_int::MAX] {
            let mut cp = sentinel_proxy();
            let mut rp = sentinel_proxy();
            c_proxy(std::ptr::null(), invalid, &mut cp);
            r_proxy(std::ptr::null(), invalid, &mut rp);
            assert_bytes(&cp, &rp, "invalid c2MakeProxy type");
            assert_bytes(&cp, &sentinel_proxy(), "C invalid proxy must be unchanged");

            assert_eq!(
                c_collided(std::ptr::null(), invalid, std::ptr::null(), c_int::MAX),
                0
            );
            assert_eq!(
                r_collided(std::ptr::null(), invalid, std::ptr::null(), c_int::MAX),
                0
            );
        }

        for valid_a in [CIRCLE, AABB, CAPSULE] {
            for invalid_b in [-1, 3, 91, c_int::MIN, c_int::MAX] {
                let c = c_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid_b);
                let rust = r_collided(std::ptr::null(), valid_a, std::ptr::null(), invalid_b);
                assert_eq!(c, 0);
                assert_eq!(c, rust, "valid typeA={valid_a}, invalid typeB={invalid_b}");
            }
        }
    }
}
