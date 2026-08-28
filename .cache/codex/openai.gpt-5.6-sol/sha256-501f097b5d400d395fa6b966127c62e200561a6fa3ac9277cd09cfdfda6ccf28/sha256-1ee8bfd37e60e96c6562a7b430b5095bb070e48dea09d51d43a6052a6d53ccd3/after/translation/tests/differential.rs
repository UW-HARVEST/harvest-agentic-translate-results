use libloading::Library;
use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Raycast {
    t: f32,
    n: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Aabb {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Poly {
    count: c_int,
    verts: [C2v; 8],
    norms: [C2v; 8],
}

#[repr(C)]
struct ExtendedPoly {
    poly: C2Poly,
    tail: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Ray {
    p: C2v,
    d: C2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2m {
    x: C2v,
    y: C2v,
}

type V2 = unsafe extern "C" fn(f32, f32) -> C2v;
type VVf = unsafe extern "C" fn(C2v, C2v) -> f32;
type Vf = unsafe extern "C" fn(C2v) -> f32;
type VVV = unsafe extern "C" fn(C2v, C2v) -> C2v;
type VScalar = unsafe extern "C" fn(C2v, f32) -> C2v;
type VV = unsafe extern "C" fn(C2v) -> C2v;
type AabbAabb = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type RayCircle = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
type RayAabb = unsafe extern "C" fn(C2Ray, C2Aabb, *mut C2Raycast) -> c_int;
type MatrixVector = unsafe extern "C" fn(C2m, C2v) -> C2v;
type AabbPoint = unsafe extern "C" fn(C2Aabb, C2v) -> c_int;
type CirclePoint = unsafe extern "C" fn(C2Circle, C2v) -> c_int;
type RayCapsule = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
type RotIdentity = unsafe extern "C" fn() -> C2r;
type XIdentity = unsafe extern "C" fn() -> C2x;
type RotVector = unsafe extern "C" fn(C2r, C2v) -> C2v;
type XVector = unsafe extern "C" fn(C2x, C2v) -> C2v;
type RayPoly = unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int;
type CastRay =
    unsafe extern "C" fn(C2Ray, *const c_void, *const C2x, c_int, *mut C2Raycast) -> c_int;
type PolyRay = unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int;

struct Api {
    _lib: Library,
    v: V2,
    dot: VVf,
    len: Vf,
    add: VVV,
    sub: VVV,
    mulvs: VScalar,
    div: VScalar,
    norm: VV,
    minv: VVV,
    maxv: VVV,
    skew: VV,
    absv: VV,
    ray_circle: RayCircle,
    aabb_aabb: AabbAabb,
    ray_aabb: RayAabb,
    ccw90: VV,
    mulmvt: MatrixVector,
    aabb_point: AabbPoint,
    circle_point: CirclePoint,
    ray_capsule: RayCapsule,
    rot_identity: RotIdentity,
    x_identity: XIdentity,
    mulrv: RotVector,
    mulrvt: RotVector,
    mulxvt: XVector,
    ray_poly: RayPoly,
    cast_ray: CastRay,
    poly_ray: PolyRay,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let symbol = unsafe { lib.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *symbol
            }};
        }
        Self {
            v: sym!("c2V", V2),
            dot: sym!("c2Dot", VVf),
            len: sym!("c2Len", Vf),
            add: sym!("c2Add", VVV),
            sub: sym!("c2Sub", VVV),
            mulvs: sym!("c2Mulvs", VScalar),
            div: sym!("c2Div", VScalar),
            norm: sym!("c2Norm", VV),
            minv: sym!("c2Minv", VVV),
            maxv: sym!("c2Maxv", VVV),
            skew: sym!("c2Skew", VV),
            absv: sym!("c2Absv", VV),
            ray_circle: sym!("c2RaytoCircle", RayCircle),
            aabb_aabb: sym!("c2AABBtoAABB", AabbAabb),
            ray_aabb: sym!("c2RaytoAABB", RayAabb),
            ccw90: sym!("c2CCW90", VV),
            mulmvt: sym!("c2MulmvT", MatrixVector),
            aabb_point: sym!("c2AABBtoPoint", AabbPoint),
            circle_point: sym!("c2CircleToPoint", CirclePoint),
            ray_capsule: sym!("c2RaytoCapsule", RayCapsule),
            rot_identity: sym!("c2RotIdentity", RotIdentity),
            x_identity: sym!("c2xIdentity", XIdentity),
            mulrv: sym!("c2Mulrv", RotVector),
            mulrvt: sym!("c2MulrvT", RotVector),
            mulxvt: sym!("c2MulxvT", XVector),
            ray_poly: sym!("c2RaytoPoly", RayPoly),
            cast_ray: sym!("c2CastRay", CastRay),
            poly_ray: sym!("poly_ray", PolyRay),
            _lib: lib,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
    _libm: UnixLibrary,
}

impl Pair {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-OPYRmD.so");
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let rust_path = root.join(format!("target/{profile}/libpoly_ray_lib.so"));
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        unsafe {
            let libm = UnixLibrary::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
                .expect("failed to load libm.so.6 globally");
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
                _libm: libm,
            }
        }
    }
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
        (x >> 32) as u32
    }

    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn nonzero(&mut self, magnitude: f32) -> f32 {
        let value = self.range(0.125, magnitude);
        if self.next_u32() & 1 == 0 {
            value
        } else {
            -value
        }
    }

    fn vector(&mut self, magnitude: f32) -> C2v {
        C2v {
            x: self.range(-magnitude, magnitude),
            y: self.range(-magnitude, magnitude),
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

fn assert_v(c: C2v, rust: C2v, context: &str) {
    assert_f32(c.x, rust.x, &format!("{context}.x"));
    assert_f32(c.y, rust.y, &format!("{context}.y"));
}

fn assert_rot(c: C2r, rust: C2r, context: &str) {
    assert_f32(c.c, rust.c, &format!("{context}.c"));
    assert_f32(c.s, rust.s, &format!("{context}.s"));
}

fn assert_x(c: C2x, rust: C2x, context: &str) {
    assert_v(c.p, rust.p, &format!("{context}.p"));
    assert_rot(c.r, rust.r, &format!("{context}.r"));
}

fn sentinel() -> C2Raycast {
    C2Raycast {
        t: f32::from_bits(0x42f6_e979),
        n: C2v {
            x: f32::from_bits(0xc2c7_0a3d),
            y: f32::from_bits(0x4121_47ae),
        },
    }
}

fn assert_cast(c_ret: c_int, c: C2Raycast, r_ret: c_int, rust: C2Raycast, context: &str) {
    assert_eq!(c_ret, r_ret, "{context}: return value");
    assert_f32(c.t, rust.t, &format!("{context}.out.t"));
    assert_v(c.n, rust.n, &format!("{context}.out.n"));
}

fn compare_ray_circle(pair: &Pair, ray: C2Ray, shape: C2Circle, context: &str) -> c_int {
    let mut c_out = sentinel();
    let mut r_out = sentinel();
    let c_ret = unsafe { (pair.c.ray_circle)(ray, shape, &mut c_out) };
    let r_ret = unsafe { (pair.rust.ray_circle)(ray, shape, &mut r_out) };
    assert_cast(c_ret, c_out, r_ret, r_out, context);
    c_ret
}

fn compare_ray_aabb(pair: &Pair, ray: C2Ray, shape: C2Aabb, context: &str) -> c_int {
    let mut c_out = sentinel();
    let mut r_out = sentinel();
    let c_ret = unsafe { (pair.c.ray_aabb)(ray, shape, &mut c_out) };
    let r_ret = unsafe { (pair.rust.ray_aabb)(ray, shape, &mut r_out) };
    assert_cast(c_ret, c_out, r_ret, r_out, context);
    c_ret
}

fn compare_ray_capsule(pair: &Pair, ray: C2Ray, shape: C2Capsule, context: &str) -> c_int {
    let mut c_out = sentinel();
    let mut r_out = sentinel();
    let c_ret = unsafe { (pair.c.ray_capsule)(ray, shape, &mut c_out) };
    let r_ret = unsafe { (pair.rust.ray_capsule)(ray, shape, &mut r_out) };
    assert_cast(c_ret, c_out, r_ret, r_out, context);
    c_ret
}

fn empty_poly(count: c_int) -> C2Poly {
    C2Poly {
        count,
        verts: [C2v { x: 0.0, y: 0.0 }; 8],
        norms: [C2v { x: 0.0, y: 0.0 }; 8],
    }
}

fn square_poly(count: c_int, half: f32) -> C2Poly {
    let mut poly = empty_poly(count);
    let verts = [
        C2v { x: half, y: -half },
        C2v { x: half, y: half },
        C2v { x: -half, y: half },
        C2v { x: -half, y: -half },
    ];
    let norms = [
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 0.0, y: 1.0 },
        C2v { x: -1.0, y: 0.0 },
        C2v { x: 0.0, y: -1.0 },
    ];
    for i in 0..8 {
        poly.verts[i] = verts[i % 4];
        poly.norms[i] = norms[i % 4];
    }
    poly
}

fn compare_ray_poly(
    pair: &Pair,
    ray: C2Ray,
    poly: &C2Poly,
    transform: Option<&C2x>,
    context: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut r_out = sentinel();
    let bx = transform.map_or(std::ptr::null(), std::ptr::from_ref);
    let c_ret = unsafe { (pair.c.ray_poly)(ray, poly, bx, &mut c_out) };
    let r_ret = unsafe { (pair.rust.ray_poly)(ray, poly, bx, &mut r_out) };
    assert_cast(c_ret, c_out, r_ret, r_out, context);
    c_ret
}

fn compare_cast_ray(
    pair: &Pair,
    ray: C2Ray,
    shape: *const c_void,
    transform: *const C2x,
    shape_type: c_int,
    context: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut r_out = sentinel();
    let c_ret = unsafe { (pair.c.cast_ray)(ray, shape, transform, shape_type, &mut c_out) };
    let r_ret = unsafe { (pair.rust.cast_ray)(ray, shape, transform, shape_type, &mut r_out) };
    assert_cast(c_ret, c_out, r_ret, r_out, context);
    c_ret
}

#[test]
fn configs_001_to_024_vector_math() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x94d0_49bb_1331_11eb);

    for iteration in 0..256 {
        let a = rng.vector(100.0);
        let b = rng.vector(100.0);
        let context = format!("vector iteration {iteration}");
        unsafe {
            assert_v((pair.c.v)(a.x, a.y), (pair.rust.v)(a.x, a.y), &context);
            assert_f32((pair.c.dot)(a, b), (pair.rust.dot)(a, b), &context);
            assert_v((pair.c.add)(a, b), (pair.rust.add)(a, b), &context);
            assert_v((pair.c.sub)(a, b), (pair.rust.sub)(a, b), &context);

            let scalar = rng.nonzero(20.0);
            assert_v(
                (pair.c.mulvs)(a, scalar),
                (pair.rust.mulvs)(a, scalar),
                &context,
            );
            assert_v((pair.c.mulvs)(a, 0.0), (pair.rust.mulvs)(a, 0.0), &context);
            assert_v(
                (pair.c.div)(a, scalar),
                (pair.rust.div)(a, scalar),
                &context,
            );
            assert_v((pair.c.div)(a, 0.0), (pair.rust.div)(a, 0.0), &context);
            assert_f32((pair.c.len)(a), (pair.rust.len)(a), &context);
            assert_v((pair.c.norm)(a), (pair.rust.norm)(a), &context);
            assert_v((pair.c.skew)(a), (pair.rust.skew)(a), &context);
            assert_v((pair.c.ccw90)(a), (pair.rust.ccw90)(a), &context);
        }
    }

    let zero = C2v { x: 0.0, y: 0.0 };
    unsafe {
        assert_f32((pair.c.len)(zero), (pair.rust.len)(zero), "zero length");
        assert_v((pair.c.norm)(zero), (pair.rust.norm)(zero), "zero norm");
        assert_v(
            (pair.c.div)(zero, 0.0),
            (pair.rust.div)(zero, 0.0),
            "zero/zero",
        );
    }

    let orderings = [
        (C2v { x: -3.0, y: -4.0 }, C2v { x: 2.0, y: 5.0 }),
        (C2v { x: -3.0, y: 6.0 }, C2v { x: 2.0, y: 5.0 }),
        (C2v { x: 3.0, y: -4.0 }, C2v { x: 2.0, y: 5.0 }),
        (C2v { x: 3.0, y: 6.0 }, C2v { x: 2.0, y: 5.0 }),
        (C2v { x: 2.0, y: 5.0 }, C2v { x: 2.0, y: 5.0 }),
    ];
    for (i, (a, b)) in orderings.into_iter().enumerate() {
        unsafe {
            assert_v(
                (pair.c.minv)(a, b),
                (pair.rust.minv)(a, b),
                &format!("min {i}"),
            );
            assert_v(
                (pair.c.maxv)(a, b),
                (pair.rust.maxv)(a, b),
                &format!("max {i}"),
            );
        }
    }

    let signs = [
        C2v { x: 3.0, y: 4.0 },
        C2v { x: -3.0, y: 4.0 },
        C2v { x: 3.0, y: -4.0 },
        C2v { x: -3.0, y: -4.0 },
        C2v { x: 0.0, y: -0.0 },
    ];
    for (i, value) in signs.into_iter().enumerate() {
        unsafe {
            assert_v(
                (pair.c.absv)(value),
                (pair.rust.absv)(value),
                &format!("abs {i}"),
            );
        }
    }
}

#[test]
fn configs_025_to_033_box_and_point_predicates() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xe465_1724_238b_74be);

    for iteration in 0..128 {
        let x = rng.range(-50.0, 50.0);
        let y = rng.range(-50.0, 50.0);
        let width = rng.range(0.5, 10.0);
        let height = rng.range(0.5, 10.0);
        let a = C2Aabb {
            min: C2v { x, y },
            max: C2v {
                x: x + width,
                y: y + height,
            },
        };
        let overlap = C2Aabb {
            min: C2v {
                x: x + width * 0.25,
                y: y + height * 0.25,
            },
            max: C2v {
                x: x + width * 1.25,
                y: y + height * 1.25,
            },
        };
        let touching = C2Aabb {
            min: C2v {
                x: a.max.x,
                y: a.min.y,
            },
            max: C2v {
                x: a.max.x + width,
                y: a.max.y,
            },
        };
        let separated = [
            C2Aabb {
                min: C2v {
                    x: x - width * 2.0,
                    y,
                },
                max: C2v {
                    x: x - width,
                    y: a.max.y,
                },
            },
            C2Aabb {
                min: C2v {
                    x: a.max.x + width,
                    y,
                },
                max: C2v {
                    x: a.max.x + width * 2.0,
                    y: a.max.y,
                },
            },
            C2Aabb {
                min: C2v {
                    x,
                    y: y - height * 2.0,
                },
                max: C2v {
                    x: a.max.x,
                    y: y - height,
                },
            },
            C2Aabb {
                min: C2v {
                    x,
                    y: a.max.y + height,
                },
                max: C2v {
                    x: a.max.x,
                    y: a.max.y + height * 2.0,
                },
            },
        ];
        unsafe {
            assert_eq!(
                (pair.c.aabb_aabb)(a, overlap),
                (pair.rust.aabb_aabb)(a, overlap),
                "overlap {iteration}"
            );
            assert_eq!(
                (pair.c.aabb_aabb)(a, touching),
                (pair.rust.aabb_aabb)(a, touching),
                "touching {iteration}"
            );
            for (side, b) in separated.into_iter().enumerate() {
                assert_eq!(
                    (pair.c.aabb_aabb)(a, b),
                    (pair.rust.aabb_aabb)(a, b),
                    "separated {iteration}/{side}"
                );
            }
        }

        let points = [
            C2v {
                x: x + width * 0.5,
                y: y + height * 0.5,
            },
            C2v { x, y },
            C2v {
                x: x - 1.0,
                y: y + height * 0.5,
            },
            C2v {
                x: a.max.x + 1.0,
                y: y + height * 0.5,
            },
            C2v {
                x: x + width * 0.5,
                y: y - 1.0,
            },
            C2v {
                x: x + width * 0.5,
                y: a.max.y + 1.0,
            },
        ];
        for (shape, point) in points.into_iter().enumerate() {
            unsafe {
                assert_eq!(
                    (pair.c.aabb_point)(a, point),
                    (pair.rust.aabb_point)(a, point),
                    "point {iteration}/{shape}"
                );
            }
        }

        let radius = rng.range(0.5, 8.0);
        let circle = C2Circle {
            p: C2v { x, y },
            r: radius,
        };
        let circle_points = [
            C2v { x, y },
            C2v { x: x + radius, y },
            C2v {
                x: x + radius + 0.25,
                y,
            },
        ];
        for (shape, point) in circle_points.into_iter().enumerate() {
            unsafe {
                assert_eq!(
                    (pair.c.circle_point)(circle, point),
                    (pair.rust.circle_point)(circle, point),
                    "circle point {iteration}/{shape}"
                );
            }
        }
    }
}

#[test]
fn configs_034_to_039_ray_circle() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x4841_5633_4197_1165);

    for iteration in 0..192 {
        let radius = ((rng.next_u32() % 8) + 1) as f32;
        let distance = ((rng.next_u32() % 8) + 1) as f32;
        let circle = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: radius,
        };
        let secant = C2Ray {
            p: C2v {
                x: -(radius + distance),
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance + radius * 2.0 + 1.0,
        };
        assert_eq!(
            compare_ray_circle(&pair, secant, circle, &format!("secant {iteration}")),
            1
        );

        let tangent_start = ((rng.next_u32() % 8) + 2) as f32;
        let tangent = C2Ray {
            p: C2v {
                x: -tangent_start,
                y: radius,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: tangent_start + 1.0,
        };
        assert_eq!(
            compare_ray_circle(&pair, tangent, circle, &format!("tangent {iteration}")),
            1
        );

        let boundary = C2Ray {
            p: C2v { x: radius, y: 0.0 },
            d: C2v { x: -1.0, y: 0.0 },
            t: radius * 2.0,
        };
        assert_eq!(
            compare_ray_circle(&pair, boundary, circle, &format!("boundary {iteration}")),
            1
        );

        let no_real_root = C2Ray {
            p: C2v {
                x: -distance,
                y: radius + 1.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance * 2.0,
        };
        assert_eq!(
            compare_ray_circle(
                &pair,
                no_real_root,
                circle,
                &format!("negative discriminant {iteration}")
            ),
            0
        );

        let behind = C2Ray {
            p: C2v {
                x: radius + distance,
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance + radius,
        };
        assert_eq!(
            compare_ray_circle(&pair, behind, circle, &format!("behind {iteration}")),
            0
        );

        let too_short = C2Ray {
            p: secant.p,
            d: secant.d,
            t: distance * 0.5,
        };
        assert_eq!(
            compare_ray_circle(&pair, too_short, circle, &format!("short {iteration}")),
            0
        );
    }
}

#[test]
fn configs_040_to_046_ray_aabb() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xd6e8_feb8_6659_fd93);

    for iteration in 0..192 {
        let tx = rng.range(-40.0, 40.0);
        let ty = rng.range(-40.0, 40.0);
        let box_shape = C2Aabb {
            min: C2v { x: tx, y: ty },
            max: C2v {
                x: tx + 2.0,
                y: ty + 2.0,
            },
        };

        let broad_miss = C2Ray {
            p: C2v {
                x: tx - 4.0,
                y: ty + 4.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: 1.0,
        };
        assert_eq!(
            compare_ray_aabb(
                &pair,
                broad_miss,
                box_shape,
                &format!("broad miss {iteration}")
            ),
            0
        );

        let sat_miss = C2Ray {
            p: C2v {
                x: tx - 1.0,
                y: ty + 1.8,
            },
            d: C2v { x: 1.1, y: 1.1 },
            t: 1.0,
        };
        assert_eq!(
            compare_ray_aabb(&pair, sat_miss, box_shape, &format!("sat miss {iteration}")),
            0
        );

        let plane_cases = [
            (
                C2Ray {
                    p: C2v {
                        x: tx - 2.0,
                        y: ty + 1.0,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 5.0,
                },
                "min-x",
            ),
            (
                C2Ray {
                    p: C2v {
                        x: tx + 4.0,
                        y: ty + 1.0,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: 5.0,
                },
                "max-x",
            ),
            (
                C2Ray {
                    p: C2v {
                        x: tx + 1.0,
                        y: ty - 2.0,
                    },
                    d: C2v { x: 0.0, y: 1.0 },
                    t: 5.0,
                },
                "min-y",
            ),
            (
                C2Ray {
                    p: C2v {
                        x: tx + 1.0,
                        y: ty + 4.0,
                    },
                    d: C2v { x: 0.0, y: -1.0 },
                    t: 5.0,
                },
                "max-y",
            ),
        ];
        for (ray, plane) in plane_cases {
            assert_eq!(
                compare_ray_aabb(&pair, ray, box_shape, &format!("{plane} hit {iteration}")),
                1
            );
        }

        let inside = C2Ray {
            p: C2v {
                x: tx + 1.0,
                y: ty + 1.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: 1.0,
        };
        assert_eq!(
            compare_ray_aabb(&pair, inside, box_shape, &format!("inside {iteration}")),
            1
        );
        let zero_length = C2Ray {
            d: C2v { x: 0.0, y: 0.0 },
            t: 0.0,
            ..inside
        };
        assert_eq!(
            compare_ray_aabb(
                &pair,
                zero_length,
                box_shape,
                &format!("zero length {iteration}")
            ),
            1
        );
    }
}

#[test]
fn configs_047_and_058_to_060_matrix_and_transforms() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0c13_001f_e1ed_093d);

    unsafe {
        assert_rot(
            (pair.c.rot_identity)(),
            (pair.rust.rot_identity)(),
            "rotation identity",
        );
        assert_x(
            (pair.c.x_identity)(),
            (pair.rust.x_identity)(),
            "transform identity",
        );
    }

    for iteration in 0..256 {
        let matrix = C2m {
            x: rng.vector(20.0),
            y: rng.vector(20.0),
        };
        let vector = rng.vector(20.0);
        let rotation = C2r {
            c: rng.range(-2.0, 2.0),
            s: rng.range(-2.0, 2.0),
        };
        let transform = C2x {
            p: rng.vector(20.0),
            r: rotation,
        };
        let context = format!("matrix/transform {iteration}");
        unsafe {
            assert_v(
                (pair.c.mulmvt)(matrix, vector),
                (pair.rust.mulmvt)(matrix, vector),
                &format!("{context} mulmvt"),
            );
            assert_v(
                (pair.c.mulrv)(rotation, vector),
                (pair.rust.mulrv)(rotation, vector),
                &format!("{context} mulrv"),
            );
            assert_v(
                (pair.c.mulrvt)(rotation, vector),
                (pair.rust.mulrvt)(rotation, vector),
                &format!("{context} mulrvt"),
            );
            assert_v(
                (pair.c.mulxvt)(transform, vector),
                (pair.rust.mulxvt)(transform, vector),
                &format!("{context} mulxvt"),
            );
        }
    }
}

#[test]
fn configs_048_to_057_ray_capsule() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x2d35_8dcc_aa6c_78a5);

    for iteration in 0..160 {
        let tx = rng.range(-25.0, 25.0);
        let ty = rng.range(-25.0, 25.0);
        let radius = rng.range(0.5, 3.0);
        let length = rng.range(3.0, 10.0);
        let capsule = C2Capsule {
            a: C2v { x: tx, y: ty },
            b: C2v {
                x: tx,
                y: ty + length,
            },
            r: radius,
        };
        let ray = |x: f32, y: f32, dx: f32, dy: f32, t: f32| C2Ray {
            p: C2v {
                x: tx + x,
                y: ty + y,
            },
            d: C2v { x: dx, y: dy },
            t,
        };

        let core = ray(0.0, length * 0.5, 1.0, 0.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(&pair, core, capsule, &format!("core {iteration}")),
            1
        );

        let cap_a = ray(0.0, -radius * 0.5, 0.0, 1.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(&pair, cap_a, capsule, &format!("cap A start {iteration}")),
            1
        );

        let cap_b = ray(0.0, length + radius * 0.5, 0.0, -1.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(&pair, cap_b, capsule, &format!("cap B start {iteration}")),
            1
        );

        let miss = ray(radius * 3.0, length * 0.5, 0.0, 1.0, radius * 2.0);
        assert_eq!(
            compare_ray_capsule(&pair, miss, capsule, &format!("final miss {iteration}")),
            0
        );

        let strip_a = ray(radius * 0.75, -radius * 0.75, 0.0, 1.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(&pair, strip_a, capsule, &format!("strip cap A {iteration}")),
            1
        );

        let strip_b = ray(
            radius * 0.75,
            length + radius * 0.75,
            0.0,
            -1.0,
            radius * 4.0,
        );
        assert_eq!(
            compare_ray_capsule(&pair, strip_b, capsule, &format!("strip cap B {iteration}")),
            1
        );

        let crossing_a = ray(radius * 2.0, -radius * 2.0, -1.0, 0.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(
                &pair,
                crossing_a,
                capsule,
                &format!("crossing cap A {iteration}")
            ),
            0
        );

        let crossing_b = ray(radius * 2.0, length + radius * 2.0, -1.0, 0.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(
                &pair,
                crossing_b,
                capsule,
                &format!("crossing cap B {iteration}")
            ),
            0
        );

        let right = ray(radius * 2.0, length * 0.5, -1.0, 0.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(&pair, right, capsule, &format!("right side {iteration}")),
            1
        );

        let left = ray(-radius * 2.0, length * 0.5, 1.0, 0.0, radius * 4.0);
        assert_eq!(
            compare_ray_capsule(&pair, left, capsule, &format!("left side {iteration}")),
            1
        );
    }
}

#[test]
fn configs_061_to_072_ray_poly() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xc6bc_2796_92b5_cc83);

    for iteration in 0..160 {
        let half = rng.range(0.5, 4.0);
        let start = half + rng.range(1.0, 5.0);
        let long_ray = C2Ray {
            p: C2v { x: -start, y: 0.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: start + half + 2.0,
        };

        let empty = empty_poly(0);
        assert_eq!(
            compare_ray_poly(&pair, long_ray, &empty, None, &format!("empty {iteration}")),
            0
        );
        let negative = empty_poly(-1 - (rng.next_u32() % 16) as c_int);
        assert_eq!(
            compare_ray_poly(
                &pair,
                long_ray,
                &negative,
                None,
                &format!("negative count {iteration}")
            ),
            0
        );

        let mut one_plane = empty_poly(1);
        one_plane.verts[0] = C2v { x: half, y: 0.0 };
        one_plane.norms[0] = C2v { x: 1.0, y: 0.0 };
        let from_right = C2Ray {
            p: C2v {
                x: half + 2.0,
                y: 0.0,
            },
            d: C2v { x: -1.0, y: 0.0 },
            t: 4.0,
        };
        assert_eq!(
            compare_ray_poly(
                &pair,
                from_right,
                &one_plane,
                None,
                &format!("one plane {iteration}")
            ),
            1
        );

        for count in 3..=7 {
            let poly = square_poly(count, half);
            assert_eq!(
                compare_ray_poly(
                    &pair,
                    long_ray,
                    &poly,
                    None,
                    &format!("count {count}, iteration {iteration}")
                ),
                1
            );
        }
        let maximum = square_poly(8, half);
        assert_eq!(
            compare_ray_poly(
                &pair,
                long_ray,
                &maximum,
                None,
                &format!("maximum count {iteration}")
            ),
            1
        );

        let identity = C2x {
            p: C2v { x: 0.0, y: 0.0 },
            r: C2r { c: 1.0, s: 0.0 },
        };
        assert_eq!(
            compare_ray_poly(
                &pair,
                long_ray,
                &maximum,
                Some(&identity),
                &format!("explicit identity {iteration}")
            ),
            1
        );

        let transform = C2x {
            p: C2v {
                x: rng.range(-20.0, 20.0),
                y: rng.range(-20.0, 20.0),
            },
            r: C2r { c: 0.0, s: 1.0 },
        };
        let transformed_ray = C2Ray {
            p: C2v {
                x: transform.p.x,
                y: transform.p.y - start,
            },
            d: C2v { x: 0.0, y: 1.0 },
            t: long_ray.t,
        };
        assert_eq!(
            compare_ray_poly(
                &pair,
                transformed_ray,
                &maximum,
                Some(&transform),
                &format!("transformed {iteration}")
            ),
            1
        );

        let square = square_poly(4, half);
        let parallel_outside = C2Ray {
            p: C2v {
                x: -start,
                y: half + 1.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: long_ray.t,
        };
        assert_eq!(
            compare_ray_poly(
                &pair,
                parallel_outside,
                &square,
                None,
                &format!("parallel outside {iteration}")
            ),
            0
        );

        // The full crossing updates both the entering (lo/index) and exiting (hi) planes.
        assert_eq!(
            compare_ray_poly(
                &pair,
                long_ray,
                &square,
                None,
                &format!("enter/exit {iteration}")
            ),
            1
        );

        let short_ray = C2Ray {
            t: (start - half) * 0.5,
            ..long_ray
        };
        assert_eq!(
            compare_ray_poly(
                &pair,
                short_ray,
                &square,
                None,
                &format!("empty interval {iteration}")
            ),
            0
        );

        let inside = C2Ray {
            p: C2v { x: 0.0, y: 0.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: half * 2.0,
        };
        assert_eq!(
            compare_ray_poly(
                &pair,
                inside,
                &square,
                None,
                &format!("no entering plane {iteration}")
            ),
            0
        );
    }
}

#[test]
fn configs_073_to_078_dispatch_and_top_level() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);

    for iteration in 0..128 {
        let ray = C2Ray {
            p: C2v { x: -5.0, y: 0.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        let circle = C2Circle {
            p: C2v {
                x: rng.range(-1.0, 1.0),
                y: 0.0,
            },
            r: rng.range(0.5, 2.0),
        };
        compare_cast_ray(
            &pair,
            ray,
            std::ptr::from_ref(&circle).cast(),
            std::ptr::null(),
            0,
            &format!("dispatch circle {iteration}"),
        );

        let aabb = C2Aabb {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        compare_cast_ray(
            &pair,
            ray,
            std::ptr::from_ref(&aabb).cast(),
            std::ptr::null(),
            1,
            &format!("dispatch aabb {iteration}"),
        );

        let capsule = C2Capsule {
            a: C2v { x: 0.0, y: -2.0 },
            b: C2v { x: 0.0, y: 2.0 },
            r: 1.0,
        };
        compare_cast_ray(
            &pair,
            ray,
            std::ptr::from_ref(&capsule).cast(),
            std::ptr::null(),
            2,
            &format!("dispatch capsule {iteration}"),
        );

        let poly = square_poly(4, 1.0);
        compare_cast_ray(
            &pair,
            ray,
            std::ptr::from_ref(&poly).cast(),
            std::ptr::null(),
            3,
            &format!("dispatch poly null transform {iteration}"),
        );
        let transform = C2x {
            p: C2v {
                x: rng.range(-1.0, 1.0),
                y: 0.0,
            },
            r: C2r { c: 1.0, s: 0.0 },
        };
        compare_cast_ray(
            &pair,
            ray,
            std::ptr::from_ref(&poly).cast(),
            std::ptr::from_ref(&transform),
            3,
            &format!("dispatch poly transform {iteration}"),
        );

        for invalid in [-1, 4, 17, c_int::MIN, c_int::MAX] {
            assert_eq!(
                compare_cast_ray(
                    &pair,
                    ray,
                    std::ptr::null(),
                    std::ptr::null(),
                    invalid,
                    &format!("invalid enum {invalid}, iteration {iteration}")
                ),
                0
            );
        }
    }

    for iteration in 0..64 {
        let mut c_first = sentinel();
        let mut c_second = sentinel();
        let mut r_first = sentinel();
        let mut r_second = sentinel();
        let c_ret = unsafe { (pair.c.poly_ray)(&mut c_first, &mut c_second) };
        let r_ret = unsafe { (pair.rust.poly_ray)(&mut r_first, &mut r_second) };
        assert_eq!(c_ret, r_ret, "poly_ray return {iteration}");
        assert_cast(
            c_ret,
            c_first,
            r_ret,
            r_first,
            &format!("poly_ray first {iteration}"),
        );
        assert_cast(
            c_ret,
            c_second,
            r_ret,
            r_second,
            &format!("poly_ray second {iteration}"),
        );
    }
}

#[test]
fn errors_001_to_020_exact_rejection_paths() {
    let pair = Pair::load();

    for iteration in 0..64 {
        let radius = ((iteration % 8) + 1) as f32;
        let circle = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: radius,
        };

        // ERRORS 1: negative discriminant.
        let negative_disc = C2Ray {
            p: C2v {
                x: -4.0,
                y: radius + 1.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        assert_eq!(
            compare_ray_circle(
                &pair,
                negative_disc,
                circle,
                &format!("E01 iteration {iteration}")
            ),
            0
        );

        // ERRORS 2: nearest root is behind the ray start.
        let behind = C2Ray {
            p: C2v {
                x: radius + 2.0,
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        assert_eq!(
            compare_ray_circle(&pair, behind, circle, &format!("E02 iteration {iteration}")),
            0
        );

        // ERRORS 3: nearest root is beyond A.t.
        let beyond = C2Ray {
            p: C2v {
                x: -(radius + 2.0),
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: 1.0,
        };
        assert_eq!(
            compare_ray_circle(&pair, beyond, circle, &format!("E03 iteration {iteration}")),
            0
        );
    }

    let a = C2Aabb {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 2.0, y: 2.0 },
    };
    let separated = [
        C2Aabb {
            min: C2v { x: -4.0, y: 0.0 },
            max: C2v { x: -1.0, y: 2.0 },
        },
        C2Aabb {
            min: C2v { x: 3.0, y: 0.0 },
            max: C2v { x: 4.0, y: 2.0 },
        },
        C2Aabb {
            min: C2v { x: 0.0, y: -4.0 },
            max: C2v { x: 2.0, y: -1.0 },
        },
        C2Aabb {
            min: C2v { x: 0.0, y: 3.0 },
            max: C2v { x: 2.0, y: 4.0 },
        },
    ];
    for (offset, b) in separated.into_iter().enumerate() {
        unsafe {
            let c = (pair.c.aabb_aabb)(a, b);
            let rust = (pair.rust.aabb_aabb)(a, b);
            assert_eq!(c, 0, "E{:02} C sentinel", offset + 4);
            assert_eq!(c, rust, "E{:02}", offset + 4);
        }
    }

    // ERRORS 8: broad-phase rejection.
    let broad_miss = C2Ray {
        p: C2v { x: -5.0, y: 5.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: 1.0,
    };
    assert_eq!(compare_ray_aabb(&pair, broad_miss, a, "E08"), 0);

    // ERRORS 9: broad phase overlaps, separating-axis distance rejects.
    let sat_miss = C2Ray {
        p: C2v { x: -1.0, y: 1.8 },
        d: C2v { x: 1.1, y: 1.1 },
        t: 1.0,
    };
    assert_eq!(compare_ray_aabb(&pair, sat_miss, a, "E09"), 0);

    // ERRORS 10: NaNs make every `tN <= 1` comparison false.
    let nan = f32::from_bits(0x7fc0_1234);
    let nan_ray = C2Ray {
        p: C2v { x: nan, y: nan },
        d: C2v { x: nan, y: nan },
        t: nan,
    };
    assert_eq!(compare_ray_aabb(&pair, nan_ray, a, "E10"), 0);

    let outside_points = [
        C2v { x: -1.0, y: 1.0 },
        C2v { x: 1.0, y: -1.0 },
        C2v { x: 3.0, y: 1.0 },
        C2v { x: 1.0, y: 3.0 },
    ];
    for (offset, point) in outside_points.into_iter().enumerate() {
        unsafe {
            let c = (pair.c.aabb_point)(a, point);
            let rust = (pair.rust.aabb_point)(a, point);
            assert_eq!(c, 0, "E{:02} C sentinel", offset + 11);
            assert_eq!(c, rust, "E{:02}", offset + 11);
        }
    }

    // ERRORS 15: circle boundary is excluded by the strict comparison.
    let boundary_circle = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 2.0,
    };
    unsafe {
        let point = C2v { x: 2.0, y: 0.0 };
        let c = (pair.c.circle_point)(boundary_circle, point);
        let rust = (pair.rust.circle_point)(boundary_circle, point);
        assert_eq!(c, 0, "E15 C sentinel");
        assert_eq!(c, rust, "E15");
    }

    // ERRORS 16: capsule final fallthrough.
    let capsule = C2Capsule {
        a: C2v { x: 0.0, y: 0.0 },
        b: C2v { x: 0.0, y: 5.0 },
        r: 1.0,
    };
    let capsule_miss = C2Ray {
        p: C2v { x: 3.0, y: 2.5 },
        d: C2v { x: 0.0, y: 1.0 },
        t: 2.0,
    };
    assert_eq!(compare_ray_capsule(&pair, capsule_miss, capsule, "E16"), 0);

    let square = square_poly(4, 1.0);
    // ERRORS 17: parallel to and outside the top plane.
    let parallel = C2Ray {
        p: C2v { x: -2.0, y: 2.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: 5.0,
    };
    assert_eq!(compare_ray_poly(&pair, parallel, &square, None, "E17"), 0);

    // ERRORS 18: entering parameter exceeds the segment's hi bound.
    let too_short = C2Ray {
        p: C2v { x: -3.0, y: 0.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: 1.0,
    };
    assert_eq!(compare_ray_poly(&pair, too_short, &square, None, "E18"), 0);

    // ERRORS 19: count zero skips the loop and leaves index at ~0.
    let empty = empty_poly(0);
    assert_eq!(compare_ray_poly(&pair, too_short, &empty, None, "E19"), 0);

    // ERRORS 20: invalid C enum values return before dereferencing any pointer.
    for invalid in [-1, 4, c_int::MIN, c_int::MAX] {
        assert_eq!(
            compare_cast_ray(
                &pair,
                too_short,
                std::ptr::null(),
                std::ptr::null(),
                invalid,
                &format!("E20 enum {invalid}")
            ),
            0
        );
    }
}

#[test]
fn generic_ffi_boundaries_with_defined_c_behavior() {
    let pair = Pair::load();

    // Null output pointers are valid on C paths that return before writing.
    let circle_miss = C2Ray {
        p: C2v { x: -2.0, y: 2.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: 1.0,
    };
    let circle = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 0.5,
    };
    unsafe {
        assert_eq!(
            (pair.c.ray_circle)(circle_miss, circle, std::ptr::null_mut()),
            (pair.rust.ray_circle)(circle_miss, circle, std::ptr::null_mut())
        );
    }

    let box_shape = C2Aabb {
        min: C2v { x: 0.0, y: 0.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    unsafe {
        assert_eq!(
            (pair.c.ray_aabb)(circle_miss, box_shape, std::ptr::null_mut()),
            (pair.rust.ray_aabb)(circle_miss, box_shape, std::ptr::null_mut())
        );
    }

    let empty = empty_poly(0);
    unsafe {
        assert_eq!(
            (pair.c.ray_poly)(circle_miss, &empty, std::ptr::null(), std::ptr::null_mut()),
            (pair.rust.ray_poly)(circle_miss, &empty, std::ptr::null(), std::ptr::null_mut())
        );
        assert_eq!(
            (pair.c.cast_ray)(
                circle_miss,
                std::ptr::null(),
                std::ptr::null(),
                4,
                std::ptr::null_mut()
            ),
            (pair.rust.cast_ray)(
                circle_miss,
                std::ptr::null(),
                std::ptr::null(),
                4,
                std::ptr::null_mut()
            )
        );
    }

    // poly_ray's first fixed ray rejects before writing cast1.
    let mut c_second = sentinel();
    let mut r_second = sentinel();
    let c_ret = unsafe { (pair.c.poly_ray)(std::ptr::null_mut(), &mut c_second) };
    let r_ret = unsafe { (pair.rust.poly_ray)(std::ptr::null_mut(), &mut r_second) };
    assert_cast(
        c_ret,
        c_second,
        r_ret,
        r_second,
        "poly_ray null first output",
    );

    // Zero, negative, maximum, and one-past fixed polygon counts.
    for count in [-1, 0, 8] {
        let poly = square_poly(count, 1.0);
        compare_ray_poly(
            &pair,
            C2Ray {
                p: C2v { x: -3.0, y: 0.0 },
                d: C2v { x: 1.0, y: 0.0 },
                t: 6.0,
            },
            &poly,
            None,
            &format!("polygon count {count}"),
        );
    }

    let mut extended = ExtendedPoly {
        poly: square_poly(9, 1.0),
        tail: [C2v { x: 0.0, y: 0.0 }; 8],
    };
    // C's norms[8] lands on the first trailing vector in this backing allocation.
    extended.tail[0] = C2v { x: 0.0, y: 0.0 };
    compare_ray_poly(
        &pair,
        C2Ray {
            p: C2v { x: -3.0, y: 0.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 6.0,
        },
        &extended.poly,
        None,
        "polygon count one past capacity",
    );
}
