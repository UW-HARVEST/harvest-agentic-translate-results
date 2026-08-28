use libloading::Library;
use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
use std::ffi::{c_float, c_int, c_void};
use std::path::PathBuf;
use std::ptr::{null, null_mut};

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
    ia: [c_int; 3],
    ib: [c_int; 3],
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
        Self {
            radius: 0.0,
            count: 0,
            verts: [V::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Sv {
    sa: V,
    sb: V,
    p: V,
    u: c_float,
    ia: c_int,
    ib: c_int,
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

struct Libs {
    _libm: UnixLibrary,
    c: Library,
    r: Library,
}

impl Libs {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c = root
            .parent()
            .unwrap()
            .join("c_src/build/libharvest-work-r4rkXW.so");
        let r = root.join("target/release/libreverse_collide_lib.so");
        assert!(c.is_file(), "missing C shared library: {}", c.display());
        assert!(r.is_file(), "missing Rust shared library: {}", r.display());
        unsafe {
            let libm = UnixLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL).unwrap();
            Self {
                _libm: libm,
                c: Library::new(c).unwrap(),
                r: Library::new(r).unwrap(),
            }
        }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        unsafe {
            (
                *self.c.get::<T>(name).unwrap(),
                *self.r.get::<T>(name).unwrap(),
            )
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d59_5df4_d0f3_3173)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn f(&mut self) -> f32 {
        (self.next_u32() % 4001) as f32 / 16.0 - 125.0
    }

    fn positive(&mut self) -> f32 {
        (self.next_u32() % 801) as f32 / 16.0
    }

    fn v(&mut self) -> V {
        V {
            x: self.f(),
            y: self.f(),
        }
    }
}

fn assert_f(c: f32, r: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={r:?} ({:#010x})",
        c.to_bits(),
        r.to_bits()
    );
}

fn assert_v(c: V, r: V, context: &str) {
    assert_f(c.x, r.x, &format!("{context}.x"));
    assert_f(c.y, r.y, &format!("{context}.y"));
}

fn assert_sv(c: Sv, r: Sv, context: &str) {
    assert_v(c.sa, r.sa, &format!("{context}.sa"));
    assert_v(c.sb, r.sb, &format!("{context}.sb"));
    assert_v(c.p, r.p, &format!("{context}.p"));
    assert_f(c.u, r.u, &format!("{context}.u"));
    assert_eq!(c.ia, r.ia, "{context}.ia");
    assert_eq!(c.ib, r.ib, "{context}.ib");
}

fn assert_simplex(c: Simplex, r: Simplex, context: &str) {
    assert_sv(c.a, r.a, &format!("{context}.a"));
    assert_sv(c.b, r.b, &format!("{context}.b"));
    assert_sv(c.c, r.c, &format!("{context}.c"));
    assert_sv(c.d, r.d, &format!("{context}.d"));
    assert_f(c.div, r.div, &format!("{context}.div"));
    assert_eq!(c.count, r.count, "{context}.count");
}

fn assert_cache(c: Cache, r: Cache, context: &str) {
    assert_f(c.metric, r.metric, &format!("{context}.metric"));
    assert_eq!(c.count, r.count, "{context}.count");
    assert_eq!(c.ia, r.ia, "{context}.ia");
    assert_eq!(c.ib, r.ib, "{context}.ib");
    assert_f(c.div, r.div, &format!("{context}.div"));
}

#[test]
fn vector_transform_and_proxy_surface() {
    unsafe {
        let libs = Libs::load();
        let (cv, rv) = libs.pair::<unsafe extern "C" fn(f32, f32) -> V>(b"c2V\0");
        let (cmul, rmul) = libs.pair::<unsafe extern "C" fn(V, f32) -> V>(b"c2Mulvs\0");
        let (cadd, radd) = libs.pair::<unsafe extern "C" fn(V, V) -> V>(b"c2Add\0");
        let (csub, rsub) = libs.pair::<unsafe extern "C" fn(V, V) -> V>(b"c2Sub\0");
        let (cdot, rdot) = libs.pair::<unsafe extern "C" fn(V, V) -> f32>(b"c2Dot\0");
        let (cdet, rdet) = libs.pair::<unsafe extern "C" fn(V, V) -> f32>(b"c2Det2\0");
        let (cmax, rmax) = libs.pair::<unsafe extern "C" fn(V, V) -> V>(b"c2Maxv\0");
        let (cmin, rmin) = libs.pair::<unsafe extern "C" fn(V, V) -> V>(b"c2Minv\0");
        let (cclamp, rclamp) = libs.pair::<unsafe extern "C" fn(V, V, V) -> V>(b"c2Clampv\0");
        let (clen, rlen) = libs.pair::<unsafe extern "C" fn(V) -> f32>(b"c2Len\0");
        let (cdiv, rdiv) = libs.pair::<unsafe extern "C" fn(V, f32) -> V>(b"c2Div\0");
        let (cnorm, rnorm) = libs.pair::<unsafe extern "C" fn(V) -> V>(b"c2Norm\0");
        let (cneg, rneg) = libs.pair::<unsafe extern "C" fn(V) -> V>(b"c2Neg\0");
        let (cskew, rskew) = libs.pair::<unsafe extern "C" fn(V) -> V>(b"c2Skew\0");
        let (cccw, rccw) = libs.pair::<unsafe extern "C" fn(V) -> V>(b"c2CCW90\0");
        let (cmulr, rmulr) = libs.pair::<unsafe extern "C" fn(R, V) -> V>(b"c2Mulrv\0");
        let (cmulrt, rmulrt) = libs.pair::<unsafe extern "C" fn(R, V) -> V>(b"c2MulrvT\0");
        let (cmulx, rmulx) = libs.pair::<unsafe extern "C" fn(X, V) -> V>(b"c2Mulxv\0");
        let (crid, rrid) = libs.pair::<unsafe extern "C" fn() -> R>(b"c2RotIdentity\0");
        let (cxid, rxid) = libs.pair::<unsafe extern "C" fn() -> X>(b"c2xIdentity\0");
        let (cbb, rbb) = libs.pair::<unsafe extern "C" fn(*mut V, *mut Aabb)>(b"c2BBVerts\0");
        let (cproxy, rproxy) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)>(b"c2MakeProxy\0");

        let ci = crid();
        let ri = rrid();
        assert_f(ci.c, ri.c, "rotation identity c");
        assert_f(ci.s, ri.s, "rotation identity s");
        let ci = cxid();
        let ri = rxid();
        assert_v(ci.p, ri.p, "transform identity p");
        assert_f(ci.r.c, ri.r.c, "transform identity c");
        assert_f(ci.r.s, ri.r.s, "transform identity s");

        let mut rng = Rng::new();
        for i in 0..256 {
            let a = rng.v();
            let b = rng.v();
            let scalar = {
                let value = rng.f();
                if value == 0.0 { 1.0 } else { value }
            };
            assert_v(cv(a.x, a.y), rv(a.x, a.y), &format!("c2V {i}"));
            assert_v(cmul(a, scalar), rmul(a, scalar), &format!("c2Mulvs {i}"));
            assert_v(cadd(a, b), radd(a, b), &format!("c2Add {i}"));
            assert_v(csub(a, b), rsub(a, b), &format!("c2Sub {i}"));
            assert_f(cdot(a, b), rdot(a, b), &format!("c2Dot {i}"));
            assert_f(cdet(a, b), rdet(a, b), &format!("c2Det2 {i}"));
            assert_v(cmax(a, b), rmax(a, b), &format!("c2Maxv {i}"));
            assert_v(cmin(a, b), rmin(a, b), &format!("c2Minv {i}"));
        }

        // Re-run clamp with identical operands for both libraries.
        for i in 0..256 {
            let a = rng.v();
            let b = rng.v();
            let value = rng.v();
            let lo = V {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            };
            let hi = V {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            };
            assert_v(
                cclamp(value, lo, hi),
                rclamp(value, lo, hi),
                &format!("clamp {i}"),
            );
            assert_f(clen(a), rlen(a), &format!("len {i}"));
            let divisor = if b.x == 0.0 { 1.0 } else { b.x };
            assert_v(cdiv(a, divisor), rdiv(a, divisor), &format!("div {i}"));
            if a.x != 0.0 || a.y != 0.0 {
                assert_v(cnorm(a), rnorm(a), &format!("norm {i}"));
            }
            assert_v(cneg(a), rneg(a), &format!("neg {i}"));
            assert_v(cskew(a), rskew(a), &format!("skew {i}"));
            assert_v(cccw(a), rccw(a), &format!("ccw {i}"));
            let rotation = R {
                c: if i & 1 == 0 { 1.0 } else { 0.0 },
                s: if i & 1 == 0 { 0.0 } else { 1.0 },
            };
            assert_v(cmulr(rotation, a), rmulr(rotation, a), &format!("mulr {i}"));
            assert_v(
                cmulrt(rotation, a),
                rmulrt(rotation, a),
                &format!("mulrt {i}"),
            );
            let transform = X { p: b, r: rotation };
            assert_v(
                cmulx(transform, a),
                rmulx(transform, a),
                &format!("mulx {i}"),
            );

            let mut cverts = [V::default(); 4];
            let mut rverts = [V::default(); 4];
            let mut cbox = Aabb { min: lo, max: hi };
            let mut rbox = cbox;
            cbb(cverts.as_mut_ptr(), &mut cbox);
            rbb(rverts.as_mut_ptr(), &mut rbox);
            for j in 0..4 {
                assert_v(cverts[j], rverts[j], &format!("bbverts {i}/{j}"));
            }

            let circle = Circle {
                p: a,
                r: rng.positive(),
            };
            let capsule = Capsule {
                a,
                b,
                r: rng.positive(),
            };
            for (kind, shape) in [
                (0, (&circle as *const Circle).cast()),
                (1, (&cbox as *const Aabb).cast()),
                (2, (&capsule as *const Capsule).cast()),
            ] {
                let mut cp = Proxy::default();
                let mut rp = Proxy::default();
                cproxy(shape, kind, &mut cp);
                rproxy(shape, kind, &mut rp);
                assert_f(cp.radius, rp.radius, "proxy radius");
                assert_eq!(cp.count, rp.count, "proxy count");
                for j in 0..cp.count as usize {
                    assert_v(cp.verts[j], rp.verts[j], "proxy vertex");
                }
            }
        }

        let special = [
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            f32::MIN,
            f32::from_bits(1),
            f32::from_bits(0x8000_0001),
            f32::from_bits(0x7fc0_1234),
            f32::from_bits(0xffc0_5678),
        ];
        for (i, &x) in special.iter().enumerate() {
            for (j, &y) in special.iter().enumerate() {
                let a = V { x, y };
                let b = V { x: y, y: x };
                let context = format!("special {i}/{j}");
                assert_v(cv(x, y), rv(x, y), &format!("{context} V"));
                assert_v(cmul(a, y), rmul(a, y), &format!("{context} mul"));
                assert_v(cadd(a, b), radd(a, b), &format!("{context} add"));
                assert_v(csub(a, b), rsub(a, b), &format!("{context} sub"));
                assert_f(cdot(a, b), rdot(a, b), &format!("{context} dot"));
                assert_f(cdet(a, b), rdet(a, b), &format!("{context} det"));
                assert_v(cmax(a, b), rmax(a, b), &format!("{context} max"));
                assert_v(cmin(a, b), rmin(a, b), &format!("{context} min"));
                assert_v(
                    cclamp(a, b, a),
                    rclamp(a, b, a),
                    &format!("{context} clamp"),
                );
                assert_f(clen(a), rlen(a), &format!("{context} len"));
                assert_v(cdiv(a, y), rdiv(a, y), &format!("{context} div"));
                assert_v(cnorm(a), rnorm(a), &format!("{context} norm"));
                assert_v(cneg(a), rneg(a), &format!("{context} neg"));
                assert_v(cskew(a), rskew(a), &format!("{context} skew"));
                assert_v(cccw(a), rccw(a), &format!("{context} ccw"));
                let rotation = R { c: x, s: y };
                assert_v(
                    cmulr(rotation, a),
                    rmulr(rotation, a),
                    &format!("{context} mulr"),
                );
                assert_v(
                    cmulrt(rotation, a),
                    rmulrt(rotation, a),
                    &format!("{context} mulrt"),
                );
                let transform = X { p: b, r: rotation };
                assert_v(
                    cmulx(transform, a),
                    rmulx(transform, a),
                    &format!("{context} mulx"),
                );
            }
        }
    }
}

fn simplex(points: &[V]) -> Simplex {
    let mut s = Simplex::default();
    s.count = points.len() as c_int;
    s.div = 1.0;
    for (i, point) in points.iter().enumerate() {
        let vertex = match i {
            0 => &mut s.a,
            1 => &mut s.b,
            2 => &mut s.c,
            _ => &mut s.d,
        };
        vertex.p = *point;
        vertex.sa = V {
            x: point.x + 3.0,
            y: point.y - 2.0,
        };
        vertex.sb = V {
            x: point.x - 5.0,
            y: point.y + 7.0,
        };
        vertex.u = (i + 1) as f32;
        vertex.ia = i as c_int;
        vertex.ib = (i + 4) as c_int;
    }
    s
}

#[test]
fn simplex_surface() {
    unsafe {
        let libs = Libs::load();
        let (cmetric, rmetric) =
            libs.pair::<unsafe extern "C" fn(*mut Simplex) -> f32>(b"c2GJKSimplexMetric\0");
        let (c22f, r22f) = libs.pair::<unsafe extern "C" fn(*mut Simplex)>(b"c22\0");
        let (c23f, r23f) = libs.pair::<unsafe extern "C" fn(*mut Simplex)>(b"c23\0");
        let (cd, rd) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2D\0");
        let (cl, rl) = libs.pair::<unsafe extern "C" fn(*mut Simplex) -> V>(b"c2L\0");
        let (cwitness, rwitness) =
            libs.pair::<unsafe extern "C" fn(*mut Simplex, *mut V, *mut V)>(b"c2Witness\0");
        let (csupport, rsupport) =
            libs.pair::<unsafe extern "C" fn(*const V, c_int, V) -> c_int>(b"c2Support\0");

        for count in [0, 1, 2, 3, 4, -1, c_int::MAX] {
            let mut cs = simplex(&[
                V { x: -2.0, y: 1.0 },
                V { x: 3.0, y: -1.0 },
                V { x: 1.0, y: 4.0 },
            ]);
            cs.count = count;
            let mut rs = cs;
            assert_f(cmetric(&mut cs), rmetric(&mut rs), "metric");
        }

        let segments = [
            [V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }],
            [V { x: -2.0, y: 0.0 }, V { x: -1.0, y: 0.0 }],
            [V { x: -1.0, y: 1.0 }, V { x: 1.0, y: 1.0 }],
        ];
        for (i, points) in segments.into_iter().enumerate() {
            let mut cs = simplex(&points);
            let mut rs = cs;
            c22f(&mut cs);
            r22f(&mut rs);
            assert_eq!(cs.count, [1, 1, 2][i], "c22 fixture {i} branch");
            if i == 1 {
                assert_v(cs.a.p, points[1], "c22 B fixture");
            }
            assert_simplex(cs, rs, &format!("c22 branch {i}"));
        }

        let triangles = [
            [
                V { x: 1.0, y: 0.0 },
                V { x: 2.0, y: 0.0 },
                V { x: 1.0, y: 1.0 },
            ],
            [
                V { x: -2.0, y: 0.0 },
                V { x: -1.0, y: 0.0 },
                V { x: -1.0, y: 1.0 },
            ],
            [
                V { x: 0.0, y: 2.0 },
                V { x: 1.0, y: 1.0 },
                V { x: 0.0, y: 1.0 },
            ],
            [
                V { x: -1.0, y: 1.0 },
                V { x: 1.0, y: 1.0 },
                V { x: 0.0, y: 3.0 },
            ],
            [
                V { x: -3.0, y: 0.0 },
                V { x: -1.0, y: -1.0 },
                V { x: -1.0, y: 1.0 },
            ],
            [
                V { x: 1.0, y: -1.0 },
                V { x: 3.0, y: 0.0 },
                V { x: 1.0, y: 1.0 },
            ],
            [
                V { x: -2.0, y: -1.0 },
                V { x: 2.0, y: -1.0 },
                V { x: 0.0, y: 2.0 },
            ],
        ];
        for (i, points) in triangles.into_iter().enumerate() {
            let mut cs = simplex(&points);
            let mut rs = cs;
            c23f(&mut cs);
            r23f(&mut rs);
            assert_eq!(cs.count, [1, 1, 1, 2, 2, 2, 3][i], "c23 fixture {i} branch");
            let expected_a = [
                points[0], points[1], points[2], points[0], points[1], points[2], points[0],
            ];
            assert_v(
                cs.a.p,
                expected_a[i],
                &format!("c23 fixture {i} first vertex"),
            );
            assert_simplex(cs, rs, &format!("c23 branch {i}"));
        }

        let direction_cases = [
            simplex(&[V { x: 2.0, y: -3.0 }]),
            simplex(&[V { x: 1.0, y: 1.0 }, V { x: 2.0, y: 0.0 }]),
            simplex(&[V { x: 1.0, y: -1.0 }, V { x: 2.0, y: 0.0 }]),
            simplex(&[
                V { x: -1.0, y: -1.0 },
                V { x: 1.0, y: -1.0 },
                V { x: 0.0, y: 1.0 },
            ]),
        ];
        for (i, initial) in direction_cases.into_iter().enumerate() {
            let mut cs = initial;
            let mut rs = initial;
            assert_v(cd(&mut cs), rd(&mut rs), &format!("direction {i}"));
        }

        for count in [1, 2, 3, 0, 4, -1] {
            let mut cs = simplex(&[
                V { x: -2.0, y: 1.0 },
                V { x: 3.0, y: -1.0 },
                V { x: 1.0, y: 4.0 },
            ]);
            cs.count = count;
            cs.div = 6.0;
            cs.a.u = 1.0;
            cs.b.u = 2.0;
            cs.c.u = 3.0;
            let mut rs = cs;
            assert_v(cl(&mut cs), rl(&mut rs), &format!("L {count}"));
            let mut ca = V::default();
            let mut cb = V::default();
            let mut ra = V::default();
            let mut rb = V::default();
            cwitness(&mut cs, &mut ca, &mut cb);
            rwitness(&mut rs, &mut ra, &mut rb);
            assert_v(ca, ra, &format!("witness A {count}"));
            assert_v(cb, rb, &format!("witness B {count}"));
        }

        let mut verts = [V::default(); 64];
        for (i, vertex) in verts.iter_mut().enumerate() {
            *vertex = V {
                x: (i as f32 % 7.0) - 3.0,
                y: (i as f32 % 11.0) - 5.0,
            };
        }
        for count in [-5, 0, 1, 2, 4, 8, 9, 64] {
            for direction in [
                V { x: 1.0, y: 0.0 },
                V { x: 0.0, y: 1.0 },
                V { x: -1.0, y: -1.0 },
                V { x: 0.0, y: 0.0 },
            ] {
                assert_eq!(
                    csupport(verts.as_ptr(), count, direction),
                    rsupport(verts.as_ptr(), count, direction),
                    "support count={count}"
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Shape {
    Circle(Circle),
    Aabb(Aabb),
    Capsule(Capsule),
}

impl Shape {
    fn kind(self) -> c_int {
        match self {
            Self::Circle(_) => 0,
            Self::Aabb(_) => 1,
            Self::Capsule(_) => 2,
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

fn random_shape(rng: &mut Rng, kind: c_int) -> Shape {
    match kind {
        0 => Shape::Circle(Circle {
            p: rng.v(),
            r: rng.positive(),
        }),
        1 => {
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
        2 => Shape::Capsule(Capsule {
            a: rng.v(),
            b: rng.v(),
            r: rng.positive(),
        }),
        _ => unreachable!(),
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

#[test]
fn gjk_surface() {
    unsafe {
        let libs = Libs::load();
        let (cg, rg) = libs.pair::<Gjk>(b"c2GJK\0");
        let mut rng = Rng::new();
        for kind_a in 0..3 {
            for kind_b in 0..3 {
                for case in 0..96 {
                    let a = random_shape(&mut rng, kind_a);
                    let b = random_shape(&mut rng, kind_b);
                    let ax = X {
                        p: rng.v(),
                        r: if case & 1 == 0 {
                            R { c: 1.0, s: 0.0 }
                        } else {
                            R { c: 0.0, s: 1.0 }
                        },
                    };
                    let bx = X {
                        p: rng.v(),
                        r: if case & 2 == 0 {
                            R { c: 1.0, s: 0.0 }
                        } else {
                            R { c: 0.0, s: -1.0 }
                        },
                    };
                    let axp = if case % 3 == 0 { null() } else { &ax };
                    let bxp = if case % 5 == 0 { null() } else { &bx };
                    let use_radius = match case % 3 {
                        0 => 0,
                        1 => 1,
                        _ => -7,
                    };
                    let mut coa = V::default();
                    let mut cob = V::default();
                    let mut roa = V::default();
                    let mut rob = V::default();
                    let mut ci = -1;
                    let mut ri = -1;
                    let mut cc = Cache::default();
                    let mut rc = Cache::default();
                    let cd = cg(
                        a.ptr(),
                        a.kind(),
                        axp,
                        b.ptr(),
                        b.kind(),
                        bxp,
                        &mut coa,
                        &mut cob,
                        use_radius,
                        &mut ci,
                        &mut cc,
                    );
                    let rd = rg(
                        a.ptr(),
                        a.kind(),
                        axp,
                        b.ptr(),
                        b.kind(),
                        bxp,
                        &mut roa,
                        &mut rob,
                        use_radius,
                        &mut ri,
                        &mut rc,
                    );
                    let context = format!("gjk {kind_a}/{kind_b}/{case}");
                    assert_f(cd, rd, &context);
                    assert_v(coa, roa, &format!("{context} outA"));
                    assert_v(cob, rob, &format!("{context} outB"));
                    assert_eq!(ci, ri, "{context} iterations");
                    assert_cache(cc, rc, &context);

                    // Exercise every optional output pointer as null.
                    let cd = cg(
                        a.ptr(),
                        a.kind(),
                        axp,
                        b.ptr(),
                        b.kind(),
                        bxp,
                        null_mut(),
                        null_mut(),
                        use_radius,
                        null_mut(),
                        null_mut(),
                    );
                    let rd = rg(
                        a.ptr(),
                        a.kind(),
                        axp,
                        b.ptr(),
                        b.kind(),
                        bxp,
                        null_mut(),
                        null_mut(),
                        use_radius,
                        null_mut(),
                        null_mut(),
                    );
                    assert_f(cd, rd, &format!("{context} null outputs"));

                    // Reuse each implementation's cache against changed geometry.
                    let changed = random_shape(&mut rng, kind_b);
                    let cd = cg(
                        a.ptr(),
                        a.kind(),
                        axp,
                        changed.ptr(),
                        changed.kind(),
                        bxp,
                        &mut coa,
                        &mut cob,
                        use_radius,
                        &mut ci,
                        &mut cc,
                    );
                    let rd = rg(
                        a.ptr(),
                        a.kind(),
                        axp,
                        changed.ptr(),
                        changed.kind(),
                        bxp,
                        &mut roa,
                        &mut rob,
                        use_radius,
                        &mut ri,
                        &mut rc,
                    );
                    assert_f(cd, rd, &format!("{context} warm cache"));
                    assert_v(coa, roa, &format!("{context} warm outA"));
                    assert_v(cob, rob, &format!("{context} warm outB"));
                    assert_eq!(ci, ri, "{context} warm iterations");
                    assert_cache(cc, rc, &format!("{context} warm cache"));
                }
            }
        }

        let deterministic = [
            (
                Shape::Circle(Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                }),
                Shape::Circle(Circle {
                    p: V { x: 5.0, y: 0.0 },
                    r: 1.0,
                }),
                1,
            ),
            (
                Shape::Circle(Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                }),
                Shape::Circle(Circle {
                    p: V { x: 5.0, y: 0.0 },
                    r: 3.0,
                }),
                1,
            ),
            (
                Shape::Aabb(Aabb {
                    min: V { x: -2.0, y: -2.0 },
                    max: V { x: 2.0, y: 2.0 },
                }),
                Shape::Aabb(Aabb {
                    min: V { x: -1.0, y: -1.0 },
                    max: V { x: 3.0, y: 3.0 },
                }),
                0,
            ),
        ];
        for (i, (a, b, use_radius)) in deterministic.into_iter().enumerate() {
            let mut coa = V::default();
            let mut cob = V::default();
            let mut roa = V::default();
            let mut rob = V::default();
            let cd = cg(
                a.ptr(),
                a.kind(),
                null(),
                b.ptr(),
                b.kind(),
                null(),
                &mut coa,
                &mut cob,
                use_radius,
                null_mut(),
                null_mut(),
            );
            let rd = rg(
                a.ptr(),
                a.kind(),
                null(),
                b.ptr(),
                b.kind(),
                null(),
                &mut roa,
                &mut rob,
                use_radius,
                null_mut(),
                null_mut(),
            );
            assert_f(cd, rd, &format!("deterministic GJK {i}"));
            assert_v(coa, roa, &format!("deterministic GJK {i} outA"));
            assert_v(cob, rob, &format!("deterministic GJK {i} outB"));
        }

        let special = [
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            f32::from_bits(1),
            f32::from_bits(0x7fc0_1234),
            f32::from_bits(0xffc0_5678),
        ];
        for (i, &x) in special.iter().enumerate() {
            for (j, &y) in special.iter().enumerate() {
                let a = Circle {
                    p: V { x, y: 0.0 },
                    r: y,
                };
                let b = Circle {
                    p: V { x: 0.0, y },
                    r: x,
                };
                for use_radius in [0, 1] {
                    let mut coa = V::default();
                    let mut cob = V::default();
                    let mut roa = V::default();
                    let mut rob = V::default();
                    let mut ci = -1;
                    let mut ri = -1;
                    let cd = cg(
                        (&a as *const Circle).cast(),
                        0,
                        null(),
                        (&b as *const Circle).cast(),
                        0,
                        null(),
                        &mut coa,
                        &mut cob,
                        use_radius,
                        &mut ci,
                        null_mut(),
                    );
                    let rd = rg(
                        (&a as *const Circle).cast(),
                        0,
                        null(),
                        (&b as *const Circle).cast(),
                        0,
                        null(),
                        &mut roa,
                        &mut rob,
                        use_radius,
                        &mut ri,
                        null_mut(),
                    );
                    let context = format!("special GJK {i}/{j}/{use_radius}");
                    assert_f(cd, rd, &context);
                    assert_v(coa, roa, &format!("{context} outA"));
                    assert_v(cob, rob, &format!("{context} outB"));
                    assert_eq!(ci, ri, "{context} iterations");
                }
            }
        }
    }
}

#[test]
fn collision_and_wrapper_surface() {
    unsafe {
        let libs = Libs::load();
        let (caa, raa) = libs.pair::<unsafe extern "C" fn(Aabb, Aabb) -> c_int>(b"c2AABBtoAABB\0");
        let (cac, rac) =
            libs.pair::<unsafe extern "C" fn(Aabb, Capsule) -> c_int>(b"c2AABBtoCapsule\0");
        let (cccaps, rccaps) =
            libs.pair::<unsafe extern "C" fn(Capsule, Capsule) -> c_int>(b"c2CapsuletoCapsule\0");
        let (ccc, rcc) =
            libs.pair::<unsafe extern "C" fn(Circle, Circle) -> c_int>(b"c2CircletoCircle\0");
        let (cca, rca) =
            libs.pair::<unsafe extern "C" fn(Circle, Aabb) -> c_int>(b"c2CircletoAABB\0");
        let (cccap, rccap) =
            libs.pair::<unsafe extern "C" fn(Circle, Capsule) -> c_int>(b"c2CircletoCapsule\0");
        let (ccollide, rcollide) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int>(
                b"c2Collided\0",
            );
        let (creverse, rreverse) =
            libs.pair::<unsafe extern "C" fn(f32, f32, f32) -> c_int>(b"reverse_collide\0");

        let mut rng = Rng::new();
        let tangent_circles = (
            Circle {
                p: V { x: 0.0, y: 0.0 },
                r: 2.0,
            },
            Circle {
                p: V { x: 5.0, y: 0.0 },
                r: 3.0,
            },
        );
        assert_eq!(
            ccc(tangent_circles.0, tangent_circles.1),
            rcc(tangent_circles.0, tangent_circles.1)
        );
        let box0 = Aabb {
            min: V { x: -2.0, y: -2.0 },
            max: V { x: 2.0, y: 2.0 },
        };
        let touching_box = Aabb {
            min: V { x: 2.0, y: -1.0 },
            max: V { x: 4.0, y: 1.0 },
        };
        assert_eq!(caa(box0, touching_box), raa(box0, touching_box));
        let touching_capsule = Capsule {
            a: V { x: 3.0, y: -1.0 },
            b: V { x: 3.0, y: 1.0 },
            r: 1.0,
        };
        assert_eq!(cac(box0, touching_capsule), rac(box0, touching_capsule));
        let capsule_left = Capsule {
            a: V { x: -2.0, y: 0.0 },
            b: V { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let capsule_right = Capsule {
            a: V { x: 2.0, y: 0.0 },
            b: V { x: 4.0, y: 0.0 },
            r: 1.0,
        };
        assert_eq!(
            cccaps(capsule_left, capsule_right),
            rccaps(capsule_left, capsule_right)
        );
        let region_capsules = [
            Capsule {
                a: V { x: 0.0, y: 0.0 },
                b: V { x: 10.0, y: 0.0 },
                r: 1.0,
            },
            Capsule {
                a: V { x: -10.0, y: 0.0 },
                b: V { x: 0.0, y: 0.0 },
                r: 1.0,
            },
        ];
        for circle in [
            Circle {
                p: V { x: -2.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 5.0, y: 2.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 12.0, y: 0.0 },
                r: 1.0,
            },
        ] {
            for capsule in region_capsules {
                assert_eq!(cccap(circle, capsule), rccap(circle, capsule));
            }
        }

        let mut reverse_bits_seen = 0;
        for i in 0..512 {
            let sa = random_shape(&mut rng, (i % 3) as c_int);
            let sb = random_shape(&mut rng, ((i / 3) % 3) as c_int);
            let circle_a = match random_shape(&mut rng, 0) {
                Shape::Circle(v) => v,
                _ => unreachable!(),
            };
            let circle_b = match random_shape(&mut rng, 0) {
                Shape::Circle(v) => v,
                _ => unreachable!(),
            };
            let aabb_a = match random_shape(&mut rng, 1) {
                Shape::Aabb(v) => v,
                _ => unreachable!(),
            };
            let aabb_b = match random_shape(&mut rng, 1) {
                Shape::Aabb(v) => v,
                _ => unreachable!(),
            };
            let capsule_a = match random_shape(&mut rng, 2) {
                Shape::Capsule(v) => v,
                _ => unreachable!(),
            };
            let capsule_b = match random_shape(&mut rng, 2) {
                Shape::Capsule(v) => v,
                _ => unreachable!(),
            };
            assert_eq!(caa(aabb_a, aabb_b), raa(aabb_a, aabb_b), "aabb/aabb {i}");
            assert_eq!(
                cac(aabb_a, capsule_a),
                rac(aabb_a, capsule_a),
                "aabb/capsule {i}"
            );
            assert_eq!(
                cccaps(capsule_a, capsule_b),
                rccaps(capsule_a, capsule_b),
                "capsule/capsule {i}"
            );
            assert_eq!(
                ccc(circle_a, circle_b),
                rcc(circle_a, circle_b),
                "circle/circle {i}"
            );
            assert_eq!(
                cca(circle_a, aabb_a),
                rca(circle_a, aabb_a),
                "circle/aabb {i}"
            );
            assert_eq!(
                cccap(circle_a, capsule_a),
                rccap(circle_a, capsule_a),
                "circle/capsule {i}"
            );
            assert_eq!(
                ccollide(sa.ptr(), sa.kind(), sb.ptr(), sb.kind()),
                rcollide(sa.ptr(), sa.kind(), sb.ptr(), sb.kind()),
                "collided {i}"
            );
            let x = rng.f();
            let y = rng.f();
            let radius = rng.positive();
            let c_result = creverse(x, y, radius);
            let r_result = rreverse(x, y, radius);
            assert_eq!(c_result, r_result, "reverse {i}");
            reverse_bits_seen |= c_result;
        }
        assert_eq!(reverse_bits_seen, 0b111, "all reverse_collide output bits");

        let special = [
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            f32::from_bits(1),
            f32::from_bits(0x7fc0_1234),
            f32::from_bits(0xffc0_5678),
        ];
        for (i, &x) in special.iter().enumerate() {
            for (j, &y) in special.iter().enumerate() {
                let circle_a = Circle {
                    p: V { x, y },
                    r: x,
                };
                let circle_b = Circle {
                    p: V { x: y, y: x },
                    r: y,
                };
                let aabb = Aabb {
                    min: V { x, y },
                    max: V { x: y, y: x },
                };
                let capsule = Capsule {
                    a: V { x, y },
                    b: V { x: y, y: x },
                    r: x,
                };
                let context = format!("special collision {i}/{j}");
                assert_eq!(
                    ccc(circle_a, circle_b),
                    rcc(circle_a, circle_b),
                    "{context} cc"
                );
                assert_eq!(cca(circle_a, aabb), rca(circle_a, aabb), "{context} ca");
                assert_eq!(
                    cccap(circle_a, capsule),
                    rccap(circle_a, capsule),
                    "{context} capsule"
                );
                assert_eq!(creverse(x, y, x), rreverse(x, y, x), "{context} reverse");
            }
        }

        // Explicitly cover touching and overlap around a box.
        for circle in [
            Circle {
                p: V { x: 3.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 2.5, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 0.0, y: 0.0 },
                r: 1.0,
            },
            Circle {
                p: V { x: 3.0, y: 3.0 },
                r: 2.0_f32.sqrt(),
            },
        ] {
            assert_eq!(cca(circle, box0), rca(circle, box0));
        }
    }
}

#[test]
fn explicit_error_surface() {
    unsafe {
        let libs = Libs::load();
        let (cproxy, rproxy) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_int, *mut Proxy)>(b"c2MakeProxy\0");
        let (ccollide, rcollide) =
            libs.pair::<unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int>(
                b"c2Collided\0",
            );
        let invalid = [-1, 3, 17, c_int::MIN, c_int::MAX];
        for kind in invalid {
            let sentinel = Proxy {
                radius: f32::from_bits(0x7fc0_1234),
                count: 0x1234_5678,
                verts: [V {
                    x: f32::from_bits(0x7fc0_5678),
                    y: -0.0,
                }; 8],
            };
            let mut cp = sentinel;
            let mut rp = sentinel;
            cproxy(null(), kind, &mut cp);
            rproxy(null(), kind, &mut rp);
            assert_eq!(cp.radius.to_bits(), rp.radius.to_bits());
            assert_eq!(cp.count, rp.count);
            for i in 0..8 {
                assert_v(cp.verts[i], rp.verts[i], "invalid proxy no-op");
            }
            // The default C switch arm does not touch even a null output.
            cproxy(null(), kind, null_mut());
            rproxy(null(), kind, null_mut());

            assert_eq!(ccollide(null(), kind, null(), kind), 0);
            assert_eq!(rcollide(null(), kind, null(), kind), 0);
            for valid_a in 0..3 {
                assert_eq!(ccollide(null(), valid_a, null(), kind), 0);
                assert_eq!(rcollide(null(), valid_a, null(), kind), 0);
            }
        }
    }
}
