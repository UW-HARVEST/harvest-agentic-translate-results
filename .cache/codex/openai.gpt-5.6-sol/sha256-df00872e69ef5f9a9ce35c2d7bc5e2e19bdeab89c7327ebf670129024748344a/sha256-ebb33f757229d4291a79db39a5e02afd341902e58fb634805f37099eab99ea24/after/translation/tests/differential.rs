use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

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
type C2BinVecFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
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
    c2_maxv: C2BinVecFn,
    c2_minv: C2BinVecFn,
    c2_clampv: C2ClampFn,
    c2_sub: C2BinVecFn,
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
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap_or_else(|e| {
            panic!("failed to load {}: {e}", path.display());
        });
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name))
            };
        }
        let c2_v = symbol!("c2V", C2VFn);
        let c2_maxv = symbol!("c2Maxv", C2BinVecFn);
        let c2_minv = symbol!("c2Minv", C2BinVecFn);
        let c2_clampv = symbol!("c2Clampv", C2ClampFn);
        let c2_sub = symbol!("c2Sub", C2BinVecFn);
        let c2_dot = symbol!("c2Dot", C2DotFn);
        let circle_circle = symbol!("c2CircletoCircle", CircleCircleFn);
        let circle_aabb = symbol!("c2CircletoAABB", CircleAabbFn);
        let aabb_aabb = symbol!("c2AABBtoAABB", AabbAabbFn);
        let f2 = symbol!("f2", F2Fn);
        let f3 = symbol!("f3", F3Fn);
        let f4 = symbol!("f4", F4Fn);
        let f5 = symbol!("f5", F5Fn);
        let f7 = symbol!("f7", F7Fn);
        let f9 = symbol!("f9", F9Fn);
        let f10 = symbol!("f10", F10Fn);
        let f11 = symbol!("f11", ColorFn);
        let f12 = symbol!("f12", ColorFn);
        let f13 = symbol!("f13", ColorFn);
        let agglom = symbol!("agglom", AgglomFn);
        Self {
            _library: library,
            c2_v,
            c2_maxv,
            c2_minv,
            c2_clampv,
            c2_sub,
            c2_dot,
            circle_circle,
            circle_aabb,
            aabb_aabb,
            f2,
            f3,
            f4,
            f5,
            f7,
            f9,
            f10,
            f11,
            f12,
            f13,
            agglom,
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-cKI30P.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libagglom_lib.so")
}

fn pair() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing release Rust shared library: {}",
        rust_path.display()
    );
    unsafe { (Api::open(&c_path), Api::open(&rust_path)) }
}

fn same_f32(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn same_f64(c: f64, rust: f64, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#018x}), Rust={rust:?} ({:#018x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn same_v(c: C2v, rust: C2v, context: &str) {
    same_f32(c.x, rust.x, &format!("{context}.x"));
    same_f32(c.y, rust.y, &format!("{context}.y"));
}

fn same_lm(c: LmVec2, rust: LmVec2, context: &str) {
    same_f32(c.x, rust.x, &format!("{context}.x"));
    same_f32(c.y, rust.y, &format!("{context}.y"));
}

fn same_rgb(c: [f32; 3], rust: [f32; 3], context: &str) {
    for i in 0..3 {
        same_f32(c[i], rust[i], &format!("{context}[{i}]"));
    }
}

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

    fn moderate_f32(&mut self) -> f32 {
        ((self.u32() % 20_001) as i32 - 10_000) as f32 / 16.0
    }

    fn unit_f32(&mut self) -> f32 {
        (self.u32() & 0xffff) as f32 / 65_535.0
    }
}

#[test]
fn vectors_configs_1_through_6() {
    let (c, rust) = pair();
    let mut rng = Rng::new(0x8e5d_7a31_c496_02bf);
    for i in 0..2_000 {
        let a = C2v {
            x: rng.moderate_f32(),
            y: rng.moderate_f32(),
        };
        let b = C2v {
            x: rng.moderate_f32(),
            y: rng.moderate_f32(),
        };
        let lo = C2v {
            x: -100.0,
            y: -50.0,
        };
        let hi = C2v { x: 100.0, y: 50.0 };
        unsafe {
            same_v(
                (c.c2_v)(a.x, a.y),
                (rust.c2_v)(a.x, a.y),
                &format!("c2V {i}"),
            );
            same_v((c.c2_maxv)(a, b), (rust.c2_maxv)(a, b), &format!("max {i}"));
            same_v((c.c2_minv)(a, b), (rust.c2_minv)(a, b), &format!("min {i}"));
            same_v(
                (c.c2_clampv)(a, lo, hi),
                (rust.c2_clampv)(a, lo, hi),
                &format!("clamp {i}"),
            );
            same_v((c.c2_sub)(a, b), (rust.c2_sub)(a, b), &format!("sub {i}"));
            same_f32((c.c2_dot)(a, b), (rust.c2_dot)(a, b), &format!("dot {i}"));
        }
    }

    let special = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_5678),
    ];
    for (i, &x) in special.iter().enumerate() {
        for (j, &y) in special.iter().enumerate() {
            let a = C2v { x, y };
            let b = C2v { x: y, y: x };
            unsafe {
                same_v(
                    (c.c2_v)(x, y),
                    (rust.c2_v)(x, y),
                    &format!("special V {i}/{j}"),
                );
                same_v((c.c2_maxv)(a, b), (rust.c2_maxv)(a, b), "special max");
                same_v((c.c2_minv)(a, b), (rust.c2_minv)(a, b), "special min");
                same_v((c.c2_sub)(a, b), (rust.c2_sub)(a, b), "special sub");
                same_f32((c.c2_dot)(a, b), (rust.c2_dot)(a, b), "special dot");
            }
        }
    }

    for x_class in 0..3 {
        for y_class in 0..3 {
            let a = C2v {
                x: [-2.0, 0.0, 2.0][x_class],
                y: [-2.0, 0.0, 2.0][y_class],
            };
            let lo = C2v { x: -1.0, y: -1.0 };
            let hi = C2v { x: 1.0, y: 1.0 };
            unsafe {
                same_v(
                    (c.c2_clampv)(a, lo, hi),
                    (rust.c2_clampv)(a, lo, hi),
                    "clamp class",
                );
            }
        }
    }
}

#[test]
fn collision_configs_7_through_18() {
    let (c, rust) = pair();
    let mut rng = Rng::new(0x19bb_48e2_51a0_d74c);
    for i in 0..3_000 {
        let a = C2Circle {
            p: C2v {
                x: rng.moderate_f32(),
                y: rng.moderate_f32(),
            },
            r: rng.moderate_f32() / 8.0,
        };
        let b = C2Circle {
            p: C2v {
                x: rng.moderate_f32(),
                y: rng.moderate_f32(),
            },
            r: rng.moderate_f32() / 8.0,
        };
        let box_a = C2Aabb {
            min: C2v {
                x: rng.moderate_f32(),
                y: rng.moderate_f32(),
            },
            max: C2v {
                x: rng.moderate_f32(),
                y: rng.moderate_f32(),
            },
        };
        let box_b = C2Aabb {
            min: C2v {
                x: rng.moderate_f32(),
                y: rng.moderate_f32(),
            },
            max: C2v {
                x: rng.moderate_f32(),
                y: rng.moderate_f32(),
            },
        };
        unsafe {
            assert_eq!(
                (c.circle_circle)(a, b),
                (rust.circle_circle)(a, b),
                "circle-circle {i}"
            );
            assert_eq!(
                (c.circle_aabb)(a, box_a),
                (rust.circle_aabb)(a, box_a),
                "circle-aabb {i}"
            );
            assert_eq!(
                (c.aabb_aabb)(box_a, box_b),
                (rust.aabb_aabb)(box_a, box_b),
                "aabb {i}"
            );
            assert_eq!(
                (c.f2)(
                    (&a as *const C2Circle).cast(),
                    0,
                    (&b as *const C2Circle).cast(),
                    0
                ),
                (rust.f2)(
                    (&a as *const C2Circle).cast(),
                    0,
                    (&b as *const C2Circle).cast(),
                    0
                ),
                "f2 CC {i}"
            );
            assert_eq!(
                (c.f2)(
                    (&a as *const C2Circle).cast(),
                    0,
                    (&box_a as *const C2Aabb).cast(),
                    1
                ),
                (rust.f2)(
                    (&a as *const C2Circle).cast(),
                    0,
                    (&box_a as *const C2Aabb).cast(),
                    1
                ),
                "f2 CA {i}"
            );
            assert_eq!(
                (c.f2)(
                    (&box_a as *const C2Aabb).cast(),
                    1,
                    (&a as *const C2Circle).cast(),
                    0
                ),
                (rust.f2)(
                    (&box_a as *const C2Aabb).cast(),
                    1,
                    (&a as *const C2Circle).cast(),
                    0
                ),
                "f2 AC {i}"
            );
            assert_eq!(
                (c.f2)(
                    (&box_a as *const C2Aabb).cast(),
                    1,
                    (&box_b as *const C2Aabb).cast(),
                    1
                ),
                (rust.f2)(
                    (&box_a as *const C2Aabb).cast(),
                    1,
                    (&box_b as *const C2Aabb).cast(),
                    1
                ),
                "f2 AA {i}"
            );
        }
    }

    let origin = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    for (distance, expected) in [(3.0, 0), (2.0, 0), (1.5, 1)] {
        let other = C2Circle {
            p: C2v {
                x: distance,
                y: 0.0,
            },
            r: 1.0,
        };
        unsafe {
            assert_eq!((c.circle_circle)(origin, other), expected);
            assert_eq!((rust.circle_circle)(origin, other), expected);
        }
    }

    let bbox = C2Aabb {
        min: C2v { x: -1.0, y: -1.0 },
        max: C2v { x: 1.0, y: 1.0 },
    };
    for &x in &[-2.0, 0.0, 2.0] {
        for &y in &[-2.0, 0.0, 2.0] {
            for &radius in &[0.0, 1.0, 2.0] {
                let circle = C2Circle {
                    p: C2v { x, y },
                    r: radius,
                };
                unsafe {
                    assert_eq!(
                        (c.circle_aabb)(circle, bbox),
                        (rust.circle_aabb)(circle, bbox)
                    );
                }
            }
        }
    }

    let touching = C2Aabb {
        min: C2v { x: 1.0, y: -1.0 },
        max: C2v { x: 2.0, y: 1.0 },
    };
    unsafe {
        assert_eq!((c.aabb_aabb)(bbox, touching), 1);
        assert_eq!((rust.aabb_aabb)(bbox, touching), 1);
    }
}

#[test]
fn f3_configs_19_through_27() {
    let (c, rust) = pair();
    let boundaries = [
        c_int::MIN,
        c_int::MIN + 1,
        -1_000_000_007,
        -3,
        -2,
        -1,
        1,
        2,
        3,
        1_000_000_007,
        c_int::MAX,
    ];
    for &v1 in &boundaries {
        for &v2 in &boundaries {
            unsafe {
                assert_eq!((c.f3)(v1, v2), (rust.f3)(v1, v2), "f3({v1}, {v2})");
            }
        }
    }
    let mut rng = Rng::new(0x752c_f840_691d_aab3);
    for i in 0..100_000 {
        let v1 = rng.u32() as c_int;
        let mut v2 = rng.u32() as c_int;
        if v2 == 0 {
            v2 = 1;
        }
        unsafe {
            assert_eq!((c.f3)(v1, v2), (rust.f3)(v1, v2), "random f3 {i}");
        }
    }
}

#[test]
fn scalar_configs_28_through_33() {
    let (c, rust) = pair();
    let mut rng = Rng::new(0x6ec9_b157_203d_48fa);
    let states = [[0, 0], [u64::MAX, u64::MAX], [0, u64::MAX], [u64::MAX, 0]];
    for state in states {
        let mut c_state = CnRnd { state };
        let mut r_state = CnRnd { state };
        unsafe {
            same_f64((c.f4)(&mut c_state), (rust.f4)(&mut r_state), "f4 boundary");
        }
        assert_eq!(c_state.state, r_state.state);
    }
    for i in 0..20_000 {
        let state = [rng.u64(), rng.u64()];
        let mut c_state = CnRnd { state };
        let mut r_state = CnRnd { state };
        unsafe {
            same_f64(
                (c.f4)(&mut c_state),
                (rust.f4)(&mut r_state),
                &format!("f4 {i}"),
            );
            assert_eq!(
                (c.f5)(state[0] as u32),
                (rust.f5)(state[0] as u32),
                "f5 {i}"
            );
        }
        assert_eq!(c_state.state, r_state.state, "f4 state {i}");
    }

    let blocks = [0, 1, 2, 65_535, u32::MAX];
    let channels = [0, 1, 2, 3, 8, u32::MAX];
    let depths = [0, 1, 8, 16, 24, 31, 32, 33, u32::MAX];
    for &block in &blocks {
        for &channel in &channels {
            for &depth in &depths {
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
    for i in 0..20_000 {
        let block = rng.u32();
        let channel = rng.u32();
        let depth = rng.u32();
        unsafe {
            assert_eq!(
                (c.f7)(block, channel, depth),
                (rust.f7)(block, channel, depth),
                "random f7 {i}"
            );
        }
    }
}

#[test]
fn f9_configs_34_and_35() {
    let (c, rust) = pair();
    let mut rng = Rng::new(0xd74a_3c91_06ef_5b28);
    for i in 0..10_000 {
        let p1 = LmVec2 {
            x: rng.moderate_f32(),
            y: rng.moderate_f32(),
        };
        let p2 = LmVec2 {
            x: p1.x + 1.0 + rng.unit_f32(),
            y: p1.y,
        };
        let p3 = LmVec2 {
            x: p1.x,
            y: p1.y + 1.0 + rng.unit_f32(),
        };
        let p = LmVec2 {
            x: rng.moderate_f32(),
            y: rng.moderate_f32(),
        };
        unsafe {
            same_lm(
                (c.f9)(p1, p2, p3, p),
                (rust.f9)(p1, p2, p3, p),
                &format!("f9 {i}"),
            );
        }
    }
    let degenerate = [
        (
            LmVec2 { x: 0.0, y: 0.0 },
            LmVec2 { x: 0.0, y: 0.0 },
            LmVec2 { x: 0.0, y: 0.0 },
        ),
        (
            LmVec2 { x: 1.0, y: 1.0 },
            LmVec2 { x: 2.0, y: 2.0 },
            LmVec2 { x: 3.0, y: 3.0 },
        ),
    ];
    for (p1, p2, p3) in degenerate {
        let p = LmVec2 { x: 4.0, y: -2.0 };
        unsafe {
            same_lm(
                (c.f9)(p1, p2, p3, p),
                (rust.f9)(p1, p2, p3, p),
                "degenerate f9",
            );
        }
    }
}

#[test]
fn f10_configs_36_through_40_exhaustive() {
    let (c, rust) = pair();
    for h in 0_u16..=u16::MAX {
        unsafe {
            same_f32((c.f10)(h), (rust.f10)(h), &format!("f10 {h:#06x}"));
        }
    }
}

fn compare_color(c_fn: ColorFn, rust_fn: ColorFn, src: [f32; 3], context: &str) {
    let mut c_out = [f32::from_bits(0x7fc0_1111); 3];
    let mut rust_out = [f32::from_bits(0x7fc0_2222); 3];
    unsafe {
        c_fn(c_out.as_mut_ptr(), src.as_ptr());
        rust_fn(rust_out.as_mut_ptr(), src.as_ptr());
    }
    same_rgb(c_out, rust_out, context);
}

#[test]
fn f11_configs_41_through_49() {
    let (c, rust) = pair();
    let mut rng = Rng::new(0x93f1_a684_25cd_70be);
    for i in 0..1_000 {
        compare_color(
            c.f11,
            rust.f11,
            [rng.moderate_f32(), 0.0, rng.unit_f32()],
            "f11 gray",
        );
        for (sector, (low, high)) in [
            (-1, (-720.0, -0.001)),
            (0, (0.0, 59.999)),
            (1, (60.0, 119.999)),
            (2, (120.0, 179.999)),
            (3, (180.0, 239.999)),
            (4, (240.0, 299.999)),
            (5, (300.0, 359.999)),
            (6, (360.0, 720.0)),
        ] {
            let h = low + (high - low) * rng.unit_f32();
            compare_color(
                c.f11,
                rust.f11,
                [h, 0.01 + rng.unit_f32(), rng.unit_f32()],
                &format!("f11 sector {sector} iteration {i}"),
            );
        }
    }
    compare_color(
        c.f11,
        rust.f11,
        [f32::from_bits(0x7fc0_1234), 1.0, 0.5],
        "f11 NaN hue",
    );
}

#[test]
fn f12_configs_50_through_56() {
    let (c, rust) = pair();
    let mut rng = Rng::new(0xba06_1dc7_e583_429f);
    for i in 0..2_000 {
        compare_color(
            c.f12,
            rust.f12,
            [rng.moderate_f32(), 0.0, rng.unit_f32()],
            "f12 gray",
        );
        for sector in -3..=8 {
            let h = (sector as f32 + rng.unit_f32()) * 60.0;
            compare_color(
                c.f12,
                rust.f12,
                [h, 0.01 + rng.unit_f32(), rng.unit_f32()],
                &format!("f12 sector {sector} iteration {i}"),
            );
        }
    }
}

#[test]
fn f13_configs_57_through_62() {
    let (c, rust) = pair();
    for src in [
        [0.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [0.0, -1.0, -0.5],
        [1.0, 0.5, 0.25],
        [1.0, 0.25, 0.5],
        [0.25, 1.0, 0.5],
        [0.25, 0.5, 1.0],
    ] {
        compare_color(c.f13, rust.f13, src, "f13 crafted");
    }
    let mut rng = Rng::new(0x41d8_7b2e_c560_9af3);
    for i in 0..20_000 {
        compare_color(
            c.f13,
            rust.f13,
            [rng.moderate_f32(), rng.moderate_f32(), rng.moderate_f32()],
            &format!("f13 random {i}"),
        );
    }
}

fn random_agglom_args(
    rng: &mut Rng,
) -> (
    [f32; 7],
    [c_int; 2],
    [u64; 2],
    u32,
    [u32; 3],
    [f32; 8],
    u16,
    [f32; 9],
) {
    (
        std::array::from_fn(|_| rng.moderate_f32()),
        [rng.u32() as c_int, (rng.u32() as c_int) | 1],
        [rng.u64(), rng.u64()],
        rng.u32(),
        [rng.u32(), rng.u32(), rng.u32()],
        std::array::from_fn(|_| rng.moderate_f32()),
        rng.u32() as u16,
        std::array::from_fn(|_| rng.moderate_f32()),
    )
}

unsafe fn call_agglom(
    f: AgglomFn,
    a: [f32; 7],
    ints: [c_int; 2],
    state: [u64; 2],
    bits: u32,
    flac: [u32; 3],
    bary: [f32; 8],
    half: u16,
    colors: [f32; 9],
) -> f64 {
    unsafe {
        f(
            a[0], a[1], a[2], a[3], a[4], a[5], a[6], ints[0], ints[1], state[0], state[1], bits,
            flac[0], flac[1], flac[2], bary[0], bary[1], bary[2], bary[3], bary[4], bary[5],
            bary[6], bary[7], half, colors[0], colors[1], colors[2], colors[3], colors[4],
            colors[5], colors[6], colors[7], colors[8],
        )
    }
}

#[test]
fn agglom_configs_63_and_64() {
    let (c, rust) = pair();
    let mut rng = Rng::new(0xf18c_540b_7a29_e6d3);
    for i in 0..10_000 {
        let (a, ints, state, bits, flac, bary, half, colors) = random_agglom_args(&mut rng);
        unsafe {
            same_f64(
                call_agglom(c.agglom, a, ints, state, bits, flac, bary, half, colors),
                call_agglom(rust.agglom, a, ints, state, bits, flac, bary, half, colors),
                &format!("agglom random {i}"),
            );
        }
    }

    let nan = f32::from_bits(0x7fc0_1234);
    let specials = [
        [nan; 33],
        [f32::INFINITY; 33],
        [f32::NEG_INFINITY; 33],
        std::array::from_fn(|i| {
            if i % 3 == 0 {
                nan
            } else if i % 3 == 1 {
                0.0
            } else {
                -0.0
            }
        }),
    ];
    for (i, values) in specials.into_iter().enumerate() {
        let a: [f32; 7] = values[0..7].try_into().unwrap();
        let bary: [f32; 8] = values[7..15].try_into().unwrap();
        let colors: [f32; 9] = values[15..24].try_into().unwrap();
        unsafe {
            same_f64(
                call_agglom(
                    c.agglom,
                    a,
                    [c_int::MIN, 0],
                    [0, u64::MAX],
                    u32::MAX,
                    [0, 2, 32],
                    bary,
                    0x7e01,
                    colors,
                ),
                call_agglom(
                    rust.agglom,
                    a,
                    [c_int::MIN, 0],
                    [0, u64::MAX],
                    u32::MAX,
                    [0, 2, 32],
                    bary,
                    0x7e01,
                    colors,
                ),
                &format!("agglom special {i}"),
            );
        }
    }
}

#[test]
fn explicit_error_rows_1_through_4() {
    let (c, rust) = pair();
    let null = std::ptr::null::<c_void>();
    unsafe {
        assert_eq!((c.f2)(null, 0, null, 2), 0);
        assert_eq!((rust.f2)(null, 0, null, 2), 0);
        assert_eq!((c.f2)(null, 1, null, -1), 0);
        assert_eq!((rust.f2)(null, 1, null, -1), 0);
        for invalid_a in [c_int::MIN, -1, 2, c_int::MAX] {
            assert_eq!((c.f2)(null, invalid_a, null, 0), 0);
            assert_eq!((rust.f2)(null, invalid_a, null, 0), 0);
        }
        for v1 in [c_int::MIN, -1, 0, 1, c_int::MAX] {
            assert_eq!((c.f3)(v1, 0), 0);
            assert_eq!((rust.f3)(v1, 0), 0);
        }
    }
}

#[test]
fn null_child() {
    let Some(library) = std::env::var_os("DIFF_NULL_LIB") else {
        return;
    };
    let case = std::env::var("DIFF_NULL_CASE").unwrap();
    let api = unsafe { Api::open(Path::new(&library)) };
    let mut out = [0.0_f32; 3];
    let src = [0.0_f32, 0.0, 0.0];
    let circle = C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: 1.0,
    };
    unsafe {
        match case.as_str() {
            "f2_a" => {
                (api.f2)(std::ptr::null(), 0, (&circle as *const C2Circle).cast(), 0);
            }
            "f2_b" => {
                (api.f2)((&circle as *const C2Circle).cast(), 0, std::ptr::null(), 0);
            }
            "f4" => {
                (api.f4)(std::ptr::null_mut());
            }
            "f11_dest" => {
                (api.f11)(std::ptr::null_mut(), src.as_ptr());
            }
            "f11_src" => {
                (api.f11)(out.as_mut_ptr(), std::ptr::null());
            }
            "f12_dest" => {
                (api.f12)(std::ptr::null_mut(), src.as_ptr());
            }
            "f12_src" => {
                (api.f12)(out.as_mut_ptr(), std::ptr::null());
            }
            "f13_dest" => {
                (api.f13)(std::ptr::null_mut(), src.as_ptr());
            }
            "f13_src" => {
                (api.f13)(out.as_mut_ptr(), std::ptr::null());
            }
            _ => panic!("unknown null case {case}"),
        }
    }
    panic!("null case unexpectedly returned: {case}");
}

#[test]
fn null_error_rows_5_through_13() {
    let current = std::env::current_exe().unwrap();
    for case in [
        "f2_a", "f2_b", "f4", "f11_dest", "f11_src", "f12_dest", "f12_src", "f13_dest", "f13_src",
    ] {
        let c_status = Command::new(&current)
            .args(["--exact", "null_child", "--nocapture"])
            .env("DIFF_NULL_LIB", c_library_path())
            .env("DIFF_NULL_CASE", case)
            .status()
            .unwrap();
        let rust_status = Command::new(&current)
            .args(["--exact", "null_child", "--nocapture"])
            .env("DIFF_NULL_LIB", rust_library_path())
            .env("DIFF_NULL_CASE", case)
            .status()
            .unwrap();
        assert!(
            !c_status.success(),
            "C null case unexpectedly succeeded: {case}"
        );
        assert!(
            !rust_status.success(),
            "Rust null case unexpectedly succeeded: {case}"
        );
        #[cfg(unix)]
        assert_eq!(
            (c_status.code(), c_status.signal()),
            (rust_status.code(), rust_status.signal()),
            "different process termination for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
        #[cfg(not(unix))]
        assert_eq!(
            c_status.code(),
            rust_status.code(),
            "different process termination for {case}"
        );
    }
}
