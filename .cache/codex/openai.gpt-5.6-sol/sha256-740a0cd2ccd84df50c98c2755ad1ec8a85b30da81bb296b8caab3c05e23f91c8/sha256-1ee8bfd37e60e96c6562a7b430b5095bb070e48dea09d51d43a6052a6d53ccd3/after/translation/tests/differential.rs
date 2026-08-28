use libloading::Library;
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

type VFn = unsafe extern "C" fn(f32, f32) -> C2v;
type VVFn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type VVVFn = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type DotFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type CircleCircleFn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type CircleAabbFn = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type AabbAabbFn = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type CollidedFn = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;

struct Api {
    c2_v: VFn,
    c2_maxv: VVFn,
    c2_minv: VVFn,
    c2_clampv: VVVFn,
    c2_sub: VVFn,
    c2_dot: DotFn,
    circle_circle: CircleCircleFn,
    circle_aabb: CircleAabbFn,
    aabb_aabb: AabbAabbFn,
    collided: CollidedFn,
    _library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: Tests retain the library for at least as long as all copied
        // function pointers, and each signature mirrors the C ABI exactly.
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
            c2_v: load!("c2V", VFn),
            c2_maxv: load!("c2Maxv", VVFn),
            c2_minv: load!("c2Minv", VVFn),
            c2_clampv: load!("c2Clampv", VVVFn),
            c2_sub: load!("c2Sub", VVFn),
            c2_dot: load!("c2Dot", DotFn),
            circle_circle: load!("c2CircletoCircle", CircleCircleFn),
            circle_aabb: load!("c2CircletoAABB", CircleAabbFn),
            aabb_aabb: load!("c2AABBtoAABB", AabbAabbFn),
            collided: load!("collided", CollidedFn),
            _library: library,
        }
    }
}

fn libraries() -> (Api, Api) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("../c_src/build/libharvest-work-bfWKpd.so");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let rust_path = manifest.join(format!("target/{profile}/libcollided_lib.so"));
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    // SAFETY: Paths are fixed build outputs and Api validates every symbol.
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn assert_v(label: &str, c: C2v, rust: C2v) {
    assert_eq!(
        c.x.to_bits(),
        rust.x.to_bits(),
        "{label}: x: {c:?} != {rust:?}"
    );
    assert_eq!(
        c.y.to_bits(),
        rust.y.to_bits(),
        "{label}: y: {c:?} != {rust:?}"
    );
}

fn assert_f(label: &str, c: f32, rust: f32) {
    assert_eq!(c.to_bits(), rust.to_bits(), "{label}: {c:?} != {rust:?}");
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x6a09_e667_f3bc_c909)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    fn finite(&mut self) -> f32 {
        (self.next_u32() % 2001) as f32 - 1000.0
    }

    fn positive(&mut self) -> f32 {
        (self.next_u32() % 50 + 1) as f32
    }

    fn vector(&mut self) -> C2v {
        C2v {
            x: self.raw_f32(),
            y: self.raw_f32(),
        }
    }

    fn finite_vector(&mut self) -> C2v {
        C2v {
            x: self.finite(),
            y: self.finite(),
        }
    }

    fn circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.finite_vector(),
            r: self.finite(),
        }
    }

    fn raw_circle(&mut self) -> C2Circle {
        C2Circle {
            p: self.vector(),
            r: self.raw_f32(),
        }
    }

    fn aabb(&mut self) -> C2Aabb {
        C2Aabb {
            min: self.finite_vector(),
            max: self.finite_vector(),
        }
    }

    fn raw_aabb(&mut self) -> C2Aabb {
        C2Aabb {
            min: self.vector(),
            max: self.vector(),
        }
    }
}

#[derive(Clone, Copy)]
enum ClampState {
    Below,
    Inside,
    Inverted,
    Above,
}

fn clamp_axis(state: ClampState, rng: &mut Rng) -> (f32, f32, f32) {
    let base = rng.finite() / 4.0;
    let delta = rng.positive();
    match state {
        ClampState::Below => (base - delta, base, base + 2.0 * delta),
        ClampState::Inside => (base, base - delta, base + delta),
        ClampState::Inverted => (base, base + delta, base - delta),
        ClampState::Above => (base + 2.0 * delta, base - delta, base + delta),
    }
}

#[test]
fn vector_helpers_cover_config_rows_1_through_27() {
    let (c, rust) = libraries();
    let mut rng = Rng::new();

    for iteration in 0..4096 {
        let x = rng.raw_f32();
        let y = rng.raw_f32();
        // SAFETY: Both loaded symbols have the declared value-only ABI.
        let (cv, rv) = unsafe { ((c.c2_v)(x, y), (rust.c2_v)(x, y)) };
        assert_v(&format!("c2V iteration {iteration}"), cv, rv);
    }

    for x_true in [false, true] {
        for y_true in [false, true] {
            for iteration in 0..256 {
                let bx = rng.finite() / 4.0;
                let by = rng.finite() / 4.0;
                let dx = rng.positive();
                let dy = rng.positive();
                let a = C2v {
                    x: if x_true { bx + dx } else { bx - dx },
                    y: if y_true { by + dy } else { by - dy },
                };
                let b = C2v { x: bx, y: by };
                // SAFETY: Value-only ABI.
                let (cm, rm, cn, rn) = unsafe {
                    (
                        (c.c2_maxv)(a, b),
                        (rust.c2_maxv)(a, b),
                        (c.c2_minv)(a, b),
                        (rust.c2_minv)(a, b),
                    )
                };
                let label = format!("max/min branches {x_true}/{y_true}, {iteration}");
                assert_v(&label, cm, rm);
                assert_v(&label, cn, rn);
            }
        }
    }

    let special = [
        f32::NAN,
        f32::from_bits(0x7fa1_2345),
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
    ];
    for (index, &value) in special.iter().enumerate() {
        let a = C2v { x: value, y: 1.0 };
        let b = C2v { x: 2.0, y: value };
        // SAFETY: Value-only ABI.
        unsafe {
            assert_v(
                &format!("c2Maxv special {index}"),
                (c.c2_maxv)(a, b),
                (rust.c2_maxv)(a, b),
            );
            assert_v(
                &format!("c2Minv special {index}"),
                (c.c2_minv)(a, b),
                (rust.c2_minv)(a, b),
            );
        }
    }

    let states = [
        ClampState::Below,
        ClampState::Inside,
        ClampState::Inverted,
        ClampState::Above,
    ];
    for &xs in &states {
        for &ys in &states {
            for iteration in 0..256 {
                let (ax, lox, hix) = clamp_axis(xs, &mut rng);
                let (ay, loy, hiy) = clamp_axis(ys, &mut rng);
                let a = C2v { x: ax, y: ay };
                let lo = C2v { x: lox, y: loy };
                let hi = C2v { x: hix, y: hiy };
                // SAFETY: Value-only ABI.
                let (cv, rv) = unsafe { ((c.c2_clampv)(a, lo, hi), (rust.c2_clampv)(a, lo, hi)) };
                assert_v(&format!("clamp branch iteration {iteration}"), cv, rv);
            }
        }
    }

    for iteration in 0..8192 {
        let a = rng.vector();
        let b = rng.vector();
        // SAFETY: Value-only ABI.
        unsafe {
            assert_v(
                &format!("c2Sub iteration {iteration}"),
                (c.c2_sub)(a, b),
                (rust.c2_sub)(a, b),
            );
            assert_f(
                &format!("c2Dot iteration {iteration}"),
                (c.c2_dot)(a, b),
                (rust.c2_dot)(a, b),
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Region {
    Below,
    Inside,
    Above,
}

#[derive(Clone, Copy)]
enum DistanceRelation {
    Less,
    Equal,
    Greater,
}

fn circle_for_region(
    x_region: Region,
    y_region: Region,
    relation: DistanceRelation,
    rng: &mut Rng,
) -> (C2Circle, C2Aabb) {
    let scale = (rng.next_u32() % 10 + 1) as f32;
    let both_outside = x_region != Region::Inside && y_region != Region::Inside;
    let dx = if both_outside {
        3.0 * scale
    } else {
        4.0 * scale
    };
    let dy = if both_outside {
        4.0 * scale
    } else {
        4.0 * scale
    };
    let axis = |region: Region, distance: f32, inside: f32| match region {
        Region::Below => -distance,
        Region::Inside => inside,
        Region::Above => 100.0 + distance,
    };
    let p = C2v {
        x: axis(x_region, dx, 20.0 + scale),
        y: axis(y_region, dy, 70.0 - scale),
    };
    let distance = if both_outside {
        5.0 * scale
    } else {
        4.0 * scale
    };
    let r = match relation {
        DistanceRelation::Less => distance + scale,
        DistanceRelation::Equal => distance,
        DistanceRelation::Greater => distance - scale,
    };
    (
        C2Circle { p, r },
        C2Aabb {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 100.0, y: 100.0 },
        },
    )
}

#[test]
fn collision_helpers_cover_config_rows_28_through_64() {
    let (c, rust) = libraries();
    let mut rng = Rng::new();

    for relation in [
        DistanceRelation::Less,
        DistanceRelation::Equal,
        DistanceRelation::Greater,
    ] {
        for iteration in 0..512 {
            let distance = (rng.next_u32() % 100 + 2) as f32;
            let radius_sum = match relation {
                DistanceRelation::Less => distance + 1.0,
                DistanceRelation::Equal => distance,
                DistanceRelation::Greater => distance - 1.0,
            };
            let a = C2Circle {
                p: C2v { x: 0.0, y: 0.0 },
                r: 0.0,
            };
            let b = C2Circle {
                p: C2v {
                    x: distance,
                    y: 0.0,
                },
                r: radius_sum,
            };
            // SAFETY: Value-only ABI.
            let (cv, rv) = unsafe { ((c.circle_circle)(a, b), (rust.circle_circle)(a, b)) };
            assert_eq!(cv, rv, "circle/circle relation iteration {iteration}");
        }
    }
    for iteration in 0..4096 {
        let a = C2Circle {
            p: rng.vector(),
            r: rng.raw_f32(),
        };
        let b = C2Circle {
            p: rng.vector(),
            r: rng.raw_f32(),
        };
        // SAFETY: Value-only ABI.
        let (cv, rv) = unsafe { ((c.circle_circle)(a, b), (rust.circle_circle)(a, b)) };
        assert_eq!(cv, rv, "arbitrary circle/circle iteration {iteration}");
    }

    let regions = [Region::Below, Region::Inside, Region::Above];
    for &xr in &regions {
        for &yr in &regions {
            if xr == Region::Inside && yr == Region::Inside {
                for iteration in 0..256 {
                    let aabb = C2Aabb {
                        min: C2v { x: 0.0, y: 0.0 },
                        max: C2v { x: 100.0, y: 100.0 },
                    };
                    for radius in [0.0, rng.positive()] {
                        let circle = C2Circle {
                            p: C2v { x: 50.0, y: 50.0 },
                            r: radius,
                        };
                        // SAFETY: Value-only ABI.
                        let (cv, rv) = unsafe {
                            (
                                (c.circle_aabb)(circle, aabb),
                                (rust.circle_aabb)(circle, aabb),
                            )
                        };
                        assert_eq!(cv, rv, "inside circle/AABB iteration {iteration}");
                    }
                }
                continue;
            }
            for relation in [
                DistanceRelation::Less,
                DistanceRelation::Equal,
                DistanceRelation::Greater,
            ] {
                for iteration in 0..256 {
                    let (circle, aabb) = circle_for_region(xr, yr, relation, &mut rng);
                    // SAFETY: Value-only ABI.
                    let (cv, rv) = unsafe {
                        (
                            (c.circle_aabb)(circle, aabb),
                            (rust.circle_aabb)(circle, aabb),
                        )
                    };
                    assert_eq!(cv, rv, "circle/AABB region iteration {iteration}");
                }
            }
        }
    }
    for iteration in 0..4096 {
        let circle = rng.circle();
        let mut aabb = rng.aabb();
        if iteration % 2 == 0 {
            std::mem::swap(&mut aabb.min.x, &mut aabb.max.x);
        } else {
            std::mem::swap(&mut aabb.min.y, &mut aabb.max.y);
        }
        // SAFETY: Value-only ABI.
        let (cv, rv) = unsafe {
            (
                (c.circle_aabb)(circle, aabb),
                (rust.circle_aabb)(circle, aabb),
            )
        };
        assert_eq!(cv, rv, "inverted circle/AABB iteration {iteration}");
    }
    for iteration in 0..4096 {
        let circle = rng.raw_circle();
        let aabb = rng.raw_aabb();
        // SAFETY: Value-only ABI.
        let (cv, rv) = unsafe {
            (
                (c.circle_aabb)(circle, aabb),
                (rust.circle_aabb)(circle, aabb),
            )
        };
        assert_eq!(cv, rv, "raw circle/AABB iteration {iteration}");
    }

    let cases = [
        (
            C2Aabb {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 10.0, y: 10.0 },
            },
            C2Aabb {
                min: C2v { x: 5.0, y: 5.0 },
                max: C2v { x: 15.0, y: 15.0 },
            },
        ),
        (
            C2Aabb {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 10.0, y: 10.0 },
            },
            C2Aabb {
                min: C2v { x: 10.0, y: 10.0 },
                max: C2v { x: 20.0, y: 20.0 },
            },
        ),
        (
            C2Aabb {
                min: C2v { x: 10.0, y: 0.0 },
                max: C2v { x: 20.0, y: 10.0 },
            },
            C2Aabb {
                min: C2v { x: 0.0, y: 2.0 },
                max: C2v { x: 9.0, y: 8.0 },
            },
        ),
        (
            C2Aabb {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 9.0, y: 10.0 },
            },
            C2Aabb {
                min: C2v { x: 10.0, y: 2.0 },
                max: C2v { x: 20.0, y: 8.0 },
            },
        ),
        (
            C2Aabb {
                min: C2v { x: 0.0, y: 10.0 },
                max: C2v { x: 10.0, y: 20.0 },
            },
            C2Aabb {
                min: C2v { x: 2.0, y: 0.0 },
                max: C2v { x: 8.0, y: 9.0 },
            },
        ),
        (
            C2Aabb {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 10.0, y: 9.0 },
            },
            C2Aabb {
                min: C2v { x: 2.0, y: 10.0 },
                max: C2v { x: 8.0, y: 20.0 },
            },
        ),
        (
            C2Aabb {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 1.0, y: 1.0 },
            },
            C2Aabb {
                min: C2v { x: 2.0, y: 2.0 },
                max: C2v { x: 3.0, y: 3.0 },
            },
        ),
    ];
    for (case_index, &(a, b)) in cases.iter().enumerate() {
        for iteration in 0..256 {
            let shift = rng.finite();
            let shifted = |v: C2v| C2v {
                x: v.x + shift,
                y: v.y - shift,
            };
            let aa = C2Aabb {
                min: shifted(a.min),
                max: shifted(a.max),
            };
            let bb = C2Aabb {
                min: shifted(b.min),
                max: shifted(b.max),
            };
            // SAFETY: Value-only ABI.
            let (cv, rv) = unsafe { ((c.aabb_aabb)(aa, bb), (rust.aabb_aabb)(aa, bb)) };
            assert_eq!(cv, rv, "AABB case {case_index}, iteration {iteration}");
        }
    }
    for iteration in 0..4096 {
        let a = rng.raw_aabb();
        let b = rng.raw_aabb();
        // SAFETY: Value-only ABI.
        let (cv, rv) = unsafe { ((c.aabb_aabb)(a, b), (rust.aabb_aabb)(a, b)) };
        assert_eq!(cv, rv, "raw AABB iteration {iteration}");
    }
}

#[test]
fn dispatcher_covers_config_rows_65_through_68() {
    let (c, rust) = libraries();
    let mut rng = Rng::new();
    for iteration in 0..8192 {
        let (circle_a, circle_b, aabb_a, aabb_b) = if iteration % 2 == 0 {
            (rng.circle(), rng.circle(), rng.aabb(), rng.aabb())
        } else {
            (
                rng.raw_circle(),
                rng.raw_circle(),
                rng.raw_aabb(),
                rng.raw_aabb(),
            )
        };
        let cases = [
            (
                (&circle_a as *const C2Circle).cast::<c_void>(),
                0,
                (&circle_b as *const C2Circle).cast::<c_void>(),
                0,
            ),
            (
                (&circle_a as *const C2Circle).cast::<c_void>(),
                0,
                (&aabb_b as *const C2Aabb).cast::<c_void>(),
                1,
            ),
            (
                (&aabb_a as *const C2Aabb).cast::<c_void>(),
                1,
                (&circle_b as *const C2Circle).cast::<c_void>(),
                0,
            ),
            (
                (&aabb_a as *const C2Aabb).cast::<c_void>(),
                1,
                (&aabb_b as *const C2Aabb).cast::<c_void>(),
                1,
            ),
        ];
        for &(a, type_a, b, type_b) in &cases {
            // SAFETY: Pointers match their accompanying type discriminants.
            let (cv, rv) = unsafe {
                (
                    (c.collided)(a, type_a, b, type_b),
                    (rust.collided)(a, type_a, b, type_b),
                )
            };
            assert_eq!(cv, rv, "dispatcher iteration {iteration}");
        }
    }
}

#[test]
fn invalid_enums_cover_all_error_rows_and_null_boundary() {
    let (c, rust) = libraries();
    let invalid = [c_int::MIN, -65_537, -1, 2, 65_537, c_int::MAX];
    let null = std::ptr::null::<c_void>();

    for &type_a in &invalid {
        for type_b in [c_int::MIN, -1, 0, 1, 2, c_int::MAX] {
            // SAFETY: The invalid type_a branch returns before reading pointers.
            let (cv, rv) = unsafe {
                (
                    (c.collided)(null, type_a, null, type_b),
                    (rust.collided)(null, type_a, null, type_b),
                )
            };
            assert_eq!(cv, rv, "invalid typeA={type_a}, typeB={type_b}");
            assert_eq!(cv, 0);
        }
    }
    for type_a in [0, 1] {
        for &type_b in &invalid {
            // SAFETY: The invalid type_b branch returns before reading pointers.
            let (cv, rv) = unsafe {
                (
                    (c.collided)(null, type_a, null, type_b),
                    (rust.collided)(null, type_a, null, type_b),
                )
            };
            assert_eq!(cv, rv, "typeA={type_a}, invalid typeB={type_b}");
            assert_eq!(cv, 0);
        }
    }
}
