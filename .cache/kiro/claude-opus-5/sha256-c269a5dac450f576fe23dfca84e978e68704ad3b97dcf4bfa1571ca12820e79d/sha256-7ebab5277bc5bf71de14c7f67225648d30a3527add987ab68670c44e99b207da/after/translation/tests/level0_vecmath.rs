//! Level 0: leaf vector/rotation math, compared C .so vs Rust .so.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 4000;

#[test]
fn t_c2V() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(f32, f32) -> c2v>(b"c2V");
    let mut rng = Rng::new(1);
    for i in 0..N {
        let (x, y) = (rng.spicy(), rng.spicy());
        let cv = unsafe { c(x, y) };
        let rv = unsafe { r(x, y) };
        assert_raw_eq!(cv, rv, "i={i} x={x:?} y={y:?}");
    }
}

#[test]
fn t_c2Mulvs() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2v, f32) -> c2v>(b"c2Mulvs");
    let mut rng = Rng::new(2);
    for i in 0..N {
        let a = rng.spicy_vec();
        let b = rng.spicy();
        assert_raw_eq!(unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} b={b:?}");
    }
}

#[test]
fn t_binary_vec_ops() {
    let l = libs();
    for name in [
        &b"c2Maxv"[..],
        &b"c2Minv"[..],
        &b"c2Sub"[..],
        &b"c2Add"[..],
    ] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2v, c2v) -> c2v>(name);
        let mut rng = Rng::new(3);
        for i in 0..N {
            let a = rng.spicy_vec();
            let b = rng.spicy_vec();
            assert_raw_eq!(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                "{} i={i} a={a:?} b={b:?}",
                String::from_utf8_lossy(name)
            );
        }
    }
}

#[test]
fn t_unary_vec_ops() {
    let l = libs();
    for name in [&b"c2Neg"[..], &b"c2Skew"[..], &b"c2CCW90"[..], &b"c2Norm"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2v) -> c2v>(name);
        let mut rng = Rng::new(4);
        for i in 0..N {
            let a = rng.spicy_vec();
            assert_raw_eq!(
                unsafe { c(a) },
                unsafe { r(a) },
                "{} i={i} a={a:?}",
                String::from_utf8_lossy(name)
            );
        }
    }
}

#[test]
fn t_scalar_vec_ops() {
    let l = libs();
    for name in [&b"c2Dot"[..], &b"c2Det2"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2v, c2v) -> f32>(name);
        let mut rng = Rng::new(5);
        for i in 0..N {
            let a = rng.spicy_vec();
            let b = rng.spicy_vec();
            assert_f32_bits_eq!(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                "{} i={i} a={a:?} b={b:?}",
                String::from_utf8_lossy(name)
            );
        }
    }
}

#[test]
fn t_c2Len() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2v) -> f32>(b"c2Len");
    let mut rng = Rng::new(6);
    for i in 0..N {
        let a = rng.spicy_vec();
        assert_f32_bits_eq!(unsafe { c(a) }, unsafe { r(a) }, "i={i} a={a:?}");
    }
}

#[test]
fn t_c2Div() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2v, f32) -> c2v>(b"c2Div");
    let mut rng = Rng::new(7);
    for i in 0..N {
        let a = rng.spicy_vec();
        let b = rng.spicy();
        assert_raw_eq!(unsafe { c(a, b) }, unsafe { r(a, b) }, "i={i} a={a:?} b={b:?}");
    }
}

#[test]
fn t_c2Clampv() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v>(b"c2Clampv");
    let mut rng = Rng::new(8);
    for i in 0..N {
        let a = rng.spicy_vec();
        let lo = rng.spicy_vec();
        let hi = rng.spicy_vec();
        assert_raw_eq!(
            unsafe { c(a, lo, hi) },
            unsafe { r(a, lo, hi) },
            "i={i} a={a:?} lo={lo:?} hi={hi:?}"
        );
    }
    // Also the well-ordered case used by c2CircletoAABB.
    let mut rng = Rng::new(9);
    for i in 0..N {
        let mnx = rng.coord();
        let mny = rng.coord();
        let lo = c2v { x: mnx, y: mny };
        let hi = c2v {
            x: mnx + rng.coord().abs(),
            y: mny + rng.coord().abs(),
        };
        let a = rng.vec();
        assert_raw_eq!(
            unsafe { c(a, lo, hi) },
            unsafe { r(a, lo, hi) },
            "ordered i={i} a={a:?} lo={lo:?} hi={hi:?}"
        );
    }
}

#[test]
fn t_identities() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn() -> c2r>(b"c2RotIdentity");
    assert_raw_eq!(unsafe { c() }, unsafe { r() }, "c2RotIdentity");
    let (c, r) = l.sym::<unsafe extern "C" fn() -> c2x>(b"c2xIdentity");
    assert_raw_eq!(unsafe { c() }, unsafe { r() }, "c2xIdentity");
}

#[test]
fn t_rot_mul() {
    let l = libs();
    for name in [&b"c2Mulrv"[..], &b"c2MulrvT"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2r, c2v) -> c2v>(name);
        let mut rng = Rng::new(10);
        for i in 0..N {
            let rot = c2r {
                c: rng.spicy(),
                s: rng.spicy(),
            };
            let b = rng.spicy_vec();
            assert_raw_eq!(
                unsafe { c(rot, b) },
                unsafe { r(rot, b) },
                "{} i={i} rot=({:?},{:?}) b={b:?}",
                String::from_utf8_lossy(name),
                rot.c,
                rot.s
            );
        }
        // Genuine rotations too.
        let mut rng = Rng::new(11);
        for i in 0..N {
            let ang = rng.unit() * std::f32::consts::PI;
            let rot = c2r {
                c: ang.cos(),
                s: ang.sin(),
            };
            let b = rng.vec();
            assert_raw_eq!(
                unsafe { c(rot, b) },
                unsafe { r(rot, b) },
                "{} rot i={i} ang={ang}",
                String::from_utf8_lossy(name)
            );
        }
    }
}

#[test]
fn t_c2Mulxv() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2x, c2v) -> c2v>(b"c2Mulxv");
    let mut rng = Rng::new(12);
    for i in 0..N {
        let x = c2x {
            p: rng.spicy_vec(),
            r: c2r {
                c: rng.spicy(),
                s: rng.spicy(),
            },
        };
        let b = rng.spicy_vec();
        assert_raw_eq!(unsafe { c(x, b) }, unsafe { r(x, b) }, "i={i} b={b:?}");
    }
}

#[test]
fn t_c2Support() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int>(b"c2Support");
    let mut rng = Rng::new(13);
    for i in 0..N {
        let n = 1 + (rng.next_u32() % 8) as usize;
        let verts: Vec<c2v> = (0..n).map(|_| rng.spicy_vec()).collect();
        let d = rng.spicy_vec();
        let cv = unsafe { c(verts.as_ptr(), n as c_int, d) };
        let rv = unsafe { r(verts.as_ptr(), n as c_int, d) };
        assert_eq!(cv, rv, "i={i} n={n} d={d:?} verts={verts:?}");
    }
    // count <= 0: C still reads verts[0] and returns 0.
    let verts = [c2v { x: 1.0, y: 2.0 }; 4];
    for count in [0i32, -1, -7] {
        let d = c2v { x: 0.5, y: -0.5 };
        let cv = unsafe { c(verts.as_ptr(), count, d) };
        let rv = unsafe { r(verts.as_ptr(), count, d) };
        assert_eq!(cv, rv, "count={count}");
    }
}

/// NaN *inputs* only. Both sides must agree on NaN-ness / finite values;
/// the NaN payload itself is register-allocation dependent (see
/// `common::f32_eq_nan_ok`) and therefore compared NaN-tolerantly.
#[test]
fn t_nan_inputs_agree() {
    let l = libs();
    let mut rng = Rng::new(101);

    for name in [
        &b"c2Maxv"[..],
        &b"c2Minv"[..],
        &b"c2Sub"[..],
        &b"c2Add"[..],
    ] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2v, c2v) -> c2v>(name);
        for i in 0..N {
            let a = rng.nanny_vec();
            let b = rng.nanny_vec();
            let (cv, rv) = (unsafe { c(a, b) }, unsafe { r(a, b) });
            assert!(
                c2v_eq_nan_ok(cv, rv),
                "{} i={i} a={a:?} b={b:?} -> C={cv:?} Rust={rv:?}",
                String::from_utf8_lossy(name)
            );
        }
    }

    for name in [&b"c2Neg"[..], &b"c2Skew"[..], &b"c2CCW90"[..], &b"c2Norm"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2v) -> c2v>(name);
        for i in 0..N {
            let a = rng.nanny_vec();
            let (cv, rv) = (unsafe { c(a) }, unsafe { r(a) });
            assert!(
                c2v_eq_nan_ok(cv, rv),
                "{} i={i} a={a:?} -> C={cv:?} Rust={rv:?}",
                String::from_utf8_lossy(name)
            );
        }
    }

    for name in [&b"c2Dot"[..], &b"c2Det2"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2v, c2v) -> f32>(name);
        for i in 0..N {
            let a = rng.nanny_vec();
            let b = rng.nanny_vec();
            let (cv, rv) = (unsafe { c(a, b) }, unsafe { r(a, b) });
            assert!(
                f32_eq_nan_ok(cv, rv),
                "{} i={i} a={a:?} b={b:?} -> C={cv:?} Rust={rv:?}",
                String::from_utf8_lossy(name)
            );
        }
    }

    let (c, r) = l.sym::<unsafe extern "C" fn(c2v) -> f32>(b"c2Len");
    for i in 0..N {
        let a = rng.nanny_vec();
        let (cv, rv) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(
            f32_eq_nan_ok(cv, rv),
            "c2Len i={i} a={a:?} -> C={cv:?} Rust={rv:?}"
        );
    }

    for name in [&b"c2Mulvs"[..], &b"c2Div"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2v, f32) -> c2v>(name);
        for i in 0..N {
            let a = rng.nanny_vec();
            let b = rng.nanny();
            let (cv, rv) = (unsafe { c(a, b) }, unsafe { r(a, b) });
            assert!(
                c2v_eq_nan_ok(cv, rv),
                "{} i={i} a={a:?} b={b:?} -> C={cv:?} Rust={rv:?}",
                String::from_utf8_lossy(name)
            );
        }
    }

    for name in [&b"c2Mulrv"[..], &b"c2MulrvT"[..]] {
        let (c, r) = l.sym::<unsafe extern "C" fn(c2r, c2v) -> c2v>(name);
        for i in 0..N {
            let rot = c2r {
                c: rng.nanny(),
                s: rng.nanny(),
            };
            let b = rng.nanny_vec();
            let (cv, rv) = (unsafe { c(rot, b) }, unsafe { r(rot, b) });
            assert!(
                c2v_eq_nan_ok(cv, rv),
                "{} i={i} rot=({:?},{:?}) b={b:?} -> C={cv:?} Rust={rv:?}",
                String::from_utf8_lossy(name),
                rot.c,
                rot.s
            );
        }
    }

    let (c, r) = l.sym::<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v>(b"c2Clampv");
    for i in 0..N {
        let a = rng.nanny_vec();
        let lo = rng.nanny_vec();
        let hi = rng.nanny_vec();
        let (cv, rv) = (unsafe { c(a, lo, hi) }, unsafe { r(a, lo, hi) });
        assert!(
            c2v_eq_nan_ok(cv, rv),
            "c2Clampv i={i} a={a:?} lo={lo:?} hi={hi:?} -> C={cv:?} Rust={rv:?}"
        );
    }
}

/// `sqrtf` of a negative operand: the C source only ever calls it on
/// `dot(a,a)`, but pin the behaviour down anyway since it is observable
/// through `c2Len`/`c2Norm` when intermediates overflow.
#[test]
fn t_sqrt_edge_cases() {
    let l = libs();
    let (c, r) = l.sym::<unsafe extern "C" fn(c2v) -> f32>(b"c2Len");
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN_POSITIVE, y: 0.0 },
        c2v { x: f32::INFINITY, y: 0.0 },
        c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
        c2v { x: 1e-30, y: 1e-30 },
        c2v { x: 3.0, y: 4.0 },
    ] {
        assert_f32_bits_eq!(unsafe { c(a) }, unsafe { r(a) }, "c2Len a={a:?}");
    }
    let (c, r) = l.sym::<unsafe extern "C" fn(c2v) -> c2v>(b"c2Norm");
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::INFINITY, y: 1.0 },
        c2v { x: 1e-30, y: -1e-30 },
    ] {
        assert_raw_eq!(unsafe { c(a) }, unsafe { r(a) }, "c2Norm a={a:?}");
    }
}
