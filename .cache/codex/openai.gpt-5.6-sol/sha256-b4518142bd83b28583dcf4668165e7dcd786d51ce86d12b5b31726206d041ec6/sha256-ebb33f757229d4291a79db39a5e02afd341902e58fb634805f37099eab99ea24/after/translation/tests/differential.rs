use libloading::Library;
use std::ffi::{c_float, c_int, c_void};
use std::mem::{MaybeUninit, size_of};
use std::path::PathBuf;
use std::slice;

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
        unsafe { MaybeUninit::<Self>::zeroed().assume_init() }
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

struct Api {
    _lib: Library,
    v: unsafe extern "C" fn(c_float, c_float) -> V,
    mulvs: unsafe extern "C" fn(V, c_float) -> V,
    maxv: unsafe extern "C" fn(V, V) -> V,
    minv: unsafe extern "C" fn(V, V) -> V,
    clampv: unsafe extern "C" fn(V, V, V) -> V,
    sub: unsafe extern "C" fn(V, V) -> V,
    dot: unsafe extern "C" fn(V, V) -> c_float,
    rot_identity: unsafe extern "C" fn() -> R,
    x_identity: unsafe extern "C" fn() -> X,
    bb_verts: unsafe extern "C" fn(*mut V, *mut Aabb),
    make_proxy: unsafe extern "C" fn(*const c_void, c_int, *mut Proxy),
    len: unsafe extern "C" fn(V) -> c_float,
    det2: unsafe extern "C" fn(V, V) -> c_float,
    metric: unsafe extern "C" fn(*mut Simplex) -> c_float,
    mulrv: unsafe extern "C" fn(R, V) -> V,
    add: unsafe extern "C" fn(V, V) -> V,
    mulxv: unsafe extern "C" fn(X, V) -> V,
    solve2: unsafe extern "C" fn(*mut Simplex),
    solve3: unsafe extern "C" fn(*mut Simplex),
    neg: unsafe extern "C" fn(V) -> V,
    skew: unsafe extern "C" fn(V) -> V,
    ccw90: unsafe extern "C" fn(V) -> V,
    direction: unsafe extern "C" fn(*mut Simplex) -> V,
    support: unsafe extern "C" fn(*const V, c_int, V) -> c_int,
    witness: unsafe extern "C" fn(*mut Simplex, *mut V, *mut V),
    div: unsafe extern "C" fn(V, c_float) -> V,
    norm: unsafe extern "C" fn(V) -> V,
    closest: unsafe extern "C" fn(*mut Simplex) -> V,
    mulrvt: unsafe extern "C" fn(R, V) -> V,
    gjk: GjkFn,
    aabb_aabb: unsafe extern "C" fn(Aabb, Aabb) -> c_int,
    aabb_capsule: unsafe extern "C" fn(Aabb, Capsule) -> c_int,
    capsule_capsule: unsafe extern "C" fn(Capsule, Capsule) -> c_int,
    circle_circle: unsafe extern "C" fn(Circle, Circle) -> c_int,
    circle_aabb: unsafe extern "C" fn(Circle, Aabb) -> c_int,
    circle_capsule: unsafe extern "C" fn(Circle, Capsule) -> c_int,
    collided: unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
    parts: unsafe extern "C" fn(c_int, c_float, c_float, c_float, c_float, c_float) -> *mut c_void,
    omni: unsafe extern "C" fn(
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
    ) -> c_int,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
            unsafe { *lib.get::<T>(name).unwrap() }
        }

        let lib = unsafe { Library::new(path).unwrap() };
        let api = Self {
            v: unsafe { sym(&lib, b"c2V") },
            mulvs: unsafe { sym(&lib, b"c2Mulvs") },
            maxv: unsafe { sym(&lib, b"c2Maxv") },
            minv: unsafe { sym(&lib, b"c2Minv") },
            clampv: unsafe { sym(&lib, b"c2Clampv") },
            sub: unsafe { sym(&lib, b"c2Sub") },
            dot: unsafe { sym(&lib, b"c2Dot") },
            rot_identity: unsafe { sym(&lib, b"c2RotIdentity") },
            x_identity: unsafe { sym(&lib, b"c2xIdentity") },
            bb_verts: unsafe { sym(&lib, b"c2BBVerts") },
            make_proxy: unsafe { sym(&lib, b"c2MakeProxy") },
            len: unsafe { sym(&lib, b"c2Len") },
            det2: unsafe { sym(&lib, b"c2Det2") },
            metric: unsafe { sym(&lib, b"c2GJKSimplexMetric") },
            mulrv: unsafe { sym(&lib, b"c2Mulrv") },
            add: unsafe { sym(&lib, b"c2Add") },
            mulxv: unsafe { sym(&lib, b"c2Mulxv") },
            solve2: unsafe { sym(&lib, b"c22") },
            solve3: unsafe { sym(&lib, b"c23") },
            neg: unsafe { sym(&lib, b"c2Neg") },
            skew: unsafe { sym(&lib, b"c2Skew") },
            ccw90: unsafe { sym(&lib, b"c2CCW90") },
            direction: unsafe { sym(&lib, b"c2D") },
            support: unsafe { sym(&lib, b"c2Support") },
            witness: unsafe { sym(&lib, b"c2Witness") },
            div: unsafe { sym(&lib, b"c2Div") },
            norm: unsafe { sym(&lib, b"c2Norm") },
            closest: unsafe { sym(&lib, b"c2L") },
            mulrvt: unsafe { sym(&lib, b"c2MulrvT") },
            gjk: unsafe { sym(&lib, b"c2GJK") },
            aabb_aabb: unsafe { sym(&lib, b"c2AABBtoAABB") },
            aabb_capsule: unsafe { sym(&lib, b"c2AABBtoCapsule") },
            capsule_capsule: unsafe { sym(&lib, b"c2CapsuletoCapsule") },
            circle_circle: unsafe { sym(&lib, b"c2CircletoCircle") },
            circle_aabb: unsafe { sym(&lib, b"c2CircletoAABB") },
            circle_capsule: unsafe { sym(&lib, b"c2CircletoCapsule") },
            collided: unsafe { sym(&lib, b"c2Collided") },
            parts: unsafe { sym(&lib, b"ptr_from_parts") },
            omni: unsafe { sym(&lib, b"omni_collide") },
            _lib: lib,
        };
        api
    }
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = root.join("../c_src/build/libharvest-work-uMUtVU.so");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let rust = root.join(format!("target/{profile}/libomni_collide_lib.so"));
    assert!(c.exists(), "missing C shared object: {}", c.display());
    assert!(
        rust.exists(),
        "missing Rust shared object: {}",
        rust.display()
    );
    unsafe { (Api::load(c), Api::load(rust)) }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe { slice::from_raw_parts((value as *const T).cast(), size_of::<T>()) }
}

fn same<T>(label: &str, c: &T, r: &T) {
    assert_eq!(bytes(c), bytes(r), "{label}");
}

fn poisoned<T: Copy>() -> T {
    let mut value = MaybeUninit::<T>::uninit();
    unsafe {
        std::ptr::write_bytes(value.as_mut_ptr().cast::<u8>(), 0xa5, size_of::<T>());
        value.assume_init()
    }
}

struct Rng(u32);

impl Rng {
    fn new() -> Self {
        Self(0x51f1_5e5d)
    }

    fn u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn f(&mut self) -> f32 {
        ((self.u32() % 4001) as i32 - 2000) as f32 / 16.0
    }

    fn nonzero(&mut self) -> f32 {
        let magnitude = (self.u32() % 255 + 1) as f32 / 32.0;
        if self.u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
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
        _ => (capsule as *const Capsule).cast(),
    }
}

fn simplex(points: [V; 3]) -> Simplex {
    let mut s = Simplex::default();
    s.a.p = points[0];
    s.b.p = points[1];
    s.c.p = points[2];
    s.a.ia = 10;
    s.b.ia = 20;
    s.c.ia = 30;
    s.count = 3;
    s
}

fn parts_for(typ: c_int, circle: &Circle, aabb: &Aabb, capsule: &Capsule) -> [f32; 5] {
    match typ {
        CIRCLE => [circle.p.x, circle.p.y, circle.r, 0.0, 0.0],
        AABB => [aabb.min.x, aabb.min.y, aabb.max.x, aabb.max.y, 0.0],
        _ => [
            capsule.a.x,
            capsule.a.y,
            capsule.b.x,
            capsule.b.y,
            capsule.r,
        ],
    }
}

fn shape_size(typ: c_int) -> usize {
    match typ {
        CIRCLE => size_of::<Circle>(),
        AABB => size_of::<Aabb>(),
        _ => size_of::<Capsule>(),
    }
}

#[test]
fn low_level_valid_paths_are_byte_identical() {
    let (c, r) = libraries();
    let mut rng = Rng::new();

    unsafe {
        for n in 0..5_000 {
            let a = rng.v();
            let b = rng.v();
            let lo = rng.v();
            let hi = rng.v();
            let scalar = if n % 11 == 0 { 0.0 } else { rng.nonzero() };
            let rot = R {
                c: rng.f() / 64.0,
                s: rng.f() / 64.0,
            };
            let transform = X { p: rng.v(), r: rot };

            same("c2V", &(c.v)(a.x, a.y), &(r.v)(a.x, a.y));
            same("c2Mulvs", &(c.mulvs)(a, scalar), &(r.mulvs)(a, scalar));
            same("c2Maxv", &(c.maxv)(a, b), &(r.maxv)(a, b));
            same("c2Minv", &(c.minv)(a, b), &(r.minv)(a, b));
            same("c2Clampv", &(c.clampv)(a, lo, hi), &(r.clampv)(a, lo, hi));
            same("c2Sub", &(c.sub)(a, b), &(r.sub)(a, b));
            same("c2Dot", &(c.dot)(a, b), &(r.dot)(a, b));
            same("c2RotIdentity", &(c.rot_identity)(), &(r.rot_identity)());
            same("c2xIdentity", &(c.x_identity)(), &(r.x_identity)());
            same("c2Len", &(c.len)(a), &(r.len)(a));
            same("c2Det2", &(c.det2)(a, b), &(r.det2)(a, b));
            same("c2Mulrv", &(c.mulrv)(rot, a), &(r.mulrv)(rot, a));
            same("c2Add", &(c.add)(a, b), &(r.add)(a, b));
            same(
                "c2Mulxv",
                &(c.mulxv)(transform, a),
                &(r.mulxv)(transform, a),
            );
            same("c2Neg", &(c.neg)(a), &(r.neg)(a));
            same("c2Skew", &(c.skew)(a), &(r.skew)(a));
            same("c2CCW90", &(c.ccw90)(a), &(r.ccw90)(a));
            let divisor = rng.nonzero();
            same("c2Div", &(c.div)(a, divisor), &(r.div)(a, divisor));
            if a.x != 0.0 || a.y != 0.0 {
                same("c2Norm", &(c.norm)(a), &(r.norm)(a));
            }
            same("c2MulrvT", &(c.mulrvt)(rot, a), &(r.mulrvt)(rot, a));

            let mut box_c = Aabb {
                min: rng.v(),
                max: rng.v(),
            };
            let mut box_r = box_c;
            let mut verts_c = [V::default(); 4];
            let mut verts_r = [V::default(); 4];
            (c.bb_verts)(verts_c.as_mut_ptr(), &mut box_c);
            (r.bb_verts)(verts_r.as_mut_ptr(), &mut box_r);
            same("c2BBVerts", &verts_c, &verts_r);

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
                let mut pc: Proxy = poisoned();
                let mut pr: Proxy = poisoned();
                let shape = shape_ptr(typ, &circle, &box_c, &capsule);
                (c.make_proxy)(shape, typ, &mut pc);
                (r.make_proxy)(shape, typ, &mut pr);
                same("c2MakeProxy", &pc, &pr);
            }

            let mut sc = Simplex::default();
            let svs = [&mut sc.a, &mut sc.b, &mut sc.c, &mut sc.d];
            for sv in svs {
                sv.sa = rng.v();
                sv.sb = rng.v();
                sv.p = rng.v();
                sv.u = (rng.u32() % 100 + 1) as f32 / 16.0;
                sv.ia = (rng.u32() % 4) as i32;
                sv.ib = (rng.u32() % 4) as i32;
            }
            sc.div = (rng.u32() % 100 + 1) as f32 / 16.0;

            for count in 1..=3 {
                sc.count = count;
                let mut sr = sc;
                same(
                    "c2GJKSimplexMetric",
                    &(c.metric)(&mut sc),
                    &(r.metric)(&mut sr),
                );
                same("c2D", &(c.direction)(&mut sc), &(r.direction)(&mut sr));
                same("c2L", &(c.closest)(&mut sc), &(r.closest)(&mut sr));
                let (mut ca, mut cb, mut ra, mut rb) =
                    (poisoned(), poisoned(), poisoned(), poisoned());
                (c.witness)(&mut sc, &mut ca, &mut cb);
                (r.witness)(&mut sr, &mut ra, &mut rb);
                same("c2Witness-a", &ca, &ra);
                same("c2Witness-b", &cb, &rb);
            }

            let count = (rng.u32() % 8 + 1) as usize;
            let mut support_verts = [V::default(); 8];
            for v in &mut support_verts[..count] {
                *v = rng.v();
            }
            same(
                "c2Support",
                &(c.support)(support_verts.as_ptr(), count as i32, a),
                &(r.support)(support_verts.as_ptr(), count as i32, a),
            );

            let mut s2c = sc;
            s2c.count = 2;
            let mut s2r = s2c;
            (c.solve2)(&mut s2c);
            (r.solve2)(&mut s2r);
            same("c22", &s2c, &s2r);

            let mut s3c = sc;
            s3c.count = 3;
            let mut s3r = s3c;
            (c.solve3)(&mut s3c);
            (r.solve3)(&mut s3r);
            same("c23", &s3c, &s3r);
        }
    }
}

#[test]
fn simplex_branch_matrix_matches() {
    let (c, r) = libraries();
    unsafe {
        let solve2_cases = [
            ([V { x: 1.0, y: 0.0 }, V { x: 2.0, y: 0.0 }], 1),
            ([V { x: 2.0, y: 0.0 }, V { x: 1.0, y: 0.0 }], 1),
            ([V { x: -1.0, y: 0.0 }, V { x: 1.0, y: 0.0 }], 2),
        ];
        for (points, expected_count) in solve2_cases {
            let mut sc = simplex([points[0], points[1], V::default()]);
            sc.count = 2;
            let mut sr = sc;
            (c.solve2)(&mut sc);
            (r.solve2)(&mut sr);
            same("c22 directed branch", &sc, &sr);
            assert_eq!(sc.count, expected_count);
        }

        let solve3_cases = [
            (
                [
                    V { x: 2.0, y: 0.0 },
                    V { x: 3.0, y: 1.0 },
                    V { x: 3.0, y: -1.0 },
                ],
                1,
                10,
            ),
            (
                [
                    V { x: 3.0, y: 1.0 },
                    V { x: 2.0, y: 0.0 },
                    V { x: 3.0, y: -1.0 },
                ],
                1,
                20,
            ),
            (
                [
                    V { x: 3.0, y: 1.0 },
                    V { x: 3.0, y: -1.0 },
                    V { x: 2.0, y: 0.0 },
                ],
                1,
                30,
            ),
            (
                [
                    V { x: -1.0, y: 2.0 },
                    V { x: 1.0, y: 2.0 },
                    V { x: 0.0, y: 4.0 },
                ],
                2,
                10,
            ),
            (
                [
                    V { x: 0.0, y: 4.0 },
                    V { x: -1.0, y: 2.0 },
                    V { x: 1.0, y: 2.0 },
                ],
                2,
                20,
            ),
            (
                [
                    V { x: 1.0, y: 2.0 },
                    V { x: 0.0, y: 4.0 },
                    V { x: -1.0, y: 2.0 },
                ],
                2,
                30,
            ),
            (
                [
                    V { x: -1.0, y: -1.0 },
                    V { x: 1.0, y: -1.0 },
                    V { x: 0.0, y: 1.0 },
                ],
                3,
                10,
            ),
        ];
        for (points, expected_count, expected_first) in solve3_cases {
            let mut sc = simplex(points);
            let mut sr = sc;
            (c.solve3)(&mut sc);
            (r.solve3)(&mut sr);
            same("c23 directed branch", &sc, &sr);
            assert_eq!(sc.count, expected_count);
            assert_eq!(sc.a.ia, expected_first);
        }
    }
}

#[test]
fn deterministic_invalid_and_boundary_paths_match() {
    let (c, r) = libraries();
    unsafe {
        let circle = Circle {
            p: V { x: 2.0, y: -3.0 },
            r: 4.0,
        };
        for invalid in [-1, 3] {
            let mut pc: Proxy = poisoned();
            let mut pr: Proxy = poisoned();
            (c.make_proxy)((&circle as *const Circle).cast(), invalid, &mut pc);
            (r.make_proxy)((&circle as *const Circle).cast(), invalid, &mut pr);
            same("invalid c2MakeProxy enum", &pc, &pr);
            assert!(bytes(&pc).iter().all(|b| *b == 0xa5));
        }

        for count in [0, 4] {
            let mut sc = simplex([V::default(); 3]);
            sc.count = count;
            let mut sr = sc;
            same(
                "invalid metric count",
                &(c.metric)(&mut sc),
                &(r.metric)(&mut sr),
            );
            same(
                "invalid direction count",
                &(c.direction)(&mut sc),
                &(r.direction)(&mut sr),
            );
            same(
                "invalid closest count",
                &(c.closest)(&mut sc),
                &(r.closest)(&mut sr),
            );
            let (mut ca, mut cb, mut ra, mut rb) = (poisoned(), poisoned(), poisoned(), poisoned());
            (c.witness)(&mut sc, &mut ca, &mut cb);
            (r.witness)(&mut sr, &mut ra, &mut rb);
            same("invalid witness a", &ca, &ra);
            same("invalid witness b", &cb, &rb);
        }

        let mut count3c = simplex([V::default(); 3]);
        let mut count3r = count3c;
        same(
            "c2L count three default",
            &(c.closest)(&mut count3c),
            &(r.closest)(&mut count3r),
        );

        let one = [V { x: 3.0, y: 4.0 }];
        for count in [0, -1] {
            same(
                "nonpositive support count",
                &(c.support)(one.as_ptr(), count, V { x: 1.0, y: 1.0 }),
                &(r.support)(one.as_ptr(), count, V { x: 1.0, y: 1.0 }),
            );
        }
        let nine = [
            V { x: 0.0, y: 0.0 },
            V { x: 1.0, y: 0.0 },
            V { x: 2.0, y: 0.0 },
            V { x: 3.0, y: 0.0 },
            V { x: 4.0, y: 0.0 },
            V { x: 5.0, y: 0.0 },
            V { x: 6.0, y: 0.0 },
            V { x: 7.0, y: 0.0 },
            V { x: 8.0, y: 0.0 },
        ];
        same(
            "oversized support count",
            &(c.support)(nine.as_ptr(), 9, V { x: 1.0, y: 0.0 }),
            &(r.support)(nine.as_ptr(), 9, V { x: 1.0, y: 0.0 }),
        );

        for zero in [0.0f32, -0.0] {
            same(
                "division by signed zero",
                &(c.div)(V { x: 0.0, y: 1.0 }, zero),
                &(r.div)(V { x: 0.0, y: 1.0 }, zero),
            );
        }
        same(
            "normalize zero",
            &(c.norm)(V::default()),
            &(r.norm)(V::default()),
        );

        for type_a in [CAPSULE, CIRCLE, AABB] {
            for invalid_b in [-1, 3] {
                same(
                    "invalid inner collision enum",
                    &(c.collided)(std::ptr::null(), type_a, std::ptr::null(), invalid_b),
                    &(r.collided)(std::ptr::null(), type_a, std::ptr::null(), invalid_b),
                );
            }
        }
        for invalid_a in [-1, 3] {
            same(
                "invalid outer collision enum",
                &(c.collided)(std::ptr::null(), invalid_a, std::ptr::null(), CIRCLE),
                &(r.collided)(std::ptr::null(), invalid_a, std::ptr::null(), CIRCLE),
            );
        }
    }
}

#[test]
fn gjk_collision_and_public_pipeline_match() {
    let (c, r) = libraries();
    let mut rng = Rng::new();

    unsafe {
        for _ in 0..1_500 {
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
            let xa = X {
                p: rng.v(),
                r: R {
                    c: rng.f() / 128.0,
                    s: rng.f() / 128.0,
                },
            };
            let xb = X {
                p: rng.v(),
                r: R {
                    c: rng.f() / 128.0,
                    s: rng.f() / 128.0,
                },
            };

            same(
                "c2AABBtoAABB",
                &(c.aabb_aabb)(aabb_a, aabb_b),
                &(r.aabb_aabb)(aabb_a, aabb_b),
            );
            same(
                "c2AABBtoCapsule",
                &(c.aabb_capsule)(aabb_a, capsule_b),
                &(r.aabb_capsule)(aabb_a, capsule_b),
            );
            same(
                "c2CapsuletoCapsule",
                &(c.capsule_capsule)(capsule_a, capsule_b),
                &(r.capsule_capsule)(capsule_a, capsule_b),
            );
            same(
                "c2CircletoCircle",
                &(c.circle_circle)(circle_a, circle_b),
                &(r.circle_circle)(circle_a, circle_b),
            );
            same(
                "c2CircletoAABB",
                &(c.circle_aabb)(circle_a, aabb_b),
                &(r.circle_aabb)(circle_a, aabb_b),
            );
            same(
                "c2CircletoCapsule",
                &(c.circle_capsule)(circle_a, capsule_b),
                &(r.circle_capsule)(circle_a, capsule_b),
            );

            for ta in [CAPSULE, CIRCLE, AABB] {
                for tb in [CAPSULE, CIRCLE, AABB] {
                    let sa = shape_ptr(ta, &circle_a, &aabb_a, &capsule_a);
                    let sb = shape_ptr(tb, &circle_b, &aabb_b, &capsule_b);
                    same(
                        "c2Collided",
                        &(c.collided)(sa, ta, sb, tb),
                        &(r.collided)(sa, ta, sb, tb),
                    );

                    for use_radius in [0, 1] {
                        let (mut coa, mut cob, mut roa, mut rob) =
                            (poisoned(), poisoned(), poisoned(), poisoned());
                        let (mut ci, mut ri) = (-99, -99);
                        let cd = (c.gjk)(
                            sa,
                            ta,
                            std::ptr::null(),
                            sb,
                            tb,
                            std::ptr::null(),
                            &mut coa,
                            &mut cob,
                            use_radius,
                            &mut ci,
                            std::ptr::null_mut(),
                        );
                        let rd = (r.gjk)(
                            sa,
                            ta,
                            std::ptr::null(),
                            sb,
                            tb,
                            std::ptr::null(),
                            &mut roa,
                            &mut rob,
                            use_radius,
                            &mut ri,
                            std::ptr::null_mut(),
                        );
                        same("c2GJK distance", &cd, &rd);
                        same("c2GJK outA", &coa, &roa);
                        same("c2GJK outB", &cob, &rob);
                        same("c2GJK iterations", &ci, &ri);
                    }

                    let (mut coa, mut cob, mut roa, mut rob) =
                        (poisoned(), poisoned(), poisoned(), poisoned());
                    let (mut ci, mut ri) = (-99, -99);
                    let (mut cc, mut rc) = (Cache::default(), Cache::default());
                    let cd = (c.gjk)(
                        sa, ta, &xa, sb, tb, &xb, &mut coa, &mut cob, 1, &mut ci, &mut cc,
                    );
                    let rd = (r.gjk)(
                        sa, ta, &xa, sb, tb, &xb, &mut roa, &mut rob, 1, &mut ri, &mut rc,
                    );
                    same("transformed c2GJK distance", &cd, &rd);
                    same("transformed c2GJK outA", &coa, &roa);
                    same("transformed c2GJK outB", &cob, &rob);
                    same("transformed c2GJK iterations", &ci, &ri);
                    same("transformed c2GJK cache", &cc, &rc);

                    let cd2 = (c.gjk)(
                        sa, ta, &xa, sb, tb, &xb, &mut coa, &mut cob, 0, &mut ci, &mut cc,
                    );
                    let rd2 = (r.gjk)(
                        sa, ta, &xa, sb, tb, &xb, &mut roa, &mut rob, 0, &mut ri, &mut rc,
                    );
                    same("warm-cache c2GJK distance", &cd2, &rd2);
                    same("warm-cache c2GJK outA", &coa, &roa);
                    same("warm-cache c2GJK outB", &cob, &rob);
                    same("warm-cache c2GJK iterations", &ci, &ri);
                    same("warm-cache c2GJK cache", &cc, &rc);

                    let cd_null = (c.gjk)(
                        sa,
                        ta,
                        std::ptr::null(),
                        sb,
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    let rd_null = (r.gjk)(
                        sa,
                        ta,
                        std::ptr::null(),
                        sb,
                        tb,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    same("all-null optional c2GJK outputs", &cd_null, &rd_null);

                    let pa = parts_for(ta, &circle_a, &aabb_a, &capsule_a);
                    let pb = parts_for(tb, &circle_b, &aabb_b, &capsule_b);
                    same(
                        "omni_collide",
                        &(c.omni)(
                            ta, pa[0], pa[1], pa[2], pa[3], pa[4], tb, pb[0], pb[1], pb[2], pb[3],
                            pb[4],
                        ),
                        &(r.omni)(
                            ta, pa[0], pa[1], pa[2], pa[3], pa[4], tb, pb[0], pb[1], pb[2], pb[3],
                            pb[4],
                        ),
                    );
                }
            }

            for typ in [CAPSULE, CIRCLE, AABB] {
                let p = parts_for(typ, &circle_a, &aabb_a, &capsule_a);
                let cp = (c.parts)(typ, p[0], p[1], p[2], p[3], p[4]);
                let rp = (r.parts)(typ, p[0], p[1], p[2], p[3], p[4]);
                assert!(!cp.is_null() && !rp.is_null());
                assert_eq!(
                    slice::from_raw_parts(cp.cast::<u8>(), shape_size(typ)),
                    slice::from_raw_parts(rp.cast::<u8>(), shape_size(typ)),
                    "ptr_from_parts type {typ}"
                );
                free(cp);
                free(rp);
            }
        }
    }
}

#[test]
fn directed_geometry_boundaries_and_options_match() {
    let (c, r) = libraries();
    unsafe {
        let circles = [
            (
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
                Circle {
                    p: V { x: 3.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
                Circle {
                    p: V { x: 2.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: 2.0,
                },
                Circle {
                    p: V { x: 1.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                Circle {
                    p: V { x: 0.0, y: 0.0 },
                    r: -2.0,
                },
                Circle {
                    p: V { x: 1.0, y: 0.0 },
                    r: 1.0,
                },
            ),
        ];
        for (a, b) in circles {
            same(
                "directed circle-circle",
                &(c.circle_circle)(a, b),
                &(r.circle_circle)(a, b),
            );
            for use_radius in [0, 1] {
                let cd = (c.gjk)(
                    (&a as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    (&b as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    use_radius,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                let rd = (r.gjk)(
                    (&a as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    (&b as *const Circle).cast(),
                    CIRCLE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    use_radius,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                same("directed circle GJK", &cd, &rd);
            }
        }

        let boxes = [
            Aabb {
                min: V { x: 0.0, y: 0.0 },
                max: V { x: 2.0, y: 2.0 },
            },
            Aabb {
                min: V { x: 2.0, y: 2.0 },
                max: V { x: 0.0, y: 0.0 },
            },
        ];
        let other_boxes = [
            Aabb {
                min: V { x: 3.0, y: 0.0 },
                max: V { x: 4.0, y: 1.0 },
            },
            Aabb {
                min: V { x: 2.0, y: 0.0 },
                max: V { x: 3.0, y: 1.0 },
            },
            Aabb {
                min: V { x: 1.0, y: 1.0 },
                max: V { x: 3.0, y: 3.0 },
            },
        ];
        for other in other_boxes {
            same(
                "directed AABB-AABB",
                &(c.aabb_aabb)(boxes[0], other),
                &(r.aabb_aabb)(boxes[0], other),
            );
        }
        let capsules = [
            Capsule {
                a: V { x: -3.0, y: 0.0 },
                b: V { x: -2.0, y: 0.0 },
                r: 0.5,
            },
            Capsule {
                a: V { x: 1.0, y: 1.0 },
                b: V { x: 1.0, y: 1.0 },
                r: 0.0,
            },
            Capsule {
                a: V { x: 0.0, y: 1.0 },
                b: V { x: 2.0, y: 1.0 },
                r: 1.0,
            },
        ];
        for box_a in boxes {
            for cap in capsules {
                same(
                    "directed AABB-capsule",
                    &(c.aabb_capsule)(box_a, cap),
                    &(r.aabb_capsule)(box_a, cap),
                );
                let circle = Circle {
                    p: V { x: 1.0, y: 0.0 },
                    r: 1.0,
                };
                same(
                    "directed circle-AABB",
                    &(c.circle_aabb)(circle, box_a),
                    &(r.circle_aabb)(circle, box_a),
                );
                same(
                    "directed circle-capsule",
                    &(c.circle_capsule)(circle, cap),
                    &(r.circle_capsule)(circle, cap),
                );
                let cd = (c.gjk)(
                    (&box_a as *const Aabb).cast(),
                    AABB,
                    std::ptr::null(),
                    (&cap as *const Capsule).cast(),
                    CAPSULE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                let rd = (r.gjk)(
                    (&box_a as *const Aabb).cast(),
                    AABB,
                    std::ptr::null(),
                    (&cap as *const Capsule).cast(),
                    CAPSULE,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                same("directed AABB-capsule GJK", &cd, &rd);
            }
        }
        for a in capsules {
            for b in capsules {
                same(
                    "directed capsule-capsule",
                    &(c.capsule_capsule)(a, b),
                    &(r.capsule_capsule)(a, b),
                );
            }
        }

        let a = circles[0].0;
        let b = circles[0].1;
        let baseline_c = (c.gjk)(
            (&a as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            (&b as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        let baseline_r = (r.gjk)(
            (&a as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            (&b as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        for option in [-1, 1, 2] {
            let cd = (c.gjk)(
                (&a as *const Circle).cast(),
                CIRCLE,
                std::ptr::null(),
                (&b as *const Circle).cast(),
                CIRCLE,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                option,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            let rd = (r.gjk)(
                (&a as *const Circle).cast(),
                CIRCLE,
                std::ptr::null(),
                (&b as *const Circle).cast(),
                CIRCLE,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                option,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            same("noncanonical use_radius", &cd, &rd);
            same("nonzero use_radius C behavior", &cd, &baseline_c);
            same("nonzero use_radius Rust behavior", &rd, &baseline_r);
        }

        let degenerate = Capsule {
            a: V::default(),
            b: V::default(),
            r: 1.0,
        };
        let probe = Circle {
            p: V::default(),
            r: 1.0,
        };
        same(
            "degenerate capsule",
            &(c.circle_capsule)(probe, degenerate),
            &(r.circle_capsule)(probe, degenerate),
        );

        let (mut coa, mut roa) = (poisoned(), poisoned());
        let (mut cc, mut rc) = (Cache::default(), Cache::default());
        let cd = (c.gjk)(
            (&a as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            (&b as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            &mut coa,
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            &mut cc,
        );
        let rd = (r.gjk)(
            (&a as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            (&b as *const Circle).cast(),
            CIRCLE,
            std::ptr::null(),
            &mut roa,
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            &mut rc,
        );
        same("mixed optional GJK distance", &cd, &rd);
        same("mixed optional GJK outA", &coa, &roa);
        same("mixed optional GJK cache", &cc, &rc);
    }
}

#[test]
fn special_float_inputs_match_where_c_behavior_is_defined() {
    let (c, r) = libraries();
    unsafe {
        let values = [
            0.0f32,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_1234),
        ];
        for &x in &values {
            for &y in &values {
                let a = V { x, y };
                let b = V { x: y, y: x };
                same("special c2V", &(c.v)(x, y), &(r.v)(x, y));
                same("special c2Add", &(c.add)(a, b), &(r.add)(a, b));
                same("special c2Sub", &(c.sub)(a, b), &(r.sub)(a, b));
                same("special c2Mulvs", &(c.mulvs)(a, y), &(r.mulvs)(a, y));
                same("special c2Maxv", &(c.maxv)(a, b), &(r.maxv)(a, b));
                same("special c2Minv", &(c.minv)(a, b), &(r.minv)(a, b));
                same("special c2Dot", &(c.dot)(a, b), &(r.dot)(a, b));
                same("special c2Det2", &(c.det2)(a, b), &(r.det2)(a, b));
                same("special c2Neg", &(c.neg)(a), &(r.neg)(a));
                same("special c2Skew", &(c.skew)(a), &(r.skew)(a));
                same("special c2CCW90", &(c.ccw90)(a), &(r.ccw90)(a));
                let circle_a = Circle { p: a, r: x };
                let circle_b = Circle { p: b, r: y };
                let box_b = Aabb { min: a, max: b };
                let capsule_b = Capsule { a, b, r: y };
                same(
                    "special circle-circle",
                    &(c.circle_circle)(circle_a, circle_b),
                    &(r.circle_circle)(circle_a, circle_b),
                );
                same(
                    "special circle-AABB",
                    &(c.circle_aabb)(circle_a, box_b),
                    &(r.circle_aabb)(circle_a, box_b),
                );
                same(
                    "special circle-capsule",
                    &(c.circle_capsule)(circle_a, capsule_b),
                    &(r.circle_capsule)(circle_a, capsule_b),
                );
            }
        }
    }
}
