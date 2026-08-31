//! Faithful, line-for-line translation of a section of David M. Gay's `dtoa.c`.
//!
//! Source ranges translated (from `c_src/src/dtoa.c`):
//!   * `s2b`     (lines 1719-1750)
//!   * `ulp`     (lines 2182-2214)
//!   * `b2d`     (lines 2217-2290, Pack_32 / non-VAX / non-IBM branch)
//!   * `ratio`   (lines 2411-2445, non-IBM branch)
//!   * tables `tens`, `bigtens`, `tinytens`, plus `SCALE_BIT` / `N_BIGTENS`
//!     (lines 2447-2470)
//!   * `match`   (lines 2551-2564, translated as `match_str`)
//!   * `hexnan`  (lines 2567-2626, the `#else` / non-GDTOA_NON_PEDANTIC_NANCHECK
//!     branch)
//!
//! Active configuration (exactly these, all others deleted):
//!   DEFINED:  IEEE_8087, IEEE_Arith, Pack_32, ULLong, USE_BF96,
//!             Avoid_Underflow, INFNAN_CHECK, Need_Hexdig, Check_FLT_ROUNDS.
//!   NOT DEF:  NO_HEX_FP, No_Hex_NaN, GDTOA_NON_PEDANTIC_NANCHECK,
//!             NO_STRTOD_BIGCOMP, MULTIPLE_THREADS, Honor_FLT_ROUNDS,
//!             SET_INEXACT, USE_LOCALE, Sudden_Underflow, ROUND_BIASED, VAX,
//!             IBM, DEBUG, Omit_Private_Memory, RND_PRODQUOT, NO_ERRNO,
//!             Bad_float_h, KR_headers, Flush_Denorm, NO_IEEE_Scale,
//!             NO_INFNAN_CHECK, ROUND_BIASED_without_Round_Up,
//!             Trust_FLT_ROUNDS, NO_STRTOD_DIGLIM.
//!   Consequently the `MTd`/`MTa`/`MTb` macro arguments expand to nothing,
//!   `Debug(x)` expands to nothing, `Set_errno(x)` is `errno = x`, and
//!   `Rounding == Flt_Rounds == 1`.
//!
//! IEEE_8087 double-precision constants (dtoa.c lines 1360-1400, 1494-1495)
//! used below: Exp_shift=20, Exp_msk1=0x100000, Exp_mask=0x7ff00000, P=53,
//! Bias=1023, Exp_1=0x3ff00000, Ebits=11, Frac_mask=0xfffff.
//!
//! Union `U { double d; ULong L[2]; ULLong LL; }` with IEEE_8087 means
//! `word0(x)` is `L[1]` (HIGH 32 bits) and `word1(x)` is `L[0]` (LOW 32 bits);
//! `dval(x)` is `d`. Here `U` is represented as an `f64` with helper accessors
//! backed by `f64::to_bits` / `f64::from_bits`.
//!
//! Bigint API note: the `d2b` in `dtoa.rs` has signature
//!   `pub fn d2b(ull: u64) -> (Bigint, i32, i32)`
//! i.e. it takes the raw 64-bit pattern of the double (`LLval`) and returns the
//! tuple `(b, e, bits)` rather than writing `*e`/`*bits` through out-pointers.
//! `d2b` is not used by any function in this module, but this is the verified
//! signature and semantics.

use crate::dtoa::{balloc, hi0bits, multadd, Bigint};
use crate::dtoa_hex::hexdig;

/* ----------------------------------------------------------------- U union */

/// `word0(x)` — the HIGH 32 bits of the double (IEEE_8087: `L[1]`).
#[inline(always)]
pub fn word0(u: f64) -> u32 {
    (u.to_bits() >> 32) as u32
}

/// `word1(x)` — the LOW 32 bits of the double (IEEE_8087: `L[0]`).
#[inline(always)]
pub fn word1(u: f64) -> u32 {
    (u.to_bits() & 0xffff_ffff) as u32
}

/// `word0(x) = v;`
#[inline(always)]
pub fn set_word0(u: &mut f64, v: u32) {
    let bits = u.to_bits();
    *u = f64::from_bits(((v as u64) << 32) | (bits & 0xffff_ffff));
}

/// `word1(x) = v;`
#[inline(always)]
pub fn set_word1(u: &mut f64, v: u32) {
    let bits = u.to_bits();
    *u = f64::from_bits((bits & 0xffff_ffff_0000_0000) | (v as u64));
}

/* -------------------------------------------------- IEEE_8087 constants */

pub const EXP_SHIFT: i32 = 20;
pub const EXP_MSK1: u32 = 0x100000;
pub const EXP_MASK: u32 = 0x7ff00000;
pub const P: i32 = 53;
pub const EXP_1: u32 = 0x3ff00000;
pub const EBITS: i32 = 11;

/* ----------------------------------------------------------------- s2b */

/// `static Bigint *s2b(const char *s, int nd0, int nd, ULong y9, int dplen)`
///
/// dtoa.c lines 1719-1750, Pack_32 branch. `s` is walked as a raw `*const u8`
/// exactly as the C `const char *`.
pub unsafe fn s2b(mut s: *const u8, nd0: i32, nd: i32, y9: u32, dplen: i32) -> Bigint {
    let mut b: Bigint;
    let mut i: i32;
    let k: i32;
    let x: i64;
    let mut y: i64;

    x = (nd as i64 + 8) / 9;
    {
        let mut kk = 0;
        y = 1;
        while x > y {
            y <<= 1;
            kk += 1;
        }
        k = kk;
    }
    // Pack_32
    b = balloc(k);
    b.x[0] = y9;
    b.wds = 1;

    i = 9;
    if 9 < nd0 {
        s = s.add(9);
        loop {
            let ch = *s as i32;
            s = s.add(1);
            b = multadd(b, 10, ch - '0' as i32);
            i += 1;
            if !(i < nd0) {
                break;
            }
        }
        s = s.add(dplen as usize);
    } else {
        s = s.add((dplen + 9) as usize);
    }
    while i < nd {
        let ch = *s as i32;
        s = s.add(1);
        b = multadd(b, 10, ch - '0' as i32);
        i += 1;
    }
    let _ = s;
    b
}

/* ----------------------------------------------------------------- ulp */

/// `static double ulp(U *x)`
///
/// dtoa.c lines 2182-2214. With `Avoid_Underflow` defined, only the first
/// branch remains (the `#ifndef Avoid_Underflow` guarded alternatives are
/// deleted). `IBM` is not defined, so the `L |= Exp_msk1 >> 4;` line is gone.
pub fn ulp(x: f64) -> f64 {
    let l: i64;
    let mut u: f64 = 0.0;

    l = (word0(x) & EXP_MASK) as i64 - (P as i64 - 1) * EXP_MSK1 as i64;
    set_word0(&mut u, l as u32);
    set_word1(&mut u, 0);
    u // dval(&u)
}

/* ----------------------------------------------------------------- b2d */

/// `static double b2d(Bigint *a, int *e)`
///
/// dtoa.c lines 2217-2290, Pack_32 / non-VAX / non-IBM branch. Returns the
/// double; `*e` is returned through the `e` out-parameter. `d0`/`d1` are the
/// `#define d0 word0(&d)` / `#define d1 word1(&d)` macros.
pub fn b2d(a: &Bigint, e: &mut i32) -> f64 {
    let mut w: u32;
    let mut y: u32;
    let mut z: u32;
    let k: i32;
    let mut d: f64 = 0.0;

    // ULong *xa, *xa0; here modeled as indices into a.x.
    let xa0: i32 = 0;
    let mut xa: i32 = a.wds;
    xa -= 1;
    y = a.x[xa as usize];
    // Debug: if (!y) Bug("zero y in b2d");  -> deleted (DEBUG undefined)
    k = hi0bits(y);
    *e = 32 - k;
    // Pack_32
    if k < EBITS {
        // d0 = Exp_1 | y >> (Ebits - k);
        set_word0(&mut d, EXP_1 | (y >> (EBITS - k)));
        w = if xa > xa0 {
            xa -= 1;
            a.x[xa as usize]
        } else {
            0
        };
        // d1 = y << ((32-Ebits) + k) | w >> (Ebits - k);
        set_word1(
            &mut d,
            y.wrapping_shl(((32 - EBITS) + k) as u32) | (w >> (EBITS - k)),
        );
        let _ = &mut w;
        return d; // goto ret_d
    }
    z = if xa > xa0 {
        xa -= 1;
        a.x[xa as usize]
    } else {
        0
    };
    let kk = k - EBITS;
    if kk != 0 {
        // d0 = Exp_1 | y << k | z >> (32 - k);   (here k has been reduced)
        set_word0(&mut d, EXP_1 | y.wrapping_shl(kk as u32) | z.wrapping_shr((32 - kk) as u32));
        y = if xa > xa0 {
            xa -= 1;
            a.x[xa as usize]
        } else {
            0
        };
        // d1 = z << k | y >> (32 - k);
        set_word1(&mut d, z.wrapping_shl(kk as u32) | y.wrapping_shr((32 - kk) as u32));
    } else {
        set_word0(&mut d, EXP_1 | y);
        set_word1(&mut d, z);
    }
    let _ = &mut z;
    // ret_d:
    d // dval(&d)
}

/* ----------------------------------------------------------------- ratio */

/// `static double ratio(Bigint *a, Bigint *b)`
///
/// dtoa.c lines 2411-2445, non-IBM branch. Pack_32 => the `32*(...)` form.
pub fn ratio(a: &Bigint, b: &Bigint) -> f64 {
    let mut da: f64;
    let mut db: f64;
    let mut k: i32;
    let mut ka: i32 = 0;
    let mut kb: i32 = 0;

    da = b2d(a, &mut ka);
    db = b2d(b, &mut kb);
    // Pack_32
    k = ka - kb + 32 * (a.wds - b.wds);
    // non-IBM
    if k > 0 {
        let v = word0(da).wrapping_add((k as u32).wrapping_mul(EXP_MSK1));
        set_word0(&mut da, v);
    } else {
        k = -k;
        let v = word0(db).wrapping_add((k as u32).wrapping_mul(EXP_MSK1));
        set_word0(&mut db, v);
    }
    da / db // dval(&da) / dval(&db)
}

/* ----------------------------------------------------------------- tables */

/// dtoa.c lines 2447-2470. VAX undefined, so `tens` stops at `1e22`.
pub static tens: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

/// IEEE_Arith branch.
pub static bigtens: [f64; 5] = [1e16, 1e32, 1e64, 1e128, 1e256];

/// IEEE_Arith + Avoid_Underflow branch. `tinytens[4]` keeps the C expression
/// `9007199254740992.*9007199254740992.e-256` (= 2^106 * 1e-256).
pub static tinytens: [f64; 5] = [
    1e-16,
    1e-32,
    1e-64,
    1e-128,
    9007199254740992.0 * 9007199254740992.0e-256,
];

/// `#define Scale_Bit 0x10`
pub const SCALE_BIT: u32 = 0x10;
/// `#define n_bigtens 5`
pub const N_BIGTENS: usize = 5;

/* ----------------------------------------------------------------- match */

/// `static int match(const char **sp, const char *t)`
///
/// dtoa.c lines 2551-2564. Named `match_str` in Rust. `sp` walks a raw
/// `*const u8`; `t` is a NUL-terminated `*const u8`.
pub unsafe fn match_str(sp: &mut *const u8, mut t: *const u8) -> i32 {
    let mut c: i32;
    let mut d: i32;
    let mut s: *const u8 = *sp;

    loop {
        d = *t as i32;
        t = t.add(1);
        if d == 0 {
            break;
        }
        s = s.add(1);
        c = *s as i32;
        if c >= 'A' as i32 && c <= 'Z' as i32 {
            c += 'a' as i32 - 'A' as i32;
        }
        if c != d {
            return 0;
        }
    }
    *sp = s.add(1);
    1
}

/* ----------------------------------------------------------------- hexnan */

/// `static void hexnan(U *rvp, const char **sp)`
///
/// dtoa.c lines 2567-2626, taking the `#else` branch (i.e.
/// GDTOA_NON_PEDANTIC_NANCHECK NOT defined). `No_Hex_NaN` is undefined so the
/// function exists. `s` walks a raw `*const u8` exactly as the C
/// `const char *`; `*(const unsigned char*)p` reads are just `*p` on `*const u8`.
pub unsafe fn hexnan(rvp: &mut f64, sp: &mut *const u8) {
    let mut c: u32;
    let mut x: [u32; 2] = [0, 0];
    let mut s: *const u8;
    let mut c1: i32;
    let mut havedig: i32;
    let mut udx0: i32;
    let mut xshift: i32;

    /**** if (!hexdig['0']) hexdig_init(); ****/
    x[0] = 0;
    x[1] = 0;
    havedig = 0;
    xshift = 0;
    udx0 = 1;
    s = *sp;
    /* allow optional initial 0x or 0X */
    loop {
        c = *s.add(1) as u32;
        if !(c != 0 && c <= ' ' as u32) {
            break;
        }
        s = s.add(1);
    }
    if *s.add(1) == b'0' && (*s.add(2) == b'x' || *s.add(2) == b'X') {
        s = s.add(2);
    }
    's_loop: loop {
        s = s.add(1);
        c = *s as u32;
        if c == 0 {
            break;
        }
        c1 = hexdig(c as u8) as i32;
        if c1 != 0 {
            c = (c1 & 0xf) as u32;
        } else if c <= ' ' as u32 {
            if udx0 != 0 && havedig != 0 {
                udx0 = 0;
                xshift = 1;
            }
            continue;
        } else {
            // #else branch (GDTOA_NON_PEDANTIC_NANCHECK not defined)
            loop {
                if c == ')' as u32 {
                    *sp = s.add(1);
                    break;
                }
                s = s.add(1);
                c = *s as u32;
                if c == 0 {
                    break;
                }
            }
            break 's_loop;
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
        set_word0(rvp, EXP_MASK | x[0]);
        set_word1(rvp, x[1]);
    }
}
