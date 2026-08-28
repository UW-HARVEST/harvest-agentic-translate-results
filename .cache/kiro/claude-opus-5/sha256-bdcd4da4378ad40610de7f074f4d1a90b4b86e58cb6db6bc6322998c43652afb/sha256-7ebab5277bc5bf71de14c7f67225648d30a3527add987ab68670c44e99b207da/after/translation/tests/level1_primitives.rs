//! Level 1: leaf vector/rotation helpers. Every call goes through the exported
//! symbols of both shared libraries.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_int;

#[test]
fn c2V_matches() {
    let (c, r) = libs().sym::<FnV>("c2V");
    for &x in &special_f32() {
        for &y in &special_f32() {
            unsafe {
                assert_bits("c2V", &format!("({x}, {y})"), &c(x, y), &r(x, y));
            }
        }
    }
    let mut rng = Rng::new(1);
    for _ in 0..20_000 {
        let (x, y) = (rng.f32_range(1e6), rng.f32_range(1e6));
        unsafe {
            assert_bits("c2V", &format!("({x}, {y})"), &c(x, y), &r(x, y));
        }
    }
}

/// Drives a `(c2v, c2v) -> c2v` pair over the special grid and random inputs.
fn check_vv_v(name: &str) {
    let (c, r) = libs().sym::<FnVV_V>(name);
    let sv = special_vecs();
    for a in &sv {
        for b in sv.iter().step_by(7) {
            unsafe {
                assert_bits(name, &format!("{a:?} {b:?}"), &c(*a, *b), &r(*a, *b));
            }
        }
    }
    let mut rng = Rng::new(0xABCD);
    for _ in 0..50_000 {
        let (a, b) = (rng.vec_range(1e4), rng.vec_range(1e4));
        unsafe {
            assert_bits(name, &format!("{a:?} {b:?}"), &c(a, b), &r(a, b));
        }
        let (a, b) = (rng.vec_coarse(), rng.vec_coarse());
        unsafe {
            assert_bits(name, &format!("{a:?} {b:?}"), &c(a, b), &r(a, b));
        }
    }
}

#[test]
fn c2Add_matches() {
    check_vv_v("c2Add");
}

#[test]
fn c2Sub_matches() {
    check_vv_v("c2Sub");
}

#[test]
fn c2Minv_matches() {
    check_vv_v("c2Minv");
}

#[test]
fn c2Maxv_matches() {
    check_vv_v("c2Maxv");
}

#[test]
fn c2Dot_matches() {
    let (c, r) = libs().sym::<FnVV_f>("c2Dot");
    let sv = special_vecs();
    for a in &sv {
        for b in sv.iter().step_by(5) {
            unsafe {
                assert_bits("c2Dot", &format!("{a:?} {b:?}"), &c(*a, *b), &r(*a, *b));
            }
        }
    }
    let mut rng = Rng::new(7);
    for _ in 0..50_000 {
        let (a, b) = (rng.vec_range(1e18), rng.vec_range(1e18));
        unsafe {
            assert_bits("c2Dot", &format!("{a:?} {b:?}"), &c(a, b), &r(a, b));
        }
        let (a, b) = (rng.vec_coarse(), rng.vec_coarse());
        unsafe {
            assert_bits("c2Dot", &format!("{a:?} {b:?}"), &c(a, b), &r(a, b));
        }
    }
}

#[test]
fn c2Len_matches() {
    let (c, r) = libs().sym::<FnV_f>("c2Len");
    for a in special_vecs() {
        unsafe {
            assert_bits("c2Len", &format!("{a:?}"), &c(a), &r(a));
        }
    }
    let mut rng = Rng::new(11);
    for _ in 0..50_000 {
        let a = rng.vec_range(1e12);
        unsafe {
            assert_bits("c2Len", &format!("{a:?}"), &c(a), &r(a));
        }
        let a = rng.vec_coarse();
        unsafe {
            assert_bits("c2Len", &format!("{a:?}"), &c(a), &r(a));
        }
    }
}

fn check_v_v(name: &str) {
    let (c, r) = libs().sym::<FnV_V>(name);
    for a in special_vecs() {
        unsafe {
            assert_bits(name, &format!("{a:?}"), &c(a), &r(a));
        }
    }
    let mut rng = Rng::new(0x5EED);
    for _ in 0..50_000 {
        let a = rng.vec_range(1e8);
        unsafe {
            assert_bits(name, &format!("{a:?}"), &c(a), &r(a));
        }
        let a = rng.vec_coarse();
        unsafe {
            assert_bits(name, &format!("{a:?}"), &c(a), &r(a));
        }
    }
}

#[test]
fn c2Norm_matches() {
    check_v_v("c2Norm");
}

#[test]
fn c2Skew_matches() {
    check_v_v("c2Skew");
}

#[test]
fn c2Absv_matches() {
    check_v_v("c2Absv");
}

#[test]
fn c2CCW90_matches() {
    check_v_v("c2CCW90");
}

fn check_vf_v(name: &str) {
    let (c, r) = libs().sym::<FnVf_V>(name);
    let sf = special_f32();
    for a in special_vecs().iter().step_by(3) {
        for &b in &sf {
            unsafe {
                assert_bits(name, &format!("{a:?} {b}"), &c(*a, b), &r(*a, b));
            }
        }
    }
    let mut rng = Rng::new(0xF00D);
    for _ in 0..50_000 {
        let (a, b) = (rng.vec_range(1e6), rng.f32_range(1e6));
        unsafe {
            assert_bits(name, &format!("{a:?} {b}"), &c(a, b), &r(a, b));
        }
        let (a, b) = (rng.vec_coarse(), rng.f32_coarse());
        unsafe {
            assert_bits(name, &format!("{a:?} {b}"), &c(a, b), &r(a, b));
        }
    }
}

#[test]
fn c2Mulvs_matches() {
    check_vf_v("c2Mulvs");
}

#[test]
fn c2Div_matches() {
    check_vf_v("c2Div");
}

#[test]
fn c2MulmvT_matches() {
    let (c, r) = libs().sym::<FnMV_V>("c2MulmvT");
    let sv = special_vecs();
    for i in (0..sv.len()).step_by(11) {
        for j in (0..sv.len()).step_by(13) {
            let m = c2m {
                x: sv[i],
                y: sv[j],
            };
            for b in sv.iter().step_by(97) {
                unsafe {
                    assert_bits("c2MulmvT", "special", &c(m, *b), &r(m, *b));
                }
            }
        }
    }
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..50_000 {
        let m = c2m {
            x: rng.vec_range(1e4),
            y: rng.vec_range(1e4),
        };
        let b = rng.vec_range(1e4);
        unsafe {
            assert_bits("c2MulmvT", &format!("{m:?} {b:?}"), &c(m, b), &r(m, b));
        }
        let m = c2m {
            x: rng.vec_coarse(),
            y: rng.vec_coarse(),
        };
        let b = rng.vec_coarse();
        unsafe {
            assert_bits("c2MulmvT", &format!("{m:?} {b:?}"), &c(m, b), &r(m, b));
        }
    }
}

#[test]
fn c2RotIdentity_matches() {
    let (c, r) = libs().sym::<Fn_R>("c2RotIdentity");
    unsafe {
        assert_bits("c2RotIdentity", "()", &c(), &r());
    }
}

#[test]
fn c2xIdentity_matches() {
    let (c, r) = libs().sym::<Fn_X>("c2xIdentity");
    unsafe {
        assert_bits("c2xIdentity", "()", &c(), &r());
    }
}

fn check_rv_v(name: &str) {
    let (c, r) = libs().sym::<FnRV_V>(name);
    let sf = special_f32();
    let sv = special_vecs();
    for &cc in &sf {
        for &ss in &sf {
            let rot = c2r { c: cc, s: ss };
            for b in sv.iter().step_by(53) {
                unsafe {
                    assert_bits(name, &format!("{rot:?} {b:?}"), &c(rot, *b), &r(rot, *b));
                }
            }
        }
    }
    let mut rng = Rng::new(0xBEEF);
    for _ in 0..50_000 {
        // Real rotations plus arbitrary garbage.
        let ang = rng.f32_range(4.0);
        let rot = c2r {
            c: ang.cos(),
            s: ang.sin(),
        };
        let b = rng.vec_range(1e4);
        unsafe {
            assert_bits(name, &format!("{rot:?} {b:?}"), &c(rot, b), &r(rot, b));
        }
        let rot = c2r {
            c: rng.f32_coarse(),
            s: rng.f32_coarse(),
        };
        let b = rng.vec_coarse();
        unsafe {
            assert_bits(name, &format!("{rot:?} {b:?}"), &c(rot, b), &r(rot, b));
        }
    }
}

#[test]
fn c2Mulrv_matches() {
    check_rv_v("c2Mulrv");
}

#[test]
fn c2MulrvT_matches() {
    check_rv_v("c2MulrvT");
}

#[test]
fn c2MulxvT_matches() {
    let (c, r) = libs().sym::<FnXV_V>("c2MulxvT");
    let sf = special_f32();
    let sv = special_vecs();
    for i in (0..sv.len()).step_by(17) {
        for &cc in sf.iter().step_by(2) {
            for &ss in sf.iter().step_by(3) {
                let x = c2x {
                    p: sv[i],
                    r: c2r { c: cc, s: ss },
                };
                for b in sv.iter().step_by(101) {
                    unsafe {
                        assert_bits("c2MulxvT", "special", &c(x, *b), &r(x, *b));
                    }
                }
            }
        }
    }
    let mut rng = Rng::new(0xDEAD);
    for _ in 0..50_000 {
        let ang = rng.f32_range(4.0);
        let x = c2x {
            p: rng.vec_range(1e3),
            r: c2r {
                c: ang.cos(),
                s: ang.sin(),
            },
        };
        let b = rng.vec_range(1e3);
        unsafe {
            assert_bits("c2MulxvT", &format!("{x:?} {b:?}"), &c(x, b), &r(x, b));
        }
        let x = c2x {
            p: rng.vec_coarse(),
            r: c2r {
                c: rng.f32_coarse(),
                s: rng.f32_coarse(),
            },
        };
        let b = rng.vec_coarse();
        unsafe {
            assert_bits("c2MulxvT", &format!("{x:?} {b:?}"), &c(x, b), &r(x, b));
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: boolean overlap / containment predicates
// ---------------------------------------------------------------------------

#[test]
fn c2AABBtoAABB_matches() {
    let (c, r) = libs().sym::<FnAABBAABB_i>("c2AABBtoAABB");
    let sv = special_vecs();
    for i in (0..sv.len()).step_by(23) {
        for j in (0..sv.len()).step_by(29) {
            let A = c2AABB {
                min: sv[i],
                max: sv[j],
            };
            for k in (0..sv.len()).step_by(31) {
                let B = c2AABB {
                    min: sv[k],
                    max: sv[(k + 7) % sv.len()],
                };
                unsafe {
                    let (a, b): (c_int, c_int) = (c(A, B), r(A, B));
                    assert_bits("c2AABBtoAABB", &format!("{A:?} {B:?}"), &a, &b);
                }
            }
        }
    }
    let mut rng = Rng::new(0x1111);
    for _ in 0..50_000 {
        let mk = |rng: &mut Rng| {
            let a = rng.vec_coarse();
            let b = rng.vec_coarse();
            c2AABB { min: a, max: b }
        };
        let (A, B) = (mk(&mut rng), mk(&mut rng));
        unsafe {
            let (x, y): (c_int, c_int) = (c(A, B), r(A, B));
            assert_bits("c2AABBtoAABB", &format!("{A:?} {B:?}"), &x, &y);
        }
    }
}

#[test]
fn c2AABBtoPoint_matches() {
    let (c, r) = libs().sym::<FnAABBV_i>("c2AABBtoPoint");
    let sv = special_vecs();
    for i in (0..sv.len()).step_by(19) {
        for j in (0..sv.len()).step_by(23) {
            let A = c2AABB {
                min: sv[i],
                max: sv[j],
            };
            for B in sv.iter().step_by(37) {
                unsafe {
                    let (x, y): (c_int, c_int) = (c(A, *B), r(A, *B));
                    assert_bits("c2AABBtoPoint", &format!("{A:?} {B:?}"), &x, &y);
                }
            }
        }
    }
    let mut rng = Rng::new(0x2222);
    for _ in 0..50_000 {
        let A = c2AABB {
            min: rng.vec_coarse(),
            max: rng.vec_coarse(),
        };
        let B = rng.vec_coarse();
        unsafe {
            let (x, y): (c_int, c_int) = (c(A, B), r(A, B));
            assert_bits("c2AABBtoPoint", &format!("{A:?} {B:?}"), &x, &y);
        }
    }
}

#[test]
fn c2CircleToPoint_matches() {
    let (c, r) = libs().sym::<FnCircleV_i>("c2CircleToPoint");
    let sv = special_vecs();
    let sf = special_f32();
    for i in (0..sv.len()).step_by(13) {
        for &rad in &sf {
            let A = c2Circle { p: sv[i], r: rad };
            for B in sv.iter().step_by(41) {
                unsafe {
                    let (x, y): (c_int, c_int) = (c(A, *B), r(A, *B));
                    assert_bits("c2CircleToPoint", &format!("{A:?} {B:?}"), &x, &y);
                }
            }
        }
    }
    let mut rng = Rng::new(0x3333);
    for _ in 0..50_000 {
        let A = c2Circle {
            p: rng.vec_coarse(),
            r: rng.f32_coarse(),
        };
        let B = rng.vec_coarse();
        unsafe {
            let (x, y): (c_int, c_int) = (c(A, B), r(A, B));
            assert_bits("c2CircleToPoint", &format!("{A:?} {B:?}"), &x, &y);
        }
    }
}
