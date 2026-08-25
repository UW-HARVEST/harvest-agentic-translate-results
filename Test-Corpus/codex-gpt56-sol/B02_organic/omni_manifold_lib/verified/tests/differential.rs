#![allow(non_snake_case)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::mem::{size_of, MaybeUninit};
use std::path::PathBuf;

const CAPSULE: c_int = 0;
const CIRCLE: c_int = 1;
const AABB: c_int = 2;
const POLY: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct V {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct H {
    n: V,
    d: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct R {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct X {
    p: V,
    r: R,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Circle {
    p: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Aabb {
    min: V,
    max: V,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Capsule {
    a: V,
    b: V,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Poly {
    count: c_int,
    verts: [V; 8],
    norms: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Cache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Manifold {
    count: c_int,
    depths: [f32; 2],
    points: [V; 2],
    n: V,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Proxy {
    radius: f32,
    count: c_int,
    verts: [V; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Sv {
    sA: V,
    sB: V,
    p: V,
    u: f32,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Simplex {
    a: Sv,
    b: Sv,
    c: Sv,
    d: Sv,
    div: f32,
    count: c_int,
}

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = std::env::var_os("C_REFERENCE_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("c_src/build/libtranslated_rust.so"));
        let rust_path = std::env::var_os("RUST_TRANSLATION_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/release/libomni_manifold_lib.so"));
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
        (
            *self.c.get::<T>(name).unwrap(),
            *self.rust.get::<T>(name).unwrap(),
        )
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, size_of::<T>()) }
}

fn same<T>(label: &str, c: &T, rust: &T) {
    assert_eq!(bytes(c), bytes(rust), "{label}");
}

fn seeded<T>() -> T {
    let mut value = MaybeUninit::<T>::uninit();
    unsafe {
        std::ptr::write_bytes(value.as_mut_ptr() as *mut u8, 0xA5, size_of::<T>());
        value.assume_init()
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

    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        let unit = (self.u32() >> 8) as f32 / ((1u32 << 24) - 1) as f32;
        lo + (hi - lo) * unit
    }

    fn v(&mut self, lo: f32, hi: f32) -> V {
        V {
            x: self.f(lo, hi),
            y: self.f(lo, hi),
        }
    }
}

const SYMBOLS: &[&[u8]] = &[
    b"c22\0",
    b"c23\0",
    b"c2AABBtoAABBManifold\0",
    b"c2AABBtoCapsuleManifold\0",
    b"c2Absv\0",
    b"c2Add\0",
    b"c2BBVerts\0",
    b"c2CCW90\0",
    b"c2CapsuletoCapsuleManifold\0",
    b"c2CapsuletoPolyManifold\0",
    b"c2CircletoAABBManifold\0",
    b"c2CircletoCapsuleManifold\0",
    b"c2CircletoCircleManifold\0",
    b"c2Clampv\0",
    b"c2Collide\0",
    b"c2D\0",
    b"c2Det2\0",
    b"c2Dist\0",
    b"c2Div\0",
    b"c2Dot\0",
    b"c2GJK\0",
    b"c2GJKSimplexMetric\0",
    b"c2Intersect\0",
    b"c2L\0",
    b"c2Len\0",
    b"c2MakeProxy\0",
    b"c2Maxv\0",
    b"c2Minv\0",
    b"c2Mulrv\0",
    b"c2MulrvT\0",
    b"c2Mulvs\0",
    b"c2Mulxv\0",
    b"c2MulxvT\0",
    b"c2Neg\0",
    b"c2Norm\0",
    b"c2Norms\0",
    b"c2PlaneAt\0",
    b"c2RotIdentity\0",
    b"c2Skew\0",
    b"c2Sub\0",
    b"c2Support\0",
    b"c2V\0",
    b"c2Witness\0",
    b"c2xIdentity\0",
    b"omni_manifold\0",
    b"ptr_from_parts\0",
];

#[test]
fn exported_symbol_surface() {
    let libs = Libs::load();
    for &name in SYMBOLS {
        unsafe {
            libs.c.get::<*const c_void>(name).unwrap();
            libs.rust.get::<*const c_void>(name).unwrap();
        }
    }
}

#[test]
fn vector_transform_and_proxy_surface() {
    type V2 = unsafe extern "C" fn(V, V) -> V;
    type VS = unsafe extern "C" fn(V, f32) -> V;
    type VF = unsafe extern "C" fn(V) -> f32;
    type V1 = unsafe extern "C" fn(V) -> V;
    type Dot = unsafe extern "C" fn(V, V) -> f32;
    type Clamp = unsafe extern "C" fn(V, V, V) -> V;
    type Dist = unsafe extern "C" fn(H, V) -> f32;
    type Plane = unsafe extern "C" fn(*const Poly, c_int) -> H;
    type RotV = unsafe extern "C" fn(R, V) -> V;
    type XV = unsafe extern "C" fn(X, V) -> V;
    type Intersect = unsafe extern "C" fn(V, V, f32, f32) -> V;
    type Construct = unsafe extern "C" fn(f32, f32) -> V;
    type BB = unsafe extern "C" fn(*mut V, *mut Aabb);
    type Make = unsafe extern "C" fn(*const c_void, c_int, *mut Proxy);

    let libs = Libs::load();
    let mut rng = Rng::new(0xC001_C022);
    unsafe {
        let add = libs.pair::<V2>(b"c2Add\0");
        let sub = libs.pair::<V2>(b"c2Sub\0");
        let mul = libs.pair::<VS>(b"c2Mulvs\0");
        let max = libs.pair::<V2>(b"c2Maxv\0");
        let min = libs.pair::<V2>(b"c2Minv\0");
        let clamp = libs.pair::<Clamp>(b"c2Clampv\0");
        let dot = libs.pair::<Dot>(b"c2Dot\0");
        let det = libs.pair::<Dot>(b"c2Det2\0");
        let len = libs.pair::<VF>(b"c2Len\0");
        let abs = libs.pair::<V1>(b"c2Absv\0");
        let neg = libs.pair::<V1>(b"c2Neg\0");
        let ccw = libs.pair::<V1>(b"c2CCW90\0");
        let skew = libs.pair::<V1>(b"c2Skew\0");
        let div = libs.pair::<VS>(b"c2Div\0");
        let norm = libs.pair::<V1>(b"c2Norm\0");
        let dist = libs.pair::<Dist>(b"c2Dist\0");
        let plane = libs.pair::<Plane>(b"c2PlaneAt\0");
        let mulrv = libs.pair::<RotV>(b"c2Mulrv\0");
        let mulrv_t = libs.pair::<RotV>(b"c2MulrvT\0");
        let mulxv = libs.pair::<XV>(b"c2Mulxv\0");
        let mulxv_t = libs.pair::<XV>(b"c2MulxvT\0");
        let intersect = libs.pair::<Intersect>(b"c2Intersect\0");
        let construct = libs.pair::<Construct>(b"c2V\0");

        for _ in 0..512 {
            let a = rng.v(-100.0, 100.0);
            let b = rng.v(-100.0, 100.0);
            let lo = V { x: -20.0, y: -20.0 };
            let hi = V { x: 20.0, y: 20.0 };
            let scalar = rng.f(-10.0, 10.0);
            let divisor = if scalar.abs() < 0.01 { 0.25 } else { scalar };
            let rot = R {
                c: rng.f(-1.0, 1.0),
                s: rng.f(-1.0, 1.0),
            };
            let x = X {
                p: rng.v(-10.0, 10.0),
                r: rot,
            };
            macro_rules! cmp {
                ($label:literal, $pair:expr, $($arg:expr),+) => {{
                    let c = ($pair.0)($($arg),+);
                    let r = ($pair.1)($($arg),+);
                    same($label, &c, &r);
                }};
            }
            cmp!("add", add, a, b);
            cmp!("sub", sub, a, b);
            cmp!("mul", mul, a, scalar);
            cmp!("max", max, a, b);
            cmp!("min", min, a, b);
            cmp!("clamp", clamp, a, lo, hi);
            cmp!("dot", dot, a, b);
            cmp!("det", det, a, b);
            cmp!("len", len, a);
            cmp!("abs", abs, a);
            cmp!("neg", neg, a);
            cmp!("ccw", ccw, a);
            cmp!("skew", skew, a);
            cmp!("div", div, a, divisor);
            if a.x != 0.0 || a.y != 0.0 {
                cmp!("norm", norm, a);
            }
            cmp!("dist", dist, H { n: a, d: scalar }, b);
            cmp!("mulrv", mulrv, rot, a);
            cmp!("mulrv_t", mulrv_t, rot, a);
            cmp!("mulxv", mulxv, x, a);
            cmp!("mulxv_t", mulxv_t, x, a);
            cmp!("construct", construct, a.x, a.y);
            let da = rng.f(0.01, 10.0);
            let db = -rng.f(0.01, 10.0);
            cmp!("intersect", intersect, a, b, da, db);

            let mut poly = Poly::default();
            poly.count = 8;
            for i in 0..8 {
                poly.verts[i] = rng.v(-10.0, 10.0);
                poly.norms[i] = rng.v(-1.0, 1.0);
                let ch = plane.0(&poly, i as c_int);
                let rh = plane.1(&poly, i as c_int);
                same("plane", &ch, &rh);
            }
        }

        let equal = V { x: 2.0, y: -3.0 };
        same("equal max", &max.0(equal, equal), &max.1(equal, equal));
        same("equal min", &min.0(equal, equal), &min.1(equal, equal));
        same(
            "equal clamp",
            &clamp.0(equal, equal, equal),
            &clamp.1(equal, equal, equal),
        );
        let endpoint_a = V { x: -1.0, y: 2.0 };
        let endpoint_b = V { x: 3.0, y: -4.0 };
        same(
            "intersection at A",
            &intersect.0(endpoint_a, endpoint_b, 0.0, -1.0),
            &intersect.1(endpoint_a, endpoint_b, 0.0, -1.0),
        );
        same(
            "intersection at B",
            &intersect.0(endpoint_a, endpoint_b, 1.0, 0.0),
            &intersect.1(endpoint_a, endpoint_b, 1.0, 0.0),
        );
        let zero = V::default();
        same("zero norm", &norm.0(zero), &norm.1(zero));

        type IdentityR = unsafe extern "C" fn() -> R;
        type IdentityX = unsafe extern "C" fn() -> X;
        let ri = libs.pair::<IdentityR>(b"c2RotIdentity\0");
        let xi = libs.pair::<IdentityX>(b"c2xIdentity\0");
        same("rot identity", &ri.0(), &ri.1());
        same("x identity", &xi.0(), &xi.1());

        let bbverts = libs.pair::<BB>(b"c2BBVerts\0");
        let make = libs.pair::<Make>(b"c2MakeProxy\0");
        for _ in 0..256 {
            let center = rng.v(-20.0, 20.0);
            let ext = rng.v(0.01, 5.0);
            let mut bb = Aabb {
                min: V {
                    x: center.x - ext.x,
                    y: center.y - ext.y,
                },
                max: V {
                    x: center.x + ext.x,
                    y: center.y + ext.y,
                },
            };
            let mut co = [V::default(); 4];
            let mut ro = [V::default(); 4];
            bbverts.0(co.as_mut_ptr(), &mut bb);
            bbverts.1(ro.as_mut_ptr(), &mut bb);
            same("bb verts", &co, &ro);

            let circle = Circle {
                p: center,
                r: rng.f(0.01, 4.0),
            };
            let cap = Capsule {
                a: rng.v(-10.0, 10.0),
                b: rng.v(-10.0, 10.0),
                r: rng.f(0.01, 4.0),
            };
            for (kind, ptr) in [
                (CIRCLE, &circle as *const _ as *const c_void),
                (AABB, &bb as *const _ as *const c_void),
                (CAPSULE, &cap as *const _ as *const c_void),
            ] {
                let mut cp: Proxy = seeded();
                let mut rp = cp;
                make.0(ptr, kind, &mut cp);
                make.1(ptr, kind, &mut rp);
                same("proxy", &cp, &rp);
            }
            for kind in [POLY, -1, 4, c_int::MAX] {
                let mut cp: Proxy = seeded();
                let mut rp = cp;
                make.0(&circle as *const _ as *const c_void, kind, &mut cp);
                make.1(&circle as *const _ as *const c_void, kind, &mut rp);
                same("unsupported proxy", &cp, &rp);
            }
        }
    }
}

fn classify_c23(s: &Simplex) -> usize {
    fn sub(a: V, b: V) -> V {
        V {
            x: a.x - b.x,
            y: a.y - b.y,
        }
    }
    fn dot(a: V, b: V) -> f32 {
        a.x * b.x + a.y * b.y
    }
    fn det(a: V, b: V) -> f32 {
        a.x * b.y - a.y * b.x
    }
    let (a, b, c) = (s.a.p, s.b.p, s.c.p);
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uABC = det(b, c) * area;
    let vABC = det(c, a) * area;
    let wABC = det(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        0
    } else if uAB <= 0.0 && vBC <= 0.0 {
        1
    } else if uBC <= 0.0 && vCA <= 0.0 {
        2
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        3
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        4
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        5
    } else {
        6
    }
}

#[test]
fn simplex_surface() {
    type MutS = unsafe extern "C" fn(*mut Simplex);
    type SF = unsafe extern "C" fn(*mut Simplex) -> f32;
    type SV = unsafe extern "C" fn(*mut Simplex) -> V;
    type Support = unsafe extern "C" fn(*const V, c_int, V) -> c_int;
    type Witness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);
    let libs = Libs::load();
    let mut rng = Rng::new(0xC023_C046);
    unsafe {
        let c22 = libs.pair::<MutS>(b"c22\0");
        let c23 = libs.pair::<MutS>(b"c23\0");
        let metric = libs.pair::<SF>(b"c2GJKSimplexMetric\0");
        let direction = libs.pair::<SV>(b"c2D\0");
        let location = libs.pair::<SV>(b"c2L\0");
        let support = libs.pair::<Support>(b"c2Support\0");
        let witness = libs.pair::<Witness>(b"c2Witness\0");

        let c22_inputs = [
            (V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }),
            (V { x: -2.0, y: 0.0 }, V { x: -1.0, y: 0.0 }),
            (V { x: -1.0, y: 0.0 }, V { x: 1.0, y: 0.0 }),
        ];
        for &(a, b) in &c22_inputs {
            for _ in 0..64 {
                let jitter = rng.f(0.8, 1.2);
                let mut cs = Simplex::default();
                cs.a.p = V {
                    x: a.x * jitter,
                    y: a.y,
                };
                cs.b.p = V {
                    x: b.x * jitter,
                    y: b.y,
                };
                cs.count = 2;
                let mut rs = cs;
                c22.0(&mut cs);
                c22.1(&mut rs);
                same("c22", &cs, &rs);
            }
        }

        let mut branch_counts = [0usize; 7];
        for _ in 0..200_000 {
            if branch_counts.iter().all(|&n| n >= 64) {
                break;
            }
            let mut cs = Simplex::default();
            cs.a.p = rng.v(-5.0, 5.0);
            cs.b.p = rng.v(-5.0, 5.0);
            cs.c.p = rng.v(-5.0, 5.0);
            cs.count = 3;
            let branch = classify_c23(&cs);
            if branch_counts[branch] >= 64 {
                continue;
            }
            branch_counts[branch] += 1;
            let mut rs = cs;
            c23.0(&mut cs);
            c23.1(&mut rs);
            same("c23", &cs, &rs);
        }
        assert!(branch_counts.iter().all(|&n| n >= 64), "{branch_counts:?}");

        for count in [0, 1, 2, 3, 4] {
            for _ in 0..128 {
                let mut cs = Simplex::default();
                cs.count = count;
                cs.div = rng.f(0.1, 10.0);
                for sv in [&mut cs.a, &mut cs.b, &mut cs.c] {
                    sv.p = rng.v(-10.0, 10.0);
                    sv.sA = rng.v(-10.0, 10.0);
                    sv.sB = rng.v(-10.0, 10.0);
                    sv.u = rng.f(0.1, 5.0);
                }
                let mut rs = cs;
                same("metric", &metric.0(&mut cs), &metric.1(&mut rs));
                same("direction", &direction.0(&mut cs), &direction.1(&mut rs));
                same("location", &location.0(&mut cs), &location.1(&mut rs));
                let mut ca = seeded::<V>();
                let mut cb = seeded::<V>();
                let mut ra = ca;
                let mut rb = cb;
                witness.0(&mut cs, &mut ca, &mut cb);
                witness.1(&mut rs, &mut ra, &mut rb);
                same("witness a", &ca, &ra);
                same("witness b", &cb, &rb);
            }
        }

        for count in 1..=8 {
            for _ in 0..128 {
                let mut verts = [V::default(); 8];
                for v in &mut verts[..count] {
                    *v = rng.v(-10.0, 10.0);
                }
                let d = rng.v(-2.0, 2.0);
                same(
                    "support",
                    &support.0(verts.as_ptr(), count as c_int, d),
                    &support.1(verts.as_ptr(), count as c_int, d),
                );
                verts[0] = V { x: 1.0, y: 1.0 };
                verts[1.min(count - 1)] = verts[0];
                same(
                    "support ties",
                    &support.0(verts.as_ptr(), count as c_int, V::default()),
                    &support.1(verts.as_ptr(), count as c_int, V::default()),
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Shape {
    kind: c_int,
    circle: Circle,
    aabb: Aabb,
    capsule: Capsule,
}

impl Shape {
    fn ptr(&self) -> *const c_void {
        match self.kind {
            CIRCLE => &self.circle as *const _ as *const c_void,
            AABB => &self.aabb as *const _ as *const c_void,
            CAPSULE => &self.capsule as *const _ as *const c_void,
            _ => std::ptr::null(),
        }
    }
}

fn shape(rng: &mut Rng, kind: c_int, offset: f32) -> Shape {
    let center = V {
        x: offset + rng.f(-0.5, 0.5),
        y: rng.f(-2.0, 2.0),
    };
    let ex = rng.f(0.2, 1.5);
    let ey = rng.f(0.2, 1.5);
    Shape {
        kind,
        circle: Circle {
            p: center,
            r: rng.f(0.1, 1.5),
        },
        aabb: Aabb {
            min: V {
                x: center.x - ex,
                y: center.y - ey,
            },
            max: V {
                x: center.x + ex,
                y: center.y + ey,
            },
        },
        capsule: Capsule {
            a: V {
                x: center.x - ex,
                y: center.y - ey * 0.25,
            },
            b: V {
                x: center.x + ex,
                y: center.y + ey * 0.25,
            },
            r: rng.f(0.1, 1.2),
        },
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
    pair: (Gjk, Gjk),
    a: &Shape,
    b: &Shape,
    ax: Option<&X>,
    bx: Option<&X>,
    use_radius: c_int,
    cache_seed: Option<Cache>,
    outputs: (bool, bool, bool),
) -> (Cache, Cache) {
    let mut ca: V = seeded();
    let mut cb: V = seeded();
    let mut ra = ca;
    let mut rb = cb;
    let mut ci: c_int = 0x5A5A;
    let mut ri = ci;
    let mut cc = cache_seed.unwrap_or_default();
    let mut rc = cc;
    let axp = ax.map_or(std::ptr::null(), |v| v);
    let bxp = bx.map_or(std::ptr::null(), |v| v);
    let cache_present = cache_seed.is_some();
    let cd = pair.0(
        a.ptr(),
        a.kind,
        axp,
        b.ptr(),
        b.kind,
        bxp,
        if outputs.0 {
            &mut ca
        } else {
            std::ptr::null_mut()
        },
        if outputs.1 {
            &mut cb
        } else {
            std::ptr::null_mut()
        },
        use_radius,
        if outputs.2 {
            &mut ci
        } else {
            std::ptr::null_mut()
        },
        if cache_present {
            &mut cc
        } else {
            std::ptr::null_mut()
        },
    );
    let rd = pair.1(
        a.ptr(),
        a.kind,
        axp,
        b.ptr(),
        b.kind,
        bxp,
        if outputs.0 {
            &mut ra
        } else {
            std::ptr::null_mut()
        },
        if outputs.1 {
            &mut rb
        } else {
            std::ptr::null_mut()
        },
        use_radius,
        if outputs.2 {
            &mut ri
        } else {
            std::ptr::null_mut()
        },
        if cache_present {
            &mut rc
        } else {
            std::ptr::null_mut()
        },
    );
    same("GJK distance", &cd, &rd);
    same("GJK outA", &ca, &ra);
    same("GJK outB", &cb, &rb);
    same("GJK iterations", &ci, &ri);
    if cache_present {
        same("GJK cache", &cc, &rc);
    }
    (cc, rc)
}

#[test]
fn gjk_surface() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xC047_C061);
    unsafe {
        let gjk = libs.pair::<Gjk>(b"c2GJK\0");
        for ak in [CIRCLE, AABB, CAPSULE] {
            for bk in [CIRCLE, AABB, CAPSULE] {
                for i in 0..128 {
                    let separated = i % 2 == 0;
                    let a = shape(&mut rng, ak, -if separated { 5.0 } else { 0.5 });
                    let b = shape(&mut rng, bk, if separated { 5.0 } else { 0.5 });
                    compare_gjk(gjk, &a, &b, None, None, 0, None, (true, true, true));
                    compare_gjk(gjk, &a, &b, None, None, 1, None, (true, true, true));
                    let angle = rng.f(-3.0, 3.0);
                    let (s, c) = angle.sin_cos();
                    let ax = X {
                        p: rng.v(-2.0, 2.0),
                        r: R { c, s },
                    };
                    let bx = X {
                        p: rng.v(-2.0, 2.0),
                        r: R { c: c, s: -s },
                    };
                    compare_gjk(
                        gjk,
                        &a,
                        &b,
                        Some(&ax),
                        Some(&bx),
                        0,
                        None,
                        (true, true, true),
                    );
                    compare_gjk(
                        gjk,
                        &a,
                        &b,
                        None,
                        None,
                        0,
                        Some(Cache::default()),
                        (true, true, true),
                    );
                    for outputs in [
                        (false, true, true),
                        (true, false, true),
                        (true, true, false),
                        (false, false, false),
                    ] {
                        compare_gjk(gjk, &a, &b, None, None, 0, None, outputs);
                    }
                    let (warm_c, warm_r) = compare_gjk(
                        gjk,
                        &a,
                        &b,
                        None,
                        None,
                        0,
                        Some(Cache::default()),
                        (true, true, true),
                    );
                    same("warm seed", &warm_c, &warm_r);
                    compare_gjk(gjk, &a, &b, None, None, 0, Some(warm_c), (true, true, true));
                }
            }
        }
    }
}

#[test]
fn manifold_and_dispatch_surface() {
    type CC = unsafe extern "C" fn(Circle, Circle, *mut Manifold);
    type CA = unsafe extern "C" fn(Circle, Aabb, *mut Manifold);
    type CP = unsafe extern "C" fn(Circle, Capsule, *mut Manifold);
    type AA = unsafe extern "C" fn(Aabb, Aabb, *mut Manifold);
    type PP = unsafe extern "C" fn(Capsule, Capsule, *mut Manifold);
    type Norms = unsafe extern "C" fn(*mut V, *mut V, c_int);
    type Collide = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut Manifold);
    let libs = Libs::load();
    let mut rng = Rng::new(0xC062_C079);
    unsafe {
        let cc = libs.pair::<CC>(b"c2CircletoCircleManifold\0");
        let ca = libs.pair::<CA>(b"c2CircletoAABBManifold\0");
        let cp = libs.pair::<CP>(b"c2CircletoCapsuleManifold\0");
        let aa = libs.pair::<AA>(b"c2AABBtoAABBManifold\0");
        let pp = libs.pair::<PP>(b"c2CapsuletoCapsuleManifold\0");
        let norms = libs.pair::<Norms>(b"c2Norms\0");
        let collide = libs.pair::<Collide>(b"c2Collide\0");

        for _ in 0..512 {
            let offsets = [
                rng.f(-4.0, 0.0),
                rng.f(0.0, 4.0),
                rng.f(-4.0, 0.0),
                rng.f(0.0, 4.0),
                rng.f(-4.0, 0.0),
                rng.f(0.0, 4.0),
            ];
            let s1 = shape(&mut rng, CIRCLE, offsets[0]);
            let s2 = shape(&mut rng, CIRCLE, offsets[1]);
            let a1 = shape(&mut rng, AABB, offsets[2]);
            let a2 = shape(&mut rng, AABB, offsets[3]);
            let p1 = shape(&mut rng, CAPSULE, offsets[4]);
            let p2 = shape(&mut rng, CAPSULE, offsets[5]);
            macro_rules! manifold {
                ($label:literal, $pair:expr, $($arg:expr),+) => {{
                    let mut cm: Manifold = seeded();
                    let mut rm = cm;
                    ($pair.0)($($arg),+, &mut cm);
                    ($pair.1)($($arg),+, &mut rm);
                    same($label, &cm, &rm);
                }};
            }
            manifold!("circle circle", cc, s1.circle, s2.circle);
            manifold!("circle aabb", ca, s1.circle, a2.aabb);
            manifold!("circle capsule", cp, s1.circle, p2.capsule);
            manifold!("aabb aabb", aa, a1.aabb, a2.aabb);
            manifold!("capsule capsule", pp, p1.capsule, p2.capsule);

            for (left, right) in [
                (s1, s2),
                (s1, a2),
                (s1, p2),
                (a1, s2),
                (a1, a2),
                (p1, s2),
                (p1, p2),
            ] {
                let mut cm: Manifold = seeded();
                let mut rm = cm;
                collide.0(left.ptr(), left.kind, right.ptr(), right.kind, &mut cm);
                collide.1(left.ptr(), left.kind, right.ptr(), right.kind, &mut rm);
                same("collide", &cm, &rm);
            }
        }

        for count in 1..=8 {
            for _ in 0..128 {
                let mut cv = [V::default(); 8];
                let mut rv = cv;
                for i in 0..count {
                    let angle = i as f32 * std::f32::consts::TAU / count as f32;
                    cv[i] = V {
                        x: angle.cos() * rng.f(0.5, 2.0),
                        y: angle.sin() * rng.f(0.5, 2.0),
                    };
                    rv[i] = cv[i];
                }
                let mut cn: [V; 8] = seeded();
                let mut rn = cn;
                norms.0(cv.as_mut_ptr(), cn.as_mut_ptr(), count as c_int);
                norms.1(rv.as_mut_ptr(), rn.as_mut_ptr(), count as c_int);
                same("norm verts", &cv, &rv);
                same("norm outputs", &cn, &rn);
            }
        }
        let mut cv: [V; 8] = seeded();
        let mut rv = cv;
        let mut cn: [V; 8] = seeded();
        let mut rn = cn;
        norms.0(cv.as_mut_ptr(), cn.as_mut_ptr(), 0);
        norms.1(rv.as_mut_ptr(), rn.as_mut_ptr(), 0);
        same("zero norms vertices", &cv, &rv);
        same("zero norms outputs", &cn, &rn);

        for outer in [POLY, -1, 4, c_int::MAX] {
            let valid = shape(&mut rng, CIRCLE, 0.0);
            let mut cm: Manifold = seeded();
            let mut rm = cm;
            collide.0(valid.ptr(), outer, valid.ptr(), CIRCLE, &mut cm);
            collide.1(valid.ptr(), outer, valid.ptr(), CIRCLE, &mut rm);
            same("invalid outer collide", &cm, &rm);
            for inner in [POLY, -1, 4, c_int::MAX] {
                cm = seeded();
                rm = cm;
                collide.0(valid.ptr(), CIRCLE, valid.ptr(), inner, &mut cm);
                collide.1(valid.ptr(), CIRCLE, valid.ptr(), inner, &mut rm);
                same("invalid inner collide", &cm, &rm);
            }
        }
    }
}

#[test]
fn manifold_boundary_and_error_surface() {
    type CC = unsafe extern "C" fn(Circle, Circle, *mut Manifold);
    type CA = unsafe extern "C" fn(Circle, Aabb, *mut Manifold);
    type CP = unsafe extern "C" fn(Circle, Capsule, *mut Manifold);
    type AA = unsafe extern "C" fn(Aabb, Aabb, *mut Manifold);
    type PP = unsafe extern "C" fn(Capsule, Capsule, *mut Manifold);
    let libs = Libs::load();
    unsafe {
        let cc = libs.pair::<CC>(b"c2CircletoCircleManifold\0");
        let ca = libs.pair::<CA>(b"c2CircletoAABBManifold\0");
        let cp = libs.pair::<CP>(b"c2CircletoCapsuleManifold\0");
        let aa = libs.pair::<AA>(b"c2AABBtoAABBManifold\0");
        let pp = libs.pair::<PP>(b"c2CapsuletoCapsuleManifold\0");
        macro_rules! manifold {
            ($label:literal, $pair:expr, $($arg:expr),+) => {{
                let mut cm: Manifold = seeded();
                let mut rm = cm;
                ($pair.0)($($arg),+, &mut cm);
                ($pair.1)($($arg),+, &mut rm);
                same($label, &cm, &rm);
            }};
        }

        let unit_circle = Circle {
            p: V::default(),
            r: 1.0,
        };
        for x in [0.0, 0.5, 2.0, 2.5] {
            manifold!(
                "circle-circle boundary",
                cc,
                unit_circle,
                Circle {
                    p: V { x, y: 0.0 },
                    r: 1.0
                }
            );
        }

        let box1 = Aabb {
            min: V { x: -1.0, y: -1.0 },
            max: V { x: 1.0, y: 1.0 },
        };
        for center in [
            V { x: 0.0, y: 0.0 },
            V { x: 0.9, y: 0.0 },
            V { x: 0.0, y: 0.9 },
            V { x: 2.0, y: 0.0 },
            V { x: 2.5, y: 0.0 },
        ] {
            manifold!(
                "circle-AABB boundary",
                ca,
                Circle { p: center, r: 1.0 },
                box1
            );
        }

        let horizontal = Capsule {
            a: V { x: -1.0, y: 0.0 },
            b: V { x: 1.0, y: 0.0 },
            r: 0.5,
        };
        for y in [0.0, 1.0, 1.5, 2.0] {
            manifold!(
                "circle-capsule boundary",
                cp,
                Circle {
                    p: V { x: 0.0, y },
                    r: 1.0
                },
                horizontal
            );
        }

        for b in [
            Aabb {
                min: V { x: 3.0, y: -1.0 },
                max: V { x: 5.0, y: 1.0 },
            },
            Aabb {
                min: V { x: -1.0, y: 3.0 },
                max: V { x: 1.0, y: 5.0 },
            },
            Aabb {
                min: V { x: -0.5, y: -1.0 },
                max: V { x: 1.5, y: 1.0 },
            },
            Aabb {
                min: V { x: -1.0, y: -0.5 },
                max: V { x: 1.0, y: 1.5 },
            },
        ] {
            manifold!("AABB-AABB boundary", aa, box1, b);
            manifold!("AABB-AABB signed boundary", aa, b, box1);
        }

        for b in [
            Capsule {
                a: V { x: -1.0, y: 0.0 },
                b: V { x: 1.0, y: 0.0 },
                r: 0.5,
            },
            Capsule {
                a: V { x: 0.0, y: -1.0 },
                b: V { x: 0.0, y: 1.0 },
                r: 0.5,
            },
            Capsule {
                a: V { x: -1.0, y: 1.0 },
                b: V { x: 1.0, y: 1.0 },
                r: 0.5,
            },
            Capsule {
                a: V { x: -1.0, y: 2.0 },
                b: V { x: 1.0, y: 2.0 },
                r: 0.5,
            },
        ] {
            manifold!("capsule-capsule boundary", pp, horizontal, b);
        }
    }
}

#[test]
fn allocation_and_omni_surface() {
    type Parts = unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void;
    type Omni = unsafe extern "C" fn(
        *mut Manifold,
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
    unsafe extern "C" {
        fn free(ptr: *mut c_void);
    }
    let libs = Libs::load();
    let mut rng = Rng::new(0xC080_C088);
    unsafe {
        let parts = libs.pair::<Parts>(b"ptr_from_parts\0");
        let omni = libs.pair::<Omni>(b"omni_manifold\0");
        for _ in 0..256 {
            let args = [
                rng.f(-5.0, 5.0),
                rng.f(-5.0, 5.0),
                rng.f(0.1, 3.0),
                rng.f(-5.0, 5.0),
                rng.f(0.1, 3.0),
            ];
            for kind in [CIRCLE, AABB, CAPSULE] {
                let cp = parts.0(kind, args[0], args[1], args[2], args[3], args[4]);
                let rp = parts.1(kind, args[0], args[1], args[2], args[3], args[4]);
                assert!(!cp.is_null() && !rp.is_null());
                let len = match kind {
                    CIRCLE => size_of::<Circle>(),
                    AABB => size_of::<Aabb>(),
                    _ => size_of::<Capsule>(),
                };
                assert_eq!(
                    std::slice::from_raw_parts(cp as *const u8, len),
                    std::slice::from_raw_parts(rp as *const u8, len),
                    "ptr_from_parts"
                );
                free(cp);
                free(rp);
            }
            for ak in [CIRCLE, AABB, CAPSULE] {
                for bk in [CIRCLE, AABB, CAPSULE] {
                    if (ak == AABB && bk == CAPSULE) || (ak == CAPSULE && bk == AABB) {
                        continue;
                    }
                    let mut cm: Manifold = seeded();
                    let mut rm = cm;
                    omni.0(
                        &mut cm, ak, args[0], args[1], args[2], args[3], args[4], bk, -args[0],
                        -args[1], args[2], -args[3], args[4],
                    );
                    omni.1(
                        &mut rm, ak, args[0], args[1], args[2], args[3], args[4], bk, -args[0],
                        -args[1], args[2], -args[3], args[4],
                    );
                    same("omni", &cm, &rm);
                }
            }
        }
    }
}
