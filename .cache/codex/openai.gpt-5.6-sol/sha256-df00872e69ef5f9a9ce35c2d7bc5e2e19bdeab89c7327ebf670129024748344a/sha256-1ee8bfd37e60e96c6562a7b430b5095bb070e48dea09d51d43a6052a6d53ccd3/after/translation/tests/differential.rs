use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CnRnd {
    state: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libharvest-work-80YgMl.so");
        let rust_path = manifest.join("target/release/libagglom_lib.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        // SAFETY: Both paths name shared libraries built by this workspace.
        unsafe {
            Self {
                c: Library::new(c_path).expect("load C library"),
                rust: Library::new(rust_path).expect("load Rust library"),
            }
        }
    }

    unsafe fn functions<T: Copy>(&self, name: &[u8]) -> (T, T) {
        // SAFETY: Every call site supplies the C declaration's exact ABI type.
        unsafe {
            (
                *self.c.get::<T>(name).expect("C symbol"),
                *self.rust.get::<T>(name).expect("Rust symbol"),
            )
        }
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

    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }

    fn finite(&mut self) -> f32 {
        (self.i32() % 65_537) as f32 / 32.0
    }

    fn unit(&mut self) -> f32 {
        (self.u32() % 10_001) as f32 / 10_000.0
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
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

fn assert_f64(c: f64, rust: f64, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#018x}), Rust={rust:?} ({:#018x})",
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

type VecCtor = unsafe extern "C" fn(f32, f32) -> C2v;
type VecBinary = unsafe extern "C" fn(C2v, C2v) -> C2v;
type VecClamp = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type VecDot = unsafe extern "C" fn(C2v, C2v) -> f32;

#[test]
fn vectors_cover_c1_through_c20() {
    let libs = Libraries::load();
    // SAFETY: Types match the C definitions.
    let (c_v, r_v) = unsafe { libs.functions::<VecCtor>(b"c2V\0") };
    let (c_max, r_max) = unsafe { libs.functions::<VecBinary>(b"c2Maxv\0") };
    let (c_min, r_min) = unsafe { libs.functions::<VecBinary>(b"c2Minv\0") };
    let (c_clamp, r_clamp) = unsafe { libs.functions::<VecClamp>(b"c2Clampv\0") };
    let (c_sub, r_sub) = unsafe { libs.functions::<VecBinary>(b"c2Sub\0") };
    let (c_dot, r_dot) = unsafe { libs.functions::<VecDot>(b"c2Dot\0") };
    let mut rng = Rng::new(0x1010_2020_3030_4040);

    // C1, including all non-finite classes and signed zero.
    let specials = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_5678),
    ];
    for i in 0..CASES {
        let x = if i < specials.len() {
            specials[i]
        } else {
            rng.finite()
        };
        let y = if i + 1 < specials.len() {
            specials[i + 1]
        } else {
            rng.finite()
        };
        assert_v(unsafe { c_v(x, y) }, unsafe { r_v(x, y) }, "C1");
    }

    // C2-C5 and C6-C9: all two-axis branch cross-products.
    for x_first in [false, true] {
        for y_first in [false, true] {
            for _ in 0..CASES {
                let b = C2v {
                    x: rng.finite(),
                    y: rng.finite(),
                };
                let a_max = C2v {
                    x: b.x + if x_first { 1.0 } else { -1.0 },
                    y: b.y + if y_first { 1.0 } else { -1.0 },
                };
                assert_v(
                    unsafe { c_max(a_max, b) },
                    unsafe { r_max(a_max, b) },
                    "C2-C5",
                );
                let a_min = C2v {
                    x: b.x + if x_first { -1.0 } else { 1.0 },
                    y: b.y + if y_first { -1.0 } else { 1.0 },
                };
                assert_v(
                    unsafe { c_min(a_min, b) },
                    unsafe { r_min(a_min, b) },
                    "C6-C9",
                );
            }
        }
    }
    let nan_a = C2v {
        x: f32::from_bits(0x7fc0_1111),
        y: f32::from_bits(0x7fc0_2222),
    };
    let ordinary = C2v { x: 1.0, y: 2.0 };
    assert_v(
        unsafe { c_max(nan_a, ordinary) },
        unsafe { r_max(nan_a, ordinary) },
        "C5 NaN",
    );
    assert_v(
        unsafe { c_min(nan_a, ordinary) },
        unsafe { r_min(nan_a, ordinary) },
        "C9 NaN",
    );

    // C10-C18: below/inside/above for each axis.
    for x_region in 0..3 {
        for y_region in 0..3 {
            for _ in 0..CASES {
                let lo = C2v {
                    x: rng.between(-100.0, 0.0),
                    y: rng.between(-100.0, 0.0),
                };
                let hi = C2v {
                    x: lo.x + rng.between(1.0, 100.0),
                    y: lo.y + rng.between(1.0, 100.0),
                };
                let select = |region, low: f32, high: f32| match region {
                    0 => low - 1.0,
                    1 => (low + high) * 0.5,
                    _ => high + 1.0,
                };
                let a = C2v {
                    x: select(x_region, lo.x, hi.x),
                    y: select(y_region, lo.y, hi.y),
                };
                assert_v(
                    unsafe { c_clamp(a, lo, hi) },
                    unsafe { r_clamp(a, lo, hi) },
                    "C10-C18",
                );
            }
        }
    }

    // C19-C20.
    for _ in 0..CASES * 8 {
        let a = C2v {
            x: rng.finite(),
            y: rng.finite(),
        };
        let b = C2v {
            x: rng.finite(),
            y: rng.finite(),
        };
        assert_v(unsafe { c_sub(a, b) }, unsafe { r_sub(a, b) }, "C19");
        assert_f32(unsafe { c_dot(a, b) }, unsafe { r_dot(a, b) }, "C20");
    }
    let signed = C2v { x: -0.0, y: 0.0 };
    assert_f32(
        unsafe { c_dot(signed, signed) },
        unsafe { r_dot(signed, signed) },
        "C20 signed zero",
    );
}

type CircleCircle = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type CircleAabb = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type AabbAabb = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type F2 = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;

#[test]
fn geometry_and_dispatch_cover_c21_through_c39() {
    let libs = Libraries::load();
    let (c_cc, r_cc) = unsafe { libs.functions::<CircleCircle>(b"c2CircletoCircle\0") };
    let (c_cb, r_cb) = unsafe { libs.functions::<CircleAabb>(b"c2CircletoAABB\0") };
    let (c_bb, r_bb) = unsafe { libs.functions::<AabbAabb>(b"c2AABBtoAABB\0") };
    let (c_f2, r_f2) = unsafe { libs.functions::<F2>(b"f2\0") };
    let mut rng = Rng::new(0x2122_2324_2526_2728);

    // C21-C23.
    for relation in 0..3 {
        for _ in 0..CASES {
            let origin = C2v {
                x: (rng.u32() % 100) as f32,
                y: (rng.u32() % 100) as f32,
            };
            let ra = (rng.u32() % 20 + 1) as f32;
            let rb = (rng.u32() % 20 + 1) as f32;
            let sum = ra + rb;
            let distance = match relation {
                0 => sum - 0.5,
                1 => sum,
                _ => sum + 0.5,
            };
            let a = C2Circle { p: origin, r: ra };
            let b = C2Circle {
                p: C2v {
                    x: origin.x + distance,
                    y: origin.y,
                },
                r: rb,
            };
            assert_eq!(unsafe { c_cc(a, b) }, unsafe { r_cc(a, b) }, "C21-C23");
        }
    }

    // C24-C29.
    for shape in 0..3 {
        for collides in [false, true] {
            for _ in 0..CASES {
                let tx = (rng.u32() % 100) as f32;
                let ty = (rng.u32() % 100) as f32;
                let b = C2Aabb {
                    min: C2v { x: tx, y: ty },
                    max: C2v {
                        x: tx + 10.0,
                        y: ty + 10.0,
                    },
                };
                let (p, r) = match (shape, collides) {
                    (0, true) => (
                        C2v {
                            x: tx + 5.0,
                            y: ty + 5.0,
                        },
                        1.0,
                    ),
                    (0, false) => (
                        C2v {
                            x: tx + 5.0,
                            y: ty + 5.0,
                        },
                        0.0,
                    ),
                    (1, true) => (
                        C2v {
                            x: tx - 1.0,
                            y: ty + 5.0,
                        },
                        2.0,
                    ),
                    (1, false) => (
                        C2v {
                            x: tx - 2.0,
                            y: ty + 5.0,
                        },
                        2.0,
                    ),
                    (2, true) => (
                        C2v {
                            x: tx - 1.0,
                            y: ty - 1.0,
                        },
                        2.0,
                    ),
                    _ => (
                        C2v {
                            x: tx - 2.0,
                            y: ty - 2.0,
                        },
                        2.0,
                    ),
                };
                let a = C2Circle { p, r };
                assert_eq!(unsafe { c_cb(a, b) }, unsafe { r_cb(a, b) }, "C24-C29");
            }
        }
    }

    // C30-C35.
    for separation in 0..6 {
        for _ in 0..CASES {
            let a = C2Aabb {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 10.0, y: 10.0 },
            };
            let b = match separation {
                0 => C2Aabb {
                    min: C2v { x: 10.0, y: 2.0 },
                    max: C2v { x: 15.0, y: 8.0 },
                },
                1 => C2Aabb {
                    min: C2v { x: -10.0, y: 2.0 },
                    max: C2v { x: -1.0, y: 8.0 },
                },
                2 => C2Aabb {
                    min: C2v { x: 11.0, y: 2.0 },
                    max: C2v { x: 20.0, y: 8.0 },
                },
                3 => C2Aabb {
                    min: C2v { x: 2.0, y: -10.0 },
                    max: C2v { x: 8.0, y: -1.0 },
                },
                4 => C2Aabb {
                    min: C2v { x: 2.0, y: 11.0 },
                    max: C2v { x: 8.0, y: 20.0 },
                },
                _ => C2Aabb {
                    min: C2v { x: 11.0, y: 11.0 },
                    max: C2v { x: 20.0, y: 20.0 },
                },
            };
            assert_eq!(unsafe { c_bb(a, b) }, unsafe { r_bb(a, b) }, "C30-C35");
        }
    }

    // C36-C39 call the generic dispatcher itself through both FFIs.
    for tags in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        for _ in 0..CASES {
            let circle_a = C2Circle {
                p: C2v {
                    x: rng.finite(),
                    y: rng.finite(),
                },
                r: rng.between(0.0, 100.0),
            };
            let circle_b = C2Circle {
                p: C2v {
                    x: rng.finite(),
                    y: rng.finite(),
                },
                r: rng.between(0.0, 100.0),
            };
            let box_a = C2Aabb {
                min: C2v { x: -10.0, y: -20.0 },
                max: C2v { x: 30.0, y: 40.0 },
            };
            let box_b = C2Aabb {
                min: C2v { x: -30.0, y: -40.0 },
                max: C2v { x: 10.0, y: 20.0 },
            };
            let (a, b): (*const c_void, *const c_void) = match tags {
                (0, 0) => (
                    (&circle_a as *const C2Circle).cast(),
                    (&circle_b as *const C2Circle).cast(),
                ),
                (0, 1) => (
                    (&circle_a as *const C2Circle).cast(),
                    (&box_b as *const C2Aabb).cast(),
                ),
                (1, 0) => (
                    (&box_a as *const C2Aabb).cast(),
                    (&circle_b as *const C2Circle).cast(),
                ),
                _ => (
                    (&box_a as *const C2Aabb).cast(),
                    (&box_b as *const C2Aabb).cast(),
                ),
            };
            assert_eq!(
                unsafe { c_f2(a, tags.0, b, tags.1) },
                unsafe { r_f2(a, tags.0, b, tags.1) },
                "C36-C39"
            );
        }
    }
}

type F3 = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[test]
fn integer_division_covers_c40_through_c50_and_e4() {
    let libs = Libraries::load();
    let (c_f3, r_f3) = unsafe { libs.functions::<F3>(b"f3\0") };
    let mut rng = Rng::new(0x4041_4243_4445_4647);
    let compare = |a, b, row: &str| {
        assert_eq!(
            unsafe { c_f3(a, b) },
            unsafe { r_f3(a, b) },
            "{row}: {a}/{b}"
        );
    };

    for _ in 0..CASES {
        let positive_a = (rng.u32() & 0x7fff_ffff) as c_int;
        let positive_b = (rng.u32() % 100_000 + 1) as c_int;
        compare(positive_a, positive_b, "C40");
        compare(positive_a, -positive_b, "C41");
        compare(positive_a, c_int::MIN, "C42");

        let divisor = (rng.u32() % 10_000 + 2) as c_int;
        let factor = (rng.u32() % 10_000 + 1) as c_int;
        compare(-divisor * factor, divisor, "C43");
        compare(-divisor * factor - 1, divisor, "C44");
        compare(-divisor * factor, -divisor, "C45");
        compare(-divisor * factor - 1, -divisor, "C46");
        compare(-((rng.u32() % 1_000_000 + 1) as c_int), c_int::MIN, "C47");
        compare(c_int::MIN, positive_b, "C48");
        compare(c_int::MIN, -positive_b, "C49");
        compare(c_int::MIN, c_int::MIN, "C50");
        compare(rng.i32(), 0, "E4");
    }

    // Broad value-dependent coverage, including signed boundaries.
    for _ in 0..CASES * 64 {
        compare(rng.i32(), rng.i32(), "C40-C50 randomized");
    }
    for a in [c_int::MIN, c_int::MIN + 1, -1, 0, 1, c_int::MAX] {
        for b in [c_int::MIN, c_int::MIN + 1, -1, 0, 1, c_int::MAX] {
            compare(a, b, "C40-C50 boundaries");
        }
    }
}

type F4 = unsafe extern "C" fn(*mut CnRnd) -> f64;
type F5 = unsafe extern "C" fn(u32) -> u32;
type F7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
type F9 = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
type F10 = unsafe extern "C" fn(u16) -> f32;

#[test]
fn numeric_helpers_cover_c51_through_c66() {
    let libs = Libraries::load();
    let (c_f4, r_f4) = unsafe { libs.functions::<F4>(b"f4\0") };
    let (c_f5, r_f5) = unsafe { libs.functions::<F5>(b"f5\0") };
    let (c_f7, r_f7) = unsafe { libs.functions::<F7>(b"f7\0") };
    let (c_f9, r_f9) = unsafe { libs.functions::<F9>(b"f9\0") };
    let (c_f10, r_f10) = unsafe { libs.functions::<F10>(b"f10\0") };
    let mut rng = Rng::new(0x5152_5354_5556_5758);

    // C51 compares both the result and externally visible state mutation.
    for i in 0..CASES * 8 {
        let initial = match i {
            0 => CnRnd { state: [0, 0] },
            1 => CnRnd {
                state: [u64::MAX, u64::MAX],
            },
            _ => CnRnd {
                state: [rng.u64(), rng.u64()],
            },
        };
        let mut c_state = initial;
        let mut r_state = initial;
        assert_f64(
            unsafe { c_f4(&mut c_state) },
            unsafe { r_f4(&mut r_state) },
            "C51",
        );
        assert_eq!(c_state, r_state, "C51 state");
    }

    // C52-C53.
    for _ in 0..CASES * 16 {
        let low = rng.u32() & 0xffff;
        assert_eq!(unsafe { c_f5(low) }, unsafe { r_f5(low) }, "C52");
        let high = low | (rng.u32() & 0xffff_0000) | 0x8000_0000;
        assert_eq!(unsafe { c_f5(high) }, unsafe { r_f5(high) }, "C53");
    }

    // C54-C57 ordinary branch cross-product.
    for channels in [1_u32, 2] {
        for bitdepth in [24_u32, 32] {
            for _ in 0..CASES {
                let blocksize = rng.u32() % 65_536;
                assert_eq!(
                    unsafe { c_f7(blocksize, channels, bitdepth) },
                    unsafe { r_f7(blocksize, channels, bitdepth) },
                    "C54-C57"
                );
            }
        }
    }
    // C58 zero boundaries.
    for _ in 0..CASES {
        let values = [
            (0, rng.u32(), rng.u32()),
            (rng.u32(), 0, rng.u32()),
            (rng.u32(), rng.u32(), 0),
            (0, 0, 0),
        ];
        for (blocksize, channels, bitdepth) in values {
            assert_eq!(
                unsafe { c_f7(blocksize, channels, bitdepth) },
                unsafe { r_f7(blocksize, channels, bitdepth) },
                "C58"
            );
        }
    }
    // C59 wrapping unsigned intermediates.
    for _ in 0..CASES * 8 {
        let values = (rng.u32() | 0x8000_0000, rng.u32(), rng.u32());
        assert_eq!(
            unsafe { c_f7(values.0, values.1, values.2) },
            unsafe { r_f7(values.0, values.1, values.2) },
            "C59"
        );
    }

    // C60 nondegenerate and C61 degenerate geometry.
    for _ in 0..CASES * 8 {
        let p1 = LmVec2 {
            x: rng.finite(),
            y: rng.finite(),
        };
        let p2 = LmVec2 {
            x: p1.x + 2.0,
            y: p1.y,
        };
        let p3 = LmVec2 {
            x: p1.x,
            y: p1.y + 3.0,
        };
        let p = LmVec2 {
            x: rng.finite(),
            y: rng.finite(),
        };
        assert_lm(
            unsafe { c_f9(p1, p2, p3, p) },
            unsafe { r_f9(p1, p2, p3, p) },
            "C60",
        );
        let same = LmVec2 {
            x: rng.finite(),
            y: rng.finite(),
        };
        assert_lm(
            unsafe { c_f9(same, same, same, p) },
            unsafe { r_f9(same, same, same, p) },
            "C61",
        );
    }

    // C62-C66: exhaustive half-float input surface.
    let mut category_counts = [0_usize; 5];
    for bits in 0_u32..=u16::MAX as u32 {
        let h = bits as u16;
        let exponent = (h >> 10) & 0x1f;
        let mantissa = h & 0x03ff;
        let category = match (exponent, mantissa) {
            (0, 0) => 0,
            (0, _) => 1,
            (31, 0) => 3,
            (31, _) => 4,
            _ => 2,
        };
        category_counts[category] += 1;
        assert_f32(unsafe { c_f10(h) }, unsafe { r_f10(h) }, "C62-C66");
    }
    assert_eq!(category_counts, [2, 2046, 61_440, 2, 2046]);
}

type ColorFn = unsafe extern "C" fn(*mut f32, *const f32);

fn compare_color(c: ColorFn, rust: ColorFn, src: [f32; 3], row: &str) {
    let mut c_out = [f32::from_bits(0x7fc0_dead); 3];
    let mut r_out = c_out;
    unsafe {
        c(c_out.as_mut_ptr(), src.as_ptr());
        rust(r_out.as_mut_ptr(), src.as_ptr());
    }
    for i in 0..3 {
        assert_f32(c_out[i], r_out[i], &format!("{row}[{i}]"));
    }
}

#[test]
fn color_helpers_cover_c67_through_c87() {
    let libs = Libraries::load();
    let (c_f11, r_f11) = unsafe { libs.functions::<ColorFn>(b"f11\0") };
    let (c_f12, r_f12) = unsafe { libs.functions::<ColorFn>(b"f12\0") };
    let (c_f13, r_f13) = unsafe { libs.functions::<ColorFn>(b"f13\0") };
    let mut rng = Rng::new(0x6768_6970_7172_7374);

    // C67.
    for _ in 0..CASES {
        compare_color(c_f11, r_f11, [rng.finite(), 0.0, rng.finite()], "C67");
    }
    // C68-C74. Each tuple is a distinct literal C branch.
    let hue_ranges = [
        (0.0, 59.99, "C68"),
        (60.0, 119.99, "C69"),
        (-720.0, -0.01, "C70"),
        (180.0, 239.99, "C71"),
        (240.0, 299.99, "C72"),
        (300.0, 359.99, "C73"),
        (120.0, 179.99, "C74"),
        (360.0, 1080.0, "C74"),
    ];
    for (low, high, row) in hue_ranges {
        for _ in 0..CASES {
            compare_color(
                c_f11,
                r_f11,
                [rng.between(low, high), rng.between(0.01, 2.0), rng.finite()],
                row,
            );
        }
    }
    for _ in 0..CASES {
        compare_color(
            c_f11,
            r_f11,
            [f32::from_bits(0x7fc0_1234), 1.0, rng.finite()],
            "C74 NaN",
        );
    }

    // C75.
    for _ in 0..CASES {
        compare_color(c_f12, r_f12, [rng.finite(), 0.0, rng.finite()], "C75");
    }
    // C76-C80 sectors 0..4.
    for sector in 0..5 {
        for _ in 0..CASES {
            compare_color(
                c_f12,
                r_f12,
                [
                    sector as f32 * 60.0 + rng.between(0.0, 59.99),
                    rng.between(0.01, 2.0),
                    rng.finite(),
                ],
                "C76-C80",
            );
        }
    }
    // C81 default, on both sides of the explicit cases.
    for _ in 0..CASES {
        for h in [rng.between(-600.0, -0.01), rng.between(300.0, 1200.0)] {
            compare_color(c_f12, r_f12, [h, 1.0, rng.finite()], "C81");
        }
    }

    // C82 equal nonzero; C83 max zero.
    for _ in 0..CASES {
        let equal = rng.between(0.01, 100.0);
        compare_color(c_f13, r_f13, [equal, equal, equal], "C82");
        compare_color(c_f13, r_f13, [0.0, -rng.unit(), -rng.unit()], "C83");
    }
    // C84-C87 max-channel and hue-correction branches.
    for _ in 0..CASES {
        let low = rng.between(0.0, 0.25);
        let mid = rng.between(0.3, 0.6);
        let high = rng.between(0.7, 1.0);
        compare_color(c_f13, r_f13, [high, mid, low], "C84");
        compare_color(c_f13, r_f13, [high, low, mid], "C85");
        compare_color(c_f13, r_f13, [low, high, mid], "C86");
        compare_color(c_f13, r_f13, [mid, low, high], "C87");
    }
}

type Agglom = unsafe extern "C" fn(
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

unsafe fn call_agglom(f: Agglom, a: AgglomArgs) -> f64 {
    // SAFETY: The function pointer and every argument match lib.h.
    unsafe {
        f(
            a.f2[0], a.f2[1], a.f2[2], a.f2[3], a.f2[4], a.f2[5], a.f2[6], a.f3[0], a.f3[1],
            a.f4[0], a.f4[1], a.f5, a.f7[0], a.f7[1], a.f7[2], a.f9[0], a.f9[1], a.f9[2], a.f9[3],
            a.f9[4], a.f9[5], a.f9[6], a.f9[7], a.f10, a.f11[0], a.f11[1], a.f11[2], a.f12[0],
            a.f12[1], a.f12[2], a.f13[0], a.f13[1], a.f13[2],
        )
    }
}

fn ordinary_agglom(rng: &mut Rng) -> AgglomArgs {
    AgglomArgs {
        f2: [
            rng.finite(),
            rng.finite(),
            rng.between(0.0, 100.0),
            -100.0,
            -100.0,
            100.0,
            100.0,
        ],
        f3: [rng.i32(), (rng.u32() % 100_000 + 1) as c_int],
        f4: [rng.u64(), rng.u64()],
        f5: rng.u32(),
        f7: [rng.u32() % 65_536, rng.u32() % 8, rng.u32() % 33],
        f9: [0.0, 0.0, 2.0, 0.0, 0.0, 3.0, rng.finite(), rng.finite()],
        f10: rng.u32() as u16,
        f11: [rng.between(-360.0, 720.0), rng.unit(), rng.unit()],
        f12: [rng.between(-360.0, 720.0), rng.unit(), rng.unit()],
        f13: [rng.unit(), rng.unit(), rng.unit()],
    }
}

#[test]
fn agglom_covers_c88_through_c90() {
    let libs = Libraries::load();
    let (c_agglom, r_agglom) = unsafe { libs.functions::<Agglom>(b"agglom\0") };
    let mut rng = Rng::new(0x8889_9091_9293_9495);
    let compare = |a, row| {
        assert_f64(
            unsafe { call_agglom(c_agglom, a) },
            unsafe { call_agglom(r_agglom, a) },
            row,
        );
    };

    // C88.
    for _ in 0..CASES * 4 {
        compare(ordinary_agglom(&mut rng), "C88");
    }
    // C89: degenerate f9 and half NaN are independently filtered.
    for _ in 0..CASES {
        let mut a = ordinary_agglom(&mut rng);
        a.f9 = [1.0; 8];
        a.f10 = 0x7e55;
        compare(a, "C89");
    }
    // C90: all integer boundaries plus exact hue branch boundaries.
    let ints = [c_int::MIN, -1, 0, 1, c_int::MAX];
    let uints = [0, 1, u32::MAX];
    let hues = [-0.0, 0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];
    for i in 0..CASES {
        let mut a = ordinary_agglom(&mut rng);
        a.f3 = [ints[i % ints.len()], ints[(i + 1) % ints.len()]];
        a.f5 = uints[i % uints.len()];
        a.f7 = [
            uints[i % uints.len()],
            uints[(i + 1) % uints.len()],
            uints[(i + 2) % uints.len()],
        ];
        a.f11[0] = hues[i % hues.len()];
        a.f12[0] = hues[(i + 1) % hues.len()];
        compare(a, "C90");
    }
}

#[test]
fn error_surface_covers_e1_through_e4() {
    let libs = Libraries::load();
    let (c_f2, r_f2) = unsafe { libs.functions::<F2>(b"f2\0") };
    let (c_f3, r_f3) = unsafe { libs.functions::<F3>(b"f3\0") };
    let mut rng = Rng::new(0xe1e2_e3e4_e5e6_e7e8);
    let null = std::ptr::null();
    let invalid = [-1, 2, 3, c_int::MIN, c_int::MAX];

    for i in 0..CASES {
        let bad = invalid[i % invalid.len()];
        let unused_type_b = rng.i32();
        // E1: invalid outer tag returns before either pointer is read.
        assert_eq!(
            unsafe { c_f2(null, bad, null, unused_type_b) },
            unsafe { r_f2(null, bad, null, unused_type_b) },
            "E1"
        );
        // E2-E3: valid outer tag, invalid inner tag, still no dereference.
        assert_eq!(
            unsafe { c_f2(null, 0, null, bad) },
            unsafe { r_f2(null, 0, null, bad) },
            "E2"
        );
        assert_eq!(
            unsafe { c_f2(null, 1, null, bad) },
            unsafe { r_f2(null, 1, null, bad) },
            "E3"
        );
        let numerator = rng.i32();
        assert_eq!(
            unsafe { c_f3(numerator, 0) },
            unsafe { r_f3(numerator, 0) },
            "E4"
        );
    }
}
