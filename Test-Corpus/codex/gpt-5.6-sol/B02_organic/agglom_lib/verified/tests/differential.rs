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
struct CnRnd {
    state: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

type C2VFn = unsafe extern "C" fn(f32, f32) -> C2v;
type C2BinaryFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type C2ClampFn = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type C2DotFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type CircleCircleFn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type CircleAabbFn = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type AabbAabbFn = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type F2Fn = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
type F3Fn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type F4Fn = unsafe extern "C" fn(*mut CnRnd) -> f64;
type F5Fn = unsafe extern "C" fn(u32) -> u32;
type F7Fn = unsafe extern "C" fn(u32, u32, u32) -> u32;
type F9Fn = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
type F10Fn = unsafe extern "C" fn(u16) -> f32;
type ColorFn = unsafe extern "C" fn(*mut f32, *const f32);
type AgglomFn = unsafe extern "C" fn(
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    c_int,
    c_int,
    u64,
    u64,
    u32,
    u32,
    u32,
    u32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    u16,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
) -> f64;

struct Api {
    _library: Library,
    c2_v: C2VFn,
    c2_maxv: C2BinaryFn,
    c2_minv: C2BinaryFn,
    c2_clampv: C2ClampFn,
    c2_sub: C2BinaryFn,
    c2_dot: C2DotFn,
    circle_circle: CircleCircleFn,
    circle_aabb: CircleAabbFn,
    aabb_aabb: AabbAabbFn,
    f2: F2Fn,
    f3: F3Fn,
    f4: F4Fn,
    f5: F5Fn,
    f7: F7Fn,
    f9: F9Fn,
    f10: F10Fn,
    f11: ColorFn,
    f12: ColorFn,
    f13: ColorFn,
    agglom: AgglomFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! load {
            ($name:literal, $ty:ty) => {{
                let symbol = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *symbol
            }};
        }
        Self {
            c2_v: load!("c2V", C2VFn),
            c2_maxv: load!("c2Maxv", C2BinaryFn),
            c2_minv: load!("c2Minv", C2BinaryFn),
            c2_clampv: load!("c2Clampv", C2ClampFn),
            c2_sub: load!("c2Sub", C2BinaryFn),
            c2_dot: load!("c2Dot", C2DotFn),
            circle_circle: load!("c2CircletoCircle", CircleCircleFn),
            circle_aabb: load!("c2CircletoAABB", CircleAabbFn),
            aabb_aabb: load!("c2AABBtoAABB", AabbAabbFn),
            f2: load!("f2", F2Fn),
            f3: load!("f3", F3Fn),
            f4: load!("f4", F4Fn),
            f5: load!("f5", F5Fn),
            f7: load!("f7", F7Fn),
            f9: load!("f9", F9Fn),
            f10: load!("f10", F10Fn),
            f11: load!("f11", ColorFn),
            f12: load!("f12", ColorFn),
            f13: load!("f13", ColorFn),
            agglom: load!("agglom", AgglomFn),
            _library: library,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libagglom_lib.so")
}

fn apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.exists(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.exists(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn u32(&mut self) -> u32 {
        self.u64() as u32
    }

    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }

    fn finite(&mut self) -> f32 {
        (self.i32() % 200_001) as f32 / 257.0
    }

    fn unit(&mut self) -> f32 {
        (self.u32() % 10_001) as f32 / 10_000.0
    }
}

fn assert_f32(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:08x}), Rust={rust:?} ({:08x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_f64(c: f64, rust: f64, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:016x}), Rust={rust:?} ({:016x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_v(c: C2v, rust: C2v, context: &str) {
    assert_f32(c.x, rust.x, &format!("{context}.x"));
    assert_f32(c.y, rust.y, &format!("{context}.y"));
}

fn assert_lm(c: LmVec2, rust: LmVec2, context: &str) {
    assert_f32(c.x, rust.x, &format!("{context}.x"));
    assert_f32(c.y, rust.y, &format!("{context}.y"));
}

fn vec(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> C2Aabb {
    C2Aabb {
        min: vec(min_x, min_y),
        max: vec(max_x, max_y),
    }
}

#[test]
fn vector_primitives_match_all_comparison_regions() {
    let (c, rust) = apis();
    let specials = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
    ];
    for &x in &specials {
        for &y in &specials {
            unsafe {
                assert_v((c.c2_v)(x, y), (rust.c2_v)(x, y), "c2V");
            }
        }
    }

    let mut rng = Rng::new(0x1057_4f90_2b6d_8a31);
    for _case in 0..CASES {
        let x = rng.finite().abs() + 1.0;
        let y = rng.finite().abs() + 1.0;
        let regions = [
            (vec(x, y), vec(-x, -y)),
            (vec(-x, -y), vec(x, y)),
            (vec(x, -y), vec(-x, y)),
        ];
        for (a, b) in regions {
            unsafe {
                assert_v((c.c2_maxv)(a, b), (rust.c2_maxv)(a, b), "c2Maxv");
                assert_v((c.c2_minv)(a, b), (rust.c2_minv)(a, b), "c2Minv");
                assert_v((c.c2_sub)(a, b), (rust.c2_sub)(a, b), "c2Sub");
                assert_f32((c.c2_dot)(a, b), (rust.c2_dot)(a, b), "c2Dot");
            }
        }
    }

    let nan = f32::from_bits(0x7fc0_4321);
    for (a, b) in [
        (vec(nan, 1.0), vec(2.0, nan)),
        (vec(2.0, nan), vec(nan, 1.0)),
        (vec(f32::INFINITY, -0.0), vec(f32::INFINITY, 0.0)),
    ] {
        unsafe {
            assert_v((c.c2_maxv)(a, b), (rust.c2_maxv)(a, b), "c2Maxv special");
            assert_v((c.c2_minv)(a, b), (rust.c2_minv)(a, b), "c2Minv special");
            assert_v((c.c2_sub)(a, b), (rust.c2_sub)(a, b), "c2Sub special");
            assert_f32((c.c2_dot)(a, b), (rust.c2_dot)(a, b), "c2Dot special");
        }
    }
    for case in 0..CASES {
        let nan = f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff));
        let a = vec(nan, rng.finite());
        let b = vec(rng.finite(), nan);
        unsafe {
            assert_v(
                (c.c2_maxv)(a, b),
                (rust.c2_maxv)(a, b),
                &format!("c2Maxv random NaN {case}"),
            );
            assert_v(
                (c.c2_minv)(a, b),
                (rust.c2_minv)(a, b),
                &format!("c2Minv random NaN {case}"),
            );
            assert_v(
                (c.c2_sub)(a, b),
                (rust.c2_sub)(a, b),
                &format!("c2Sub random NaN {case}"),
            );
            assert_f32(
                (c.c2_dot)(a, b),
                (rust.c2_dot)(a, b),
                &format!("c2Dot random NaN {case}"),
            );
        }
    }
}

#[test]
fn clamp_matches_all_axis_regions() {
    let (c, rust) = apis();
    let lo = vec(-10.0, -20.0);
    let hi = vec(10.0, 20.0);
    let axis = [(-30.0, 0_u8), (0.0, 1), (30.0, 2)];
    for &(x, xr) in &axis {
        for &(y, yr) in &axis {
            for jitter in 0..CASES {
                let delta = jitter as f32 / 10_000.0;
                let input = vec(x - delta, y + delta);
                unsafe {
                    assert_v(
                        (c.c2_clampv)(input, lo, hi),
                        (rust.c2_clampv)(input, lo, hi),
                        &format!("c2Clampv regions {xr}/{yr}"),
                    );
                }
            }
        }
    }
    let nan = f32::from_bits(0x7fc0_0101);
    for (input, low, high) in [
        (vec(nan, 0.0), lo, hi),
        (vec(0.0, nan), lo, hi),
        (vec(0.0, 0.0), vec(nan, -20.0), hi),
        (vec(0.0, 0.0), lo, vec(nan, 20.0)),
    ] {
        unsafe {
            assert_v(
                (c.c2_clampv)(input, low, high),
                (rust.c2_clampv)(input, low, high),
                "c2Clampv NaN",
            );
        }
    }
    for case in 0..CASES {
        let nan = f32::from_bits(0x7fc0_0001 | (case as u32 * 7919 & 0x003f_ffff));
        let input = vec(nan, case as f32 / 7.0);
        unsafe {
            assert_v(
                (c.c2_clampv)(input, lo, hi),
                (rust.c2_clampv)(input, lo, hi),
                &format!("c2Clampv random NaN {case}"),
            );
        }
    }
}

#[test]
fn collision_primitives_and_dispatch_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x44e2_d3b1_94a5_c807);
    for case in 0..CASES {
        let r1 = 0.5 + rng.unit() * 20.0;
        let r2 = 0.5 + rng.unit() * 20.0;
        let sum = r1 + r2;
        for distance in [sum * 0.5, sum, sum + 1.0 + rng.unit()] {
            let a = C2Circle {
                p: vec(0.0, 0.0),
                r: r1,
            };
            let b = C2Circle {
                p: vec(distance, 0.0),
                r: r2,
            };
            unsafe {
                assert_eq!(
                    (c.circle_circle)(a, b),
                    (rust.circle_circle)(a, b),
                    "circle/circle case {case}"
                );
                assert_eq!(
                    (c.f2)((&raw const a).cast(), 0, (&raw const b).cast(), 0),
                    (rust.f2)((&raw const a).cast(), 0, (&raw const b).cast(), 0),
                    "f2 circle/circle case {case}"
                );
            }
        }
    }
    for case in 0..CASES {
        let nan = f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff));
        let a = C2Circle {
            p: vec(rng.finite(), nan),
            r: -(0.01 + rng.unit() * 100.0),
        };
        let b = C2Circle {
            p: vec(nan, rng.finite()),
            r: -(0.01 + rng.unit() * 100.0),
        };
        unsafe {
            assert_eq!(
                (c.circle_circle)(a, b),
                (rust.circle_circle)(a, b),
                "circle/circle special case {case}"
            );
        }
    }

    let box1 = aabb(-10.0, -20.0, 10.0, 20.0);
    for region in 0..5 {
        for case in 0..CASES {
            let dx = 1.0 + rng.unit() * 30.0;
            let dy = 1.0 + rng.unit() * 30.0;
            let point = match region {
                0 => vec(-10.0 - dx, -20.0 - dy),
                1 => vec(-9.0 + rng.unit() * 18.0, -19.0 + rng.unit() * 38.0),
                2 => vec(10.0 + dx, 20.0 + dy),
                3 => vec(-10.0 - dx, -19.0 + rng.unit() * 38.0),
                _ => vec(-9.0 + rng.unit() * 18.0, 20.0 + dy),
            };
            let radius = match case % 4 {
                0 => 0.01 + rng.unit(),
                1 => 20.0 + rng.unit() * 20.0,
                2 => 100.0 + rng.unit() * 100.0,
                _ => -(0.01 + rng.unit() * 100.0),
            };
            let circle = C2Circle {
                p: point,
                r: radius,
            };
            unsafe {
                assert_eq!(
                    (c.circle_aabb)(circle, box1),
                    (rust.circle_aabb)(circle, box1),
                    "circle/AABB region {region} case {case}"
                );
                assert_eq!(
                    (c.f2)((&raw const circle).cast(), 0, (&raw const box1).cast(), 1),
                    (rust.f2)((&raw const circle).cast(), 0, (&raw const box1).cast(), 1),
                    "f2 circle/AABB region {region} case {case}"
                );
                assert_eq!(
                    (c.f2)((&raw const box1).cast(), 1, (&raw const circle).cast(), 0),
                    (rust.f2)((&raw const box1).cast(), 1, (&raw const circle).cast(), 0),
                    "f2 AABB/circle region {region} case {case}"
                );
            }
        }
    }

    for case in 0..CASES {
        let radius = 0.25 + rng.unit() * 30.0;
        let circle = C2Circle {
            p: vec(-10.0 - radius, -19.0 + rng.unit() * 38.0),
            r: radius,
        };
        unsafe {
            assert_eq!(
                (c.circle_aabb)(circle, box1),
                (rust.circle_aabb)(circle, box1),
                "circle/AABB tangent case {case}"
            );
        }
    }

    for region in 0..7 {
        for case in 0..CASES {
            let dx = 0.1 + rng.unit() * 30.0;
            let dy = 0.1 + rng.unit() * 30.0;
            let b = match region {
                0 => aabb(-9.0, -19.0, 9.0, 19.0),
                1 => aabb(10.0, -10.0, 10.0 + dx, 10.0),
                2 => aabb(10.0 + dx, -5.0, 20.0 + dx, 5.0),
                3 => aabb(-20.0 - dx, -5.0, -10.0 - dx, 5.0),
                4 => aabb(-5.0, 20.0 + dy, 5.0, 30.0 + dy),
                5 => aabb(-5.0, -30.0 - dy, 5.0, -20.0 - dy),
                _ => aabb(10.0 + dx, 20.0 + dy, -10.0, -20.0),
            };
            unsafe {
                assert_eq!(
                    (c.aabb_aabb)(box1, b),
                    (rust.aabb_aabb)(box1, b),
                    "AABB/AABB region {region} case {case}"
                );
                assert_eq!(
                    (c.f2)((&raw const box1).cast(), 1, (&raw const b).cast(), 1),
                    (rust.f2)((&raw const box1).cast(), 1, (&raw const b).cast(), 1),
                    "f2 AABB/AABB region {region} case {case}"
                );
            }
        }
    }

    for case in 0..CASES {
        let nan = f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff));
        let b = aabb(nan, rng.finite(), rng.finite(), rng.finite());
        let circle = C2Circle {
            p: vec(nan, rng.finite()),
            r: -(0.01 + rng.unit() * 100.0),
        };
        unsafe {
            assert_eq!(
                (c.aabb_aabb)(box1, b),
                (rust.aabb_aabb)(box1, b),
                "AABB/AABB special {case}"
            );
            assert_eq!(
                (c.circle_aabb)(circle, box1),
                (rust.circle_aabb)(circle, box1),
                "circle/AABB special {case}"
            );
        }
    }
}

#[test]
fn integer_and_rng_functions_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xd301_7395_afa8_4c2b);
    let boundaries = [
        i32::MIN,
        i32::MIN + 1,
        -100,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        100,
        i32::MAX,
    ];
    for &a in &boundaries {
        for &b in &boundaries {
            if b != 0 {
                unsafe {
                    assert_eq!((c.f3)(a, b), (rust.f3)(a, b), "f3({a}, {b})");
                }
            }
        }
    }
    for case in 0..(CASES * 8) {
        let a = rng.i32();
        let mut b = rng.i32();
        if b == 0 {
            b = 1;
        }
        unsafe {
            assert_eq!((c.f3)(a, b), (rust.f3)(a, b), "random f3 case {case}");
        }
    }

    for case in 0..CASES {
        let initial = if case == 0 {
            CnRnd { state: [0, 0] }
        } else {
            CnRnd {
                state: [rng.u64(), rng.u64()],
            }
        };
        let mut c_state = initial;
        let mut rust_state = initial;
        unsafe {
            let c_result = (c.f4)(&raw mut c_state);
            let rust_result = (rust.f4)(&raw mut rust_state);
            assert_f64(c_result, rust_result, "f4 result");
        }
        assert_eq!(c_state.state, rust_state.state, "f4 state case {case}");
    }

    for value in [0, 1, u16::MAX as u32, 0xffff_0000, u32::MAX] {
        unsafe {
            assert_eq!((c.f5)(value), (rust.f5)(value), "f5 {value:#x}");
        }
    }
    for case in 0..CASES * 8 {
        let value = rng.u32();
        unsafe {
            assert_eq!((c.f5)(value), (rust.f5)(value), "random f5 case {case}");
        }
    }

    let channels = [0, 1, 2, 3, u32::MAX];
    let depths = [0, 1, 16, 24, 31, 32, 33, u32::MAX];
    let blocks = [0, 1, 2, 255, 4096, u32::MAX];
    for block in blocks {
        for channel in channels {
            for depth in depths {
                unsafe {
                    assert_eq!(
                        (c.f7)(block, channel, depth),
                        (rust.f7)(block, channel, depth),
                        "f7({block}, {channel}, {depth})"
                    );
                }
            }
        }
    }
    for case in 0..CASES * 8 {
        let args = (rng.u32(), rng.u32(), rng.u32());
        unsafe {
            assert_eq!(
                (c.f7)(args.0, args.1, args.2),
                (rust.f7)(args.0, args.1, args.2),
                "random f7 case {case}"
            );
        }
    }
    for stereo in [false, true] {
        for depth_32 in [false, true] {
            for case in 0..CASES {
                let block = rng.u32();
                let channel = if stereo {
                    2
                } else {
                    let candidate = rng.u32();
                    if candidate == 2 { 3 } else { candidate }
                };
                let depth = if depth_32 {
                    32
                } else {
                    let candidate = rng.u32();
                    if candidate == 32 { 31 } else { candidate }
                };
                unsafe {
                    assert_eq!(
                        (c.f7)(block, channel, depth),
                        (rust.f7)(block, channel, depth),
                        "partitioned f7 {stereo}/{depth_32} case {case}"
                    );
                }
            }
        }
    }
}

#[test]
fn barycentric_coordinates_match() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xf40c_b95d_6217_83ae);
    for case in 0..CASES * 4 {
        let p1 = LmVec2 {
            x: rng.finite(),
            y: rng.finite(),
        };
        let p2 = LmVec2 {
            x: p1.x + 1.0 + rng.unit() * 10.0,
            y: p1.y + rng.unit(),
        };
        let p3 = LmVec2 {
            x: p1.x + rng.unit(),
            y: p1.y + 1.0 + rng.unit() * 10.0,
        };
        let p = LmVec2 {
            x: rng.finite(),
            y: rng.finite(),
        };
        unsafe {
            assert_lm(
                (c.f9)(p1, p2, p3, p),
                (rust.f9)(p1, p2, p3, p),
                &format!("nondegenerate f9 case {case}"),
            );
        }
    }

    for case in 0..CASES {
        let origin = LmVec2 {
            x: rng.finite(),
            y: rng.finite(),
        };
        let dx = 0.1 + rng.unit() * 10.0;
        let dy = 0.1 + rng.unit() * 10.0;
        let points = [
            origin,
            LmVec2 {
                x: origin.x + dx,
                y: origin.y + dy,
            },
            LmVec2 {
                x: origin.x + 2.0 * dx,
                y: origin.y + 2.0 * dy,
            },
            LmVec2 {
                x: origin.x - dx,
                y: origin.y - dy,
            },
        ];
        unsafe {
            assert_lm(
                (c.f9)(points[0], points[1], points[2], points[3]),
                (rust.f9)(points[0], points[1], points[2], points[3]),
                &format!("degenerate f9 case {case}"),
            );
        }
    }

    for (case, points) in [
        [
            LmVec2 {
                x: f32::INFINITY,
                y: 0.0,
            },
            LmVec2 { x: 1.0, y: 2.0 },
            LmVec2 { x: 3.0, y: 4.0 },
            LmVec2 { x: 5.0, y: 6.0 },
        ],
        [
            LmVec2 {
                x: f32::from_bits(0x7fc0_1234),
                y: 0.0,
            },
            LmVec2 { x: 1.0, y: 2.0 },
            LmVec2 { x: 3.0, y: 4.0 },
            LmVec2 { x: 5.0, y: 6.0 },
        ],
    ]
    .into_iter()
    .enumerate()
    {
        unsafe {
            assert_lm(
                (c.f9)(points[0], points[1], points[2], points[3]),
                (rust.f9)(points[0], points[1], points[2], points[3]),
                &format!("special f9 case {case}"),
            );
        }
    }
    for case in 0..CASES {
        let nan = f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff));
        let points = [
            LmVec2 {
                x: nan,
                y: rng.finite(),
            },
            LmVec2 {
                x: rng.finite(),
                y: rng.finite(),
            },
            LmVec2 {
                x: rng.finite(),
                y: nan,
            },
            LmVec2 {
                x: rng.finite(),
                y: rng.finite(),
            },
        ];
        unsafe {
            assert_lm(
                (c.f9)(points[0], points[1], points[2], points[3]),
                (rust.f9)(points[0], points[1], points[2], points[3]),
                &format!("random special f9 case {case}"),
            );
        }
    }
}

#[test]
fn half_conversion_matches_exhaustively() {
    let (c, rust) = apis();
    for input in 0_u16..=u16::MAX {
        unsafe {
            assert_f32(
                (c.f10)(input),
                (rust.f10)(input),
                &format!("f10({input:#06x})"),
            );
        }
    }
}

fn compare_color(function_name: &str, c: ColorFn, rust: ColorFn, input: [f32; 3]) {
    let mut c_output = [f32::from_bits(0x7fc0_1111); 3];
    let mut rust_output = c_output;
    unsafe {
        c(c_output.as_mut_ptr(), input.as_ptr());
        rust(rust_output.as_mut_ptr(), input.as_ptr());
    }
    for index in 0..3 {
        assert_f32(
            c_output[index],
            rust_output[index],
            &format!("{function_name}({input:?})[{index}]"),
        );
    }

    let mut c_alias = input;
    let mut rust_alias = input;
    unsafe {
        c(c_alias.as_mut_ptr(), c_alias.as_ptr());
        rust(rust_alias.as_mut_ptr(), rust_alias.as_ptr());
    }
    for index in 0..3 {
        assert_f32(
            c_alias[index],
            rust_alias[index],
            &format!("{function_name} aliased ({input:?})[{index}]"),
        );
    }
}

#[test]
fn hsl_to_rgb_matches_every_branch() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x0bea_d571_303c_a842);
    let hue_ranges = [
        (-720.0, -0.001),
        (0.0, 59.999),
        (60.0, 119.999),
        (120.0, 179.999),
        (180.0, 239.999),
        (240.0, 299.999),
        (300.0, 359.999),
        (360.0, 720.0),
    ];
    for &(low, high) in &hue_ranges {
        for _ in 0..CASES {
            let hue = low + (high - low) * rng.unit();
            let saturation = 0.001 + rng.unit() * 1.5;
            let lightness = -0.25 + rng.unit() * 1.5;
            compare_color("f11", c.f11, rust.f11, [hue, saturation, lightness]);
        }
    }
    for hue in [
        -0.0,
        0.0,
        60.0,
        120.0,
        180.0,
        240.0,
        300.0,
        360.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_4321),
    ] {
        compare_color("f11", c.f11, rust.f11, [hue, 0.0, 0.75]);
        compare_color("f11", c.f11, rust.f11, [hue, 0.8, 0.25]);
    }
    for _ in 0..CASES {
        let nan = f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff));
        compare_color("f11", c.f11, rust.f11, [nan, rng.unit(), rng.unit()]);
    }
}

#[test]
fn hsv_to_rgb_matches_every_branch() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x05a1_32bc_798e_d64f);
    for selector in -4..=8 {
        for _ in 0..CASES {
            let hue = (selector as f32 + rng.unit() * 0.999) * 60.0;
            let saturation = 0.001 + rng.unit() * 1.5;
            let value = -0.25 + rng.unit() * 1.5;
            compare_color("f12", c.f12, rust.f12, [hue, saturation, value]);
        }
    }
    for hue in [
        -240.0, -60.0, -0.0, 0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 540.0,
    ] {
        compare_color("f12", c.f12, rust.f12, [hue, 0.0, 0.75]);
        compare_color("f12", c.f12, rust.f12, [hue, 0.8, 0.25]);
    }
    for case in 0..CASES {
        let hue = match case % 5 {
            0 => f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff)),
            1 => f32::INFINITY,
            2 => f32::NEG_INFINITY,
            3 => f32::MAX,
            _ => f32::MIN,
        };
        compare_color("f12", c.f12, rust.f12, [hue, 0.8, 0.25]);
    }
}

#[test]
fn rgb_to_hsv_matches_every_branch() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xab59_07e2_684d_31cf);
    for _ in 0..CASES {
        let low = rng.unit();
        let middle = low + 0.1 + rng.unit();
        let high = middle + 0.1 + rng.unit();
        for input in [
            [high, middle, low],
            [high, low, middle],
            [middle, high, low],
            [low, middle, high],
            [low, high, middle],
        ] {
            compare_color("f13", c.f13, rust.f13, input);
        }
    }
    for input in [
        [0.0, 0.0, 0.0],
        [-1.0, -1.0, -1.0],
        [0.0, -1.0, -2.0],
        [-0.0, 0.0, -0.0],
        [f32::INFINITY, 1.0, 0.0],
        [f32::NEG_INFINITY, -1.0, 0.0],
        [f32::from_bits(0x7fc0_1234), 1.0, 2.0],
        [1.0, f32::from_bits(0x7fc0_2345), 2.0],
        [1.0, 2.0, f32::from_bits(0x7fc0_3456)],
    ] {
        compare_color("f13", c.f13, rust.f13, input);
    }
    for _ in 0..CASES {
        let low1 = -(0.1 + rng.unit() * 10.0);
        let low2 = -(0.1 + rng.unit() * 10.0);
        compare_color("f13", c.f13, rust.f13, [0.0, low1, low2]);
    }
    for case in 0..CASES {
        let nan = f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff));
        let mut input = [rng.finite(), rng.finite(), rng.finite()];
        input[case % 3] = nan;
        compare_color("f13", c.f13, rust.f13, input);
    }
}

#[derive(Clone, Copy)]
struct AgglomArgs {
    f2: [f32; 7],
    f3: [c_int; 2],
    f4: [u64; 2],
    f5: u32,
    f7: [u32; 3],
    f9: [f32; 8],
    f10: u16,
    f11: [f32; 3],
    f12: [f32; 3],
    f13: [f32; 3],
}

unsafe fn call_agglom(function: AgglomFn, a: AgglomArgs) -> f64 {
    unsafe {
        function(
            a.f2[0], a.f2[1], a.f2[2], a.f2[3], a.f2[4], a.f2[5], a.f2[6], a.f3[0], a.f3[1],
            a.f4[0], a.f4[1], a.f5, a.f7[0], a.f7[1], a.f7[2], a.f9[0], a.f9[1], a.f9[2], a.f9[3],
            a.f9[4], a.f9[5], a.f9[6], a.f9[7], a.f10, a.f11[0], a.f11[1], a.f11[2], a.f12[0],
            a.f12[1], a.f12[2], a.f13[0], a.f13[1], a.f13[2],
        )
    }
}

fn random_agglom_args(rng: &mut Rng) -> AgglomArgs {
    let mut values = [0.0_f32; 24];
    for value in &mut values {
        *value = rng.finite() / 20.0;
    }
    let mut divisor = rng.i32();
    if divisor == 0 {
        divisor = 1;
    }
    AgglomArgs {
        f2: values[0..7].try_into().unwrap(),
        f3: [rng.i32(), divisor],
        f4: [rng.u64(), rng.u64()],
        f5: rng.u32(),
        f7: [rng.u32(), rng.u32(), rng.u32()],
        f9: values[7..15].try_into().unwrap(),
        f10: rng.u32() as u16,
        f11: [rng.finite(), rng.unit(), rng.unit()],
        f12: [rng.finite(), rng.unit(), rng.unit()],
        f13: values[15..18].try_into().unwrap(),
    }
}

#[test]
fn composed_agglom_matches() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x972f_06cb_31e4_a85d);
    for case in 0..CASES * 8 {
        let args = random_agglom_args(&mut rng);
        unsafe {
            assert_f64(
                call_agglom(c.agglom, args),
                call_agglom(rust.agglom, args),
                &format!("random agglom case {case}"),
            );
        }
    }

    let mut boundary = random_agglom_args(&mut rng);
    boundary.f3 = [i32::MIN, -1];
    boundary.f4 = [0, u64::MAX];
    boundary.f5 = u32::MAX;
    boundary.f7 = [u32::MAX, 2, 32];
    for half in [0x0000, 0x0001, 0x7bff, 0x7c00, 0x7e01, 0xfc00] {
        boundary.f10 = half;
        unsafe {
            assert_f64(
                call_agglom(c.agglom, boundary),
                call_agglom(rust.agglom, boundary),
                &format!("boundary agglom half {half:#06x}"),
            );
        }
    }

    for field in 0..5 {
        for case in 0..CASES {
            let nan = f32::from_bits(0x7fc0_0001 | (rng.u32() & 0x003f_ffff));
            let mut special = random_agglom_args(&mut rng);
            match field {
                0 => special.f2[0] = nan,
                1 => special.f9[0] = f32::INFINITY,
                2 => special.f11[0] = nan,
                3 => special.f12[1] = nan,
                _ => special.f13[2] = nan,
            }
            unsafe {
                assert_f64(
                    call_agglom(c.agglom, special),
                    call_agglom(rust.agglom, special),
                    &format!("special agglom field {field} case {case}"),
                );
            }
        }
    }
}

#[test]
fn explicit_rejections_match() {
    let (c, rust) = apis();
    let circle = C2Circle {
        p: vec(1.0, 2.0),
        r: 3.0,
    };
    let box1 = aabb(-1.0, -2.0, 3.0, 4.0);
    for invalid in [i32::MIN, -2, -1, 2, 3, i32::MAX] {
        unsafe {
            assert_eq!(
                (c.f2)((&raw const circle).cast(), 0, std::ptr::null(), invalid),
                (rust.f2)((&raw const circle).cast(), 0, std::ptr::null(), invalid)
            );
            assert_eq!(
                (c.f2)((&raw const box1).cast(), 1, std::ptr::null(), invalid),
                (rust.f2)((&raw const box1).cast(), 1, std::ptr::null(), invalid)
            );
            assert_eq!(
                (c.f2)(std::ptr::null(), invalid, std::ptr::null(), invalid),
                (rust.f2)(std::ptr::null(), invalid, std::ptr::null(), invalid)
            );
        }
    }
    for value in [i32::MIN, -1, 0, 1, i32::MAX] {
        unsafe {
            assert_eq!((c.f3)(value, 0), 0);
            assert_eq!((rust.f3)(value, 0), 0);
        }
    }
}

#[test]
fn null_probe() {
    let Ok(library_path) = std::env::var("DIFF_NULL_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_NULL_CASE").expect("missing DIFF_NULL_CASE");
    let api = unsafe { Api::load(Path::new(&library_path)) };
    let circle = C2Circle {
        p: vec(1.0, 2.0),
        r: 3.0,
    };
    let mut output = [0.0_f32; 3];
    let input = [1.0_f32, 0.5, 0.25];
    unsafe {
        match case.as_str() {
            "f2_a" => {
                (api.f2)(std::ptr::null(), 0, (&raw const circle).cast(), 0);
            }
            "f2_b" => {
                (api.f2)((&raw const circle).cast(), 0, std::ptr::null(), 0);
            }
            "f4" => {
                (api.f4)(std::ptr::null_mut());
            }
            "f11_dest" => (api.f11)(std::ptr::null_mut(), input.as_ptr()),
            "f11_src" => (api.f11)(output.as_mut_ptr(), std::ptr::null()),
            "f12_dest" => (api.f12)(std::ptr::null_mut(), input.as_ptr()),
            "f12_src" => (api.f12)(output.as_mut_ptr(), std::ptr::null()),
            "f13_dest" => (api.f13)(std::ptr::null_mut(), input.as_ptr()),
            "f13_src" => (api.f13)(output.as_mut_ptr(), std::ptr::null()),
            _ => panic!("unknown null case {case}"),
        }
    }
    panic!("{case} unexpectedly returned");
}

#[cfg(unix)]
#[test]
fn unchecked_null_pointer_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let executable = std::env::current_exe().expect("test executable");
    for case in [
        "f2_a", "f2_b", "f4", "f11_dest", "f11_src", "f12_dest", "f12_src", "f13_dest", "f13_src",
    ] {
        let run = |library: PathBuf| {
            Command::new(&executable)
                .arg("--exact")
                .arg("null_probe")
                .arg("--nocapture")
                .env("DIFF_NULL_LIBRARY", library)
                .env("DIFF_NULL_CASE", case)
                .status()
                .expect("run null probe")
        };
        let c_status = run(c_library_path());
        let rust_status = run(rust_library_path());
        assert!(!c_status.success(), "C {case} unexpectedly succeeded");
        assert!(!rust_status.success(), "Rust {case} unexpectedly succeeded");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different terminating signal for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}
