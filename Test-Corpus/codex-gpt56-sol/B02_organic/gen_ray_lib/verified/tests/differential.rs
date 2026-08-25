use libloading::Library;
#[cfg(target_arch = "x86_64")]
use std::arch::asm;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct V {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Raycast {
    t: f32,
    n: V,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Circle {
    p: V,
    r: f32,
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
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Ray {
    p: V,
    d: V,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct M {
    x: V,
    y: V,
}

type V2 = unsafe extern "C" fn(f32, f32) -> V;
type VV = unsafe extern "C" fn(V, V) -> V;
type VF = unsafe extern "C" fn(V) -> f32;
type VVF = unsafe extern "C" fn(V, V) -> f32;
type VS = unsafe extern "C" fn(V, f32) -> V;
type V1 = unsafe extern "C" fn(V) -> V;
type AabbAabb = unsafe extern "C" fn(Aabb, Aabb) -> c_int;
type MV = unsafe extern "C" fn(M, V) -> V;
type AabbPoint = unsafe extern "C" fn(Aabb, V) -> c_int;
type CirclePoint = unsafe extern "C" fn(Circle, V) -> c_int;
type RayCircle = unsafe extern "C" fn(Ray, Circle, *mut Raycast) -> c_int;
type RayAabb = unsafe extern "C" fn(Ray, Aabb, *mut Raycast) -> c_int;
type RayCapsule = unsafe extern "C" fn(Ray, Capsule, *mut Raycast) -> c_int;
type CastRay = unsafe extern "C" fn(Ray, *const c_void, c_int, *mut Raycast) -> c_int;
type GenRay = unsafe extern "C" fn(
    *mut Raycast,
    *mut Raycast,
    *mut Raycast,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
) -> c_int;

struct Api {
    _library: Library,
    c2_v: V2,
    c2_dot: VVF,
    c2_len: VF,
    c2_add: VV,
    c2_sub: VV,
    c2_mulvs: VS,
    c2_div: VS,
    c2_norm: V1,
    c2_minv: VV,
    c2_maxv: VV,
    c2_skew: V1,
    c2_absv: V1,
    ray_circle: RayCircle,
    aabb_aabb: AabbAabb,
    ray_aabb: RayAabb,
    c2_ccw90: V1,
    c2_mulmvt: MV,
    aabb_point: AabbPoint,
    circle_point: CirclePoint,
    ray_capsule: RayCapsule,
    cast_ray: CastRay,
    gen_ray: GenRay,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
            };
        }
        Self {
            c2_v: symbol!("c2V", V2),
            c2_dot: symbol!("c2Dot", VVF),
            c2_len: symbol!("c2Len", VF),
            c2_add: symbol!("c2Add", VV),
            c2_sub: symbol!("c2Sub", VV),
            c2_mulvs: symbol!("c2Mulvs", VS),
            c2_div: symbol!("c2Div", VS),
            c2_norm: symbol!("c2Norm", V1),
            c2_minv: symbol!("c2Minv", VV),
            c2_maxv: symbol!("c2Maxv", VV),
            c2_skew: symbol!("c2Skew", V1),
            c2_absv: symbol!("c2Absv", V1),
            ray_circle: symbol!("c2RaytoCircle", RayCircle),
            aabb_aabb: symbol!("c2AABBtoAABB", AabbAabb),
            ray_aabb: symbol!("c2RaytoAABB", RayAabb),
            c2_ccw90: symbol!("c2CCW90", V1),
            c2_mulmvt: symbol!("c2MulmvT", MV),
            aabb_point: symbol!("c2AABBtoPoint", AabbPoint),
            circle_point: symbol!("c2CircleToPoint", CirclePoint),
            ray_capsule: symbol!("c2RaytoCapsule", RayCapsule),
            cast_ray: symbol!("c2CastRay", CastRay),
            gen_ray: symbol!("gen_ray", GenRay),
            _library: library,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = root.join("c_src/build/libtranslated_rust.so");
    static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();
    let rust = RUST_LIBRARY
        .get_or_init(|| {
            let output_dir = root.join("target/differential");
            std::fs::create_dir_all(&output_dir)
                .unwrap_or_else(|error| panic!("cannot create {}: {error}", output_dir.display()));
            let output = output_dir.join("libgen_ray_lib.so");
            let result = Command::new("rustc")
                .current_dir(&root)
                .args([
                    "--crate-name",
                    "gen_ray_lib",
                    "--crate-type",
                    "cdylib",
                    "--edition",
                    "2024",
                    "-C",
                    "panic=abort",
                    "src/lib.rs",
                    "-o",
                ])
                .arg(&output)
                .output()
                .expect("failed to execute rustc for differential cdylib");
            assert!(
                result.status.success(),
                "rustc failed:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
            output
        })
        .clone();
    assert!(c.exists(), "C library not found at {}", c.display());
    (c, rust)
}

fn apis() -> (Api, Api) {
    let (c, rust) = library_paths();
    unsafe { (Api::load(&c), Api::load(&rust)) }
}

fn v(x: f32, y: f32) -> V {
    V { x, y }
}

fn sentinel(seed: u32) -> Raycast {
    Raycast {
        t: f32::from_bits(0x3f00_0000 | (seed & 0x007f_ffff)),
        n: v(
            f32::from_bits(0x4000_0000 | (seed & 0x007f_ffff)),
            f32::from_bits(0x4040_0000 | (seed & 0x003f_ffff)),
        ),
    }
}

fn bits_v(value: V) -> [u32; 2] {
    [value.x.to_bits(), value.y.to_bits()]
}

fn bits_cast(value: Raycast) -> [u32; 3] {
    [value.t.to_bits(), value.n.x.to_bits(), value.n.y.to_bits()]
}

fn assert_v(row: &str, c: V, rust: V) {
    assert_eq!(bits_v(c), bits_v(rust), "{row}: C={c:?}, Rust={rust:?}");
}

fn assert_f(row: &str, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{row}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_cast(row: &str, c_ret: c_int, c_out: Raycast, r_ret: c_int, r_out: Raycast) {
    assert_eq!(c_ret, r_ret, "{row}: return value");
    assert_eq!(
        bits_cast(c_out),
        bits_cast(r_out),
        "{row}: C={c_out:?}, Rust={r_out:?}"
    );
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

    fn finite(&mut self, low: f32, high: f32) -> f32 {
        let unit = (self.next_u32() >> 8) as f32 / ((1_u32 << 24) - 1) as f32;
        low + (high - low) * unit
    }
}

unsafe fn compare_ray_circle(
    row: &str,
    c: &Api,
    rust: &Api,
    ray: Ray,
    circle: Circle,
    seed: u32,
) -> c_int {
    let mut c_out = sentinel(seed);
    let mut r_out = c_out;
    let c_ret = unsafe { (c.ray_circle)(ray, circle, &mut c_out) };
    let r_ret = unsafe { (rust.ray_circle)(ray, circle, &mut r_out) };
    assert_cast(row, c_ret, c_out, r_ret, r_out);
    c_ret
}

unsafe fn compare_ray_aabb(
    row: &str,
    c: &Api,
    rust: &Api,
    ray: Ray,
    aabb: Aabb,
    seed: u32,
) -> c_int {
    let mut c_out = sentinel(seed);
    let mut r_out = c_out;
    let c_ret = unsafe { (c.ray_aabb)(ray, aabb, &mut c_out) };
    let r_ret = unsafe { (rust.ray_aabb)(ray, aabb, &mut r_out) };
    assert_cast(row, c_ret, c_out, r_ret, r_out);
    c_ret
}

unsafe fn compare_ray_capsule(
    row: &str,
    c: &Api,
    rust: &Api,
    ray: Ray,
    capsule: Capsule,
    seed: u32,
) -> c_int {
    let mut c_out = sentinel(seed);
    let mut r_out = c_out;
    let c_ret = unsafe { (c.ray_capsule)(ray, capsule, &mut c_out) };
    let r_ret = unsafe { (rust.ray_capsule)(ray, capsule, &mut r_out) };
    assert_cast(row, c_ret, c_out, r_ret, r_out);
    c_ret
}

#[test]
fn scalar_vector_and_point_surface_c01_c31() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);

    for i in 0..256 {
        let a = v(rng.finite(-50.0, 50.0), rng.finite(-50.0, 50.0));
        let b = v(rng.finite(-50.0, 50.0), rng.finite(-50.0, 50.0));
        let s = rng.finite(-10.0, 10.0);
        unsafe {
            assert_v("C01", (c.c2_v)(a.x, a.y), (rust.c2_v)(a.x, a.y));
            assert_f("C02", (c.c2_dot)(a, b), (rust.c2_dot)(a, b));
            assert_f("C03", (c.c2_len)(a), (rust.c2_len)(a));
            assert_v("C04", (c.c2_add)(a, b), (rust.c2_add)(a, b));
            assert_v("C05", (c.c2_sub)(a, b), (rust.c2_sub)(a, b));
            assert_v("C06", (c.c2_mulvs)(a, s), (rust.c2_mulvs)(a, s));
            let divisor = if s.abs() < 0.01 { 0.25 } else { s };
            assert_v("C07", (c.c2_div)(a, divisor), (rust.c2_div)(a, divisor));
            assert_v("C08", (c.c2_norm)(a), (rust.c2_norm)(a));
            assert_v("C09", (c.c2_minv)(a, b), (rust.c2_minv)(a, b));
            assert_v("C11", (c.c2_maxv)(a, b), (rust.c2_maxv)(a, b));
            assert_v("C13", (c.c2_skew)(a), (rust.c2_skew)(a));
            assert_v("C14", (c.c2_absv)(a), (rust.c2_absv)(a));
            assert_v("C21", (c.c2_ccw90)(a), (rust.c2_ccw90)(a));
            let m = M { x: a, y: b };
            assert_v("C22", (c.c2_mulmvt)(m, a), (rust.c2_mulmvt)(m, a));
        }
        assert!(i < 256);
    }

    let special = [
        v(0.0, -0.0),
        v(-0.0, 0.0),
        v(f32::INFINITY, f32::NEG_INFINITY),
        v(f32::from_bits(0x7fc0_1234), 1.0),
        v(1.0, f32::from_bits(0x7fc0_5678)),
    ];
    for &a in &special {
        for &b in &special {
            unsafe {
                assert_v("C01", (c.c2_v)(a.x, a.y), (rust.c2_v)(a.x, a.y));
                assert_v("C10", (c.c2_minv)(a, b), (rust.c2_minv)(a, b));
                assert_v("C12", (c.c2_maxv)(a, b), (rust.c2_maxv)(a, b));
                assert_v("C13", (c.c2_skew)(a), (rust.c2_skew)(a));
                assert_v("C14", (c.c2_absv)(a), (rust.c2_absv)(a));
                assert_v("C21", (c.c2_ccw90)(a), (rust.c2_ccw90)(a));
            }
        }
    }
    unsafe {
        assert_f("C03", (c.c2_len)(v(0.0, 0.0)), (rust.c2_len)(v(0.0, 0.0)));
        assert_v(
            "C06",
            (c.c2_mulvs)(v(3.0, -4.0), 0.0),
            (rust.c2_mulvs)(v(3.0, -4.0), 0.0),
        );
        for divisor in [0.0, -0.0] {
            assert_v(
                "C07",
                (c.c2_div)(v(3.0, -4.0), divisor),
                (rust.c2_div)(v(3.0, -4.0), divisor),
            );
        }
        assert_v("C08", (c.c2_norm)(v(0.0, 0.0)), (rust.c2_norm)(v(0.0, 0.0)));
    }

    for i in 0..192 {
        let cx = rng.finite(-50.0, 50.0);
        let cy = rng.finite(-50.0, 50.0);
        let width = rng.finite(0.5, 10.0);
        let height = rng.finite(0.5, 10.0);
        let gap = rng.finite(0.01, 5.0);
        let base = Aabb {
            min: v(cx - width, cy - height),
            max: v(cx + width, cy + height),
        };
        let boxes = [
            (
                "C15",
                Aabb {
                    min: v(cx - width * 0.5, cy - height * 0.5),
                    max: v(cx + width * 0.5, cy + height * 0.5),
                },
                1,
            ),
            (
                "C16",
                Aabb {
                    min: v(base.max.x, base.max.y),
                    max: v(base.max.x + width, base.max.y + height),
                },
                1,
            ),
            (
                "C17/E04",
                Aabb {
                    min: v(base.min.x - width - gap, cy - height * 0.5),
                    max: v(base.min.x - gap, cy + height * 0.5),
                },
                0,
            ),
            (
                "C18/E05",
                Aabb {
                    min: v(base.max.x + gap, cy - height * 0.5),
                    max: v(base.max.x + width + gap, cy + height * 0.5),
                },
                0,
            ),
            (
                "C19/E06",
                Aabb {
                    min: v(cx - width * 0.5, base.min.y - height - gap),
                    max: v(cx + width * 0.5, base.min.y - gap),
                },
                0,
            ),
            (
                "C20/E07",
                Aabb {
                    min: v(cx - width * 0.5, base.max.y + gap),
                    max: v(cx + width * 0.5, base.max.y + height + gap),
                },
                0,
            ),
        ];
        for (row, other, expected) in boxes {
            unsafe {
                let c_value = (c.aabb_aabb)(base, other);
                let r_value = (rust.aabb_aabb)(base, other);
                assert_eq!(
                    (c_value, r_value),
                    (expected, expected),
                    "{row} iteration {i}"
                );
            }
        }

        let points = [
            ("C23", v(cx, cy), 1),
            ("C24", base.min, 1),
            ("C24", base.max, 1),
            ("C25/E11", v(base.min.x - gap, cy), 0),
            ("C26/E12", v(cx, base.min.y - gap), 0),
            ("C27/E13", v(base.max.x + gap, cy), 0),
            ("C28/E14", v(cx, base.max.y + gap), 0),
        ];
        for (row, point, expected) in points {
            unsafe {
                let c_value = (c.aabb_point)(base, point);
                let r_value = (rust.aabb_point)(base, point);
                assert_eq!(
                    (c_value, r_value),
                    (expected, expected),
                    "{row} iteration {i}"
                );
            }
        }
    }

    for i in 0..128 {
        let radius = rng.finite(0.25, 20.0);
        let center = v(rng.finite(-10.0, 10.0), rng.finite(-10.0, 10.0));
        let circle = Circle {
            p: center,
            r: radius,
        };
        for (row, point, expected) in [
            ("C29", v(center.x + radius * 0.5, center.y), 1),
            ("C31/E15", v(center.x + radius * 1.5, center.y), 0),
        ] {
            unsafe {
                let c_value = (c.circle_point)(circle, point);
                let r_value = (rust.circle_point)(circle, point);
                assert_eq!(
                    (c_value, r_value),
                    (expected, expected),
                    "{row} iteration {i}"
                );
            }
        }
        let boundary_circle = Circle {
            p: v(0.0, 0.0),
            r: radius,
        };
        unsafe {
            let point = v(radius, 0.0);
            let c_value = (c.circle_point)(boundary_circle, point);
            let r_value = (rust.circle_point)(boundary_circle, point);
            assert_eq!((c_value, r_value), (0, 0), "C30/E15 boundary iteration {i}");
        }
    }
}

#[test]
fn ray_circle_surface_c32_c37_and_e01_e03() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    for i in 0..192_u32 {
        let y = rng.finite(2.0, 20.0);
        let radius = rng.finite(0.1, y * 0.8);
        let distance = rng.finite(2.0, 30.0);
        let ray = Ray {
            p: v(0.0, 0.0),
            d: v(1.0, 0.0),
            t: distance + 20.0,
        };
        let result = unsafe {
            compare_ray_circle(
                "C32/E01",
                &c,
                &rust,
                ray,
                Circle {
                    p: v(distance, y),
                    r: radius,
                },
                i,
            )
        };
        assert_eq!(result, 0, "C32/E01 iteration {i}");

        let result = unsafe {
            compare_ray_circle(
                "C33/E02",
                &c,
                &rust,
                ray,
                Circle {
                    p: v(-distance, 0.0),
                    r: radius,
                },
                i ^ 0x1111,
            )
        };
        assert_eq!(result, 0, "C33/E02 iteration {i}");

        let short_ray = Ray { t: 0.5, ..ray };
        let result = unsafe {
            compare_ray_circle(
                "C34/E03",
                &c,
                &rust,
                short_ray,
                Circle {
                    p: v(distance, 0.0),
                    r: radius.min(distance * 0.25),
                },
                i ^ 0x2222,
            )
        };
        assert_eq!(result, 0, "C34/E03 iteration {i}");

        let hit_radius = rng.finite(0.25, distance * 0.4);
        let result = unsafe {
            compare_ray_circle(
                "C35",
                &c,
                &rust,
                ray,
                Circle {
                    p: v(distance, 0.0),
                    r: hit_radius,
                },
                i ^ 0x3333,
            )
        };
        assert_eq!(result, 1, "C35 iteration {i}");
    }

    for i in 1..=128_u32 {
        let distance = (i % 13 + 2) as f32;
        let radius = (i % 7 + 1) as f32;
        let ray = Ray {
            p: v(0.0, 0.0),
            d: v(1.0, 0.0),
            t: distance + radius + 1.0,
        };
        let tangent = unsafe {
            compare_ray_circle(
                "C36",
                &c,
                &rust,
                ray,
                Circle {
                    p: v(distance, radius),
                    r: radius,
                },
                i ^ 0x4444,
            )
        };
        assert_eq!(tangent, 1, "C36 iteration {i}");

        let boundary = unsafe {
            compare_ray_circle(
                "C37",
                &c,
                &rust,
                ray,
                Circle {
                    p: v(radius, 0.0),
                    r: radius,
                },
                i ^ 0x5555,
            )
        };
        assert_eq!(boundary, 1, "C37 iteration {i}");
    }

    let miss_ray = Ray {
        p: v(0.0, 0.0),
        d: v(1.0, 0.0),
        t: 1.0,
    };
    let miss_circle = Circle {
        p: v(0.0, 5.0),
        r: 1.0,
    };
    unsafe {
        let c_ret = (c.ray_circle)(miss_ray, miss_circle, std::ptr::null_mut());
        let r_ret = (rust.ray_circle)(miss_ray, miss_circle, std::ptr::null_mut());
        assert_eq!((c_ret, r_ret), (0, 0), "E01 null no-write output");
    }
}

#[test]
fn ray_aabb_surface_c38_c46_and_e08_e10() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);

    for i in 0..192_u32 {
        let offset = rng.finite(-50.0, 50.0);
        let half = rng.finite(0.5, 5.0);
        let aabb = Aabb {
            min: v(offset - half, -half),
            max: v(offset + half, half),
        };
        let span = half + rng.finite(1.0, 10.0);

        let broad_miss = Ray {
            p: v(offset - span * 3.0, half + 2.0),
            d: v(1.0, 0.0),
            t: span,
        };
        assert_eq!(
            unsafe { compare_ray_aabb("C38/E08", &c, &rust, broad_miss, aabb, i) },
            0
        );

        let corner_miss = Ray {
            p: v(offset - half * 2.0, half * 0.9),
            d: v(1.0, 0.4),
            t: half * 2.9,
        };
        assert_eq!(
            unsafe { compare_ray_aabb("C39/E09", &c, &rust, corner_miss, aabb, i ^ 0x1000) },
            0
        );

        let cases = [
            (
                "C40",
                Ray {
                    p: v(offset - span, 0.0),
                    d: v(1.0, 0.0),
                    t: span * 2.0,
                },
                [-1.0_f32, 0.0_f32],
            ),
            (
                "C41",
                Ray {
                    p: v(offset + span, 0.0),
                    d: v(-1.0, 0.0),
                    t: span * 2.0,
                },
                [1.0, 0.0],
            ),
            (
                "C42",
                Ray {
                    p: v(offset, -span),
                    d: v(0.0, 1.0),
                    t: span * 2.0,
                },
                [0.0, -1.0],
            ),
            (
                "C43",
                Ray {
                    p: v(offset, span),
                    d: v(0.0, -1.0),
                    t: span * 2.0,
                },
                [0.0, 1.0],
            ),
        ];
        for (row, ray, expected_normal) in cases {
            let mut c_out = sentinel(i ^ 0x2000);
            let mut r_out = c_out;
            unsafe {
                let c_ret = (c.ray_aabb)(ray, aabb, &mut c_out);
                let r_ret = (rust.ray_aabb)(ray, aabb, &mut r_out);
                assert_cast(row, c_ret, c_out, r_ret, r_out);
                assert_eq!(c_ret, 1, "{row} iteration {i}");
                assert_eq!(
                    bits_v(c_out.n),
                    bits_v(v(expected_normal[0], expected_normal[1])),
                    "{row}"
                );
            }
        }

        let starts_inside = Ray {
            p: v(offset, 0.0),
            d: v(1.0, 0.25),
            t: half,
        };
        assert_eq!(
            unsafe { compare_ray_aabb("C44", &c, &rust, starts_inside, aabb, i ^ 0x3000) },
            1
        );

        let zero_length = Ray {
            p: v(offset, 0.0),
            d: v(1.0, 0.0),
            t: 0.0,
        };
        assert_eq!(
            unsafe { compare_ray_aabb("C45", &c, &rust, zero_length, aabb, i ^ 0x4000) },
            1
        );
    }

    for i in 0..128_u32 {
        let nan = f32::from_bits(0x7fc0_0001 | (i * 7919));
        let point = rng.finite(-20.0, 20.0);
        let nan_ray = Ray {
            p: v(point, point),
            d: v(nan, nan),
            t: rng.finite(0.5, 5.0),
        };
        let reversed = Aabb {
            min: v(point + 1.0, point + 1.0),
            max: v(point - 1.0, point - 1.0),
        };
        assert_eq!(
            unsafe { compare_ray_aabb("C46/E10", &c, &rust, nan_ray, reversed, i) },
            0
        );
    }

    let broad_miss = Ray {
        p: v(-10.0, 10.0),
        d: v(1.0, 0.0),
        t: 1.0,
    };
    let aabb = Aabb {
        min: v(-1.0, -1.0),
        max: v(1.0, 1.0),
    };
    unsafe {
        let c_ret = (c.ray_aabb)(broad_miss, aabb, std::ptr::null_mut());
        let r_ret = (rust.ray_aabb)(broad_miss, aabb, std::ptr::null_mut());
        assert_eq!((c_ret, r_ret), (0, 0), "E08 null no-write output");
    }
}

#[test]
fn ray_capsule_surface_c47_c58_and_e16_e20() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xa54f_f53a_5f1d_36f1);

    for i in 0..192_u32 {
        let radius = rng.finite(0.5, 3.0);
        let length = rng.finite(radius * 3.0, radius * 10.0);
        let capsule = Capsule {
            a: v(0.0, 0.0),
            b: v(0.0, length),
            r: radius,
        };
        let long = radius * 10.0 + length;

        let cases = [
            (
                "C47",
                Ray {
                    p: v(0.0, length * 0.5),
                    d: v(1.0, 0.0),
                    t: long,
                },
                1,
            ),
            (
                "C48",
                Ray {
                    p: v(0.0, -radius * 0.5),
                    d: v(1.0, 0.0),
                    t: long,
                },
                1,
            ),
            (
                "C49",
                Ray {
                    p: v(0.0, length + radius * 0.5),
                    d: v(1.0, 0.0),
                    t: long,
                },
                1,
            ),
            (
                "C50/E20",
                Ray {
                    p: v(radius * 3.0, length * 0.5),
                    d: v(1.0, 0.0),
                    t: long,
                },
                0,
            ),
            (
                "C51/E16",
                Ray {
                    p: v(radius * 0.5, -radius * 2.0),
                    d: v(0.0, -1.0),
                    t: long,
                },
                0,
            ),
            (
                "C52/E17",
                Ray {
                    p: v(radius * 0.5, length + radius * 2.0),
                    d: v(0.0, 1.0),
                    t: long,
                },
                0,
            ),
            (
                "C53/E18",
                Ray {
                    p: v(radius * 3.0, -radius * 2.0),
                    d: v(-1.0, 0.0),
                    t: long,
                },
                0,
            ),
            (
                "C54/E19",
                Ray {
                    p: v(radius * 3.0, length + radius * 2.0),
                    d: v(-1.0, 0.0),
                    t: long,
                },
                0,
            ),
            (
                "C55",
                Ray {
                    p: v(radius * 3.0, length * 0.5),
                    d: v(-1.0, 0.0),
                    t: long,
                },
                1,
            ),
            (
                "C56",
                Ray {
                    p: v(-radius * 3.0, length * 0.5),
                    d: v(1.0, 0.0),
                    t: long,
                },
                1,
            ),
        ];
        for (row, ray, expected) in cases {
            let actual = unsafe {
                compare_ray_capsule(row, &c, &rust, ray, capsule, i ^ row.as_bytes()[1] as u32)
            };
            assert_eq!(actual, expected, "{row} iteration {i}");
        }

        for (row, y) in [
            ("C53 cap hit", -radius * 0.5),
            ("C54 cap hit", length + radius * 0.5),
        ] {
            let ray = Ray {
                p: v(radius * 3.0, y),
                d: v(-1.0, 0.0),
                t: long,
            };
            assert_eq!(
                unsafe { compare_ray_capsule(row, &c, &rust, ray, capsule, i ^ 0x5000) },
                1,
                "{row} iteration {i}"
            );
        }
    }

    for i in 0..128_u32 {
        let center = v(i as f32 * 0.125, -(i as f32) * 0.0625);
        let degenerate = Capsule {
            a: center,
            b: center,
            r: 1.0,
        };
        let ray = Ray {
            p: v(center.x - 3.0, center.y),
            d: v(1.0, 0.0),
            t: 6.0,
        };
        unsafe {
            compare_ray_capsule("C57", &c, &rust, ray, degenerate, i ^ 0x6000);
        }

        let zero_radius = Capsule {
            a: v(0.0, 0.0),
            b: v(0.0, 4.0),
            r: 0.0,
        };
        let boundary_ray = Ray {
            p: v(0.0, 2.0),
            d: v(1.0, 0.0),
            t: 2.0,
        };
        unsafe {
            compare_ray_capsule("C58", &c, &rust, boundary_ray, zero_radius, i ^ 0x7000);
        }
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn call_cast_with_zero_rax(
    function: CastRay,
    ray: Ray,
    shape: *const c_void,
    mode: c_int,
    out: *mut Raycast,
) -> c_int {
    let first = (ray.p.x.to_bits() as u64) | ((ray.p.y.to_bits() as u64) << 32);
    let second = (ray.d.x.to_bits() as u64) | ((ray.d.y.to_bits() as u64) << 32);
    let third = ray.t.to_bits();
    let result: c_int;
    unsafe {
        asm!(
            "sub rsp, 32",
            "mov qword ptr [rsp], {first}",
            "mov qword ptr [rsp + 8], {second}",
            "mov dword ptr [rsp + 16], {third:e}",
            "xor eax, eax",
            "call r11",
            "add rsp, 32",
            first = in(reg) first,
            second = in(reg) second,
            third = in(reg) third,
            in("r11") function,
            in("rdi") shape,
            in("esi") mode,
            in("rdx") out,
            lateout("eax") result,
            clobber_abi("C"),
        );
    }
    result
}

#[test]
fn cast_ray_modes_c59_c61_and_enum_boundaries() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x510e_527f_ade6_82d1);

    for i in 0..192_u32 {
        let y = rng.finite(-20.0, 20.0);
        let radius = rng.finite(0.25, 2.0);
        let ray = Ray {
            p: v(-10.0, y),
            d: v(1.0, 0.0),
            t: 20.0,
        };
        let circle = Circle {
            p: v(0.0, y),
            r: radius,
        };
        let aabb = Aabb {
            min: v(-radius, y - radius),
            max: v(radius, y + radius),
        };
        let capsule = Capsule {
            a: v(0.0, y - radius),
            b: v(0.0, y + radius),
            r: radius,
        };

        for (row, shape, mode) in [
            ("C59", (&circle as *const Circle).cast::<c_void>(), 0),
            ("C60", (&aabb as *const Aabb).cast::<c_void>(), 1),
            ("C61", (&capsule as *const Capsule).cast::<c_void>(), 2),
        ] {
            let mut c_out = sentinel(i);
            let mut r_out = c_out;
            unsafe {
                let c_ret = (c.cast_ray)(ray, shape, mode, &mut c_out);
                let r_ret = (rust.cast_ray)(ray, shape, mode, &mut r_out);
                assert_cast(row, c_ret, c_out, r_ret, r_out);
                assert_eq!(c_ret, 1, "{row} iteration {i}");
            }
        }
    }

    let ray = Ray {
        p: v(0.0, 0.0),
        d: v(1.0, 0.0),
        t: 1.0,
    };
    for mode in [c_int::MIN, -1, 3, 4, c_int::MAX] {
        let mut c_out = sentinel(mode as u32);
        let mut r_out = c_out;
        unsafe {
            #[cfg(target_arch = "x86_64")]
            let (c_ret, r_ret) = (
                call_cast_with_zero_rax(c.cast_ray, ray, std::ptr::null(), mode, &mut c_out),
                call_cast_with_zero_rax(rust.cast_ray, ray, std::ptr::null(), mode, &mut r_out),
            );
            #[cfg(not(target_arch = "x86_64"))]
            let (c_ret, r_ret) = (
                (c.cast_ray)(ray, std::ptr::null(), mode, &mut c_out),
                (rust.cast_ray)(ray, std::ptr::null(), mode, &mut r_out),
            );
            assert_cast("out-of-range C2_TYPE", c_ret, c_out, r_ret, r_out);
            assert_eq!(c_ret, 0, "out-of-range mode {mode}");
        }
    }

    let miss_circle = Circle {
        p: v(0.0, 10.0),
        r: 1.0,
    };
    unsafe {
        let c_ret = (c.cast_ray)(
            ray,
            (&miss_circle as *const Circle).cast(),
            0,
            std::ptr::null_mut(),
        );
        let r_ret = (rust.cast_ray)(
            ray,
            (&miss_circle as *const Circle).cast(),
            0,
            std::ptr::null_mut(),
        );
        assert_eq!((c_ret, r_ret), (0, 0), "dispatcher null no-write output");
    }
}

#[derive(Clone, Copy)]
struct GenInput {
    mouse: V,
    ray_origin: V,
    circle: Circle,
    capsule: Capsule,
    aabb: Aabb,
}

unsafe fn call_gen(api: &Api, input: GenInput, outputs: &mut [Raycast; 3]) -> c_int {
    unsafe {
        (api.gen_ray)(
            &mut outputs[0],
            &mut outputs[1],
            &mut outputs[2],
            input.mouse.x,
            input.mouse.y,
            input.ray_origin.x,
            input.ray_origin.y,
            input.circle.p.x,
            input.circle.p.y,
            input.circle.r,
            input.capsule.a.x,
            input.capsule.a.y,
            input.capsule.b.x,
            input.capsule.b.y,
            input.capsule.r,
            input.aabb.min.x,
            input.aabb.min.y,
            input.aabb.max.x,
            input.aabb.max.y,
        )
    }
}

unsafe fn compare_gen(row: &str, c: &Api, rust: &Api, input: GenInput, seed: u32) -> c_int {
    let mut c_out = [
        sentinel(seed),
        sentinel(seed ^ 0x1111_1111),
        sentinel(seed ^ 0x2222_2222),
    ];
    let mut r_out = c_out;
    let c_ret = unsafe { call_gen(c, input, &mut c_out) };
    let r_ret = unsafe { call_gen(rust, input, &mut r_out) };
    assert_eq!(c_ret, r_ret, "{row}: return value");
    for index in 0..3 {
        assert_eq!(
            bits_cast(c_out[index]),
            bits_cast(r_out[index]),
            "{row}: output {index}, C={:?}, Rust={:?}",
            c_out[index],
            r_out[index]
        );
    }
    c_ret
}

fn gen_input(base: f32, y: f32, mask: c_int, radius: f32) -> GenInput {
    let circle_hit = mask & 1 != 0;
    let capsule_hit = mask & 2 != 0;
    let aabb_hit = mask & 4 != 0;
    GenInput {
        mouse: v(base + 20.0, y),
        ray_origin: v(base, y),
        circle: Circle {
            p: v(base + 3.0, if circle_hit { y } else { y + 5.0 }),
            r: radius,
        },
        capsule: Capsule {
            a: v(base + 7.0, if capsule_hit { y - 1.0 } else { y + 5.0 }),
            b: v(base + 7.0, if capsule_hit { y + 1.0 } else { y + 7.0 }),
            r: radius,
        },
        aabb: Aabb {
            min: v(base + 11.0, if aabb_hit { y - radius } else { y + 5.0 }),
            max: v(base + 12.0, if aabb_hit { y + radius } else { y + 6.0 }),
        },
    }
}

#[test]
fn gen_ray_hit_masks_c62_c69() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x9b05_688c_2b3e_6c1f);

    for mask in 0..=7 {
        for i in 0..192_u32 {
            let base = rng.finite(-100.0, 100.0);
            let y = rng.finite(-50.0, 50.0);
            let radius = rng.finite(0.25, 0.75);
            let input = gen_input(base, y, mask, radius);
            let row = match mask {
                0 => "C62",
                1 => "C63",
                2 => "C64",
                3 => "C65",
                4 => "C66",
                5 => "C67",
                6 => "C68",
                7 => "C69",
                _ => unreachable!(),
            };
            let actual = unsafe { compare_gen(row, &c, &rust, input, i ^ mask as u32) };
            assert_eq!(actual, mask, "{row} iteration {i}");
        }
    }
}

#[test]
fn gen_ray_degenerate_surface_c70_c71() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x1f83_d9ab_fb41_bd6b);

    for i in 0..192_u32 {
        let point = v(rng.finite(-20.0, 20.0), rng.finite(-20.0, 20.0));
        let zero_ray = GenInput {
            mouse: point,
            ray_origin: point,
            circle: Circle { p: point, r: 1.0 },
            capsule: Capsule {
                a: v(point.x, point.y - 1.0),
                b: v(point.x, point.y + 1.0),
                r: 0.5,
            },
            aabb: Aabb {
                min: v(point.x - 1.0, point.y - 1.0),
                max: v(point.x + 1.0, point.y + 1.0),
            },
        };
        unsafe {
            compare_gen("C70", &c, &rust, zero_ray, i);
        }

        let odd_shapes = GenInput {
            mouse: v(point.x + 5.0, point.y),
            ray_origin: point,
            circle: Circle {
                p: v(point.x + 2.0, point.y),
                r: -rng.finite(0.0, 2.0),
            },
            capsule: Capsule {
                a: v(point.x + 3.0, point.y),
                b: v(point.x + 3.0, point.y),
                r: if i & 1 == 0 { 0.0 } else { -1.0 },
            },
            aabb: Aabb {
                min: v(point.x + 5.0, point.y + 1.0),
                max: v(point.x + 4.0, point.y - 1.0),
            },
        };
        unsafe {
            compare_gen("C71", &c, &rust, odd_shapes, i ^ 0x8000);
        }
    }
}
