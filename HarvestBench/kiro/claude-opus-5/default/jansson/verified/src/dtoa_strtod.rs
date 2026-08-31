//! Faithful, line-for-line translation of `sulp`, `bigcomp` and
//! `strtod__unused` from David M. Gay's `dtoa.c` (`c_src/src/dtoa.c`).
//!
//! Source ranges translated:
//!   * `sulp`           (dtoa.c lines 3269-3283)
//!   * `bigcomp`        (dtoa.c lines 3286-3487)
//!   * `strtod__unused` (dtoa.c lines 3489-4911)
//!
//! Active configuration (exactly these; all other preprocessor branches
//! deleted):
//!   DEFINED:  IEEE_8087, IEEE_Arith, Pack_32, ULLong, USE_BF96,
//!             Avoid_Underflow, INFNAN_CHECK, Need_Hexdig, Check_FLT_ROUNDS.
//!   NOT DEF:  NO_HEX_FP, No_Hex_NaN, GDTOA_NON_PEDANTIC_NANCHECK,
//!             NO_STRTOD_BIGCOMP, MULTIPLE_THREADS, Honor_FLT_ROUNDS,
//!             Trust_FLT_ROUNDS, SET_INEXACT, USE_LOCALE, NO_LOCALE_CACHE,
//!             Sudden_Underflow, ROUND_BIASED, ROUND_BIASED_without_Round_Up,
//!             VAX, IBM, DEBUG, Omit_Private_Memory, RND_PRODQUOT, NO_ERRNO,
//!             Bad_float_h, KR_headers, Flush_Denorm, NO_IEEE_Scale,
//!             NO_INFNAN_CHECK, NO_STRTOD_DIGLIM, DIGLIM_DEBUG, NO_BF96,
//!             Flt_Rounds_Debug.
//!
//! Consequences applied throughout:
//!   * `MTd`/`MTa`/`MTb` macro arguments expand to nothing.
//!   * `Debug(x)` expands to nothing.
//!   * `Set_errno(x)` is `errno = x`.
//!   * `Rounding == Flt_Rounds == FLT_ROUNDS == 1`. Since Honor_FLT_ROUNDS is
//!     undefined, `bc.rounding` is never set in strtod; all Honor_FLT_ROUNDS
//!     blocks are deleted. `bc.rounding` is initialised to 0.
//!   * `strtod_diglim == STRTOD_DIGLIM == 40`.
//!   * `Scale_Bit == 0x10`, `n_bigtens == 5`.
//!   * `gethex(&s, &rv, 1, sign)` (the non-Honor_FLT_ROUNDS form).
//!   * `rounded_product(a,b)` is `a *= b`; `rounded_quotient(a,b)` is `a /= b`.
//!
//! IEEE_8087 double constants (dtoa.c lines 1360-1400, 1494-1495, 1505):
//!   Exp_shift=20, Exp_msk1=0x100000, Exp_mask=0x7ff00000, P=53, Bias=1023,
//!   Emin=-1022, Exp_1=0x3ff00000, Ebits=11, Frac_mask=0xfffff, Ten_pmax=22,
//!   Bndry_mask=0xfffff, Bndry_mask1=0xfffff, LSB=1, Sign_bit=0x80000000,
//!   Log2P=1, Tiny0=0, Tiny1=1, Big0=0x7fefffff, Big1=0xffffffff, DBL_DIG=15,
//!   DBL_MAX_EXP=1024, NAN_WORD0=0x7ff80000, NAN_WORD1=0.
//!
//! Union `U { double d; ULong L[2]; ULLong LL; }` with IEEE_8087 means
//! `word0(x)` is `L[1]` (HIGH 32 bits), `word1(x)` is `L[0]` (LOW 32 bits),
//! `dval(x)` is `d`, `LLval(x)` is `LL` (the raw 64-bit pattern). Each `U`
//! local is an `f64`; `word0`/`word1`/`set_word0`/`set_word1` do the `L[]`
//! accesses and `to_bits`/`from_bits` do the `LL` access.
//!
//! `Long L` is a C `int` (32-bit signed); the exponent-accumulation loop relies
//! on that width. `ULong` is `u32`, `ULLong` is `u64`.

#![allow(unused_assignments, unused_variables, unused_mut, non_snake_case)]
#![allow(clippy::all)]

use core::ffi::{c_char, c_int, c_void};

use crate::dtoa::{
    balloc, bcopy, cmp, diff, hi0bits, i2b, lshift, mult, multadd, pow5mult, quorem, Bigint,
};
use crate::dtoa_hex::{gethex, hexdig};
use crate::dtoa_strtod_helpers::{
    b2d, bigtens, hexnan, match_str, ratio, s2b, set_word0, set_word1, tens, tinytens, ulp, word0,
    word1, N_BIGTENS, SCALE_BIT,
};
use crate::dtoa_tables::{Bf96, PFIVE, PTEN};
use crate::types::{set_errno, ERANGE};

/* -------------------------------------------------- IEEE_8087 constants */

const EXP_SHIFT: i32 = 20;
const EXP_SHIFT1: i32 = 20;
const EXP_MSK1: u32 = 0x100000;
const EXP_MSK11: u32 = 0x100000;
const EXP_MASK: u32 = 0x7ff00000;
const P: i32 = 53;
const NBITS: i32 = 53;
const BIAS: i32 = 1023;
const EMAX: i32 = 1023;
const EMIN: i32 = -1022;
const EXP_1: u32 = 0x3ff00000;
const EXP_11: u32 = 0x3ff00000;
const EBITS: i32 = 11;
const FRAC_MASK: u32 = 0xfffff;
const FRAC_MASK1: u32 = 0xfffff;
const TEN_PMAX: i32 = 22;
const BLETCH: u32 = 0x10;
const BNDRY_MASK: u32 = 0xfffff;
const BNDRY_MASK1: u32 = 0xfffff;
const LSB: u32 = 1;
const SIGN_BIT: u32 = 0x80000000;
const LOG2P: i32 = 1;
const TINY0: u32 = 0;
const TINY1: u32 = 1;
const QUICK_MAX: i32 = 14;
const INT_MAX: i32 = 14;
const BIG0: u32 = 0x7fefffff;
const BIG1: u32 = 0xffffffff;
const DBL_DIG: i32 = 15;
const DBL_MAX_EXP: i32 = 1024;
const DBL_MAX_10_EXP: i32 = 308;
const FLT_RADIX: i32 = 2;
const NAN_WORD0: u32 = 0x7ff80000;
const NAN_WORD1: u32 = 0;

/* #define Flt_Rounds 1 / #define Rounding Flt_Rounds */
const FLT_ROUNDS: i32 = 1;

/* #define strtod_diglim 40 (STRTOD_DIGLIM) */
const STRTOD_DIGLIM: i32 = 40;

/* struct BCinfo { int dp0, dp1, dplen, dsign, e0, inexact, nd, nd0, rounding,
 *                 scale, uflchk; } */
#[derive(Default)]
struct BCinfo {
    dp0: i32,
    dp1: i32,
    dplen: i32,
    dsign: i32,
    e0: i32,
    inexact: i32,
    nd: i32,
    nd0: i32,
    rounding: i32,
    scale: i32,
    uflchk: i32,
}

/* dshift(): dtoa.c lines 3140-3147.  kmask == 31. */
fn dshift(b: &Bigint, p2: i32) -> i32 {
    let mut rv = hi0bits(b.x[(b.wds - 1) as usize]) - 4;
    if p2 > 0 {
        rv -= p2;
    }
    rv & 31
}

/* ----------------------------------------------------------------- sulp */

/// `static double sulp(U *x, BCinfo *bc)` (dtoa.c lines 3268-3284).
///
/// Avoid_Underflow defined, so this scaling helper exists.
fn sulp(x: f64, bc: &BCinfo) -> f64 {
    let mut u: f64 = 0.0;
    let rv: f64;
    let i: i32;

    rv = ulp(x);
    if bc.scale == 0 || {
        i = 2 * P + 1 - (((word0(x) & EXP_MASK) >> EXP_SHIFT) as i32);
        i <= 0
    } {
        return rv; /* Is there an example where i <= 0 ? */
    }
    { let __wv = (EXP_1 as i32 + (i << EXP_SHIFT)) as u32; set_word0(&mut u, __wv); };
    set_word1(&mut u, 0);
    rv * u /* rv * u.d */
}

/* --------------------------------------------------------------- bigcomp */

/// `static void bigcomp(U *rv, const char *s0, BCinfo *bc)`
/// (dtoa.c lines 3285-3487).
///
/// NO_STRTOD_BIGCOMP undefined => function exists. Sudden_Underflow undefined,
/// Honor_FLT_ROUNDS undefined, Avoid_Underflow defined.
unsafe fn bigcomp(rv: &mut f64, s0: *const u8, bc: &mut BCinfo) {
    let mut b: Bigint;
    let mut d: Bigint;
    let mut b2: i32;
    let mut bbits: i32;
    let mut d2: i32;
    let mut dd: i32 = 0;
    let mut dig: i32;
    let mut dsign: i32;
    let mut i: i32;
    let mut j: i32;
    let mut nd: i32;
    let mut nd0: i32;
    let mut p2: i32;
    let mut p5: i32;
    let mut speccase: i32;

    dsign = bc.dsign;
    nd = bc.nd;
    nd0 = bc.nd0;
    p5 = nd + bc.e0 - 1;
    speccase = 0;
    /* #ifndef Sudden_Underflow */
    if *rv == 0.0 {
        /* special case: value near underflow-to-zero */
        /* threshold was rounded to zero */
        b = i2b(1);
        p2 = EMIN - P + 1;
        bbits = 1;
        /* Avoid_Underflow */
        { let __wv = ((P + 2) << EXP_SHIFT) as u32; set_word0(rv, __wv); };
        i = 0;
        /* Honor_FLT_ROUNDS undefined => unconditional */
        {
            speccase = 1;
            p2 -= 1;
            dsign = 0;
            /* goto have_i; */
        }
    } else {
        let (bb, e, bts) = crate::dtoa::d2b(rv.to_bits());
        b = bb;
        p2 = e;
        bbits = bts;
        /* Avoid_Underflow */
        p2 -= bc.scale;
        /* floor(log2(rv)) == bbits - 1 + p2 */
        /* Check for denormal case. */
        i = P - bbits;
        j = P - EMIN - 1 + p2;
        if i > j {
            /* Sudden_Underflow undefined => i = j; */
            i = j;
        }
        /* Honor_FLT_ROUNDS undefined => else-branch */
        {
            b = lshift(b, i + 1);
            i += 1;
            b.x[0] |= 1;
        }
    }
    /* have_i: */
    p2 -= p5 + i;
    d = i2b(1);
    /* Arrange for convenient computation of quotients:
     * shift left if necessary so divisor has 4 leading 0 bits.
     */
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
    i = dshift(&d, d2);
    b2 += i;
    if b2 > 0 {
        b = lshift(b, b2);
    }
    d2 += i;
    if d2 > 0 {
        d = lshift(d, d2);
    }

    /* Now b/d = exactly half-way between the two floating-point values */
    /* on either side of the input string.  Compute first digit of b/d. */

    dig = quorem(&mut b, &d);
    if dig == 0 {
        b = multadd(b, 10, 0); /* very unlikely */
        dig = quorem(&mut b, &d);
    }

    /* Compare b/d with s0 */

    'ret: {
        i = 0;
        while i < nd0 {
            dd = *s0.add(i as usize) as i32 - '0' as i32 - dig;
            i += 1;
            if dd != 0 {
                break 'ret;
            }
            if b.x[0] == 0 && b.wds == 1 {
                if i < nd {
                    dd = 1;
                }
                break 'ret;
            }
            b = multadd(b, 10, 0);
            dig = quorem(&mut b, &d);
        }
        j = bc.dp1;
        loop {
            let cond = {
                let old = i;
                i += 1;
                old < nd
            };
            if !cond {
                break;
            }
            dd = *s0.add(j as usize) as i32 - '0' as i32 - dig;
            j += 1;
            if dd != 0 {
                break 'ret;
            }
            if b.x[0] == 0 && b.wds == 1 {
                if i < nd {
                    dd = 1;
                }
                break 'ret;
            }
            b = multadd(b, 10, 0);
            dig = quorem(&mut b, &d);
        }
        if dig > 0 || b.x[0] != 0 || b.wds > 1 {
            dd = -1;
        }
    }
    /* ret: */
    /* Bfree(b); Bfree(d); -> drop */
    drop(b);
    drop(d);

    /* Honor_FLT_ROUNDS undefined => this branch */
    'retpath: {
        if speccase != 0 {
            if dd <= 0 {
                *rv = 0.0;
            }
        } else if dd < 0 {
            if dsign == 0 {
                /* does not happen for round-near */
                /* retlow1: */
                *rv -= sulp(*rv, bc);
            }
        } else if dd > 0 {
            if dsign != 0 {
                /* rethi1: */
                *rv += sulp(*rv, bc);
            }
        } else {
            /* Exact half-way case:  apply round-even rule. */
            j = (((word0(*rv) & EXP_MASK) >> EXP_SHIFT) as i32) - bc.scale;
            'checkodd: {
                if j <= 0 {
                    i = 1 - j;
                    if i <= 31 {
                        if word1(*rv) & (0x1u32 << i) != 0 {
                            /* goto odd; */
                            if dsign != 0 {
                                *rv += sulp(*rv, bc);
                            } else {
                                *rv -= sulp(*rv, bc);
                            }
                            break 'checkodd;
                        }
                    } else if word0(*rv) & (0x1u32 << (i - 32)) != 0 {
                        /* goto odd; */
                        if dsign != 0 {
                            *rv += sulp(*rv, bc);
                        } else {
                            *rv -= sulp(*rv, bc);
                        }
                        break 'checkodd;
                    }
                } else if word1(*rv) & 1 != 0 {
                    /* odd: */
                    if dsign != 0 {
                        *rv += sulp(*rv, bc);
                    } else {
                        *rv -= sulp(*rv, bc);
                    }
                }
            }
        }
        /* ret1: (Honor_FLT_ROUNDS) -> just return */
    }
}

/* --------------------------------------------------------- strtod__unused */

/// `double strtod__unused(const char *s00, char **se)` (dtoa.c lines 3488-4911).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod__unused(s00: *const c_char, se: *mut *mut c_char) -> f64 {
    let mut bb2: i32;
    let mut bb5: i32;
    let mut bbe: i32 = 0;
    let mut bd2: i32;
    let mut bd5: i32;
    let mut bbbits: i32 = 0;
    let mut bs2: i32;
    let mut c: i32;
    let mut e: i32;
    let mut e1: i32;
    let mut esign: i32;
    let mut i: i32;
    let mut j: i32;
    let mut k: i32;
    let mut nd: i32;
    let mut nd0: i32;
    let mut nf: i32;
    let mut nz: i32;
    let mut nz0: i32;
    let mut nz1: i32;
    let mut sign: i32;
    let mut s: *const u8;
    let mut s0: *const u8;
    let mut s1: *const u8;
    let mut s00: *const u8 = s00 as *const u8;
    let mut aadj: f64;
    let mut aadj1: f64;
    let mut L: i32; /* Long == int (32-bit) */
    let mut aadj2: f64 = 0.0; /* U */
    let mut adj: f64 = 0.0; /* U */
    let mut rv: f64 = 0.0; /* U */
    let mut rv0: f64 = 0.0; /* U */
    let mut y: u32;
    let mut z: u32;
    let mut bc: BCinfo = BCinfo::default();
    let mut bb: Option<Bigint> = None;
    let mut bb1: Option<Bigint>;
    let mut bd: Option<Bigint> = None;
    let mut bd0: Option<Bigint> = None;
    let mut bs: Option<Bigint> = None;
    let mut delta: Option<Bigint> = None;
    /* USE_BF96 */
    let mut bhi: u64;
    let mut blo: u64;
    let mut brv: u64;
    let mut t00: u64;
    let mut t01: u64;
    let mut t02: u64;
    let mut t10: u64;
    let mut t11: u64;
    let mut terv: u64;
    let mut tg: u64;
    let mut tlo: u64;
    let mut yz: u64;
    let mut p10: &Bf96;
    let mut bexact: i32;
    let mut erv: i32;
    /* Avoid_Underflow */
    let mut Lsb: u32;
    let mut Lsb1: u32;
    /* NO_STRTOD_BIGCOMP undefined */
    let mut req_bigcomp: i32 = 0;

    /* Honor_FLT_ROUNDS undefined => bc.rounding is never explicitly assigned in
     * strtod. However Check_FLT_ROUNDS IS defined, and the sole `switch
     * (bc.rounding)` it guards (in the `aadj > 2` branch) must behave as
     * round-to-nearest to match the C reference (which resolves to the `switch`
     * default). `Rounding == Flt_Rounds == FLT_ROUNDS == 1`, so initialise to 1.
     * (The bigcomp Honor_FLT_ROUNDS blocks that also read bc.rounding are
     * deleted, so this value is only observed here.) */
    bc.rounding = 1;

    sign = 0;
    nz0 = 0;
    nz1 = 0;
    nz = 0;
    bc.dplen = 0;
    bc.uflchk = 0;
    rv = 0.0; /* dval(&rv) = 0.; */

    // The whole body funnels to `'ret` (the C `ret:` label). Exits `ovfl`,
    // `undfl`, `range_err`, `ret0` are modelled with nested labelled blocks.
    'ret: {
        // ------------------------------------------------------------------
        // leading whitespace / sign scan (the `for(s = s00;;s++) switch(*s)`)
        // ------------------------------------------------------------------
        'ret0: {
            'break2: {
                s = s00;
                loop {
                    match *s {
                        b'-' => {
                            sign = 1;
                            /* fall through to '+' */
                            s = s.add(1);
                            if *s != 0 {
                                break 'break2;
                            }
                            break 'ret0;
                        }
                        b'+' => {
                            s = s.add(1);
                            if *s != 0 {
                                break 'break2;
                            }
                            break 'ret0;
                        }
                        0 => {
                            break 'ret0;
                        }
                        b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b' ' => {
                            s = s.add(1);
                            continue;
                        }
                        _ => {
                            break 'break2;
                        }
                    }
                }
            }
            /* break2: */
            if *s == b'0' {
                /* NO_HEX_FP undefined */
                match *s.add(1) {
                    b'x' | b'X' => {
                        /* Honor_FLT_ROUNDS undefined */
                        let mut sp: *const c_char = s as *const c_char;
                        gethex(
                            &mut sp as *mut *const c_char,
                            &mut rv as *mut f64 as *mut c_void,
                            1,
                            sign,
                        );
                        s = sp as *const u8;
                        break 'ret;
                    }
                    _ => {}
                }
                nz0 = 1;
                s = s.add(1);
                while *s == b'0' {
                    s = s.add(1);
                }
                if *s == 0 {
                    break 'ret;
                }
            }
            s0 = s;
            nd = 0;
            nf = 0;
            /* USE_BF96 */
            yz = 0;
            loop {
                c = *s as i32;
                if !(c >= '0' as i32 && c <= '9' as i32) {
                    break;
                }
                if nd < 19 {
                    yz = 10 * yz + (c - '0' as i32) as u64;
                }
                nd += 1;
                s = s.add(1);
            }
            nd0 = nd;
            bc.dp0 = s.offset_from(s0) as i32;
            bc.dp1 = bc.dp0;
            s1 = s;
            while s1 > s0 && {
                s1 = s1.offset(-1);
                *s1 == b'0'
            } {
                nz1 += 1;
            }
            /* USE_LOCALE undefined */
            if c == '.' as i32 {
                s = s.add(1);
                c = *s as i32;
                bc.dp1 = s.offset_from(s0) as i32;
                bc.dplen = bc.dp1 - bc.dp0;
                'skip_frac: {
                    if nd == 0 {
                        while c == '0' as i32 {
                            s = s.add(1);
                            c = *s as i32;
                            nz += 1;
                        }
                        if c > '0' as i32 && c <= '9' as i32 {
                            bc.dp0 = s0.offset_from(s) as i32;
                            bc.dp1 = bc.dp0 + bc.dplen;
                            s0 = s;
                            nf += nz;
                            nz = 0;
                            /* goto have_dig; */
                        } else {
                            break 'skip_frac; /* goto dig_done; */
                        }
                        // fall into have_dig path: emulate the for-loop body
                        // starting at the `have_dig:` label for the current c.
                        loop {
                            /* have_dig: */
                            nz += 1;
                            c -= '0' as i32;
                            if c != 0 {
                                nf += nz;
                                i = 1;
                                /* USE_BF96 */
                                while i < nz {
                                    nd += 1;
                                    if nd <= 19 {
                                        yz *= 10;
                                    }
                                    i += 1;
                                }
                                nd += 1;
                                if nd <= 19 {
                                    yz = 10 * yz + c as u64;
                                }
                                nz = 0;
                                nz1 = 0;
                            }
                            /* loop tail: c = *++s; test c in [0-9] */
                            s = s.add(1);
                            c = *s as i32;
                            if !(c >= '0' as i32 && c <= '9' as i32) {
                                break;
                            }
                        }
                        break 'skip_frac;
                    }
                    /* nd != 0: the plain `for(; c in [0-9]; c=*++s)` loop */
                    while c >= '0' as i32 && c <= '9' as i32 {
                        /* have_dig: */
                        nz += 1;
                        c -= '0' as i32;
                        if c != 0 {
                            nf += nz;
                            i = 1;
                            while i < nz {
                                nd += 1;
                                if nd <= 19 {
                                    yz *= 10;
                                }
                                i += 1;
                            }
                            nd += 1;
                            if nd <= 19 {
                                yz = 10 * yz + c as u64;
                            }
                            nz = 0;
                            nz1 = 0;
                        }
                        s = s.add(1);
                        c = *s as i32;
                    }
                }
            }
            /* dig_done: */
            e = 0;
            if c == 'e' as i32 || c == 'E' as i32 {
                if nd == 0 && nz == 0 && nz0 == 0 {
                    break 'ret0;
                }
                s00 = s;
                esign = 0;
                s = s.add(1);
                c = *s as i32;
                match c {
                    x if x == '-' as i32 => {
                        esign = 1;
                        s = s.add(1);
                        c = *s as i32;
                    }
                    x if x == '+' as i32 => {
                        s = s.add(1);
                        c = *s as i32;
                    }
                    _ => {}
                }
                if c >= '0' as i32 && c <= '9' as i32 {
                    while c == '0' as i32 {
                        s = s.add(1);
                        c = *s as i32;
                    }
                    if c > '0' as i32 && c <= '9' as i32 {
                        L = c - '0' as i32;
                        loop {
                            s = s.add(1);
                            c = *s as i32;
                            if !(c >= '0' as i32 && c <= '9' as i32) {
                                break;
                            }
                            if L <= 19999 {
                                L = 10 * L + (c - '0' as i32);
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
                    /* INFNAN_CHECK */
                    'infnan_done: {
                        if bc.dplen == 0 {
                            match c {
                                x if x == 'i' as i32 || x == 'I' as i32 => {
                                    let mut sp: *const u8 = s;
                                    if match_str(&mut sp, b"nf\0".as_ptr()) != 0 {
                                        s = sp;
                                        s = s.offset(-1);
                                        let mut sp2: *const u8 = s;
                                        if match_str(&mut sp2, b"inity\0".as_ptr()) == 0 {
                                            s = s.add(1);
                                        } else {
                                            s = sp2;
                                        }
                                        set_word0(&mut rv, 0x7ff00000);
                                        set_word1(&mut rv, 0);
                                        break 'ret;
                                    }
                                }
                                x if x == 'n' as i32 || x == 'N' as i32 => {
                                    let mut sp: *const u8 = s;
                                    if match_str(&mut sp, b"an\0".as_ptr()) != 0 {
                                        s = sp;
                                        set_word0(&mut rv, NAN_WORD0);
                                        set_word1(&mut rv, NAN_WORD1);
                                        /* No_Hex_NaN undefined */
                                        if *s == b'(' {
                                            let mut sp3: *const u8 = s;
                                            hexnan(&mut rv, &mut sp3);
                                            s = sp3;
                                        }
                                        break 'ret;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    break 'ret0;
                }
                break 'ret;
            }
            /* real number path: skip 'ret0' */
            e -= nf;
            bc.e0 = e;
            e1 = e;

            /* Now we have nd0 digits, starting at s0, followed by a decimal
             * point, followed by nd-nd0 digits. */

            if nd0 == 0 {
                nd0 = nd;
            }
            /* USE_BF96 defined => the non-USE_BF96 `k`/`dval(&rv)` block is
             * deleted. */
            bd0 = None;
            if nd <= DBL_DIG
                /* RND_PRODQUOT undefined, Honor_FLT_ROUNDS undefined */
                && FLT_ROUNDS == 1
            {
                /* USE_BF96 */
                rv = yz as f64;
                if e == 0 {
                    break 'ret;
                }
                /* ROUND_BIASED_without_Round_Up undefined */
                if e > 0 {
                    if e <= TEN_PMAX {
                        /* VAX undefined, Honor_FLT_ROUNDS undefined */
                        rv *= tens[e as usize]; /* rounded_product */
                        break 'ret;
                    }
                    i = DBL_DIG - nd;
                    if e <= TEN_PMAX + i {
                        /* A fancier test would sometimes let us do this for
                         * larger i values. */
                        e -= i;
                        rv *= tens[i as usize];
                        /* VAX undefined */
                        rv *= tens[e as usize]; /* rounded_product */
                        break 'ret;
                    }
                }
                /* Inaccurate_Divide undefined */
                else if e >= -TEN_PMAX {
                    /* Honor_FLT_ROUNDS undefined */
                    rv /= tens[(-e) as usize]; /* rounded_quotient */
                    break 'ret;
                }
            }
            /* USE_BF96 */
            k = if nd < 19 { nd } else { 19 };
            e1 += nd - k; /* scale factor = 10^e1 */

            /* IEEE_Arith, SET_INEXACT undefined, Honor_FLT_ROUNDS undefined */

            // ==============================================================
            //  USE_BF96 fast path (dtoa.c lines 3866-4083)
            // ==============================================================
            'undfl: {
                'ovfl: {
                    'many_digits: {
                        /* Debug(++dtoa_stats[0]); deleted */
                        i = e1 + 342;
                        if i < 0 {
                            break 'undfl; /* goto undfl */
                        }
                        if i > 650 {
                            break 'ovfl; /* goto ovfl */
                        }
                        p10 = &PTEN[i as usize];
                        brv = yz;
                        /* shift brv left, with i = number of bits shifted */
                        i = 0;
                        if brv & 0xffffffff00000000u64 == 0 {
                            i = 32;
                            brv <<= 32;
                        }
                        if brv & 0xffff000000000000u64 == 0 {
                            i += 16;
                            brv <<= 16;
                        }
                        if brv & 0xff00000000000000u64 == 0 {
                            i += 8;
                            brv <<= 8;
                        }
                        if brv & 0xf000000000000000u64 == 0 {
                            i += 4;
                            brv <<= 4;
                        }
                        if brv & 0xc000000000000000u64 == 0 {
                            i += 2;
                            brv <<= 2;
                        }
                        if brv & 0x8000000000000000u64 == 0 {
                            i += 1;
                            brv <<= 1;
                        }
                        erv = (64 + 0x3fe) + p10.e - i;
                        if erv <= 0 && nd > 19 {
                            break 'many_digits; /* denormal: look at all digits */
                        }
                        bhi = brv >> 32;
                        blo = brv & 0xffffffffu64;
                        t01 = bhi.wrapping_mul(p10.b1 as u64);
                        t10 = blo
                            .wrapping_mul(p10.b0 as u64)
                            .wrapping_add(t01 & 0xffffffffu64);
                        t00 = bhi
                            .wrapping_mul(p10.b0 as u64)
                            .wrapping_add(t01 >> 32)
                            .wrapping_add(t10 >> 32);

                        // The many jump targets in this section (`denormal`,
                        // `roundup`, `noround`, `denormal1`, `roundup1`,
                        // `noround1`) are reachable both from the 3-multiply
                        // stage and the 96-bit stage, so they are modelled with
                        // an explicit goto-dispatcher.
                        #[derive(PartialEq)]
                        enum G {
                            Fresh,
                            Denormal,
                            Roundup,
                            Noround,
                            Denormal1,
                            Roundup1,
                            Noround1,
                        }
                        let mut goto = G::Fresh;

                        'dispatch: loop {
                            if goto == G::Fresh {
                                if t00 & 0x8000000000000000u64 != 0 {
                                    if (t00 & 0x3ff) != 0 && (!t00 & 0x3fe) != 0 {
                                        /* unambiguous result? */
                                        if nd > 19
                                            && (((t00.wrapping_add(1u64 << i).wrapping_add(2))
                                                & 0x400)
                                                ^ (t00 & 0x400))
                                                != 0
                                        {
                                            break 'many_digits;
                                        }
                                        if erv <= 0 {
                                            goto = G::Denormal;
                                            continue 'dispatch;
                                        }
                                        /* Honor_FLT_ROUNDS switch deleted */
                                        if t00 & 0x400 != 0 && t00 & 0xbff != 0 {
                                            goto = G::Roundup;
                                            continue 'dispatch;
                                        }
                                        goto = G::Noround;
                                        continue 'dispatch;
                                    }
                                } else {
                                    if (t00 & 0x1ff) != 0 && (!t00 & 0x1fe) != 0 {
                                        /* unambiguous result? */
                                        if nd > 19
                                            && (((t00.wrapping_add(1u64 << i).wrapping_add(2))
                                                & 0x200)
                                                ^ (t00 & 0x200))
                                                != 0
                                        {
                                            break 'many_digits;
                                        }
                                        if erv <= 1 {
                                            goto = G::Denormal1;
                                            continue 'dispatch;
                                        }
                                        if t00 & 0x200 != 0 {
                                            goto = G::Roundup1;
                                            continue 'dispatch;
                                        }
                                        goto = G::Noround1;
                                        continue 'dispatch;
                                    }
                                }
                                /* 3 multiplies did not suffice; try a 96-bit approx */
                                t02 = bhi.wrapping_mul(p10.b2 as u64);
                                t11 = blo
                                    .wrapping_mul(p10.b1 as u64)
                                    .wrapping_add(t02 & 0xffffffffu64);
                                bexact = 1;
                                if e1 < 0
                                    || e1 > 41
                                    || (t10 | t11) & 0xffffffffu64 != 0
                                    || nd > 19
                                {
                                    bexact = 0;
                                }
                                tlo = (t10 & 0xffffffffu64)
                                    .wrapping_add(t02 >> 32)
                                    .wrapping_add(t11 >> 32);
                                if bexact == 0 && (tlo.wrapping_add(0x10)) >> 32 > tlo >> 32 {
                                    break 'many_digits;
                                }
                                t00 = t00.wrapping_add(tlo >> 32);
                                if t00 & 0x8000000000000000u64 != 0 {
                                    if erv <= 0 {
                                        /* denormal result */
                                        if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x3ff)) == 0 {
                                            break 'many_digits;
                                        }
                                        goto = G::Denormal;
                                        continue 'dispatch;
                                    }
                                    if bexact != 0 {
                                        /* SET_INEXACT undefined; Honor_FLT_ROUNDS deleted */
                                        if t00 & 0x400 != 0
                                            && ((tlo & 0xffffffff) | (t00 & 0xbff)) != 0
                                        {
                                            goto = G::Roundup;
                                            continue 'dispatch;
                                        }
                                        goto = G::Noround;
                                        continue 'dispatch;
                                    }
                                    if ((tlo & 0xfffffff0) | (t00 & 0x3ff)) != 0
                                        && (nd <= 19
                                            || (t00.wrapping_add(1u64 << i)
                                                & 0xfffffffffffffc00u64)
                                                == (t00 & 0xfffffffffffffc00u64))
                                    {
                                        /* Unambiguous result. */
                                        if t00 & 0x400 != 0 {
                                            goto = G::Roundup;
                                            continue 'dispatch;
                                        }
                                        goto = G::Noround;
                                        continue 'dispatch;
                                    }
                                    break 'many_digits;
                                } else {
                                    if erv <= 1 {
                                        /* denormal result */
                                        if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x1ff)) == 0 {
                                            break 'many_digits;
                                        }
                                        goto = G::Denormal1;
                                        continue 'dispatch;
                                    }
                                    if bexact != 0 {
                                        if t00 & 0x200 != 0 && (t00 & 0x5ff != 0 || tlo != 0) {
                                            goto = G::Roundup1;
                                            continue 'dispatch;
                                        }
                                        goto = G::Noround1;
                                        continue 'dispatch;
                                    }
                                    if ((tlo & 0xfffffff0) | (t00 & 0x1ff)) != 0
                                        && (nd <= 19
                                            || (t00.wrapping_add(1u64 << i)
                                                & 0x7ffffffffffffe00u64)
                                                == (t00 & 0x7ffffffffffffe00u64))
                                    {
                                        /* Unambiguous result. */
                                        if t00 & 0x200 != 0 {
                                            goto = G::Roundup1;
                                            continue 'dispatch;
                                        }
                                        goto = G::Noround1;
                                        continue 'dispatch;
                                    }
                                    break 'many_digits;
                                }
                            }
                            /* --- label targets --- */
                            if goto == G::Denormal {
                                /* denormal: */
                                if erv <= -52 {
                                    if erv < -52 || (t00 & 0x7fffffffffffffffu64) == 0 {
                                        break 'undfl;
                                    }
                                    /* goto tiniest */
                                    rv = f64::from_bits(1);
                                    set_errno(ERANGE);
                                    break 'ret;
                                }
                                tg = 1u64 << (11 - erv);
                                t00 &= !(tg - 1); /* clear low bits */
                                if t00 & tg != 0 {
                                    /* roundup_den: */
                                    t00 = t00.wrapping_add(tg << 1);
                                    if t00 & 0x8000000000000000u64 == 0 {
                                        erv += 1;
                                        if erv > 0 {
                                            /* goto smallest_normal */
                                            rv = f64::from_bits(0x0010000000000000u64);
                                            break 'ret;
                                        }
                                        t00 = 0x8000000000000000u64;
                                    }
                                }
                                /* noround_den: */
                                rv = f64::from_bits(t00 >> (12 - erv));
                                set_errno(ERANGE);
                                break 'ret;
                            }
                            if goto == G::Roundup {
                                /* roundup: */
                                t00 = t00.wrapping_add(0x800);
                                if t00 & 0x8000000000000000u64 == 0 {
                                    /* rounded up to a power of 2 */
                                    if erv >= 0x7fe {
                                        break 'ovfl;
                                    }
                                    terv = (erv + 1) as u64;
                                    rv = f64::from_bits(terv << 52);
                                    break 'ret;
                                }
                                goto = G::Noround;
                                /* fall through to noround */
                            }
                            if goto == G::Noround {
                                /* noround: */
                                if erv >= 0x7ff {
                                    break 'ovfl;
                                }
                                terv = erv as u64;
                                rv = f64::from_bits(
                                    (terv << 52) | ((t00 & 0x7ffffffffffff800u64) >> 11),
                                );
                                break 'ret;
                            }
                            if goto == G::Denormal1 {
                                /* denormal1: */
                                if erv <= -51 {
                                    if erv < -51 || (t00 & 0x3fffffffffffffffu64) == 0 {
                                        break 'undfl;
                                    }
                                    /* tiniest: */
                                    rv = f64::from_bits(1);
                                    set_errno(ERANGE);
                                    break 'ret;
                                }
                                tg = 1u64 << (11 - erv);
                                if t00 & tg != 0 {
                                    /* roundup1_den: */
                                    t00 = t00.wrapping_add(tg << 1);
                                    if 0x8000000000000000u64 & t00 != 0 && erv == 1 {
                                        /* smallest_normal: */
                                        rv = f64::from_bits(0x0010000000000000u64);
                                        break 'ret;
                                    }
                                }
                                /* noround1_den: */
                                if erv <= -52 {
                                    break 'undfl;
                                }
                                rv = f64::from_bits(t00 >> (12 - erv));
                                set_errno(ERANGE);
                                break 'ret;
                            }
                            if goto == G::Roundup1 {
                                /* roundup1: */
                                t00 = t00.wrapping_add(0x400);
                                if t00 & 0x4000000000000000u64 == 0 {
                                    /* rounded up to a power of 2 */
                                    if erv >= 0x7ff {
                                        break 'ovfl;
                                    }
                                    terv = erv as u64;
                                    rv = f64::from_bits(terv << 52);
                                    break 'ret;
                                }
                                goto = G::Noround1;
                                /* fall through to noround1 */
                            }
                            if goto == G::Noround1 {
                                /* noround1: */
                                if erv >= 0x800 {
                                    break 'ovfl;
                                }
                                terv = (erv - 1) as u64;
                                rv = f64::from_bits(
                                    (terv << 52) | ((t00 & 0x3ffffffffffffc00u64) >> 10),
                                );
                                break 'ret;
                            }
                            /* no dispatcher case matched — should not happen */
                            break 'many_digits;
                        }
                    }
                    /* many_digits: */
                    /* Debug(++dtoa_stats[2]); deleted */
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
                    rv = yz as f64;

                    // ----------------------------------------------------------
                    //  slow refinement path continues below (shared with the
                    //  e1-scaling and the big correction loop).
                    // ----------------------------------------------------------
                    /* IEEE_Arith + Avoid_Underflow */
                    bc.scale = 0;

                    /* Get starting approximation = rv * 10**e1 */

                    if e1 > 0 {
                        i = e1 & 15;
                        if i != 0 {
                            rv *= tens[i as usize];
                        }
                        e1 &= !15;
                        if e1 != 0 {
                            if e1 > DBL_MAX_10_EXP {
                                break 'ovfl; /* goto ovfl */
                            }
                            e1 >>= 4;
                            j = 0;
                            while e1 > 1 {
                                if e1 & 1 != 0 {
                                    rv *= bigtens[j as usize];
                                }
                                j += 1;
                                e1 >>= 1;
                            }
                            /* The last multiplication could overflow. */
                            { let __wv = word0(rv).wrapping_sub((P as u32) * EXP_MSK1); set_word0(&mut rv, __wv); };
                            rv *= bigtens[j as usize];
                            z = word0(rv) & EXP_MASK;
                            if z > EXP_MSK1 * ((DBL_MAX_EXP + BIAS - P) as u32) {
                                break 'ovfl;
                            }
                            if z > EXP_MSK1 * ((DBL_MAX_EXP + BIAS - 1 - P) as u32) {
                                /* set to largest number (Can't trust DBL_MAX) */
                                set_word0(&mut rv, BIG0);
                                set_word1(&mut rv, BIG1);
                            } else {
                                { let __wv = word0(rv).wrapping_add((P as u32) * EXP_MSK1); set_word0(&mut rv, __wv); };
                            }
                        }
                    } else if e1 < 0 {
                        e1 = -e1;
                        i = e1 & 15;
                        if i != 0 {
                            rv /= tens[i as usize];
                        }
                        e1 >>= 4;
                        if e1 != 0 {
                            if e1 >= (1 << N_BIGTENS) {
                                break 'undfl; /* goto undfl */
                            }
                            /* Avoid_Underflow */
                            if e1 & (SCALE_BIT as i32) != 0 {
                                bc.scale = 2 * P;
                            }
                            j = 0;
                            while e1 > 0 {
                                if e1 & 1 != 0 {
                                    rv *= tinytens[j as usize];
                                }
                                j += 1;
                                e1 >>= 1;
                            }
                            if bc.scale != 0 && {
                                j = 2 * P + 1 - (((word0(rv) & EXP_MASK) >> EXP_SHIFT) as i32);
                                j > 0
                            } {
                                /* scaled rv is denormal; clear j low bits */
                                if j >= 32 {
                                    if j > 54 {
                                        break 'undfl;
                                    }
                                    set_word1(&mut rv, 0);
                                    if j >= 53 {
                                        { let __wv = (P as u32 + 2) * EXP_MSK1; set_word0(&mut rv, __wv); };
                                    } else {
                                        { let __wv = word0(rv) & (0xffffffffu32.wrapping_shl((j - 32) as u32)); set_word0(&mut rv, __wv); };
                                    }
                                } else {
                                    { let __wv = word1(rv) & (0xffffffffu32.wrapping_shl(j as u32)); set_word1(&mut rv, __wv); };
                                }
                            }
                            /* else-branch (non-Avoid_Underflow) deleted */
                            if rv == 0.0 {
                                break 'undfl; /* goto undfl */
                            }
                        }
                    }

                    /* Now the hard part -- adjusting rv to the correct value. */

                    /* Put digits into bd: true value = bd * 10^e */

                    bc.nd = nd - nz1;
                    /* NO_STRTOD_BIGCOMP undefined */
                    bc.nd0 = nd0; /* Only needed if nd > strtod_diglim */
                    if nd > STRTOD_DIGLIM {
                        /* ASSERT(strtod_diglim >= 18) */
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
                            if *s0.add(j as usize) != b'0' {
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
                            /* must recompute y */
                            y = 0;
                            i = 0;
                            while i < nd0 {
                                y = 10 * y + (*s0.add(i as usize) as u32 - '0' as u32);
                                i += 1;
                            }
                            j = bc.dp1;
                            while i < nd {
                                y = 10 * y + (*s0.add(j as usize) as u32 - '0' as u32);
                                j += 1;
                                i += 1;
                            }
                        }
                    }
                    bd0 = Some(s2b(s0, nd0, nd, y, bc.dplen));

                    // ============================================================
                    //  the correction loop `for(;;) { ... }`
                    // ============================================================
                    'correction: loop {
                        bd = Some({
                            let src = bd0.as_ref().unwrap();
                            let mut nb = balloc(src.k);
                            bcopy(&mut nb, src);
                            nb
                        });
                        {
                            let (nbb, e_, bits_) = crate::dtoa::d2b(rv.to_bits());
                            bb = Some(nbb);
                            bbe = e_;
                            bbbits = bits_;
                        } /* rv = bb * 2^bbe */
                        bs = Some(i2b(1));

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
                        /* Honor_FLT_ROUNDS undefined */
                        /* Avoid_Underflow */
                        Lsb = LSB;
                        Lsb1 = 0;
                        j = bbe - bc.scale;
                        i = j + bbbits - 1; /* logb(rv) */
                        j = P + 1 - bbbits;
                        if i < EMIN {
                            /* denormal */
                            i = EMIN - i;
                            j -= i;
                            if i < 32 {
                                Lsb <<= i;
                            } else if i < 52 {
                                Lsb1 = Lsb << (i - 32);
                            } else {
                                Lsb1 = EXP_MASK;
                            }
                        }
                        bb2 += j;
                        bd2 += j;
                        /* Avoid_Underflow */
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
                            bs = Some(pow5mult(bs.take().unwrap(), bb5));
                            let bb1v = mult(bs.as_ref().unwrap(), bb.as_ref().unwrap());
                            /* Bfree(bb); bb = bb1; */
                            bb = Some(bb1v);
                        }
                        if bb2 > 0 {
                            bb = Some(lshift(bb.take().unwrap(), bb2));
                        }
                        if bd5 > 0 {
                            bd = Some(pow5mult(bd.take().unwrap(), bd5));
                        }
                        if bd2 > 0 {
                            bd = Some(lshift(bd.take().unwrap(), bd2));
                        }
                        if bs2 > 0 {
                            bs = Some(lshift(bs.take().unwrap(), bs2));
                        }
                        delta = Some(diff(bb.as_ref().unwrap(), bd.as_ref().unwrap()));
                        bc.dsign = delta.as_ref().unwrap().sign;
                        delta.as_mut().unwrap().sign = 0;
                        i = cmp(delta.as_ref().unwrap(), bs.as_ref().unwrap());
                        /* NO_STRTOD_BIGCOMP undefined */
                        if bc.nd > nd && i <= 0 {
                            if bc.dsign != 0 {
                                /* Must use bigcomp(). */
                                req_bigcomp = 1;
                                break 'correction;
                            }
                            /* Honor_FLT_ROUNDS undefined => else */
                            i = -1; /* Discarded digits make delta smaller. */
                        }
                        /* Honor_FLT_ROUNDS block deleted */

                        // `break 'cont` == C `goto cont`; `break 'correction`
                        // == C `break`.  `break 'drop_down` == C `goto drop_down`.
                        'cont: {
                            'aadj: {
                                'drop_down: {
                                    if i < 0 {
                                        /* Error is less than half an ulp --
                                         * check for special case of mantissa a
                                         * power of two. */
                                        if bc.dsign != 0
                                            || word1(rv) != 0
                                            || word0(rv) & BNDRY_MASK != 0
                                            /* IEEE_Arith + Avoid_Underflow */
                                            || (word0(rv) & EXP_MASK)
                                                <= (2 * P as u32 + 1) * EXP_MSK1
                                        {
                                            /* SET_INEXACT undefined */
                                            break 'correction;
                                        }
                                        if delta.as_ref().unwrap().x[0] == 0
                                            && delta.as_ref().unwrap().wds <= 1
                                        {
                                            /* exact result */
                                            break 'correction;
                                        }
                                        delta = Some(lshift(delta.take().unwrap(), LOG2P));
                                        if cmp(delta.as_ref().unwrap(), bs.as_ref().unwrap()) > 0 {
                                            break 'drop_down;
                                        }
                                        break 'correction;
                                    }
                                    if i == 0 {
                                        /* exactly half-way between */
                                        if bc.dsign != 0 {
                                            if (word0(rv) & BNDRY_MASK1) == BNDRY_MASK1
                                                && word1(rv)
                                                    == ({
                                                        /* Avoid_Underflow */
                                                        if bc.scale != 0 && {
                                                            y = word0(rv) & EXP_MASK;
                                                            y <= 2 * P as u32 * EXP_MSK1
                                                        } {
                                                            0xffffffffu32
                                                                & (0xffffffffu32.wrapping_shl(
                                                                    (2 * P as u32 + 1
                                                                        - (y >> EXP_SHIFT))
                                                                        as u32,
                                                                ))
                                                        } else {
                                                            0xffffffffu32
                                                        }
                                                    })
                                            {
                                                /* boundary case -- increment exponent */
                                                if word0(rv) == BIG0 && word1(rv) == BIG1 {
                                                    break 'ovfl;
                                                }
                                                { let __wv = (word0(rv) & EXP_MASK) + EXP_MSK1; set_word0(&mut rv, __wv); };
                                                set_word1(&mut rv, 0);
                                                /* Avoid_Underflow */
                                                bc.dsign = 0;
                                                break 'correction;
                                            }
                                        } else if (word0(rv) & BNDRY_MASK) == 0 && word1(rv) == 0 {
                                            /* drop_down: fall through */
                                            break 'drop_down;
                                        }
                                        /* ROUND_BIASED undefined */
                                        /* Avoid_Underflow */
                                        if Lsb1 != 0 {
                                            if word0(rv) & Lsb1 == 0 {
                                                break 'correction;
                                            }
                                        } else if word1(rv) & Lsb == 0 {
                                            break 'correction;
                                        }
                                        if bc.dsign != 0 {
                                            /* Avoid_Underflow */
                                            rv += sulp(rv, &bc);
                                        }
                                        /* ROUND_BIASED undefined => else */
                                        else {
                                            /* Avoid_Underflow */
                                            rv -= sulp(rv, &bc);
                                            /* Sudden_Underflow undefined */
                                            if rv == 0.0 {
                                                if bc.nd > nd {
                                                    bc.uflchk = 1;
                                                    break 'correction;
                                                }
                                                break 'undfl;
                                            }
                                        }
                                        /* Avoid_Underflow */
                                        bc.dsign = 1 - bc.dsign;
                                        break 'correction;
                                    }
                                    /* i > 0 : fall through to aadj code */
                                    break 'aadj;
                                }
                                /* drop_down: (boundary case -- decrement exponent) */
                                /* Sudden_Underflow undefined; Avoid_Underflow */
                                if bc.scale != 0 {
                                    L = (word0(rv) & EXP_MASK) as i32;
                                    if (L as u32) <= (2 * P as u32 + 1) * EXP_MSK1 {
                                        if (L as u32) > (P as u32 + 2) * EXP_MSK1 {
                                            /* round even ==> accept rv */
                                            break 'correction;
                                        }
                                        /* rv = smallest denormal */
                                        if bc.nd > nd {
                                            bc.uflchk = 1;
                                            break 'correction;
                                        }
                                        break 'undfl;
                                    }
                                }
                                L = ((word0(rv) & EXP_MASK) - EXP_MSK1) as i32;
                                { let __wv = (L as u32) | BNDRY_MASK1; set_word0(&mut rv, __wv); };
                                set_word1(&mut rv, 0xffffffff);
                                /* IBM undefined; NO_STRTOD_BIGCOMP undefined */
                                if bc.nd > nd {
                                    break 'cont;
                                }
                                break 'correction;
                            }
                            /* aadj: i > 0, more than half an ulp away */
                            aadj = ratio(delta.as_ref().unwrap(), bs.as_ref().unwrap());
                            if aadj <= 2.0 {
                                if bc.dsign != 0 {
                                    aadj = 1.0;
                                    aadj1 = 1.0;
                                } else if word1(rv) != 0 || word0(rv) & BNDRY_MASK != 0 {
                                    /* Sudden_Underflow undefined */
                                    if word1(rv) == TINY1 && word0(rv) == 0 {
                                        if bc.nd > nd {
                                            bc.uflchk = 1;
                                            break 'correction;
                                        }
                                        break 'undfl;
                                    }
                                    aadj = 1.0;
                                    aadj1 = -1.0;
                                } else {
                                    /* special case -- power of FLT_RADIX to be
                                     * rounded down... */
                                    if aadj < 2.0 / FLT_RADIX as f64 {
                                        aadj = 1.0 / FLT_RADIX as f64;
                                    } else {
                                        aadj *= 0.5;
                                    }
                                    aadj1 = -aadj;
                                }
                            } else {
                                aadj *= 0.5;
                                aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                                /* Check_FLT_ROUNDS defined */
                                match bc.rounding {
                                    2 => {
                                        /* towards +infinity */
                                        aadj1 -= 0.5;
                                    }
                                    0 | 3 => {
                                        /* towards 0 / -infinity */
                                        aadj1 += 0.5;
                                    }
                                    _ => {}
                                }
                            }
                            y = word0(rv) & EXP_MASK;

                            /* Check for overflow */

                            if y == EXP_MSK1 * ((DBL_MAX_EXP + BIAS - 1) as u32) {
                                rv0 = rv;
                                { let __wv = word0(rv).wrapping_sub((P as u32) * EXP_MSK1); set_word0(&mut rv, __wv); };
                                adj = aadj1 * ulp(rv);
                                rv += adj;
                                if (word0(rv) & EXP_MASK)
                                    >= EXP_MSK1 * ((DBL_MAX_EXP + BIAS - P) as u32)
                                {
                                    if word0(rv0) == BIG0 && word1(rv0) == BIG1 {
                                        break 'ovfl;
                                    }
                                    set_word0(&mut rv, BIG0);
                                    set_word1(&mut rv, BIG1);
                                    break 'cont;
                                } else {
                                    { let __wv = word0(rv).wrapping_add((P as u32) * EXP_MSK1); set_word0(&mut rv, __wv); };
                                }
                            } else {
                                /* Avoid_Underflow */
                                if bc.scale != 0 && y <= 2 * P as u32 * EXP_MSK1 {
                                    if aadj <= 0x7fffffff as f64 {
                                        z = aadj as u32;
                                        if (z as i32) <= 0 {
                                            z = 1;
                                        }
                                        aadj = z as f64;
                                        aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                                    }
                                    aadj2 = aadj1;
                                    { let __wv = word0(aadj2)
                                            .wrapping_add((2 * P as u32 + 1) * EXP_MSK1 - y); set_word0(&mut aadj2, __wv); };
                                    aadj1 = aadj2;
                                    adj = aadj1 * ulp(rv);
                                    rv += adj;
                                    if rv == 0.0 {
                                        /* NO_STRTOD_BIGCOMP undefined */
                                        req_bigcomp = 1;
                                        break 'correction;
                                    }
                                } else {
                                    adj = aadj1 * ulp(rv);
                                    rv += adj;
                                }
                            }
                            z = word0(rv) & EXP_MASK;
                            /* SET_INEXACT undefined */
                            if bc.nd == nd {
                                /* Avoid_Underflow */
                                if bc.scale == 0 {
                                    if y == z {
                                        /* Can we stop now? */
                                        L = aadj as i32; /* (Long)aadj */
                                        aadj -= L as f64;
                                        /* The tolerances below are conservative. */
                                        if bc.dsign != 0
                                            || word1(rv) != 0
                                            || word0(rv) & BNDRY_MASK != 0
                                        {
                                            if aadj < 0.4999999 || aadj > 0.5000001 {
                                                break 'correction;
                                            }
                                        } else if aadj < 0.4999999 / FLT_RADIX as f64 {
                                            break 'correction;
                                        }
                                    }
                                }
                            }
                            /* fall through to cont */
                        }
                        /* cont: */
                        /* Bfree(bb); Bfree(bd); Bfree(bs); Bfree(delta); */
                        bb = None;
                        bd = None;
                        bs = None;
                        delta = None;
                    } // 'correction loop

                    /* after the loop: free bigints */
                    bb = None;
                    bd = None;
                    bs = None;
                    bd0 = None;
                    delta = None;
                    /* NO_STRTOD_BIGCOMP undefined */
                    if req_bigcomp != 0 {
                        bd0 = None;
                        bc.e0 += nz1;
                        bigcomp(&mut rv, s0, &mut bc);
                        y = word0(rv) & EXP_MASK;
                        if y == EXP_MASK {
                            break 'ovfl;
                        }
                        if y == 0 && rv == 0.0 {
                            break 'undfl;
                        }
                    }
                    /* Avoid_Underflow */
                    if bc.scale != 0 {
                        { let __wv = EXP_1.wrapping_sub(2 * P as u32 * EXP_MSK1); set_word0(&mut rv0, __wv); };
                        set_word1(&mut rv0, 0);
                        rv *= rv0;
                        /* NO_ERRNO undefined; IEEE_Arith */
                        if word0(rv) & EXP_MASK == 0 {
                            set_errno(ERANGE);
                        }
                    }
                    break 'ret;
                } /* 'ovfl block end */
                /* ovfl: */
                /* Can't trust HUGE_VAL */
                /* IEEE_Arith, Honor_FLT_ROUNDS undefined */
                set_word0(&mut rv, EXP_MASK);
                set_word1(&mut rv, 0);
                /* SET_INEXACT undefined */
                /* range_err: */
                /* bd0 frees are no-ops (drop) */
                bb = None;
                bd = None;
                bs = None;
                bd0 = None;
                delta = None;
                set_errno(ERANGE);
                break 'ret;
            } /* 'undfl block end */
            /* undfl: */
            rv = 0.0; /* dval(&rv) = 0.; */
            /* Honor_FLT_ROUNDS undefined */
            /* goto range_err; */
            bb = None;
            bd = None;
            bs = None;
            bd0 = None;
            delta = None;
            set_errno(ERANGE);
            break 'ret;
        } /* 'ret0 block end */
        /* ret0: */
        s = s00;
        sign = 0;
        /* fall through to ret */
    } /* 'ret block end */
    /* ret: */
    /* SET_INEXACT undefined */
    if !se.is_null() {
        *se = s as *mut c_char;
    }
    if sign != 0 {
        -rv
    } else {
        rv
    }
}
