//! Translation of the `strtod`/`bigcomp` half of `src/dtoa.c`.
//!
//! In this build `strtod` is renamed to `strtod__unused` by the surrounding
//! project (jansson uses the C library's `strtod`), but the symbol is still
//! exported, so the code is translated faithfully.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use crate::dtoa::*;
use crate::dtoa_tables::*;
use crate::types::{set_errno, ERANGE};
use core::ffi::{c_char, c_int};
use core::ptr::null_mut;

/* ------------------------------------------------------------------- sulp */

unsafe fn sulp(x: *mut U, bc: *mut BCinfo) -> f64 {
    let mut u = U { LL: 0 };
    let rv: f64;
    let i: c_int;

    rv = ulp(x);
    i = 2 * 53 + 1 - ((((*x).L[1] & 0x7ff00000) >> 20) as c_int);
    if (*bc).scale == 0 || i <= 0 {
        return rv;
    }
    u.L[1] = (0x3ff00000u32).wrapping_add((i as u32) << 20);
    u.L[0] = 0;
    rv * u.d
}

/* ---------------------------------------------------------------- bigcomp */

unsafe fn bigcomp(rv: *mut U, s0: *const c_char, bc: *mut BCinfo) {
    let mut b: *mut Bigint;
    let mut d: *mut Bigint;
    let mut b2: c_int;
    let mut bbits: c_int = 0;
    let mut d2: c_int;
    let mut dd: c_int = 0;
    let mut dig: c_int;
    let mut dsign: c_int;
    let mut i: c_int = 0;
    let mut j: c_int = 0;
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
    if (*rv).d == 0.0 {
        b = i2b(1);
        p2 = -1022 - 53 + 1;
        bbits = 1;
        (*rv).L[1] = (53 + 2) << 20;
        i = 0;
        speccase = 1;
        p2 -= 1;
        dsign = 0;
        have_i = true;
    } else {
        b = d2b(rv, &mut p2, &mut bbits);
        i = 0;
    }

    if !have_i {
        p2 -= (*bc).scale;
        i = 53 - bbits;
        j = 53 - (-1022) - 1 + p2;
        if i > j {
            i = j;
        }
        i += 1;
        b = lshift(b, i);
        *bx(b).add(0) |= 1;
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
            dd = (*s0.add(i as usize) as c_int) - '0' as c_int - dig;
            i += 1;
            if dd != 0 {
                break 'ret;
            }
            if *bx(b).add(0) == 0 && (*b).wds == 1 {
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
            dd = (*s0.add(j as usize) as c_int) - '0' as c_int - dig;
            j += 1;
            if dd != 0 {
                break 'ret;
            }
            if *bx(b).add(0) == 0 && (*b).wds == 1 {
                if i < nd {
                    dd = 1;
                }
                break 'ret;
            }
            b = multadd(b, 10, 0);
            dig = quorem(b, d);
        }
        if dig > 0 || *bx(b).add(0) != 0 || (*b).wds > 1 {
            dd = -1;
        }
    }

    /* ret: */
    Bfree(b);
    Bfree(d);

    let mut retlow1 = false;
    let mut rethi1 = false;

    if speccase != 0 {
        if dd <= 0 {
            (*rv).d = 0.0;
        }
    } else if dd < 0 {
        if dsign == 0 {
            retlow1 = true;
        }
    } else if dd > 0 {
        if dsign != 0 {
            rethi1 = true;
        }
    } else {
        let mut odd = false;
        j = ((((*rv).L[1] & 0x7ff00000) >> 20) as c_int) - (*bc).scale;
        if j <= 0 {
            i = 1 - j;
            if i <= 31 {
                if (*rv).L[0] & (1u32 << i) != 0 {
                    odd = true;
                }
            } else if (*rv).L[1] & (1u32 << (i - 32)) != 0 {
                odd = true;
            }
        } else if (*rv).L[0] & 1 != 0 {
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
        (*rv).d += sulp(rv, bc);
    } else if retlow1 {
        (*rv).d -= sulp(rv, bc);
    }
}

/* --------------------------------------------------------- strtod__unused */

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sd {
    Denormal,
    Denormal1,
    Tiniest,
    SmallestNormal,
    Roundup,
    Roundup1,
    Noround,
    Noround1,
    ManyDigits,
    Ovfl,
    Undfl,
    RangeErr,
    Ret,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod__unused(s00: *const c_char, se: *mut *mut c_char) -> f64 {
    let mut s00 = s00;

    let mut bb2: c_int = 0;
    let mut bb5: c_int = 0;
    let mut bbe: c_int = 0;
    let mut bd2: c_int = 0;
    let mut bd5: c_int = 0;
    let mut bbbits: c_int = 0;
    let mut bs2: c_int = 0;
    let mut c: c_int = 0;
    let mut e: c_int = 0;
    let mut e1: c_int = 0;
    let mut esign: c_int = 0;
    let mut i: c_int;
    let mut j: c_int;
    let mut k: c_int = 0;
    let mut nd: c_int = 0;
    let mut nd0: c_int = 0;
    let mut nf: c_int = 0;
    let mut nz: c_int = 0;
    let mut nz0: c_int = 0;
    let mut nz1: c_int = 0;
    let mut sign: c_int = 0;
    let mut s: *const c_char = core::ptr::null();
    let mut s0: *const c_char = core::ptr::null();
    let mut s1: *const c_char = core::ptr::null();
    let mut aadj: f64 = 0.0;
    let mut aadj1: f64 = 0.0;
    let mut L: c_int = 0;
    let mut aadj2 = U { LL: 0 };
    let mut adj = U { LL: 0 };
    let mut rv = U { LL: 0 };
    let mut rv0 = U { LL: 0 };
    let mut y: u32 = 0;
    let mut z: u32 = 0;
    let mut bc = BCinfo {
        dp0: 0,
        dp1: 0,
        dplen: 0,
        dsign: 0,
        e0: 0,
        inexact: 0,
        nd: 0,
        nd0: 0,
        rounding: 0,
        scale: 0,
        uflchk: 0,
    };
    let mut bb: *mut Bigint = null_mut();
    let mut bb1: *mut Bigint = null_mut();
    let mut bd: *mut Bigint = null_mut();
    let mut bd0: *mut Bigint = null_mut();
    let mut bs: *mut Bigint = null_mut();
    let mut delta: *mut Bigint = null_mut();
    let mut bhi: u64 = 0;
    let mut blo: u64 = 0;
    let mut brv: u64 = 0;
    let mut t00: u64 = 0;
    let mut t01: u64 = 0;
    let mut t02: u64 = 0;
    let mut t10: u64 = 0;
    let mut t11: u64 = 0;
    let mut terv: u64 = 0;
    let mut tg: u64 = 0;
    let mut tlo: u64 = 0;
    let mut yz: u64 = 0;
    let mut p10: *const BF96 = core::ptr::null();
    let mut bexact: c_int = 0;
    let mut erv: c_int = 0;
    let mut Lsb: u32 = 0;
    let mut Lsb1: u32 = 0;
    let mut req_bigcomp: c_int = 0;

    sign = 0;
    nz0 = 0;
    nz1 = 0;
    nz = 0;
    bc.dplen = 0;
    bc.uflchk = 0;
    rv.d = 0.0;

    let mut st = Sd::Ret;

    'linear: {
        /* --- leading sign and whitespace ------------------------------- */
        s = s00;
        let mut goto_ret0 = false;
        loop {
            let ch = *s as u8;
            match ch {
                b'-' | b'+' => {
                    if ch == b'-' {
                        sign = 1;
                    }
                    s = s.add(1);
                    if *s != 0 {
                        break;
                    }
                    goto_ret0 = true;
                    break;
                }
                0 => {
                    goto_ret0 = true;
                    break;
                }
                b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b' ' => {
                    s = s.add(1);
                    continue;
                }
                _ => break,
            }
        }
        if goto_ret0 {
            /* ret0: */
            s = s00;
            sign = 0;
            break 'linear;
        }

        /* break2: */
        if *s as u8 == b'0' {
            let c1 = *s.add(1) as u8;
            if c1 == b'x' || c1 == b'X' {
                gethex(&mut s, &mut rv, 1, sign);
                break 'linear;
            }
            nz0 = 1;
            loop {
                s = s.add(1);
                if *s as u8 != b'0' {
                    break;
                }
            }
            if *s == 0 {
                break 'linear;
            }
        }
        s0 = s;
        nd = 0;
        nf = 0;
        yz = 0;
        loop {
            c = *s as c_int;
            if !(c >= '0' as c_int && c <= '9' as c_int) {
                break;
            }
            if nd < 19 {
                yz = 10u64.wrapping_mul(yz).wrapping_add((c - '0' as c_int) as u64);
            }
            nd += 1;
            s = s.add(1);
        }
        nd0 = nd;
        bc.dp0 = s.offset_from(s0) as c_int;
        bc.dp1 = bc.dp0;
        s1 = s;
        while s1 > s0 {
            s1 = s1.sub(1);
            if *s1 as u8 != b'0' {
                break;
            }
            nz1 += 1;
        }

        let mut goto_have_dig = false;
        let mut goto_dig_done = false;
        if c == '.' as c_int {
            s = s.add(1);
            c = *s as c_int;
            bc.dp1 = s.offset_from(s0) as c_int;
            bc.dplen = bc.dp1 - bc.dp0;
            if nd == 0 {
                while c == '0' as c_int {
                    nz += 1;
                    s = s.add(1);
                    c = *s as c_int;
                }
                if c > '0' as c_int && c <= '9' as c_int {
                    bc.dp0 = s0.offset_from(s) as c_int;
                    bc.dp1 = bc.dp0 + bc.dplen;
                    s0 = s;
                    nf += nz;
                    nz = 0;
                    goto_have_dig = true;
                } else {
                    goto_dig_done = true;
                }
            }
            if !goto_dig_done {
                let mut first = goto_have_dig;
                loop {
                    if !first && !(c >= '0' as c_int && c <= '9' as c_int) {
                        break;
                    }
                    first = false;
                    /* have_dig: */
                    nz += 1;
                    c -= '0' as c_int;
                    if c != 0 {
                        nf += nz;
                        i = 1;
                        while i < nz {
                            nd += 1;
                            if nd <= 19 {
                                yz = yz.wrapping_mul(10);
                            }
                            i += 1;
                        }
                        nd += 1;
                        if nd <= 19 {
                            yz = 10u64.wrapping_mul(yz).wrapping_add(c as u64);
                        }
                        nz = 0;
                        nz1 = 0;
                    }
                    s = s.add(1);
                    c = *s as c_int;
                }
            }
        }

        /* dig_done: */
        e = 0;
        if c == 'e' as c_int || c == 'E' as c_int {
            if nd == 0 && nz == 0 && nz0 == 0 {
                /* ret0 */
                s = s00;
                sign = 0;
                break 'linear;
            }
            s00 = s;
            esign = 0;
            s = s.add(1);
            c = *s as c_int;
            if c == '-' as c_int {
                esign = 1;
                s = s.add(1);
                c = *s as c_int;
            } else if c == '+' as c_int {
                s = s.add(1);
                c = *s as c_int;
            }
            if c >= '0' as c_int && c <= '9' as c_int {
                while c == '0' as c_int {
                    s = s.add(1);
                    c = *s as c_int;
                }
                if c > '0' as c_int && c <= '9' as c_int {
                    L = c - '0' as c_int;
                    loop {
                        s = s.add(1);
                        c = *s as c_int;
                        if !(c >= '0' as c_int && c <= '9' as c_int) {
                            break;
                        }
                        if L <= 19999 {
                            L = 10 * L + c - '0' as c_int;
                        }
                    }
                    if L > 19999 {
                        e = 19999;
                    } else {
                        e = L;
                    }
                    if esign != 0 {
                        e = -e;
                    }
                } else {
                    e = 0;
                }
            } else {
                s = s00;
            }
        }

        if nd == 0 {
            if nz == 0 && nz0 == 0 {
                let mut handled = false;
                if bc.dplen == 0 {
                    if c == 'i' as c_int || c == 'I' as c_int {
                        if matchstr(&mut s, b"nf\0".as_ptr() as *const c_char) != 0 {
                            s = s.sub(1);
                            if matchstr(&mut s, b"inity\0".as_ptr() as *const c_char) == 0 {
                                s = s.add(1);
                            }
                            rv.L[1] = 0x7ff00000;
                            rv.L[0] = 0;
                            handled = true;
                        }
                    } else if c == 'n' as c_int || c == 'N' as c_int {
                        if matchstr(&mut s, b"an\0".as_ptr() as *const c_char) != 0 {
                            rv.L[1] = 0x7ff80000;
                            rv.L[0] = 0;
                            if *s as u8 == b'(' {
                                hexnan(&mut rv, &mut s);
                            }
                            handled = true;
                        }
                    }
                }
                if handled {
                    break 'linear;
                }
                /* ret0: */
                s = s00;
                sign = 0;
            }
            break 'linear;
        }

        e -= nf;
        e1 = e;
        bc.e0 = e;
        if nd0 == 0 {
            nd0 = nd;
        }
        bd0 = null_mut();

        if nd <= 15 {
            rv.d = yz as f64;
            if e == 0 {
                break 'linear;
            }
            if e > 0 {
                if e <= 22 {
                    rv.d *= TENS[e as usize];
                    break 'linear;
                }
                i = 15 - nd;
                if e <= 22 + i {
                    e -= i;
                    rv.d *= TENS[i as usize];
                    rv.d *= TENS[e as usize];
                    break 'linear;
                }
            } else if e >= -22 {
                rv.d /= TENS[(-e) as usize];
                break 'linear;
            }
        }

        k = if nd < 19 { nd } else { 19 };
        e1 += nd - k;
        i = e1 + 342;
        if i < 0 {
            st = Sd::Undfl;
            break 'linear;
        }
        if i > 650 {
            st = Sd::Ovfl;
            break 'linear;
        }
        p10 = PTEN.as_ptr().add(i as usize);
        brv = yz;
        i = 0;
        if brv & 0xffffffff00000000 == 0 {
            i = 32;
            brv <<= 32;
        }
        if brv & 0xffff000000000000 == 0 {
            i += 16;
            brv <<= 16;
        }
        if brv & 0xff00000000000000 == 0 {
            i += 8;
            brv <<= 8;
        }
        if brv & 0xf000000000000000 == 0 {
            i += 4;
            brv <<= 4;
        }
        if brv & 0xc000000000000000 == 0 {
            i += 2;
            brv <<= 2;
        }
        if brv & 0x8000000000000000 == 0 {
            i += 1;
            brv <<= 1;
        }
        erv = (64 + 0x3fe) + (*p10).e - i;
        if erv <= 0 && nd > 19 {
            st = Sd::ManyDigits;
            break 'linear;
        }
        bhi = brv >> 32;
        blo = brv & 0xffffffff;
        t01 = bhi.wrapping_mul((*p10).b1 as u64);
        t10 = blo
            .wrapping_mul((*p10).b0 as u64)
            .wrapping_add(t01 & 0xffffffff);
        t00 = bhi
            .wrapping_mul((*p10).b0 as u64)
            .wrapping_add(t01 >> 32)
            .wrapping_add(t10 >> 32);

        /* The original uses `1 << i` (int arithmetic) in the two `nd > 19`
           pre-checks and `1ull << i` (64-bit) in the two later checks; keep
           both forms distinct. */
        let one_shl_i_int = (1i32.wrapping_shl(i as u32)) as i64 as u64;
        let one_shl_i_ll = 1u64.wrapping_shl(i as u32);

        if t00 & 0x8000000000000000 != 0 {
            if (t00 & 0x3ff) != 0 && ((!t00) & 0x3fe) != 0 {
                if nd > 19
                    && ((t00.wrapping_add(one_shl_i_int).wrapping_add(2) & 0x400) ^ (t00 & 0x400)) != 0
                {
                    st = Sd::ManyDigits;
                    break 'linear;
                }
                if erv <= 0 {
                    st = Sd::Denormal;
                    break 'linear;
                }
                if t00 & 0x400 != 0 && t00 & 0xbff != 0 {
                    st = Sd::Roundup;
                    break 'linear;
                }
                st = Sd::Noround;
                break 'linear;
            }
        } else if (t00 & 0x1ff) != 0 && ((!t00) & 0x1fe) != 0 {
            if nd > 19
                && ((t00.wrapping_add(one_shl_i_int).wrapping_add(2) & 0x200) ^ (t00 & 0x200)) != 0
            {
                st = Sd::ManyDigits;
                break 'linear;
            }
            if erv <= 1 {
                st = Sd::Denormal1;
                break 'linear;
            }
            if t00 & 0x200 != 0 {
                st = Sd::Roundup1;
                break 'linear;
            }
            st = Sd::Noround1;
            break 'linear;
        }

        t02 = bhi.wrapping_mul((*p10).b2 as u64);
        t11 = blo
            .wrapping_mul((*p10).b1 as u64)
            .wrapping_add(t02 & 0xffffffff);
        bexact = 1;
        if e1 < 0 || e1 > 41 || (t10 | t11) & 0xffffffff != 0 || nd > 19 {
            bexact = 0;
        }
        tlo = (t10 & 0xffffffff)
            .wrapping_add(t02 >> 32)
            .wrapping_add(t11 >> 32);
        if bexact == 0 && (tlo.wrapping_add(0x10)) >> 32 > tlo >> 32 {
            st = Sd::ManyDigits;
            break 'linear;
        }
        t00 = t00.wrapping_add(tlo >> 32);

        if t00 & 0x8000000000000000 != 0 {
            if erv <= 0 {
                if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x3ff)) == 0 {
                    st = Sd::ManyDigits;
                    break 'linear;
                }
                st = Sd::Denormal;
                break 'linear;
            }
            if bexact != 0 {
                if t00 & 0x400 != 0 && ((tlo & 0xffffffff) | (t00 & 0xbff)) != 0 {
                    st = Sd::Roundup;
                    break 'linear;
                }
                st = Sd::Noround;
                break 'linear;
            }
            if ((tlo & 0xfffffff0) | (t00 & 0x3ff)) != 0
                && (nd <= 19
                    || (t00.wrapping_add(one_shl_i_ll) & 0xfffffffffffffc00)
                        == (t00 & 0xfffffffffffffc00))
            {
                if t00 & 0x400 != 0 {
                    st = Sd::Roundup;
                    break 'linear;
                }
                st = Sd::Noround;
                break 'linear;
            }
        } else {
            if erv <= 1 {
                if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x1ff)) == 0 {
                    st = Sd::ManyDigits;
                    break 'linear;
                }
                st = Sd::Denormal1;
                break 'linear;
            }
            if bexact != 0 {
                if t00 & 0x200 != 0 && (t00 & 0x5ff != 0 || tlo != 0) {
                    st = Sd::Roundup1;
                    break 'linear;
                }
                st = Sd::Noround1;
                break 'linear;
            }
            if ((tlo & 0xfffffff0) | (t00 & 0x1ff)) != 0
                && (nd <= 19
                    || (t00.wrapping_add(one_shl_i_ll) & 0x7ffffffffffffe00)
                        == (t00 & 0x7ffffffffffffe00))
            {
                if t00 & 0x200 != 0 {
                    st = Sd::Roundup1;
                    break 'linear;
                }
                st = Sd::Noround1;
                break 'linear;
            }
        }

        st = Sd::ManyDigits;
    }

    'sm: loop {
        match st {
            Sd::Denormal => {
                if erv <= -52 {
                    if erv < -52 || (t00 & 0x7fffffffffffffff) == 0 {
                        st = Sd::Undfl;
                        continue 'sm;
                    }
                    st = Sd::Tiniest;
                    continue 'sm;
                }
                tg = 1u64 << (11 - erv);
                t00 &= !(tg - 1);
                if t00 & tg != 0 {
                    t00 = t00.wrapping_add(tg << 1);
                    if t00 & 0x8000000000000000 == 0 {
                        erv += 1;
                        if erv > 0 {
                            st = Sd::SmallestNormal;
                            continue 'sm;
                        }
                        t00 = 0x8000000000000000;
                    }
                }
                rv.LL = t00 >> (12 - erv);
                set_errno(ERANGE);
                st = Sd::Ret;
            }

            Sd::Denormal1 => {
                if erv <= -51 {
                    if erv < -51 || (t00 & 0x3fffffffffffffff) == 0 {
                        st = Sd::Undfl;
                        continue 'sm;
                    }
                    st = Sd::Tiniest;
                    continue 'sm;
                }
                tg = 1u64 << (11 - erv);
                if t00 & tg != 0 {
                    t00 = t00.wrapping_add(tg << 1);
                    if 0x8000000000000000u64 & t00 != 0 && erv == 1 {
                        st = Sd::SmallestNormal;
                        continue 'sm;
                    }
                }
                if erv <= -52 {
                    st = Sd::Undfl;
                    continue 'sm;
                }
                rv.LL = t00 >> (12 - erv);
                set_errno(ERANGE);
                st = Sd::Ret;
            }

            Sd::Tiniest => {
                rv.LL = 1;
                set_errno(ERANGE);
                st = Sd::Ret;
            }

            Sd::SmallestNormal => {
                rv.LL = 0x0010000000000000;
                st = Sd::Ret;
            }

            Sd::Roundup => {
                t00 = t00.wrapping_add(0x800);
                if t00 & 0x8000000000000000 == 0 {
                    if erv >= 0x7fe {
                        st = Sd::Ovfl;
                        continue 'sm;
                    }
                    terv = (erv + 1) as u64;
                    rv.LL = terv << 52;
                    st = Sd::Ret;
                    continue 'sm;
                }
                st = Sd::Noround;
            }

            Sd::Noround => {
                if erv >= 0x7ff {
                    st = Sd::Ovfl;
                    continue 'sm;
                }
                terv = erv as u64;
                rv.LL = (terv << 52) | ((t00 & 0x7ffffffffffff800) >> 11);
                st = Sd::Ret;
            }

            Sd::Roundup1 => {
                t00 = t00.wrapping_add(0x400);
                if t00 & 0x4000000000000000 == 0 {
                    if erv >= 0x7ff {
                        st = Sd::Ovfl;
                        continue 'sm;
                    }
                    terv = erv as u64;
                    rv.LL = terv << 52;
                    st = Sd::Ret;
                    continue 'sm;
                }
                st = Sd::Noround1;
            }

            Sd::Noround1 => {
                if erv >= 0x800 {
                    st = Sd::Ovfl;
                    continue 'sm;
                }
                terv = (erv - 1) as u64;
                rv.LL = (terv << 52) | ((t00 & 0x3ffffffffffffc00) >> 10);
                st = Sd::Ret;
            }

            Sd::Ovfl => {
                rv.L[1] = 0x7ff00000;
                rv.L[0] = 0;
                st = Sd::RangeErr;
            }

            Sd::Undfl => {
                rv.d = 0.0;
                st = Sd::RangeErr;
            }

            Sd::RangeErr => {
                if !bd0.is_null() {
                    Bfree(bb);
                    Bfree(bd);
                    Bfree(bs);
                    Bfree(bd0);
                    Bfree(delta);
                }
                set_errno(ERANGE);
                st = Sd::Ret;
            }

            Sd::ManyDigits => {
                if nd > 17 {
                    if nd > 18 {
                        yz /= 100;
                        e1 += 2;
                    } else {
                        yz /= 10;
                        e1 += 1;
                    }
                    y = (yz / 100000000) as u32;
                } else if nd > 9 {
                    i = nd - 9;
                    y = ((yz >> i) / PFIVE[(i - 1) as usize]) as u32;
                } else {
                    y = yz as u32;
                }
                rv.d = yz as f64;
                bc.scale = 0;
                if e1 > 0 {
                    i = e1 & 15;
                    if i != 0 {
                        rv.d *= TENS[i as usize];
                    }
                    e1 &= !15;
                    if e1 != 0 {
                        if e1 > 308 {
                            st = Sd::Ovfl;
                            continue 'sm;
                        }
                        e1 >>= 4;
                        j = 0;
                        while e1 > 1 {
                            if e1 & 1 != 0 {
                                rv.d *= BIGTENS[j as usize];
                            }
                            j += 1;
                            e1 >>= 1;
                        }
                        rv.L[1] = rv.L[1].wrapping_sub(53 * 0x100000);
                        rv.d *= BIGTENS[j as usize];
                        z = rv.L[1] & 0x7ff00000;
                        if z > 0x100000 * (1024 + 1023 - 53) {
                            st = Sd::Ovfl;
                            continue 'sm;
                        }
                        if z > 0x100000 * (1024 + 1023 - 1 - 53) {
                            rv.L[1] = 0xfffff | 0x100000 * (1024 + 1023 - 1);
                            rv.L[0] = 0xffffffff;
                        } else {
                            rv.L[1] = rv.L[1].wrapping_add(53 * 0x100000);
                        }
                    }
                } else if e1 < 0 {
                    e1 = -e1;
                    i = e1 & 15;
                    if i != 0 {
                        rv.d /= TENS[i as usize];
                    }
                    e1 >>= 4;
                    if e1 != 0 {
                        if e1 >= 1 << 5 {
                            st = Sd::Undfl;
                            continue 'sm;
                        }
                        if e1 & 0x10 != 0 {
                            bc.scale = 2 * 53;
                        }
                        j = 0;
                        while e1 > 0 {
                            if e1 & 1 != 0 {
                                rv.d *= TINYTENS[j as usize];
                            }
                            j += 1;
                            e1 >>= 1;
                        }
                        if bc.scale != 0 {
                            j = 2 * 53 + 1 - (((rv.L[1] & 0x7ff00000) >> 20) as c_int);
                            if j > 0 {
                                if j >= 32 {
                                    if j > 54 {
                                        st = Sd::Undfl;
                                        continue 'sm;
                                    }
                                    rv.L[0] = 0;
                                    if j >= 53 {
                                        rv.L[1] = (53 + 2) * 0x100000;
                                    } else {
                                        rv.L[1] &= 0xffffffffu32 << (j - 32);
                                    }
                                } else {
                                    rv.L[0] &= 0xffffffffu32 << j;
                                }
                            }
                        }
                        if rv.d == 0.0 {
                            st = Sd::Undfl;
                            continue 'sm;
                        }
                    }
                }
                bc.nd = nd - nz1;
                bc.nd0 = nd0;
                if nd > 40 {
                    i = 18;
                    j = 18;
                    if i > nd0 {
                        j += bc.dplen;
                    }
                    loop {
                        j -= 1;
                        if j < bc.dp1 && j >= bc.dp0 {
                            j = bc.dp0 - 1;
                        }
                        if *s0.add(j as usize) as u8 != b'0' {
                            break;
                        }
                        i -= 1;
                    }
                    e += nd - i;
                    nd = i;
                    if nd0 > nd {
                        nd0 = nd;
                    }
                    if nd < 9 {
                        y = 0;
                        i = 0;
                        while i < nd0 {
                            y = 10u32
                                .wrapping_mul(y)
                                .wrapping_add((*s0.add(i as usize) as u32).wrapping_sub(b'0' as u32));
                            i += 1;
                        }
                        j = bc.dp1;
                        while i < nd {
                            y = 10u32
                                .wrapping_mul(y)
                                .wrapping_add((*s0.add(j as usize) as u32).wrapping_sub(b'0' as u32));
                            j += 1;
                            i += 1;
                        }
                    }
                }
                bd0 = s2b(s0, nd0, nd, y, bc.dplen);

                let mut term: Option<Sd> = None;
                'bigloop: loop {
                    bd = Balloc((*bd0).k);
                    Bcopy(bd, bd0);
                    bb = d2b(&mut rv, &mut bbe, &mut bbbits);
                    bs = i2b(1);
                    if e >= 0 {
                        bb2 = 0;
                        bb5 = 0;
                        bd2 = e;
                        bd5 = e;
                    } else {
                        bb2 = -e;
                        bb5 = -e;
                        bd2 = 0;
                        bd5 = 0;
                    }
                    if bbe >= 0 {
                        bb2 += bbe;
                    } else {
                        bd2 -= bbe;
                    }
                    bs2 = bb2;
                    Lsb = 1;
                    Lsb1 = 0;
                    j = bbe - bc.scale;
                    i = j + bbbits - 1;
                    j = 53 + 1 - bbbits;
                    if i < -1022 {
                        i = -1022 - i;
                        j -= i;
                        if i < 32 {
                            Lsb <<= i;
                        } else if i < 52 {
                            Lsb1 = Lsb << (i - 32);
                        } else {
                            Lsb1 = 0x7ff00000;
                        }
                    }
                    bb2 += j;
                    bd2 += j;
                    bd2 += bc.scale;
                    i = if bb2 < bd2 { bb2 } else { bd2 };
                    if i > bs2 {
                        i = bs2;
                    }
                    if i > 0 {
                        bb2 -= i;
                        bd2 -= i;
                        bs2 -= i;
                    }
                    if bb5 > 0 {
                        bs = pow5mult(bs, bb5);
                        bb1 = mult(bs, bb);
                        Bfree(bb);
                        bb = bb1;
                    }
                    if bb2 > 0 {
                        bb = lshift(bb, bb2);
                    }
                    if bd5 > 0 {
                        bd = pow5mult(bd, bd5);
                    }
                    if bd2 > 0 {
                        bd = lshift(bd, bd2);
                    }
                    if bs2 > 0 {
                        bs = lshift(bs, bs2);
                    }
                    delta = diff(bb, bd);
                    bc.dsign = (*delta).sign;
                    (*delta).sign = 0;
                    i = cmp(delta, bs);
                    if bc.nd > nd && i <= 0 {
                        if bc.dsign != 0 {
                            req_bigcomp = 1;
                            break 'bigloop;
                        }
                        i = -1;
                    }
                    if i < 0 {
                        if bc.dsign != 0
                            || rv.L[0] != 0
                            || rv.L[1] & 0xfffff != 0
                            || rv.L[1] & 0x7ff00000 <= (2 * 53 + 1) * 0x100000
                        {
                            break 'bigloop;
                        }
                        if *bx(delta).add(0) == 0 && (*delta).wds <= 1 {
                            break 'bigloop;
                        }
                        delta = lshift(delta, 1);
                        if cmp(delta, bs) > 0 {
                            /* goto drop_down */
                            if bc.scale != 0 {
                                L = (rv.L[1] & 0x7ff00000) as c_int;
                                if L <= (2 * 53 + 1) * 0x100000 {
                                    if L > (53 + 2) * 0x100000 {
                                        break 'bigloop;
                                    }
                                    if bc.nd > nd {
                                        bc.uflchk = 1;
                                        break 'bigloop;
                                    }
                                    term = Some(Sd::Undfl);
                                    break 'bigloop;
                                }
                            }
                            L = ((rv.L[1] & 0x7ff00000) as c_int) - 0x100000;
                            rv.L[1] = (L as u32) | 0xfffff;
                            rv.L[0] = 0xffffffff;
                            if bc.nd > nd {
                                /* goto cont */
                                Bfree(bb);
                                Bfree(bd);
                                Bfree(bs);
                                Bfree(delta);
                                continue 'bigloop;
                            }
                            break 'bigloop;
                        }
                        break 'bigloop;
                    }
                    if i == 0 {
                        let mut drop_down = false;
                        let done = false;
                        if bc.dsign != 0 {
                            let lim = if bc.scale != 0 && {
                                y = rv.L[1] & 0x7ff00000;
                                y <= 2 * 53 * 0x100000
                            } {
                                0xffffffffu32 & (0xffffffffu32 << (2 * 53 + 1 - (y >> 20)))
                            } else {
                                0xffffffff
                            };
                            if (rv.L[1] & 0xfffff) == 0xfffff && rv.L[0] == lim {
                                if rv.L[1] == (0xfffff | 0x100000 * (1024 + 1023 - 1))
                                    && rv.L[0] == 0xffffffff
                                {
                                    term = Some(Sd::Ovfl);
                                    break 'bigloop;
                                }
                                rv.L[1] = (rv.L[1] & 0x7ff00000).wrapping_add(0x100000);
                                rv.L[0] = 0;
                                bc.dsign = 0;
                                break 'bigloop;
                            }
                        } else if rv.L[1] & 0xfffff == 0 && rv.L[0] == 0 {
                            drop_down = true;
                        }
                        if drop_down {
                            if bc.scale != 0 {
                                L = (rv.L[1] & 0x7ff00000) as c_int;
                                if L <= (2 * 53 + 1) * 0x100000 {
                                    if L > (53 + 2) * 0x100000 {
                                        break 'bigloop;
                                    }
                                    if bc.nd > nd {
                                        bc.uflchk = 1;
                                        break 'bigloop;
                                    }
                                    term = Some(Sd::Undfl);
                                    break 'bigloop;
                                }
                            }
                            L = ((rv.L[1] & 0x7ff00000) as c_int) - 0x100000;
                            rv.L[1] = (L as u32) | 0xfffff;
                            rv.L[0] = 0xffffffff;
                            if bc.nd > nd {
                                Bfree(bb);
                                Bfree(bd);
                                Bfree(bs);
                                Bfree(delta);
                                continue 'bigloop;
                            }
                            break 'bigloop;
                        }
                        if Lsb1 != 0 {
                            if rv.L[1] & Lsb1 == 0 {
                                break 'bigloop;
                            }
                        } else if rv.L[0] & Lsb == 0 {
                            break 'bigloop;
                        }
                        if bc.dsign != 0 {
                            rv.d += sulp(&mut rv, &mut bc);
                        } else {
                            rv.d -= sulp(&mut rv, &mut bc);
                            if rv.d == 0.0 {
                                if bc.nd > nd {
                                    bc.uflchk = 1;
                                    break 'bigloop;
                                }
                                term = Some(Sd::Undfl);
                                break 'bigloop;
                            }
                        }
                        bc.dsign = 1 - bc.dsign;
                        let _ = done;
                        break 'bigloop;
                    }
                    aadj = ratio(delta, bs);
                    if aadj <= 2.0 {
                        if bc.dsign != 0 {
                            aadj = 1.0;
                            aadj1 = 1.0;
                        } else if rv.L[0] != 0 || rv.L[1] & 0xfffff != 0 {
                            if rv.L[0] == 1 && rv.L[1] == 0 {
                                if bc.nd > nd {
                                    bc.uflchk = 1;
                                    break 'bigloop;
                                }
                                term = Some(Sd::Undfl);
                                break 'bigloop;
                            }
                            aadj = 1.0;
                            aadj1 = -1.0;
                        } else {
                            if aadj < 2.0 / 2.0 {
                                aadj = 1.0 / 2.0;
                            } else {
                                aadj *= 0.5;
                            }
                            aadj1 = -aadj;
                        }
                    } else {
                        aadj *= 0.5;
                        aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                    }
                    y = rv.L[1] & 0x7ff00000;
                    if y == 0x100000 * (1024 + 1023 - 1) {
                        rv0.d = rv.d;
                        rv.L[1] = rv.L[1].wrapping_sub(53 * 0x100000);
                        adj.d = aadj1 * ulp(&mut rv);
                        rv.d += adj.d;
                        if rv.L[1] & 0x7ff00000 >= 0x100000 * (1024 + 1023 - 53) {
                            if rv0.L[1] == (0xfffff | 0x100000 * (1024 + 1023 - 1))
                                && rv0.L[0] == 0xffffffff
                            {
                                term = Some(Sd::Ovfl);
                                break 'bigloop;
                            }
                            rv.L[1] = 0xfffff | 0x100000 * (1024 + 1023 - 1);
                            rv.L[0] = 0xffffffff;
                            Bfree(bb);
                            Bfree(bd);
                            Bfree(bs);
                            Bfree(delta);
                            continue 'bigloop;
                        } else {
                            rv.L[1] = rv.L[1].wrapping_add(53 * 0x100000);
                        }
                    } else if bc.scale != 0 && y <= 2 * 53 * 0x100000 {
                        if aadj <= 0x7fffffffu32 as f64 {
                            z = aadj as u32;
                            if z == 0 {
                                z = 1;
                            }
                            aadj = z as f64;
                            aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                        }
                        aadj2.d = aadj1;
                        aadj2.L[1] = aadj2.L[1]
                            .wrapping_add(((2 * 53 + 1) * 0x100000u32).wrapping_sub(y));
                        aadj1 = aadj2.d;
                        adj.d = aadj1 * ulp(&mut rv);
                        rv.d += adj.d;
                        if rv.d == 0.0 {
                            req_bigcomp = 1;
                            break 'bigloop;
                        }
                    } else {
                        adj.d = aadj1 * ulp(&mut rv);
                        rv.d += adj.d;
                    }
                    z = rv.L[1] & 0x7ff00000;
                    if bc.nd == nd && bc.scale == 0 && y == z {
                        L = aadj as c_int;
                        aadj -= L as f64;
                        if bc.dsign != 0 || rv.L[0] != 0 || rv.L[1] & 0xfffff != 0 {
                            if aadj < 0.4999999 || aadj > 0.5000001 {
                                break 'bigloop;
                            }
                        } else if aadj < 0.4999999 / 2.0 {
                            break 'bigloop;
                        }
                    }
                    /* cont: */
                    Bfree(bb);
                    Bfree(bd);
                    Bfree(bs);
                    Bfree(delta);
                }

                if let Some(t) = term {
                    st = t;
                    continue 'sm;
                }

                Bfree(bb);
                Bfree(bd);
                Bfree(bs);
                Bfree(bd0);
                Bfree(delta);
                if req_bigcomp != 0 {
                    bd0 = null_mut();
                    bc.e0 += nz1;
                    bigcomp(&mut rv, s0, &mut bc);
                    y = rv.L[1] & 0x7ff00000;
                    if y == 0x7ff00000 {
                        st = Sd::Ovfl;
                        continue 'sm;
                    }
                    if y == 0 && rv.d == 0.0 {
                        st = Sd::Undfl;
                        continue 'sm;
                    }
                }
                if bc.scale != 0 {
                    rv0.L[1] = 0x3ff00000 - 2 * 53 * 0x100000;
                    rv0.L[0] = 0;
                    rv.d *= rv0.d;
                    if rv.L[1] & 0x7ff00000 == 0 {
                        set_errno(ERANGE);
                    }
                }
                st = Sd::Ret;
            }

            Sd::Ret => break 'sm,
        }
    }

    if !se.is_null() {
        *se = s as *mut c_char;
    }
    if sign != 0 {
        -rv.d
    } else {
        rv.d
    }
}
