//! Translation of the hex-float part of `src/dtoa.c`: the `hexdig[]` table,
//! `increment()`, `rshift()`, `any_on()` and the exported `gethex()`.
//!
//! Same configuration as [`crate::dtoa`]: `IEEE_8087`, `Pack_32`,
//! `MULTIPLE_THREADS` unset, `Honor_FLT_ROUNDS` unset, `USE_LOCALE` unset,
//! `NO_HEX_FP` unset, `IBM`/`VAX` unset.

use crate::dtoa::{balloc, bcopy, hi0bits, lshift, multadd_inplace, Bigint};
use crate::types::{set_errno, ERANGE};
use core::ffi::{c_char, c_int, c_void};

/* Pack_32 */
const ULBITS: i32 = 32;
const KSHIFT: i32 = 5;
const KMASK: i32 = 31;

/* IEEE double constants (see dtoa.c lines 1367..1381 and 1494..1495) */
const NBITS: i32 = 53;
const BIAS: i32 = 1023;
const EMAX_C: i32 = 1023;
const EMIN_C: i32 = -1022;
const P: i32 = 53;
const EXP_SHIFT: i32 = 20;
const EXP_MASK: u32 = 0x7ff00000;
const BIG0: u32 = 0x7fef_ffff; /* Frac_mask1 | Exp_msk1*(DBL_MAX_EXP+Bias-1) */
const BIG1: u32 = 0xffff_ffff;

/* enum { emax = 0x7fe - Bias - P + 1, emin = Emin - P + 1 } */
const EMAX: i32 = 0x7fe - BIAS - P + 1; /* 971 */
const EMIN: i32 = EMIN_C - P + 1; /* -1074 */

/* rounding values: same as FLT_ROUNDS */
pub const ROUND_ZERO: c_int = 0;
pub const ROUND_NEAR: c_int = 1;
pub const ROUND_UP: c_int = 2;
pub const ROUND_DOWN: c_int = 3;

/// `static unsigned char hexdig[256]` (the pre-initialised variant, dtoa.c
/// line 2518: digits map to `0x10 + value`, everything else is 0).
pub static HEXDIG: [u8; 256] = {
    let mut h = [0u8; 256];
    let mut i = 0;
    while i < 10 {
        h[b'0' as usize + i] = (0x10 + i) as u8;
        i += 1;
    }
    let mut i = 0;
    while i < 6 {
        h[b'a' as usize + i] = (0x10 + 10 + i) as u8;
        h[b'A' as usize + i] = (0x10 + 10 + i) as u8;
        i += 1;
    }
    h
};

#[inline]
pub fn hexdig(c: u8) -> u8 {
    HEXDIG[c as usize]
}

/// `increment(b)`
pub fn increment(mut b: Bigint) -> Bigint {
    let wds = b.wds;
    let mut i = 0usize;
    loop {
        if b.x[i] < 0xffff_ffffu32 {
            b.x[i] += 1;
            return b;
        }
        b.x[i] = 0;
        i += 1;
        if i >= wds as usize {
            break;
        }
    }
    {
        if b.wds >= b.maxwds {
            let mut b1 = balloc(b.k + 1);
            bcopy(&mut b1, &b);
            b = b1;
        }
        let w = b.wds;
        b.x[w as usize] = 1;
        b.wds = w + 1;
    }
    b
}

/// `rshift(b, k)`
pub fn rshift(b: &mut Bigint, k_in: i32) {
    let mut k = k_in;
    let mut x1: usize = 0; /* index of x1 into b->x */
    let mut xi: usize; /* index of x */
    let n0 = k >> KSHIFT;
    let wds = b.wds;

    if n0 < wds {
        let xe = wds as usize;
        xi = n0 as usize;
        k &= KMASK;
        if k != 0 {
            let n = 32 - k;
            let mut y = b.x[xi] >> k;
            xi += 1;
            while xi < xe {
                let v = (y | (b.x[xi].wrapping_shl(n as u32))) & 0xffff_ffff;
                b.x[x1] = v;
                x1 += 1;
                y = b.x[xi] >> k;
                xi += 1;
            }
            b.x[x1] = y;
            if y != 0 {
                x1 += 1;
            }
        } else {
            while xi < xe {
                let v = b.x[xi];
                xi += 1;
                b.x[x1] = v;
                x1 += 1;
            }
        }
    }
    b.wds = x1 as i32;
    if x1 == 0 {
        b.x[0] = 0;
    }
}

/// `any_on(b, k)`
pub fn any_on(b: &Bigint, k_in: i32) -> u32 {
    let mut k = k_in;
    let nwds = b.wds;
    let mut n = k >> KSHIFT;
    if n > nwds {
        n = nwds;
    } else if n < nwds {
        k &= KMASK;
        if k != 0 {
            let x2 = b.x[n as usize];
            let mut x1 = x2;
            x1 >>= k;
            x1 <<= k;
            if x1 != x2 {
                return 1;
            }
        }
    }
    let mut i = n;
    while i > 0 {
        i -= 1;
        if b.x[i as usize] != 0 {
            return 1;
        }
    }
    0
}

/// `U` as used by `gethex()`: `union { double d; ULong L[2]; ULLong LL; }` with
/// `IEEE_8087`, so `word0` is `L[1]` and `word1` is `L[0]`.
#[inline]
unsafe fn set_word0(rvp: *mut c_void, v: u32) {
    *(rvp as *mut u32).add(1) = v;
}

#[inline]
unsafe fn set_word1(rvp: *mut c_void, v: u32) {
    *(rvp as *mut u32) = v;
}

#[inline]
unsafe fn set_d(rvp: *mut c_void, v: f64) {
    *(rvp as *mut f64) = v;
}

/// Where the C code jumps to; used to keep the control flow of the original
/// `goto` graph.
enum G {
    Done,
    RetTiny,
    Retz,
    Retz1,
    RetBig,
    Ovfl1,
    Normal,
}

/// `void gethex(const char **sp, U *rvp, int rounding, int sign)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethex(
    sp: *mut *const c_char,
    rvp: *mut c_void,
    rounding: c_int,
    sign: c_int,
) {
    let mut b: Option<Bigint> = None;
    let mut d: u8;
    let mut decpt: *const u8;
    let mut s0: *const u8;
    let mut s: *const u8;
    let s1: *const u8;
    let mut e: i64;
    let mut e1: i64;
    let mut l: u32;
    let mut lostbits: u32;
    let mut big: i32;
    let mut denorm: i32;
    let mut esign: i32;
    let mut havedig: i32;
    let mut k: i32;
    let mut n: i32;
    let nb: i32;
    let mut nbits: i32;
    let nz: i32;
    let mut up: i32;
    let zret: i32;
    let mut check_denorm = 0;

    /**** if (!hexdig['0']) hexdig_init(); ****/
    havedig = 0;
    s0 = (*sp as *const u8).add(2);
    while *s0.add(havedig as usize) == b'0' {
        havedig += 1;
    }
    s0 = s0.add(havedig as usize);
    s = s0;
    decpt = core::ptr::null();
    let mut zret_v = 0;
    e = 0;

    'pcheck: {
        if hexdig(*s) != 0 {
            havedig += 1;
        } else {
            zret_v = 1;
            if *s != b'.' {
                break 'pcheck;
            }
            s = s.add(1);
            decpt = s;
            if hexdig(*s) == 0 {
                break 'pcheck;
            }
            while *s == b'0' {
                s = s.add(1);
            }
            if hexdig(*s) != 0 {
                zret_v = 0;
            }
            havedig = 1;
            s0 = s;
        }
        while hexdig(*s) != 0 {
            s = s.add(1);
        }
        if *s == b'.' && decpt.is_null() {
            s = s.add(1);
            decpt = s;
            while hexdig(*s) != 0 {
                s = s.add(1);
            }
        }
        if !decpt.is_null() {
            e = -(((s.offset_from(decpt) as i64) << 2) as i64);
        }
    }
    zret = zret_v;

    /* pcheck: */
    s1 = s;
    big = 0;
    esign = 0;
    match *s {
        b'p' | b'P' => 'pexp: {
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
            n = hexdig(*s) as i32;
            if n == 0 || n > 0x19 {
                s = s1;
                break 'pexp;
            }
            e1 = (n - 0x10) as i64;
            loop {
                s = s.add(1);
                n = hexdig(*s) as i32;
                if n == 0 || n > 0x19 {
                    break;
                }
                if (e1 as u32) & 0xf800_0000 != 0 {
                    big = 1;
                }
                e1 = 10i64.wrapping_mul(e1).wrapping_add((n - 0x10) as i64);
            }
            if esign != 0 {
                e1 = -e1;
            }
            e += e1;
        }
        _ => {}
    }

    *sp = s as *const c_char;
    if havedig == 0 {
        *sp = (s0 as *const c_char).offset(-1);
    }

    let mut goto: G = G::Done;

    'flow: {
        if zret != 0 {
            goto = G::Retz1;
            break 'flow;
        }
        if big != 0 {
            if esign != 0 {
                match rounding {
                    ROUND_UP => {
                        if sign == 0 {
                            goto = G::RetTiny;
                            break 'flow;
                        }
                    }
                    ROUND_DOWN => {
                        if sign != 0 {
                            goto = G::RetTiny;
                            break 'flow;
                        }
                    }
                    _ => {}
                }
                goto = G::Retz;
                break 'flow;
            }
            match rounding {
                ROUND_NEAR => {
                    goto = G::Ovfl1;
                    break 'flow;
                }
                ROUND_UP => {
                    if sign == 0 {
                        goto = G::Ovfl1;
                        break 'flow;
                    }
                    goto = G::RetBig;
                    break 'flow;
                }
                ROUND_DOWN => {
                    if sign != 0 {
                        goto = G::Ovfl1;
                        break 'flow;
                    }
                    goto = G::RetBig;
                    break 'flow;
                }
                _ => {}
            }
            goto = G::RetBig;
            break 'flow;
        }

        n = s1.offset_from(s0) as i32 - 1;
        k = 0;
        while n > (1 << (KSHIFT - 2)) - 1 {
            n >>= 1;
            k += 1;
        }
        let mut bb = balloc(k);
        let mut xi: usize = 0;
        havedig = 0;
        n = 0;
        let _ = nz;
        l = 0;
        let mut s1m = s1;
        while s1m > s0 {
            s1m = s1m.offset(-1);
            if *s1m == b'.' {
                continue;
            }
            d = hexdig(*s1m);
            if d != 0 {
                havedig = 1;
            } else if havedig == 0 {
                e += 4;
                continue;
            }
            if n == ULBITS {
                bb.x[xi] = l;
                xi += 1;
                l = 0;
                n = 0;
            }
            l |= ((d & 0x0f) as u32) << n;
            n += 4;
        }
        bb.x[xi] = l;
        xi += 1;
        n = xi as i32;
        bb.wds = n;
        nb = ULBITS * n - hi0bits(l);
        nbits = NBITS;
        lostbits = 0;
        if nb > nbits {
            n = nb - nbits;
            if any_on(&bb, n) != 0 {
                lostbits = 1;
                k = n - 1;
                if bb.x[(k >> KSHIFT) as usize] & (1u32 << (k & KMASK)) != 0 {
                    lostbits = 2;
                    if k > 0 && any_on(&bb, k) != 0 {
                        lostbits = 3;
                    }
                }
            }
            rshift(&mut bb, n);
            e += n as i64;
        } else if nb < nbits {
            n = nbits - nb;
            bb = lshift(bb, n);
            e -= n as i64;
        }
        b = Some(bb);

        if e > EMAX as i64 {
            /* ovfl: Bfree(b); ovfl1: */
            b = None;
            goto = G::Ovfl1;
            break 'flow;
        }
        let bb = b.as_mut().unwrap();
        denorm = 0;
        let mut skip_lostbits = false;
        if e < EMIN as i64 {
            denorm = 1;
            n = (EMIN as i64 - e) as i32;
            if n >= nbits {
                match rounding {
                    ROUND_NEAR => {
                        if n == nbits && (n < 2 || lostbits != 0 || any_on(bb, n - 1) != 0) {
                            b = None;
                            goto = G::RetTiny;
                            break 'flow;
                        }
                    }
                    ROUND_UP => {
                        if sign == 0 {
                            b = None;
                            goto = G::RetTiny;
                            break 'flow;
                        }
                    }
                    ROUND_DOWN => {
                        if sign != 0 {
                            b = None;
                            goto = G::RetTiny;
                            break 'flow;
                        }
                    }
                    _ => {}
                }
                b = None;
                goto = G::Retz;
                break 'flow;
            }
            k = n - 1;
            let mut emin_check = false;
            if k == 0 {
                match rounding {
                    ROUND_NEAR => {
                        if (bb.x[0] & 3) == 3 || (lostbits != 0 && (bb.x[0] & 1) != 0) {
                            multadd_inplace(bb, 1, 1);
                            emin_check = true;
                        }
                    }
                    ROUND_UP => {
                        if sign == 0 && (lostbits != 0 || (bb.x[0] & 1) != 0) {
                            /* incr_denorm: */
                            multadd_inplace(bb, 1, 2);
                            check_denorm = 1;
                            lostbits = 0;
                            emin_check = true;
                        }
                    }
                    ROUND_DOWN => {
                        if sign != 0 && (lostbits != 0 || (bb.x[0] & 1) != 0) {
                            multadd_inplace(bb, 1, 2);
                            check_denorm = 1;
                            lostbits = 0;
                            emin_check = true;
                        }
                    }
                    _ => {}
                }
            }
            if emin_check {
                /* emin_check: */
                if bb.wds > 1 && bb.x[1] == (1u32 << (EXP_SHIFT + 1)) {
                    rshift(bb, 1);
                    e = EMIN as i64;
                    goto = G::Normal;
                    break 'flow;
                }
            }
            if lostbits != 0 {
                lostbits = 1;
            } else if k > 0 {
                lostbits = any_on(bb, k);
            } else if check_denorm != 0 {
                skip_lostbits = true;
            }
            if !skip_lostbits && bb.x[(k >> KSHIFT) as usize] & (1u32 << (k & KMASK)) != 0 {
                lostbits |= 2;
            }
            /* no_lostbits: */
            nbits -= n;
            rshift(bb, n);
            e = EMIN as i64;
        }
        if lostbits != 0 {
            up = 0;
            match rounding {
                ROUND_ZERO => {}
                ROUND_NEAR => {
                    if lostbits & 2 != 0 && ((lostbits & 1) | (bb.x[0] & 1)) != 0 {
                        up = 1;
                    }
                }
                ROUND_UP => {
                    up = 1 - sign;
                }
                ROUND_DOWN => {
                    up = sign;
                }
                _ => {}
            }
            if up != 0 {
                k = bb.wds;
                let taken = b.take().unwrap();
                let mut nbb = increment(taken);
                if denorm == 0 {
                    let cond = nbb.wds > k
                        || ({
                            n = nbits & KMASK;
                            n != 0 && hi0bits(nbb.x[(k - 1) as usize]) < 32 - n
                        });
                    if cond {
                        rshift(&mut nbb, 1);
                        e += 1;
                        if e > EMAX_C as i64 {
                            /* goto ovfl */
                            b = None;
                            let _ = nbb;
                            goto = G::Ovfl1;
                            break 'flow;
                        }
                    }
                }
                b = Some(nbb);
            }
        }

        let bb = b.as_ref().unwrap();
        if denorm != 0 {
            set_word0(
                rvp,
                if bb.wds > 1 {
                    bb.x[1] & !0x100000
                } else {
                    0
                },
            );
        } else {
            /* normal: */
            set_word0(
                rvp,
                (bb.x.get(1).copied().unwrap_or(0) & !0x100000)
                    | (((e + 0x3ff + 52) as u32) << 20),
            );
        }
        set_word1(rvp, bb.x[0]);
        goto = G::Done;
    }

    match goto {
        G::Done => {}
        G::Normal => {
            let bb = b.as_ref().unwrap();
            set_word0(
                rvp,
                (bb.x.get(1).copied().unwrap_or(0) & !0x100000)
                    | (((e + 0x3ff + 52) as u32) << 20),
            );
            set_word1(rvp, bb.x[0]);
        }
        G::RetTiny => {
            /* ret_tinyf: Bfree(b); ret_tiny: */
            set_errno(ERANGE);
            set_word0(rvp, 0);
            set_word1(rvp, 1);
            return;
        }
        G::Retz => {
            set_errno(ERANGE);
            set_d(rvp, 0.0);
            return;
        }
        G::Retz1 => {
            set_d(rvp, 0.0);
            return;
        }
        G::RetBig => {
            set_word0(rvp, BIG0);
            set_word1(rvp, BIG1);
            return;
        }
        G::Ovfl1 => {
            set_errno(ERANGE);
            set_word0(rvp, EXP_MASK);
            set_word1(rvp, 0);
            return;
        }
    }

    /* Bfree(b) */
    drop(b);
}
