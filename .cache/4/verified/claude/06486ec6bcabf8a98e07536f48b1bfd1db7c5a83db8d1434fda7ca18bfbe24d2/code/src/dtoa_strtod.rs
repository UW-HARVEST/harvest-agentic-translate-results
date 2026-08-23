// Literal translation of the strtod-side helpers of c_src/src/dtoa.c:
//   match(), hexnan(), gethex(), sulp(), bigcomp()
// (IEEE_8087, USE_BF96, long long available, no MULTIPLE_THREADS,
//  no USE_LOCALE, no Honor_FLT_ROUNDS, INFNAN_CHECK, Avoid_Underflow).
#![allow(dead_code, non_snake_case, non_upper_case_globals, unused_assignments, unused_mut, unused_variables, unused_parens, unused_labels)]

use crate::dtoa::*;
use crate::dtoa_tables::*;
use crate::libc;
use std::ffi::{c_char, c_int};
use std::ptr;

/* rounding values: same as FLT_ROUNDS */
pub const Round_zero: c_int = 0;
pub const Round_near: c_int = 1;
pub const Round_up: c_int = 2;
pub const Round_down: c_int = 3;

#[inline]
unsafe fn hexdig_of(c: u8) -> c_int {
    HEXDIG[c as usize] as c_int
}

pub unsafe fn match_(sp: *mut *const c_char, t0: *const c_char) -> c_int {
    let mut c: c_int;
    let mut d: c_int;
    let mut t = t0;
    let mut s = *sp;

    loop {
        d = *t as c_int;
        t = t.add(1);
        if d == 0 {
            break;
        }
        s = s.add(1);
        c = *s as c_int;
        if c >= 'A' as c_int && c <= 'Z' as c_int {
            c += 'a' as c_int - 'A' as c_int;
        }
        if c != d {
            return 0;
        }
    }
    *sp = s.add(1);
    1
}

pub unsafe fn hexnan(rvp: *mut U, sp: *mut *const c_char) {
    let mut c: ULong;
    let mut x: [ULong; 2] = [0, 0];
    let mut s: *const u8;
    let mut c1: c_int;
    let mut havedig: c_int;
    let mut udx0: c_int;
    let mut xshift: c_int;

    havedig = 0;
    xshift = 0;
    udx0 = 1;
    s = *sp as *const u8;

    loop {
        c = *s.add(1) as ULong;
        if !(c != 0 && c <= b' ' as ULong) {
            break;
        }
        s = s.add(1);
    }
    if *s.add(1) == b'0' && (*s.add(2) == b'x' || *s.add(2) == b'X') {
        s = s.add(2);
    }
    loop {
        s = s.add(1);
        c = *s as ULong;
        if c == 0 {
            break;
        }
        c1 = hexdig_of(c as u8);
        if c1 != 0 {
            c = (c1 & 0xf) as ULong;
        } else if c <= b' ' as ULong {
            if udx0 != 0 && havedig != 0 {
                udx0 = 0;
                xshift = 1;
            }
            continue;
        } else {
            loop {
                if c == b')' as ULong {
                    *sp = s.add(1) as *const c_char;
                    break;
                }
                s = s.add(1);
                c = *s as ULong;
                if c == 0 {
                    break;
                }
            }
            break;
        }
        havedig = 1;
        if xshift != 0 {
            xshift = 0;
            x[0] = x[1];
            x[1] = 0;
        }
        if udx0 != 0 {
            x[0] = (x[0] << 4) | (x[1] >> 28);
        }
        x[1] = (x[1] << 4) | c;
    }
    x[0] &= 0xfffff;
    if x[0] != 0 || x[1] != 0 {
        (*rvp).set_w0(Exp_mask | x[0]);
        (*rvp).set_w1(x[1]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethex(
    sp: *mut *const c_char,
    rvp: *mut U,
    rounding: c_int,
    sign: c_int,
) {
    let mut b: *mut Bigint = ptr::null_mut();
    let mut d: c_int;
    let mut decpt: *const u8;
    let mut s0: *const u8;
    let mut s: *const u8;
    let mut s1: *const u8;
    let mut e: c_int;
    let mut e1: c_int;
    let mut L: ULong;
    let mut lostbits: ULong;
    let mut x: *mut ULong;
    let mut big: c_int;
    let mut denorm: c_int;
    let mut esign: c_int;
    let mut havedig: c_int;
    let mut k: c_int;
    let mut n: c_int;
    let mut nb: c_int;
    let mut nbits: c_int;
    let mut nz: c_int;
    let mut up: c_int;
    let mut zret: c_int;
    const emax: c_int = 0x7fe - 1023 - 53 + 1;
    const emin: c_int = -1022 - 53 + 1;
    let mut check_denorm: c_int = 0;

    havedig = 0;
    s0 = (*sp as *const u8).add(2);
    while *s0.add(havedig as usize) == b'0' {
        havedig += 1;
    }
    s0 = s0.add(havedig as usize);
    s = s0;
    decpt = ptr::null();
    zret = 0;
    e = 0;

    'pcheck: {
        if hexdig_of(*s) != 0 {
            havedig += 1;
        } else {
            zret = 1;
            if *s != b'.' {
                break 'pcheck;
            }
            s = s.add(1);
            decpt = s;
            if hexdig_of(*s) == 0 {
                break 'pcheck;
            }
            while *s == b'0' {
                s = s.add(1);
            }
            if hexdig_of(*s) != 0 {
                zret = 0;
            }
            havedig = 1;
            s0 = s;
        }
        while hexdig_of(*s) != 0 {
            s = s.add(1);
        }
        if *s == b'.' && decpt.is_null() {
            s = s.add(1);
            decpt = s;
            while hexdig_of(*s) != 0 {
                s = s.add(1);
            }
        }
        if !decpt.is_null() {
            e = -(((s.offset_from(decpt) as c_int)) << 2);
        }
    }
    /* pcheck: */
    s1 = s;
    big = 0;
    esign = 0;
    if *s == b'p' || *s == b'P' {
        'pexp: {
            s = s.add(1);
            match *s {
                b'-' => {
                    esign = 1;
                    s = s.add(1);
                }
                b'+' => {
                    s = s.add(1);
                }
                _ => {}
            }
            n = hexdig_of(*s);
            if n == 0 || n > 0x19 {
                s = s1;
                break 'pexp;
            }
            e1 = n - 0x10;
            loop {
                s = s.add(1);
                n = hexdig_of(*s);
                if !(n != 0 && n <= 0x19) {
                    break;
                }
                if (e1 & 0xf8000000u32 as c_int) != 0 {
                    big = 1;
                }
                e1 = 10 * e1 + n - 0x10;
            }
            if esign != 0 {
                e1 = -e1;
            }
            e += e1;
        }
    }
    *sp = s as *const c_char;
    if havedig == 0 {
        *sp = (s0 as *const c_char).sub(1);
    }
    if zret != 0 {
        /* retz1 */
        (*rvp).set_dval(0.0);
        return;
    }
    if big != 0 {
        if esign != 0 {
            let mut to_ret_tiny = false;
            match rounding {
                Round_up => {
                    if sign != 0 {
                        /* break out of switch */
                    } else {
                        to_ret_tiny = true;
                    }
                }
                Round_down => {
                    if sign == 0 {
                        /* break out of switch */
                    } else {
                        to_ret_tiny = true;
                    }
                }
                _ => {}
            }
            if to_ret_tiny {
                /* ret_tiny */
                libc::set_errno(libc::ERANGE);
                (*rvp).set_w0(0);
                (*rvp).set_w1(1);
                return;
            }
            /* goto retz */
            libc::set_errno(libc::ERANGE);
            (*rvp).set_dval(0.0);
            return;
        }
        let mut to_ovfl1 = false;
        match rounding {
            Round_near => to_ovfl1 = true,
            Round_up => {
                if sign == 0 {
                    to_ovfl1 = true;
                }
            }
            Round_down => {
                if sign != 0 {
                    to_ovfl1 = true;
                }
            }
            _ => {}
        }
        if to_ovfl1 {
            libc::set_errno(libc::ERANGE);
            (*rvp).set_w0(Exp_mask);
            (*rvp).set_w1(0);
            return;
        }
        /* ret_big */
        (*rvp).set_w0(Big0);
        (*rvp).set_w1(Big1);
        return;
    }
    n = (s1.offset_from(s0) as c_int) - 1;
    k = 0;
    while n > (1 << (5 - 2)) - 1 {
        k += 1;
        n >>= 1;
    }
    b = Balloc(k);
    x = bx(b);
    havedig = 0;
    n = 0;
    nz = 0;
    L = 0;
    while s1 > s0 {
        s1 = s1.sub(1);
        if *s1 == b'.' {
            continue;
        }
        d = hexdig_of(*s1);
        if d != 0 {
            havedig = 1;
        } else if havedig == 0 {
            e += 4;
            continue;
        }
        if n == 32 {
            *x = L;
            x = x.add(1);
            L = 0;
            n = 0;
        }
        L |= ((d & 0x0f) as ULong) << n;
        n += 4;
    }
    *x = L;
    x = x.add(1);
    n = x.offset_from(bx(b)) as c_int;
    (*b).wds = n;
    nb = 32 * n - hi0bits(L);
    nbits = 53;
    lostbits = 0;
    x = bx(b);
    if nb > nbits {
        n = nb - nbits;
        if any_on(b, n) != 0 {
            lostbits = 1;
            k = n - 1;
            if (*x.add((k >> 5) as usize) & (1 << (k & 31))) != 0 {
                lostbits = 2;
                if k > 0 && any_on(b, k) != 0 {
                    lostbits = 3;
                }
            }
        }
        rshift(b, n);
        e += n;
    } else if nb < nbits {
        n = nbits - nb;
        b = lshift(b, n);
        e -= n;
        x = bx(b);
    }
    if e > emax {
        /* ovfl */
        Bfree(b);
        libc::set_errno(libc::ERANGE);
        (*rvp).set_w0(Exp_mask);
        (*rvp).set_w1(0);
        return;
    }
    denorm = 0;
    let mut goto_normal = false;
    if e < emin {
        denorm = 1;
        n = emin - e;
        if n >= nbits {
            let mut to_ret_tinyf = false;
            match rounding {
                Round_near => {
                    if n == nbits && (n < 2 || lostbits != 0 || any_on(b, n - 1) != 0) {
                        to_ret_tinyf = true;
                    }
                }
                Round_up => {
                    if sign == 0 {
                        to_ret_tinyf = true;
                    }
                }
                Round_down => {
                    if sign != 0 {
                        to_ret_tinyf = true;
                    }
                }
                _ => {}
            }
            if to_ret_tinyf {
                /* ret_tinyf */
                Bfree(b);
                libc::set_errno(libc::ERANGE);
                (*rvp).set_w0(0);
                (*rvp).set_w1(1);
                return;
            }
            Bfree(b);
            /* retz */
            libc::set_errno(libc::ERANGE);
            (*rvp).set_dval(0.0);
            return;
        }
        k = n - 1;
        let mut goto_no_lostbits = false;
        if k == 0 {
            let mut do_emin_check = false;
            let mut do_incr_denorm = false;
            match rounding {
                Round_near => {
                    if (*bx(b) & 3) == 3 || (lostbits != 0 && (*bx(b) & 1) != 0) {
                        multadd(b, 1, 1);
                        do_emin_check = true;
                    }
                }
                Round_up => {
                    if sign == 0 && (lostbits != 0 || (*bx(b) & 1) != 0) {
                        do_incr_denorm = true;
                    }
                }
                Round_down => {
                    if sign != 0 && (lostbits != 0 || (*bx(b) & 1) != 0) {
                        do_incr_denorm = true;
                    }
                }
                _ => {}
            }
            if do_incr_denorm {
                /* incr_denorm */
                multadd(b, 1, 2);
                check_denorm = 1;
                lostbits = 0;
                do_emin_check = true;
            }
            if do_emin_check {
                /* emin_check */
                if *bx(b).add(1) == (1 << (Exp_shift + 1)) {
                    rshift(b, 1);
                    e = emin;
                    goto_normal = true;
                }
            }
        }
        if !goto_normal {
            if lostbits != 0 {
                lostbits = 1;
            } else if k > 0 {
                lostbits = any_on(b, k);
            } else if check_denorm != 0 {
                goto_no_lostbits = true;
            }
            if !goto_no_lostbits && (*x.add((k >> 5) as usize) & (1 << (k & 31))) != 0 {
                lostbits |= 2;
            }
            /* no_lostbits: */
            nbits -= n;
            rshift(b, n);
            e = emin;
        }
    }
    if !goto_normal {
        if lostbits != 0 {
            up = 0;
            match rounding {
                Round_zero => {}
                Round_near => {
                    if (lostbits & 2) != 0 && ((lostbits & 1) | (*x & 1)) != 0 {
                        up = 1;
                    }
                }
                Round_up => {
                    up = 1 - sign;
                }
                Round_down => {
                    up = sign;
                }
                _ => {}
            }
            if up != 0 {
                k = (*b).wds;
                b = increment(b);
                x = bx(b);
                if denorm == 0 {
                    n = nbits & 31;
                    if (*b).wds > k || (n != 0 && hi0bits(*x.add((k - 1) as usize)) < 32 - n) {
                        rshift(b, 1);
                        e += 1;
                        if e > 1023 {
                            /* ovfl */
                            Bfree(b);
                            libc::set_errno(libc::ERANGE);
                            (*rvp).set_w0(Exp_mask);
                            (*rvp).set_w1(0);
                            return;
                        }
                    }
                }
            }
        }
        if denorm != 0 {
            (*rvp).set_w0(if (*b).wds > 1 {
                *bx(b).add(1) & !Exp_msk1
            } else {
                0
            });
            (*rvp).set_w1(*bx(b));
            Bfree(b);
            return;
        }
    }
    /* normal: */
    (*rvp).set_w0(
        (*bx(b).add(1) & !Exp_msk1) | (((e + Bias + 52) as ULong) << Exp_shift),
    );
    (*rvp).set_w1(*bx(b));
    Bfree(b);
}

pub unsafe fn sulp(x: *mut U, bc: *const BCinfo) -> f64 {
    let mut u = U::new();
    let rv: f64;
    let i: c_int;

    rv = ulp(x);
    i = 2 * P + 1 - ((((*x).w0() & Exp_mask) >> Exp_shift) as c_int);
    if (*bc).scale == 0 || i <= 0 {
        return rv;
    }
    u.set_w0(Exp_1 + ((i as ULong) << Exp_shift));
    u.set_w1(0);
    rv * u.dval()
}

pub unsafe fn bigcomp(rv: *mut U, s0: *const c_char, bc: *mut BCinfo) {
    let mut b: *mut Bigint;
    let mut d: *mut Bigint;
    let mut b2: c_int;
    let mut bbits: c_int = 0;
    let mut d2: c_int;
    let mut dd: c_int = 0;
    let mut dig: c_int;
    let mut dsign: c_int;
    let mut i: c_int = 0;
    let mut j: c_int;
    let nd: c_int;
    let nd0: c_int;
    let mut p2: c_int = 0;
    let p5: c_int;
    let mut speccase: c_int;

    dsign = (*bc).dsign;
    nd = (*bc).nd;
    nd0 = (*bc).nd0;
    p5 = nd + (*bc).e0 - 1;
    speccase = 0;
    let mut have_i = false;
    if (*rv).dval() == 0.0 {
        b = i2b(1);
        p2 = Emin - P + 1;
        bbits = 1;
        (*rv).set_w0(((P + 2) as ULong) << Exp_shift);
        i = 0;
        speccase = 1;
        p2 -= 1;
        dsign = 0;
        have_i = true;
    } else {
        b = d2b(rv, &mut p2, &mut bbits);
    }
    if !have_i {
        p2 -= (*bc).scale;
        i = P - bbits;
        j = P - Emin - 1 + p2;
        if i > j {
            i = j;
        }
        i += 1;
        b = lshift(b, i);
        *bx(b) |= 1;
    }
    /* have_i: */
    p2 -= p5 + i;
    d = i2b(1);
    if p5 > 0 {
        d = pow5mult(d, p5);
    } else if p5 < 0 {
        b = pow5mult(b, -p5);
    }
    if p2 > 0 {
        b2 = p2;
        d2 = 0;
    } else {
        b2 = 0;
        d2 = -p2;
    }
    i = dshift(d, d2);
    b2 += i;
    if b2 > 0 {
        b = lshift(b, b2);
    }
    d2 += i;
    if d2 > 0 {
        d = lshift(d, d2);
    }
    dig = quorem(b, d);
    if dig == 0 {
        b = multadd(b, 10, 0);
        dig = quorem(b, d);
    }
    'ret: {
        i = 0;
        while i < nd0 {
            dd = (*s0.add(i as usize) as c_int) - ('0' as c_int) - dig;
            i += 1;
            if dd != 0 {
                break 'ret;
            }
            if *bx(b) == 0 && (*b).wds == 1 {
                if i < nd {
                    dd = 1;
                }
                break 'ret;
            }
            b = multadd(b, 10, 0);
            dig = quorem(b, d);
        }
        j = (*bc).dp1;
        loop {
            let old_i = i;
            i += 1;
            if !(old_i < nd) {
                break;
            }
            dd = (*s0.add(j as usize) as c_int) - ('0' as c_int) - dig;
            j += 1;
            if dd != 0 {
                break 'ret;
            }
            if *bx(b) == 0 && (*b).wds == 1 {
                if i < nd {
                    dd = 1;
                }
                break 'ret;
            }
            b = multadd(b, 10, 0);
            dig = quorem(b, d);
        }
        if dig > 0 || *bx(b) != 0 || (*b).wds > 1 {
            dd = -1;
        }
    }
    /* ret: */
    Bfree(b);
    Bfree(d);
    if speccase != 0 {
        if dd <= 0 {
            (*rv).set_dval(0.0);
        }
        return;
    }

    let mut retlow1 = false;
    let mut rethi1 = false;
    if dd < 0 {
        if dsign == 0 {
            retlow1 = true;
        }
    } else if dd > 0 {
        if dsign != 0 {
            rethi1 = true;
        }
    } else {
        let mut odd = false;
        j = ((((*rv).w0() & Exp_mask) >> Exp_shift) as c_int) - (*bc).scale;
        if j <= 0 {
            i = 1 - j;
            if i <= 31 {
                if ((*rv).w1() & (0x1u32 << i)) != 0 {
                    odd = true;
                }
            } else if ((*rv).w0() & (0x1u32 << (i - 32))) != 0 {
                odd = true;
            }
        } else if ((*rv).w1() & 1) != 0 {
            odd = true;
        }
        if odd {
            if dsign != 0 {
                rethi1 = true;
            } else {
                retlow1 = true;
            }
        }
    }
    if rethi1 {
        let v = (*rv).dval() + sulp(rv, bc);
        (*rv).set_dval(v);
    } else if retlow1 {
        let v = (*rv).dval() - sulp(rv, bc);
        (*rv).set_dval(v);
    }
}
