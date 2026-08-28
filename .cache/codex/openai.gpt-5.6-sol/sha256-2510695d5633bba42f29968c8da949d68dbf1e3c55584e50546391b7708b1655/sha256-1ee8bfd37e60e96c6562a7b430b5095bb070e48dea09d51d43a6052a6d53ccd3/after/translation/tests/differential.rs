#![allow(non_snake_case)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

type VFn = unsafe extern "C" fn(f32, f32) -> C2v;
type MulvsFn = unsafe extern "C" fn(C2v, f32) -> C2v;
type BinaryVFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type ClampFn = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type DotFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type CircleCircleFn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type CircleAabbFn = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type CircleCapsuleFn = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
type CollidedFn = unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int;
type CircleCollideFn = unsafe extern "C" fn(f32, f32, f32) -> c_int;

struct Api {
    _library: Library,
    c2V: VFn,
    c2Mulvs: MulvsFn,
    c2Maxv: BinaryVFn,
    c2Minv: BinaryVFn,
    c2Clampv: ClampFn,
    c2Sub: BinaryVFn,
    c2Dot: DotFn,
    c2CircletoCircle: CircleCircleFn,
    c2CircletoAABB: CircleAabbFn,
    c2CircletoCapsule: CircleCapsuleFn,
    c2Collided: CollidedFn,
    circle_collide: CircleCollideFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: The test keeps the library alive for at least as long as all
        // copied function pointers and declares each symbol with its C ABI.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                // SAFETY: Symbol names and signatures come directly from lib.c.
                unsafe { *library.get::<$ty>(concat!($name, "\0").as_bytes()).unwrap() }
            }};
        }
        Self {
            c2V: symbol!("c2V", VFn),
            c2Mulvs: symbol!("c2Mulvs", MulvsFn),
            c2Maxv: symbol!("c2Maxv", BinaryVFn),
            c2Minv: symbol!("c2Minv", BinaryVFn),
            c2Clampv: symbol!("c2Clampv", ClampFn),
            c2Sub: symbol!("c2Sub", BinaryVFn),
            c2Dot: symbol!("c2Dot", DotFn),
            c2CircletoCircle: symbol!("c2CircletoCircle", CircleCircleFn),
            c2CircletoAABB: symbol!("c2CircletoAABB", CircleAabbFn),
            c2CircletoCapsule: symbol!("c2CircletoCapsule", CircleCapsuleFn),
            c2Collided: symbol!("c2Collided", CollidedFn),
            circle_collide: symbol!("circle_collide", CircleCollideFn),
            _library: library,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_build = manifest.join("../c_src/build");
    let mut c_libraries: Vec<_> = fs::read_dir(&c_build)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", c_build.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    c_libraries.sort();
    let c_library = c_libraries
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no C shared library in {}", c_build.display()));
    let rust_library = manifest.join("target/release/libcircle_collide_lib.so");
    assert!(
        rust_library.is_file(),
        "build the Rust cdylib first: cargo build --release"
    );
    (c_library, rust_library)
}

fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    // SAFETY: Api::load validates that every required symbol can be resolved.
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn integer(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next_u32() % (high - low) as u32) as i32
    }

    fn moderate(&mut self) -> f32 {
        self.integer(-10_000, 10_001) as f32 / 16.0
    }

    fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

fn assert_float_eq(c: f32, rust: f32, context: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{context}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_vec_eq(c: C2v, rust: C2v, context: &str) {
    assert_float_eq(c.x, rust.x, &format!("{context}.x"));
    assert_float_eq(c.y, rust.y, &format!("{context}.y"));
}

fn compare_v(c: &Api, rust: &Api, x: f32, y: f32) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe { assert_vec_eq((c.c2V)(x, y), (rust.c2V)(x, y), "c2V") }
}

fn compare_mul(c: &Api, rust: &Api, value: C2v, scalar: f32) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe {
        assert_vec_eq(
            (c.c2Mulvs)(value, scalar),
            (rust.c2Mulvs)(value, scalar),
            "c2Mulvs",
        )
    }
}

fn compare_binary(c_fn: BinaryVFn, rust_fn: BinaryVFn, a: C2v, b: C2v, context: &str) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe { assert_vec_eq(c_fn(a, b), rust_fn(a, b), context) }
}

fn compare_clamp(c: &Api, rust: &Api, value: C2v, low: C2v, high: C2v) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe {
        assert_vec_eq(
            (c.c2Clampv)(value, low, high),
            (rust.c2Clampv)(value, low, high),
            "c2Clampv",
        )
    }
}

fn compare_dot(c: &Api, rust: &Api, a: C2v, b: C2v) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe { assert_float_eq((c.c2Dot)(a, b), (rust.c2Dot)(a, b), "c2Dot") }
}

fn compare_circle_circle(c: &Api, rust: &Api, a: C2Circle, b: C2Circle) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe {
        assert_eq!(
            (c.c2CircletoCircle)(a, b),
            (rust.c2CircletoCircle)(a, b),
            "c2CircletoCircle: A={a:?}, B={b:?}"
        )
    }
}

fn compare_circle_aabb(c: &Api, rust: &Api, a: C2Circle, b: C2Aabb) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe {
        assert_eq!(
            (c.c2CircletoAABB)(a, b),
            (rust.c2CircletoAABB)(a, b),
            "c2CircletoAABB: A={a:?}, B={b:?}"
        )
    }
}

fn compare_circle_capsule(c: &Api, rust: &Api, a: C2Circle, b: C2Capsule) {
    // SAFETY: Both function pointers have the source C signature.
    unsafe {
        assert_eq!(
            (c.c2CircletoCapsule)(a, b),
            (rust.c2CircletoCapsule)(a, b),
            "c2CircletoCapsule: A={a:?}, B={b:?}"
        )
    }
}

#[test]
fn vector_construction_multiply_subtract_and_dot_rows_1_2_21_22() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x8128_8f3d_2a6c_19e7);
    for _ in 0..4096 {
        let a = C2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let b = C2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let scalar = rng.any_f32();
        compare_v(&c, &rust, a.x, a.y);
        compare_mul(&c, &rust, a, scalar);
        compare_binary(c.c2Sub, rust.c2Sub, a, b, "c2Sub");
        compare_dot(&c, &rust, a, b);
    }

    let special = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_5678),
    ];
    for &x in &special {
        for &y in &special {
            let a = C2v { x, y };
            let b = C2v { x: y, y: x };
            compare_v(&c, &rust, x, y);
            compare_mul(&c, &rust, a, y);
            compare_binary(c.c2Sub, rust.c2Sub, a, b, "c2Sub special");
            compare_dot(&c, &rust, a, b);
        }
    }
}

#[test]
fn max_and_min_comparison_cross_products_rows_3_to_10() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xa1e4_5d79_002b_c831);
    for x_true in [false, true] {
        for y_true in [false, true] {
            for _ in 0..512 {
                let b = C2v {
                    x: rng.moderate(),
                    y: rng.moderate(),
                };
                let dx = rng.integer(1, 100) as f32;
                let dy = rng.integer(1, 100) as f32;
                let max_a = C2v {
                    x: b.x + if x_true { dx } else { -dx },
                    y: b.y + if y_true { dy } else { -dy },
                };
                compare_binary(c.c2Maxv, rust.c2Maxv, max_a, b, "c2Maxv");

                let min_a = C2v {
                    x: b.x + if x_true { -dx } else { dx },
                    y: b.y + if y_true { -dy } else { dy },
                };
                compare_binary(c.c2Minv, rust.c2Minv, min_a, b, "c2Minv");
            }
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
    for &ax in &special {
        for &ay in &special {
            for &bx in &special {
                let a = C2v { x: ax, y: ay };
                let b = C2v { x: bx, y: -0.0 };
                compare_binary(c.c2Maxv, rust.c2Maxv, a, b, "c2Maxv special");
                compare_binary(c.c2Minv, rust.c2Minv, a, b, "c2Minv special");
            }
        }
    }
}

fn position(mode: usize, low: f32, high: f32, delta: f32, use_boundary: bool) -> f32 {
    match mode {
        0 => low - delta,
        1 => (low + high) * 0.5,
        2 if use_boundary => high,
        2 => high + delta,
        _ => unreachable!(),
    }
}

#[test]
fn clamp_position_cross_product_rows_11_to_20() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x7bd3_ba11_d517_42c9);
    for x_mode in 0..3 {
        for y_mode in 0..3 {
            for iteration in 0..512 {
                let low = C2v {
                    x: rng.integer(-500, 0) as f32,
                    y: rng.integer(-500, 0) as f32,
                };
                let high = C2v {
                    x: low.x + rng.integer(1, 200) as f32,
                    y: low.y + rng.integer(1, 200) as f32,
                };
                let value = C2v {
                    x: position(
                        x_mode,
                        low.x,
                        high.x,
                        rng.integer(1, 100) as f32,
                        iteration % 2 == 0,
                    ),
                    y: position(
                        y_mode,
                        low.y,
                        high.y,
                        rng.integer(1, 100) as f32,
                        iteration % 3 == 0,
                    ),
                };
                compare_clamp(&c, &rust, value, low, high);
            }
        }
    }

    for _ in 0..4096 {
        compare_clamp(
            &c,
            &rust,
            C2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            C2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
            C2v {
                x: rng.any_f32(),
                y: rng.any_f32(),
            },
        );
    }
}

#[test]
fn circle_to_circle_contact_and_nonfinite_rows_23_to_25_69() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x90ff_731c_6dd8_e205);
    for relation in -1..=1 {
        for _ in 0..1024 {
            let center = C2v {
                x: rng.integer(-100, 101) as f32,
                y: rng.integer(-100, 101) as f32,
            };
            let radius_a = rng.integer(1, 30) as f32;
            let radius_b = rng.integer(1, 30) as f32;
            let distance = radius_a + radius_b + relation as f32;
            let a = C2Circle {
                p: center,
                r: radius_a,
            };
            let b = C2Circle {
                p: C2v {
                    x: center.x + distance,
                    y: center.y,
                },
                r: radius_b,
            };
            compare_circle_circle(&c, &rust, a, b);
        }
    }

    let special = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0123),
    ];
    for &value in &special {
        compare_circle_circle(
            &c,
            &rust,
            C2Circle {
                p: C2v { x: value, y: 1.0 },
                r: value,
            },
            C2Circle {
                p: C2v {
                    x: -value,
                    y: value,
                },
                r: 2.0,
            },
        );
    }
}

fn aabb_coordinate(mode: usize, low: f32, high: f32, low_offset: f32, high_offset: f32) -> f32 {
    match mode {
        0 => low - low_offset,
        1 => (low + high) * 0.5,
        2 => high + high_offset,
        _ => unreachable!(),
    }
}

#[test]
fn circle_to_aabb_position_contact_cross_product_rows_26_to_52_70() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x41e6_a2cb_d908_573f);
    for x_mode in 0..3 {
        for y_mode in 0..3 {
            for relation in -1..=1 {
                for _ in 0..256 {
                    let scale = rng.integer(1, 5) as f32;
                    let low = C2v {
                        x: rng.integer(-100, 101) as f32,
                        y: rng.integer(-100, 101) as f32,
                    };
                    let high = C2v {
                        x: low.x + 20.0 * scale,
                        y: low.y + 20.0 * scale,
                    };
                    let x_offset = if y_mode == 1 { 5.0 } else { 3.0 } * scale;
                    let y_offset = if x_mode == 1 { 5.0 } else { 4.0 } * scale;
                    let point = C2v {
                        x: aabb_coordinate(x_mode, low.x, high.x, x_offset, x_offset),
                        y: aabb_coordinate(y_mode, low.y, high.y, y_offset, y_offset),
                    };
                    let distance = if x_mode == 1 && y_mode == 1 {
                        0.0
                    } else {
                        5.0 * scale
                    };
                    let radius = if distance == 0.0 && relation == 1 {
                        f32::NAN
                    } else {
                        distance - relation as f32 * scale
                    };
                    compare_circle_aabb(
                        &c,
                        &rust,
                        C2Circle {
                            p: point,
                            r: radius,
                        },
                        C2Aabb {
                            min: low,
                            max: high,
                        },
                    );
                }
            }
        }
    }

    for _ in 0..4096 {
        compare_circle_aabb(
            &c,
            &rust,
            C2Circle {
                p: C2v {
                    x: rng.any_f32(),
                    y: rng.any_f32(),
                },
                r: rng.any_f32(),
            },
            C2Aabb {
                min: C2v {
                    x: rng.any_f32(),
                    y: rng.any_f32(),
                },
                max: C2v {
                    x: rng.any_f32(),
                    y: rng.any_f32(),
                },
            },
        );
    }
}

fn capsule_case(
    rng: &mut Rng,
    region: usize,
    relation: i32,
    vertical: bool,
) -> (C2Circle, C2Capsule) {
    let scale = rng.integer(1, 5) as f32;
    let origin = C2v {
        x: rng.integer(-100, 101) as f32,
        y: rng.integer(-100, 101) as f32,
    };
    let along = match region {
        0 => -3.0 * scale,
        1 => 5.0 * scale,
        2 => 13.0 * scale,
        _ => unreachable!(),
    };
    let perpendicular = 4.0 * scale;
    let point = if vertical {
        C2v {
            x: origin.x + perpendicular,
            y: origin.y + along,
        }
    } else {
        C2v {
            x: origin.x + along,
            y: origin.y + perpendicular,
        }
    };
    let end = if vertical {
        C2v {
            x: origin.x,
            y: origin.y + 10.0 * scale,
        }
    } else {
        C2v {
            x: origin.x + 10.0 * scale,
            y: origin.y,
        }
    };
    let capsule_radius = 2.0 * scale;
    let total_radius = (5.0 - relation as f32) * scale;
    (
        C2Circle {
            p: point,
            r: total_radius - capsule_radius,
        },
        C2Capsule {
            a: origin,
            b: end,
            r: capsule_radius,
        },
    )
}

#[test]
fn circle_to_capsule_region_contact_cross_product_rows_53_to_64_71() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xf509_31a8_62cd_704e);
    for region in 0..3 {
        for relation in -1..=1 {
            for iteration in 0..512 {
                let (circle, capsule) =
                    capsule_case(&mut rng, region, relation, iteration % 2 == 0);
                compare_circle_capsule(&c, &rust, circle, capsule);
            }
        }
    }

    for relation in -1..=1 {
        for _ in 0..512 {
            let point = C2v {
                x: rng.integer(-100, 101) as f32,
                y: rng.integer(-100, 101) as f32,
            };
            let scale = rng.integer(1, 5) as f32;
            compare_circle_capsule(
                &c,
                &rust,
                C2Circle {
                    p: C2v {
                        x: point.x + 5.0 * scale,
                        y: point.y,
                    },
                    r: (3.0 - relation as f32) * scale,
                },
                C2Capsule {
                    a: point,
                    b: point,
                    r: 2.0 * scale,
                },
            );
        }
    }

    for _ in 0..4096 {
        compare_circle_capsule(
            &c,
            &rust,
            C2Circle {
                p: C2v {
                    x: rng.any_f32(),
                    y: rng.any_f32(),
                },
                r: rng.any_f32(),
            },
            C2Capsule {
                a: C2v {
                    x: rng.any_f32(),
                    y: rng.any_f32(),
                },
                b: C2v {
                    x: rng.any_f32(),
                    y: rng.any_f32(),
                },
                r: rng.any_f32(),
            },
        );
    }
}

#[test]
fn collided_valid_dispatch_rows_65_to_67() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x33ba_c824_704f_ed91);
    for _ in 0..4096 {
        let circle = C2Circle {
            p: C2v {
                x: rng.moderate(),
                y: rng.moderate(),
            },
            r: rng.moderate(),
        };
        let other_circle = C2Circle {
            p: C2v {
                x: rng.moderate(),
                y: rng.moderate(),
            },
            r: rng.moderate(),
        };
        let aabb = C2Aabb {
            min: C2v {
                x: rng.moderate(),
                y: rng.moderate(),
            },
            max: C2v {
                x: rng.moderate(),
                y: rng.moderate(),
            },
        };
        let capsule = C2Capsule {
            a: C2v {
                x: rng.moderate(),
                y: rng.moderate(),
            },
            b: C2v {
                x: rng.moderate(),
                y: rng.moderate(),
            },
            r: rng.moderate(),
        };
        let cases = [
            ((&other_circle as *const C2Circle).cast::<c_void>(), 0),
            ((&aabb as *const C2Aabb).cast::<c_void>(), 1),
            ((&capsule as *const C2Capsule).cast::<c_void>(), 2),
        ];
        for (other, kind) in cases {
            // SAFETY: Pointers match the selected C enum and remain alive.
            unsafe {
                assert_eq!(
                    (c.c2Collided)((&circle as *const C2Circle).cast::<c_void>(), other, kind,),
                    (rust.c2Collided)((&circle as *const C2Circle).cast::<c_void>(), other, kind,),
                    "c2Collided type {kind}"
                );
            }
        }
    }

    let special_circle = C2Circle {
        p: C2v {
            x: f32::NAN,
            y: f32::INFINITY,
        },
        r: -0.0,
    };
    let special_aabb = C2Aabb {
        min: C2v {
            x: f32::NEG_INFINITY,
            y: f32::NAN,
        },
        max: C2v {
            x: f32::INFINITY,
            y: -0.0,
        },
    };
    let special_capsule = C2Capsule {
        a: C2v { x: -0.0, y: 0.0 },
        b: C2v {
            x: f32::NAN,
            y: f32::INFINITY,
        },
        r: f32::NEG_INFINITY,
    };
    let cases = [
        ((&special_circle as *const C2Circle).cast::<c_void>(), 0),
        ((&special_aabb as *const C2Aabb).cast::<c_void>(), 1),
        ((&special_capsule as *const C2Capsule).cast::<c_void>(), 2),
    ];
    for (other, kind) in cases {
        // SAFETY: Pointers match the selected C enum and remain alive.
        unsafe {
            assert_eq!(
                (c.c2Collided)(
                    (&special_circle as *const C2Circle).cast::<c_void>(),
                    other,
                    kind,
                ),
                (rust.c2Collided)(
                    (&special_circle as *const C2Circle).cast::<c_void>(),
                    other,
                    kind,
                ),
                "c2Collided non-finite type {kind}"
            );
        }
    }
}

#[test]
fn circle_collide_full_pipeline_rows_68_72() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0xd924_08fe_651a_b37c);
    for _ in 0..20_000 {
        let x = rng.moderate();
        let y = rng.moderate();
        let r = rng.moderate();
        // SAFETY: Both function pointers have the source C signature.
        unsafe {
            assert_eq!(
                (c.circle_collide)(x, y, r),
                (rust.circle_collide)(x, y, r),
                "circle_collide({x:?}, {y:?}, {r:?})"
            );
        }
    }
    for value in [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
    ] {
        let inputs = [(value, 1.0, 2.0), (1.0, value, 2.0), (1.0, 2.0, value)];
        for (x, y, r) in inputs {
            // SAFETY: Both function pointers have the source C signature.
            unsafe {
                assert_eq!(
                    (c.circle_collide)(x, y, r),
                    (rust.circle_collide)(x, y, r),
                    "circle_collide non-finite input"
                );
            }
        }
    }
}

#[test]
fn invalid_dispatch_enum_error_row_1() {
    let (c, rust) = apis();
    let mut rng = Rng::new(0x5151_aee3_9974_0c62);
    let pointers = [
        (std::ptr::null(), std::ptr::null()),
        (1usize as *const c_void, std::ptr::null()),
        (std::ptr::null(), 1usize as *const c_void),
        (1usize as *const c_void, 1usize as *const c_void),
    ];
    for kind in [-1, 3, c_int::MIN, c_int::MAX] {
        for &(a, b) in &pointers {
            // SAFETY: The C default branch does not inspect pointers for an
            // invalid enum, which is the behavior under test.
            unsafe {
                let c_result = (c.c2Collided)(a, b, kind);
                let rust_result = (rust.c2Collided)(a, b, kind);
                assert_eq!(c_result, 0, "C invalid enum {kind}");
                assert_eq!(rust_result, c_result, "Rust invalid enum {kind}");
            }
        }
    }
    for _ in 0..4096 {
        let mut kind = rng.next_u32() as c_int;
        if (0..=2).contains(&kind) {
            kind = 3;
        }
        // SAFETY: Invalid enum values take the non-dereferencing default arm.
        unsafe {
            assert_eq!(
                (c.c2Collided)(std::ptr::null(), std::ptr::null(), kind),
                (rust.c2Collided)(std::ptr::null(), std::ptr::null(), kind),
                "random invalid enum {kind}"
            );
        }
    }
}

#[test]
fn null_pointer_probe_helper() {
    let Ok(specification) = std::env::var("COLLISION_NULL_PROBE") else {
        return;
    };
    let (implementation, kind) = specification
        .split_once(':')
        .expect("probe specification must be implementation:kind");
    let kind: c_int = kind.parse().expect("probe kind must be an integer");
    let (c_path, rust_path) = library_paths();
    let path = match implementation {
        "c" => c_path,
        "rust" => rust_path,
        _ => panic!("unknown probe implementation {implementation}"),
    };
    // SAFETY: This deliberately reproduces the C API's undefined null-pointer
    // dereference in an isolated process so it cannot terminate the test run.
    unsafe {
        let api = Api::load(&path);
        let _ = (api.c2Collided)(std::ptr::null(), std::ptr::null(), kind);
    }
    panic!("valid-enum null pointer unexpectedly returned");
}

#[cfg(unix)]
#[test]
fn valid_dispatch_null_pointers_have_matching_observed_termination() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().expect("current test executable");
    for kind in 0..=2 {
        let run = |implementation: &str| {
            Command::new(&executable)
                .args(["--exact", "null_pointer_probe_helper", "--nocapture"])
                .env("COLLISION_NULL_PROBE", format!("{implementation}:{kind}"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("run isolated null-pointer probe")
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(!c_status.success(), "C null probe type {kind} returned");
        assert!(
            !rust_status.success(),
            "Rust null probe type {kind} returned"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "null probe type {kind}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}
