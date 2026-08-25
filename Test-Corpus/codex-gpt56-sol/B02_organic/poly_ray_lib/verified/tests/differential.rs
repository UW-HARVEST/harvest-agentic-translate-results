use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

const CASES: usize = 128;

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

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&root);
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
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn functions<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = unsafe { *self.c.get::<T>(name).expect("load C symbol") };
        let rust = unsafe { *self.rust.get::<T>(name).expect("load Rust symbol") };
        (c, rust)
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    let profile_dir = std::env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("profile directory")
        .to_path_buf();
    let candidate = profile_dir.join("libpoly_ray_lib.so");
    if candidate.is_file() {
        candidate
    } else {
        root.join("target/debug/libpoly_ray_lib.so")
    }
}

fn bytes_of<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            std::ptr::from_ref(value).cast::<u8>(),
            std::mem::size_of::<T>(),
        )
    }
}

fn assert_bytes_eq<T>(c: &T, rust: &T, context: &str) {
    assert_eq!(
        bytes_of(c),
        bytes_of(rust),
        "{context}: C={:02x?}, Rust={:02x?}",
        bytes_of(c),
        bytes_of(rust)
    );
}

fn sentinel() -> C2Raycast {
    C2Raycast {
        t: f32::from_bits(0x7fc1_2345),
        n: C2v {
            x: f32::from_bits(0x7fc2_3456),
            y: f32::from_bits(0xffc3_4567),
        },
    }
}

#[derive(Clone, Copy)]
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

    fn f32(&mut self, low: f32, high: f32) -> f32 {
        let unit = (self.u32() as f64 / u32::MAX as f64) as f32;
        low + (high - low) * unit
    }

    fn nonzero(&mut self) -> f32 {
        let magnitude = self.f32(0.125, 16.0);
        if self.u32() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
    }

    fn vector(&mut self) -> C2v {
        C2v {
            x: self.f32(-16.0, 16.0),
            y: self.f32(-16.0, 16.0),
        }
    }
}

fn square(count: c_int) -> C2Poly {
    let z = C2v { x: 0.0, y: 0.0 };
    let mut poly = C2Poly {
        count,
        verts: [z; 8],
        norms: [z; 8],
    };
    poly.verts[0] = C2v { x: 1.0, y: -1.0 };
    poly.verts[1] = C2v { x: 1.0, y: 1.0 };
    poly.verts[2] = C2v { x: -1.0, y: 1.0 };
    poly.verts[3] = C2v { x: -1.0, y: -1.0 };
    poly.norms[0] = C2v { x: 1.0, y: 0.0 };
    poly.norms[1] = C2v { x: 0.0, y: 1.0 };
    poly.norms[2] = C2v { x: -1.0, y: 0.0 };
    poly.norms[3] = C2v { x: 0.0, y: -1.0 };
    for i in 4..8 {
        poly.verts[i] = poly.verts[i - 4];
        poly.norms[i] = poly.norms[i - 4];
    }
    poly
}

type RayCircleFn = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
type RayAabbFn = unsafe extern "C" fn(C2Ray, C2Aabb, *mut C2Raycast) -> c_int;
type RayCapsuleFn = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
type RayPolyFn = unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int;
type CastRayFn =
    unsafe extern "C" fn(C2Ray, *const c_void, *const C2x, c_int, *mut C2Raycast) -> c_int;

fn compare_ray_circle(
    c_fn: RayCircleFn,
    rust_fn: RayCircleFn,
    ray: C2Ray,
    circle: C2Circle,
    context: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { c_fn(ray, circle, &mut c_out) };
    let rust_result = unsafe { rust_fn(ray, circle, &mut rust_out) };
    assert_eq!(c_result, rust_result, "{context}: return value");
    assert_bytes_eq(&c_out, &rust_out, context);
    c_result
}

fn compare_ray_aabb(
    c_fn: RayAabbFn,
    rust_fn: RayAabbFn,
    ray: C2Ray,
    aabb: C2Aabb,
    context: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { c_fn(ray, aabb, &mut c_out) };
    let rust_result = unsafe { rust_fn(ray, aabb, &mut rust_out) };
    assert_eq!(c_result, rust_result, "{context}: return value");
    assert_bytes_eq(&c_out, &rust_out, context);
    c_result
}

fn compare_ray_capsule(
    c_fn: RayCapsuleFn,
    rust_fn: RayCapsuleFn,
    ray: C2Ray,
    capsule: C2Capsule,
    context: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { c_fn(ray, capsule, &mut c_out) };
    let rust_result = unsafe { rust_fn(ray, capsule, &mut rust_out) };
    assert_eq!(c_result, rust_result, "{context}: return value");
    assert_bytes_eq(&c_out, &rust_out, context);
    c_result
}

fn compare_ray_poly(
    c_fn: RayPolyFn,
    rust_fn: RayPolyFn,
    ray: C2Ray,
    poly: &C2Poly,
    transform: Option<&C2x>,
    context: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let transform_ptr = transform.map_or(std::ptr::null(), std::ptr::from_ref);
    let c_result = unsafe { c_fn(ray, poly, transform_ptr, &mut c_out) };
    let rust_result = unsafe { rust_fn(ray, poly, transform_ptr, &mut rust_out) };
    assert_eq!(c_result, rust_result, "{context}: return value");
    assert_bytes_eq(&c_out, &rust_out, context);
    c_result
}

fn compare_cast_ray(
    c_fn: CastRayFn,
    rust_fn: CastRayFn,
    ray: C2Ray,
    shape: *const c_void,
    transform: *const C2x,
    shape_type: c_int,
    context: &str,
) -> c_int {
    let mut c_out = sentinel();
    let mut rust_out = sentinel();
    let c_result = unsafe { c_fn(ray, shape, transform, shape_type, &mut c_out) };
    let rust_result = unsafe { rust_fn(ray, shape, transform, shape_type, &mut rust_out) };
    assert_eq!(c_result, rust_result, "{context}: return value");
    assert_bytes_eq(&c_out, &rust_out, context);
    c_result
}

#[test]
fn utility_and_transform_surface_matches() {
    let libs = Libraries::load();
    unsafe {
        let (c_v, r_v) = libs.functions::<unsafe extern "C" fn(f32, f32) -> C2v>(b"c2V\0");
        let (c_dot, r_dot) = libs.functions::<unsafe extern "C" fn(C2v, C2v) -> f32>(b"c2Dot\0");
        let (c_len, r_len) = libs.functions::<unsafe extern "C" fn(C2v) -> f32>(b"c2Len\0");
        let (c_add, r_add) = libs.functions::<unsafe extern "C" fn(C2v, C2v) -> C2v>(b"c2Add\0");
        let (c_sub, r_sub) = libs.functions::<unsafe extern "C" fn(C2v, C2v) -> C2v>(b"c2Sub\0");
        let (c_mulvs, r_mulvs) =
            libs.functions::<unsafe extern "C" fn(C2v, f32) -> C2v>(b"c2Mulvs\0");
        let (c_div, r_div) = libs.functions::<unsafe extern "C" fn(C2v, f32) -> C2v>(b"c2Div\0");
        let (c_norm, r_norm) = libs.functions::<unsafe extern "C" fn(C2v) -> C2v>(b"c2Norm\0");
        let (c_min, r_min) = libs.functions::<unsafe extern "C" fn(C2v, C2v) -> C2v>(b"c2Minv\0");
        let (c_max, r_max) = libs.functions::<unsafe extern "C" fn(C2v, C2v) -> C2v>(b"c2Maxv\0");
        let (c_skew, r_skew) = libs.functions::<unsafe extern "C" fn(C2v) -> C2v>(b"c2Skew\0");
        let (c_abs, r_abs) = libs.functions::<unsafe extern "C" fn(C2v) -> C2v>(b"c2Absv\0");
        let (c_ccw, r_ccw) = libs.functions::<unsafe extern "C" fn(C2v) -> C2v>(b"c2CCW90\0");
        let (c_mulmvt, r_mulmvt) =
            libs.functions::<unsafe extern "C" fn(C2m, C2v) -> C2v>(b"c2MulmvT\0");
        let (c_rot_identity, r_rot_identity) =
            libs.functions::<unsafe extern "C" fn() -> C2r>(b"c2RotIdentity\0");
        let (c_x_identity, r_x_identity) =
            libs.functions::<unsafe extern "C" fn() -> C2x>(b"c2xIdentity\0");
        let (c_mulrv, r_mulrv) =
            libs.functions::<unsafe extern "C" fn(C2r, C2v) -> C2v>(b"c2Mulrv\0");
        let (c_mulrvt, r_mulrvt) =
            libs.functions::<unsafe extern "C" fn(C2r, C2v) -> C2v>(b"c2MulrvT\0");
        let (c_mulxvt, r_mulxvt) =
            libs.functions::<unsafe extern "C" fn(C2x, C2v) -> C2v>(b"c2MulxvT\0");

        let mut rng = Rng::new(0x243f_6a88_85a3_08d3);
        for case in 0..CASES {
            let a = rng.vector();
            let b = rng.vector();
            let scalar = match case % 3 {
                0 => rng.f32(0.125, 16.0),
                1 => 0.0,
                _ => -rng.f32(0.125, 16.0),
            };
            let divisor = rng.nonzero();
            let matrix = C2m {
                x: rng.vector(),
                y: rng.vector(),
            };
            let angle = rng.f32(-3.0, 3.0);
            let rotation = C2r {
                c: angle.cos(),
                s: angle.sin(),
            };
            let transform = C2x {
                p: rng.vector(),
                r: rotation,
            };

            assert_bytes_eq(&c_v(a.x, a.y), &r_v(a.x, a.y), "c2V");
            assert_bytes_eq(&c_dot(a, b), &r_dot(a, b), "c2Dot");
            assert_bytes_eq(&c_len(a), &r_len(a), "c2Len");
            assert_bytes_eq(&c_add(a, b), &r_add(a, b), "c2Add");
            assert_bytes_eq(&c_sub(a, b), &r_sub(a, b), "c2Sub");
            assert_bytes_eq(&c_mulvs(a, scalar), &r_mulvs(a, scalar), "c2Mulvs");
            assert_bytes_eq(&c_div(a, divisor), &r_div(a, divisor), "c2Div");
            assert_bytes_eq(&c_div(a, 0.0), &r_div(a, 0.0), "c2Div positive zero");
            assert_bytes_eq(&c_div(a, -0.0), &r_div(a, -0.0), "c2Div negative zero");
            assert_bytes_eq(&c_norm(a), &r_norm(a), "c2Norm");
            assert_bytes_eq(&c_min(a, b), &r_min(a, b), "c2Minv");
            assert_bytes_eq(&c_max(a, b), &r_max(a, b), "c2Maxv");
            assert_bytes_eq(&c_skew(a), &r_skew(a), "c2Skew");
            assert_bytes_eq(&c_abs(a), &r_abs(a), "c2Absv");
            assert_bytes_eq(&c_ccw(a), &r_ccw(a), "c2CCW90");
            assert_bytes_eq(&c_mulmvt(matrix, a), &r_mulmvt(matrix, a), "c2MulmvT");
            assert_bytes_eq(&c_mulrv(rotation, a), &r_mulrv(rotation, a), "c2Mulrv");
            assert_bytes_eq(&c_mulrvt(rotation, a), &r_mulrvt(rotation, a), "c2MulrvT");
            assert_bytes_eq(&c_mulxvt(transform, a), &r_mulxvt(transform, a), "c2MulxvT");
        }

        let zero = C2v { x: 0.0, y: -0.0 };
        assert_bytes_eq(&c_len(zero), &r_len(zero), "c2Len zero");
        assert_bytes_eq(&c_norm(zero), &r_norm(zero), "c2Norm zero");
        for divisor in [0.0_f32, -0.0_f32] {
            for vector in [zero, C2v { x: 2.0, y: -3.0 }] {
                assert_bytes_eq(
                    &c_div(vector, divisor),
                    &r_div(vector, divisor),
                    "c2Div signed zero",
                );
            }
        }

        let branch_pairs = [
            (C2v { x: -2.0, y: -2.0 }, C2v { x: 2.0, y: 2.0 }),
            (C2v { x: -2.0, y: 2.0 }, C2v { x: 2.0, y: -2.0 }),
            (C2v { x: 2.0, y: -2.0 }, C2v { x: -2.0, y: 2.0 }),
            (C2v { x: 2.0, y: 2.0 }, C2v { x: -2.0, y: -2.0 }),
            (C2v { x: 0.0, y: -0.0 }, C2v { x: -0.0, y: 0.0 }),
        ];
        for (a, b) in branch_pairs {
            assert_bytes_eq(&c_min(a, b), &r_min(a, b), "c2Minv branches");
            assert_bytes_eq(&c_max(a, b), &r_max(a, b), "c2Maxv branches");
            assert_bytes_eq(&c_abs(a), &r_abs(a), "c2Absv branches");
        }

        assert_bytes_eq(&c_rot_identity(), &r_rot_identity(), "c2RotIdentity");
        assert_bytes_eq(&c_x_identity(), &r_x_identity(), "c2xIdentity");

        let identity = C2r { c: 1.0, s: 0.0 };
        let translation = C2x {
            p: C2v { x: 3.0, y: -4.0 },
            r: identity,
        };
        let value = C2v { x: 7.0, y: 9.0 };
        assert_bytes_eq(
            &c_mulrv(identity, value),
            &r_mulrv(identity, value),
            "c2Mulrv identity",
        );
        assert_bytes_eq(
            &c_mulrvt(identity, value),
            &r_mulrvt(identity, value),
            "c2MulrvT identity",
        );
        assert_bytes_eq(
            &c_mulxvt(translation, value),
            &r_mulxvt(translation, value),
            "c2MulxvT translation",
        );
    }
}

#[test]
fn circle_and_predicate_surface_matches() {
    let libs = Libraries::load();
    unsafe {
        let (c_ray_circle, r_ray_circle) = libs.functions::<RayCircleFn>(b"c2RaytoCircle\0");
        let (c_aabb_aabb, r_aabb_aabb) =
            libs.functions::<unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int>(b"c2AABBtoAABB\0");
        let (c_aabb_point, r_aabb_point) =
            libs.functions::<unsafe extern "C" fn(C2Aabb, C2v) -> c_int>(b"c2AABBtoPoint\0");
        let (c_circle_point, r_circle_point) =
            libs.functions::<unsafe extern "C" fn(C2Circle, C2v) -> c_int>(b"c2CircleToPoint\0");

        let mut rng = Rng::new(0x1319_8a2e_0370_7344);
        for _ in 0..CASES {
            let radius = rng.f32(0.5, 4.0);
            let center = rng.vector();
            let offset_y = rng.f32(-0.8 * radius, 0.8 * radius);
            let distance = rng.f32(radius + 1.0, radius + 8.0);
            let circle = C2Circle {
                p: center,
                r: radius,
            };
            let hit = C2Ray {
                p: C2v {
                    x: center.x - distance,
                    y: center.y + offset_y,
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance + radius + 1.0,
            };
            assert_eq!(
                compare_ray_circle(c_ray_circle, r_ray_circle, hit, circle, "circle secant"),
                1
            );

            let tangent = C2Ray {
                p: C2v {
                    x: center.x - distance,
                    y: center.y + radius * 0.95,
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance + 1.0,
            };
            assert_eq!(
                compare_ray_circle(
                    c_ray_circle,
                    r_ray_circle,
                    tangent,
                    circle,
                    "circle tangent",
                ),
                1
            );

            let disc_miss = C2Ray {
                p: C2v {
                    x: center.x - distance,
                    y: center.y + radius + rng.f32(0.1, 3.0),
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance * 2.0,
            };
            assert_eq!(
                compare_ray_circle(
                    c_ray_circle,
                    r_ray_circle,
                    disc_miss,
                    circle,
                    "circle negative discriminant",
                ),
                0
            );

            let behind = C2Ray {
                p: C2v {
                    x: center.x + distance,
                    y: center.y,
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance * 2.0,
            };
            assert_eq!(
                compare_ray_circle(
                    c_ray_circle,
                    r_ray_circle,
                    behind,
                    circle,
                    "circle root before origin",
                ),
                0
            );

            let too_short = C2Ray {
                p: C2v {
                    x: center.x - distance,
                    y: center.y,
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance - radius - 0.1,
            };
            assert_eq!(
                compare_ray_circle(
                    c_ray_circle,
                    r_ray_circle,
                    too_short,
                    circle,
                    "circle root past segment",
                ),
                0
            );

            let half_x = rng.f32(0.25, 5.0);
            let half_y = rng.f32(0.25, 5.0);
            let aabb = C2Aabb {
                min: C2v {
                    x: center.x - half_x,
                    y: center.y - half_y,
                },
                max: C2v {
                    x: center.x + half_x,
                    y: center.y + half_y,
                },
            };
            let overlap = C2Aabb {
                min: center,
                max: C2v {
                    x: center.x + half_x * 2.0,
                    y: center.y + half_y * 2.0,
                },
            };
            assert_eq!(c_aabb_aabb(aabb, overlap), r_aabb_aabb(aabb, overlap));

            let contained = C2Aabb {
                min: C2v {
                    x: center.x - half_x * 0.5,
                    y: center.y - half_y * 0.5,
                },
                max: C2v {
                    x: center.x + half_x * 0.5,
                    y: center.y + half_y * 0.5,
                },
            };
            assert_eq!(c_aabb_aabb(aabb, contained), r_aabb_aabb(aabb, contained));

            let touching = C2Aabb {
                min: C2v {
                    x: aabb.max.x,
                    y: aabb.max.y,
                },
                max: C2v {
                    x: aabb.max.x + half_x,
                    y: aabb.max.y + half_y,
                },
            };
            assert_eq!(c_aabb_aabb(aabb, touching), r_aabb_aabb(aabb, touching));

            let inside = center;
            let boundary = C2v {
                x: aabb.max.x,
                y: aabb.min.y,
            };
            assert_eq!(c_aabb_point(aabb, inside), r_aabb_point(aabb, inside));
            assert_eq!(c_aabb_point(aabb, boundary), r_aabb_point(aabb, boundary));
            let outside_points = [
                C2v {
                    x: aabb.min.x - rng.f32(0.1, 4.0),
                    y: center.y,
                },
                C2v {
                    x: center.x,
                    y: aabb.min.y - rng.f32(0.1, 4.0),
                },
                C2v {
                    x: aabb.max.x + rng.f32(0.1, 4.0),
                    y: center.y,
                },
                C2v {
                    x: center.x,
                    y: aabb.max.y + rng.f32(0.1, 4.0),
                },
            ];
            for point in outside_points {
                assert_eq!(c_aabb_point(aabb, point), 0);
                assert_eq!(c_aabb_point(aabb, point), r_aabb_point(aabb, point));
            }

            let separated = [
                C2Aabb {
                    min: C2v {
                        x: aabb.min.x - 4.0 * half_x,
                        y: center.y - 0.5 * half_y,
                    },
                    max: C2v {
                        x: aabb.min.x - 2.0 * half_x,
                        y: center.y + 0.5 * half_y,
                    },
                },
                C2Aabb {
                    min: C2v {
                        x: aabb.max.x + 2.0 * half_x,
                        y: center.y - 0.5 * half_y,
                    },
                    max: C2v {
                        x: aabb.max.x + 4.0 * half_x,
                        y: center.y + 0.5 * half_y,
                    },
                },
                C2Aabb {
                    min: C2v {
                        x: center.x - 0.5 * half_x,
                        y: aabb.min.y - 4.0 * half_y,
                    },
                    max: C2v {
                        x: center.x + 0.5 * half_x,
                        y: aabb.min.y - 2.0 * half_y,
                    },
                },
                C2Aabb {
                    min: C2v {
                        x: center.x - 0.5 * half_x,
                        y: aabb.max.y + 2.0 * half_y,
                    },
                    max: C2v {
                        x: center.x + 0.5 * half_x,
                        y: aabb.max.y + 4.0 * half_y,
                    },
                },
            ];
            for other in separated {
                assert_eq!(c_aabb_aabb(aabb, other), 0);
                assert_eq!(c_aabb_aabb(aabb, other), r_aabb_aabb(aabb, other));
            }

            let circle_inside = C2v {
                x: center.x + radius * 0.5,
                y: center.y,
            };
            let circle_boundary = C2v {
                x: center.x + radius,
                y: center.y,
            };
            let circle_outside = C2v {
                x: center.x + radius + 0.25,
                y: center.y,
            };
            for point in [circle_inside, circle_boundary, circle_outside] {
                assert_eq!(c_circle_point(circle, point), r_circle_point(circle, point));
            }
        }

        let a = C2Aabb {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        let separated = [
            C2Aabb {
                min: C2v { x: -4.0, y: -0.5 },
                max: C2v { x: -2.0, y: 0.5 },
            },
            C2Aabb {
                min: C2v { x: 2.0, y: -0.5 },
                max: C2v { x: 4.0, y: 0.5 },
            },
            C2Aabb {
                min: C2v { x: -0.5, y: -4.0 },
                max: C2v { x: 0.5, y: -2.0 },
            },
            C2Aabb {
                min: C2v { x: -0.5, y: 2.0 },
                max: C2v { x: 0.5, y: 4.0 },
            },
        ];
        for b in separated {
            assert_eq!(c_aabb_aabb(a, b), 0);
            assert_eq!(c_aabb_aabb(a, b), r_aabb_aabb(a, b));
        }

        let outside_points = [
            C2v { x: -2.0, y: 0.0 },
            C2v { x: 0.0, y: -2.0 },
            C2v { x: 2.0, y: 0.0 },
            C2v { x: 0.0, y: 2.0 },
        ];
        for point in outside_points {
            assert_eq!(c_aabb_point(a, point), 0);
            assert_eq!(c_aabb_point(a, point), r_aabb_point(a, point));
        }

        let miss = C2Ray {
            p: C2v { x: -4.0, y: 3.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 8.0,
        };
        let circle = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        let exact_tangent = C2Ray {
            p: C2v { x: -4.0, y: 1.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 8.0,
        };
        assert_eq!(
            compare_ray_circle(
                c_ray_circle,
                r_ray_circle,
                exact_tangent,
                circle,
                "circle exact tangent",
            ),
            1
        );
        assert_eq!(c_ray_circle(miss, circle, std::ptr::null_mut()), 0);
        assert_eq!(r_ray_circle(miss, circle, std::ptr::null_mut()), 0);
    }
}

#[test]
fn ray_to_aabb_surface_matches() {
    let libs = Libraries::load();
    unsafe {
        let (c_ray_aabb, r_ray_aabb) = libs.functions::<RayAabbFn>(b"c2RaytoAABB\0");
        let mut rng = Rng::new(0xa409_3822_299f_31d0);

        for _ in 0..CASES {
            let center = rng.vector();
            let hx = rng.f32(0.5, 4.0);
            let hy = rng.f32(0.5, 4.0);
            let aabb = C2Aabb {
                min: C2v {
                    x: center.x - hx,
                    y: center.y - hy,
                },
                max: C2v {
                    x: center.x + hx,
                    y: center.y + hy,
                },
            };
            let horizontal_distance = rng.f32(hx + 0.5, hx + 8.0);
            let vertical_distance = rng.f32(hy + 0.5, hy + 8.0);

            let face_hits = [
                (
                    C2Ray {
                        p: C2v {
                            x: center.x - horizontal_distance,
                            y: center.y,
                        },
                        d: C2v { x: 1.0, y: 0.0 },
                        t: horizontal_distance + hx + 1.0,
                    },
                    "AABB min-x face",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: center.x + horizontal_distance,
                            y: center.y,
                        },
                        d: C2v { x: -1.0, y: 0.0 },
                        t: horizontal_distance + hx + 1.0,
                    },
                    "AABB max-x face",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: center.x,
                            y: center.y - vertical_distance,
                        },
                        d: C2v { x: 0.0, y: 1.0 },
                        t: vertical_distance + hy + 1.0,
                    },
                    "AABB min-y face",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: center.x,
                            y: center.y + vertical_distance,
                        },
                        d: C2v { x: 0.0, y: -1.0 },
                        t: vertical_distance + hy + 1.0,
                    },
                    "AABB max-y face",
                ),
            ];
            for (ray, context) in face_hits {
                assert_eq!(
                    compare_ray_aabb(c_ray_aabb, r_ray_aabb, ray, aabb, context),
                    1
                );
            }

            let broad_miss = C2Ray {
                p: C2v {
                    x: aabb.min.x - hx * 3.0,
                    y: aabb.max.y + hy * 2.0,
                },
                d: C2v { x: -1.0, y: 0.0 },
                t: hx,
            };
            assert_eq!(
                compare_ray_aabb(
                    c_ray_aabb,
                    r_ray_aabb,
                    broad_miss,
                    aabb,
                    "AABB broad-phase miss",
                ),
                0
            );

            let diagonal_miss = C2Ray {
                p: C2v {
                    x: aabb.min.x - 2.0 * hx,
                    y: aabb.max.y - 0.1 * hy,
                },
                d: C2v {
                    x: 2.9 * hx,
                    y: 2.1 * hy,
                },
                t: 1.0,
            };
            assert_eq!(
                compare_ray_aabb(
                    c_ray_aabb,
                    r_ray_aabb,
                    diagonal_miss,
                    aabb,
                    "AABB separating-axis miss",
                ),
                0
            );
        }

        let aabb = C2Aabb {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        let nan_ray = C2Ray {
            p: C2v {
                x: f32::NAN,
                y: f32::NAN,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: 2.0,
        };
        assert_eq!(
            compare_ray_aabb(
                c_ray_aabb,
                r_ray_aabb,
                nan_ray,
                aabb,
                "AABB all plane comparisons false",
            ),
            0
        );

        let broad_miss = C2Ray {
            p: C2v { x: -4.0, y: 4.0 },
            d: C2v { x: -1.0, y: 0.0 },
            t: 1.0,
        };
        assert_eq!(c_ray_aabb(broad_miss, aabb, std::ptr::null_mut()), 0);
        assert_eq!(r_ray_aabb(broad_miss, aabb, std::ptr::null_mut()), 0);
    }
}

#[test]
fn ray_to_capsule_surface_matches() {
    let libs = Libraries::load();
    unsafe {
        let (c_ray_capsule, r_ray_capsule) = libs.functions::<RayCapsuleFn>(b"c2RaytoCapsule\0");
        let mut rng = Rng::new(0x082e_fa98_ec4e_6c89);

        for _ in 0..CASES {
            let origin = rng.vector();
            let radius = rng.f32(0.5, 3.0);
            let length = rng.f32(2.5 * radius, 8.0 * radius);
            let capsule = C2Capsule {
                a: origin,
                b: C2v {
                    x: origin.x,
                    y: origin.y + length,
                },
                r: radius,
            };
            let scenarios = [
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x + 0.5 * radius,
                            y: origin.y + 0.5 * length,
                        },
                        d: C2v { x: 1.0, y: 0.0 },
                        t: radius,
                    },
                    1,
                    "capsule starts in body",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x,
                            y: origin.y - 0.5 * radius,
                        },
                        d: C2v { x: 0.0, y: -1.0 },
                        t: radius,
                    },
                    1,
                    "capsule starts in cap A",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x,
                            y: origin.y + length + 0.5 * radius,
                        },
                        d: C2v { x: 0.0, y: 1.0 },
                        t: radius,
                    },
                    1,
                    "capsule starts in cap B",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x + 0.8 * radius,
                            y: origin.y - 2.0 * radius,
                        },
                        d: C2v { x: 0.0, y: 1.0 },
                        t: 4.0 * radius,
                    },
                    1,
                    "capsule narrow route to cap A",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x + 0.8 * radius,
                            y: origin.y + length + 2.0 * radius,
                        },
                        d: C2v { x: 0.0, y: -1.0 },
                        t: 4.0 * radius,
                    },
                    1,
                    "capsule narrow route to cap B",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x + 2.0 * radius,
                            y: origin.y - 0.5 * radius,
                        },
                        d: C2v { x: -1.0, y: 0.0 },
                        t: 4.0 * radius,
                    },
                    1,
                    "capsule crossing before cap A",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x + 2.0 * radius,
                            y: origin.y + length + 0.5 * radius,
                        },
                        d: C2v { x: -1.0, y: 0.0 },
                        t: 4.0 * radius,
                    },
                    1,
                    "capsule crossing after cap B",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x + 2.0 * radius,
                            y: origin.y + 0.5 * length,
                        },
                        d: C2v { x: -1.0, y: 0.0 },
                        t: 4.0 * radius,
                    },
                    1,
                    "capsule positive body side",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x - 2.0 * radius,
                            y: origin.y + 0.5 * length,
                        },
                        d: C2v { x: 1.0, y: 0.0 },
                        t: 4.0 * radius,
                    },
                    1,
                    "capsule negative body side",
                ),
                (
                    C2Ray {
                        p: C2v {
                            x: origin.x + 2.0 * radius,
                            y: origin.y + 0.5 * length,
                        },
                        d: C2v { x: 1.0, y: 0.0 },
                        t: radius,
                    },
                    0,
                    "capsule no crossing",
                ),
            ];

            for (ray, expected, context) in scenarios {
                assert_eq!(
                    compare_ray_capsule(c_ray_capsule, r_ray_capsule, ray, capsule, context,),
                    expected
                );
            }
        }

        let degenerate = C2Capsule {
            a: C2v { x: 1.0, y: 2.0 },
            b: C2v { x: 1.0, y: 2.0 },
            r: 1.0,
        };
        let ray = C2Ray {
            p: C2v { x: -2.0, y: 2.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 6.0,
        };
        compare_ray_capsule(
            c_ray_capsule,
            r_ray_capsule,
            ray,
            degenerate,
            "degenerate capsule",
        );
    }
}

#[test]
fn ray_to_poly_surface_matches() {
    let libs = Libraries::load();
    unsafe {
        let (c_ray_poly, r_ray_poly) = libs.functions::<RayPolyFn>(b"c2RaytoPoly\0");
        let mut rng = Rng::new(0x4528_21e6_38d0_1377);

        for _ in 0..CASES {
            let y = rng.f32(-0.8, 0.8);
            let distance = rng.f32(2.0, 12.0);
            let left_hit = C2Ray {
                p: C2v { x: -distance, y },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance + 2.0,
            };
            for count in [4, 8] {
                assert_eq!(
                    compare_ray_poly(
                        c_ray_poly,
                        r_ray_poly,
                        left_hit,
                        &square(count),
                        None,
                        "polygon identity hit",
                    ),
                    1
                );
            }

            let right_hit = C2Ray {
                p: C2v { x: distance, y },
                d: C2v { x: -1.0, y: 0.0 },
                t: distance + 2.0,
            };
            assert_eq!(
                compare_ray_poly(
                    c_ray_poly,
                    r_ray_poly,
                    right_hit,
                    &square(1),
                    None,
                    "single-plane polygon hit",
                ),
                1
            );

            let angle = rng.f32(-2.8, 2.8);
            let rotation = C2r {
                c: angle.cos(),
                s: angle.sin(),
            };
            let translation = rng.vector();
            let transform = C2x {
                p: translation,
                r: rotation,
            };
            let local_p = C2v { x: -distance, y };
            let world_ray = C2Ray {
                p: C2v {
                    x: rotation.c * local_p.x - rotation.s * local_p.y + translation.x,
                    y: rotation.s * local_p.x + rotation.c * local_p.y + translation.y,
                },
                d: C2v {
                    x: rotation.c,
                    y: rotation.s,
                },
                t: distance + 2.0,
            };
            assert_eq!(
                compare_ray_poly(
                    c_ray_poly,
                    r_ray_poly,
                    world_ray,
                    &square(4),
                    Some(&transform),
                    "transformed polygon hit",
                ),
                1
            );

            let parallel_outside = C2Ray {
                p: C2v {
                    x: -distance,
                    y: 1.0 + rng.f32(0.25, 3.0),
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance * 2.0,
            };
            assert_eq!(
                compare_ray_poly(
                    c_ray_poly,
                    r_ray_poly,
                    parallel_outside,
                    &square(4),
                    None,
                    "polygon parallel outside",
                ),
                0
            );

            let too_short = C2Ray {
                p: C2v { x: -distance, y },
                d: C2v { x: 1.0, y: 0.0 },
                t: distance - 1.0 - 0.1,
            };
            assert_eq!(
                compare_ray_poly(
                    c_ray_poly,
                    r_ray_poly,
                    too_short,
                    &square(4),
                    None,
                    "polygon empty clipping interval",
                ),
                0
            );

            let starts_inside = C2Ray {
                p: C2v {
                    x: rng.f32(-0.8, 0.8),
                    y: rng.f32(-0.8, 0.8),
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: 4.0,
            };
            assert_eq!(
                compare_ray_poly(
                    c_ray_poly,
                    r_ray_poly,
                    starts_inside,
                    &square(4),
                    None,
                    "polygon no entering edge",
                ),
                0
            );

            for count in [0, -1 - (rng.u32() % 32) as c_int] {
                assert_eq!(
                    compare_ray_poly(
                        c_ray_poly,
                        r_ray_poly,
                        left_hit,
                        &square(count),
                        None,
                        "polygon skipped loop",
                    ),
                    0
                );
            }
        }

        let parallel_outside = C2Ray {
            p: C2v { x: -3.0, y: 2.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 8.0,
        };
        for count in [9, c_int::MAX] {
            assert_eq!(
                compare_ray_poly(
                    c_ray_poly,
                    r_ray_poly,
                    parallel_outside,
                    &square(count),
                    None,
                    "oversized polygon early rejection",
                ),
                0
            );
        }

        let poly = square(4);
        assert_eq!(
            c_ray_poly(
                parallel_outside,
                &poly,
                std::ptr::null(),
                std::ptr::null_mut(),
            ),
            0
        );
        assert_eq!(
            r_ray_poly(
                parallel_outside,
                &poly,
                std::ptr::null(),
                std::ptr::null_mut(),
            ),
            0
        );

        let zero_count = square(0);
        assert_eq!(
            c_ray_poly(
                parallel_outside,
                &zero_count,
                std::ptr::null(),
                std::ptr::null_mut(),
            ),
            0
        );
        assert_eq!(
            r_ray_poly(
                parallel_outside,
                &zero_count,
                std::ptr::null(),
                std::ptr::null_mut(),
            ),
            0
        );
    }
}

#[test]
fn dispatcher_and_composed_api_match() {
    let libs = Libraries::load();
    unsafe {
        let (c_cast, r_cast) = libs.functions::<CastRayFn>(b"c2CastRay\0");
        let (c_poly_ray, r_poly_ray) = libs
            .functions::<unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int>(
                b"poly_ray\0",
            );
        let mut rng = Rng::new(0xbe54_66cf_34e9_0c6c);

        for _ in 0..CASES {
            let center = rng.vector();
            let radius = rng.f32(0.5, 3.0);

            let circle = C2Circle {
                p: center,
                r: radius,
            };
            let horizontal_ray = C2Ray {
                p: C2v {
                    x: center.x - 3.0 * radius,
                    y: center.y,
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: 6.0 * radius,
            };
            assert_eq!(
                compare_cast_ray(
                    c_cast,
                    r_cast,
                    horizontal_ray,
                    std::ptr::from_ref(&circle).cast(),
                    std::ptr::null(),
                    0,
                    "dispatch circle",
                ),
                1
            );

            let aabb = C2Aabb {
                min: C2v {
                    x: center.x - radius,
                    y: center.y - radius,
                },
                max: C2v {
                    x: center.x + radius,
                    y: center.y + radius,
                },
            };
            assert_eq!(
                compare_cast_ray(
                    c_cast,
                    r_cast,
                    horizontal_ray,
                    std::ptr::from_ref(&aabb).cast(),
                    std::ptr::null(),
                    1,
                    "dispatch AABB",
                ),
                1
            );

            let capsule = C2Capsule {
                a: C2v {
                    x: center.x,
                    y: center.y - 2.0 * radius,
                },
                b: C2v {
                    x: center.x,
                    y: center.y + 2.0 * radius,
                },
                r: radius,
            };
            assert_eq!(
                compare_cast_ray(
                    c_cast,
                    r_cast,
                    horizontal_ray,
                    std::ptr::from_ref(&capsule).cast(),
                    std::ptr::null(),
                    2,
                    "dispatch capsule",
                ),
                1
            );

            let poly = square(4);
            let poly_ray = C2Ray {
                p: C2v {
                    x: -rng.f32(2.0, 10.0),
                    y: rng.f32(-0.8, 0.8),
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: 12.0,
            };
            assert_eq!(
                compare_cast_ray(
                    c_cast,
                    r_cast,
                    poly_ray,
                    std::ptr::from_ref(&poly).cast(),
                    std::ptr::null(),
                    3,
                    "dispatch polygon identity",
                ),
                1
            );

            let transform = C2x {
                p: C2v {
                    x: rng.f32(-4.0, 4.0),
                    y: 0.0,
                },
                r: C2r { c: 1.0, s: 0.0 },
            };
            let translated_ray = C2Ray {
                p: C2v {
                    x: transform.p.x - rng.f32(2.0, 10.0),
                    y: rng.f32(-0.8, 0.8),
                },
                d: C2v { x: 1.0, y: 0.0 },
                t: 12.0,
            };
            assert_eq!(
                compare_cast_ray(
                    c_cast,
                    r_cast,
                    translated_ray,
                    std::ptr::from_ref(&poly).cast(),
                    &transform,
                    3,
                    "dispatch transformed polygon",
                ),
                1
            );
        }

        let dummy_ray = C2Ray {
            p: C2v { x: 0.0, y: 0.0 },
            d: C2v { x: 0.0, y: 0.0 },
            t: 0.0,
        };
        for invalid_type in [c_int::MIN, -1, 4, c_int::MAX] {
            assert_eq!(
                c_cast(
                    dummy_ray,
                    std::ptr::null(),
                    std::ptr::null(),
                    invalid_type,
                    std::ptr::null_mut(),
                ),
                0
            );
            assert_eq!(
                r_cast(
                    dummy_ray,
                    std::ptr::null(),
                    std::ptr::null(),
                    invalid_type,
                    std::ptr::null_mut(),
                ),
                0
            );
        }
        for _ in 0..CASES {
            let invalid_type = 4 + (rng.u32() & 0x3fff_ffff) as c_int;
            assert_eq!(
                c_cast(
                    dummy_ray,
                    std::ptr::null(),
                    std::ptr::null(),
                    invalid_type,
                    std::ptr::null_mut(),
                ),
                r_cast(
                    dummy_ray,
                    std::ptr::null(),
                    std::ptr::null(),
                    invalid_type,
                    std::ptr::null_mut(),
                )
            );
        }

        for _ in 0..CASES {
            let mut c_first = sentinel();
            let mut c_second = sentinel();
            let mut rust_first = sentinel();
            let mut rust_second = sentinel();
            let c_result = c_poly_ray(&mut c_first, &mut c_second);
            let rust_result = r_poly_ray(&mut rust_first, &mut rust_second);
            assert_eq!(c_result, rust_result, "poly_ray return value");
            assert_bytes_eq(&c_first, &rust_first, "poly_ray first output");
            assert_bytes_eq(&c_second, &rust_second, "poly_ray second output");
        }

        let mut c_second = sentinel();
        let mut rust_second = sentinel();
        assert_eq!(
            c_poly_ray(std::ptr::null_mut(), &mut c_second),
            r_poly_ray(std::ptr::null_mut(), &mut rust_second)
        );
        assert_bytes_eq(&c_second, &rust_second, "poly_ray safe null first output");
    }
}
