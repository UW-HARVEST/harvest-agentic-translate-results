#![allow(dead_code, non_snake_case, non_upper_case_globals, unused_assignments, unused_mut, unused_variables, unused_parens, unused_labels)]
use crate::dtoa::*;
use crate::dtoa_strtod::*;
use crate::dtoa_tables::*;
use crate::libc;
use std::ffi::{c_char, c_int};

/* One variant per C label that is either the target of a backward goto or of
   gotos coming from several different nesting levels.  Pure forward jumps use
   Rust labelled blocks instead. */
#[derive(Copy, Clone, PartialEq)]
enum St {
    Main,
    Ret0,
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
    RangeErr,
    Undfl,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod__unused(s00: *const c_char, se: *mut *mut c_char) -> f64 {
    /* C: const char *s00 (reassigned by the function) */
    let mut s00 = s00;

    /* int bb2, bb5, bbe, bd2, bd5, bbbits, bs2, c, e, e1; */
    let mut bb2: c_int = 0;
    let mut bb5: c_int = 0;
    let mut bbe: c_int = 0; /* C leaves this uninitialised */
    let mut bd2: c_int = 0;
    let mut bd5: c_int = 0;
    let mut bbbits: c_int = 0; /* C leaves this uninitialised */
    let mut bs2: c_int = 0;
    let mut c: c_int = 0; /* C leaves this uninitialised */
    let mut e: c_int = 0;
    let mut e1: c_int = 0;
    /* int esign, i, j, k, nd, nd0, nf, nz, nz0, nz1, sign; */
    let mut esign: c_int = 0;
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    let mut k: c_int = 0;
    let mut nd: c_int = 0;
    let mut nd0: c_int = 0;
    let mut nf: c_int = 0;
    let mut nz: c_int;
    let mut nz0: c_int;
    let mut nz1: c_int;
    let mut sign: c_int;
    /* const char *s, *s0, *s1; */
    let mut s: *const c_char = std::ptr::null();
    let mut s0: *const c_char = std::ptr::null();
    let mut s1: *const c_char;
    /* double aadj, aadj1; */
    let mut aadj: f64 = 0.0;
    let mut aadj1: f64 = 0.0;
    /* int L; */
    let mut L: c_int = 0;
    /* U aadj2, adj, rv, rv0; */
    let mut aadj2 = U::new(); /* C leaves this uninitialised */
    let mut adj = U::new(); /* C leaves this uninitialised */
    let mut rv = U::new(); /* C leaves this uninitialised */
    let mut rv0 = U::new(); /* C leaves this uninitialised */
    /* ULong y, z; */
    let mut y: ULong = 0; /* C leaves this uninitialised */
    let mut z: ULong = 0; /* C leaves this uninitialised */
    /* BCinfo bc; */
    let mut bc: BCinfo = Default::default(); /* C leaves most fields uninitialised */
    /* Bigint *bb, *bb1, *bd, *bd0, *bs, *delta; */
    let mut bb: *mut Bigint = std::ptr::null_mut(); /* C leaves this uninitialised */
    let mut bb1: *mut Bigint = std::ptr::null_mut(); /* C leaves this uninitialised */
    let mut bd: *mut Bigint = std::ptr::null_mut(); /* C leaves this uninitialised */
    let mut bd0: *mut Bigint = std::ptr::null_mut(); /* C leaves this uninitialised */
    let mut bs: *mut Bigint = std::ptr::null_mut(); /* C leaves this uninitialised */
    let mut delta: *mut Bigint = std::ptr::null_mut(); /* C leaves this uninitialised */
    /* unsigned long long bhi, blo, brv, t00, t01, t02, t10, t11, terv, tg, tlo, yz; */
    let mut bhi: u64 = 0; /* C leaves this uninitialised */
    let mut blo: u64 = 0; /* C leaves this uninitialised */
    let mut brv: u64 = 0; /* C leaves this uninitialised */
    let mut t00: u64 = 0; /* C leaves this uninitialised */
    let mut t01: u64 = 0; /* C leaves this uninitialised */
    let mut t02: u64 = 0; /* C leaves this uninitialised */
    let mut t10: u64 = 0; /* C leaves this uninitialised */
    let mut t11: u64 = 0; /* C leaves this uninitialised */
    let mut terv: u64 = 0; /* C leaves this uninitialised */
    let mut tg: u64 = 0; /* C leaves this uninitialised */
    let mut tlo: u64 = 0; /* C leaves this uninitialised */
    let mut yz: u64 = 0; /* C leaves this uninitialised */
    /* const BF96 *p10; */
    let mut p10: *const BF96 = std::ptr::null(); /* C leaves this uninitialised */
    /* int bexact, erv; */
    let mut bexact: c_int = 0; /* C leaves this uninitialised */
    let mut erv: c_int = 0; /* C leaves this uninitialised */
    /* ULong Lsb, Lsb1; */
    let mut Lsb: ULong = 0; /* C leaves this uninitialised */
    let mut Lsb1: ULong = 0; /* C leaves this uninitialised */
    let mut req_bigcomp: c_int = 0;

    sign = 0;
    nz0 = 0;
    nz1 = 0;
    nz = 0;
    bc.dplen = 0;
    bc.uflchk = 0;
    rv.set_dval(0.);

    let mut st = St::Main;
    'ret: loop {
        match st {
            /* ------------------------------------------------------------ */
            St::Main => {
                /* for(s = s00;;s++) switch(*s) { ... } */
                'break2: {
                    s = s00;
                    loop {
                        let sw = *s as c_int;
                        if sw == '-' as c_int {
                            sign = 1;
                            /* FALLTHROUGH to case '+' */
                            s = s.add(1);
                            if *s != 0 {
                                break 'break2;
                            }
                            /* FALLTHROUGH to case 0 */
                            st = St::Ret0;
                            continue 'ret;
                        } else if sw == '+' as c_int {
                            s = s.add(1);
                            if *s != 0 {
                                break 'break2;
                            }
                            /* FALLTHROUGH to case 0 */
                            st = St::Ret0;
                            continue 'ret;
                        } else if sw == 0 {
                            st = St::Ret0;
                            continue 'ret;
                        } else if sw == 0x09
                            || sw == 0x0a
                            || sw == 0x0b
                            || sw == 0x0c
                            || sw == 0x0d
                            || sw == 0x20
                        {
                            /* continue; */
                        } else {
                            break 'break2;
                        }
                        s = s.add(1); /* s++ */
                    }
                }
                /* break2: */
                if *s as c_int == '0' as c_int {
                    let sw = *s.add(1) as c_int;
                    if sw == 'x' as c_int || sw == 'X' as c_int {
                        gethex(&mut s, &mut rv, 1, sign);
                        break 'ret;
                    }
                    nz0 = 1;
                    loop {
                        s = s.add(1);
                        if *s as c_int != '0' as c_int {
                            break;
                        }
                    }
                    if *s == 0 {
                        break 'ret;
                    }
                }
                s0 = s;
                nd = 0;
                nf = 0;
                yz = 0;
                /* for(; (c = *s) >= '0' && c <= '9'; nd++, s++) */
                loop {
                    c = *s as c_int;
                    if !(c >= '0' as c_int && c <= '9' as c_int) {
                        break;
                    }
                    if nd < 19 {
                        yz = (10u64.wrapping_mul(yz)).wrapping_add((c - '0' as c_int) as u64);
                    }
                    nd += 1;
                    s = s.add(1);
                }
                nd0 = nd;
                bc.dp0 = s.offset_from(s0) as c_int;
                bc.dp1 = bc.dp0;
                /* for(s1 = s; s1 > s0 && *--s1 == '0'; ) ++nz1; */
                s1 = s;
                while s1 > s0
                    && {
                        s1 = s1.sub(1);
                        *s1 as c_int == '0' as c_int
                    }
                {
                    nz1 += 1;
                }
                'dig_done: {
                    if c == '.' as c_int {
                        s = s.add(1);
                        c = *s as c_int;
                        bc.dp1 = s.offset_from(s0) as c_int;
                        bc.dplen = bc.dp1 - bc.dp0;
                        /* set when control enters the digit loop at `have_dig' */
                        let mut at_have_dig = false;
                        if nd == 0 {
                            /* for(; c == '0'; c = *++s) nz++; */
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
                                at_have_dig = true; /* goto have_dig */
                            } else {
                                break 'dig_done; /* goto dig_done */
                            }
                        }
                        /* for(; c >= '0' && c <= '9'; c = *++s) { have_dig: ... } */
                        loop {
                            if !at_have_dig {
                                if !(c >= '0' as c_int && c <= '9' as c_int) {
                                    break;
                                }
                            }
                            at_have_dig = false;
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
                                    yz = (10u64.wrapping_mul(yz)).wrapping_add(c as u64);
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
                        st = St::Ret0;
                        continue 'ret;
                    }
                    s00 = s;
                    esign = 0;
                    /* switch(c = *++s) { case '-': esign = 1; case '+': c = *++s; } */
                    s = s.add(1);
                    c = *s as c_int;
                    if c == '-' as c_int {
                        esign = 1;
                        /* FALLTHROUGH */
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
                        if bc.dplen == 0 {
                            /* switch(c) { case 'i': case 'I': ... case 'n': case 'N': ... } */
                            if c == 'i' as c_int || c == 'I' as c_int {
                                if match_(&mut s, b"nf\0".as_ptr() as *const c_char) != 0 {
                                    s = s.sub(1);
                                    if match_(&mut s, b"inity\0".as_ptr() as *const c_char) == 0 {
                                        s = s.add(1);
                                    }
                                    rv.set_w0(0x7ff00000);
                                    rv.set_w1(0);
                                    break 'ret;
                                }
                                /* break; */
                            } else if c == 'n' as c_int || c == 'N' as c_int {
                                if match_(&mut s, b"an\0".as_ptr() as *const c_char) != 0 {
                                    rv.set_w0(0x7ff80000);
                                    rv.set_w1(0);
                                    if *s as c_int == '(' as c_int {
                                        hexnan(&mut rv, &mut s);
                                    }
                                    break 'ret;
                                }
                            }
                        }
                        /* ret0: */
                        s = s00;
                        sign = 0;
                    }
                    break 'ret;
                }
                /* bc.e0 = e1 = e -= nf; */
                e -= nf;
                e1 = e;
                bc.e0 = e1;
                if nd0 == 0 {
                    nd0 = nd;
                }
                bd0 = std::ptr::null_mut();
                if nd <= 15 && 1 == 1 {
                    rv.set_dval(yz as f64);
                    if e == 0 {
                        break 'ret;
                    }
                    if e > 0 {
                        if e <= 22 {
                            rv.set_dval(rv.dval() * TENS[e as usize]);
                            break 'ret;
                        }
                        i = 15 - nd;
                        if e <= 22 + i {
                            e -= i;
                            rv.set_dval(rv.dval() * TENS[i as usize]);
                            rv.set_dval(rv.dval() * TENS[e as usize]);
                            break 'ret;
                        }
                    } else if e >= -22 {
                        rv.set_dval(rv.dval() / TENS[(-e) as usize]);
                        break 'ret;
                    }
                }
                k = if nd < 19 { nd } else { 19 };
                e1 += nd - k;
                /* (empty statement in the C source) */
                i = e1 + 342;
                if i < 0 {
                    st = St::Undfl;
                    continue 'ret;
                }
                if i > 650 {
                    st = St::Ovfl;
                    continue 'ret;
                }
                p10 = &PTEN[i as usize] as *const BF96;
                brv = yz;
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
                erv = (64 + 0x3fe) + (*p10).e - i;
                if erv <= 0 && nd > 19 {
                    st = St::ManyDigits;
                    continue 'ret;
                }
                bhi = brv >> 32;
                blo = brv & 0xffffffffu64;
                t01 = bhi.wrapping_mul((*p10).b1 as u64);
                t10 = blo
                    .wrapping_mul((*p10).b0 as u64)
                    .wrapping_add(t01 & 0xffffffffu64);
                t00 = bhi
                    .wrapping_mul((*p10).b0 as u64)
                    .wrapping_add(t01 >> 32)
                    .wrapping_add(t10 >> 32);
                if t00 & 0x8000000000000000u64 != 0 {
                    if (t00 & 0x3ff) != 0 && (!t00 & 0x3fe) != 0 {
                        /* `1<<i' is an int shift in C; only reached with nd > 19,
                           where i <= 4.  The &31 mirrors x86 shift semantics. */
                        if nd > 19
                            && (((t00
                                .wrapping_add((((1 as c_int) << (i & 31)) as u64))
                                .wrapping_add(2))
                                & 0x400)
                                ^ (t00 & 0x400))
                                != 0
                        {
                            st = St::ManyDigits;
                            continue 'ret;
                        }
                        if erv <= 0 {
                            st = St::Denormal;
                            continue 'ret;
                        }
                        if t00 & 0x400 != 0 && t00 & 0xbff != 0 {
                            st = St::Roundup;
                            continue 'ret;
                        }
                        st = St::Noround;
                        continue 'ret;
                    }
                } else {
                    if (t00 & 0x1ff) != 0 && (!t00 & 0x1fe) != 0 {
                        if nd > 19
                            && (((t00
                                .wrapping_add((((1 as c_int) << (i & 31)) as u64))
                                .wrapping_add(2))
                                & 0x200)
                                ^ (t00 & 0x200))
                                != 0
                        {
                            st = St::ManyDigits;
                            continue 'ret;
                        }
                        if erv <= 1 {
                            st = St::Denormal1;
                            continue 'ret;
                        }
                        if t00 & 0x200 != 0 {
                            st = St::Roundup1;
                            continue 'ret;
                        }
                        st = St::Noround1;
                        continue 'ret;
                    }
                }
                /* ; (empty statement) */
                t02 = bhi.wrapping_mul((*p10).b2 as u64);
                t11 = blo
                    .wrapping_mul((*p10).b1 as u64)
                    .wrapping_add(t02 & 0xffffffffu64);
                bexact = 1;
                if e1 < 0 || e1 > 41 || ((t10 | t11) & 0xffffffffu64) != 0 || nd > 19 {
                    bexact = 0;
                }
                tlo = (t10 & 0xffffffffu64)
                    .wrapping_add(t02 >> 32)
                    .wrapping_add(t11 >> 32);
                if bexact == 0 && (tlo.wrapping_add(0x10)) >> 32 > tlo >> 32 {
                    st = St::ManyDigits;
                    continue 'ret;
                }
                t00 = t00.wrapping_add(tlo >> 32);
                if t00 & 0x8000000000000000u64 != 0 {
                    if erv <= 0 {
                        if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x3ff)) == 0 {
                            st = St::ManyDigits;
                            continue 'ret;
                        }
                        st = St::Denormal;
                        continue 'ret;
                    }
                    if bexact != 0 {
                        if t00 & 0x400 != 0 && ((tlo & 0xffffffff) | (t00 & 0xbff)) != 0 {
                            st = St::Roundup;
                            continue 'ret;
                        }
                        st = St::Noround;
                        continue 'ret;
                    }
                    if ((tlo & 0xfffffff0) | (t00 & 0x3ff)) != 0
                        && (nd <= 19
                            || ((t00.wrapping_add(1u64 << (i & 63))) & 0xfffffffffffffc00u64)
                                == (t00 & 0xfffffffffffffc00u64))
                    {
                        if t00 & 0x400 != 0 {
                            st = St::Roundup;
                            continue 'ret;
                        }
                        st = St::Noround;
                        continue 'ret;
                    }
                } else {
                    if erv <= 1 {
                        if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x1ff)) == 0 {
                            st = St::ManyDigits;
                            continue 'ret;
                        }
                        st = St::Denormal1;
                        continue 'ret;
                    }
                    if bexact != 0 {
                        if t00 & 0x200 != 0 && ((t00 & 0x5ff) != 0 || tlo != 0) {
                            st = St::Roundup1;
                            continue 'ret;
                        }
                        st = St::Noround1;
                        continue 'ret;
                    }
                    if ((tlo & 0xfffffff0) | (t00 & 0x1ff)) != 0
                        && (nd <= 19
                            || ((t00.wrapping_add(1u64 << (i & 63))) & 0x7ffffffffffffe00u64)
                                == (t00 & 0x7ffffffffffffe00u64))
                    {
                        if t00 & 0x200 != 0 {
                            st = St::Roundup1;
                            continue 'ret;
                        }
                        st = St::Noround1;
                        continue 'ret;
                    }
                }
                st = St::ManyDigits;
                continue 'ret;
            }

            /* ------------------------------------------------------------ */
            /* ret0: */
            St::Ret0 => {
                s = s00;
                sign = 0;
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* denormal: */
            St::Denormal => {
                if erv <= -52 {
                    if erv < -52 || (t00 & 0x7fffffffffffffffu64) == 0 {
                        st = St::Undfl;
                        continue 'ret;
                    }
                    st = St::Tiniest;
                    continue 'ret;
                }
                tg = 1u64 << (11 - erv);
                t00 &= !(tg.wrapping_sub(1));
                if t00 & tg != 0 {
                    t00 = t00.wrapping_add(tg << 1);
                    if t00 & 0x8000000000000000u64 == 0 {
                        erv += 1;
                        if erv > 0 {
                            st = St::SmallestNormal;
                            continue 'ret;
                        }
                        t00 = 0x8000000000000000u64;
                    }
                }
                rv.LL = t00 >> (12 - erv);
                libc::set_errno(libc::ERANGE);
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* denormal1: */
            St::Denormal1 => {
                if erv <= -51 {
                    if erv < -51 || (t00 & 0x3fffffffffffffffu64) == 0 {
                        st = St::Undfl;
                        continue 'ret;
                    }
                    /* FALLTHROUGH to tiniest: */
                    st = St::Tiniest;
                    continue 'ret;
                }
                tg = 1u64 << (11 - erv);
                if t00 & tg != 0 {
                    t00 = t00.wrapping_add(tg << 1);
                    if 0x8000000000000000u64 & t00 != 0 && erv == 1 {
                        st = St::SmallestNormal;
                        continue 'ret;
                    }
                }
                if erv <= -52 {
                    st = St::Undfl;
                    continue 'ret;
                }
                rv.LL = t00 >> (12 - erv);
                libc::set_errno(libc::ERANGE);
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* tiniest: */
            St::Tiniest => {
                rv.LL = 1;
                libc::set_errno(libc::ERANGE);
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* smallest_normal: */
            St::SmallestNormal => {
                rv.LL = 0x0010000000000000u64;
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* roundup: */
            St::Roundup => {
                t00 = t00.wrapping_add(0x800);
                if t00 & 0x8000000000000000u64 == 0 {
                    if erv >= 0x7fe {
                        st = St::Ovfl;
                        continue 'ret;
                    }
                    terv = (erv + 1) as u64;
                    rv.LL = terv << 52;
                    break 'ret;
                }
                /* FALLTHROUGH to noround: */
                st = St::Noround;
                continue 'ret;
            }

            /* ------------------------------------------------------------ */
            /* noround: */
            St::Noround => {
                if erv >= 0x7ff {
                    st = St::Ovfl;
                    continue 'ret;
                }
                terv = erv as u64;
                rv.LL = (terv << 52) | ((t00 & 0x7ffffffffffff800u64) >> 11);
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* roundup1: */
            St::Roundup1 => {
                t00 = t00.wrapping_add(0x400);
                if t00 & 0x4000000000000000u64 == 0 {
                    if erv >= 0x7ff {
                        st = St::Ovfl;
                        continue 'ret;
                    }
                    terv = erv as u64;
                    rv.LL = terv << 52;
                    break 'ret;
                }
                /* FALLTHROUGH to noround1: */
                st = St::Noround1;
                continue 'ret;
            }

            /* ------------------------------------------------------------ */
            /* noround1: */
            St::Noround1 => {
                if erv >= 0x800 {
                    st = St::Ovfl;
                    continue 'ret;
                }
                terv = (erv - 1) as u64;
                rv.LL = (terv << 52) | ((t00 & 0x3ffffffffffffc00u64) >> 10);
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* ovfl: */
            St::Ovfl => {
                rv.set_w0(0x7ff00000);
                rv.set_w1(0);
                /* FALLTHROUGH to range_err: */
                st = St::RangeErr;
                continue 'ret;
            }

            /* ------------------------------------------------------------ */
            /* range_err: */
            St::RangeErr => {
                if !bd0.is_null() {
                    Bfree(bb);
                    Bfree(bd);
                    Bfree(bs);
                    Bfree(bd0);
                    Bfree(delta);
                }
                libc::set_errno(libc::ERANGE);
                break 'ret;
            }

            /* ------------------------------------------------------------ */
            /* undfl: */
            St::Undfl => {
                rv.set_dval(0.);
                st = St::RangeErr;
                continue 'ret;
            }

            /* ------------------------------------------------------------ */
            /* many_digits: */
            St::ManyDigits => {
                /* (empty statement in the C source) */
                if nd > 17 {
                    if nd > 18 {
                        yz /= 100;
                        e1 += 2;
                    } else {
                        yz /= 10;
                        e1 += 1;
                    }
                    y = (yz / 100000000) as ULong;
                } else if nd > 9 {
                    i = nd - 9;
                    y = ((yz >> i) / PFIVE[(i - 1) as usize]) as ULong;
                } else {
                    y = yz as ULong;
                }
                rv.set_dval(yz as f64);
                bc.scale = 0;
                if e1 > 0 {
                    i = e1 & 15;
                    if i != 0 {
                        rv.set_dval(rv.dval() * TENS[i as usize]);
                    }
                    e1 &= !15;
                    if e1 != 0 {
                        if e1 > 308 {
                            st = St::Ovfl;
                            continue 'ret;
                        }
                        e1 >>= 4;
                        j = 0;
                        while e1 > 1 {
                            if e1 & 1 != 0 {
                                rv.set_dval(rv.dval() * BIGTENS[j as usize]);
                            }
                            j += 1;
                            e1 >>= 1;
                        }
                        rv.set_w0(rv.w0().wrapping_sub((53 * 0x100000) as ULong));
                        rv.set_dval(rv.dval() * BIGTENS[j as usize]);
                        z = rv.w0() & 0x7ff00000;
                        if z > (0x100000 * (1024 + 1023 - 53)) as ULong {
                            st = St::Ovfl;
                            continue 'ret;
                        }
                        if z > (0x100000 * (1024 + 1023 - 1 - 53)) as ULong {
                            rv.set_w0((0xfffff | 0x100000 * (1024 + 1023 - 1)) as ULong);
                            rv.set_w1(0xffffffff);
                        } else {
                            rv.set_w0(rv.w0().wrapping_add((53 * 0x100000) as ULong));
                        }
                    }
                } else if e1 < 0 {
                    e1 = -e1;
                    i = e1 & 15;
                    if i != 0 {
                        rv.set_dval(rv.dval() / TENS[i as usize]);
                    }
                    e1 >>= 4;
                    if e1 != 0 {
                        if e1 >= 1 << 5 {
                            st = St::Undfl;
                            continue 'ret;
                        }
                        if e1 & 0x10 != 0 {
                            bc.scale = 2 * 53;
                        }
                        j = 0;
                        while e1 > 0 {
                            if e1 & 1 != 0 {
                                rv.set_dval(rv.dval() * TINYTENS[j as usize]);
                            }
                            j += 1;
                            e1 >>= 1;
                        }
                        if bc.scale != 0 && {
                            j = 2 * 53 + 1 - (((rv.w0() & 0x7ff00000) >> 20) as c_int);
                            j > 0
                        } {
                            if j >= 32 {
                                if j > 54 {
                                    st = St::Undfl;
                                    continue 'ret;
                                }
                                rv.set_w1(0);
                                if j >= 53 {
                                    rv.set_w0(((53 + 2) * 0x100000) as ULong);
                                } else {
                                    rv.set_w0(rv.w0() & (0xffffffffu32 << (j - 32)));
                                }
                            } else {
                                rv.set_w1(rv.w1() & (0xffffffffu32 << j));
                            }
                        }
                        if rv.dval() == 0. {
                            /* undfl: */
                            st = St::Undfl;
                            continue 'ret;
                        }
                    }
                }
                bc.nd = nd - nz1;
                bc.nd0 = nd0;
                if nd > 40 {
                    j = 18;
                    i = 18;
                    if i > nd0 {
                        j += bc.dplen;
                    }
                    loop {
                        j -= 1;
                        if j < bc.dp1 && j >= bc.dp0 {
                            j = bc.dp0 - 1;
                        }
                        if *s0.offset(j as isize) as c_int != '0' as c_int {
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
                            y = (10u32.wrapping_mul(y))
                                .wrapping_add(*s0.offset(i as isize) as c_int as ULong)
                                .wrapping_sub('0' as ULong);
                            i += 1;
                        }
                        j = bc.dp1;
                        while i < nd {
                            y = (10u32.wrapping_mul(y))
                                .wrapping_add(*s0.offset(j as isize) as c_int as ULong)
                                .wrapping_sub('0' as ULong);
                            j += 1;
                            i += 1;
                        }
                    }
                }
                bd0 = s2b(s0, nd0, nd, y, bc.dplen);
                'forloop: loop {
                    'cont: {
                        bd = Balloc((*bd0).k);
                        /* memcpy(&bd->sign, &bd0->sign, bd0->wds*sizeof(int) + 2*sizeof(int)) */
                        std::ptr::copy_nonoverlapping(
                            std::ptr::addr_of!((*bd0).sign) as *const u8,
                            std::ptr::addr_of_mut!((*bd).sign) as *mut u8,
                            ((*bd0).wds as usize) * std::mem::size_of::<c_int>()
                                + 2 * std::mem::size_of::<c_int>(),
                        );
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
                                break 'forloop;
                            }
                            i = -1;
                        }
                        'after_drop_down: {
                            'drop_down: {
                                if i < 0 {
                                    if bc.dsign != 0
                                        || rv.w1() != 0
                                        || (rv.w0() & 0xfffff) != 0
                                        || (rv.w0() & 0x7ff00000)
                                            <= ((2 * 53 + 1) * 0x100000) as ULong
                                    {
                                        break 'forloop;
                                    }
                                    if *bx(delta) == 0 && (*delta).wds <= 1 {
                                        break 'forloop;
                                    }
                                    delta = lshift(delta, 1);
                                    if cmp(delta, bs) > 0 {
                                        break 'drop_down; /* goto drop_down */
                                    }
                                    break 'forloop;
                                }
                                if i == 0 {
                                    if bc.dsign != 0 {
                                        if (rv.w0() & 0xfffff) == 0xfffff
                                            && rv.w1()
                                                == (if bc.scale != 0 && {
                                                    y = rv.w0() & 0x7ff00000;
                                                    y <= (2 * 53 * 0x100000) as ULong
                                                } {
                                                    /* the shift count is 1..107 in C;
                                                       &31 mirrors x86 shift semantics */
                                                    0xffffffffu32
                                                        & (0xffffffffu32
                                                            << ((2 * 53 + 1
                                                                - ((y >> 20) as c_int))
                                                                & 31))
                                                } else {
                                                    0xffffffff
                                                })
                                        {
                                            if rv.w0()
                                                == ((0xfffff | 0x100000 * (1024 + 1023 - 1))
                                                    as ULong)
                                                && rv.w1() == 0xffffffff
                                            {
                                                st = St::Ovfl;
                                                continue 'ret;
                                            }
                                            rv.set_w0(
                                                (rv.w0() & 0x7ff00000).wrapping_add(0x100000),
                                            );
                                            rv.set_w1(0);
                                            bc.dsign = 0;
                                            break 'forloop;
                                        }
                                    } else if (rv.w0() & 0xfffff) == 0 && rv.w1() == 0 {
                                        /* FALLTHROUGH to drop_down: */
                                        break 'drop_down;
                                    }
                                    if Lsb1 != 0 {
                                        if rv.w0() & Lsb1 == 0 {
                                            break 'forloop;
                                        }
                                    } else if rv.w1() & Lsb == 0 {
                                        break 'forloop;
                                    }
                                    if bc.dsign != 0 {
                                        let t = sulp(&mut rv as *mut U, &bc as *const BCinfo);
                                        rv.set_dval(rv.dval() + t);
                                    } else {
                                        let t = sulp(&mut rv as *mut U, &bc as *const BCinfo);
                                        rv.set_dval(rv.dval() - t);
                                        if rv.dval() == 0. {
                                            if bc.nd > nd {
                                                bc.uflchk = 1;
                                                break 'forloop;
                                            }
                                            st = St::Undfl;
                                            continue 'ret;
                                        }
                                    }
                                    bc.dsign = 1 - bc.dsign;
                                    break 'forloop;
                                }
                                break 'after_drop_down;
                            }
                            /* drop_down: */
                            if bc.scale != 0 {
                                L = (rv.w0() & 0x7ff00000) as c_int;
                                if L <= (2 * 53 + 1) * 0x100000 {
                                    if L > (53 + 2) * 0x100000 {
                                        break 'forloop;
                                    }
                                    if bc.nd > nd {
                                        bc.uflchk = 1;
                                        break 'forloop;
                                    }
                                    st = St::Undfl;
                                    continue 'ret;
                                }
                            }
                            L = ((rv.w0() & 0x7ff00000).wrapping_sub(0x100000)) as c_int;
                            rv.set_w0((L as ULong) | 0xfffff);
                            rv.set_w1(0xffffffff);
                            if bc.nd > nd {
                                break 'cont; /* goto cont */
                            }
                            break 'forloop;
                        }
                        aadj = ratio(delta, bs);
                        if aadj <= 2. {
                            if bc.dsign != 0 {
                                aadj = 1.;
                                aadj1 = 1.;
                            } else if rv.w1() != 0 || (rv.w0() & 0xfffff) != 0 {
                                if rv.w1() == 1 && rv.w0() == 0 {
                                    if bc.nd > nd {
                                        bc.uflchk = 1;
                                        break 'forloop;
                                    }
                                    st = St::Undfl;
                                    continue 'ret;
                                }
                                aadj = 1.;
                                aadj1 = -1.;
                            } else {
                                if aadj < 2. / 2. {
                                    aadj = 1. / 2.;
                                } else {
                                    aadj *= 0.5;
                                }
                                aadj1 = -aadj;
                            }
                        } else {
                            aadj *= 0.5;
                            aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                            if 1 == 0 {
                                aadj1 += 0.5;
                            }
                        }
                        y = rv.w0() & 0x7ff00000;
                        if y == (0x100000 * (1024 + 1023 - 1)) as ULong {
                            rv0.set_dval(rv.dval());
                            rv.set_w0(rv.w0().wrapping_sub((53 * 0x100000) as ULong));
                            adj.set_dval(aadj1 * ulp(&mut rv as *mut U));
                            rv.set_dval(rv.dval() + adj.dval());
                            if (rv.w0() & 0x7ff00000)
                                >= (0x100000 * (1024 + 1023 - 53)) as ULong
                            {
                                if rv0.w0()
                                    == ((0xfffff | 0x100000 * (1024 + 1023 - 1)) as ULong)
                                    && rv0.w1() == 0xffffffff
                                {
                                    st = St::Ovfl;
                                    continue 'ret;
                                }
                                rv.set_w0((0xfffff | 0x100000 * (1024 + 1023 - 1)) as ULong);
                                rv.set_w1(0xffffffff);
                                break 'cont; /* goto cont */
                            } else {
                                rv.set_w0(rv.w0().wrapping_add((53 * 0x100000) as ULong));
                            }
                        } else {
                            if bc.scale != 0 && y <= (2 * 53 * 0x100000) as ULong {
                                if aadj <= 0x7fffffff as f64 {
                                    z = aadj as ULong;
                                    if z == 0 {
                                        z = 1;
                                    }
                                    aadj = z as f64;
                                    aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                                }
                                aadj2.set_dval(aadj1);
                                aadj2.set_w0(
                                    aadj2
                                        .w0()
                                        .wrapping_add(((2 * 53 + 1) * 0x100000) as ULong)
                                        .wrapping_sub(y),
                                );
                                aadj1 = aadj2.dval();
                                adj.set_dval(aadj1 * ulp(&mut rv as *mut U));
                                rv.set_dval(rv.dval() + adj.dval());
                                if rv.dval() == 0. {
                                    req_bigcomp = 1;
                                    break 'forloop;
                                }
                            } else {
                                adj.set_dval(aadj1 * ulp(&mut rv as *mut U));
                                rv.set_dval(rv.dval() + adj.dval());
                            }
                        }
                        z = rv.w0() & 0x7ff00000;
                        if bc.nd == nd {
                            if bc.scale == 0 {
                                if y == z {
                                    L = aadj as c_int;
                                    aadj -= L as f64;
                                    if bc.dsign != 0
                                        || rv.w1() != 0
                                        || (rv.w0() & 0xfffff) != 0
                                    {
                                        if aadj < 0.4999999 || aadj > 0.5000001 {
                                            break 'forloop;
                                        }
                                    } else if aadj < 0.4999999 / 2. {
                                        break 'forloop;
                                    }
                                }
                            }
                        }
                    }
                    /* cont: */
                    Bfree(bb);
                    Bfree(bd);
                    Bfree(bs);
                    Bfree(delta);
                }
                Bfree(bb);
                Bfree(bd);
                Bfree(bs);
                Bfree(bd0);
                Bfree(delta);
                if req_bigcomp != 0 {
                    bd0 = std::ptr::null_mut();
                    bc.e0 += nz1;
                    bigcomp(&mut rv, s0, &mut bc);
                    y = rv.w0() & 0x7ff00000;
                    if y == 0x7ff00000 {
                        st = St::Ovfl;
                        continue 'ret;
                    }
                    if y == 0 && rv.dval() == 0. {
                        st = St::Undfl;
                        continue 'ret;
                    }
                }
                if bc.scale != 0 {
                    rv0.set_w0((0x3ff00000 - 2 * 53 * 0x100000) as ULong);
                    rv0.set_w1(0);
                    rv.set_dval(rv.dval() * rv0.dval());
                    if (rv.w0() & 0x7ff00000) == 0 {
                        libc::set_errno(libc::ERANGE);
                    }
                }
                break 'ret;
            }
        }
    }
    /* ret: */
    if !se.is_null() {
        *se = s as *mut c_char;
    }
    return if sign != 0 { -rv.dval() } else { rv.dval() };
}
