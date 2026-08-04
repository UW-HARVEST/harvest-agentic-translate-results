use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

fn c_lib_path() -> &'static str {
    "c_src/build/libtranslated_rust.so"
}

fn rust_lib_path() -> &'static str {
    // The cdylib output of this crate.
    "target/release/libcollided_lib.so"
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C lib");
        let r = Library::new(rust_lib_path()).expect("failed to load Rust lib");
        (c, r)
    }
}

// Helper: bit-equal float comparison.
fn bits_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

fn vecs_bits_eq(a: C2v, b: C2v) -> bool {
    bits_eq(a.x, b.x) && bits_eq(a.y, b.y)
}

// Test inputs cover a wide range of values.
fn sample_floats() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.5,
        -2.5,
        100.0,
        -100.0,
        0.1,
        -0.1,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-30,
        1e30,
        -1e30,
    ]
}

fn sample_vecs() -> Vec<C2v> {
    let mut v = vec![];
    for x in [0.0_f32, 1.0, -1.0, 2.5, -2.5, 5.0] {
        for y in [0.0_f32, 1.0, -1.0, 2.5, -2.5, 5.0] {
            v.push(C2v { x, y });
        }
    }
    v
}

#[test]
fn test_c2v() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c.get(b"c2V").unwrap();
        let rf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r.get(b"c2V").unwrap();
        for &x in &sample_floats() {
            for &y in &sample_floats() {
                let cv = cf(x, y);
                let rv = rf(x, y);
                assert!(
                    vecs_bits_eq(cv, rv),
                    "c2V({}, {}): C={:?} Rust={:?}",
                    x,
                    y,
                    cv,
                    rv
                );
            }
        }
    }
}

#[test]
fn test_c2_minmax() {
    let (c, r) = load_libs();
    unsafe {
        let c_max: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Maxv").unwrap();
        let r_max: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Maxv").unwrap();
        let c_min: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Minv").unwrap();
        let r_min: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Minv").unwrap();

        for &a in &sample_vecs() {
            for &b in &sample_vecs() {
                let cm = c_max(a, b);
                let rm = r_max(a, b);
                assert!(vecs_bits_eq(cm, rm), "Maxv mismatch a={:?} b={:?}", a, b);
                let cn = c_min(a, b);
                let rn = r_min(a, b);
                assert!(vecs_bits_eq(cn, rn), "Minv mismatch a={:?} b={:?}", a, b);
            }
        }
    }
}

#[test]
fn test_c2_clampv() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = c.get(b"c2Clampv").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = r.get(b"c2Clampv").unwrap();
        let v = sample_vecs();
        for &a in &v {
            for &lo in &v {
                for &hi in &v {
                    let cv = cf(a, lo, hi);
                    let rv = rf(a, lo, hi);
                    assert!(vecs_bits_eq(cv, rv), "Clampv mismatch");
                }
            }
        }
    }
}

#[test]
fn test_c2_sub() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Sub").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Sub").unwrap();
        for &a in &sample_vecs() {
            for &b in &sample_vecs() {
                let cv = cf(a, b);
                let rv = rf(a, b);
                assert!(vecs_bits_eq(cv, rv), "Sub mismatch a={:?} b={:?}", a, b);
            }
        }
    }
}

#[test]
fn test_c2_dot() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get(b"c2Dot").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get(b"c2Dot").unwrap();
        for &a in &sample_vecs() {
            for &b in &sample_vecs() {
                let cv = cf(a, b);
                let rv = rf(a, b);
                assert!(bits_eq(cv, rv), "Dot mismatch a={:?} b={:?} c={} r={}", a, b, cv, rv);
            }
        }
    }
}

fn sample_circles() -> Vec<C2Circle> {
    let mut out = vec![];
    for &v in &sample_vecs() {
        for &r in &[0.0_f32, 0.5, 1.0, 2.0, 3.0, 10.0] {
            out.push(C2Circle { p: v, r });
        }
    }
    out
}

fn sample_aabbs() -> Vec<C2AABB> {
    let mut out = vec![];
    let pts = sample_vecs();
    // make valid AABBs (min<=max) and a few intentionally-inverted ones too
    for &min in &pts {
        for &max in &pts {
            out.push(C2AABB { min, max });
        }
    }
    out
}

#[test]
fn test_c2_circle_to_circle() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            c.get(b"c2CircletoCircle").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            r.get(b"c2CircletoCircle").unwrap();
        let circles = sample_circles();
        for (i, &a) in circles.iter().enumerate() {
            for &b in &circles[i..i.saturating_add(20).min(circles.len())] {
                let cv = cf(a, b);
                let rv = rf(a, b);
                assert_eq!(cv, rv, "CircletoCircle mismatch a={:?} b={:?}", a, b);
            }
        }
    }
}

#[test]
fn test_c2_circle_to_aabb() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            c.get(b"c2CircletoAABB").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            r.get(b"c2CircletoAABB").unwrap();
        let circles = sample_circles();
        let aabbs = sample_aabbs();
        // Subsample to keep test reasonable.
        for (i, a) in circles.iter().enumerate().step_by(7) {
            for (j, b) in aabbs.iter().enumerate().step_by(13) {
                let cv = cf(*a, *b);
                let rv = rf(*a, *b);
                assert_eq!(cv, rv, "CircletoAABB mismatch i={} j={} a={:?} b={:?}", i, j, a, b);
            }
        }
    }
}

#[test]
fn test_c2_aabb_to_aabb() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            c.get(b"c2AABBtoAABB").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            r.get(b"c2AABBtoAABB").unwrap();
        let aabbs = sample_aabbs();
        for (i, a) in aabbs.iter().enumerate().step_by(11) {
            for (j, b) in aabbs.iter().enumerate().step_by(11) {
                let cv = cf(*a, *b);
                let rv = rf(*a, *b);
                assert_eq!(cv, rv, "AABBtoAABB mismatch i={} j={} a={:?} b={:?}", i, j, a, b);
            }
        }
    }
}

#[test]
fn test_collided() {
    let (c, r) = load_libs();
    unsafe {
        let cf: Symbol<
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
        > = c.get(b"collided").unwrap();
        let rf: Symbol<
            unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int,
        > = r.get(b"collided").unwrap();

        let circles = sample_circles();
        let aabbs = sample_aabbs();

        // Circle vs Circle
        for ca in circles.iter().step_by(7) {
            for cb in circles.iter().step_by(7) {
                let cv = cf(
                    ca as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                    cb as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                );
                let rv = rf(
                    ca as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                    cb as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                );
                assert_eq!(cv, rv, "collided CC mismatch");
            }
        }

        // Circle vs AABB
        for ca in circles.iter().step_by(7) {
            for ab in aabbs.iter().step_by(13) {
                let cv = cf(
                    ca as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                    ab as *const _ as *const c_void,
                    C2_TYPE_AABB,
                );
                let rv = rf(
                    ca as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                    ab as *const _ as *const c_void,
                    C2_TYPE_AABB,
                );
                assert_eq!(cv, rv, "collided CA mismatch");
            }
        }

        // AABB vs Circle
        for ab in aabbs.iter().step_by(13) {
            for ca in circles.iter().step_by(7) {
                let cv = cf(
                    ab as *const _ as *const c_void,
                    C2_TYPE_AABB,
                    ca as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                );
                let rv = rf(
                    ab as *const _ as *const c_void,
                    C2_TYPE_AABB,
                    ca as *const _ as *const c_void,
                    C2_TYPE_CIRCLE,
                );
                assert_eq!(cv, rv, "collided AC mismatch");
            }
        }

        // AABB vs AABB
        for a in aabbs.iter().step_by(11) {
            for b in aabbs.iter().step_by(11) {
                let cv = cf(
                    a as *const _ as *const c_void,
                    C2_TYPE_AABB,
                    b as *const _ as *const c_void,
                    C2_TYPE_AABB,
                );
                let rv = rf(
                    a as *const _ as *const c_void,
                    C2_TYPE_AABB,
                    b as *const _ as *const c_void,
                    C2_TYPE_AABB,
                );
                assert_eq!(cv, rv, "collided AA mismatch");
            }
        }

        // Unknown types should return 0.
        let circle = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let cv = cf(
            &circle as *const _ as *const c_void,
            999,
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
        );
        let rv = rf(
            &circle as *const _ as *const c_void,
            999,
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
        );
        assert_eq!(cv, rv);
        assert_eq!(cv, 0);

        let cv = cf(
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            &circle as *const _ as *const c_void,
            999,
        );
        let rv = rf(
            &circle as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            &circle as *const _ as *const c_void,
            999,
        );
        assert_eq!(cv, rv);
        assert_eq!(cv, 0);
    }
}

#[test]
fn test_exported_symbols_match() {
    // Both libraries should export the same set of T symbols.
    let symbols = [
        "c2V",
        "c2Maxv",
        "c2Minv",
        "c2Clampv",
        "c2Sub",
        "c2Dot",
        "c2CircletoCircle",
        "c2CircletoAABB",
        "c2AABBtoAABB",
        "collided",
    ];
    unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C lib");
        let r = Library::new(rust_lib_path()).expect("failed to load Rust lib");
        for sym in &symbols {
            let cs: Result<Symbol<unsafe extern "C" fn()>, _> = c.get(sym.as_bytes());
            assert!(cs.is_ok(), "C lib missing symbol {}", sym);
            let rs: Result<Symbol<unsafe extern "C" fn()>, _> = r.get(sym.as_bytes());
            assert!(rs.is_ok(), "Rust lib missing symbol {}", sym);
        }
    }
}
