use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::ptr;

const CASES: usize = 256;

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
type DotFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type CircleCircleFn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type CircleAabbFn = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type AabbAabbFn = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type CollidedFn = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;

struct Api {
    _library: Library,
    v: VFn,
    maxv: VVFn,
    minv: VVFn,
    clampv: unsafe extern "C" fn(C2v, C2v, C2v) -> C2v,
    sub: VVFn,
    dot: DotFn,
    circle_circle: CircleCircleFn,
    circle_aabb: CircleAabbFn,
    aabb_aabb: AabbAabbFn,
    collided: CollidedFn,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        // SAFETY: The paths point to the two libraries built from this workspace.
        let library = unsafe { Library::new(&path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        // SAFETY: Each type exactly matches the corresponding C ABI declaration.
        let v = unsafe { *library.get::<VFn>(b"c2V\0").unwrap() };
        let maxv = unsafe { *library.get::<VVFn>(b"c2Maxv\0").unwrap() };
        let minv = unsafe { *library.get::<VVFn>(b"c2Minv\0").unwrap() };
        let clampv = unsafe {
            *library
                .get::<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v>(b"c2Clampv\0")
                .unwrap()
        };
        let sub = unsafe { *library.get::<VVFn>(b"c2Sub\0").unwrap() };
        let dot = unsafe { *library.get::<DotFn>(b"c2Dot\0").unwrap() };
        let circle_circle = unsafe {
            *library
                .get::<CircleCircleFn>(b"c2CircletoCircle\0")
                .unwrap()
        };
        let circle_aabb = unsafe { *library.get::<CircleAabbFn>(b"c2CircletoAABB\0").unwrap() };
        let aabb_aabb = unsafe { *library.get::<AabbAabbFn>(b"c2AABBtoAABB\0").unwrap() };
        let collided = unsafe { *library.get::<CollidedFn>(b"collided\0").unwrap() };
        Self {
            _library: library,
            v,
            maxv,
            minv,
            clampv,
            sub,
            dot,
            circle_circle,
            circle_aabb,
            aabb_aabb,
            collided,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-BZe1V7.so");
        let rust_path = root.join("target/release/libcollided_lib.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        // SAFETY: Api::load validates symbol presence and binds exact ABI types.
        unsafe {
            Self {
                c: Api::load(c_path),
                rust: Api::load(rust_path),
            }
        }
    }
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

    fn integer(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next_u32() % ((high - low + 1) as u32)) as i32
    }

    fn finite(&mut self) -> f32 {
        self.integer(-4000, 4000) as f32 * 0.25
    }

    fn positive(&mut self) -> f32 {
        self.integer(1, 50) as f32
    }

    fn nan(&mut self) -> f32 {
        let payload = (self.next_u32() & 0x003f_ffff).max(1);
        f32::from_bits(0x7fc0_0000 | payload)
    }
}

fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn circle(x: f32, y: f32, r: f32) -> C2Circle {
    C2Circle { p: v(x, y), r }
}

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> C2Aabb {
    C2Aabb {
        min: v(min_x, min_y),
        max: v(max_x, max_y),
    }
}

fn assert_f32(row: usize, c: f32, rust: f32) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "CONFIGS.md row {row}: C={c:?} ({:#010x}), Rust={rust:?} ({:#010x})",
        c.to_bits(),
        rust.to_bits()
    );
}

fn assert_v(row: usize, c: C2v, rust: C2v) {
    assert_f32(row, c.x, rust.x);
    assert_f32(row, c.y, rust.y);
}

fn assert_i(row: usize, c: c_int, rust: c_int) {
    assert_eq!(c, rust, "CONFIGS.md/ERRORS.md row {row}");
}

#[test]
fn vectors_cover_configs_1_through_27() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);

    for _ in 0..CASES {
        let x = rng.finite();
        let y = rng.finite();
        // SAFETY: Function pointers and by-value ABI types were validated at load.
        unsafe { assert_v(1, (pair.c.v)(x, y), (pair.rust.v)(x, y)) };
    }

    for i in 0..CASES {
        let x = match i % 4 {
            0 => f32::INFINITY,
            1 => f32::NEG_INFINITY,
            2 => rng.nan(),
            _ => f32::from_bits(rng.nan().to_bits() | 0x8000_0000),
        };
        let y = rng.nan();
        unsafe { assert_v(2, (pair.c.v)(x, y), (pair.rust.v)(x, y)) };
    }

    for row in 3..=6 {
        let x_true = row == 3 || row == 4;
        let y_true = row == 3 || row == 5;
        for _ in 0..CASES {
            let bx = rng.finite();
            let by = rng.finite();
            let dx = rng.positive();
            let dy = rng.positive();
            let a = v(
                if x_true { bx + dx } else { bx - dx },
                if y_true { by + dy } else { by - dy },
            );
            let b = v(bx, by);
            unsafe { assert_v(row, (pair.c.maxv)(a, b), (pair.rust.maxv)(a, b)) };
        }
    }

    for _ in 0..CASES {
        let equal = rng.finite();
        let b_nan = rng.nan();
        let a = v(equal, rng.nan());
        let b = v(equal, b_nan);
        unsafe { assert_v(7, (pair.c.maxv)(a, b), (pair.rust.maxv)(a, b)) };
    }

    for row in 8..=11 {
        let x_true = row == 8 || row == 9;
        let y_true = row == 8 || row == 10;
        for _ in 0..CASES {
            let bx = rng.finite();
            let by = rng.finite();
            let dx = rng.positive();
            let dy = rng.positive();
            let a = v(
                if x_true { bx - dx } else { bx + dx },
                if y_true { by - dy } else { by + dy },
            );
            let b = v(bx, by);
            unsafe { assert_v(row, (pair.c.minv)(a, b), (pair.rust.minv)(a, b)) };
        }
    }

    for _ in 0..CASES {
        let equal = rng.finite();
        let b_nan = rng.nan();
        let a = v(equal, rng.nan());
        let b = v(equal, b_nan);
        unsafe { assert_v(12, (pair.c.minv)(a, b), (pair.rust.minv)(a, b)) };
    }

    for x_state in 0..3 {
        for y_state in 0..3 {
            let row = 13 + x_state * 3 + y_state;
            for _ in 0..CASES {
                let lo = v(rng.finite(), rng.finite());
                let hi = v(lo.x + rng.positive(), lo.y + rng.positive());
                let select = |state: usize, lo: f32, hi: f32, delta: f32| match state {
                    0 => lo - delta,
                    1 => lo + (hi - lo) * 0.5,
                    _ => hi + delta,
                };
                let value = v(
                    select(x_state, lo.x, hi.x, rng.positive()),
                    select(y_state, lo.y, hi.y, rng.positive()),
                );
                unsafe {
                    assert_v(
                        row,
                        (pair.c.clampv)(value, lo, hi),
                        (pair.rust.clampv)(value, lo, hi),
                    )
                };
            }
        }
    }

    for i in 0..CASES {
        let lo = if i % 2 == 0 { -0.0 } else { rng.finite() };
        let hi = if i % 2 == 0 { 0.0 } else { lo + rng.positive() };
        let value = if i % 3 == 0 { lo } else { hi };
        unsafe {
            assert_v(
                22,
                (pair.c.clampv)(v(value, hi), v(lo, lo), v(hi, hi)),
                (pair.rust.clampv)(v(value, hi), v(lo, lo), v(hi, hi)),
            )
        };
    }

    for i in 0..CASES {
        let a = if i % 2 == 0 {
            v(rng.nan(), rng.finite())
        } else {
            v(rng.finite(), rng.nan())
        };
        let lo = v(10.0 + rng.positive(), rng.nan());
        let hi = v(-10.0 - rng.positive(), rng.finite());
        unsafe {
            assert_v(
                23,
                (pair.c.clampv)(a, lo, hi),
                (pair.rust.clampv)(a, lo, hi),
            )
        };
    }

    for _ in 0..CASES {
        let a = v(rng.finite(), rng.finite());
        let b = v(rng.finite(), rng.finite());
        unsafe { assert_v(24, (pair.c.sub)(a, b), (pair.rust.sub)(a, b)) };
    }
    for i in 0..CASES {
        let a = v(
            if i % 2 == 0 { f32::INFINITY } else { rng.nan() },
            f32::NEG_INFINITY,
        );
        let b = v(f32::INFINITY, rng.nan());
        unsafe { assert_v(25, (pair.c.sub)(a, b), (pair.rust.sub)(a, b)) };
    }

    for _ in 0..CASES {
        let a = v(rng.finite(), rng.finite());
        let b = v(rng.finite(), rng.finite());
        unsafe { assert_f32(26, (pair.c.dot)(a, b), (pair.rust.dot)(a, b)) };
    }
    let finite_boundaries = [
        (v(f32::MAX, f32::MAX), v(2.0, -2.0)),
        (v(f32::MIN_POSITIVE, f32::MIN_POSITIVE), v(0.25, -0.25)),
        (v(0.0, -0.0), v(f32::MAX, f32::MAX)),
        (v(1.0e20, 1.0e20), v(1.0e20, -1.0e20)),
    ];
    for (a, b) in finite_boundaries {
        unsafe { assert_f32(26, (pair.c.dot)(a, b), (pair.rust.dot)(a, b)) };
    }
    for i in 0..CASES {
        let a = v(
            if i % 2 == 0 { f32::INFINITY } else { rng.nan() },
            f32::NEG_INFINITY,
        );
        let b = v(if i % 3 == 0 { 0.0 } else { 1.0 }, rng.nan());
        unsafe { assert_f32(27, (pair.c.dot)(a, b), (pair.rust.dot)(a, b)) };
    }
}

#[test]
fn circle_circle_covers_configs_28_through_32() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);
    for row in 28..=30 {
        for _ in 0..CASES {
            let ax = rng.finite();
            let ay = rng.finite();
            let ra = rng.positive();
            let rb = rng.positive();
            let sum = ra + rb;
            let distance = match row {
                28 => sum - 0.5,
                29 => sum,
                _ => sum + 0.5,
            };
            let a = circle(ax, ay, ra);
            let b = circle(ax + distance, ay, rb);
            unsafe {
                assert_i(
                    row,
                    (pair.c.circle_circle)(a, b),
                    (pair.rust.circle_circle)(a, b),
                )
            };
        }
    }

    for i in 0..CASES {
        let ra = match i % 3 {
            0 => 0.0,
            1 => -rng.positive(),
            _ => rng.positive(),
        };
        let rb = -rng.positive();
        let a = circle(rng.finite(), rng.finite(), ra);
        let b = circle(rng.finite(), rng.finite(), rb);
        unsafe {
            assert_i(
                31,
                (pair.c.circle_circle)(a, b),
                (pair.rust.circle_circle)(a, b),
            )
        };
    }

    for i in 0..CASES {
        let a = circle(rng.nan(), rng.finite(), rng.positive());
        let b = circle(
            rng.finite(),
            if i % 2 == 0 { f32::INFINITY } else { rng.nan() },
            rng.nan(),
        );
        unsafe {
            assert_i(
                32,
                (pair.c.circle_circle)(a, b),
                (pair.rust.circle_circle)(a, b),
            )
        };
    }
}

fn circle_aabb_case(
    rng: &mut Rng,
    x_region: i32,
    y_region: i32,
    outcome: usize,
) -> (C2Circle, C2Aabb) {
    let min_x = rng.integer(-100, 100) as f32;
    let min_y = rng.integer(-100, 100) as f32;
    let width = rng.integer(2, 20) as f32;
    let height = rng.integer(2, 20) as f32;
    let max_x = min_x + width;
    let max_y = min_y + height;
    let scale = rng.integer(1, 8) as f32;
    let dx = if x_region == 0 { 0.0 } else { 3.0 * scale };
    let dy = if y_region == 0 { 0.0 } else { 4.0 * scale };
    let x = match x_region {
        -1 => min_x - dx,
        0 => min_x + width * 0.5,
        _ => max_x + dx,
    };
    let y = match y_region {
        -1 => min_y - dy,
        0 => min_y + height * 0.5,
        _ => max_y + dy,
    };
    let distance = if dx == 0.0 {
        dy
    } else if dy == 0.0 {
        dx
    } else {
        5.0 * scale
    };
    let radius = match outcome {
        0 => distance + 1.0,
        1 => distance,
        _ => distance - 1.0,
    };
    (circle(x, y, radius), aabb(min_x, min_y, max_x, max_y))
}

#[test]
fn circle_aabb_covers_configs_33_through_59() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
    let regions = [
        (-1, -1, 33usize),
        (-1, 0, 36),
        (-1, 1, 39),
        (0, -1, 42),
        (0, 1, 47),
        (1, -1, 50),
        (1, 0, 53),
        (1, 1, 56),
    ];
    for (x_region, y_region, first_row) in regions {
        for outcome in 0..3 {
            let row = first_row + outcome;
            for _ in 0..CASES {
                let (circle, box_) = circle_aabb_case(&mut rng, x_region, y_region, outcome);
                unsafe {
                    assert_i(
                        row,
                        (pair.c.circle_aabb)(circle, box_),
                        (pair.rust.circle_aabb)(circle, box_),
                    )
                };
            }
        }
    }

    for _ in 0..CASES {
        let min_x = rng.finite();
        let min_y = rng.finite();
        let box_ = aabb(min_x, min_y, min_x + rng.positive(), min_y + rng.positive());
        let inside = circle(
            (box_.min.x + box_.max.x) * 0.5,
            (box_.min.y + box_.max.y) * 0.5,
            rng.positive(),
        );
        unsafe {
            assert_i(
                45,
                (pair.c.circle_aabb)(inside, box_),
                (pair.rust.circle_aabb)(inside, box_),
            )
        };
        let zero = C2Circle { r: 0.0, ..inside };
        unsafe {
            assert_i(
                46,
                (pair.c.circle_aabb)(zero, box_),
                (pair.rust.circle_aabb)(zero, box_),
            )
        };
    }

    for i in 0..CASES {
        let box_ = if i % 2 == 0 {
            aabb(10.0, 20.0, -10.0, -20.0)
        } else {
            aabb(rng.nan(), -1.0, 1.0, rng.nan())
        };
        let value = circle(
            if i % 3 == 0 { rng.nan() } else { rng.finite() },
            if i % 3 == 1 {
                f32::INFINITY
            } else {
                rng.finite()
            },
            if i % 3 == 2 {
                rng.nan()
            } else {
                rng.positive()
            },
        );
        unsafe {
            assert_i(
                59,
                (pair.c.circle_aabb)(value, box_),
                (pair.rust.circle_aabb)(value, box_),
            )
        };
    }
}

fn interval_for_relation(
    rng: &mut Rng,
    min: f32,
    max: f32,
    relation: usize,
    positive_side: bool,
) -> (f32, f32) {
    let extra = rng.positive();
    match (relation, positive_side) {
        (0, false) => (min - extra - 1.0, min - 1.0),
        (0, true) => (max + 1.0, max + extra + 1.0),
        (1, false) => (min - extra, min),
        (1, true) => (max, max + extra),
        _ => {
            let inset = ((max - min) * 0.25).max(0.25);
            (min + inset, max - inset)
        }
    }
}

#[test]
fn aabb_aabb_covers_configs_60_through_70() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xa54f_f53a_5f1d_36f1);

    for x_relation in 0..3 {
        for y_relation in 0..3 {
            let row = 60 + x_relation * 3 + y_relation;
            for _ in 0..CASES {
                let min_x = rng.finite();
                let min_y = rng.finite();
                let a = aabb(min_x, min_y, min_x + 20.0, min_y + 20.0);
                let (b_min_x, b_max_x) =
                    interval_for_relation(&mut rng, a.min.x, a.max.x, x_relation, false);
                let (b_min_y, b_max_y) =
                    interval_for_relation(&mut rng, a.min.y, a.max.y, y_relation, false);
                let b = aabb(b_min_x, b_min_y, b_max_x, b_max_y);
                unsafe { assert_i(row, (pair.c.aabb_aabb)(a, b), (pair.rust.aabb_aabb)(a, b)) };
            }
        }
    }

    for x_relation in 0..3 {
        for y_relation in 0..3 {
            for reflection in 1..4 {
                for _ in 0..CASES {
                    let min_x = rng.finite();
                    let min_y = rng.finite();
                    let a = aabb(min_x, min_y, min_x + 20.0, min_y + 20.0);
                    let (b_min_x, b_max_x) = interval_for_relation(
                        &mut rng,
                        a.min.x,
                        a.max.x,
                        x_relation,
                        reflection & 1 != 0,
                    );
                    let (b_min_y, b_max_y) = interval_for_relation(
                        &mut rng,
                        a.min.y,
                        a.max.y,
                        y_relation,
                        reflection & 2 != 0,
                    );
                    let b = aabb(b_min_x, b_min_y, b_max_x, b_max_y);
                    unsafe { assert_i(69, (pair.c.aabb_aabb)(a, b), (pair.rust.aabb_aabb)(a, b)) };
                }
            }
        }
    }

    for i in 0..CASES {
        let a = if i % 2 == 0 {
            aabb(10.0, 10.0, -10.0, -10.0)
        } else {
            aabb(rng.nan(), -1.0, 1.0, rng.nan())
        };
        let b = if i % 3 == 0 {
            aabb(f32::NEG_INFINITY, 4.0, f32::INFINITY, -4.0)
        } else {
            aabb(rng.finite(), rng.nan(), rng.nan(), rng.finite())
        };
        unsafe { assert_i(70, (pair.c.aabb_aabb)(a, b), (pair.rust.aabb_aabb)(a, b)) };
    }
}

#[test]
fn collided_dispatch_covers_configs_71_through_74() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x510e_527f_ade6_82d1);
    for _ in 0..CASES {
        let ca = circle(rng.finite(), rng.finite(), rng.finite());
        let cb = circle(rng.finite(), rng.finite(), rng.finite());
        let aa = aabb(rng.finite(), rng.finite(), rng.finite(), rng.finite());
        let ab = aabb(rng.finite(), rng.finite(), rng.finite(), rng.finite());
        unsafe {
            assert_i(
                71,
                (pair.c.collided)(ptr::from_ref(&ca).cast(), 0, ptr::from_ref(&cb).cast(), 0),
                (pair.rust.collided)(ptr::from_ref(&ca).cast(), 0, ptr::from_ref(&cb).cast(), 0),
            );
            assert_i(
                72,
                (pair.c.collided)(ptr::from_ref(&ca).cast(), 0, ptr::from_ref(&ab).cast(), 1),
                (pair.rust.collided)(ptr::from_ref(&ca).cast(), 0, ptr::from_ref(&ab).cast(), 1),
            );
            assert_i(
                73,
                (pair.c.collided)(ptr::from_ref(&aa).cast(), 1, ptr::from_ref(&cb).cast(), 0),
                (pair.rust.collided)(ptr::from_ref(&aa).cast(), 1, ptr::from_ref(&cb).cast(), 0),
            );
            assert_i(
                74,
                (pair.c.collided)(ptr::from_ref(&aa).cast(), 1, ptr::from_ref(&ab).cast(), 1),
                (pair.rust.collided)(ptr::from_ref(&aa).cast(), 1, ptr::from_ref(&ab).cast(), 1),
            );
        }
    }
}

#[test]
fn rejected_enum_values_cover_all_error_rows() {
    let pair = Pair::load();
    let invalid = [-1, 2, 3, c_int::MIN, c_int::MAX];
    for type_b in invalid {
        unsafe {
            let c = (pair.c.collided)(ptr::null(), 0, ptr::null(), type_b);
            let rust = (pair.rust.collided)(ptr::null(), 0, ptr::null(), type_b);
            assert_eq!(c, 0, "ERRORS.md row 1 C sentinel");
            assert_i(1, c, rust);

            let c = (pair.c.collided)(ptr::null(), 1, ptr::null(), type_b);
            let rust = (pair.rust.collided)(ptr::null(), 1, ptr::null(), type_b);
            assert_eq!(c, 0, "ERRORS.md row 2 C sentinel");
            assert_i(2, c, rust);
        }
    }

    for type_a in invalid {
        for type_b in [c_int::MIN, -1, 0, 1, 2, c_int::MAX] {
            unsafe {
                let c = (pair.c.collided)(ptr::null(), type_a, ptr::null(), type_b);
                let rust = (pair.rust.collided)(ptr::null(), type_a, ptr::null(), type_b);
                assert_eq!(c, 0, "ERRORS.md row 3 C sentinel");
                assert_i(3, c, rust);
            }
        }
    }
}
