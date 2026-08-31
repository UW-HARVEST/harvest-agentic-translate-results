//! Phase C — argument-evaluation order of the floating-point `png_set_*`
//! wrappers.
//!
//! Five C wrappers convert several `double`s with `png_fixed()` /
//! `png_fixed_ITU()` *inside the argument list of one call*, so the order in
//! which the conversions happen (and therefore WHICH "fixed point overflow in
//! ..." message the caller sees when more than one argument is out of range) is
//! unspecified in C.  The reference build must still be reproduced exactly, so
//! this file pins the observable order down for every subset of out-of-range
//! arguments.
mod common;
use common::*;

/// A value that `png_fixed`/`png_fixed_ITU` cannot represent.
const OVF: f64 = 1e30;
/// A value both accept.
const OK: f64 = 0.25;

fn pick(mask: u32, i: u32) -> f64 {
    if mask & (1 << i) != 0 {
        OVF
    } else {
        OK
    }
}

fn run(label: &str, nargs: u32, call: impl Fn(&'static Api, png_structp, png_infop, u32) + Copy) {
    // every subset of out-of-range arguments (capped for the wide functions)
    let total = 1u32 << nargs;
    let step = if total > 1024 { total / 1024 } else { 1 };
    let mut m = 0u32;
    while m < total {
        let mut outs = Vec::new();
        for api in both() {
            unsafe {
                set_current_api(api);
                diag_reset();
                let s = ReadSess::new(api, &[]);
                let ok = guard(|| call(api, s.png, s.info, m)).is_some();
                outs.push((ok, diag_take()));
            }
        }
        assert_eq!(
            outs[0], outs[1],
            "{}: overflow mask {:#b} -> C {:?} vs RS {:?}",
            label, m, outs[0], outs[1]
        );
        m += step;
    }
    // and the all-overflow case explicitly
    let mut outs = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let s = ReadSess::new(api, &[]);
            let ok = guard(|| call(api, s.png, s.info, total - 1)).is_some();
            outs.push((ok, diag_take()));
        }
    }
    assert_eq!(outs[0], outs[1], "{}: all arguments overflow", label);
}

#[test]
fn set_cHRM_conversion_order() {
    run("png_set_cHRM", 8, |api, png, info, m| unsafe {
        (api.png_set_cHRM)(
            png,
            info,
            pick(m, 0),
            pick(m, 1),
            pick(m, 2),
            pick(m, 3),
            pick(m, 4),
            pick(m, 5),
            pick(m, 6),
            pick(m, 7),
        )
    });
}

#[test]
fn set_cHRM_XYZ_conversion_order() {
    run("png_set_cHRM_XYZ", 9, |api, png, info, m| unsafe {
        (api.png_set_cHRM_XYZ)(
            png,
            info,
            pick(m, 0),
            pick(m, 1),
            pick(m, 2),
            pick(m, 3),
            pick(m, 4),
            pick(m, 5),
            pick(m, 6),
            pick(m, 7),
            pick(m, 8),
        )
    });
}

#[test]
fn set_cLLI_conversion_order() {
    run("png_set_cLLI", 2, |api, png, info, m| unsafe {
        (api.png_set_cLLI)(png, info, pick(m, 0), pick(m, 1))
    });
}

#[test]
fn set_mDCV_conversion_order() {
    run("png_set_mDCV", 10, |api, png, info, m| unsafe {
        (api.png_set_mDCV)(
            png,
            info,
            pick(m, 0),
            pick(m, 1),
            pick(m, 2),
            pick(m, 3),
            pick(m, 4),
            pick(m, 5),
            pick(m, 6),
            pick(m, 7),
            pick(m, 8),
            pick(m, 9),
        )
    });
}

#[test]
fn set_gamma_conversion_order() {
    // png_set_gamma converts both arguments with convert_gamma_value(), which
    // can raise "gamma value out of range" / "fixed point overflow"; the
    // reference build converts `file_gamma` first.
    let vals = [
        0.0f64,
        -1.0,
        1.0,
        0.45455,
        2.2,
        1e30,
        -1e30,
        f64::MAX,
        f64::MIN,
        1e-30,
        21474.83648,
        100000.0,
    ];
    for &a in &vals {
        for &b in &vals {
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let s = ReadSess::new(api, &[]);
                    let ok = guard(|| (api.png_set_gamma)(s.png, a, b)).is_some();
                    outs.push((ok, diag_take()));
                }
            }
            assert_eq!(
                outs[0], outs[1],
                "png_set_gamma({}, {}): C {:?} vs RS {:?}",
                a, b, outs[0], outs[1]
            );
        }
    }
}

#[test]
fn set_rgb_to_gray_conversion_order() {
    run("png_set_rgb_to_gray", 2, |api, png, info, m| unsafe {
        let _ = info;
        (api.png_set_rgb_to_gray)(png, 1, pick(m, 0), pick(m, 1))
    });
}

/// The same functions with a mix of in-range, boundary and huge values, so the
/// *successful* conversions are compared too (not only the error messages).
#[test]
fn conversion_order_with_boundary_values() {
    let vals = [
        0.0f64,
        1.0,
        -1.0,
        0.5,
        21474.83647,
        21474.83648,
        -21474.83647,
        -21474.83648,
        1e-9,
        1e9,
        OVF,
        -OVF,
        f64::MAX,
        f64::MIN,
    ];
    let mut rng = Rng::new(0x0f1e_2d3c_4b5a_6907);
    for _ in 0..400 {
        let mut a = [0.0f64; 10];
        for x in a.iter_mut() {
            *x = vals[rng.below(vals.len() as u32) as usize];
        }
        for which in 0..5 {
            let mut outs = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let s = ReadSess::new(api, &[]);
                    let mut got = (0u32, 0.0f64, 0.0f64);
                    let ok = guard(|| {
                        match which {
                            0 => (api.png_set_cHRM)(
                                s.png, s.info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                            ),
                            1 => (api.png_set_cHRM_XYZ)(
                                s.png, s.info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                                a[8],
                            ),
                            2 => (api.png_set_cLLI)(s.png, s.info, a[0], a[1]),
                            3 => (api.png_set_mDCV)(
                                s.png, s.info, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                                a[8], a[9],
                            ),
                            _ => (api.png_set_rgb_to_gray)(s.png, 1, a[0], a[1]),
                        }
                        got.0 = (api.png_get_valid)(s.png, s.info, 0xffff_ffff);
                        // read back whatever was stored
                        let mut wx = 0i32;
                        let mut wy = 0i32;
                        let mut rx = 0i32;
                        let mut ry = 0i32;
                        let mut gx = 0i32;
                        let mut gy = 0i32;
                        let mut bx = 0i32;
                        let mut by = 0i32;
                        (api.png_get_cHRM_fixed)(
                            s.png, s.info, &mut wx, &mut wy, &mut rx, &mut ry, &mut gx,
                            &mut gy, &mut bx, &mut by,
                        );
                        got.1 = wx as f64 + wy as f64 + rx as f64 + ry as f64;
                        got.2 = gx as f64 + gy as f64 + bx as f64 + by as f64;
                    })
                    .is_some();
                    outs.push((ok, diag_take(), got.0, got.1.to_bits(), got.2.to_bits()));
                }
            }
            assert_eq!(
                outs[0], outs[1],
                "case {} with {:?}: C {:?} vs RS {:?}",
                which, a, outs[0], outs[1]
            );
        }
    }
}
