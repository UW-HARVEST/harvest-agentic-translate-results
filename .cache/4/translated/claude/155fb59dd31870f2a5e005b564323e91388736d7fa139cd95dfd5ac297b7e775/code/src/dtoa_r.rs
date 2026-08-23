#![allow(dead_code, non_snake_case, non_upper_case_globals, unused_assignments, unused_mut, unused_variables, unused_parens, unused_labels)]
use crate::dtoa::*;
use crate::dtoa_tables::*;
use std::ffi::{c_char, c_int};

/* Literal translation of dtoa_r() from David M. Gay's dtoa.c
 * (IEEE_8087, USE_BF96, long long available, no MULTIPLE_THREADS,
 *  dtoa_divmax == 2, Rounding == 1).
 *
 * The C function is a dense web of gotos; it is modelled here with an explicit
 * state machine whose states correspond one-for-one with the C labels.  Array
 * accesses to the constant tables use raw pointers so that, exactly as in C,
 * no bounds check is performed (pfive[k-1] with k == 0 is reachable in the C
 * source).
 */
#[derive(Copy, Clone, PartialEq, Eq)]
enum St {
    UseExact,
    NoDiv,
    UlpReached,
    Roundup,
    Retc,
    Ret1,
    NoDigits,
    OneDigit,
    Toobig,
    FastFailed,
    FastFailed1,
    Ret,
    Round9Up,
    Roundoff,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa_r(
    dd: f64,
    mode: c_int,
    ndigits: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
    buf: *mut c_char,
    blen: usize,
) -> *mut c_char {
    let mut mode: c_int = mode;
    let mut ndigits: c_int = ndigits;
    let mut buf: *mut c_char = buf;
    let mut blen: usize = blen;

    let mut bbits: c_int = 0;
    let mut b2: c_int = 0;
    let mut b5: c_int = 0;
    let mut be: c_int = 0;
    let mut dig: c_int = 0;
    let mut i: c_int = 0;
    let mut ilim: c_int = 0;
    let mut ilim1: c_int = 0;
    let mut j: c_int = 0;
    let mut j1: c_int = 0;
    let mut k: c_int = 0;
    let mut leftright: c_int = 0;
    let mut m2: c_int = 0;
    let mut m5: c_int = 0;
    let mut s2: c_int = 0;
    let mut s5: c_int = 0;
    let mut spec_case: c_int = 0;
    let mut denorm: c_int = 0;
    let mut b: *mut Bigint = std::ptr::null_mut();
    let mut b1: *mut Bigint = std::ptr::null_mut();
    let mut delta: *mut Bigint = std::ptr::null_mut();
    let mut mlo: *mut Bigint = std::ptr::null_mut();
    let mut mhi: *mut Bigint = std::ptr::null_mut();
    let mut S: *mut Bigint = std::ptr::null_mut();
    let mut u: U = U::new();
    let mut s: *mut c_char = std::ptr::null_mut();
    let mut p10: *const BF96 = std::ptr::null();
    let mut dbhi: u64 = 0;
    let mut dbits: u64 = 0;
    let mut dblo: u64 = 0;
    let mut den: u64 = 0;
    let mut hb: u64 = 0;
    let mut rb: u64 = 0;
    let mut rblo: u64 = 0;
    let mut res: u64 = 0;
    let mut res0: u64 = 0;
    let mut res3: u64 = 0;
    let mut reslo: u64 = 0;
    let mut sres: u64 = 0;
    let mut sulp: u64 = 0;
    let mut tv0: u64 = 0;
    let mut tv1: u64 = 0;
    let mut tv2: u64 = 0;
    let mut tv3: u64 = 0;
    let mut ulp: u64 = 0;
    let mut ulplo: u64 = 0;
    let mut ulpmask: u64 = 0;
    let mut ures: u64 = 0;
    let mut ureslo: u64 = 0;
    let mut zb: u64 = 0;
    let mut eulp: c_int = 0;
    let mut k1: c_int = 0;
    let mut n2: c_int = 0;
    let mut ulpadj: c_int = 0;
    let mut ulpshift: c_int = 0;

    /* Unchecked table accessors, mirroring C's array subscripting (no bounds
     * checks, so no Rust panic where C silently reads adjacent memory).
     *
     * Note on pfive(): the C source contains "pfive[k-1]" (see the
     * "ilim == 0 && j + k >= 0" test below), which is reached with k == 0 for
     * mode 3/5/7/9 and ndigits == -1 when 1 <= |d| < 10.  In the C library
     * pfive[] is preceded by zero padding, so pfive[-1] reads as 0; reproduce
     * that value here so the comparison takes the same branch. */
    let pfive = |n: c_int| -> u64 {
        if n < 0 {
            0
        } else {
            *PFIVE.as_ptr().offset(n as isize)
        }
    };
    let pfivebits = |n: c_int| -> c_int { *PFIVEBITS.as_ptr().offset(n as isize) };

    u.set_dval(dd);
    if (u.w0() & 0x80000000) != 0 {
        *sign = 1;
        u.set_w0(u.w0() & !0x80000000);
    } else {
        *sign = 0;
    }
    if (u.w0() & 0x7ff00000) == 0x7ff00000 {
        *decpt = 9999;
        if u.w1() == 0 && (u.w0() & 0xfffff) == 0 {
            return nrv_alloc(
                b"Infinity\0".as_ptr() as *const c_char,
                buf,
                blen,
                rve,
                8,
            );
        }
        return nrv_alloc(b"NaN\0".as_ptr() as *const c_char, buf, blen, rve, 3);
    }
    if u.dval() == 0.0 {
        *decpt = 1;
        return nrv_alloc(b"0\0".as_ptr() as *const c_char, buf, blen, rve, 1);
    }
    dbits = (u.LL & 0xfffffffffffffu64) << 11; /* fraction bits */
    be = (u.LL >> 52) as c_int; /* biased exponent; nonzero ==> normal */
    if be != 0 {
        dbits |= 0x8000000000000000u64;
        ulpadj = 0;
        denorm = 0;
    } else {
        denorm = 1;
        ulpadj = be + 1;
        dbits <<= 1;
        if (dbits & 0xffffffff00000000u64) == 0 {
            dbits <<= 32;
            be -= 32;
        }
        if (dbits & 0xffff000000000000u64) == 0 {
            dbits <<= 16;
            be -= 16;
        }
        if (dbits & 0xff00000000000000u64) == 0 {
            dbits <<= 8;
            be -= 8;
        }
        if (dbits & 0xf000000000000000u64) == 0 {
            dbits <<= 4;
            be -= 4;
        }
        if (dbits & 0xc000000000000000u64) == 0 {
            dbits <<= 2;
            be -= 2;
        }
        if (dbits & 0x8000000000000000u64) == 0 {
            dbits <<= 1;
            be -= 1;
        }
        ulpadj -= be;
    }
    j = *LHINT.as_ptr().offset((be + 51) as isize) as c_int;
    p10 = PTEN.as_ptr().offset(j as isize);
    dbhi = dbits >> 32;
    dblo = dbits & 0xffffffffu64;
    i = be - 0x3fe;
    if i < (*p10).e
        || (i == (*p10).e
            && (dbhi < (*p10).b0 as u64 || (dbhi == (*p10).b0 as u64 && dblo < (*p10).b1 as u64)))
    {
        j -= 1;
    }
    k = j - 342;
    if mode < 0 || mode > 9 {
        mode = 0;
    }
    if mode > 5 {
        mode -= 4;
    }
    leftright = 1;
    ilim = -1;
    ilim1 = -1;
    match mode {
        0 | 1 => {
            i = 18;
            ndigits = 0;
        }
        2 | 4 => {
            if mode == 2 {
                leftright = 0; /* case 2 falls through to case 4 */
            }
            if ndigits <= 0 {
                ndigits = 1;
            }
            i = ndigits;
            ilim1 = i;
            ilim = i;
        }
        3 | 5 => {
            if mode == 3 {
                leftright = 0; /* case 3 falls through to case 5 */
            }
            i = ndigits + k + 1;
            ilim = i;
            ilim1 = i - 1;
            if i <= 0 {
                i = 1;
            }
        }
        _ => {}
    }
    if buf.is_null() {
        buf = rv_alloc(i);
        blen = std::mem::size_of::<Bigint>()
            + (((1usize << *(buf as *const c_int).sub(1)) - 1) * std::mem::size_of::<ULong>())
            - std::mem::size_of::<c_int>();
    } else if blen <= (i as usize) {
        buf = std::ptr::null_mut();
        if !rve.is_null() {
            *rve = buf.wrapping_add(i as usize);
        }
        return buf;
    }
    s = buf;
    spec_case = 0;
    if mode < 2 || leftright != 0 {
        if u.w1() == 0
            && (u.w0() & 0xfffff) == 0
            && (u.w0() & (0x7ff00000u32 & !0x100000u32)) != 0
        {
            spec_case = 1;
        }
    }
    b = std::ptr::null_mut();

    let mut state: St;
    'pre: {
        if ilim < 0 && (mode == 3 || mode == 5) {
            S = std::ptr::null_mut();
            mhi = std::ptr::null_mut();
            state = St::NoDigits;
            break 'pre;
        }
        i = 1;
        j = 52 + 0x3ff - be;
        ulpshift = 0;
        ulplo = 0;
        if k < 0 {
            if k < -25 {
                state = St::Toobig;
                break 'pre;
            }
            res = dbits >> 11; /* residual */
            k1 = -(k + 1);
            n2 = pfivebits(k1) + 53;
            j1 = j;
            if n2 > 61 {
                ulpshift = n2 - 61;
                ulpmask = (1u64 << ulpshift).wrapping_sub(1);
                if (res & ulpmask) != 0 {
                    state = St::Toobig;
                    break 'pre;
                }
                j -= ulpshift;
                res >>= ulpshift;
            }
            ulp = pfive(k1);
            res = res.wrapping_mul(ulp);
            if ulpshift != 0 {
                ulplo = ulp;
                ulp >>= ulpshift;
            }
            j += k;
            if ilim == 0 {
                S = std::ptr::null_mut();
                mhi = std::ptr::null_mut();
                if res > (5u64 << j) {
                    state = St::OneDigit;
                    break 'pre;
                }
                state = St::NoDigits;
                break 'pre;
            }
            state = St::NoDiv;
            break 'pre;
        }
        if ilim == 0 && j + k >= 0 {
            S = std::ptr::null_mut();
            mhi = std::ptr::null_mut();
            if (dbits >> 11) > (pfive(k - 1) << j) {
                state = St::OneDigit;
                break 'pre;
            }
            state = St::NoDigits;
            break 'pre;
        }
        if k <= dtoa_divmax && j + k >= 0 {
            state = St::UseExact;
            break 'pre;
        }
        state = St::Toobig;
    }

    'sm: loop {
        match state {
            /* ============================== use_exact ====================== */
            St::UseExact => {
                res = dbits >> 11; /* residual */
                ulp = 1;
                if k <= 0 {
                    state = St::NoDiv;
                    continue 'sm;
                }
                j1 = j + k + 1;
                den = pfive(k - i) << (j1 - i);
                let nxt: St;
                'l1: loop {
                    dig = (res / den) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res = res.wrapping_sub((dig as u64).wrapping_mul(den));
                    if res == 0 {
                        nxt = St::Retc;
                        break 'l1;
                    }
                    if ilim < 0 {
                        ures = den.wrapping_sub(res);
                        if res.wrapping_mul(2) <= ulp
                            && (if spec_case != 0 {
                                res.wrapping_mul(4) <= ulp
                            } else {
                                res.wrapping_mul(2) < ulp || (dig & 1) != 0
                            })
                        {
                            nxt = St::UlpReached;
                            break 'l1;
                        }
                        if ures.wrapping_mul(2) < ulp {
                            nxt = St::Roundup;
                            break 'l1;
                        }
                    } else if i == ilim {
                        /* switch(Rounding) with Rounding == 1: no case matches */
                        ures = res.wrapping_mul(2);
                        if ures > den
                            || (ures == den && (dig & 1) != 0)
                            || (spec_case != 0 && res <= ulp && res.wrapping_mul(2) >= ulp)
                        {
                            nxt = St::Roundup;
                            break 'l1;
                        }
                        nxt = St::Retc;
                        break 'l1;
                    }
                    i += 1;
                    if j1 < i {
                        res = res.wrapping_mul(10);
                        ulp = ulp.wrapping_mul(10);
                    } else {
                        if i > k {
                            nxt = St::NoDiv;
                            break 'l1;
                        }
                        den = pfive(k - i) << (j1 - i);
                    }
                }
                state = nxt;
                continue 'sm;
            }

            /* ============================== no_div ========================= */
            St::NoDiv => {
                let nxt: St;
                'l2: loop {
                    den = res >> j;
                    dig = den as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res = res.wrapping_sub(den << j);
                    if res == 0 {
                        nxt = St::Retc;
                        break 'l2;
                    }
                    if ilim < 0 {
                        ures = (1u64 << j).wrapping_sub(res);
                        if res.wrapping_mul(2) <= ulp
                            && (if spec_case != 0 {
                                res.wrapping_mul(4) <= ulp
                            } else {
                                res.wrapping_mul(2) < ulp || (dig & 1) != 0
                            })
                        {
                            nxt = St::UlpReached;
                            break 'l2;
                        }
                        if ures.wrapping_mul(2) < ulp {
                            nxt = St::Roundup;
                            break 'l2;
                        }
                    }
                    j -= 1;
                    if i == ilim {
                        hb = 1u64 << j;
                        if (res & hb) != 0 && ((dig & 1) != 0 || (res & hb.wrapping_sub(1)) != 0) {
                            nxt = St::Roundup;
                            break 'l2;
                        }
                        if spec_case != 0 && res <= ulp && res.wrapping_mul(2) >= ulp {
                            nxt = St::Roundup;
                            break 'l2;
                        }
                        nxt = St::Retc;
                        break 'l2;
                    }
                    i += 1;
                    res = res.wrapping_mul(5);
                    if ulpshift != 0 {
                        ulplo = (ulplo & ulpmask).wrapping_mul(5);
                        ulp = ulp.wrapping_mul(5).wrapping_add(ulplo >> ulpshift);
                    } else {
                        ulp = ulp.wrapping_mul(5);
                    }
                }
                state = nxt;
                continue 'sm;
            }

            /* ============================== ulp_reached ==================== */
            St::UlpReached => {
                if ures < res || (ures == res && (dig & 1) != 0) {
                    state = St::Roundup;
                    continue 'sm;
                }
                state = St::Retc;
                continue 'sm;
            }

            /* ============================== Roundup ======================== */
            St::Roundup => {
                let mut carried = false;
                loop {
                    s = s.sub(1);
                    if *s != b'9' as c_char {
                        break;
                    }
                    if s == buf {
                        k += 1;
                        *s = b'1' as c_char;
                        s = s.add(1);
                        carried = true;
                        break;
                    }
                }
                if !carried {
                    *s = ((*s as c_int) + 1) as c_char;
                    s = s.add(1);
                }
                state = St::Ret1;
                continue 'sm;
            }

            /* ============================== toobig ========================= */
            St::Toobig => {
                if ilim > 28 {
                    state = St::FastFailed1;
                    continue 'sm;
                }
                p10 = PTEN.as_ptr().offset((342 - k) as isize);
                tv0 = ((*p10).b2 as u64).wrapping_mul(dblo);
                tv1 = ((*p10).b1 as u64)
                    .wrapping_mul(dblo)
                    .wrapping_add(tv0 >> 32);
                tv2 = ((*p10).b2 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv1 & 0xffffffffu64);
                tv3 = ((*p10).b0 as u64)
                    .wrapping_mul(dblo)
                    .wrapping_add(tv1 >> 32)
                    .wrapping_add(tv2 >> 32);
                res3 = ((*p10).b1 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv3 & 0xffffffffu64);
                res = ((*p10).b0 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv3 >> 32)
                    .wrapping_add(res3 >> 32);
                be += (*p10).e - 0x3fe;
                j1 = be - 54 + ulpadj;
                eulp = j1;
                if (res & 0x8000000000000000u64) == 0 {
                    be -= 1;
                    res3 <<= 1;
                    res = (res << 1) | ((res3 & 0x100000000u64) >> 32);
                }
                res0 = res;
                if ilim > 19 {
                    state = St::FastFailed;
                    continue 'sm;
                }
                res >>= 4 - be;
                ulp = (*p10).b0 as u64;
                ulp = (ulp << 29) | (((*p10).b1 >> 3) as u64);
                if ilim == 0 {
                    if (res & 0x7fffffffffffffeu64) == 0 || ((!res) & 0x7fffffffffffffeu64) == 0 {
                        state = St::FastFailed1;
                        continue 'sm;
                    }
                    S = std::ptr::null_mut();
                    mhi = std::ptr::null_mut();
                    if res >= 0x5000000000000000u64 {
                        state = St::OneDigit;
                        continue 'sm;
                    }
                    state = St::NoDigits;
                    continue 'sm;
                }
                rb = 1;
                let nxt: St;
                'l3: loop {
                    dig = (res >> 60) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res &= 0xfffffffffffffffu64;
                    if ilim < 0 {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if eulp > 0 {
                            sulp = ulp << (eulp - 1);
                            if res <= ures {
                                if res.wrapping_add(rb) > ures.wrapping_sub(rb) {
                                    nxt = St::FastFailed;
                                    break 'l3;
                                }
                                if res < sulp {
                                    nxt = St::Retc;
                                    break 'l3;
                                }
                            } else {
                                if res.wrapping_sub(rb) <= ures.wrapping_add(rb) {
                                    nxt = St::FastFailed;
                                    break 'l3;
                                }
                                if ures < sulp {
                                    nxt = St::Roundup;
                                    break 'l3;
                                }
                            }
                        } else {
                            zb = (1u64 << (eulp + 63)).wrapping_neg();
                            if (zb & res) == 0 {
                                sres = res << (1 - eulp);
                                if sres < ulp
                                    && (spec_case == 0 || sres.wrapping_mul(2) < ulp)
                                {
                                    if (res.wrapping_add(rb) << (1 - eulp)) >= ulp {
                                        nxt = St::FastFailed;
                                        break 'l3;
                                    }
                                    if ures < res {
                                        if ures.wrapping_add(rb) >= res.wrapping_sub(rb) {
                                            nxt = St::FastFailed;
                                            break 'l3;
                                        }
                                        nxt = St::Roundup;
                                        break 'l3;
                                    }
                                    if ures.wrapping_sub(rb) < res.wrapping_add(rb) {
                                        nxt = St::FastFailed;
                                        break 'l3;
                                    }
                                    nxt = St::Retc;
                                    break 'l3;
                                }
                            }
                            if (zb & ures) == 0 && (ures << (-eulp)) < ulp {
                                if (ures << (1 - eulp)) < ulp {
                                    nxt = St::Roundup;
                                    break 'l3;
                                }
                                nxt = St::FastFailed;
                                break 'l3;
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if ures < res {
                            if ures <= rb || res.wrapping_sub(rb) <= ures.wrapping_add(rb) {
                                if j + k >= 0 && k >= 0 && k <= 27 {
                                    /* use_exact1 */
                                    s = buf;
                                    i = 1;
                                    nxt = St::UseExact;
                                    break 'l3;
                                }
                                nxt = St::FastFailed;
                                break 'l3;
                            }
                            nxt = St::Roundup;
                            break 'l3;
                        }
                        if res <= rb || ures.wrapping_sub(rb) <= res.wrapping_add(rb) {
                            if j + k >= 0 && k >= 0 && k <= 27 {
                                /* use_exact1: */
                                s = buf;
                                i = 1;
                                nxt = St::UseExact;
                                break 'l3;
                            }
                            nxt = St::FastFailed;
                            break 'l3;
                        }
                        nxt = St::Retc;
                        break 'l3;
                    }
                    rb = rb.wrapping_mul(10);
                    if rb >= 0x1000000000000000u64 {
                        nxt = St::FastFailed;
                        break 'l3;
                    }
                    res = res.wrapping_mul(10);
                    ulp = ulp.wrapping_mul(5);
                    if (ulp & 0x8000000000000000u64) != 0 {
                        eulp += 4;
                        ulp >>= 3;
                    } else {
                        eulp += 3;
                        ulp >>= 2;
                    }
                    i += 1; /* for(;;++i) */
                }
                state = nxt;
                continue 'sm;
            }

            /* ============================== Fast_failed ==================== */
            St::FastFailed => {
                s = buf;
                i = 4 - be;
                res = res0 >> i;
                reslo = 0xffffffffu64 & res3;
                if i != 0 {
                    reslo = ((res0 << (64 - i)) >> 32) | (reslo >> i);
                }
                rb = 0;
                rblo = 4;
                ulp = (*p10).b0 as u64;
                ulp = (ulp << 29) | (((*p10).b1 >> 3) as u64);
                eulp = j1;
                let nxt: St;
                i = 1;
                'l4: loop {
                    'more96: {
                        dig = (res >> 60) as c_int;
                        *s = (b'0' as c_int + dig) as c_char;
                        s = s.add(1);
                        res &= 0xfffffffffffffffu64;
                        if ilim < 0 {
                            ures = 0x1000000000000000u64.wrapping_sub(res);
                            ureslo = 0;
                            if reslo != 0 {
                                ureslo = 0x100000000u64.wrapping_sub(reslo);
                                ures = ures.wrapping_sub(1);
                            }
                            if eulp > 0 {
                                sulp = (ulp << (eulp - 1)).wrapping_sub(rb);
                                if res <= ures {
                                    if res < sulp {
                                        if res.wrapping_add(rb) < ures.wrapping_sub(rb) {
                                            nxt = St::Retc;
                                            break 'l4;
                                        }
                                    }
                                } else if ures < sulp {
                                    if res.wrapping_sub(rb) > ures.wrapping_add(rb) {
                                        nxt = St::Roundup;
                                        break 'l4;
                                    }
                                }
                                nxt = St::FastFailed1;
                                break 'l4;
                            } else {
                                zb = (1u64 << (eulp + 60)).wrapping_neg();
                                if (zb & res.wrapping_add(rb)) == 0 {
                                    sres = res.wrapping_sub(rb) << (1 - eulp);
                                    if sres < ulp
                                        && (spec_case == 0 || sres.wrapping_mul(2) < ulp)
                                    {
                                        sres = res << (1 - eulp);
                                        j = eulp + 31;
                                        if j > 0 {
                                            sres = sres.wrapping_add(
                                                rblo.wrapping_add(reslo) >> j,
                                            );
                                        } else {
                                            sres = sres.wrapping_add(
                                                rblo.wrapping_add(reslo) << (-j),
                                            );
                                        }
                                        if sres.wrapping_add(rb << (1 - eulp)) >= ulp {
                                            nxt = St::FastFailed1;
                                            break 'l4;
                                        }
                                        if sres >= ulp {
                                            break 'more96;
                                        }
                                        if ures < res || (ures == res && ureslo < reslo) {
                                            if ures.wrapping_add(rb) >= res.wrapping_sub(rb) {
                                                nxt = St::FastFailed1;
                                                break 'l4;
                                            }
                                            nxt = St::Roundup;
                                            break 'l4;
                                        }
                                        if ures.wrapping_sub(rb) <= res.wrapping_add(rb) {
                                            nxt = St::FastFailed1;
                                            break 'l4;
                                        }
                                        nxt = St::Retc;
                                        break 'l4;
                                    }
                                }
                                if (zb & ures) == 0
                                    && (ures.wrapping_sub(rb) << (1 - eulp)) < ulp
                                {
                                    if (ures.wrapping_add(rb) << (1 - eulp)) < ulp {
                                        nxt = St::Roundup;
                                        break 'l4;
                                    }
                                    nxt = St::FastFailed1;
                                    break 'l4;
                                }
                            }
                        } else if i == ilim {
                            ures = 0x1000000000000000u64.wrapping_sub(res);
                            ureslo = 0;
                            sres = 0;
                            if reslo != 0 {
                                ureslo = 0x100000000u64.wrapping_sub(reslo);
                                ures = ures.wrapping_sub(1);
                                sres = reslo.wrapping_add(rblo) >> 31;
                            }
                            sres = sres.wrapping_add(rb.wrapping_mul(2));
                            if ures <= res {
                                if ures <= sres || res.wrapping_sub(ures) <= sres {
                                    nxt = St::FastFailed1;
                                    break 'l4;
                                }
                                nxt = St::Roundup;
                                break 'l4;
                            }
                            if res <= sres || ures.wrapping_sub(res) <= sres {
                                nxt = St::FastFailed1;
                                break 'l4;
                            }
                            nxt = St::Retc;
                            break 'l4;
                        }
                    }
                    /* more96: */
                    rblo = rblo.wrapping_mul(10);
                    rb = rb.wrapping_mul(10).wrapping_add(rblo >> 32);
                    rblo &= 0xffffffffu64;
                    if rb >= 0x1000000000000000u64 {
                        nxt = St::FastFailed1;
                        break 'l4;
                    }
                    reslo = reslo.wrapping_mul(10);
                    res = res.wrapping_mul(10).wrapping_add(reslo >> 32);
                    reslo &= 0xffffffffu64;
                    ulp = ulp.wrapping_mul(5);
                    if (ulp & 0x8000000000000000u64) != 0 {
                        eulp += 4;
                        ulp >>= 3;
                    } else {
                        eulp += 3;
                        ulp >>= 2;
                    }
                    i += 1; /* for(i = 1;;++i) */
                }
                state = nxt;
                continue 'sm;
            }

            /* ============================== Fast_failed1 =================== */
            St::FastFailed1 => {
                S = std::ptr::null_mut();
                mhi = std::ptr::null_mut();
                mlo = std::ptr::null_mut();
                b = d2b(&mut u, &mut be, &mut bbits);
                s = buf;
                i = ((u.w0() >> 20) & (0x7ff00000u32 >> 20)) as c_int;
                i -= 1023;
                if ulpadj != 0 {
                    i -= ulpadj - 1;
                }
                j = bbits - i - 1;
                if j >= 0 {
                    b2 = 0;
                    s2 = j;
                } else {
                    b2 = -j;
                    s2 = 0;
                }
                if k >= 0 {
                    b5 = 0;
                    s5 = k;
                    s2 += k;
                } else {
                    b2 -= k;
                    b5 = -k;
                    s5 = 0;
                }
                m2 = b2;
                m5 = b5;
                mhi = std::ptr::null_mut();
                mlo = std::ptr::null_mut();
                if leftright != 0 {
                    i = if denorm != 0 {
                        be + (1023 + (53 - 1) - 1 + 1)
                    } else {
                        1 + 53 - bbits
                    };
                    b2 += i;
                    s2 += i;
                    mhi = i2b(1);
                }
                if m2 > 0 && s2 > 0 {
                    i = if m2 < s2 { m2 } else { s2 };
                    b2 -= i;
                    m2 -= i;
                    s2 -= i;
                }
                if b5 > 0 {
                    if leftright != 0 {
                        if m5 > 0 {
                            mhi = pow5mult(mhi, m5);
                            b1 = mult(mhi, b);
                            Bfree(b);
                            b = b1;
                        }
                        j = b5 - m5;
                        if j != 0 {
                            b = pow5mult(b, j);
                        }
                    } else {
                        b = pow5mult(b, b5);
                    }
                }
                S = i2b(1);
                if s5 > 0 {
                    S = pow5mult(S, s5);
                }
                if spec_case != 0 {
                    b2 += 1;
                    s2 += 1;
                }
                i = dshift(S, s2);
                b2 += i;
                m2 += i;
                s2 += i;
                if b2 > 0 {
                    b = lshift(b, b2);
                }
                if s2 > 0 {
                    S = lshift(S, s2);
                }
                if ilim <= 0 && (mode == 3 || mode == 5) {
                    if ilim < 0 {
                        state = St::NoDigits;
                        continue 'sm;
                    }
                    S = multadd(S, 5, 0);
                    if cmp(b, S) <= 0 {
                        state = St::NoDigits;
                        continue 'sm;
                    }
                    /* one_digit: */
                    state = St::OneDigit;
                    continue 'sm;
                }
                if leftright != 0 {
                    if m2 > 0 {
                        mhi = lshift(mhi, m2);
                    }
                    mlo = mhi;
                    if spec_case != 0 {
                        mhi = Balloc((*mlo).k);
                        std::ptr::copy_nonoverlapping(
                            std::ptr::addr_of!((*mlo).sign) as *const u8,
                            std::ptr::addr_of_mut!((*mhi).sign) as *mut u8,
                            ((*mlo).wds as usize) * std::mem::size_of::<c_int>()
                                + 2 * std::mem::size_of::<c_int>(),
                        );
                        mhi = lshift(mhi, 1);
                    }
                    i = 1;
                    loop {
                        dig = quorem(b, S) + b'0' as c_int;
                        j = cmp(b, mlo);
                        delta = diff(S, mhi);
                        j1 = if (*delta).sign != 0 { 1 } else { cmp(b, delta) };
                        Bfree(delta);
                        if j1 == 0 && mode != 1 && (u.w1() & 1) == 0 {
                            if dig == b'9' as c_int {
                                state = St::Round9Up;
                                continue 'sm;
                            }
                            if j > 0 {
                                dig += 1;
                            }
                            *s = dig as c_char;
                            s = s.add(1);
                            state = St::Ret;
                            continue 'sm;
                        }
                        if j < 0 || (j == 0 && mode != 1 && (u.w1() & 1) == 0) {
                            'accept_dig: {
                                if *bx(b).add(0) == 0 && (*b).wds <= 1 {
                                    break 'accept_dig;
                                }
                                if j1 > 0 {
                                    b = lshift(b, 1);
                                    j1 = cmp(b, S);
                                    if j1 > 0 || (j1 == 0 && (dig & 1) != 0) {
                                        let old_dig = dig;
                                        dig += 1;
                                        if old_dig == b'9' as c_int {
                                            state = St::Round9Up;
                                            continue 'sm;
                                        }
                                    }
                                }
                            }
                            /* accept_dig: */
                            *s = dig as c_char;
                            s = s.add(1);
                            state = St::Ret;
                            continue 'sm;
                        }
                        if j1 > 0 {
                            if dig == b'9' as c_int {
                                state = St::Round9Up;
                                continue 'sm;
                            }
                            *s = (dig + 1) as c_char;
                            s = s.add(1);
                            state = St::Ret;
                            continue 'sm;
                        }
                        *s = dig as c_char;
                        s = s.add(1);
                        if i == ilim {
                            break;
                        }
                        b = multadd(b, 10, 0);
                        if mlo == mhi {
                            mhi = multadd(mhi, 10, 0);
                            mlo = mhi;
                        } else {
                            mlo = multadd(mlo, 10, 0);
                            mhi = multadd(mhi, 10, 0);
                        }
                        i += 1;
                    }
                } else {
                    i = 1;
                    loop {
                        dig = quorem(b, S) + b'0' as c_int;
                        *s = dig as c_char;
                        s = s.add(1);
                        if *bx(b).add(0) == 0 && (*b).wds <= 1 {
                            state = St::Ret;
                            continue 'sm;
                        }
                        if i >= ilim {
                            break;
                        }
                        b = multadd(b, 10, 0);
                        i += 1;
                    }
                }
                /* Round off last digit */
                b = lshift(b, 1);
                j = cmp(b, S);
                if j > 0 || (j == 0 && (dig & 1) != 0) {
                    state = St::Roundoff;
                    continue 'sm;
                }
                state = St::Ret;
                continue 'sm;
            }

            /* ============================== round_9_up ===================== */
            St::Round9Up => {
                *s = b'9' as c_char;
                s = s.add(1);
                state = St::Roundoff;
                continue 'sm;
            }

            /* ============================== roundoff ======================= */
            St::Roundoff => {
                let mut carried = false;
                loop {
                    s = s.sub(1);
                    if *s != b'9' as c_char {
                        break;
                    }
                    if s == buf {
                        k += 1;
                        *s = b'1' as c_char;
                        s = s.add(1);
                        carried = true;
                        break;
                    }
                }
                if !carried {
                    *s = ((*s as c_int) + 1) as c_char;
                    s = s.add(1);
                }
                state = St::Ret;
                continue 'sm;
            }

            /* ============================== no_digits ====================== */
            St::NoDigits => {
                k = -1 - ndigits;
                state = St::Ret;
                continue 'sm;
            }

            /* ============================== one_digit ====================== */
            St::OneDigit => {
                *s = b'1' as c_char;
                s = s.add(1);
                k += 1;
                state = St::Ret;
                continue 'sm;
            }

            /* ============================== ret ============================ */
            St::Ret => {
                Bfree(S);
                if !mhi.is_null() {
                    if !mlo.is_null() && mlo != mhi {
                        Bfree(mlo);
                    }
                    Bfree(mhi);
                }
                state = St::Retc;
                continue 'sm;
            }

            /* ============================== retc =========================== */
            St::Retc => {
                while s > buf && *s.sub(1) == b'0' as c_char {
                    s = s.sub(1);
                }
                state = St::Ret1;
                continue 'sm;
            }

            /* ============================== ret1 =========================== */
            St::Ret1 => {
                if !b.is_null() {
                    Bfree(b);
                }
                *s = 0;
                *decpt = k + 1;
                if !rve.is_null() {
                    *rve = s;
                }
                return buf;
            }
        }
    }
}
