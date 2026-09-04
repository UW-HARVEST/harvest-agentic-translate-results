use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

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

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Poly9 {
    poly: C2Poly,
    ninth_norm: C2v,
}

type VFn = unsafe extern "C" fn(f32, f32) -> C2v;
type VVFloatFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type VFloatFn = unsafe extern "C" fn(C2v) -> f32;
type VVFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type VSFn = unsafe extern "C" fn(C2v, f32) -> C2v;
type VFn1 = unsafe extern "C" fn(C2v) -> C2v;
type AabbAabbFn = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type AabbPointFn = unsafe extern "C" fn(C2Aabb, C2v) -> c_int;
type CirclePointFn = unsafe extern "C" fn(C2Circle, C2v) -> c_int;
type RayCircleFn = unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int;
type RayAabbFn = unsafe extern "C" fn(C2Ray, C2Aabb, *mut C2Raycast) -> c_int;
type RayCapsuleFn = unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int;
type MulmvTFn = unsafe extern "C" fn(C2m, C2v) -> C2v;
type RotIdentityFn = unsafe extern "C" fn() -> C2r;
type XIdentityFn = unsafe extern "C" fn() -> C2x;
type MulrvFn = unsafe extern "C" fn(C2r, C2v) -> C2v;
type MulxvTFn = unsafe extern "C" fn(C2x, C2v) -> C2v;
type RayPolyFn = unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int;
type CastRayFn =
    unsafe extern "C" fn(C2Ray, *const c_void, *const C2x, c_int, *mut C2Raycast) -> c_int;
type PolyRayFn = unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int;

fn c_library_path() -> PathBuf {
    let build = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(build)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    assert_eq!(candidates.len(), 1, "expected exactly one C shared object");
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libpoly_ray_lib.so")
}

unsafe fn libraries() -> (Library, Library, Library) {
    let libm: Library = unsafe {
        libloading::os::unix::Library::open(
            Some("libm.so.6"),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
        )
        .unwrap()
        .into()
    };
    (
        libm,
        unsafe { Library::new(c_library_path()).unwrap() },
        unsafe { Library::new(rust_library_path()).unwrap() },
    )
}

unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
    unsafe { *library.get::<T>(name).unwrap() }
}

fn bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts((value as *const T).cast::<u8>(), std::mem::size_of::<T>())
    }
}

fn assert_same<T: std::fmt::Debug>(label: &str, c: T, rust: T) {
    assert_eq!(bytes(&c), bytes(&rust), "{label}: C={c:?} Rust={rust:?}");
}

fn sentinel() -> C2Raycast {
    C2Raycast {
        t: f32::from_bits(0x4f12_3456),
        n: C2v {
            x: f32::from_bits(0xcf23_4567),
            y: f32::from_bits(0x4f34_5678),
        },
    }
}

fn compare_out<F>(label: &str, mut call: F)
where
    F: FnMut(bool, *mut C2Raycast) -> c_int,
{
    let mut c_out = sentinel();
    let mut r_out = sentinel();
    let c_result = call(false, &mut c_out);
    let r_result = call(true, &mut r_out);
    assert_same(&format!("{label} return"), c_result, r_result);
    assert_same(&format!("{label} output"), c_out, r_out);
}

fn compare_rejection<F>(label: &str, mut call: F)
where
    F: FnMut(bool, *mut C2Raycast) -> c_int,
{
    let mut c_out = sentinel();
    let mut r_out = sentinel();
    let c_result = call(false, &mut c_out);
    let r_result = call(true, &mut r_out);
    assert_eq!(c_result, 0, "{label}: C did not reject");
    assert_same(&format!("{label} return"), c_result, r_result);
    assert_same(&format!("{label} output"), c_out, r_out);
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
        x as u32
    }

    fn bounded(&mut self, low: i32, high: i32) -> f32 {
        let width = (high - low + 1) as u32;
        (low + (self.next_u32() % width) as i32) as f32 / 8.0
    }

    fn positive(&mut self) -> f32 {
        0.25 + (self.next_u32() % 64) as f32 / 8.0
    }

    fn nonzero(&mut self) -> f32 {
        let value = self.bounded(-64, 64);
        if value == 0.0 { 0.125 } else { value }
    }
}

fn square(half: f32) -> C2Poly {
    let z = C2v { x: 0.0, y: 0.0 };
    let mut p = C2Poly {
        count: 4,
        verts: [z; 8],
        norms: [z; 8],
    };
    p.verts[0] = C2v { x: half, y: -half };
    p.verts[1] = C2v { x: half, y: half };
    p.verts[2] = C2v { x: -half, y: half };
    p.verts[3] = C2v { x: -half, y: -half };
    p.norms[0] = C2v { x: 1.0, y: 0.0 };
    p.norms[1] = C2v { x: 0.0, y: 1.0 };
    p.norms[2] = C2v { x: -1.0, y: 0.0 };
    p.norms[3] = C2v { x: 0.0, y: -1.0 };
    p
}

#[test]
fn phase_a_all_c_symbols_load_from_both_shared_objects() {
    let names: &[&[u8]] = &[
        b"c2AABBtoAABB\0",
        b"c2AABBtoPoint\0",
        b"c2Absv\0",
        b"c2Add\0",
        b"c2CCW90\0",
        b"c2CastRay\0",
        b"c2CircleToPoint\0",
        b"c2Div\0",
        b"c2Dot\0",
        b"c2Len\0",
        b"c2Maxv\0",
        b"c2Minv\0",
        b"c2MulmvT\0",
        b"c2Mulrv\0",
        b"c2MulrvT\0",
        b"c2Mulvs\0",
        b"c2MulxvT\0",
        b"c2Norm\0",
        b"c2RaytoAABB\0",
        b"c2RaytoCapsule\0",
        b"c2RaytoCircle\0",
        b"c2RaytoPoly\0",
        b"c2RotIdentity\0",
        b"c2Skew\0",
        b"c2Sub\0",
        b"c2V\0",
        b"c2xIdentity\0",
        b"poly_ray\0",
    ];
    let (_libm, c, rust) = unsafe { libraries() };
    for name in names {
        unsafe {
            c.get::<*const c_void>(name).unwrap();
            rust.get::<*const c_void>(name).unwrap();
        }
    }
}

#[test]
fn phase_b_low_level_and_predicate_configurations() {
    let (_libm, c, rust) = unsafe { libraries() };
    let c_v: VFn = unsafe { symbol(&c, b"c2V\0") };
    let r_v: VFn = unsafe { symbol(&rust, b"c2V\0") };
    let c_dot: VVFloatFn = unsafe { symbol(&c, b"c2Dot\0") };
    let r_dot: VVFloatFn = unsafe { symbol(&rust, b"c2Dot\0") };
    let c_len: VFloatFn = unsafe { symbol(&c, b"c2Len\0") };
    let r_len: VFloatFn = unsafe { symbol(&rust, b"c2Len\0") };
    let c_add: VVFn = unsafe { symbol(&c, b"c2Add\0") };
    let r_add: VVFn = unsafe { symbol(&rust, b"c2Add\0") };
    let c_sub: VVFn = unsafe { symbol(&c, b"c2Sub\0") };
    let r_sub: VVFn = unsafe { symbol(&rust, b"c2Sub\0") };
    let c_mulvs: VSFn = unsafe { symbol(&c, b"c2Mulvs\0") };
    let r_mulvs: VSFn = unsafe { symbol(&rust, b"c2Mulvs\0") };
    let c_div: VSFn = unsafe { symbol(&c, b"c2Div\0") };
    let r_div: VSFn = unsafe { symbol(&rust, b"c2Div\0") };
    let c_norm: VFn1 = unsafe { symbol(&c, b"c2Norm\0") };
    let r_norm: VFn1 = unsafe { symbol(&rust, b"c2Norm\0") };
    let c_min: VVFn = unsafe { symbol(&c, b"c2Minv\0") };
    let r_min: VVFn = unsafe { symbol(&rust, b"c2Minv\0") };
    let c_max: VVFn = unsafe { symbol(&c, b"c2Maxv\0") };
    let r_max: VVFn = unsafe { symbol(&rust, b"c2Maxv\0") };
    let c_skew: VFn1 = unsafe { symbol(&c, b"c2Skew\0") };
    let r_skew: VFn1 = unsafe { symbol(&rust, b"c2Skew\0") };
    let c_abs: VFn1 = unsafe { symbol(&c, b"c2Absv\0") };
    let r_abs: VFn1 = unsafe { symbol(&rust, b"c2Absv\0") };
    let c_ccw: VFn1 = unsafe { symbol(&c, b"c2CCW90\0") };
    let r_ccw: VFn1 = unsafe { symbol(&rust, b"c2CCW90\0") };
    let c_mulmvt: MulmvTFn = unsafe { symbol(&c, b"c2MulmvT\0") };
    let r_mulmvt: MulmvTFn = unsafe { symbol(&rust, b"c2MulmvT\0") };
    let c_rot_identity: RotIdentityFn = unsafe { symbol(&c, b"c2RotIdentity\0") };
    let r_rot_identity: RotIdentityFn = unsafe { symbol(&rust, b"c2RotIdentity\0") };
    let c_x_identity: XIdentityFn = unsafe { symbol(&c, b"c2xIdentity\0") };
    let r_x_identity: XIdentityFn = unsafe { symbol(&rust, b"c2xIdentity\0") };
    let c_mulrv: MulrvFn = unsafe { symbol(&c, b"c2Mulrv\0") };
    let r_mulrv: MulrvFn = unsafe { symbol(&rust, b"c2Mulrv\0") };
    let c_mulrvt: MulrvFn = unsafe { symbol(&c, b"c2MulrvT\0") };
    let r_mulrvt: MulrvFn = unsafe { symbol(&rust, b"c2MulrvT\0") };
    let c_mulxvt: MulxvTFn = unsafe { symbol(&c, b"c2MulxvT\0") };
    let r_mulxvt: MulxvTFn = unsafe { symbol(&rust, b"c2MulxvT\0") };

    let mut rng = Rng::new(0x91ab_cdef_1234_5678);
    for i in 0..256 {
        let a = C2v {
            x: rng.bounded(-64, 64),
            y: rng.bounded(-64, 64),
        };
        let b = C2v {
            x: rng.bounded(-64, 64),
            y: rng.bounded(-64, 64),
        };
        let scalar = rng.nonzero();
        unsafe {
            assert_same(&format!("config 1 c2V {i}"), c_v(a.x, a.y), r_v(a.x, a.y));
            assert_same(&format!("config 2 c2Dot {i}"), c_dot(a, b), r_dot(a, b));
            assert_same(&format!("config 4 c2Len {i}"), c_len(a), r_len(a));
            assert_same(&format!("config 5 c2Add {i}"), c_add(a, b), r_add(a, b));
            assert_same(&format!("config 6 c2Sub {i}"), c_sub(a, b), r_sub(a, b));
            assert_same(
                &format!("config 8 c2Mulvs {i}"),
                c_mulvs(a, scalar),
                r_mulvs(a, scalar),
            );
            assert_same(
                &format!("config 9 c2Div {i}"),
                c_div(a, scalar),
                r_div(a, scalar),
            );
            if a.x != 0.0 || a.y != 0.0 {
                assert_same(&format!("config 11 c2Norm {i}"), c_norm(a), r_norm(a));
            }
            assert_same(
                &format!("config 13-14 c2Minv {i}"),
                c_min(a, b),
                r_min(a, b),
            );
            assert_same(
                &format!("config 15-16 c2Maxv {i}"),
                c_max(a, b),
                r_max(a, b),
            );
            assert_same(&format!("config 17 c2Skew {i}"), c_skew(a), r_skew(a));
            assert_same(&format!("config 18 c2Absv {i}"), c_abs(a), r_abs(a));
            assert_same(&format!("config 44 c2CCW90 {i}"), c_ccw(a), r_ccw(a));

            let matrix = C2m { x: a, y: b };
            assert_same(
                &format!("config 45 c2MulmvT {i}"),
                c_mulmvt(matrix, b),
                r_mulmvt(matrix, b),
            );

            let rotation = C2r {
                c: rng.bounded(-8, 8),
                s: rng.bounded(-8, 8),
            };
            assert_same(
                &format!("config 56 c2Mulrv {i}"),
                c_mulrv(rotation, a),
                r_mulrv(rotation, a),
            );
            assert_same(
                &format!("config 57 c2MulrvT {i}"),
                c_mulrvt(rotation, a),
                r_mulrvt(rotation, a),
            );
            let transform = C2x { p: b, r: rotation };
            assert_same(
                &format!("config 58 c2MulxvT {i}"),
                c_mulxvt(transform, a),
                r_mulxvt(transform, a),
            );
        }
    }

    let zero = C2v { x: 0.0, y: 0.0 };
    let signed_zero = C2v { x: -0.0, y: 0.0 };
    unsafe {
        assert_same("config 3 c2Len zero", c_len(zero), r_len(zero));
        assert_same(
            "config 7 c2Mulvs zero",
            c_mulvs(signed_zero, 0.0),
            r_mulvs(signed_zero, 0.0),
        );
        assert_same(
            "config 10 c2Div zero",
            c_div(C2v { x: 1.0, y: 0.0 }, 0.0),
            r_div(C2v { x: 1.0, y: 0.0 }, 0.0),
        );
        assert_same("config 12 c2Norm zero", c_norm(zero), r_norm(zero));
        assert_same(
            "config 18 c2Absv signed zero",
            c_abs(signed_zero),
            r_abs(signed_zero),
        );
        assert_same(
            "config 54 c2RotIdentity",
            c_rot_identity(),
            r_rot_identity(),
        );
        assert_same("config 55 c2xIdentity", c_x_identity(), r_x_identity());
    }
}

#[test]
fn phase_b_aabb_circle_and_ray_circle_configurations() {
    let (_libm, c, rust) = unsafe { libraries() };
    let c_aabb: AabbAabbFn = unsafe { symbol(&c, b"c2AABBtoAABB\0") };
    let r_aabb: AabbAabbFn = unsafe { symbol(&rust, b"c2AABBtoAABB\0") };
    let c_point: AabbPointFn = unsafe { symbol(&c, b"c2AABBtoPoint\0") };
    let r_point: AabbPointFn = unsafe { symbol(&rust, b"c2AABBtoPoint\0") };
    let c_circle_point: CirclePointFn = unsafe { symbol(&c, b"c2CircleToPoint\0") };
    let r_circle_point: CirclePointFn = unsafe { symbol(&rust, b"c2CircleToPoint\0") };
    let c_ray_circle: RayCircleFn = unsafe { symbol(&c, b"c2RaytoCircle\0") };
    let r_ray_circle: RayCircleFn = unsafe { symbol(&rust, b"c2RaytoCircle\0") };

    let mut rng = Rng::new(0x7777_3333_9999_1111);
    for i in 0..128 {
        let x = rng.bounded(-32, 32);
        let y = rng.bounded(-32, 32);
        let w = rng.positive();
        let h = rng.positive();
        let gap = rng.positive();
        let a = C2Aabb {
            min: C2v { x, y },
            max: C2v { x: x + w, y: y + h },
        };
        let overlap = C2Aabb {
            min: C2v {
                x: x + w / 2.0,
                y: y + h / 2.0,
            },
            max: C2v {
                x: x + w * 1.5,
                y: y + h * 1.5,
            },
        };
        let touching = C2Aabb {
            min: C2v { x: x + w, y },
            max: C2v {
                x: x + w + gap,
                y: y + h,
            },
        };
        let left = C2Aabb {
            min: C2v { x: x - gap - w, y },
            max: C2v {
                x: x - gap,
                y: y + h,
            },
        };
        let right = C2Aabb {
            min: C2v { x: x + w + gap, y },
            max: C2v {
                x: x + 2.0 * w + gap,
                y: y + h,
            },
        };
        let below = C2Aabb {
            min: C2v { x, y: y - gap - h },
            max: C2v {
                x: x + w,
                y: y - gap,
            },
        };
        let above = C2Aabb {
            min: C2v { x, y: y + h + gap },
            max: C2v {
                x: x + w,
                y: y + 2.0 * h + gap,
            },
        };
        unsafe {
            assert_same(
                &format!("config 19 overlap {i}"),
                c_aabb(a, overlap),
                r_aabb(a, overlap),
            );
            assert_same(
                &format!("config 20 touching {i}"),
                c_aabb(a, touching),
                r_aabb(a, touching),
            );
            assert_same(
                &format!("config 21 left {i}"),
                c_aabb(a, left),
                r_aabb(a, left),
            );
            assert_same(
                &format!("config 22 right {i}"),
                c_aabb(a, right),
                r_aabb(a, right),
            );
            assert_same(
                &format!("config 23 below {i}"),
                c_aabb(a, below),
                r_aabb(a, below),
            );
            assert_same(
                &format!("config 24 above {i}"),
                c_aabb(a, above),
                r_aabb(a, above),
            );
        }

        let interior = C2v {
            x: x + w / 2.0,
            y: y + h / 2.0,
        };
        let boundaries = [
            a.min,
            a.max,
            C2v {
                x: a.min.x,
                y: a.max.y,
            },
            C2v {
                x: a.max.x,
                y: a.min.y,
            },
        ];
        let outside = [
            C2v { x: x - gap, y },
            C2v { x, y: y - gap },
            C2v { x: x + w + gap, y },
            C2v { x, y: y + h + gap },
        ];
        unsafe {
            assert_same(
                &format!("config 25 point interior {i}"),
                c_point(a, interior),
                r_point(a, interior),
            );
            for (j, point) in boundaries.into_iter().enumerate() {
                assert_same(
                    &format!("config 26 point boundary {i}/{j}"),
                    c_point(a, point),
                    r_point(a, point),
                );
            }
            for (j, point) in outside.into_iter().enumerate() {
                assert_same(
                    &format!("config 27 point outside {i}/{j}"),
                    c_point(a, point),
                    r_point(a, point),
                );
            }
        }

        let center = C2v { x, y };
        let radius = rng.positive();
        let circle = C2Circle {
            p: center,
            r: radius,
        };
        unsafe {
            assert_same(
                &format!("config 28 circle inside {i}"),
                c_circle_point(circle, center),
                r_circle_point(circle, center),
            );
            assert_same(
                &format!("config 29 circle boundary {i}"),
                c_circle_point(circle, C2v { x: x + radius, y }),
                r_circle_point(circle, C2v { x: x + radius, y }),
            );
            assert_same(
                &format!("config 30 circle outside {i}"),
                c_circle_point(
                    circle,
                    C2v {
                        x: x + radius + gap,
                        y,
                    },
                ),
                r_circle_point(
                    circle,
                    C2v {
                        x: x + radius + gap,
                        y,
                    },
                ),
            );
        }

        let distance = radius + gap;
        let hit_ray = C2Ray {
            p: C2v { x: x - distance, y },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance + radius,
        };
        compare_out(
            &format!("config 31 circle hit {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_circle(hit_ray, circle, out)
                } else {
                    c_ray_circle(hit_ray, circle, out)
                }
            },
        );
        let tangent_ray = C2Ray {
            p: C2v {
                x: x - distance,
                y: y + radius,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance + radius,
        };
        compare_out(
            &format!("config 32 circle tangent {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_circle(tangent_ray, circle, out)
                } else {
                    c_ray_circle(tangent_ray, circle, out)
                }
            },
        );
        let disc_miss = C2Ray {
            p: C2v {
                x: x - distance,
                y: y + radius + gap,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance + radius,
        };
        compare_out(
            &format!("config 33 circle disc miss {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_circle(disc_miss, circle, out)
                } else {
                    c_ray_circle(disc_miss, circle, out)
                }
            },
        );
        let behind = C2Ray {
            p: C2v {
                x: x + radius + gap,
                y,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: distance + radius,
        };
        compare_out(
            &format!("config 34 circle behind {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_circle(behind, circle, out)
                } else {
                    c_ray_circle(behind, circle, out)
                }
            },
        );
        let beyond = C2Ray {
            t: gap / 2.0,
            ..hit_ray
        };
        compare_out(
            &format!("config 35 circle beyond {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_circle(beyond, circle, out)
                } else {
                    c_ray_circle(beyond, circle, out)
                }
            },
        );
    }
}

#[test]
fn phase_b_ray_aabb_and_capsule_configurations() {
    let (_libm, c, rust) = unsafe { libraries() };
    let c_ray_aabb: RayAabbFn = unsafe { symbol(&c, b"c2RaytoAABB\0") };
    let r_ray_aabb: RayAabbFn = unsafe { symbol(&rust, b"c2RaytoAABB\0") };
    let c_ray_capsule: RayCapsuleFn = unsafe { symbol(&c, b"c2RaytoCapsule\0") };
    let r_ray_capsule: RayCapsuleFn = unsafe { symbol(&rust, b"c2RaytoCapsule\0") };
    let mut rng = Rng::new(0x1357_2468_aaaa_5555);

    for i in 0..128 {
        let x = rng.bounded(-32, 32);
        let y = rng.bounded(-32, 32);
        let w = rng.positive();
        let h = rng.positive();
        let gap = rng.positive();
        let aabb = C2Aabb {
            min: C2v { x, y },
            max: C2v { x: x + w, y: y + h },
        };
        let mid_x = x + w / 2.0;
        let mid_y = y + h / 2.0;
        let rays = [
            (
                36,
                C2Ray {
                    p: C2v {
                        x: x - gap,
                        y: mid_y,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: gap + w,
                },
            ),
            (
                37,
                C2Ray {
                    p: C2v {
                        x: x + w + gap,
                        y: mid_y,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: gap + w,
                },
            ),
            (
                38,
                C2Ray {
                    p: C2v {
                        x: mid_x,
                        y: y - gap,
                    },
                    d: C2v { x: 0.0, y: 1.0 },
                    t: gap + h,
                },
            ),
            (
                39,
                C2Ray {
                    p: C2v {
                        x: mid_x,
                        y: y + h + gap,
                    },
                    d: C2v { x: 0.0, y: -1.0 },
                    t: gap + h,
                },
            ),
            (
                40,
                C2Ray {
                    p: C2v { x: mid_x, y: mid_y },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: w,
                },
            ),
            (
                41,
                C2Ray {
                    p: C2v {
                        x: x - gap,
                        y: mid_y,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: gap,
                },
            ),
        ];
        for (config, ray) in rays {
            compare_out(
                &format!("config {config} ray/AABB {i}"),
                |is_rust, out| unsafe {
                    if is_rust {
                        r_ray_aabb(ray, aabb, out)
                    } else {
                        c_ray_aabb(ray, aabb, out)
                    }
                },
            );
        }

        let half = rng.positive();
        let corner_box = C2Aabb {
            min: C2v {
                x: x - half,
                y: y - half,
            },
            max: C2v {
                x: x + half,
                y: y + half,
            },
        };
        let sat_miss = C2Ray {
            p: C2v {
                x: x - 2.0 * half,
                y: y + 0.5 * half,
            },
            d: C2v {
                x: 2.5 * half,
                y: 1.5 * half,
            },
            t: 1.0,
        };
        compare_out(
            &format!("config 42 ray/AABB SAT miss {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_aabb(sat_miss, corner_box, out)
                } else {
                    c_ray_aabb(sat_miss, corner_box, out)
                }
            },
        );

        let nan = f32::from_bits(0x7fc0_1234);
        let nan_ray = C2Ray {
            p: C2v { x: nan, y: nan },
            d: C2v { x: nan, y: nan },
            t: nan,
        };
        compare_out(
            &format!("config 43 ray/AABB NaN plane miss {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_aabb(nan_ray, corner_box, out)
                } else {
                    c_ray_aabb(nan_ray, corner_box, out)
                }
            },
        );

        let radius = rng.positive();
        let height = radius + rng.positive();
        let capsule = C2Capsule {
            a: C2v { x, y },
            b: C2v { x, y: y + height },
            r: radius,
        };
        let cap_rays = [
            (
                46,
                C2Ray {
                    p: C2v {
                        x,
                        y: y + height / 2.0,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: radius,
                },
            ),
            (
                47,
                C2Ray {
                    p: C2v {
                        x,
                        y: y - radius / 2.0,
                    },
                    d: C2v { x: 0.0, y: -1.0 },
                    t: radius,
                },
            ),
            (
                48,
                C2Ray {
                    p: C2v {
                        x,
                        y: y + height + radius / 2.0,
                    },
                    d: C2v { x: 0.0, y: 1.0 },
                    t: radius,
                },
            ),
            (
                49,
                C2Ray {
                    p: C2v {
                        x: x + 2.0 * radius,
                        y: y + height / 2.0,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: 2.0 * radius,
                },
            ),
            (
                50,
                C2Ray {
                    p: C2v {
                        x: x - 2.0 * radius,
                        y: y + height / 2.0,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: 2.0 * radius,
                },
            ),
            (
                51,
                C2Ray {
                    p: C2v {
                        x,
                        y: y - 3.0 * radius,
                    },
                    d: C2v { x: 0.0, y: 1.0 },
                    t: 4.0 * radius,
                },
            ),
            (
                52,
                C2Ray {
                    p: C2v {
                        x,
                        y: y + height + 3.0 * radius,
                    },
                    d: C2v { x: 0.0, y: -1.0 },
                    t: 4.0 * radius,
                },
            ),
            (
                53,
                C2Ray {
                    p: C2v {
                        x: x + 2.0 * radius,
                        y: y + height / 2.0,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: radius,
                },
            ),
        ];
        for (config, ray) in cap_rays {
            compare_out(
                &format!("config {config} ray/capsule {i}"),
                |is_rust, out| unsafe {
                    if is_rust {
                        r_ray_capsule(ray, capsule, out)
                    } else {
                        c_ray_capsule(ray, capsule, out)
                    }
                },
            );
        }
    }
}

#[test]
fn phase_b_polygon_dispatch_and_wrapper_configurations() {
    let (_libm, c, rust) = unsafe { libraries() };
    let c_ray_poly: RayPolyFn = unsafe { symbol(&c, b"c2RaytoPoly\0") };
    let r_ray_poly: RayPolyFn = unsafe { symbol(&rust, b"c2RaytoPoly\0") };
    let c_cast: CastRayFn = unsafe { symbol(&c, b"c2CastRay\0") };
    let r_cast: CastRayFn = unsafe { symbol(&rust, b"c2CastRay\0") };
    let c_poly_ray: PolyRayFn = unsafe { symbol(&c, b"poly_ray\0") };
    let r_poly_ray: PolyRayFn = unsafe { symbol(&rust, b"poly_ray\0") };
    let mut rng = Rng::new(0xcafe_f00d_dead_beef);

    for i in 0..128 {
        let half = rng.positive();
        let poly = square(half);
        let gap = rng.positive();
        let ray = C2Ray {
            p: C2v {
                x: -half - gap,
                y: 0.0,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: gap + 2.0 * half,
        };
        compare_out(
            &format!("config 59 polygon identity {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(ray, &poly, std::ptr::null(), out)
                } else {
                    c_ray_poly(ray, &poly, std::ptr::null(), out)
                }
            },
        );

        let tx = rng.bounded(-32, 32);
        let ty = rng.bounded(-32, 32);
        let transform = C2x {
            p: C2v { x: tx, y: ty },
            r: C2r { c: 0.0, s: 1.0 },
        };
        let transformed_ray = C2Ray {
            p: C2v {
                x: tx,
                y: ty - half - gap,
            },
            d: C2v { x: 0.0, y: 1.0 },
            t: gap + 2.0 * half,
        };
        compare_out(
            &format!("config 60 polygon transformed {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(transformed_ray, &poly, &transform, out)
                } else {
                    c_ray_poly(transformed_ray, &poly, &transform, out)
                }
            },
        );

        for count in [0, -1] {
            let mut empty = poly;
            empty.count = count;
            compare_out(
                &format!("config 61 polygon count {count} {i}"),
                |is_rust, out| unsafe {
                    if is_rust {
                        r_ray_poly(ray, &empty, std::ptr::null(), out)
                    } else {
                        c_ray_poly(ray, &empty, std::ptr::null(), out)
                    }
                },
            );
        }

        let mut one = poly;
        one.count = 1;
        let one_ray = C2Ray {
            p: C2v {
                x: half + gap,
                y: 0.0,
            },
            d: C2v { x: -1.0, y: 0.0 },
            t: gap + half,
        };
        compare_out(
            &format!("config 62 polygon count 1 {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(one_ray, &one, std::ptr::null(), out)
                } else {
                    c_ray_poly(one_ray, &one, std::ptr::null(), out)
                }
            },
        );

        let mut eight = poly;
        eight.count = 8;
        for edge in 0..4 {
            eight.verts[edge + 4] = eight.verts[edge];
            eight.norms[edge + 4] = eight.norms[edge];
        }
        compare_out(
            &format!("config 62 polygon count 8 {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(ray, &eight, std::ptr::null(), out)
                } else {
                    c_ray_poly(ray, &eight, std::ptr::null(), out)
                }
            },
        );

        let mut nine = C2Poly9 {
            poly: eight,
            ninth_norm: C2v { x: 0.0, y: 0.0 },
        };
        nine.poly.count = 9;
        compare_out(
            &format!("config 63 polygon count 9 {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(ray, &nine.poly, std::ptr::null(), out)
                } else {
                    c_ray_poly(ray, &nine.poly, std::ptr::null(), out)
                }
            },
        );

        let parallel = C2Ray {
            p: C2v {
                x: half + gap,
                y: 0.0,
            },
            d: C2v { x: 0.0, y: 1.0 },
            t: half,
        };
        compare_out(
            &format!("config 64 polygon parallel outside {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(parallel, &poly, std::ptr::null(), out)
                } else {
                    c_ray_poly(parallel, &poly, std::ptr::null(), out)
                }
            },
        );

        let z = C2v { x: 0.0, y: 0.0 };
        let mut impossible = C2Poly {
            count: 2,
            verts: [z; 8],
            norms: [z; 8],
        };
        impossible.verts[0] = C2v { x: -half, y: 0.0 };
        impossible.norms[0] = C2v { x: 1.0, y: 0.0 };
        impossible.verts[1] = C2v { x: half, y: 0.0 };
        impossible.norms[1] = C2v { x: -1.0, y: 0.0 };
        let impossible_ray = C2Ray {
            p: z,
            d: C2v { x: 1.0, y: 0.0 },
            t: half * 4.0,
        };
        compare_out(
            &format!("config 65 polygon hi<lo {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(impossible_ray, &impossible, std::ptr::null(), out)
                } else {
                    c_ray_poly(impossible_ray, &impossible, std::ptr::null(), out)
                }
            },
        );

        let outward = C2Ray {
            p: z,
            d: C2v { x: 1.0, y: 0.0 },
            t: half * 2.0,
        };
        compare_out(
            &format!("config 66 polygon no entering plane {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(outward, &poly, std::ptr::null(), out)
                } else {
                    c_ray_poly(outward, &poly, std::ptr::null(), out)
                }
            },
        );

        let circle = C2Circle { p: z, r: half };
        compare_out(
            &format!("config 67 dispatcher circle {i}"),
            |is_rust, out| unsafe {
                let object = (&raw const circle).cast::<c_void>();
                if is_rust {
                    r_cast(ray, object, std::ptr::null(), 0, out)
                } else {
                    c_cast(ray, object, std::ptr::null(), 0, out)
                }
            },
        );
        let aabb = C2Aabb {
            min: C2v { x: -half, y: -half },
            max: C2v { x: half, y: half },
        };
        compare_out(
            &format!("config 68 dispatcher AABB {i}"),
            |is_rust, out| unsafe {
                let object = (&raw const aabb).cast::<c_void>();
                if is_rust {
                    r_cast(ray, object, std::ptr::null(), 1, out)
                } else {
                    c_cast(ray, object, std::ptr::null(), 1, out)
                }
            },
        );
        let capsule = C2Capsule {
            a: C2v { x: 0.0, y: -half },
            b: C2v { x: 0.0, y: half },
            r: half,
        };
        compare_out(
            &format!("config 69 dispatcher capsule {i}"),
            |is_rust, out| unsafe {
                let object = (&raw const capsule).cast::<c_void>();
                if is_rust {
                    r_cast(ray, object, std::ptr::null(), 2, out)
                } else {
                    c_cast(ray, object, std::ptr::null(), 2, out)
                }
            },
        );
        compare_out(
            &format!("config 70 dispatcher polygon null transform {i}"),
            |is_rust, out| unsafe {
                let object = (&raw const poly).cast::<c_void>();
                if is_rust {
                    r_cast(ray, object, std::ptr::null(), 3, out)
                } else {
                    c_cast(ray, object, std::ptr::null(), 3, out)
                }
            },
        );
        compare_out(
            &format!("config 71 dispatcher polygon transform {i}"),
            |is_rust, out| unsafe {
                let object = (&raw const poly).cast::<c_void>();
                if is_rust {
                    r_cast(transformed_ray, object, &transform, 3, out)
                } else {
                    c_cast(transformed_ray, object, &transform, 3, out)
                }
            },
        );
        for invalid in [-1, 4, c_int::MIN, c_int::MAX] {
            compare_out(
                &format!("config 72 invalid dispatcher {invalid} {i}"),
                |is_rust, out| unsafe {
                    if is_rust {
                        r_cast(ray, std::ptr::null(), std::ptr::null(), invalid, out)
                    } else {
                        c_cast(ray, std::ptr::null(), std::ptr::null(), invalid, out)
                    }
                },
            );
        }
    }

    for i in 0..128 {
        let mut c1 = sentinel();
        let mut c2 = sentinel();
        let mut r1 = sentinel();
        let mut r2 = sentinel();
        unsafe {
            assert_same(
                &format!("config 73 poly_ray return {i}"),
                c_poly_ray(&mut c1, &mut c2),
                r_poly_ray(&mut r1, &mut r2),
            );
        }
        assert_same(&format!("config 73 poly_ray cast1 {i}"), c1, r1);
        assert_same(&format!("config 73 poly_ray cast2 {i}"), c2, r2);
    }
}

#[test]
fn phase_c_all_error_surface_rows() {
    let (_libm, c, rust) = unsafe { libraries() };
    let c_ray_circle: RayCircleFn = unsafe { symbol(&c, b"c2RaytoCircle\0") };
    let r_ray_circle: RayCircleFn = unsafe { symbol(&rust, b"c2RaytoCircle\0") };
    let c_aabb: AabbAabbFn = unsafe { symbol(&c, b"c2AABBtoAABB\0") };
    let r_aabb: AabbAabbFn = unsafe { symbol(&rust, b"c2AABBtoAABB\0") };
    let c_ray_aabb: RayAabbFn = unsafe { symbol(&c, b"c2RaytoAABB\0") };
    let r_ray_aabb: RayAabbFn = unsafe { symbol(&rust, b"c2RaytoAABB\0") };
    let c_point: AabbPointFn = unsafe { symbol(&c, b"c2AABBtoPoint\0") };
    let r_point: AabbPointFn = unsafe { symbol(&rust, b"c2AABBtoPoint\0") };
    let c_circle_point: CirclePointFn = unsafe { symbol(&c, b"c2CircleToPoint\0") };
    let r_circle_point: CirclePointFn = unsafe { symbol(&rust, b"c2CircleToPoint\0") };
    let c_ray_capsule: RayCapsuleFn = unsafe { symbol(&c, b"c2RaytoCapsule\0") };
    let r_ray_capsule: RayCapsuleFn = unsafe { symbol(&rust, b"c2RaytoCapsule\0") };
    let c_ray_poly: RayPolyFn = unsafe { symbol(&c, b"c2RaytoPoly\0") };
    let r_ray_poly: RayPolyFn = unsafe { symbol(&rust, b"c2RaytoPoly\0") };
    let c_cast: CastRayFn = unsafe { symbol(&c, b"c2CastRay\0") };
    let r_cast: CastRayFn = unsafe { symbol(&rust, b"c2CastRay\0") };
    let mut rng = Rng::new(0x0bad_f00d_1234_4321);

    for i in 0..128 {
        let x = rng.bounded(-32, 32);
        let y = rng.bounded(-32, 32);
        let radius = rng.positive();
        let gap = rng.positive();
        let circle = C2Circle {
            p: C2v { x, y },
            r: radius,
        };
        let disc_miss = C2Ray {
            p: C2v {
                x: x - radius - gap,
                y: y + radius + gap,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: radius + gap,
        };
        compare_rejection(
            &format!("error 1 negative discriminant {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_circle(disc_miss, circle, out)
                } else {
                    c_ray_circle(disc_miss, circle, out)
                }
            },
        );
        let behind = C2Ray {
            p: C2v {
                x: x + radius + gap,
                y,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: radius + gap,
        };
        compare_rejection(
            &format!("error 2 t below zero {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_circle(behind, circle, out)
                } else {
                    c_ray_circle(behind, circle, out)
                }
            },
        );
        let beyond = C2Ray {
            p: C2v {
                x: x - radius - gap,
                y,
            },
            d: C2v { x: 1.0, y: 0.0 },
            t: gap / 2.0,
        };
        compare_rejection(&format!("error 3 t above A.t {i}"), |is_rust, out| unsafe {
            if is_rust {
                r_ray_circle(beyond, circle, out)
            } else {
                c_ray_circle(beyond, circle, out)
            }
        });

        let w = rng.positive();
        let h = rng.positive();
        let aabb = C2Aabb {
            min: C2v { x, y },
            max: C2v { x: x + w, y: y + h },
        };
        let separated = [
            C2Aabb {
                min: C2v { x: x - gap - w, y },
                max: C2v {
                    x: x - gap,
                    y: y + h,
                },
            },
            C2Aabb {
                min: C2v { x: x + w + gap, y },
                max: C2v {
                    x: x + 2.0 * w + gap,
                    y: y + h,
                },
            },
            C2Aabb {
                min: C2v { x, y: y - gap - h },
                max: C2v {
                    x: x + w,
                    y: y - gap,
                },
            },
            C2Aabb {
                min: C2v { x, y: y + h + gap },
                max: C2v {
                    x: x + w,
                    y: y + 2.0 * h + gap,
                },
            },
        ];
        for (offset, other) in separated.into_iter().enumerate() {
            let c_result = unsafe { c_aabb(aabb, other) };
            let r_result = unsafe { r_aabb(aabb, other) };
            assert_eq!(c_result, 0, "error {} C did not reject", 4 + offset);
            assert_same(
                &format!("error {} AABB separation {i}", 4 + offset),
                c_result,
                r_result,
            );
        }

        let bbox_miss = C2Ray {
            p: C2v {
                x: x - gap,
                y: y + h / 2.0,
            },
            d: C2v { x: -1.0, y: 0.0 },
            t: gap,
        };
        compare_rejection(
            &format!("error 8 ray bbox miss {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_aabb(bbox_miss, aabb, out)
                } else {
                    c_ray_aabb(bbox_miss, aabb, out)
                }
            },
        );
        let half = rng.positive();
        let centered = C2Aabb {
            min: C2v {
                x: x - half,
                y: y - half,
            },
            max: C2v {
                x: x + half,
                y: y + half,
            },
        };
        let sat_miss = C2Ray {
            p: C2v {
                x: x - 2.0 * half,
                y: y + 0.5 * half,
            },
            d: C2v {
                x: 2.5 * half,
                y: 1.5 * half,
            },
            t: 1.0,
        };
        compare_rejection(
            &format!("error 9 ray SAT miss {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_aabb(sat_miss, centered, out)
                } else {
                    c_ray_aabb(sat_miss, centered, out)
                }
            },
        );
        let nan = f32::from_bits(0x7fc0_1234);
        let nan_ray = C2Ray {
            p: C2v { x: nan, y: nan },
            d: C2v { x: nan, y: nan },
            t: nan,
        };
        compare_rejection(
            &format!("error 10 all plane comparisons false {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_aabb(nan_ray, centered, out)
                } else {
                    c_ray_aabb(nan_ray, centered, out)
                }
            },
        );

        let outside_points = [
            C2v { x: x - gap, y },
            C2v { x, y: y - gap },
            C2v { x: x + w + gap, y },
            C2v { x, y: y + h + gap },
        ];
        for (offset, point) in outside_points.into_iter().enumerate() {
            let c_result = unsafe { c_point(aabb, point) };
            let r_result = unsafe { r_point(aabb, point) };
            assert_eq!(c_result, 0, "error {} C did not reject", 11 + offset);
            assert_same(
                &format!("error {} point outside {i}", 11 + offset),
                c_result,
                r_result,
            );
        }
        let boundary = C2v { x: x + radius, y };
        let c_result = unsafe { c_circle_point(circle, boundary) };
        let r_result = unsafe { r_circle_point(circle, boundary) };
        assert_eq!(c_result, 0, "error 15 C did not reject boundary");
        assert_same(&format!("error 15 circle boundary {i}"), c_result, r_result);

        let height = radius + rng.positive();
        let capsule = C2Capsule {
            a: C2v { x, y },
            b: C2v { x, y: y + height },
            r: radius,
        };
        let capsule_rejections = [
            (
                16,
                C2Ray {
                    p: C2v {
                        x: x + 2.0 * radius,
                        y: y + height / 2.0,
                    },
                    d: C2v { x: 1.0, y: 0.0 },
                    t: radius,
                },
            ),
            (
                17,
                C2Ray {
                    p: C2v {
                        x,
                        y: y - 3.0 * radius,
                    },
                    d: C2v { x: 0.0, y: -1.0 },
                    t: radius,
                },
            ),
            (
                18,
                C2Ray {
                    p: C2v {
                        x,
                        y: y + height + 3.0 * radius,
                    },
                    d: C2v { x: 0.0, y: 1.0 },
                    t: radius,
                },
            ),
            (
                19,
                C2Ray {
                    p: C2v {
                        x: x + 2.0 * radius,
                        y: y - 2.0 * radius,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: 4.0 * radius,
                },
            ),
            (
                20,
                C2Ray {
                    p: C2v {
                        x: x + 2.0 * radius,
                        y: y + height + 2.0 * radius,
                    },
                    d: C2v { x: -1.0, y: 0.0 },
                    t: 4.0 * radius,
                },
            ),
        ];
        for (error, ray) in capsule_rejections {
            compare_rejection(
                &format!("error {error} capsule rejection {i}"),
                |is_rust, out| unsafe {
                    if is_rust {
                        r_ray_capsule(ray, capsule, out)
                    } else {
                        c_ray_capsule(ray, capsule, out)
                    }
                },
            );
        }

        let poly = square(half);
        let parallel = C2Ray {
            p: C2v {
                x: half + gap,
                y: 0.0,
            },
            d: C2v { x: 0.0, y: 1.0 },
            t: half,
        };
        compare_rejection(
            &format!("error 21 polygon parallel outside {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(parallel, &poly, std::ptr::null(), out)
                } else {
                    c_ray_poly(parallel, &poly, std::ptr::null(), out)
                }
            },
        );
        let z = C2v { x: 0.0, y: 0.0 };
        let mut impossible = C2Poly {
            count: 2,
            verts: [z; 8],
            norms: [z; 8],
        };
        impossible.verts[0] = C2v { x: -half, y: 0.0 };
        impossible.norms[0] = C2v { x: 1.0, y: 0.0 };
        impossible.verts[1] = C2v { x: half, y: 0.0 };
        impossible.norms[1] = C2v { x: -1.0, y: 0.0 };
        let outward = C2Ray {
            p: z,
            d: C2v { x: 1.0, y: 0.0 },
            t: half * 4.0,
        };
        compare_rejection(
            &format!("error 22 polygon hi<lo {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(outward, &impossible, std::ptr::null(), out)
                } else {
                    c_ray_poly(outward, &impossible, std::ptr::null(), out)
                }
            },
        );
        compare_rejection(
            &format!("error 23 polygon no entering {i}"),
            |is_rust, out| unsafe {
                if is_rust {
                    r_ray_poly(outward, &poly, std::ptr::null(), out)
                } else {
                    c_ray_poly(outward, &poly, std::ptr::null(), out)
                }
            },
        );
        for count in [0, -1] {
            let mut empty = poly;
            empty.count = count;
            compare_rejection(
                &format!("error 24 polygon count {count} {i}"),
                |is_rust, out| unsafe {
                    if is_rust {
                        r_ray_poly(outward, &empty, std::ptr::null(), out)
                    } else {
                        c_ray_poly(outward, &empty, std::ptr::null(), out)
                    }
                },
            );
        }
        for invalid in [-1, 4, c_int::MIN, c_int::MAX] {
            compare_rejection(
                &format!("error 25 invalid enum {invalid} {i}"),
                |is_rust, out| unsafe {
                    if is_rust {
                        r_cast(outward, std::ptr::null(), std::ptr::null(), invalid, out)
                    } else {
                        c_cast(outward, std::ptr::null(), std::ptr::null(), invalid, out)
                    }
                },
            );
        }
    }
}

#[test]
fn phase_c_generic_zero_oversized_and_safe_null_boundaries() {
    let (_libm, c, rust) = unsafe { libraries() };
    let c_ray_circle: RayCircleFn = unsafe { symbol(&c, b"c2RaytoCircle\0") };
    let r_ray_circle: RayCircleFn = unsafe { symbol(&rust, b"c2RaytoCircle\0") };
    let c_ray_aabb: RayAabbFn = unsafe { symbol(&c, b"c2RaytoAABB\0") };
    let r_ray_aabb: RayAabbFn = unsafe { symbol(&rust, b"c2RaytoAABB\0") };
    let c_ray_capsule: RayCapsuleFn = unsafe { symbol(&c, b"c2RaytoCapsule\0") };
    let r_ray_capsule: RayCapsuleFn = unsafe { symbol(&rust, b"c2RaytoCapsule\0") };
    let c_ray_poly: RayPolyFn = unsafe { symbol(&c, b"c2RaytoPoly\0") };
    let r_ray_poly: RayPolyFn = unsafe { symbol(&rust, b"c2RaytoPoly\0") };
    let c_cast: CastRayFn = unsafe { symbol(&c, b"c2CastRay\0") };
    let r_cast: CastRayFn = unsafe { symbol(&rust, b"c2CastRay\0") };
    let c_poly_ray: PolyRayFn = unsafe { symbol(&c, b"poly_ray\0") };
    let r_poly_ray: PolyRayFn = unsafe { symbol(&rust, b"poly_ray\0") };

    let circle = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    let aabb = C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let capsule = C2Capsule {
        a: C2v { x: 0.0, y: -1.0 },
        b: C2v { x: 0.0, y: 1.0 },
        r: 1.0,
    };
    let poly = square(1.0);
    let zero_length = C2Ray {
        p: C2v { x: -3.0, y: 0.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: 0.0,
    };
    let oversized = C2Ray {
        p: C2v { x: -3.0, y: 0.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: f32::MAX,
    };
    for (label, ray) in [
        ("zero length", zero_length),
        ("oversized length", oversized),
    ] {
        compare_out(&format!("{label} circle"), |is_rust, out| unsafe {
            if is_rust {
                r_ray_circle(ray, circle, out)
            } else {
                c_ray_circle(ray, circle, out)
            }
        });
        compare_out(&format!("{label} AABB"), |is_rust, out| unsafe {
            if is_rust {
                r_ray_aabb(ray, aabb, out)
            } else {
                c_ray_aabb(ray, aabb, out)
            }
        });
        compare_out(&format!("{label} capsule"), |is_rust, out| unsafe {
            if is_rust {
                r_ray_capsule(ray, capsule, out)
            } else {
                c_ray_capsule(ray, capsule, out)
            }
        });
        compare_out(&format!("{label} polygon"), |is_rust, out| unsafe {
            if is_rust {
                r_ray_poly(ray, &poly, std::ptr::null(), out)
            } else {
                c_ray_poly(ray, &poly, std::ptr::null(), out)
            }
        });
    }

    let miss = C2Ray {
        p: C2v { x: -3.0, y: 3.0 },
        d: C2v { x: -1.0, y: 0.0 },
        t: 1.0,
    };
    unsafe {
        assert_same(
            "null out safe circle miss",
            c_ray_circle(miss, circle, std::ptr::null_mut()),
            r_ray_circle(miss, circle, std::ptr::null_mut()),
        );
        assert_same(
            "null out safe AABB miss",
            c_ray_aabb(miss, aabb, std::ptr::null_mut()),
            r_ray_aabb(miss, aabb, std::ptr::null_mut()),
        );
        assert_same(
            "null out safe polygon miss",
            c_ray_poly(miss, &poly, std::ptr::null(), std::ptr::null_mut()),
            r_ray_poly(miss, &poly, std::ptr::null(), std::ptr::null_mut()),
        );
        assert_same(
            "all-null invalid dispatcher",
            c_cast(
                miss,
                std::ptr::null(),
                std::ptr::null(),
                4,
                std::ptr::null_mut(),
            ),
            r_cast(
                miss,
                std::ptr::null(),
                std::ptr::null(),
                4,
                std::ptr::null_mut(),
            ),
        );
    }

    let mut c_second = sentinel();
    let mut r_second = sentinel();
    unsafe {
        assert_same(
            "poly_ray null cast1 return",
            c_poly_ray(std::ptr::null_mut(), &mut c_second),
            r_poly_ray(std::ptr::null_mut(), &mut r_second),
        );
    }
    assert_same("poly_ray null cast1 surviving output", c_second, r_second);
    let mut c_first = sentinel();
    let mut r_first = sentinel();
    unsafe {
        assert_same(
            "poly_ray null cast2 return",
            c_poly_ray(&mut c_first, std::ptr::null_mut()),
            r_poly_ray(&mut r_first, std::ptr::null_mut()),
        );
        assert_same(
            "poly_ray both outputs null return",
            c_poly_ray(std::ptr::null_mut(), std::ptr::null_mut()),
            r_poly_ray(std::ptr::null_mut(), std::ptr::null_mut()),
        );
    }
    assert_same("poly_ray null cast2 surviving output", c_first, r_first);

    let mut eight = poly;
    eight.count = 8;
    for edge in 0..4 {
        eight.verts[edge + 4] = eight.verts[edge];
        eight.norms[edge + 4] = eight.norms[edge];
    }
    let nine = C2Poly9 {
        poly: C2Poly { count: 9, ..eight },
        ninth_norm: C2v { x: 0.0, y: 0.0 },
    };
    compare_out("one-past-capacity polygon count", |is_rust, out| unsafe {
        if is_rust {
            r_ray_poly(oversized, &nine.poly, std::ptr::null(), out)
        } else {
            c_ray_poly(oversized, &nine.poly, std::ptr::null(), out)
        }
    });
}

#[test]
fn ffi_crash_probe() {
    let Ok(kind) = std::env::var("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_CRASH_CASE").unwrap();
    let libm: Library = unsafe {
        libloading::os::unix::Library::open(
            Some("libm.so.6"),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
        )
        .unwrap()
        .into()
    };
    let path = if kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let library = unsafe { Library::new(path).unwrap() };
    let ray = C2Ray {
        p: C2v { x: -3.0, y: 0.0 },
        d: C2v { x: 1.0, y: 0.0 },
        t: 4.0,
    };
    let circle = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    let aabb = C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    let capsule = C2Capsule {
        a: C2v { x: 0.0, y: -1.0 },
        b: C2v { x: 0.0, y: 1.0 },
        r: 1.0,
    };
    let poly = square(1.0);
    let mut out = sentinel();
    unsafe {
        match case.as_str() {
            "ray_circle_out_null" => {
                let f: RayCircleFn = symbol(&library, b"c2RaytoCircle\0");
                f(ray, circle, std::ptr::null_mut());
            }
            "ray_aabb_out_null" => {
                let f: RayAabbFn = symbol(&library, b"c2RaytoAABB\0");
                f(ray, aabb, std::ptr::null_mut());
            }
            "ray_capsule_out_null" => {
                let f: RayCapsuleFn = symbol(&library, b"c2RaytoCapsule\0");
                f(ray, capsule, std::ptr::null_mut());
            }
            "ray_poly_b_null" => {
                let f: RayPolyFn = symbol(&library, b"c2RaytoPoly\0");
                f(ray, std::ptr::null(), std::ptr::null(), &mut out);
            }
            "ray_poly_out_null" => {
                let f: RayPolyFn = symbol(&library, b"c2RaytoPoly\0");
                f(ray, &poly, std::ptr::null(), std::ptr::null_mut());
            }
            "cast_b_null" => {
                let f: CastRayFn = symbol(&library, b"c2CastRay\0");
                f(ray, std::ptr::null(), std::ptr::null(), 0, &mut out);
            }
            "cast_out_null" => {
                let f: CastRayFn = symbol(&library, b"c2CastRay\0");
                f(
                    ray,
                    (&raw const circle).cast::<c_void>(),
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                );
            }
            _ => panic!("unknown crash case"),
        }
    }
    std::hint::black_box(libm);
}

#[test]
fn phase_c_null_dereference_process_parity() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let executable = std::env::current_exe().unwrap();
    let cases = [
        "ray_circle_out_null",
        "ray_aabb_out_null",
        "ray_capsule_out_null",
        "ray_poly_b_null",
        "ray_poly_out_null",
        "cast_b_null",
        "cast_out_null",
    ];
    for case in cases {
        let run = |kind: &str| {
            Command::new(&executable)
                .arg("--exact")
                .arg("ffi_crash_probe")
                .arg("--nocapture")
                .env("DIFF_CRASH_LIBRARY", kind)
                .env("DIFF_CRASH_CASE", case)
                .status()
                .unwrap()
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(!c_status.success(), "{case}: C unexpectedly survived");
        assert!(!rust_status.success(), "{case}: Rust unexpectedly survived");
        assert_eq!(
            (c_status.code(), c_status.signal()),
            (rust_status.code(), rust_status.signal()),
            "{case}: different process termination"
        );
    }
}
