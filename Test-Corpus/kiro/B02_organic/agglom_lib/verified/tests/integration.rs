use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle { p: C2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2AABB { min: C2v, max: C2v }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LmVec2 { x: f32, y: f32 }

#[repr(C)]
struct CnRnd { state: [u64; 2] }

fn c_lib() -> Library {
    unsafe { Library::new("/tmp/harvest-work-zksuP3/translated_rust/c_src/build/libtranslated_rust.so").unwrap() }
}

fn rust_lib() -> Library {
    unsafe { Library::new("/tmp/harvest-work-zksuP3/translated_rust/target/debug/libagglom_lib.so").unwrap() }
}

// --- Test c2V ---
#[test]
fn test_c2v() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(f32, f32) -> C2v;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"c2V").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"c2V").unwrap() };
    for (x, y) in [(0.0f32, 0.0), (1.5, -2.3), (f32::MAX, f32::MIN)] {
        let cv = unsafe { c_fn(x, y) };
        let rv = unsafe { r_fn(x, y) };
        assert_eq!(cv.x.to_bits(), rv.x.to_bits());
        assert_eq!(cv.y.to_bits(), rv.y.to_bits());
    }
}

// --- Test c2Dot ---
#[test]
fn test_c2dot() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(C2v, C2v) -> f32;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"c2Dot").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"c2Dot").unwrap() };
    let cases = [
        (C2v{x:1.0,y:2.0}, C2v{x:3.0,y:4.0}),
        (C2v{x:0.0,y:0.0}, C2v{x:1.0,y:1.0}),
        (C2v{x:-1.5,y:2.5}, C2v{x:3.0,y:-4.0}),
    ];
    for (a, b) in cases {
        let cv = unsafe { c_fn(a, b) };
        let rv = unsafe { r_fn(a, b) };
        assert_eq!(cv.to_bits(), rv.to_bits(), "c2Dot mismatch for ({:?},{:?})", a, b);
    }
}

// --- Test c2Sub, c2Maxv, c2Minv, c2Clampv ---
#[test]
fn test_c2_vec_ops() {
    let c = c_lib();
    let r = rust_lib();
    type Fn2 = unsafe extern "C" fn(C2v, C2v) -> C2v;
    type Fn3 = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
    let pairs: &[(C2v, C2v)] = &[
        (C2v{x:1.0,y:2.0}, C2v{x:3.0,y:0.5}),
        (C2v{x:-1.0,y:-2.0}, C2v{x:3.0,y:4.0}),
    ];
    for name in [b"c2Sub" as &[u8], b"c2Maxv", b"c2Minv"] {
        let c_fn: Symbol<Fn2> = unsafe { c.get(name).unwrap() };
        let r_fn: Symbol<Fn2> = unsafe { r.get(name).unwrap() };
        for &(a, b_) in pairs {
            let cv = unsafe { c_fn(a, b_) };
            let rv = unsafe { r_fn(a, b_) };
            assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "{:?} x mismatch", std::str::from_utf8(name));
            assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "{:?} y mismatch", std::str::from_utf8(name));
        }
    }
    // c2Clampv
    let c_clamp: Symbol<Fn3> = unsafe { c.get(b"c2Clampv").unwrap() };
    let r_clamp: Symbol<Fn3> = unsafe { r.get(b"c2Clampv").unwrap() };
    let a = C2v{x:5.0,y:-1.0};
    let lo = C2v{x:0.0,y:0.0};
    let hi = C2v{x:3.0,y:3.0};
    let cv = unsafe { c_clamp(a, lo, hi) };
    let rv = unsafe { r_clamp(a, lo, hi) };
    assert_eq!(cv.x.to_bits(), rv.x.to_bits());
    assert_eq!(cv.y.to_bits(), rv.y.to_bits());
}

// --- Test c2CircletoCircle, c2CircletoAABB, c2AABBtoAABB ---
#[test]
fn test_collision() {
    let c = c_lib();
    let r = rust_lib();
    // CircletoCircle
    {
        type Fn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
        let c_fn: Symbol<Fn> = unsafe { c.get(b"c2CircletoCircle").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r.get(b"c2CircletoCircle").unwrap() };
        let cases = [
            (C2Circle{p:C2v{x:0.0,y:0.0},r:1.0}, C2Circle{p:C2v{x:0.5,y:0.0},r:1.0}),
            (C2Circle{p:C2v{x:0.0,y:0.0},r:1.0}, C2Circle{p:C2v{x:10.0,y:0.0},r:1.0}),
        ];
        for (a, b_) in cases {
            assert_eq!(unsafe{c_fn(a,b_)}, unsafe{r_fn(a,b_)}, "c2CircletoCircle mismatch");
        }
    }
    // CircletoAABB
    {
        type Fn = unsafe extern "C" fn(C2Circle, C2AABB) -> c_int;
        let c_fn: Symbol<Fn> = unsafe { c.get(b"c2CircletoAABB").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r.get(b"c2CircletoAABB").unwrap() };
        let cases = [
            (C2Circle{p:C2v{x:0.0,y:0.0},r:2.0}, C2AABB{min:C2v{x:1.0,y:1.0},max:C2v{x:3.0,y:3.0}}),
            (C2Circle{p:C2v{x:0.0,y:0.0},r:0.1}, C2AABB{min:C2v{x:5.0,y:5.0},max:C2v{x:6.0,y:6.0}}),
        ];
        for (a, b_) in cases {
            assert_eq!(unsafe{c_fn(a,b_)}, unsafe{r_fn(a,b_)}, "c2CircletoAABB mismatch");
        }
    }
    // AABBtoAABB
    {
        type Fn = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
        let c_fn: Symbol<Fn> = unsafe { c.get(b"c2AABBtoAABB").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { r.get(b"c2AABBtoAABB").unwrap() };
        let cases = [
            (C2AABB{min:C2v{x:0.0,y:0.0},max:C2v{x:2.0,y:2.0}}, C2AABB{min:C2v{x:1.0,y:1.0},max:C2v{x:3.0,y:3.0}}),
            (C2AABB{min:C2v{x:0.0,y:0.0},max:C2v{x:1.0,y:1.0}}, C2AABB{min:C2v{x:5.0,y:5.0},max:C2v{x:6.0,y:6.0}}),
        ];
        for (a, b_) in cases {
            assert_eq!(unsafe{c_fn(a,b_)}, unsafe{r_fn(a,b_)}, "c2AABBtoAABB mismatch");
        }
    }
}

// --- Test f2 (dispatch) ---
#[test]
fn test_f2() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f2").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f2").unwrap() };
    // Circle vs AABB
    let circle = C2Circle{p:C2v{x:0.0,y:0.0},r:2.0};
    let aabb = C2AABB{min:C2v{x:1.0,y:1.0},max:C2v{x:3.0,y:3.0}};
    let cv = unsafe { c_fn(&circle as *const _ as *const u8, 0, &aabb as *const _ as *const u8, 1) };
    let rv = unsafe { r_fn(&circle as *const _ as *const u8, 0, &aabb as *const _ as *const u8, 1) };
    assert_eq!(cv, rv, "f2 circle-aabb mismatch");
    // AABB vs Circle (reversed)
    let cv = unsafe { c_fn(&aabb as *const _ as *const u8, 1, &circle as *const _ as *const u8, 0) };
    let rv = unsafe { r_fn(&aabb as *const _ as *const u8, 1, &circle as *const _ as *const u8, 0) };
    assert_eq!(cv, rv, "f2 aabb-circle mismatch");
}

// --- Test f3 ---
#[test]
fn test_f3() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f3").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f3").unwrap() };
    let cases: &[(c_int, c_int)] = &[
        (10, 3), (-10, 3), (10, -3), (-10, -3),
        (0, 5), (5, 0), (7, 1), (-7, 1),
        (i32::MIN, 1), (i32::MIN, -1), (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN), (1, i32::MIN), (-1, i32::MIN),
        (100, 7), (-100, 7), (100, -7), (-100, -7),
    ];
    for &(v1, v2) in cases {
        let cv = unsafe { c_fn(v1, v2) };
        let rv = unsafe { r_fn(v1, v2) };
        assert_eq!(cv, rv, "f3({},{}) mismatch: C={} Rust={}", v1, v2, cv, rv);
    }
}

// --- Test f4 ---
#[test]
fn test_f4() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(*mut CnRnd) -> f64;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f4").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f4").unwrap() };
    let seeds: &[(u64, u64)] = &[(1, 2), (0, 0), (u64::MAX, u64::MAX), (12345, 67890)];
    for &(s0, s1) in seeds {
        let mut c_rnd = CnRnd { state: [s0, s1] };
        let mut r_rnd = CnRnd { state: [s0, s1] };
        for i in 0..10 {
            let cv = unsafe { c_fn(&mut c_rnd) };
            let rv = unsafe { r_fn(&mut r_rnd) };
            assert_eq!(cv.to_bits(), rv.to_bits(), "f4 mismatch at iter {} seed ({},{})", i, s0, s1);
        }
    }
}

// --- Test f5 ---
#[test]
fn test_f5() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(u32) -> u32;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f5").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f5").unwrap() };
    for a in [0u32, 1, 0xFFFF, 0xAAAA, 0x5555, 0x1234, 0xABCD, 0xFF00, 0x00FF] {
        let cv = unsafe { c_fn(a) };
        let rv = unsafe { r_fn(a) };
        assert_eq!(cv, rv, "f5({:#x}) mismatch: C={:#x} Rust={:#x}", a, cv, rv);
    }
}

// --- Test f7 ---
#[test]
fn test_f7() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(u32, u32, u32) -> u32;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f7").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f7").unwrap() };
    let cases: &[(u32, u32, u32)] = &[
        (1024, 2, 16), (4096, 1, 24), (256, 2, 32), (512, 6, 16), (1, 1, 8),
    ];
    for &(bs, ch, bd) in cases {
        let cv = unsafe { c_fn(bs, ch, bd) };
        let rv = unsafe { r_fn(bs, ch, bd) };
        assert_eq!(cv, rv, "f7({},{},{}) mismatch: C={} Rust={}", bs, ch, bd, cv, rv);
    }
}

// --- Test f9 ---
#[test]
fn test_f9() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f9").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f9").unwrap() };
    let cases = [
        (LmVec2{x:0.0,y:0.0}, LmVec2{x:1.0,y:0.0}, LmVec2{x:0.0,y:1.0}, LmVec2{x:0.25,y:0.25}),
        (LmVec2{x:1.0,y:1.0}, LmVec2{x:4.0,y:1.0}, LmVec2{x:1.0,y:5.0}, LmVec2{x:2.0,y:3.0}),
    ];
    for (p1, p2, p3, p) in cases {
        let cv = unsafe { c_fn(p1, p2, p3, p) };
        let rv = unsafe { r_fn(p1, p2, p3, p) };
        assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "f9 x mismatch");
        assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "f9 y mismatch");
    }
}

// --- Test f10 ---
#[test]
fn test_f10() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(u16) -> f32;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f10").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f10").unwrap() };
    // Test a range of half-float values
    let vals: &[u16] = &[0, 0x3C00, 0xBC00, 0x7BFF, 0x0001, 0x7C00, 0xFC00, 0x0400, 0x3555, 0x4000];
    for &h in vals {
        let cv = unsafe { c_fn(h) };
        let rv = unsafe { r_fn(h) };
        assert_eq!(cv.to_bits(), rv.to_bits(), "f10({:#06x}) mismatch: C={} Rust={}", h, cv, rv);
    }
}

// --- Test f11 ---
#[test]
fn test_f11() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(*mut f32, *const f32);
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f11").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f11").unwrap() };
    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.5],   // s=0
        [0.0, 1.0, 0.5],   // h=0
        [30.0, 0.5, 0.5],
        [90.0, 0.8, 0.3],
        [150.0, 0.6, 0.7],
        [200.0, 0.4, 0.2],
        [270.0, 1.0, 0.5],
        [330.0, 0.9, 0.4],
        [360.0, 0.5, 0.5],  // edge
        [119.0, 0.5, 0.5],  // tests the h<120 && h<180 branch
    ];
    for src in cases {
        let mut c_dest = [0.0f32; 3];
        let mut r_dest = [0.0f32; 3];
        unsafe { c_fn(c_dest.as_mut_ptr(), src.as_ptr()) };
        unsafe { r_fn(r_dest.as_mut_ptr(), src.as_ptr()) };
        for i in 0..3 {
            assert_eq!(c_dest[i].to_bits(), r_dest[i].to_bits(),
                "f11 mismatch for src={:?} idx={}: C={} Rust={}", src, i, c_dest[i], r_dest[i]);
        }
    }
}

// --- Test f12 ---
#[test]
fn test_f12() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(*mut f32, *const f32);
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f12").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f12").unwrap() };
    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.5],
        [0.0, 1.0, 1.0],
        [60.0, 0.5, 0.8],
        [120.0, 0.8, 0.3],
        [180.0, 0.6, 0.7],
        [240.0, 0.4, 0.2],
        [300.0, 1.0, 0.5],
        [359.0, 0.9, 0.4],
    ];
    for src in cases {
        let mut c_dest = [0.0f32; 3];
        let mut r_dest = [0.0f32; 3];
        unsafe { c_fn(c_dest.as_mut_ptr(), src.as_ptr()) };
        unsafe { r_fn(r_dest.as_mut_ptr(), src.as_ptr()) };
        for i in 0..3 {
            assert_eq!(c_dest[i].to_bits(), r_dest[i].to_bits(),
                "f12 mismatch for src={:?} idx={}: C={} Rust={}", src, i, c_dest[i], r_dest[i]);
        }
    }
}

// --- Test f13 ---
#[test]
fn test_f13() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(*mut f32, *const f32);
    let c_fn: Symbol<Fn> = unsafe { c.get(b"f13").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"f13").unwrap() };
    let cases: &[[f32; 3]] = &[
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.5, 0.5, 0.5],
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.2, 0.8, 0.4],
        [0.9, 0.1, 0.5],
    ];
    for src in cases {
        let mut c_dest = [0.0f32; 3];
        let mut r_dest = [0.0f32; 3];
        unsafe { c_fn(c_dest.as_mut_ptr(), src.as_ptr()) };
        unsafe { r_fn(r_dest.as_mut_ptr(), src.as_ptr()) };
        for i in 0..3 {
            assert_eq!(c_dest[i].to_bits(), r_dest[i].to_bits(),
                "f13 mismatch for src={:?} idx={}: C={} Rust={}", src, i, c_dest[i], r_dest[i]);
        }
    }
}

// --- Test agglom (top-level) ---
#[test]
fn test_agglom() {
    let c = c_lib();
    let r = rust_lib();
    type Fn = unsafe extern "C" fn(
        f32,f32,f32,f32,f32,f32,f32,
        c_int,c_int,
        u64,u64,
        u32,
        u32,u32,u32,
        f32,f32,f32,f32,f32,f32,f32,f32,
        u16,
        f32,f32,f32,
        f32,f32,f32,
        f32,f32,f32,
    ) -> f64;
    let c_fn: Symbol<Fn> = unsafe { c.get(b"agglom").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r.get(b"agglom").unwrap() };
    // Test case 1: typical values
    let cv = unsafe { c_fn(
        0.0,0.0,2.0, 1.0,1.0,3.0,3.0,
        10,3,
        1,2,
        0x1234,
        1024,2,16,
        0.0,0.0,1.0,0.0,0.0,1.0,0.25,0.25,
        0x3C00,
        30.0,0.5,0.5,
        120.0,0.8,0.3,
        0.2,0.8,0.4,
    )};
    let rv = unsafe { r_fn(
        0.0,0.0,2.0, 1.0,1.0,3.0,3.0,
        10,3,
        1,2,
        0x1234,
        1024,2,16,
        0.0,0.0,1.0,0.0,0.0,1.0,0.25,0.25,
        0x3C00,
        30.0,0.5,0.5,
        120.0,0.8,0.3,
        0.2,0.8,0.4,
    )};
    assert_eq!(cv.to_bits(), rv.to_bits(), "agglom case 1 mismatch: C={} Rust={}", cv, rv);
    // Test case 2: zeros
    let cv = unsafe { c_fn(
        0.0,0.0,0.0,0.0,0.0,0.0,0.0,
        0,0,
        0,0,
        0,
        0,0,0,
        0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,
        0,
        0.0,0.0,0.0,
        0.0,0.0,0.0,
        0.0,0.0,0.0,
    )};
    let rv = unsafe { r_fn(
        0.0,0.0,0.0,0.0,0.0,0.0,0.0,
        0,0,
        0,0,
        0,
        0,0,0,
        0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,
        0,
        0.0,0.0,0.0,
        0.0,0.0,0.0,
        0.0,0.0,0.0,
    )};
    assert_eq!(cv.to_bits(), rv.to_bits(), "agglom case 2 mismatch: C={} Rust={}", cv, rv);
}
