#![allow(non_camel_case_types, non_snake_case)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};

const CAPSULE: c_int = 0;
const CIRCLE: c_int = 1;
const AABB: c_int = 2;
const POLY: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2v {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Manifold {
    count: c_int,
    depths: [c_float; 2],
    contact_points: [c2v; 2],
    n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2h {
    n: c2v,
    d: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2GJKCache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Proxy {
    radius: c_float,
    count: c_int,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: c_float,
    count: c_int,
}

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_dir = root.join("../c_src/build");
        let c_path = fs::read_dir(&c_dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", c_dir.display()))
            .map(|entry| entry.unwrap().path())
            .find(|path| path.extension().is_some_and(|ext| ext == "so"))
            .unwrap_or_else(|| panic!("no C shared library in {}", c_dir.display()));
        let rust_path = rust_library_path(root);
        unsafe {
            Self {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("load {}: {e}", c_path.display())),
                rust: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("load {}: {e}", rust_path.display())),
            }
        }
    }

    unsafe fn funcs<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = unsafe { *self.c.get::<T>(name).unwrap() };
        let rust = unsafe { *self.rust.get::<T>(name).unwrap() };
        (c, rust)
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let profile_path = target.join(profile).join("libomni_manifold_lib.so");
    if profile_path.exists() {
        profile_path
    } else {
        target.join("release").join("libomni_manifold_lib.so")
    }
}

#[derive(Clone, Copy)]
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

    fn f32(&mut self) -> f32 {
        (self.next_u32() % 4001) as f32 / 100.0 - 20.0
    }

    fn positive(&mut self) -> f32 {
        (self.next_u32() % 1000 + 1) as f32 / 100.0
    }

    fn vec(&mut self) -> c2v {
        c2v {
            x: self.f32(),
            y: self.f32(),
        }
    }
}

fn filled<T: Copy>(byte: u8) -> T {
    unsafe {
        let mut value = std::mem::MaybeUninit::<T>::uninit();
        std::ptr::write_bytes(
            value.as_mut_ptr().cast::<u8>(),
            byte,
            std::mem::size_of::<T>(),
        );
        value.assume_init()
    }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn assert_bytes<T: std::fmt::Debug>(id: &str, c: &T, rust: &T) {
    assert_eq!(
        bytes(c),
        bytes(rust),
        "{id}: byte mismatch\nC: {c:?}\nRust: {rust:?}"
    );
}

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn identity() -> c2x {
    c2x {
        p: v(0.0, 0.0),
        r: c2r { c: 1.0, s: 0.0 },
    }
}

fn transform(tx: f32, ty: f32, angle: f32) -> c2x {
    c2x {
        p: v(tx, ty),
        r: c2r {
            c: angle.cos(),
            s: angle.sin(),
        },
    }
}

fn box_at(x: f32, y: f32, hx: f32, hy: f32) -> c2AABB {
    c2AABB {
        min: v(x - hx, y - hy),
        max: v(x + hx, y + hy),
    }
}

fn square_poly(h: f32) -> c2Poly {
    let mut poly = c2Poly::default();
    poly.count = 4;
    poly.verts[..4].copy_from_slice(&[v(-h, -h), v(h, -h), v(h, h), v(-h, h)]);
    poly.norms[..4].copy_from_slice(&[v(0.0, -1.0), v(1.0, 0.0), v(0.0, 1.0), v(-1.0, 0.0)]);
    poly
}

fn seeded_manifold() -> c2Manifold {
    filled(0x5a)
}

fn random_circle(rng: &mut Rng) -> c2Circle {
    c2Circle {
        p: rng.vec(),
        r: rng.positive(),
    }
}

fn random_aabb(rng: &mut Rng) -> c2AABB {
    let center = rng.vec();
    box_at(center.x, center.y, rng.positive(), rng.positive())
}

fn random_capsule(rng: &mut Rng) -> c2Capsule {
    c2Capsule {
        a: rng.vec(),
        b: rng.vec(),
        r: rng.positive(),
    }
}

enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn typ(&self) -> c_int {
        match self {
            Self::Circle(_) => CIRCLE,
            Self::Aabb(_) => AABB,
            Self::Capsule(_) => CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Self::Circle(value) => (value as *const c2Circle).cast(),
            Self::Aabb(value) => (value as *const c2AABB).cast(),
            Self::Capsule(value) => (value as *const c2Capsule).cast(),
        }
    }

    fn parts(&self) -> [f32; 5] {
        match self {
            Self::Circle(value) => [value.p.x, value.p.y, value.r, 0.0, 0.0],
            Self::Aabb(value) => [value.min.x, value.min.y, value.max.x, value.max.y, 0.0],
            Self::Capsule(value) => [value.a.x, value.a.y, value.b.x, value.b.y, value.r],
        }
    }

    fn byte_len(&self) -> usize {
        match self {
            Self::Circle(_) => std::mem::size_of::<c2Circle>(),
            Self::Aabb(_) => std::mem::size_of::<c2AABB>(),
            Self::Capsule(_) => std::mem::size_of::<c2Capsule>(),
        }
    }
}

fn random_shape(rng: &mut Rng, typ: c_int) -> Shape {
    match typ {
        CIRCLE => Shape::Circle(random_circle(rng)),
        AABB => Shape::Aabb(random_aabb(rng)),
        CAPSULE => Shape::Capsule(random_capsule(rng)),
        _ => unreachable!(),
    }
}

fn random_collision_pair(rng: &mut Rng, ta: c_int, tb: c_int) -> (Shape, Shape) {
    if (ta == AABB && tb == CAPSULE) || (ta == CAPSULE && tb == AABB) {
        let radius = rng.positive();
        let aabb = Shape::Aabb(box_at(0.0, 0.0, 2.0, 2.0));
        let capsule = Shape::Capsule(c2Capsule {
            a: v(-1.0, 2.0 + radius + rng.positive()),
            b: v(1.0, 2.0 + radius + rng.positive()),
            r: radius,
        });
        if ta == AABB {
            (aabb, capsule)
        } else {
            (capsule, aabb)
        }
    } else {
        (random_shape(rng, ta), random_shape(rng, tb))
    }
}

fn random_sv(rng: &mut Rng) -> c2sv {
    c2sv {
        sA: rng.vec(),
        sB: rng.vec(),
        p: rng.vec(),
        u: rng.positive(),
        iA: (rng.next_u32() % 4) as c_int,
        iB: (rng.next_u32() % 4) as c_int,
    }
}

fn c23_region(s: &c2Simplex) -> usize {
    let dot = |a: c2v, b: c2v| a.x * b.x + a.y * b.y;
    let sub = |a: c2v, b: c2v| v(a.x - b.x, a.y - b.y);
    let det = |a: c2v, b: c2v| a.x * b.y - a.y * b.x;
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

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

#[test]
fn low_level_vector_and_proxy_surface_c001_c030() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0102_0304_0506_0708);
    unsafe {
        type VV = unsafe extern "C" fn(c2v, c2v) -> c2v;
        type VS = unsafe extern "C" fn(c2v, f32) -> c2v;
        type VF = unsafe extern "C" fn(c2v) -> f32;
        type V1 = unsafe extern "C" fn(c2v) -> c2v;
        type Dot = unsafe extern "C" fn(c2v, c2v) -> f32;
        type VCtor = unsafe extern "C" fn(f32, f32) -> c2v;
        let (c_v, r_v) = libs.funcs::<VCtor>(b"c2V\0");
        let (c_mulvs, r_mulvs) = libs.funcs::<VS>(b"c2Mulvs\0");
        let (c_max, r_max) = libs.funcs::<VV>(b"c2Maxv\0");
        let (c_min, r_min) = libs.funcs::<VV>(b"c2Minv\0");
        let (c_sub, r_sub) = libs.funcs::<VV>(b"c2Sub\0");
        let (c_dot, r_dot) = libs.funcs::<Dot>(b"c2Dot\0");
        let (c_len, r_len) = libs.funcs::<VF>(b"c2Len\0");
        let (c_det, r_det) = libs.funcs::<Dot>(b"c2Det2\0");
        let (c_add, r_add) = libs.funcs::<VV>(b"c2Add\0");
        let (c_neg, r_neg) = libs.funcs::<V1>(b"c2Neg\0");
        let (c_ccw, r_ccw) = libs.funcs::<V1>(b"c2CCW90\0");
        let (c_skew, r_skew) = libs.funcs::<V1>(b"c2Skew\0");
        let (c_abs, r_abs) = libs.funcs::<V1>(b"c2Absv\0");
        let (c_norm, r_norm) = libs.funcs::<V1>(b"c2Norm\0");
        let (c_div, r_div) = libs.funcs::<VS>(b"c2Div\0");

        for i in 0..256 {
            let a = rng.vec();
            let b = rng.vec();
            let scalar = match i % 3 {
                0 => -rng.positive(),
                1 => 0.0,
                _ => rng.positive(),
            };
            assert_bytes("C001", &c_v(a.x, a.y), &r_v(a.x, a.y));
            assert_bytes("C002", &c_mulvs(a, scalar), &r_mulvs(a, scalar));
            assert_bytes("C003", &c_max(a, b), &r_max(a, b));
            assert_bytes("C004", &c_min(a, b), &r_min(a, b));
            assert_bytes("C006", &c_sub(a, b), &r_sub(a, b));
            assert_bytes("C007", &c_dot(a, b), &r_dot(a, b));
            assert_bytes("C015", &c_len(a), &r_len(a));
            assert_bytes("C016", &c_det(a, b), &r_det(a, b));
            assert_bytes("C022", &c_add(a, b), &r_add(a, b));
            assert_bytes("C030-neg", &c_neg(a), &r_neg(a));
            assert_bytes("C030-ccw", &c_ccw(a), &r_ccw(a));
            assert_bytes("C030-skew", &c_skew(a), &r_skew(a));
            assert_bytes("C030-abs", &c_abs(a), &r_abs(a));
            if a.x != 0.0 || a.y != 0.0 {
                assert_bytes("C028", &c_norm(a), &r_norm(a));
            }
            if scalar != 0.0 {
                assert_bytes("C026", &c_div(a, scalar), &r_div(a, scalar));
            }
        }
        for &(x, y) in &[
            (0.0, -0.0),
            (-0.0, 0.0),
            (f32::MAX, -f32::MAX),
            (f32::MIN_POSITIVE, -f32::MIN_POSITIVE),
        ] {
            assert_bytes("C001-special", &c_v(x, y), &r_v(x, y));
        }
        for &zero in &[0.0, -0.0] {
            assert_bytes(
                "C027",
                &c_div(v(1.0, -1.0), zero),
                &r_div(v(1.0, -1.0), zero),
            );
        }
        assert_bytes("C029", &c_norm(v(0.0, 0.0)), &r_norm(v(0.0, 0.0)));

        type Clamp = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
        let (c_clamp, r_clamp) = libs.funcs::<Clamp>(b"c2Clampv\0");
        let states = [-2.0, 0.5, 3.0];
        for _ in 0..64 {
            let shift = rng.f32();
            for &x in &states {
                for &y in &states {
                    let a = v(x + shift, y + shift);
                    let lo = v(-1.0 + shift, -1.0 + shift);
                    let hi = v(2.0 + shift, 2.0 + shift);
                    assert_bytes("C005", &c_clamp(a, lo, hi), &r_clamp(a, lo, hi));
                }
            }
        }

        type Dist = unsafe extern "C" fn(c2h, c2v) -> f32;
        let (c_dist, r_dist) = libs.funcs::<Dist>(b"c2Dist\0");
        for _ in 0..128 {
            let h = c2h {
                n: rng.vec(),
                d: rng.f32(),
            };
            let p = rng.vec();
            assert_bytes("C008", &c_dist(h, p), &r_dist(h, p));
        }

        type Plane = unsafe extern "C" fn(*const c2Poly, c_int) -> c2h;
        let (c_plane, r_plane) = libs.funcs::<Plane>(b"c2PlaneAt\0");
        for _ in 0..64 {
            let mut p = square_poly(rng.positive());
            p.verts[0].x += rng.f32();
            for &index in &[0, p.count - 1] {
                assert_bytes("C009", &c_plane(&p, index), &r_plane(&p, index));
            }
        }

        type RotId = unsafe extern "C" fn() -> c2r;
        type XId = unsafe extern "C" fn() -> c2x;
        let (c_ri, r_ri) = libs.funcs::<RotId>(b"c2RotIdentity\0");
        let (c_xi, r_xi) = libs.funcs::<XId>(b"c2xIdentity\0");
        for _ in 0..64 {
            assert_bytes("C010-rot", &c_ri(), &r_ri());
            assert_bytes("C010-x", &c_xi(), &r_xi());
        }

        type BB = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
        let (c_bb, r_bb) = libs.funcs::<BB>(b"c2BBVerts\0");
        for i in 0..192 {
            let mut bb = random_aabb(&mut rng);
            if i % 3 == 1 {
                bb.max = bb.min;
            } else if i % 3 == 2 {
                std::mem::swap(&mut bb.min, &mut bb.max);
            }
            let mut co = [filled::<c2v>(0xa5); 4];
            let mut ro = co;
            c_bb(co.as_mut_ptr(), &mut bb);
            r_bb(ro.as_mut_ptr(), &mut bb);
            assert_bytes("C011", &co, &ro);
        }

        type Proxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
        let (c_proxy, r_proxy) = libs.funcs::<Proxy>(b"c2MakeProxy\0");
        for _ in 0..128 {
            let circle = random_circle(&mut rng);
            let aabb = random_aabb(&mut rng);
            let capsule = random_capsule(&mut rng);
            for (id, typ, ptr) in [
                ("C012", CIRCLE, (&circle as *const c2Circle).cast()),
                ("C013", AABB, (&aabb as *const c2AABB).cast()),
                ("C014", CAPSULE, (&capsule as *const c2Capsule).cast()),
            ] {
                let mut cp = filled::<c2Proxy>(0x3c);
                let mut rp = cp;
                c_proxy(ptr, typ, &mut cp);
                r_proxy(ptr, typ, &mut rp);
                assert_bytes(id, &cp, &rp);
            }
        }

        type RV = unsafe extern "C" fn(c2r, c2v) -> c2v;
        type XV = unsafe extern "C" fn(c2x, c2v) -> c2v;
        let (c_rv, r_rv) = libs.funcs::<RV>(b"c2Mulrv\0");
        let (c_rvt, r_rvt) = libs.funcs::<RV>(b"c2MulrvT\0");
        let (c_xv, r_xv) = libs.funcs::<XV>(b"c2Mulxv\0");
        let (c_xvt, r_xvt) = libs.funcs::<XV>(b"c2MulxvT\0");
        for i in 0..128 {
            let x = if i == 0 {
                identity()
            } else {
                transform(rng.f32(), rng.f32(), rng.f32())
            };
            let point = rng.vec();
            assert_bytes("C020", &c_rv(x.r, point), &r_rv(x.r, point));
            assert_bytes("C021", &c_rvt(x.r, point), &r_rvt(x.r, point));
            assert_bytes("C023", &c_xv(x, point), &r_xv(x, point));
            assert_bytes("C024", &c_xvt(x, point), &r_xvt(x, point));
        }

        type Intersect = unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v;
        let (c_intersect, r_intersect) = libs.funcs::<Intersect>(b"c2Intersect\0");
        for i in 0..192 {
            let a = rng.vec();
            let b = rng.vec();
            let (da, db) = match i % 3 {
                0 => (rng.positive(), -rng.positive()),
                1 => (0.0, rng.positive()),
                _ => (-rng.positive(), 0.0),
            };
            assert_bytes(
                "C025",
                &c_intersect(a, b, da, db),
                &r_intersect(a, b, da, db),
            );
        }
    }
}

#[test]
fn simplex_surface_c031_c053() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x3141_5926_5358_9793);
    unsafe {
        type SimplexVoid = unsafe extern "C" fn(*mut c2Simplex);
        type SimplexFloat = unsafe extern "C" fn(*mut c2Simplex) -> f32;
        type SimplexVec = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        type Witness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
        type Support = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
        let (c_22, r_22) = libs.funcs::<SimplexVoid>(b"c22\0");
        let (c_23, r_23) = libs.funcs::<SimplexVoid>(b"c23\0");
        let (c_metric, r_metric) = libs.funcs::<SimplexFloat>(b"c2GJKSimplexMetric\0");
        let (c_d, r_d) = libs.funcs::<SimplexVec>(b"c2D\0");
        let (c_witness, r_witness) = libs.funcs::<Witness>(b"c2Witness\0");
        let (c_l, r_l) = libs.funcs::<SimplexVec>(b"c2L\0");
        let (c_support, r_support) = libs.funcs::<Support>(b"c2Support\0");

        for branch in 0..3 {
            for _ in 0..128 {
                let shift = rng.vec();
                let scale = rng.positive();
                let (a, b) = match branch {
                    0 => (shift, v(shift.x + scale, shift.y)),
                    1 => (v(shift.x + scale, shift.y), shift),
                    _ => (v(shift.x - scale, shift.y), v(shift.x + scale, shift.y)),
                };
                let mut cs = filled::<c2Simplex>(0x33);
                cs.a.p = a;
                cs.b.p = b;
                cs.count = 2;
                let mut rs = cs;
                c_22(&mut cs);
                r_22(&mut rs);
                assert_bytes(&format!("C{:03}", 31 + branch), &cs, &rs);
            }
        }

        let mut seen = [0usize; 7];
        for _ in 0..1_000_000 {
            let mut s = filled::<c2Simplex>(0x44);
            s.a = random_sv(&mut rng);
            s.b = random_sv(&mut rng);
            s.c = random_sv(&mut rng);
            // Integer-grid points avoid nearly equal predicates while retaining
            // all Voronoi regions.
            s.a.p = v(
                (rng.next_u32() % 21) as f32 - 10.0,
                (rng.next_u32() % 21) as f32 - 10.0,
            );
            s.b.p = v(
                (rng.next_u32() % 21) as f32 - 10.0,
                (rng.next_u32() % 21) as f32 - 10.0,
            );
            s.c.p = v(
                (rng.next_u32() % 21) as f32 - 10.0,
                (rng.next_u32() % 21) as f32 - 10.0,
            );
            s.count = 3;
            let region = c23_region(&s);
            if seen[region] < 128 {
                let mut cs = s;
                let mut rs = s;
                c_23(&mut cs);
                r_23(&mut rs);
                assert_bytes(&format!("C{:03}", 34 + region), &cs, &rs);
                seen[region] += 1;
            }
            if seen.iter().all(|count| *count >= 128) {
                break;
            }
        }
        assert_eq!(seen, [128; 7], "C034-C040 branch coverage");

        for &count in &[-1, 0, 1, 2, 3, 4] {
            for _ in 0..128 {
                let mut cs = filled::<c2Simplex>(0x22);
                cs.a = random_sv(&mut rng);
                cs.b = random_sv(&mut rng);
                cs.c = random_sv(&mut rng);
                cs.div = rng.positive();
                cs.count = count;
                let mut rs = cs;
                let id = match count {
                    2 => "C018",
                    3 => "C019",
                    _ => "C017",
                };
                assert_bytes(id, &c_metric(&mut cs), &r_metric(&mut rs));
            }
        }

        for _ in 0..128 {
            let mut cs = filled::<c2Simplex>(0x19);
            cs.a = random_sv(&mut rng);
            cs.a.p = rng.vec();
            cs.count = 1;
            let mut rs = cs;
            assert_bytes("C041", &c_d(&mut cs), &r_d(&mut rs));
        }
        for side in 0..2 {
            for _ in 0..128 {
                let length = rng.positive();
                let y = rng.positive() * if side == 0 { 1.0 } else { -1.0 };
                let mut cs = filled::<c2Simplex>(0x20);
                cs.a.p = v(-length, y);
                cs.b.p = v(length, y);
                cs.count = 2;
                let mut rs = cs;
                assert_bytes(
                    if side == 0 { "C042" } else { "C043" },
                    &c_d(&mut cs),
                    &r_d(&mut rs),
                );
            }
        }
        for &count in &[0, 3, 4] {
            for _ in 0..64 {
                let mut cs = filled::<c2Simplex>(0x21);
                cs.count = count;
                let mut rs = cs;
                assert_bytes("C044", &c_d(&mut cs), &r_d(&mut rs));
            }
        }

        for _ in 0..128 {
            let verts = [rng.vec()];
            let direction = rng.vec();
            assert_bytes(
                "C045",
                &c_support(verts.as_ptr(), 1, direction),
                &r_support(verts.as_ptr(), 1, direction),
            );
        }
        for i in 0..256 {
            let mut verts = [c2v::default(); 8];
            for point in &mut verts {
                *point = rng.vec();
            }
            let direction = if i % 2 == 0 {
                rng.vec()
            } else {
                verts[1] = verts[0];
                v(1.0, 0.0)
            };
            assert_bytes(
                "C046",
                &c_support(verts.as_ptr(), verts.len() as c_int, direction),
                &r_support(verts.as_ptr(), verts.len() as c_int, direction),
            );
        }

        for &count in &[0, 1, 2, 3, 4] {
            for _ in 0..128 {
                let mut cs = filled::<c2Simplex>(0x61);
                cs.a = random_sv(&mut rng);
                cs.b = random_sv(&mut rng);
                cs.c = random_sv(&mut rng);
                cs.div = rng.positive();
                cs.count = count;
                let mut rs = cs;
                let mut ca = filled::<c2v>(0x91);
                let mut cb = filled::<c2v>(0x92);
                let mut ra = ca;
                let mut rb = cb;
                c_witness(&mut cs, &mut ca, &mut cb);
                r_witness(&mut rs, &mut ra, &mut rb);
                let id = match count {
                    1 => "C047",
                    2 => "C048",
                    3 => "C049",
                    _ => "C050",
                };
                assert_bytes(id, &(ca, cb), &(ra, rb));
            }
        }

        for &count in &[0, 1, 2, 3, 4] {
            for _ in 0..128 {
                let mut cs = filled::<c2Simplex>(0x71);
                cs.a = random_sv(&mut rng);
                cs.b = random_sv(&mut rng);
                cs.div = rng.positive();
                cs.count = count;
                let mut rs = cs;
                let id = match count {
                    1 => "C051",
                    2 => "C052",
                    _ => "C053",
                };
                assert_bytes(id, &c_l(&mut cs), &r_l(&mut rs));
            }
        }
    }
}

#[test]
fn gjk_surface_c054_c068() {
    type Gjk = unsafe extern "C" fn(
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

    let libs = Libs::load();
    let mut rng = Rng::new(0x2718_2818_2845_9045);
    unsafe {
        let (c_gjk, r_gjk) = libs.funcs::<Gjk>(b"c2GJK\0");
        let types = [CIRCLE, AABB, CAPSULE];
        for (pair_index, (ta, tb)) in types
            .iter()
            .flat_map(|a| types.iter().map(move |b| (*a, *b)))
            .enumerate()
        {
            for _ in 0..128 {
                let a = random_shape(&mut rng, ta);
                let b = random_shape(&mut rng, tb);
                let mut ca = filled::<c2v>(0xa1);
                let mut cb = filled::<c2v>(0xb2);
                let mut ra = ca;
                let mut rb = cb;
                let mut ci = -77;
                let mut ri = ci;
                let cd = c_gjk(
                    a.ptr(),
                    a.typ(),
                    std::ptr::null(),
                    b.ptr(),
                    b.typ(),
                    std::ptr::null(),
                    &mut ca,
                    &mut cb,
                    0,
                    &mut ci,
                    std::ptr::null_mut(),
                );
                let rd = r_gjk(
                    a.ptr(),
                    a.typ(),
                    std::ptr::null(),
                    b.ptr(),
                    b.typ(),
                    std::ptr::null(),
                    &mut ra,
                    &mut rb,
                    0,
                    &mut ri,
                    std::ptr::null_mut(),
                );
                let id = format!("C{:03}", 54 + pair_index);
                assert_bytes(&id, &(cd, ca, cb, ci), &(rd, ra, rb, ri));
            }
        }

        for mode in 0..4 {
            for _ in 0..128 {
                let a = random_shape(&mut rng, CIRCLE);
                let b = random_shape(&mut rng, AABB);
                let ax = transform(rng.f32(), rng.f32(), rng.f32());
                let bx = transform(rng.f32(), rng.f32(), rng.f32());
                let axp = if mode & 1 == 0 { std::ptr::null() } else { &ax };
                let bxp = if mode & 2 == 0 { std::ptr::null() } else { &bx };
                let mut ca = filled::<c2v>(0x41);
                let mut cb = filled::<c2v>(0x42);
                let mut ra = ca;
                let mut rb = cb;
                let cd = c_gjk(
                    a.ptr(),
                    a.typ(),
                    axp,
                    b.ptr(),
                    b.typ(),
                    bxp,
                    &mut ca,
                    &mut cb,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                let rd = r_gjk(
                    a.ptr(),
                    a.typ(),
                    axp,
                    b.ptr(),
                    b.typ(),
                    bxp,
                    &mut ra,
                    &mut rb,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                assert_bytes("C063", &(cd, ca, cb), &(rd, ra, rb));
            }
        }

        for overlap in 0..2 {
            for _ in 0..128 {
                let radius_a = rng.positive();
                let radius_b = rng.positive();
                let gap = if overlap == 0 {
                    radius_a + radius_b + rng.positive()
                } else {
                    (radius_a + radius_b) * 0.25
                };
                let a = c2Circle {
                    p: v(0.0, 0.0),
                    r: radius_a,
                };
                let b = c2Circle {
                    p: v(gap, 0.0),
                    r: radius_b,
                };
                for use_radius in [0, 1, -1] {
                    let mut ca = filled::<c2v>(0x51);
                    let mut cb = filled::<c2v>(0x52);
                    let mut ra = ca;
                    let mut rb = cb;
                    let cd = c_gjk(
                        (&a as *const c2Circle).cast(),
                        CIRCLE,
                        std::ptr::null(),
                        (&b as *const c2Circle).cast(),
                        CIRCLE,
                        std::ptr::null(),
                        &mut ca,
                        &mut cb,
                        use_radius,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    let rd = r_gjk(
                        (&a as *const c2Circle).cast(),
                        CIRCLE,
                        std::ptr::null(),
                        (&b as *const c2Circle).cast(),
                        CIRCLE,
                        std::ptr::null(),
                        &mut ra,
                        &mut rb,
                        use_radius,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    assert_bytes(
                        if overlap == 0 { "C064" } else { "C065" },
                        &(cd, ca, cb),
                        &(rd, ra, rb),
                    );
                }
            }
        }

        for mode in 0..4 {
            for _ in 0..128 {
                let a = random_circle(&mut rng);
                let b = random_capsule(&mut rng);
                let mut ca = filled::<c2v>(0x61);
                let mut cb = filled::<c2v>(0x62);
                let mut ra = ca;
                let mut rb = cb;
                let cap = if mode & 1 == 0 {
                    std::ptr::null_mut()
                } else {
                    &mut ca
                };
                let cbp = if mode & 2 == 0 {
                    std::ptr::null_mut()
                } else {
                    &mut cb
                };
                let rap = if mode & 1 == 0 {
                    std::ptr::null_mut()
                } else {
                    &mut ra
                };
                let rbp = if mode & 2 == 0 {
                    std::ptr::null_mut()
                } else {
                    &mut rb
                };
                let cd = c_gjk(
                    (&a as *const c2Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    (&b as *const c2Capsule).cast(),
                    CAPSULE,
                    std::ptr::null(),
                    cap,
                    cbp,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                let rd = r_gjk(
                    (&a as *const c2Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    (&b as *const c2Capsule).cast(),
                    CAPSULE,
                    std::ptr::null(),
                    rap,
                    rbp,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                assert_bytes("C066", &(cd, ca, cb), &(rd, ra, rb));
            }
        }

        for with_iterations in [false, true] {
            for _ in 0..128 {
                let a = random_aabb(&mut rng);
                let b = random_capsule(&mut rng);
                let mut ci = -99;
                let mut ri = ci;
                let cip = if with_iterations {
                    &mut ci
                } else {
                    std::ptr::null_mut()
                };
                let rip = if with_iterations {
                    &mut ri
                } else {
                    std::ptr::null_mut()
                };
                let cd = c_gjk(
                    (&a as *const c2AABB).cast(),
                    AABB,
                    std::ptr::null(),
                    (&b as *const c2Capsule).cast(),
                    CAPSULE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                    cip,
                    std::ptr::null_mut(),
                );
                let rd = r_gjk(
                    (&a as *const c2AABB).cast(),
                    AABB,
                    std::ptr::null(),
                    (&b as *const c2Capsule).cast(),
                    CAPSULE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                    rip,
                    std::ptr::null_mut(),
                );
                assert_bytes("C067", &(cd, ci), &(rd, ri));
            }
        }

        for _ in 0..128 {
            let a = random_circle(&mut rng);
            let b = random_aabb(&mut rng);
            let mut cc = c2GJKCache::default();
            let mut rc = cc;
            for pass in 0..2 {
                let mut ca = filled::<c2v>(0x81);
                let mut cb = filled::<c2v>(0x82);
                let mut ra = ca;
                let mut rb = cb;
                let cd = c_gjk(
                    (&a as *const c2Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    (&b as *const c2AABB).cast(),
                    AABB,
                    std::ptr::null(),
                    &mut ca,
                    &mut cb,
                    0,
                    std::ptr::null_mut(),
                    &mut cc,
                );
                let rd = r_gjk(
                    (&a as *const c2Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    (&b as *const c2AABB).cast(),
                    AABB,
                    std::ptr::null(),
                    &mut ra,
                    &mut rb,
                    0,
                    std::ptr::null_mut(),
                    &mut rc,
                );
                assert_bytes(
                    if pass == 0 { "C068-cold" } else { "C068-warm" },
                    &(cd, ca, cb, cc),
                    &(rd, ra, rb, rc),
                );
            }
        }
    }
}

#[test]
fn manifold_surface_c069_c091() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x1618_0339_8874_9894);
    unsafe {
        type CC = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
        type CB = unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold);
        type CK = unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold);
        type BB = unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold);
        type Norms = unsafe extern "C" fn(*mut c2v, *mut c2v, c_int);
        type BK = unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold);
        type KK = unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold);
        let (c_cc, r_cc) = libs.funcs::<CC>(b"c2CircletoCircleManifold\0");
        let (c_cb, r_cb) = libs.funcs::<CB>(b"c2CircletoAABBManifold\0");
        let (c_ck, r_ck) = libs.funcs::<CK>(b"c2CircletoCapsuleManifold\0");
        let (c_bb, r_bb) = libs.funcs::<BB>(b"c2AABBtoAABBManifold\0");
        let (c_norms, r_norms) = libs.funcs::<Norms>(b"c2Norms\0");
        let (c_bk, r_bk) = libs.funcs::<BK>(b"c2AABBtoCapsuleManifold\0");
        let (c_kk, r_kk) = libs.funcs::<KK>(b"c2CapsuletoCapsuleManifold\0");

        for case in 0..3 {
            for _ in 0..128 {
                let center = rng.vec();
                let ra = rng.positive();
                let rb = rng.positive();
                let distance = match case {
                    0 => {
                        ra + rb
                            + if rng.next_u32() & 1 == 0 {
                                0.0
                            } else {
                                rng.positive()
                            }
                    }
                    1 => (ra + rb) * 0.5,
                    _ => 0.0,
                };
                let a = c2Circle { p: center, r: ra };
                let b = c2Circle {
                    p: v(center.x + distance, center.y),
                    r: rb,
                };
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_cc(a, b, &mut cm);
                r_cc(a, b, &mut rm);
                assert_bytes(
                    match case {
                        0 => "C069",
                        1 => "C070",
                        _ => "C071",
                    },
                    &cm,
                    &rm,
                );
            }
        }

        for case in 0..4 {
            for _ in 0..128 {
                let bx = rng.f32();
                let by = rng.f32();
                let hx = rng.positive() + 1.0;
                let hy = rng.positive() + 1.0;
                let b = box_at(bx, by, hx, hy);
                let radius = rng.positive();
                let p = match case {
                    0 => v(b.max.x + radius + rng.positive(), by),
                    1 => v(b.max.x + radius * 0.5, by),
                    2 => v(bx + hx * 0.75, by),
                    _ => v(bx, by + hy * 0.75),
                };
                let a = c2Circle { p, r: radius };
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_cb(a, b, &mut cm);
                r_cb(a, b, &mut rm);
                assert_bytes(
                    match case {
                        0 => "C072",
                        1 => "C073",
                        2 => "C074",
                        _ => "C075",
                    },
                    &cm,
                    &rm,
                );
            }
        }

        for case in 0..3 {
            for _ in 0..128 {
                let radius_a = rng.positive();
                let radius_b = rng.positive();
                let capsule = c2Capsule {
                    a: v(-2.0, 0.0),
                    b: v(2.0, 0.0),
                    r: radius_b,
                };
                let center = match case {
                    0 => v(0.0, radius_a + radius_b + rng.positive()),
                    1 => v(0.0, (radius_a + radius_b) * 0.5),
                    _ => v(0.0, 0.0),
                };
                let circle = c2Circle {
                    p: center,
                    r: radius_a,
                };
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_ck(circle, capsule, &mut cm);
                r_ck(circle, capsule, &mut rm);
                assert_bytes(
                    match case {
                        0 => "C076",
                        1 => "C077",
                        _ => "C078",
                    },
                    &cm,
                    &rm,
                );
            }
        }

        for case in 0..4 {
            for _ in 0..128 {
                let a = box_at(0.0, 0.0, rng.positive() + 1.0, rng.positive() + 1.0);
                let b = match case {
                    0 => box_at(a.max.x + 4.0, 0.0, 1.0, 1.0),
                    1 => box_at(0.0, a.max.y + 4.0, 1.0, 1.0),
                    2 => box_at(
                        if rng.next_u32() & 1 == 0 { 1.5 } else { -1.5 },
                        0.0,
                        1.0,
                        10.0,
                    ),
                    _ => box_at(
                        0.0,
                        if rng.next_u32() & 1 == 0 { 1.5 } else { -1.5 },
                        10.0,
                        1.0,
                    ),
                };
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_bb(a, b, &mut cm);
                r_bb(a, b, &mut rm);
                assert_bytes(
                    match case {
                        0 => "C079",
                        1 => "C080",
                        2 => "C081",
                        _ => "C082",
                    },
                    &cm,
                    &rm,
                );
            }
        }

        for &count in &[1, 2, 8] {
            for _ in 0..128 {
                let mut cv = [c2v::default(); 8];
                for point in &mut cv {
                    *point = rng.vec();
                }
                if count == 1 {
                    cv[0] = v(0.0, 0.0);
                }
                let mut rv = cv;
                let mut cn = [filled::<c2v>(0x93); 8];
                let mut rn = cn;
                c_norms(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                r_norms(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
                assert_bytes("C089", &cn, &rn);
            }
        }

        for case in 0..3 {
            for _ in 0..128 {
                let aabb = box_at(0.0, 0.0, 2.0, 2.0);
                let radius = rng.positive();
                let y = match case {
                    0 => 2.0 + radius + rng.positive(),
                    1 => 2.0 + radius * 0.5,
                    _ => 0.0,
                };
                let capsule = c2Capsule {
                    a: v(-1.0, y),
                    b: v(1.0, y),
                    r: radius,
                };
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_bk(aabb, capsule, &mut cm);
                r_bk(aabb, capsule, &mut rm);
                assert_bytes("C090", &cm, &rm);
            }
        }

        for case in 0..3 {
            for _ in 0..128 {
                let ra = rng.positive();
                let rb = rng.positive();
                let a = c2Capsule {
                    a: v(-2.0, 0.0),
                    b: v(2.0, 0.0),
                    r: ra,
                };
                let y = match case {
                    0 => ra + rb + rng.positive(),
                    1 => (ra + rb) * 0.5,
                    _ => 0.0,
                };
                let b = c2Capsule {
                    a: v(-2.0, y),
                    b: v(2.0, y),
                    r: rb,
                };
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_kk(a, b, &mut cm);
                r_kk(a, b, &mut rm);
                assert_bytes("C091", &cm, &rm);
            }
        }
    }
}

#[test]
fn composed_surface_c092_c112() {
    type Collide =
        unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
    type Parts = unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void;
    type Omni = unsafe extern "C" fn(
        *mut c2Manifold,
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

    let libs = Libs::load();
    let mut rng = Rng::new(0x1414_2135_6237_3095);
    unsafe {
        let (c_collide, r_collide) = libs.funcs::<Collide>(b"c2Collide\0");
        let (c_parts, r_parts) = libs.funcs::<Parts>(b"ptr_from_parts\0");
        let (c_omni, r_omni) = libs.funcs::<Omni>(b"omni_manifold\0");
        let types = [CIRCLE, AABB, CAPSULE];

        for (pair_index, (ta, tb)) in types
            .iter()
            .flat_map(|a| types.iter().map(move |b| (*a, *b)))
            .enumerate()
        {
            for _ in 0..128 {
                let (a, b) = random_collision_pair(&mut rng, ta, tb);
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_collide(a.ptr(), a.typ(), b.ptr(), b.typ(), &mut cm);
                r_collide(a.ptr(), a.typ(), b.ptr(), b.typ(), &mut rm);
                assert_bytes(&format!("C{:03}", 92 + pair_index), &cm, &rm);
            }
        }

        for (index, typ) in types.iter().copied().enumerate() {
            for _ in 0..128 {
                let shape = random_shape(&mut rng, typ);
                let p = shape.parts();
                let cp = c_parts(typ, p[0], p[1], p[2], p[3], p[4]);
                let rp = r_parts(typ, p[0], p[1], p[2], p[3], p[4]);
                assert!(!cp.is_null() && !rp.is_null());
                let cb = std::slice::from_raw_parts(cp.cast::<u8>(), shape.byte_len());
                let rb = std::slice::from_raw_parts(rp.cast::<u8>(), shape.byte_len());
                assert_eq!(cb, rb, "C{:03}: allocated layout", 101 + index);
                free(cp);
                free(rp);
            }
        }

        for (pair_index, (ta, tb)) in types
            .iter()
            .flat_map(|a| types.iter().map(move |b| (*a, *b)))
            .enumerate()
        {
            for _ in 0..128 {
                let (a, b) = random_collision_pair(&mut rng, ta, tb);
                let ap = a.parts();
                let bp = b.parts();
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_omni(
                    &mut cm, ta, ap[0], ap[1], ap[2], ap[3], ap[4], tb, bp[0], bp[1], bp[2], bp[3],
                    bp[4],
                );
                r_omni(
                    &mut rm, ta, ap[0], ap[1], ap[2], ap[3], ap[4], tb, bp[0], bp[1], bp[2], bp[3],
                    bp[4],
                );
                assert_bytes(&format!("C{:03}", 104 + pair_index), &cm, &rm);
            }
        }
    }
}

#[test]
fn error_surface_e01_e16() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xdead_beef_cafe_f00d);
    unsafe {
        type CC = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
        type CB = unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold);
        type CK = unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold);
        type BB = unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold);
        type BK = unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold);
        type KK = unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold);
        type Collide =
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
        type Proxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
        type Norms = unsafe extern "C" fn(*mut c2v, *mut c2v, c_int);
        type Support = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
        let (c_cc, r_cc) = libs.funcs::<CC>(b"c2CircletoCircleManifold\0");
        let (c_cb, r_cb) = libs.funcs::<CB>(b"c2CircletoAABBManifold\0");
        let (c_ck, r_ck) = libs.funcs::<CK>(b"c2CircletoCapsuleManifold\0");
        let (c_bb, r_bb) = libs.funcs::<BB>(b"c2AABBtoAABBManifold\0");
        let (c_bk, r_bk) = libs.funcs::<BK>(b"c2AABBtoCapsuleManifold\0");
        let (c_kk, r_kk) = libs.funcs::<KK>(b"c2CapsuletoCapsuleManifold\0");
        let (c_collide, r_collide) = libs.funcs::<Collide>(b"c2Collide\0");
        let (c_proxy, r_proxy) = libs.funcs::<Proxy>(b"c2MakeProxy\0");
        let (c_norms, r_norms) = libs.funcs::<Norms>(b"c2Norms\0");
        let (c_support, r_support) = libs.funcs::<Support>(b"c2Support\0");

        for _ in 0..128 {
            let ra = rng.positive();
            let rb = rng.positive();
            let a = c2Circle {
                p: rng.vec(),
                r: ra,
            };
            let b = c2Circle {
                p: v(a.p.x + ra + rb + rng.positive(), a.p.y),
                r: rb,
            };
            let mut cm = seeded_manifold();
            let mut rm = cm;
            c_cc(a, b, &mut cm);
            r_cc(a, b, &mut rm);
            assert_eq!(cm.count, 0, "E01 C rejection");
            assert_bytes("E01", &cm, &rm);
        }

        for _ in 0..128 {
            let b = random_aabb(&mut rng);
            let radius = rng.positive();
            let a = c2Circle {
                p: v(b.max.x + radius + rng.positive(), (b.min.y + b.max.y) * 0.5),
                r: radius,
            };
            let mut cm = seeded_manifold();
            let mut rm = cm;
            c_cb(a, b, &mut cm);
            r_cb(a, b, &mut rm);
            assert_eq!(cm.count, 0, "E02 C rejection");
            assert_bytes("E02", &cm, &rm);
        }

        for _ in 0..128 {
            let ra = rng.positive();
            let rb = rng.positive();
            let capsule = c2Capsule {
                a: v(-2.0, 0.0),
                b: v(2.0, 0.0),
                r: rb,
            };
            let circle = c2Circle {
                p: v(0.0, ra + rb + rng.positive()),
                r: ra,
            };
            let mut cm = seeded_manifold();
            let mut rm = cm;
            c_ck(circle, capsule, &mut cm);
            r_ck(circle, capsule, &mut rm);
            assert_eq!(cm.count, 0, "E03 C rejection");
            assert_bytes("E03", &cm, &rm);
        }

        for axis in 0..2 {
            for _ in 0..128 {
                let a = box_at(0.0, 0.0, 1.0, 1.0);
                let b = if axis == 0 {
                    box_at(4.0 + rng.positive(), 0.0, 1.0, 1.0)
                } else {
                    box_at(0.0, 4.0 + rng.positive(), 1.0, 1.0)
                };
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_bb(a, b, &mut cm);
                r_bb(a, b, &mut rm);
                assert_eq!(cm.count, 0, "E0{} C rejection", 4 + axis);
                assert_bytes(if axis == 0 { "E04" } else { "E05" }, &cm, &rm);
            }
        }

        for _ in 0..128 {
            let radius = rng.positive();
            let aabb = box_at(0.0, 0.0, 2.0, 2.0);
            let y = 2.0 + radius + rng.positive();
            let capsule = c2Capsule {
                a: v(-1.0, y),
                b: v(1.0, y),
                r: radius,
            };
            let mut cm = seeded_manifold();
            let mut rm = cm;
            c_bk(aabb, capsule, &mut cm);
            r_bk(aabb, capsule, &mut rm);
            assert_eq!(cm.count, 0, "E07 C rejection");
            assert_bytes("E07", &cm, &rm);
        }

        for _ in 0..128 {
            let ra = rng.positive();
            let rb = rng.positive();
            let a = c2Capsule {
                a: v(-2.0, 0.0),
                b: v(2.0, 0.0),
                r: ra,
            };
            let b = c2Capsule {
                a: v(-2.0, ra + rb + rng.positive()),
                b: v(2.0, ra + rb + rng.positive()),
                r: rb,
            };
            let mut cm = seeded_manifold();
            let mut rm = cm;
            c_kk(a, b, &mut cm);
            r_kk(a, b, &mut rm);
            assert_eq!(cm.count, 0, "E08 C rejection");
            assert_bytes("E08", &cm, &rm);
        }

        let shape = c2Circle {
            p: v(1.0, 2.0),
            r: 3.0,
        };
        for &bad_type in &[-1, 4, c_int::MAX] {
            for _ in 0..64 {
                let mut cm = seeded_manifold();
                let mut rm = cm;
                c_collide(
                    (&shape as *const c2Circle).cast(),
                    bad_type,
                    (&shape as *const c2Circle).cast(),
                    CIRCLE,
                    &mut cm,
                );
                r_collide(
                    (&shape as *const c2Circle).cast(),
                    bad_type,
                    (&shape as *const c2Circle).cast(),
                    CIRCLE,
                    &mut rm,
                );
                assert_eq!(cm.count, 0, "E09 C rejection");
                assert_bytes("E09", &cm, &rm);
            }
        }

        for &type_a in &[CIRCLE, AABB, CAPSULE] {
            let a = random_shape(&mut rng, type_a);
            for &bad_type in &[POLY, -1, 4, c_int::MAX] {
                for _ in 0..64 {
                    let mut cm = seeded_manifold();
                    let mut rm = cm;
                    c_collide(a.ptr(), type_a, a.ptr(), bad_type, &mut cm);
                    r_collide(a.ptr(), type_a, a.ptr(), bad_type, &mut rm);
                    assert_eq!(cm.count, 0, "E10 C rejection");
                    assert_bytes("E10", &cm, &rm);
                }
            }
        }

        for &bad_type in &[POLY, -1, 4, c_int::MAX] {
            for _ in 0..64 {
                let mut cp = filled::<c2Proxy>(0x6d);
                let mut rp = cp;
                c_proxy((&shape as *const c2Circle).cast(), bad_type, &mut cp);
                r_proxy((&shape as *const c2Circle).cast(), bad_type, &mut rp);
                assert_bytes("E11", &cp, &rp);
                assert_eq!(bytes(&cp), vec![0x6d; std::mem::size_of::<c2Proxy>()]);
            }
        }

        for &(id, count) in &[("E12", 0), ("E13", -1)] {
            for _ in 0..128 {
                let mut cv = [rng.vec(); 8];
                let mut rv = cv;
                let mut cn = [filled::<c2v>(0x7e); 8];
                let mut rn = cn;
                c_norms(cv.as_mut_ptr(), cn.as_mut_ptr(), count);
                r_norms(rv.as_mut_ptr(), rn.as_mut_ptr(), count);
                assert_bytes(id, &cn, &rn);
            }
        }

        for _ in 0..128 {
            let verts = [rng.vec()];
            let direction = rng.vec();
            let ci = c_support(verts.as_ptr(), 0, direction);
            let ri = r_support(verts.as_ptr(), 0, direction);
            assert_eq!(ci, 0, "E14 C sentinel");
            assert_bytes("E14", &ci, &ri);
        }

        for _ in 0..128 {
            let mut cv = [c2v::default(); 9];
            for point in &mut cv {
                *point = rng.vec();
            }
            let mut rv = cv;
            let mut cn = [filled::<c2v>(0x8f); 9];
            let mut rn = cn;
            c_norms(cv.as_mut_ptr(), cn.as_mut_ptr(), 9);
            r_norms(rv.as_mut_ptr(), rn.as_mut_ptr(), 9);
            assert_bytes("E15", &cn, &rn);

            let direction = rng.vec();
            let ci = c_support(cv.as_ptr(), 9, direction);
            let ri = r_support(rv.as_ptr(), 9, direction);
            assert_bytes("E16", &ci, &ri);
        }
    }
}
