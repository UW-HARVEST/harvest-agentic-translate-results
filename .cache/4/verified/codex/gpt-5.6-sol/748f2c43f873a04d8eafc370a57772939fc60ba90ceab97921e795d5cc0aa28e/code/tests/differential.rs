use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};

const CIRCLE: c_int = 0;
const AABB: c_int = 1;
const SAMPLES: usize = 256;

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
type Vec2Fn = unsafe extern "C" fn(C2v, C2v) -> C2v;
type ClampFn = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
type DotFn = unsafe extern "C" fn(C2v, C2v) -> f32;
type CircleCircleFn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
type CircleAabbFn = unsafe extern "C" fn(C2Circle, C2Aabb) -> c_int;
type AabbAabbFn = unsafe extern "C" fn(C2Aabb, C2Aabb) -> c_int;
type CollidedFn = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;

#[derive(Clone, Copy)]
struct Symbols {
    v: VFn,
    maxv: Vec2Fn,
    minv: Vec2Fn,
    clampv: ClampFn,
    sub: Vec2Fn,
    dot: DotFn,
    circle_circle: CircleCircleFn,
    circle_aabb: CircleAabbFn,
    aabb_aabb: AabbAabbFn,
    collided: CollidedFn,
}

struct ApiPair {
    _c_library: Library,
    _rust_library: Library,
    c: Symbols,
    rust: Symbols,
}

impl Symbols {
    unsafe fn load(library: &Library) -> Self {
        Self {
            v: *unsafe { library.get(b"c2V\0") }.expect("load c2V"),
            maxv: *unsafe { library.get(b"c2Maxv\0") }.expect("load c2Maxv"),
            minv: *unsafe { library.get(b"c2Minv\0") }.expect("load c2Minv"),
            clampv: *unsafe { library.get(b"c2Clampv\0") }.expect("load c2Clampv"),
            sub: *unsafe { library.get(b"c2Sub\0") }.expect("load c2Sub"),
            dot: *unsafe { library.get(b"c2Dot\0") }.expect("load c2Dot"),
            circle_circle: *unsafe { library.get(b"c2CircletoCircle\0") }
                .expect("load c2CircletoCircle"),
            circle_aabb: *unsafe { library.get(b"c2CircletoAABB\0") }.expect("load c2CircletoAABB"),
            aabb_aabb: *unsafe { library.get(b"c2AABBtoAABB\0") }.expect("load c2AABBtoAABB"),
            collided: *unsafe { library.get(b"collided\0") }.expect("load collided"),
        }
    }
}

impl ApiPair {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&root);
        assert!(
            c_path.is_file(),
            "C shared library missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library missing: {}",
            rust_path.display()
        );

        let c_library = unsafe { Library::new(&c_path) }.expect("open C shared library");
        let rust_library = unsafe { Library::new(&rust_path) }.expect("open Rust shared library");
        let c = unsafe { Symbols::load(&c_library) };
        let rust = unsafe { Symbols::load(&rust_library) };
        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c,
            rust,
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    let profile_dir = std::env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("profile directory")
        .to_path_buf();
    let profile_path = profile_dir.join("libcollided_lib.so");
    if profile_path.is_file() {
        profile_path
    } else {
        root.join("target/debug/deps/libcollided_lib.so")
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn raw_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    fn finite(&mut self) -> f32 {
        (self.next_u32() % 20_001) as f32 / 16.0 - 625.0
    }

    fn positive(&mut self) -> f32 {
        (self.next_u32() % 1000 + 1) as f32 / 16.0
    }

    fn vector_bits(&mut self) -> C2v {
        C2v {
            x: self.raw_f32(),
            y: self.raw_f32(),
        }
    }

    fn vector_finite(&mut self) -> C2v {
        C2v {
            x: self.finite(),
            y: self.finite(),
        }
    }

    fn circle_bits(&mut self) -> C2Circle {
        C2Circle {
            p: self.vector_bits(),
            r: self.raw_f32(),
        }
    }

    fn aabb_bits(&mut self) -> C2Aabb {
        C2Aabb {
            min: self.vector_bits(),
            max: self.vector_bits(),
        }
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

fn ordered_pair(rng: &mut Rng, true_branch: bool, greater: bool) -> (f32, f32) {
    let base = rng.finite();
    let delta = rng.positive();
    match (true_branch, greater) {
        (true, true) | (false, false) => (base + delta, base),
        (false, true) | (true, false) => (base - delta, base),
    }
}

fn clamp_coordinate(rng: &mut Rng, state: usize) -> (f32, f32, f32) {
    let base = rng.finite();
    let delta = rng.positive();
    match state {
        0 => (base, base - 2.0 * delta, base + 2.0 * delta),
        1 => (base, base + delta, base + 2.0 * delta),
        2 => (base + delta, base - 2.0 * delta, base),
        3 => (base + 2.0 * delta, base + delta, base),
        _ => unreachable!(),
    }
}

fn aabb_axis(rng: &mut Rng, mask: usize) -> ((f32, f32), (f32, f32)) {
    let base = rng.finite();
    let delta = rng.positive();
    match mask {
        0b00 => (
            (base, base + 3.0 * delta),
            (base + delta, base + 2.0 * delta),
        ),
        0b01 => (
            (base + 2.0 * delta, base + 3.0 * delta),
            (base, base + delta),
        ),
        0b10 => (
            (base, base + delta),
            (base + 2.0 * delta, base + 3.0 * delta),
        ),
        0b11 => ((base + 2.0 * delta, base), (base + 2.0 * delta, base)),
        _ => unreachable!(),
    }
}

#[test]
fn row_01_vector_constructor() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x01ac_e001);
    for sample in 0..(SAMPLES * 4) {
        let x = rng.raw_f32();
        let y = rng.raw_f32();
        let c = unsafe { (api.c.v)(x, y) };
        let rust = unsafe { (api.rust.v)(x, y) };
        assert_vec_eq(c, rust, &format!("row 1 sample {sample}"));
    }
}

#[test]
fn rows_02_05_max_branches() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x05ac_e002);
    let mut seen = 0_u8;
    for mask in 0..4 {
        for sample in 0..SAMPLES {
            let (ax, bx) = ordered_pair(&mut rng, mask & 1 != 0, true);
            let (ay, by) = ordered_pair(&mut rng, mask & 2 != 0, true);
            let a = C2v { x: ax, y: ay };
            let b = C2v { x: bx, y: by };
            let actual_mask = usize::from(a.x > b.x) | (usize::from(a.y > b.y) << 1);
            assert_eq!(actual_mask, mask);
            seen |= 1 << mask;
            let c = unsafe { (api.c.maxv)(a, b) };
            let rust = unsafe { (api.rust.maxv)(a, b) };
            assert_vec_eq(c, rust, &format!("rows 2-5 mask {mask} sample {sample}"));
        }
    }

    let edges = [0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN];
    for &x in &edges {
        for &y in &edges {
            let a = C2v { x, y };
            let b = C2v { x: y, y: x };
            assert_vec_eq(
                unsafe { (api.c.maxv)(a, b) },
                unsafe { (api.rust.maxv)(a, b) },
                "rows 2-5 IEEE edge",
            );
        }
    }
    assert_eq!(seen, 0b1111);
}

#[test]
fn rows_06_09_min_branches() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x09ac_e006);
    let mut seen = 0_u8;
    for mask in 0..4 {
        for sample in 0..SAMPLES {
            let (ax, bx) = ordered_pair(&mut rng, mask & 1 != 0, false);
            let (ay, by) = ordered_pair(&mut rng, mask & 2 != 0, false);
            let a = C2v { x: ax, y: ay };
            let b = C2v { x: bx, y: by };
            let actual_mask = usize::from(a.x < b.x) | (usize::from(a.y < b.y) << 1);
            assert_eq!(actual_mask, mask);
            seen |= 1 << mask;
            let c = unsafe { (api.c.minv)(a, b) };
            let rust = unsafe { (api.rust.minv)(a, b) };
            assert_vec_eq(c, rust, &format!("rows 6-9 mask {mask} sample {sample}"));
        }
    }

    let edges = [0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN];
    for &x in &edges {
        for &y in &edges {
            let a = C2v { x, y };
            let b = C2v { x: y, y: x };
            assert_vec_eq(
                unsafe { (api.c.minv)(a, b) },
                unsafe { (api.rust.minv)(a, b) },
                "rows 6-9 IEEE edge",
            );
        }
    }
    assert_eq!(seen, 0b1111);
}

#[test]
fn rows_10_25_clamp_cross_product() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x25ac_e010);
    let mut seen = 0_u32;
    for x_state in 0..4 {
        for y_state in 0..4 {
            let state_pair = x_state | (y_state << 2);
            for sample in 0..SAMPLES {
                let (ax, lox, hix) = clamp_coordinate(&mut rng, x_state);
                let (ay, loy, hiy) = clamp_coordinate(&mut rng, y_state);
                let a = C2v { x: ax, y: ay };
                let lo = C2v { x: lox, y: loy };
                let hi = C2v { x: hix, y: hiy };
                seen |= 1 << state_pair;
                let c = unsafe { (api.c.clampv)(a, lo, hi) };
                let rust = unsafe { (api.rust.clampv)(a, lo, hi) };
                assert_vec_eq(
                    c,
                    rust,
                    &format!("rows 10-25 x state {x_state}, y state {y_state}, sample {sample}"),
                );
            }
        }
    }
    assert_eq!(seen, 0xffff);
}

#[test]
fn row_26_subtract() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x26ac_e026);
    for sample in 0..(SAMPLES * 4) {
        let a = rng.vector_bits();
        let b = rng.vector_bits();
        let c = unsafe { (api.c.sub)(a, b) };
        let rust = unsafe { (api.rust.sub)(a, b) };
        assert_vec_eq(c, rust, &format!("row 26 sample {sample}"));
    }
}

#[test]
fn row_27_dot_product() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x27ac_e027);
    for sample in 0..(SAMPLES * 8) {
        let a = rng.vector_bits();
        let b = rng.vector_bits();
        let c = unsafe { (api.c.dot)(a, b) };
        let rust = unsafe { (api.rust.dot)(a, b) };
        assert_float_eq(c, rust, &format!("row 27 sample {sample}"));
    }
}

#[test]
fn rows_28_29_circle_circle_outcomes() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x29ac_e028);
    let mut outcomes = 0_u8;
    for expected in 0..=1 {
        for sample in 0..SAMPLES {
            let a = C2Circle {
                p: rng.vector_finite(),
                r: if expected == 1 { 200.0 } else { 0.0 },
            };
            let offset = C2v {
                x: rng.finite() / 10.0,
                y: rng.finite() / 10.0,
            };
            let b = C2Circle {
                p: C2v {
                    x: a.p.x + offset.x,
                    y: a.p.y + offset.y,
                },
                r: if expected == 1 { 200.0 } else { 0.0 },
            };
            let c = unsafe { (api.c.circle_circle)(a, b) };
            let rust = unsafe { (api.rust.circle_circle)(a, b) };
            assert_eq!(c, expected, "generator outcome sample {sample}");
            assert_eq!(c, rust, "rows 28-29 outcome {expected} sample {sample}");
            outcomes |= 1 << c;
        }
    }

    for sample in 0..SAMPLES {
        let a = rng.circle_bits();
        let b = rng.circle_bits();
        assert_eq!(
            unsafe { (api.c.circle_circle)(a, b) },
            unsafe { (api.rust.circle_circle)(a, b) },
            "rows 28-29 raw-bit sample {sample}"
        );
    }
    assert_eq!(outcomes, 0b11);
}

#[test]
fn rows_30_61_circle_aabb_clamp_and_outcomes() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x31ac_e030);
    let mut seen_hit = 0_u32;
    let mut seen_miss = 0_u32;
    for expected in 0..=1 {
        for x_state in 0..4 {
            for y_state in 0..4 {
                let pair = x_state | (y_state << 2);
                for sample in 0..SAMPLES {
                    let (px, minx, maxx) = clamp_coordinate(&mut rng, x_state);
                    let (py, miny, maxy) = clamp_coordinate(&mut rng, y_state);
                    let circle = C2Circle {
                        p: C2v { x: px, y: py },
                        r: if expected == 1 { 500.0 } else { 0.0 },
                    };
                    let aabb = C2Aabb {
                        min: C2v { x: minx, y: miny },
                        max: C2v { x: maxx, y: maxy },
                    };
                    let c = unsafe { (api.c.circle_aabb)(circle, aabb) };
                    let rust = unsafe { (api.rust.circle_aabb)(circle, aabb) };
                    assert_eq!(c, expected, "generator outcome pair {pair} sample {sample}");
                    assert_eq!(
                        c, rust,
                        "rows 30-61 outcome {expected}, pair {pair}, sample {sample}"
                    );
                    if expected == 1 {
                        seen_hit |= 1 << pair;
                    } else {
                        seen_miss |= 1 << pair;
                    }
                }
            }
        }
    }
    assert_eq!(seen_hit, 0xffff);
    assert_eq!(seen_miss, 0xffff);
}

#[test]
fn rows_62_77_aabb_direction_cross_product() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x32ac_e032);
    let mut seen = 0_u32;
    for mask in 0..16 {
        for sample in 0..SAMPLES {
            let ((aminx, amaxx), (bminx, bmaxx)) = aabb_axis(&mut rng, mask & 0b11);
            let ((aminy, amaxy), (bminy, bmaxy)) = aabb_axis(&mut rng, (mask >> 2) & 0b11);
            let a = C2Aabb {
                min: C2v { x: aminx, y: aminy },
                max: C2v { x: amaxx, y: amaxy },
            };
            let b = C2Aabb {
                min: C2v { x: bminx, y: bminy },
                max: C2v { x: bmaxx, y: bmaxy },
            };
            let actual = usize::from(b.max.x < a.min.x)
                | (usize::from(a.max.x < b.min.x) << 1)
                | (usize::from(b.max.y < a.min.y) << 2)
                | (usize::from(a.max.y < b.min.y) << 3);
            assert_eq!(actual, mask);
            seen |= 1 << actual;
            assert_eq!(
                unsafe { (api.c.aabb_aabb)(a, b) },
                unsafe { (api.rust.aabb_aabb)(a, b) },
                "rows 62-77 mask {mask} sample {sample}"
            );
        }
    }
    assert_eq!(seen, 0xffff);
}

#[test]
fn rows_78_81_collided_dispatch() {
    let api = ApiPair::load();
    let mut rng = Rng::new(0x36ac_e033);
    let mut seen = 0_u8;
    for sample in 0..(SAMPLES * 2) {
        let circle_a = rng.circle_bits();
        let circle_b = rng.circle_bits();
        let aabb_a = rng.aabb_bits();
        let aabb_b = rng.aabb_bits();
        let cases = [
            (
                (&circle_a as *const C2Circle).cast(),
                CIRCLE,
                (&circle_b as *const C2Circle).cast(),
                CIRCLE,
            ),
            (
                (&circle_a as *const C2Circle).cast(),
                CIRCLE,
                (&aabb_b as *const C2Aabb).cast(),
                AABB,
            ),
            (
                (&aabb_a as *const C2Aabb).cast(),
                AABB,
                (&circle_b as *const C2Circle).cast(),
                CIRCLE,
            ),
            (
                (&aabb_a as *const C2Aabb).cast(),
                AABB,
                (&aabb_b as *const C2Aabb).cast(),
                AABB,
            ),
        ];
        for (case, &(a, type_a, b, type_b)) in cases.iter().enumerate() {
            seen |= 1 << case;
            assert_eq!(
                unsafe { (api.c.collided)(a, type_a, b, type_b) },
                unsafe { (api.rust.collided)(a, type_a, b, type_b) },
                "rows 78-81 case {case} sample {sample}"
            );
        }
    }
    assert_eq!(seen, 0b1111);
}

#[test]
fn error_rows_01_03_invalid_enums_and_null_boundaries() {
    let api = ApiPair::load();
    let null = std::ptr::null();
    let mut rng = Rng::new(0xe003_e001);
    for row in 1..=3 {
        for sample in 0..(SAMPLES * 2) {
            let mut invalid = rng.next_u32() as c_int;
            if invalid == CIRCLE || invalid == AABB {
                invalid = 2;
            }
            let (type_a, type_b) = match row {
                1 => (invalid, invalid),
                2 => (CIRCLE, invalid),
                3 => (AABB, invalid),
                _ => unreachable!(),
            };
            let c = unsafe { (api.c.collided)(null, type_a, null, type_b) };
            let rust = unsafe { (api.rust.collided)(null, type_a, null, type_b) };
            assert_eq!(c, 0, "C rejection row {row} sample {sample}");
            assert_eq!(c, rust, "error row {row} sample {sample}");
        }
    }

    for &invalid in &[-1, 2] {
        for &(type_a, type_b) in &[(invalid, 0), (CIRCLE, invalid), (AABB, invalid)] {
            assert_eq!(
                unsafe { (api.c.collided)(null, type_a, null, type_b) },
                unsafe { (api.rust.collided)(null, type_a, null, type_b) },
                "one-step enum boundary ({type_a}, {type_b})"
            );
        }
    }
}
