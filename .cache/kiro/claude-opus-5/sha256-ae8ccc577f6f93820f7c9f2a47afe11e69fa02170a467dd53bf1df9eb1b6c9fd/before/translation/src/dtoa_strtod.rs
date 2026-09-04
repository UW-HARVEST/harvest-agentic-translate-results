/* Included from dtoa.rs: translation of strtod__unused() in src/dtoa.c.
 *
 * jansson renames dtoa.c's strtod() to strtod__unused (it uses the C library's
 * strtod instead), but the symbol is still exported, so the function is
 * translated in full.
 */

/// Terminal labels of the original function.
#[derive(Clone, Copy, PartialEq)]
enum Fin {
    Ret,
    Ret0,
    Ovfl,
    Undfl,
    RangeErr,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod__unused(s00_in: *const c_char, se: *mut *mut c_char) -> f64 {
    unsafe {
        let mut s00 = s00_in;

        let mut bb2: c_int;
        let mut bb5: c_int;
        let mut bbe: c_int = 0;
        let mut bd2: c_int;
        let mut bd5: c_int;
        let mut bbbits: c_int = 0;
        let mut bs2: c_int;
        let mut c: c_int;
        let mut e: c_int;
        let mut e1: c_int;
        let mut esign: c_int;
        let mut i: c_int;
        let mut j: c_int;
        let k: c_int;
        let mut nd: c_int;
        let mut nd0: c_int;
        let mut nf: c_int;
        let mut nz: c_int;
        let mut nz0: c_int;
        let mut nz1: c_int;
        let mut sign: c_int;
        let mut s: *const c_char;
        let mut s0: *const c_char;
        let mut s1: *const c_char;
        let mut aadj: f64;
        let mut aadj1: f64 = 0.0;
        let mut L: i32;
        let mut aadj2 = U::zero();
        let mut adj = U::zero();
        let mut rv = U::zero();
        let mut rv0 = U::zero();
        let mut y: u32;
        let mut z: u32;
        let mut bc = BCinfo::zero();
        let mut bb: *mut Bigint = null_mut();
        let mut bb1: *mut Bigint;
        let mut bd: *mut Bigint = null_mut();
        let mut bd0: *mut Bigint = null_mut();
        let mut bs: *mut Bigint = null_mut();
        let mut delta: *mut Bigint = null_mut();
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
        let mut p10: usize;
        let mut bexact: c_int;
        let mut erv: c_int;
        let mut Lsb: u32;
        let mut Lsb1: u32;
        let mut req_bigcomp: c_int = 0;

        let mut fin = Fin::Ret;

        sign = 0;
        nz0 = 0;
        nz1 = 0;
        nz = 0;
        bc.dplen = 0;
        bc.uflchk = 0;
        rv.set_d(0.0);

        'main: {
            /* leading sign and white space */
            s = s00;
            let mut reached_break2 = false;
            while !reached_break2 {
                let ch = *s as c_int;
                match ch {
                    x if x == '-' as c_int || x == '+' as c_int => {
                        if x == '-' as c_int {
                            sign = 1;
                        }
                        s = s.add(1);
                        if *s != 0 {
                            reached_break2 = true;
                        } else {
                            fin = Fin::Ret0;
                            break 'main;
                        }
                    }
                    0 => {
                        fin = Fin::Ret0;
                        break 'main;
                    }
                    0x09 | 0x0a | 0x0b | 0x0c | 0x0d | 0x20 => {
                        s = s.add(1);
                    }
                    _ => reached_break2 = true,
                }
            }

            /* break2: */
            if *s == b'0' as c_char {
                let n = *s.add(1);
                if n == b'x' as c_char || n == b'X' as c_char {
                    gethex(&mut s, &mut rv, 1, sign);
                    fin = Fin::Ret;
                    break 'main;
                }

                nz0 = 1;
                loop {
                    s = s.add(1);
                    if *s != b'0' as c_char {
                        break;
                    }
                }
                if *s == 0 {
                    fin = Fin::Ret;
                    break 'main;
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
                    yz = 10u64
                        .wrapping_mul(yz)
                        .wrapping_add((c - '0' as c_int) as u64);
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
                if *s1 != b'0' as c_char {
                    break;
                }
                nz1 += 1;
            }

            let mut dig_done = false;
            if c == '.' as c_int {
                s = s.add(1);
                c = *s as c_int;
                bc.dp1 = s.offset_from(s0) as c_int;
                bc.dplen = bc.dp1 - bc.dp0;
                let mut have_dig = false;
                if nd == 0 {
                    while c == '0' as c_int {
                        nz += 1;
                        s = s.add(1);
                        c = *s as c_int;
                    }
                    if c > '0' as c_int && c <= '9' as c_int {
                        bc.dp0 = -(s.offset_from(s0) as c_int);
                        bc.dp1 = bc.dp0 + bc.dplen;
                        s0 = s;
                        nf += nz;
                        nz = 0;
                        have_dig = true;
                    } else {
                        dig_done = true;
                    }
                }
                if !dig_done {
                    loop {
                        if !have_dig {
                            if !(c >= '0' as c_int && c <= '9' as c_int) {
                                break;
                            }
                        }
                        have_dig = false;
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
                    fin = Fin::Ret0;
                    break 'main;
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
                            /* Avoid confusion from exponents so large that e
                            might overflow. */
                            e = 19999; /* safe for 16 bit ints */
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
                    let mut done = false;
                    if bc.dplen == 0 {
                        if c == 'i' as c_int || c == 'I' as c_int {
                            if match_(&mut s, b"nf\0") != 0 {
                                s = s.sub(1);
                                if match_(&mut s, b"inity\0") == 0 {
                                    s = s.add(1);
                                }
                                rv.set_w0(0x7ff00000);
                                rv.set_w1(0);
                                done = true;
                            }
                        } else if c == 'n' as c_int || c == 'N' as c_int {
                            if match_(&mut s, b"an\0") != 0 {
                                rv.set_w0(NAN_WORD0);
                                rv.set_w1(NAN_WORD1);
                                if *s == b'(' as c_char {
                                    hexnan(&mut rv, &mut s);
                                }
                                done = true;
                            }
                        }
                    }
                    if done {
                        fin = Fin::Ret;
                        break 'main;
                    }
                    fin = Fin::Ret0;
                    break 'main;
                }
                fin = Fin::Ret;
                break 'main;
            }
            e -= nf;
            e1 = e;
            bc.e0 = e;

            /* Now we have nd0 digits, starting at s0, followed by a
             * decimal point, followed by nd-nd0 digits.  The number we're
             * after is the integer represented by those digits times 10**e */

            if nd0 == 0 {
                nd0 = nd;
            }
            bd0 = null_mut();
            if nd <= DBL_DIG && Flt_Rounds == 1 {
                rv.set_d(yz as f64);

                if e == 0 {
                    fin = Fin::Ret;
                    break 'main;
                }

                if e > 0 {
                    if e <= Ten_pmax {
                        rv.set_d(rv.d() * tens[e as usize]);
                        fin = Fin::Ret;
                        break 'main;
                    }
                    i = DBL_DIG - nd;
                    if e <= Ten_pmax + i {
                        /* A fancier test would sometimes let us do
                         * this for larger i values. */
                        e -= i;
                        rv.set_d(rv.d() * tens[i as usize]);
                        rv.set_d(rv.d() * tens[e as usize]);
                        fin = Fin::Ret;
                        break 'main;
                    }
                } else if e >= -Ten_pmax {
                    rv.set_d(rv.d() / tens[(-e) as usize]);
                    fin = Fin::Ret;
                    break 'main;
                }
            }

            k = if nd < 19 { nd } else { 19 };

            e1 += nd - k; /* scale factor = 10^e1 */

            /* ------------------------------------------------------------ */
            /* the 96-bit software floating point fast path                 */
            /* ------------------------------------------------------------ */
            #[derive(Clone, Copy, PartialEq)]
            enum F {
                Head,
                Denormal,
                Denormal1,
                Tiniest,
                SmallestNormal,
                Roundup,
                Noround,
                Roundup1,
                Noround1,
                ManyDigits,
                Done,
            }

            let mut fs = F::Head;
            /* values shared between the states */
            t00 = 0;
            tlo = 0;
            erv = 0;
            bexact = 0;
            i = 0;
            p10 = 0;

            'fast: loop {
                match fs {
                    F::Head => {
                        i = e1 + 342;
                        if i < 0 {
                            fin = Fin::Undfl;
                            break 'main;
                        }
                        if i > 650 {
                            fin = Fin::Ovfl;
                            break 'main;
                        }
                        p10 = i as usize;
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
                        erv = (64 + 0x3fe) + pten[p10].e - i;
                        if erv <= 0 && nd > 19 {
                            /* denormal: may need to look at all digits */
                            fs = F::ManyDigits;
                            continue 'fast;
                        }
                        bhi = brv >> 32;
                        blo = brv & 0xffffffff;

                        t01 = bhi.wrapping_mul(pten[p10].b1 as u64);
                        t10 = blo
                            .wrapping_mul(pten[p10].b0 as u64)
                            .wrapping_add(t01 & 0xffffffff);
                        t00 = bhi
                            .wrapping_mul(pten[p10].b0 as u64)
                            .wrapping_add(t01 >> 32)
                            .wrapping_add(t10 >> 32);
                        if t00 & 0x8000000000000000 != 0 {
                            if (t00 & 0x3ff) != 0 && (!t00 & 0x3fe) != 0 {
                                /* unambiguous result? */
                                if nd > 19
                                    && (((t00
                                        .wrapping_add(1u64 << i)
                                        .wrapping_add(2))
                                        & 0x400)
                                        ^ (t00 & 0x400))
                                        != 0
                                {
                                    fs = F::ManyDigits;
                                    continue 'fast;
                                }
                                if erv <= 0 {
                                    fs = F::Denormal;
                                    continue 'fast;
                                }
                                if t00 & 0x400 != 0 && t00 & 0xbff != 0 {
                                    fs = F::Roundup;
                                    continue 'fast;
                                }
                                fs = F::Noround;
                                continue 'fast;
                            }
                        } else {
                            if (t00 & 0x1ff) != 0 && (!t00 & 0x1fe) != 0 {
                                /* unambiguous result? */
                                if nd > 19
                                    && (((t00
                                        .wrapping_add(1u64 << i)
                                        .wrapping_add(2))
                                        & 0x200)
                                        ^ (t00 & 0x200))
                                        != 0
                                {
                                    fs = F::ManyDigits;
                                    continue 'fast;
                                }
                                if erv <= 1 {
                                    fs = F::Denormal1;
                                    continue 'fast;
                                }
                                if t00 & 0x200 != 0 {
                                    fs = F::Roundup1;
                                    continue 'fast;
                                }
                                fs = F::Noround1;
                                continue 'fast;
                            }
                        }

                        t02 = bhi.wrapping_mul(pten[p10].b2 as u64);
                        t11 = blo
                            .wrapping_mul(pten[p10].b1 as u64)
                            .wrapping_add(t02 & 0xffffffff);
                        bexact = 1;
                        if e1 < 0 || e1 > 41 || (t10 | t11) & 0xffffffff != 0 || nd > 19 {
                            bexact = 0;
                        }
                        tlo = (t10 & 0xffffffff)
                            .wrapping_add(t02 >> 32)
                            .wrapping_add(t11 >> 32);
                        if bexact == 0 && (tlo.wrapping_add(0x10)) >> 32 > tlo >> 32 {
                            fs = F::ManyDigits;
                            continue 'fast;
                        }
                        t00 = t00.wrapping_add(tlo >> 32);
                        if t00 & 0x8000000000000000 != 0 {
                            if erv <= 0 {
                                /* denormal result */
                                if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x3ff)) == 0 {
                                    fs = F::ManyDigits;
                                    continue 'fast;
                                }
                                fs = F::Denormal;
                                continue 'fast;
                            }
                            if bexact != 0 {
                                if t00 & 0x400 != 0
                                    && ((tlo & 0xffffffff) | (t00 & 0xbff)) != 0
                                {
                                    fs = F::Roundup;
                                    continue 'fast;
                                }
                                fs = F::Noround;
                                continue 'fast;
                            }
                            if ((tlo & 0xfffffff0) | (t00 & 0x3ff)) != 0
                                && (nd <= 19
                                    || ((t00.wrapping_add(1u64 << i))
                                        & 0xfffffffffffffc00)
                                        == (t00 & 0xfffffffffffffc00))
                            {
                                if t00 & 0x400 != 0 {
                                    /* round up */
                                    fs = F::Roundup;
                                    continue 'fast;
                                }
                                fs = F::Noround;
                                continue 'fast;
                            }
                        } else {
                            if erv <= 1 {
                                /* denormal result */
                                if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x1ff)) == 0 {
                                    fs = F::ManyDigits;
                                    continue 'fast;
                                }
                                fs = F::Denormal1;
                                continue 'fast;
                            }
                            if bexact != 0 {
                                if t00 & 0x200 != 0 && ((t00 & 0x5ff) != 0 || tlo != 0) {
                                    fs = F::Roundup1;
                                    continue 'fast;
                                }
                                fs = F::Noround1;
                                continue 'fast;
                            }
                            if ((tlo & 0xfffffff0) | (t00 & 0x1ff)) != 0
                                && (nd <= 19
                                    || ((t00.wrapping_add(1u64 << i))
                                        & 0x7ffffffffffffe00)
                                        == (t00 & 0x7ffffffffffffe00))
                            {
                                if t00 & 0x200 != 0 {
                                    /* round up */
                                    fs = F::Roundup1;
                                    continue 'fast;
                                }
                                fs = F::Noround1;
                                continue 'fast;
                            }
                        }
                        fs = F::ManyDigits;
                    }

                    F::Denormal => {
                        if erv <= -52 {
                            if erv < -52 || (t00 & 0x7fffffffffffffff) == 0 {
                                fin = Fin::Undfl;
                                break 'main;
                            }
                            fs = F::Tiniest;
                            continue 'fast;
                        }
                        tg = 1u64 << (11 - erv);
                        t00 &= !(tg - 1); /* clear low bits */

                        if t00 & tg != 0 {
                            t00 = t00.wrapping_add(tg << 1);
                            if t00 & 0x8000000000000000 == 0 {
                                erv += 1;
                                if erv > 0 {
                                    fs = F::SmallestNormal;
                                    continue 'fast;
                                }
                                t00 = 0x8000000000000000;
                            }
                        }

                        rv.ll = t00 >> (12 - erv);
                        set_errno(ERANGE);
                        fs = F::Done;
                    }

                    F::Denormal1 => {
                        if erv <= -51 {
                            if erv < -51 || (t00 & 0x3fffffffffffffff) == 0 {
                                fin = Fin::Undfl;
                                break 'main;
                            }
                            fs = F::Tiniest;
                            continue 'fast;
                        }
                        tg = 1u64 << (11 - erv);
                        if t00 & tg != 0 {
                            t00 = t00.wrapping_add(tg << 1);
                            if 0x8000000000000000 & t00 != 0 && erv == 1 {
                                fs = F::SmallestNormal;
                                continue 'fast;
                            }
                        }

                        if erv <= -52 {
                            fin = Fin::Undfl;
                            break 'main;
                        }
                        rv.ll = t00 >> (12 - erv);
                        set_errno(ERANGE);
                        fs = F::Done;
                    }

                    F::Tiniest => {
                        rv.ll = 1;
                        set_errno(ERANGE);
                        fs = F::Done;
                    }

                    F::SmallestNormal => {
                        rv.ll = 0x0010000000000000;
                        fs = F::Done;
                    }

                    F::Roundup => {
                        t00 = t00.wrapping_add(0x800);
                        if t00 & 0x8000000000000000 == 0 {
                            if erv >= 0x7fe {
                                fin = Fin::Ovfl;
                                break 'main;
                            }
                            terv = (erv + 1) as u64;
                            rv.ll = terv << 52;
                            fs = F::Done;
                            continue 'fast;
                        }
                        fs = F::Noround;
                    }

                    F::Noround => {
                        if erv >= 0x7ff {
                            fin = Fin::Ovfl;
                            break 'main;
                        }
                        terv = erv as u64;
                        rv.ll = (terv << 52) | ((t00 & 0x7ffffffffffff800) >> 11);
                        fs = F::Done;
                    }

                    F::Roundup1 => {
                        t00 = t00.wrapping_add(0x400);
                        if t00 & 0x4000000000000000 == 0 {
                            if erv >= 0x7ff {
                                fin = Fin::Ovfl;
                                break 'main;
                            }
                            terv = erv as u64;
                            rv.ll = terv << 52;
                            fs = F::Done;
                            continue 'fast;
                        }
                        fs = F::Noround1;
                    }

                    F::Noround1 => {
                        if erv >= 0x800 {
                            fin = Fin::Ovfl;
                            break 'main;
                        }
                        terv = (erv - 1) as u64;
                        rv.ll = (terv << 52) | ((t00 & 0x3ffffffffffffc00) >> 10);
                        fs = F::Done;
                    }

                    F::Done => {
                        fin = Fin::Ret;
                        break 'main;
                    }

                    F::ManyDigits => break 'fast,
                }
            }

            /* many_digits: */
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
                y = ((yz >> i) / pfive[(i - 1) as usize]) as u32;
            } else {
                y = yz as u32;
            }
            rv.set_d(yz as f64);

            bc.scale = 0;

            /* Get starting approximation = rv * 10**e1 */

            if e1 > 0 {
                i = e1 & 15;
                if i != 0 {
                    rv.set_d(rv.d() * tens[i as usize]);
                }
                e1 &= !15;
                if e1 != 0 {
                    if e1 > DBL_MAX_10_EXP {
                        fin = Fin::Ovfl;
                        break 'main;
                    }
                    e1 >>= 4;
                    j = 0;
                    while e1 > 1 {
                        if e1 & 1 != 0 {
                            rv.set_d(rv.d() * bigtens[j as usize]);
                        }
                        j += 1;
                        e1 >>= 1;
                    }
                    /* The last multiplication could overflow. */
                    rv.set_w0(rv.w0().wrapping_sub((P as u32).wrapping_mul(Exp_msk1)));
                    rv.set_d(rv.d() * bigtens[j as usize]);
                    z = rv.w0() & Exp_mask;
                    if z > Exp_msk1.wrapping_mul((DBL_MAX_EXP + Bias - P) as u32) {
                        fin = Fin::Ovfl;
                        break 'main;
                    }
                    if z > Exp_msk1.wrapping_mul((DBL_MAX_EXP + Bias - 1 - P) as u32) {
                        /* set to largest number (can't trust DBL_MAX) */
                        rv.set_w0(Big0);
                        rv.set_w1(Big1);
                    } else {
                        rv.set_w0(rv.w0().wrapping_add((P as u32).wrapping_mul(Exp_msk1)));
                    }
                }
            } else if e1 < 0 {
                e1 = -e1;
                i = e1 & 15;
                if i != 0 {
                    rv.set_d(rv.d() / tens[i as usize]);
                }
                e1 >>= 4;
                if e1 != 0 {
                    if e1 >= 1 << n_bigtens {
                        fin = Fin::Undfl;
                        break 'main;
                    }
                    if e1 & Scale_Bit != 0 {
                        bc.scale = 2 * P;
                    }
                    j = 0;
                    while e1 > 0 {
                        if e1 & 1 != 0 {
                            rv.set_d(rv.d() * tinytens[j as usize]);
                        }
                        j += 1;
                        e1 >>= 1;
                    }
                    if bc.scale != 0 {
                        j = 2 * P + 1 - (((rv.w0() & Exp_mask) >> Exp_shift) as c_int);
                        if j > 0 {
                            /* scaled rv is denormal; clear j low bits */
                            if j >= 32 {
                                if j > 54 {
                                    fin = Fin::Undfl;
                                    break 'main;
                                }
                                rv.set_w1(0);
                                if j >= 53 {
                                    rv.set_w0(((P + 2) as u32).wrapping_mul(Exp_msk1));
                                } else {
                                    rv.set_w0(rv.w0() & (0xffffffffu32 << (j - 32)));
                                }
                            } else {
                                rv.set_w1(rv.w1() & (0xffffffffu32 << j));
                            }
                        }
                    }
                    if rv.d() == 0.0 {
                        fin = Fin::Undfl;
                        break 'main;
                    }
                }
            }

            /* Now the hard part -- adjusting rv to the correct value. */
            /* Put digits into bd: true value = bd * 10^e */

            bc.nd = nd - nz1;
            bc.nd0 = nd0;
            if nd > strtod_diglim {
                /* ASSERT(strtod_diglim >= 18); */
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
                    if *s0.add(j as usize) != b'0' as c_char {
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
                        y = 10u32
                            .wrapping_mul(y)
                            .wrapping_add((*s0.add(i as usize) as c_int - '0' as c_int) as u32);
                        i += 1;
                    }
                    j = bc.dp1;
                    while i < nd {
                        y = 10u32
                            .wrapping_mul(y)
                            .wrapping_add((*s0.add(j as usize) as c_int - '0' as c_int) as u32);
                        j += 1;
                        i += 1;
                    }
                }
            }

            bd0 = s2b(s0, nd0, nd, y, bc.dplen);

            'bigloop: loop {
                bd = Balloc((*bd0).k);
                Bcopy(bd, bd0);
                bb = d2b(&mut rv, &mut bbe, &mut bbbits); /* rv = bb * 2^bbe */
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

                Lsb = LSB;
                Lsb1 = 0;
                j = bbe - bc.scale;
                i = j + bbbits - 1; /* logb(rv) */
                j = P + 1 - bbbits;
                if i < Emin {
                    /* denormal */
                    i = Emin - i;
                    j -= i;
                    if i < 32 {
                        Lsb <<= i;
                    } else if i < 52 {
                        Lsb1 = Lsb << (i - 32);
                    } else {
                        Lsb1 = Exp_mask;
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
                        /* Must use bigcomp(). */
                        req_bigcomp = 1;
                        break 'bigloop;
                    }
                    i = -1; /* Discarded digits make delta smaller. */
                }

                'body: {
                    let mut drop_down = false;

                    if i < 0 {
                        /* Error is less than half an ulp -- check for
                         * special case of mantissa a power of two. */
                        if bc.dsign != 0
                            || rv.w1() != 0
                            || (rv.w0() & Bndry_mask) != 0
                            || (rv.w0() & Exp_mask)
                                <= ((2 * P + 1) as u32).wrapping_mul(Exp_msk1)
                        {
                            break 'bigloop;
                        }
                        if xat(delta, 0) == 0 && (*delta).wds <= 1 {
                            /* exact result */
                            break 'bigloop;
                        }
                        delta = lshift(delta, Log2P);
                        if cmp(delta, bs) > 0 {
                            drop_down = true;
                        } else {
                            break 'bigloop;
                        }
                    }

                    if drop_down || i == 0 {
                        /* exactly half-way between */
                        if !drop_down {
                            if bc.dsign != 0 {
                                let want: u32 = if bc.scale != 0 && {
                                    y = rv.w0() & Exp_mask;
                                    y <= ((2 * P) as u32).wrapping_mul(Exp_msk1)
                                } {
                                    0xffffffffu32
                                        & (0xffffffffu32
                                            .wrapping_shl((2 * P + 1) as u32 - (y >> Exp_shift)))
                                } else {
                                    0xffffffff
                                };
                                if (rv.w0() & Bndry_mask1) == Bndry_mask1 && rv.w1() == want {
                                    /* boundary case -- increment exponent */
                                    if rv.w0() == Big0 && rv.w1() == Big1 {
                                        fin = Fin::Ovfl;
                                        break 'main;
                                    }
                                    rv.set_w0((rv.w0() & Exp_mask).wrapping_add(Exp_msk1));
                                    rv.set_w1(0);
                                    bc.dsign = 0;
                                    break 'bigloop;
                                }
                            } else if (rv.w0() & Bndry_mask) == 0 && rv.w1() == 0 {
                                drop_down = true;
                            }
                        }

                        if drop_down {
                            /* boundary case -- decrement exponent */
                            if bc.scale != 0 {
                                L = (rv.w0() & Exp_mask) as i32;
                                if L <= (((2 * P + 1) as u32).wrapping_mul(Exp_msk1)) as i32 {
                                    if L > (((P + 2) as u32).wrapping_mul(Exp_msk1)) as i32 {
                                        /* round even ==> accept rv */
                                        break 'bigloop;
                                    }
                                    /* rv = smallest denormal */
                                    if bc.nd > nd {
                                        bc.uflchk = 1;
                                        break 'bigloop;
                                    }
                                    fin = Fin::Undfl;
                                    break 'main;
                                }
                            }
                            L = ((rv.w0() & Exp_mask).wrapping_sub(Exp_msk1)) as i32;
                            rv.set_w0((L as u32) | Bndry_mask1);
                            rv.set_w1(0xffffffff);
                            if bc.nd > nd {
                                break 'body; /* goto cont */
                            }
                            break 'bigloop;
                        }

                        if Lsb1 != 0 {
                            if rv.w0() & Lsb1 == 0 {
                                break 'bigloop;
                            }
                        } else if rv.w1() & Lsb == 0 {
                            break 'bigloop;
                        }

                        if bc.dsign != 0 {
                            let su = sulp(&mut rv, &bc);
                            rv.set_d(rv.d() + su);
                        } else {
                            let su = sulp(&mut rv, &bc);
                            rv.set_d(rv.d() - su);
                            if rv.d() == 0.0 {
                                if bc.nd > nd {
                                    bc.uflchk = 1;
                                    break 'bigloop;
                                }
                                fin = Fin::Undfl;
                                break 'main;
                            }
                        }
                        bc.dsign = 1 - bc.dsign;
                        break 'bigloop;
                    }

                    aadj = ratio(delta, bs);
                    if aadj <= 2.0 {
                        if bc.dsign != 0 {
                            aadj = 1.0;
                            aadj1 = 1.0;
                        } else if rv.w1() != 0 || (rv.w0() & Bndry_mask) != 0 {
                            if rv.w1() == Tiny1 && rv.w0() == 0 {
                                if bc.nd > nd {
                                    bc.uflchk = 1;
                                    break 'bigloop;
                                }
                                fin = Fin::Undfl;
                                break 'main;
                            }
                            aadj = 1.0;
                            aadj1 = -1.0;
                        } else {
                            if aadj < 2.0 / FLT_RADIX {
                                aadj = 1.0 / FLT_RADIX;
                            } else {
                                aadj *= 0.5;
                            }
                            aadj1 = -aadj;
                        }
                    } else {
                        aadj *= 0.5;
                        aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                        if Flt_Rounds == 0 {
                            aadj1 += 0.5;
                        }
                    }
                    y = rv.w0() & Exp_mask;

                    /* Check for overflow */

                    if y == Exp_msk1.wrapping_mul((DBL_MAX_EXP + Bias - 1) as u32) {
                        rv0.set_d(rv.d());
                        rv.set_w0(rv.w0().wrapping_sub((P as u32).wrapping_mul(Exp_msk1)));
                        adj.set_d(aadj1 * ulp(&rv));
                        rv.set_d(rv.d() + adj.d());
                        if (rv.w0() & Exp_mask)
                            >= Exp_msk1.wrapping_mul((DBL_MAX_EXP + Bias - P) as u32)
                        {
                            if rv0.w0() == Big0 && rv0.w1() == Big1 {
                                fin = Fin::Ovfl;
                                break 'main;
                            }
                            rv.set_w0(Big0);
                            rv.set_w1(Big1);
                            break 'body; /* goto cont */
                        } else {
                            rv.set_w0(rv.w0().wrapping_add((P as u32).wrapping_mul(Exp_msk1)));
                        }
                    } else {
                        if bc.scale != 0 && y <= ((2 * P) as u32).wrapping_mul(Exp_msk1) {
                            if aadj <= 0x7fffffff as f64 {
                                z = aadj as u32;
                                if z == 0 {
                                    z = 1;
                                }
                                aadj = z as f64;
                                aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                            }
                            aadj2.set_d(aadj1);
                            aadj2.set_w0(
                                aadj2
                                    .w0()
                                    .wrapping_add(((2 * P + 1) as u32).wrapping_mul(Exp_msk1))
                                    .wrapping_sub(y),
                            );
                            aadj1 = aadj2.d();
                            adj.set_d(aadj1 * ulp(&rv));
                            rv.set_d(rv.d() + adj.d());
                            if rv.d() == 0.0 {
                                req_bigcomp = 1;
                                break 'bigloop;
                            }
                        } else {
                            adj.set_d(aadj1 * ulp(&rv));
                            rv.set_d(rv.d() + adj.d());
                        }
                    }
                    z = rv.w0() & Exp_mask;

                    if bc.nd == nd {
                        if bc.scale == 0 {
                            if y == z {
                                /* Can we stop now? */
                                L = aadj as i32;
                                aadj -= L as f64;
                                /* The tolerances below are conservative. */
                                if bc.dsign != 0 || rv.w1() != 0 || (rv.w0() & Bndry_mask) != 0
                                {
                                    if aadj < 0.4999999 || aadj > 0.5000001 {
                                        break 'bigloop;
                                    }
                                } else if aadj < 0.4999999 / FLT_RADIX {
                                    break 'bigloop;
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
                bd0 = null_mut();
                bc.e0 += nz1;
                bigcomp(&mut rv, s0, &mut bc);
                y = rv.w0() & Exp_mask;
                if y == Exp_mask {
                    fin = Fin::Ovfl;
                    break 'main;
                }
                if y == 0 && rv.d() == 0.0 {
                    fin = Fin::Undfl;
                    break 'main;
                }
            }

            if bc.scale != 0 {
                rv0.set_w0(Exp_1.wrapping_sub(((2 * P) as u32).wrapping_mul(Exp_msk1)));
                rv0.set_w1(0);
                rv.set_d(rv.d() * rv0.d());
                /* try to avoid the bug of testing an 8087 register value */
                if (rv.w0() & Exp_mask) == 0 {
                    set_errno(ERANGE);
                }
            }

            fin = Fin::Ret;
        }

        /* terminal labels */
        if fin == Fin::Ovfl {
            /* Can't trust HUGE_VAL */
            rv.set_w0(Exp_mask);
            rv.set_w1(0);
            fin = Fin::RangeErr;
        }
        if fin == Fin::Undfl {
            rv.set_d(0.0);
            fin = Fin::RangeErr;
        }
        if fin == Fin::RangeErr {
            if !bd0.is_null() {
                Bfree(bb);
                Bfree(bd);
                Bfree(bs);
                Bfree(bd0);
                Bfree(delta);
            }
            set_errno(ERANGE);
            fin = Fin::Ret;
        }
        if fin == Fin::Ret0 {
            s = s00;
            sign = 0;
        }

        /* ret: */
        if !se.is_null() {
            *se = s as *mut c_char;
        }
        if sign != 0 { -rv.d() } else { rv.d() }
    }
}
