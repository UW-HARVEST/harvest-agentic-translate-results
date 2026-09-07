#![allow(non_camel_case_types, non_snake_case, unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::mem::{MaybeUninit, size_of};
use std::path::PathBuf;

const CAPSULE: c_int = 0;
const CIRCLE: c_int = 1;
const AABB: c_int = 2;
const POLY: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2v {
    x: c_float,
    y: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2Manifold {
    count: c_int,
    depths: [c_float; 2],
    contact_points: [c2v; 2],
    n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2h {
    n: c2v,
    d: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2r {
    c: c_float,
    s: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2Circle {
    p: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2GJKCache {
    metric: c_float,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2Proxy {
    radius: c_float,
    count: c_int,
    verts: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: c_float,
    iA: c_int,
    iB: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: c_float,
    count: c_int,
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libharvest-work-QCp39q.so"),
        root.join("target/release/libomni_manifold_lib.so"),
    )
}

unsafe fn libraries() -> (Library, Library) {
    let (c, r) = library_paths();
    (Library::new(c).unwrap(), Library::new(r).unwrap())
}

unsafe fn symbol<T: Copy>(lib: &Library, name: &[u8]) -> T {
    *lib.get::<T>(name).unwrap()
}

fn object_bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, size_of::<T>()) }
}

fn assert_same<T: std::fmt::Debug>(left: &T, right: &T, context: &str) {
    assert_eq!(
        object_bytes(left),
        object_bytes(right),
        "{context}\nC: {left:?}\nRust: {right:?}"
    );
}

fn sentinel<T>() -> T {
    let mut out = MaybeUninit::<T>::uninit();
    unsafe {
        out.as_mut_ptr()
            .cast::<u8>()
            .write_bytes(0xA5, size_of::<T>());
        out.assume_init()
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn f32(&mut self) -> f32 {
        let unit = (self.u32() as f64 / u32::MAX as f64) as f32;
        unit * 40.0 - 20.0
    }

    fn positive(&mut self) -> f32 {
        self.f32().abs() + 0.05
    }

    fn v(&mut self) -> c2v {
        c2v {
            x: self.f32(),
            y: self.f32(),
        }
    }
}

fn circle(rng: &mut Rng) -> c2Circle {
    c2Circle {
        p: rng.v(),
        r: rng.positive(),
    }
}

fn aabb(rng: &mut Rng) -> c2AABB {
    let p = rng.v();
    let e = c2v {
        x: rng.positive(),
        y: rng.positive(),
    };
    c2AABB {
        min: c2v {
            x: p.x - e.x,
            y: p.y - e.y,
        },
        max: c2v {
            x: p.x + e.x,
            y: p.y + e.y,
        },
    }
}

fn capsule(rng: &mut Rng) -> c2Capsule {
    c2Capsule {
        a: rng.v(),
        b: rng.v(),
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
            Shape::Circle(_) => CIRCLE,
            Shape::Aabb(_) => AABB,
            Shape::Capsule(_) => CAPSULE,
        }
    }

    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(v) => v as *const _ as *const c_void,
            Shape::Aabb(v) => v as *const _ as *const c_void,
            Shape::Capsule(v) => v as *const _ as *const c_void,
        }
    }
}

fn random_shape(rng: &mut Rng, typ: c_int) -> Shape {
    match typ {
        CIRCLE => Shape::Circle(circle(rng)),
        AABB => Shape::Aabb(aabb(rng)),
        CAPSULE => Shape::Capsule(capsule(rng)),
        _ => unreachable!(),
    }
}

#[test]
fn all_c_symbols_are_loadable_from_both_shared_objects() {
    const NAMES: &[&[u8]] = &[
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
    unsafe {
        let (c, r) = libraries();
        for name in NAMES {
            c.get::<*const c_void>(name).unwrap();
            r.get::<*const c_void>(name).unwrap();
        }
    }
}

#[test]
fn vector_plane_transform_and_proxy_surface_matches() {
    unsafe {
        let (c, r) = libraries();
        type V2 = unsafe extern "C" fn(c2v, c2v) -> c2v;
        type VF = unsafe extern "C" fn(c2v, c_float) -> c2v;
        type F2 = unsafe extern "C" fn(c2v, c2v) -> c_float;
        type V1 = unsafe extern "C" fn(c2v) -> c2v;
        type F1 = unsafe extern "C" fn(c2v) -> c_float;
        type Clamp = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
        type Dist = unsafe extern "C" fn(c2h, c2v) -> c_float;
        type PlaneAt = unsafe extern "C" fn(*const c2Poly, c_int) -> c2h;
        type RotV = unsafe extern "C" fn(c2r, c2v) -> c2v;
        type XV = unsafe extern "C" fn(c2x, c2v) -> c2v;
        type Intersect = unsafe extern "C" fn(c2v, c2v, c_float, c_float) -> c2v;

        let vector2_names: &[&[u8]] = &[b"c2Add\0", b"c2Sub\0", b"c2Maxv\0", b"c2Minv\0"];
        let float2_names: &[&[u8]] = &[b"c2Dot\0", b"c2Det2\0"];
        let vector1_names: &[&[u8]] = &[
            b"c2Neg\0",
            b"c2Absv\0",
            b"c2CCW90\0",
            b"c2Skew\0",
            b"c2Norm\0",
        ];
        let float1_names: &[&[u8]] = &[b"c2Len\0"];
        let mut rng = Rng::new(0xC0FFEE);

        for case in 0..512 {
            let a = rng.v();
            let b = rng.v();
            let scalar = match case % 8 {
                0 => 0.0,
                1 => -0.0,
                2 => 1.0,
                3 => -1.0,
                _ => rng.f32(),
            };
            for name in vector2_names {
                let cf: V2 = symbol(&c, name);
                let rf: V2 = symbol(&r, name);
                assert_same(
                    &cf(a, b),
                    &rf(a, b),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }
            for name in float2_names {
                let cf: F2 = symbol(&c, name);
                let rf: F2 = symbol(&r, name);
                assert_same(
                    &cf(a, b),
                    &rf(a, b),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }
            for name in vector1_names {
                let cf: V1 = symbol(&c, name);
                let rf: V1 = symbol(&r, name);
                assert_same(
                    &cf(a),
                    &rf(a),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }
            for name in float1_names {
                let cf: F1 = symbol(&c, name);
                let rf: F1 = symbol(&r, name);
                assert_same(&cf(a), &rf(a), "c2Len");
            }
            for name in [b"c2Mulvs\0".as_slice(), b"c2Div\0".as_slice()] {
                let cf: VF = symbol(&c, name);
                let rf: VF = symbol(&r, name);
                assert_same(
                    &cf(a, scalar),
                    &rf(a, scalar),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }

            let lo = c2v { x: -5.0, y: -3.0 };
            let hi = c2v { x: 7.0, y: 11.0 };
            let cf: Clamp = symbol(&c, b"c2Clampv\0");
            let rf: Clamp = symbol(&r, b"c2Clampv\0");
            assert_same(&cf(a, lo, hi), &rf(a, lo, hi), "c2Clampv");

            let h = c2h { n: b, d: scalar };
            let cf: Dist = symbol(&c, b"c2Dist\0");
            let rf: Dist = symbol(&r, b"c2Dist\0");
            assert_same(&cf(h, a), &rf(h, a), "c2Dist");

            let rot = c2r {
                c: rng.f32(),
                s: rng.f32(),
            };
            for name in [b"c2Mulrv\0".as_slice(), b"c2MulrvT\0".as_slice()] {
                let cf: RotV = symbol(&c, name);
                let rf: RotV = symbol(&r, name);
                assert_same(
                    &cf(rot, a),
                    &rf(rot, a),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }
            let x = c2x { p: b, r: rot };
            for name in [b"c2Mulxv\0".as_slice(), b"c2MulxvT\0".as_slice()] {
                let cf: XV = symbol(&c, name);
                let rf: XV = symbol(&r, name);
                assert_same(
                    &cf(x, a),
                    &rf(x, a),
                    std::str::from_utf8(&name[..name.len() - 1]).unwrap(),
                );
            }

            let da = if case % 9 == 0 { scalar } else { rng.f32() };
            let db = if case % 9 == 0 { scalar } else { rng.f32() };
            let cf: Intersect = symbol(&c, b"c2Intersect\0");
            let rf: Intersect = symbol(&r, b"c2Intersect\0");
            assert_same(&cf(a, b, da, db), &rf(a, b, da, db), "c2Intersect");
        }

        type RotIdentity = unsafe extern "C" fn() -> c2r;
        type XIdentity = unsafe extern "C" fn() -> c2x;
        let cf: RotIdentity = symbol(&c, b"c2RotIdentity\0");
        let rf: RotIdentity = symbol(&r, b"c2RotIdentity\0");
        assert_same(&cf(), &rf(), "c2RotIdentity");
        let cf: XIdentity = symbol(&c, b"c2xIdentity\0");
        let rf: XIdentity = symbol(&r, b"c2xIdentity\0");
        assert_same(&cf(), &rf(), "c2xIdentity");

        let mut poly = c2Poly::default();
        poly.count = 8;
        for i in 0..8 {
            poly.verts[i] = rng.v();
            poly.norms[i] = rng.v();
        }
        let cf: PlaneAt = symbol(&c, b"c2PlaneAt\0");
        let rf: PlaneAt = symbol(&r, b"c2PlaneAt\0");
        for i in 0..8 {
            assert_same(&cf(&poly, i), &rf(&poly, i), "c2PlaneAt");
        }

        type BBVerts = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
        type MakeProxy = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
        let cbb: BBVerts = symbol(&c, b"c2BBVerts\0");
        let rbb: BBVerts = symbol(&r, b"c2BBVerts\0");
        let cmp: MakeProxy = symbol(&c, b"c2MakeProxy\0");
        let rmp: MakeProxy = symbol(&r, b"c2MakeProxy\0");
        for _ in 0..128 {
            let mut bb_c = aabb(&mut rng);
            let mut bb_r = bb_c;
            let mut out_c: [c2v; 4] = sentinel();
            let mut out_r = out_c;
            cbb(out_c.as_mut_ptr(), &mut bb_c);
            rbb(out_r.as_mut_ptr(), &mut bb_r);
            assert_same(&out_c, &out_r, "c2BBVerts");

            let shapes = [
                Shape::Circle(circle(&mut rng)),
                Shape::Aabb(aabb(&mut rng)),
                Shape::Capsule(capsule(&mut rng)),
            ];
            for shape in &shapes {
                let mut pc: c2Proxy = sentinel();
                let mut pr = pc;
                cmp(shape.ptr(), shape.typ(), &mut pc);
                rmp(shape.ptr(), shape.typ(), &mut pr);
                assert_same(&pc, &pr, "c2MakeProxy supported");
            }
            let mut pc: c2Proxy = sentinel();
            let mut pr = pc;
            cmp(std::ptr::null(), POLY, &mut pc);
            rmp(std::ptr::null(), POLY, &mut pr);
            assert_same(&pc, &pr, "c2MakeProxy unsupported");
        }
    }
}

#[test]
fn simplex_support_witness_and_norms_match() {
    unsafe {
        let (c, r) = libraries();
        type Metric = unsafe extern "C" fn(*mut c2Simplex) -> c_float;
        type SimplexMut = unsafe extern "C" fn(*mut c2Simplex);
        type SimplexVec = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        type Witness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
        type Support = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
        type Norms = unsafe extern "C" fn(*mut c2v, *mut c2v, c_int);
        let cm: Metric = symbol(&c, b"c2GJKSimplexMetric\0");
        let rm: Metric = symbol(&r, b"c2GJKSimplexMetric\0");
        let c22f: SimplexMut = symbol(&c, b"c22\0");
        let r22f: SimplexMut = symbol(&r, b"c22\0");
        let c23f: SimplexMut = symbol(&c, b"c23\0");
        let r23f: SimplexMut = symbol(&r, b"c23\0");
        let cd: SimplexVec = symbol(&c, b"c2D\0");
        let rd: SimplexVec = symbol(&r, b"c2D\0");
        let cl: SimplexVec = symbol(&c, b"c2L\0");
        let rl: SimplexVec = symbol(&r, b"c2L\0");
        let cw: Witness = symbol(&c, b"c2Witness\0");
        let rw: Witness = symbol(&r, b"c2Witness\0");
        let cs: Support = symbol(&c, b"c2Support\0");
        let rs: Support = symbol(&r, b"c2Support\0");
        let cn: Norms = symbol(&c, b"c2Norms\0");
        let rn: Norms = symbol(&r, b"c2Norms\0");
        let mut rng = Rng::new(0x51A1_5EED);

        let targeted22 = [
            (c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }),
            (c2v { x: -2.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }),
            (c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }),
        ];
        for (a, b) in targeted22 {
            let mut sc: c2Simplex = sentinel();
            sc.a.p = a;
            sc.b.p = b;
            sc.count = 2;
            let mut sr = sc;
            c22f(&mut sc);
            r22f(&mut sr);
            assert_same(&sc, &sr, "c22 targeted");
        }

        for case in 0..2048 {
            let mut base: c2Simplex = sentinel();
            base.a.p = rng.v();
            base.b.p = rng.v();
            base.c.p = rng.v();
            base.a.sA = rng.v();
            base.a.sB = rng.v();
            base.b.sA = rng.v();
            base.b.sB = rng.v();
            base.c.sA = rng.v();
            base.c.sB = rng.v();
            base.a.u = rng.positive();
            base.b.u = rng.positive();
            base.c.u = rng.positive();
            base.div = base.a.u + base.b.u + base.c.u;
            base.count = (case % 6) - 1;

            let mut sc = base;
            let mut sr = base;
            assert_same(&cm(&mut sc), &rm(&mut sr), "c2GJKSimplexMetric");
            assert_same(&cd(&mut sc), &rd(&mut sr), "c2D");
            assert_same(&cl(&mut sc), &rl(&mut sr), "c2L");
            let mut ac: c2v = sentinel();
            let mut bc: c2v = sentinel();
            let mut ar = ac;
            let mut br = bc;
            cw(&mut sc, &mut ac, &mut bc);
            rw(&mut sr, &mut ar, &mut br);
            assert_same(&ac, &ar, "c2Witness A");
            assert_same(&bc, &br, "c2Witness B");

            let mut s22c = base;
            let mut s22r = base;
            s22c.count = 2;
            s22r.count = 2;
            c22f(&mut s22c);
            r22f(&mut s22r);
            assert_same(&s22c, &s22r, "c22 random");

            let mut s23c = base;
            let mut s23r = base;
            s23c.count = 3;
            s23r.count = 3;
            c23f(&mut s23c);
            r23f(&mut s23r);
            assert_same(&s23c, &s23r, "c23 random");
        }

        for count in [1, 2, 4, 8] {
            for _ in 0..128 {
                let mut verts = [c2v::default(); 8];
                for v in verts.iter_mut().take(count) {
                    *v = rng.v();
                }
                let d = rng.v();
                assert_same(
                    &cs(verts.as_ptr(), count as c_int, d),
                    &rs(verts.as_ptr(), count as c_int, d),
                    "c2Support",
                );
                let mut nc: [c2v; 8] = sentinel();
                let mut nr = nc;
                let mut vc = verts;
                let mut vr = verts;
                cn(vc.as_mut_ptr(), nc.as_mut_ptr(), count as c_int);
                rn(vr.as_mut_ptr(), nr.as_mut_ptr(), count as c_int);
                assert_same(&nc, &nr, "c2Norms");
            }
        }
        let mut vc = [c2v::default(); 8];
        let mut vr = vc;
        let mut nc: [c2v; 8] = sentinel();
        let mut nr = nc;
        cn(vc.as_mut_ptr(), nc.as_mut_ptr(), 0);
        rn(vr.as_mut_ptr(), nr.as_mut_ptr(), 0);
        assert_same(&nc, &nr, "c2Norms count zero");

        let tied = [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 1.0, y: 2.0 },
            c2v { x: -1.0, y: 4.0 },
        ];
        let d = c2v { x: 1.0, y: 0.0 };
        assert_same(
            &cs(tied.as_ptr(), 3, d),
            &rs(tied.as_ptr(), 3, d),
            "c2Support tie",
        );
    }
}

#[test]
fn gjk_options_shape_pairs_transforms_and_cache_match() {
    unsafe {
        let (c, r) = libraries();
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
        ) -> c_float;
        let cg: Gjk = symbol(&c, b"c2GJK\0");
        let rg: Gjk = symbol(&r, b"c2GJK\0");
        let mut rng = Rng::new(0x6A6B_2026);
        let types = [CIRCLE, AABB, CAPSULE];

        for &ta in &types {
            for &tb in &types {
                for case in 0..256 {
                    let a = random_shape(&mut rng, ta);
                    let b = random_shape(&mut rng, tb);
                    let ax = c2x {
                        p: rng.v(),
                        r: if case % 3 == 0 {
                            c2r { c: 1.0, s: 0.0 }
                        } else {
                            c2r {
                                c: rng.f32(),
                                s: rng.f32(),
                            }
                        },
                    };
                    let bx = c2x {
                        p: rng.v(),
                        r: if case % 5 == 0 {
                            c2r { c: 1.0, s: 0.0 }
                        } else {
                            c2r {
                                c: rng.f32(),
                                s: rng.f32(),
                            }
                        },
                    };
                    let axp = if case % 2 == 0 { std::ptr::null() } else { &ax };
                    let bxp = if case % 4 == 0 { std::ptr::null() } else { &bx };
                    let use_radius = (case % 2) as c_int;
                    let mut out_ac: c2v = sentinel();
                    let mut out_bc: c2v = sentinel();
                    let mut out_ar = out_ac;
                    let mut out_br = out_bc;
                    let mut iter_c: c_int = sentinel();
                    let mut iter_r = iter_c;
                    let dc = cg(
                        a.ptr(),
                        ta,
                        axp,
                        b.ptr(),
                        tb,
                        bxp,
                        &mut out_ac,
                        &mut out_bc,
                        use_radius,
                        &mut iter_c,
                        std::ptr::null_mut(),
                    );
                    let dr = rg(
                        a.ptr(),
                        ta,
                        axp,
                        b.ptr(),
                        tb,
                        bxp,
                        &mut out_ar,
                        &mut out_br,
                        use_radius,
                        &mut iter_r,
                        std::ptr::null_mut(),
                    );
                    assert_same(&dc, &dr, "c2GJK distance");
                    assert_same(&out_ac, &out_ar, "c2GJK outA");
                    assert_same(&out_bc, &out_br, "c2GJK outB");
                    assert_same(&iter_c, &iter_r, "c2GJK iterations");

                    let dc = cg(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        use_radius,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    let dr = rg(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        use_radius,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    assert_same(&dc, &dr, "c2GJK null optional outputs");
                }
            }
        }

        for &ta in &types {
            for &tb in &types {
                for _ in 0..128 {
                    let a = random_shape(&mut rng, ta);
                    let b = random_shape(&mut rng, tb);
                    let mut cache_c = c2GJKCache::default();
                    let mut cache_r = cache_c;
                    let mut ac = c2v::default();
                    let mut bc = c2v::default();
                    let mut ar = ac;
                    let mut br = bc;
                    let dc = cg(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        &mut ac,
                        &mut bc,
                        0,
                        std::ptr::null_mut(),
                        &mut cache_c,
                    );
                    let dr = rg(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        &mut ar,
                        &mut br,
                        0,
                        std::ptr::null_mut(),
                        &mut cache_r,
                    );
                    assert_same(&dc, &dr, "c2GJK empty cache distance");
                    assert_same(&cache_c, &cache_r, "c2GJK populated cache");

                    let dc2 = cg(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        &mut ac,
                        &mut bc,
                        1,
                        std::ptr::null_mut(),
                        &mut cache_c,
                    );
                    let dr2 = rg(
                        a.ptr(),
                        ta,
                        std::ptr::null(),
                        b.ptr(),
                        tb,
                        std::ptr::null(),
                        &mut ar,
                        &mut br,
                        1,
                        std::ptr::null_mut(),
                        &mut cache_r,
                    );
                    assert_same(&dc2, &dr2, "c2GJK reused cache distance");
                    assert_same(&ac, &ar, "c2GJK reused cache outA");
                    assert_same(&bc, &br, "c2GJK reused cache outB");
                    assert_same(&cache_c, &cache_r, "c2GJK reused cache state");
                }
            }
        }
    }
}

#[test]
fn direct_manifold_functions_match_randomized_and_boundary_cases() {
    unsafe {
        let (c, r) = libraries();
        type CC = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
        type CA = unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold);
        type CP = unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold);
        type AA = unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold);
        type PP = unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold);
        let ccc: CC = symbol(&c, b"c2CircletoCircleManifold\0");
        let rcc: CC = symbol(&r, b"c2CircletoCircleManifold\0");
        let cca: CA = symbol(&c, b"c2CircletoAABBManifold\0");
        let rca: CA = symbol(&r, b"c2CircletoAABBManifold\0");
        let ccp: CP = symbol(&c, b"c2CircletoCapsuleManifold\0");
        let rcp: CP = symbol(&r, b"c2CircletoCapsuleManifold\0");
        let caa: AA = symbol(&c, b"c2AABBtoAABBManifold\0");
        let raa: AA = symbol(&r, b"c2AABBtoAABBManifold\0");
        let cpp: PP = symbol(&c, b"c2CapsuletoCapsuleManifold\0");
        let rpp: PP = symbol(&r, b"c2CapsuletoCapsuleManifold\0");
        let mut rng = Rng::new(0xC011_1DE);

        for _ in 0..4096 {
            let c1 = circle(&mut rng);
            let c2 = circle(&mut rng);
            let bb1 = aabb(&mut rng);
            let bb2 = aabb(&mut rng);
            let p1 = capsule(&mut rng);
            let p2 = capsule(&mut rng);

            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            ccc(c1, c2, &mut mc);
            rcc(c1, c2, &mut mr);
            assert_same(&mc, &mr, "c2CircletoCircleManifold");

            mc = sentinel();
            mr = mc;
            cca(c1, bb1, &mut mc);
            rca(c1, bb1, &mut mr);
            assert_same(&mc, &mr, "c2CircletoAABBManifold");

            mc = sentinel();
            mr = mc;
            ccp(c1, p1, &mut mc);
            rcp(c1, p1, &mut mr);
            assert_same(&mc, &mr, "c2CircletoCapsuleManifold");

            mc = sentinel();
            mr = mc;
            caa(bb1, bb2, &mut mc);
            raa(bb1, bb2, &mut mr);
            assert_same(&mc, &mr, "c2AABBtoAABBManifold");

            mc = sentinel();
            mr = mc;
            cpp(p1, p2, &mut mc);
            rpp(p1, p2, &mut mr);
            assert_same(&mc, &mr, "c2CapsuletoCapsuleManifold");
        }

        let circle_cases = [
            (
                c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
                c2Circle {
                    p: c2v { x: 2.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: 2.0,
                },
                c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                c2Circle {
                    p: c2v { x: -1.0, y: 0.0 },
                    r: 2.0,
                },
                c2Circle {
                    p: c2v { x: 1.5, y: 0.0 },
                    r: 1.0,
                },
            ),
        ];
        for (a, b) in circle_cases {
            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            ccc(a, b, &mut mc);
            rcc(a, b, &mut mr);
            assert_same(&mc, &mr, "circle boundary");
        }

        let box0 = c2AABB {
            min: c2v { x: -2.0, y: -1.0 },
            max: c2v { x: 2.0, y: 1.0 },
        };
        for c0 in [
            c2Circle {
                p: c2v { x: 3.0, y: 0.0 },
                r: 1.0,
            },
            c2Circle {
                p: c2v { x: 2.5, y: 0.0 },
                r: 1.0,
            },
            c2Circle {
                p: c2v { x: 0.0, y: 0.0 },
                r: 1.0,
            },
            c2Circle {
                p: c2v { x: 1.9, y: 0.0 },
                r: 1.0,
            },
        ] {
            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            cca(c0, box0, &mut mc);
            rca(c0, box0, &mut mr);
            assert_same(&mc, &mr, "circle AABB boundary");
        }
    }
}

#[test]
fn collide_omni_and_all_ordered_shape_pairs_match() {
    unsafe {
        let (c, r) = libraries();
        type Collide =
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
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
        let cc: Collide = symbol(&c, b"c2Collide\0");
        let rc: Collide = symbol(&r, b"c2Collide\0");
        let co: Omni = symbol(&c, b"omni_manifold\0");
        let ro: Omni = symbol(&r, b"omni_manifold\0");
        let mut rng = Rng::new(0x0A11_0A11);
        let types = [CIRCLE, AABB, CAPSULE];

        for &ta in &types {
            for &tb in &types {
                for _ in 0..1024 {
                    if (ta == AABB && tb == CAPSULE) || (ta == CAPSULE && tb == AABB) {
                        continue;
                    }
                    let a = random_shape(&mut rng, ta);
                    let b = random_shape(&mut rng, tb);
                    let mut mc: c2Manifold = sentinel();
                    let mut mr = mc;
                    cc(a.ptr(), ta, b.ptr(), tb, &mut mc);
                    rc(a.ptr(), ta, b.ptr(), tb, &mut mr);
                    assert_same(&mc, &mr, "c2Collide ordered pair");

                    let av = [rng.f32(), rng.f32(), rng.f32(), rng.f32(), rng.f32()];
                    let bv = [rng.f32(), rng.f32(), rng.f32(), rng.f32(), rng.f32()];
                    mc = sentinel();
                    mr = mc;
                    co(
                        &mut mc, ta, av[0], av[1], av[2], av[3], av[4], tb, bv[0], bv[1], bv[2],
                        bv[3], bv[4],
                    );
                    ro(
                        &mut mr, ta, av[0], av[1], av[2], av[3], av[4], tb, bv[0], bv[1], bv[2],
                        bv[3], bv[4],
                    );
                    assert_same(&mc, &mr, "omni_manifold ordered pair");
                }
            }
        }

        for &(ta, tb) in &[(POLY, CIRCLE), (99, CIRCLE), (CIRCLE, POLY), (CIRCLE, -7)] {
            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            cc(std::ptr::null(), ta, std::ptr::null(), tb, &mut mc);
            rc(std::ptr::null(), ta, std::ptr::null(), tb, &mut mr);
            assert_same(&mc, &mr, "c2Collide unsupported enum");
            assert_eq!(mc.count, 0);
        }
    }
}

#[test]
fn ptr_from_parts_valid_field_mapping_matches() {
    unsafe {
        let (c, r) = libraries();
        type Parts = unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut c_void;
        let cp: Parts = symbol(&c, b"ptr_from_parts\0");
        let rp: Parts = symbol(&r, b"ptr_from_parts\0");
        let values = (1.25, -2.5, 3.75, -4.5, 5.25);

        let pc = cp(CIRCLE, values.0, values.1, values.2, values.3, values.4) as *const c2Circle;
        let pr = rp(CIRCLE, values.0, values.1, values.2, values.3, values.4) as *const c2Circle;
        assert_same(&*pc, &*pr, "ptr_from_parts circle");

        let pc = cp(AABB, values.0, values.1, values.2, values.3, values.4) as *const c2AABB;
        let pr = rp(AABB, values.0, values.1, values.2, values.3, values.4) as *const c2AABB;
        assert_same(&*pc, &*pr, "ptr_from_parts AABB");

        let pc = cp(CAPSULE, values.0, values.1, values.2, values.3, values.4) as *const c2Capsule;
        let pr = rp(CAPSULE, values.0, values.1, values.2, values.3, values.4) as *const c2Capsule;
        assert_same(&*pc, &*pr, "ptr_from_parts capsule");
    }
}

#[test]
fn capsule_poly_entry_point_matches_for_rectangles_and_transforms() {
    unsafe {
        let (c, r) = libraries();
        type CapsulePoly =
            unsafe extern "C" fn(c2Capsule, *const c2Poly, *const c2x, *mut c2Manifold);
        let cf: CapsulePoly = symbol(&c, b"c2CapsuletoPolyManifold\0");
        let rf: CapsulePoly = symbol(&r, b"c2CapsuletoPolyManifold\0");
        let mut rng = Rng::new(0xF011_u64);
        for case in 0..512 {
            let min = rng.v();
            let ext = c2v {
                x: rng.positive(),
                y: rng.positive(),
            };
            let max = c2v {
                x: min.x + ext.x,
                y: min.y + ext.y,
            };
            let mut poly = c2Poly::default();
            poly.count = 4;
            poly.verts[0] = min;
            poly.verts[1] = c2v { x: max.x, y: min.y };
            poly.verts[2] = max;
            poly.verts[3] = c2v { x: min.x, y: max.y };
            poly.norms[0] = c2v { x: 0.0, y: -1.0 };
            poly.norms[1] = c2v { x: 1.0, y: 0.0 };
            poly.norms[2] = c2v { x: 0.0, y: 1.0 };
            poly.norms[3] = c2v { x: -1.0, y: 0.0 };
            let far = 1.0e10_f32 + case as f32 * 1024.0;
            let cap = c2Capsule {
                a: c2v { x: far, y: far },
                b: c2v {
                    x: far + 128.0,
                    y: far + 256.0,
                },
                r: 0.5,
            };
            let transform = c2x {
                p: rng.v(),
                r: c2r {
                    c: if case % 2 == 0 { 1.0 } else { 0.0 },
                    s: if case % 2 == 0 { 0.0 } else { 1.0 },
                },
            };
            let xp = if case % 3 == 0 {
                std::ptr::null()
            } else {
                &transform
            };
            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            cf(cap, &poly, xp, &mut mc);
            rf(cap, &poly, xp, &mut mr);
            assert_same(&mc, &mr, "c2CapsuletoPolyManifold");
        }
    }
}

#[test]
fn polygon_composed_exports_match_on_stable_no_contact_cases() {
    unsafe {
        let (c, r) = libraries();
        type AP = unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold);
        type Collide =
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
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
        let cap: AP = symbol(&c, b"c2AABBtoCapsuleManifold\0");
        let rap: AP = symbol(&r, b"c2AABBtoCapsuleManifold\0");
        let cc: Collide = symbol(&c, b"c2Collide\0");
        let rc: Collide = symbol(&r, b"c2Collide\0");
        let co: Omni = symbol(&c, b"omni_manifold\0");
        let ro: Omni = symbol(&r, b"omni_manifold\0");
        let mut rng = Rng::new(0xAABB_CA95);

        for i in 0..512 {
            let base = 100_000.0 + i as f32 * 100.0;
            let bb = c2AABB {
                min: c2v {
                    x: base,
                    y: base + rng.f32(),
                },
                max: c2v {
                    x: base + rng.positive(),
                    y: base + 50.0 + rng.positive(),
                },
            };
            let cp = c2Capsule {
                a: c2v { x: -base, y: -base },
                b: c2v {
                    x: -base - 10.0,
                    y: -base - 20.0,
                },
                r: 0.5,
            };
            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            cap(bb, cp, &mut mc);
            rap(bb, cp, &mut mr);
            assert_same(&mc, &mr, "c2AABBtoCapsuleManifold stable rejection");

            mc = sentinel();
            mr = mc;
            cc(
                &bb as *const _ as *const c_void,
                AABB,
                &cp as *const _ as *const c_void,
                CAPSULE,
                &mut mc,
            );
            rc(
                &bb as *const _ as *const c_void,
                AABB,
                &cp as *const _ as *const c_void,
                CAPSULE,
                &mut mr,
            );
            assert_same(&mc, &mr, "c2Collide AABB-capsule stable rejection");

            mc = sentinel();
            mr = mc;
            co(
                &mut mc, AABB, bb.min.x, bb.min.y, bb.max.x, bb.max.y, 0.0, CAPSULE, cp.a.x,
                cp.a.y, cp.b.x, cp.b.y, cp.r,
            );
            ro(
                &mut mr, AABB, bb.min.x, bb.min.y, bb.max.x, bb.max.y, 0.0, CAPSULE, cp.a.x,
                cp.a.y, cp.b.x, cp.b.y, cp.r,
            );
            assert_same(&mc, &mr, "omni AABB-capsule stable rejection");
        }
    }
}

#[test]
fn explicit_error_and_boundary_results_match() {
    unsafe {
        let (c, r) = libraries();
        type Metric = unsafe extern "C" fn(*mut c2Simplex) -> c_float;
        type SimplexVec = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        type Witness = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
        type Collide =
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int, *mut c2Manifold);
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
        type PlaneAt = unsafe extern "C" fn(*const c2Poly, c_int) -> c2h;
        let cm: Metric = symbol(&c, b"c2GJKSimplexMetric\0");
        let rm: Metric = symbol(&r, b"c2GJKSimplexMetric\0");
        let cd: SimplexVec = symbol(&c, b"c2D\0");
        let rd: SimplexVec = symbol(&r, b"c2D\0");
        let cl: SimplexVec = symbol(&c, b"c2L\0");
        let rl: SimplexVec = symbol(&r, b"c2L\0");
        let cw: Witness = symbol(&c, b"c2Witness\0");
        let rw: Witness = symbol(&r, b"c2Witness\0");
        let cc: Collide = symbol(&c, b"c2Collide\0");
        let rc: Collide = symbol(&r, b"c2Collide\0");
        let co: Omni = symbol(&c, b"omni_manifold\0");
        let ro: Omni = symbol(&r, b"omni_manifold\0");

        for count in [-7, 0, 1, 3, 4, 99] {
            let mut sc: c2Simplex = sentinel();
            sc.count = count;
            sc.div = 1.0;
            let mut sr = sc;
            assert_same(&cm(&mut sc), &rm(&mut sr), "metric default count");
            assert_same(&cd(&mut sc), &rd(&mut sr), "direction default count");
            assert_same(&cl(&mut sc), &rl(&mut sr), "L default count");
            let mut ac: c2v = sentinel();
            let mut bc: c2v = sentinel();
            let mut ar = ac;
            let mut br = bc;
            cw(&mut sc, &mut ac, &mut bc);
            rw(&mut sr, &mut ar, &mut br);
            assert_same(&ac, &ar, "witness default A");
            assert_same(&bc, &br, "witness default B");
        }

        for &(ta, tb) in &[
            (POLY, CIRCLE),
            (-1, CIRCLE),
            (4, CIRCLE),
            (CIRCLE, POLY),
            (CIRCLE, -1),
            (CIRCLE, 4),
        ] {
            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            cc(std::ptr::null(), ta, std::ptr::null(), tb, &mut mc);
            rc(std::ptr::null(), ta, std::ptr::null(), tb, &mut mr);
            assert_same(&mc, &mr, "unsupported c2Collide enum");
            assert_eq!(mc.count, 0);
        }

        for &(ta, tb) in &[(POLY, CIRCLE), (-1, CIRCLE), (CIRCLE, POLY), (CIRCLE, 77)] {
            let mut mc: c2Manifold = sentinel();
            let mut mr = mc;
            co(
                &mut mc, ta, 1.0, 2.0, 3.0, 4.0, 5.0, tb, 6.0, 7.0, 8.0, 9.0, 10.0,
            );
            ro(
                &mut mr, ta, 1.0, 2.0, 3.0, 4.0, 5.0, tb, 6.0, 7.0, 8.0, 9.0, 10.0,
            );
            assert_same(&mc, &mr, "unsupported omni enum");
            assert_eq!(mc.count, 0);
        }

        #[repr(C)]
        struct PolyWithTail {
            poly: c2Poly,
            tail: [c2v; 2],
        }
        let mut wrapped = PolyWithTail {
            poly: c2Poly::default(),
            tail: [c2v { x: 13.0, y: -17.0 }, c2v { x: 19.0, y: -23.0 }],
        };
        wrapped.poly.count = 8;
        for i in 0..8 {
            wrapped.poly.verts[i] = c2v {
                x: i as f32 + 0.25,
                y: i as f32 - 0.5,
            };
            wrapped.poly.norms[i] = c2v {
                x: i as f32 + 10.0,
                y: i as f32 + 20.0,
            };
        }
        let cp: PlaneAt = symbol(&c, b"c2PlaneAt\0");
        let rp: PlaneAt = symbol(&r, b"c2PlaneAt\0");
        assert_same(
            &cp(&wrapped.poly, 8),
            &rp(&wrapped.poly, 8),
            "c2PlaneAt one-past fixed array",
        );
    }
}

#[test]
fn ffi_required_null_child() {
    let Ok(which) = std::env::var("FFI_NULL_LIBRARY") else {
        return;
    };
    unsafe {
        let (c_path, r_path) = library_paths();
        let lib = Library::new(if which == "c" { c_path } else { r_path }).unwrap();
        type CC = unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold);
        let f: CC = symbol(&lib, b"c2CircletoCircleManifold\0");
        f(
            c2Circle::default(),
            c2Circle::default(),
            std::ptr::null_mut(),
        );
    }
}

#[test]
fn required_null_pointer_termination_matches() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let exe = std::env::current_exe().unwrap();
    let run = |which: &str| {
        Command::new(&exe)
            .args(["--exact", "ffi_required_null_child", "--nocapture"])
            .env("FFI_NULL_LIBRARY", which)
            .output()
            .unwrap()
            .status
    };
    let c = run("c");
    let r = run("rust");
    assert!(!c.success(), "C required-null call unexpectedly succeeded");
    assert!(
        !r.success(),
        "Rust required-null call unexpectedly succeeded"
    );
    assert_eq!(c.signal(), r.signal(), "required-null termination signal");
}
