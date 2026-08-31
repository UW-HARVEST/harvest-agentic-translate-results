// Translation of c_src/src/jsdtoa.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::dtoadata::*;
use std::ffi::{c_char, c_int};

/*
 * format exponent like sprintf(p, "e%+d", e)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_fmtexp(p: *mut c_char, e: c_int) {
    unsafe {
        let mut se: [c_char; 9] = [0; 9];
        let mut i: usize;
        let mut p = p;
        let mut e = e;

        *p = b'e' as c_char;
        p = p.add(1);
        if e < 0 {
            *p = b'-' as c_char;
            p = p.add(1);
            e = -e;
        } else {
            *p = b'+' as c_char;
            p = p.add(1);
        }
        i = 0;
        while e != 0 {
            se[i] = ((e % 10) as u8 + b'0') as c_char;
            i += 1;
            e /= 10;
        }
        while i < 1 {
            se[i] = b'0' as c_char;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            *p = se[i];
            p = p.add(1);
        }
        *p = 0;
    }
}

/* grisu2 */

#[derive(Clone, Copy)]
struct diy_fp_t {
    f: u64,
    e: c_int,
}

const DIY_SIGNIFICAND_SIZE: c_int = 64;
const D_1_LOG2_10: f64 = 0.30102999566398114; /* 1 / lg(10) */

fn cached_power(k: c_int) -> diy_fp_t {
    let index = (343 + k) as usize;
    diy_fp_t {
        f: POWERS_TEN[index],
        e: POWERS_TEN_E[index],
    }
}

fn k_comp(e: c_int, alpha: c_int, _gamma: c_int) -> c_int {
    unsafe { ceil(((alpha - e + 63) as f64) * D_1_LOG2_10) as c_int }
}

fn minus(x: diy_fp_t, y: diy_fp_t) -> diy_fp_t {
    diy_fp_t {
        f: x.f.wrapping_sub(y.f),
        e: x.e,
    }
}

fn multiply(x: diy_fp_t, y: diy_fp_t) -> diy_fp_t {
    let m32: u64 = 0xFFFFFFFF;
    let a = x.f >> 32;
    let b = x.f & m32;
    let c = y.f >> 32;
    let d = y.f & m32;
    let ac = a.wrapping_mul(c);
    let bc = b.wrapping_mul(c);
    let ad = a.wrapping_mul(d);
    let bd = b.wrapping_mul(d);
    let mut tmp = (bd >> 32).wrapping_add(ad & m32).wrapping_add(bc & m32);
    tmp = tmp.wrapping_add(1u64 << 31);
    diy_fp_t {
        f: ac
            .wrapping_add(ad >> 32)
            .wrapping_add(bc >> 32)
            .wrapping_add(tmp >> 32),
        e: x.e + y.e + 64,
    }
}

fn double_to_uint64(d: f64) -> u64 {
    d.to_bits()
}

const DP_SIGNIFICAND_SIZE: c_int = 52;
const DP_EXPONENT_BIAS: c_int = 0x3FF + DP_SIGNIFICAND_SIZE;
const DP_MIN_EXPONENT: c_int = -DP_EXPONENT_BIAS;
const DP_EXPONENT_MASK: u64 = 0x7FF0000000000000;
const DP_SIGNIFICAND_MASK: u64 = 0x000FFFFFFFFFFFFF;
const DP_HIDDEN_BIT: u64 = 0x0010000000000000;

fn double2diy_fp(d: f64) -> diy_fp_t {
    let d64 = double_to_uint64(d);
    let biased_e = ((d64 & DP_EXPONENT_MASK) >> DP_SIGNIFICAND_SIZE) as c_int;
    let significand = d64 & DP_SIGNIFICAND_MASK;
    if biased_e != 0 {
        diy_fp_t {
            f: significand + DP_HIDDEN_BIT,
            e: biased_e - DP_EXPONENT_BIAS,
        }
    } else {
        diy_fp_t {
            f: significand,
            e: DP_MIN_EXPONENT + 1,
        }
    }
}

fn normalize_boundary(in_: diy_fp_t) -> diy_fp_t {
    let mut res = in_;
    /* the original number could have been a denormal. */
    while (res.f & (DP_HIDDEN_BIT << 1)) == 0 {
        res.f <<= 1;
        res.e -= 1;
    }
    /* do the final shifts in one go. */
    res.f <<= DIY_SIGNIFICAND_SIZE - DP_SIGNIFICAND_SIZE - 2;
    res.e = res.e - (DIY_SIGNIFICAND_SIZE - DP_SIGNIFICAND_SIZE - 2);
    res
}

fn normalized_boundaries(d: f64, out_m_minus: &mut diy_fp_t, out_m_plus: &mut diy_fp_t) {
    let v = double2diy_fp(d);
    let significand_is_zero = v.f == DP_HIDDEN_BIT;
    let mut pl = diy_fp_t {
        f: (v.f << 1) + 1,
        e: v.e - 1,
    };
    pl = normalize_boundary(pl);
    let mut mi = if significand_is_zero {
        diy_fp_t {
            f: (v.f << 2) - 1,
            e: v.e - 2,
        }
    } else {
        diy_fp_t {
            f: (v.f << 1) - 1,
            e: v.e - 1,
        }
    };
    mi.f <<= mi.e - pl.e;
    mi.e = pl.e;
    *out_m_plus = pl;
    *out_m_minus = mi;
}

const TEN2: u32 = 100;

unsafe fn digit_gen(
    Mp: diy_fp_t,
    delta: diy_fp_t,
    buffer: *mut c_char,
    len: *mut c_int,
    K: *mut c_int,
) {
    unsafe {
        let mut delta = delta;
        let mut div: u32;
        let mut p1: u32;
        let mut p2: u64;
        let mut d: c_int;
        let mut kappa: c_int;
        let one = diy_fp_t {
            f: 1u64 << (-Mp.e),
            e: Mp.e,
        };
        p1 = (Mp.f >> (-one.e)) as u32;
        p2 = Mp.f & (one.f - 1);
        *len = 0;
        kappa = 3;
        div = TEN2;
        while kappa > 0 {
            d = (p1 / div) as c_int;
            if d != 0 || *len != 0 {
                *buffer.offset(*len as isize) = (b'0' as c_int + d) as c_char;
                *len += 1;
            }
            p1 %= div;
            kappa -= 1;
            div /= 10;
            if ((p1 as u64) << (-one.e)) + p2 <= delta.f {
                *K += kappa;
                return;
            }
        }
        loop {
            p2 = p2.wrapping_mul(10);
            d = (p2 >> (-one.e)) as c_int;
            if d != 0 || *len != 0 {
                *buffer.offset(*len as isize) = (b'0' as c_int + d) as c_char;
                *len += 1;
            }
            p2 &= one.f - 1;
            kappa -= 1;
            delta.f = delta.f.wrapping_mul(10);
            if !(p2 > delta.f) {
                break;
            }
        }
        *K += kappa;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_grisu2(v: f64, buffer: *mut c_char, K: *mut c_int) -> c_int {
    unsafe {
        let mut length: c_int = 0;
        let mut w_m = diy_fp_t { f: 0, e: 0 };
        let mut w_p = diy_fp_t { f: 0, e: 0 };
        let q = 64;
        let alpha = -59;
        let gamma = -56;
        normalized_boundaries(v, &mut w_m, &mut w_p);
        let mk = k_comp(w_p.e + q, alpha, gamma);
        let c_mk = cached_power(mk);
        let mut Wp = multiply(w_p, c_mk);
        let mut Wm = multiply(w_m, c_mk);
        Wm.f = Wm.f.wrapping_add(1);
        Wp.f = Wp.f.wrapping_sub(1);
        let delta = minus(Wp, Wm);
        *K = -mk;
        digit_gen(Wp, delta, buffer, &mut length, K);
        length
    }
}

/* strtod */

static maxExponent: c_int = 511;

static powersOf10: [f64; 9] = [
    10., 100., 1.0e4, 1.0e8, 1.0e16, 1.0e32, 1.0e64, 1.0e128, 1.0e256,
];

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_strtod(string: *const c_char, endPtr: *mut *mut c_char) -> f64 {
    unsafe {
        let sign: bool;
        let mut expSign: bool = false;
        let mut fraction: f64;
        let mut dblExp: f64;
        let mut p: *const c_char;
        let mut c: c_int;

        /* Exponent read from "EX" field. */
        let mut exp: c_int = 0;
        let fracExp: c_int;
        let mut mantSize: c_int;
        let mut decPt: c_int;
        let pExp: *const c_char;

        p = string;
        while *p == b' ' as c_char
            || *p == b'\t' as c_char
            || *p == b'\n' as c_char
            || *p == b'\r' as c_char
        {
            p = p.add(1);
        }
        if *p == b'-' as c_char {
            sign = true;
            p = p.add(1);
        } else {
            if *p == b'+' as c_char {
                p = p.add(1);
            }
            sign = false;
        }

        decPt = -1;
        mantSize = 0;
        loop {
            c = *p as c_int;
            if !(c >= b'0' as c_int && c <= b'9' as c_int) {
                if c != b'.' as c_int || decPt >= 0 {
                    break;
                }
                decPt = mantSize;
            }
            p = p.add(1);
            mantSize += 1;
        }

        pExp = p;
        p = p.offset(-(mantSize as isize));
        if decPt < 0 {
            decPt = mantSize;
        } else {
            mantSize -= 1; /* One of the digits was the point. */
        }
        if mantSize > 18 {
            fracExp = decPt - 18;
            mantSize = 18;
        } else {
            fracExp = decPt - mantSize;
        }
        if mantSize == 0 {
            fraction = 0.0;
            p = string;
            if !endPtr.is_null() {
                *endPtr = p as *mut c_char;
            }
            if sign {
                return -fraction;
            }
            return fraction;
        } else {
            let mut frac1: c_int;
            let mut frac2: c_int;
            frac1 = 0;
            while mantSize > 9 {
                c = *p as c_int;
                p = p.add(1);
                if c == b'.' as c_int {
                    c = *p as c_int;
                    p = p.add(1);
                }
                frac1 = 10 * frac1 + (c - b'0' as c_int);
                mantSize -= 1;
            }
            frac2 = 0;
            while mantSize > 0 {
                c = *p as c_int;
                p = p.add(1);
                if c == b'.' as c_int {
                    c = *p as c_int;
                    p = p.add(1);
                }
                frac2 = 10 * frac2 + (c - b'0' as c_int);
                mantSize -= 1;
            }
            fraction = (1.0e9 * frac1 as f64) + frac2 as f64;
        }

        /* Skim off the exponent. */
        p = pExp;
        if *p == b'E' as c_char || *p == b'e' as c_char {
            p = p.add(1);
            if *p == b'-' as c_char {
                expSign = true;
                p = p.add(1);
            } else {
                if *p == b'+' as c_char {
                    p = p.add(1);
                }
                expSign = false;
            }
            while *p >= b'0' as c_char && *p <= b'9' as c_char && exp < INT_MAX / 100 {
                exp = exp * 10 + (*p as c_int - b'0' as c_int);
                p = p.add(1);
            }
            while *p >= b'0' as c_char && *p <= b'9' as c_char {
                p = p.add(1);
            }
        }
        if expSign {
            exp = fracExp - exp;
        } else {
            exp = fracExp + exp;
        }

        if exp < -maxExponent {
            exp = maxExponent;
            expSign = true;
        } else if exp > maxExponent {
            exp = maxExponent;
            expSign = false;
        } else if exp < 0 {
            expSign = true;
            exp = -exp;
        } else {
            expSign = false;
        }
        dblExp = 1.0;
        let mut di = 0usize;
        while exp != 0 {
            if exp & 1 != 0 {
                dblExp *= powersOf10[di];
            }
            exp >>= 1;
            di += 1;
        }
        if expSign {
            fraction /= dblExp;
        } else {
            fraction *= dblExp;
        }

        if !endPtr.is_null() {
            *endPtr = p as *mut c_char;
        }

        if sign {
            return -fraction;
        }
        fraction
    }
}
