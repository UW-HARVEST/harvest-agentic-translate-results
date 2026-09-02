//! Minimal re-implementation of the C library's `printf` `%g` conversion,
//! matching glibc byte for byte (including round-half-to-even on exact ties,
//! `nan`/`inf` spellings and the two-digit minimum exponent field).

/// Number of fractional digits requested from Rust's exact float formatter.
/// A `f64` that came from a `f32` needs at most ~113 significant decimal
/// digits to be written exactly (the worst case is the smallest subnormal),
/// so 160 fractional digits is always exact, with zero padding beyond.
const EXACT_FRAC_DIGITS: usize = 160;

/// `printf("%.*g", precision, value)` for a `double`.
pub fn format_g(value: f64, precision: usize) -> String {
    // glibc: a precision of 0 is treated as 1.
    let p = if precision == 0 { 1 } else { precision };

    if value.is_nan() {
        return if value.is_sign_negative() {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }

    let neg = value.is_sign_negative();
    let mag = if neg { -value } else { value };

    let (digits, exp) = exact_decimal(mag);
    let (digits, exp) = round_significant(&digits, exp, p);

    let body = if exp < -4 || exp >= p as i32 {
        // %e style with precision p-1, trailing zeros removed.
        let mut frac: Vec<u8> = digits[1..p].to_vec();
        while frac.last() == Some(&b'0') {
            frac.pop();
        }
        let mut s = String::new();
        s.push(digits[0] as char);
        if !frac.is_empty() {
            s.push('.');
            s.push_str(std::str::from_utf8(&frac).unwrap());
        }
        let (esign, eabs) = if exp < 0 {
            ('-', (-(exp as i64)) as u64)
        } else {
            ('+', exp as u64)
        };
        s.push('e');
        s.push(esign);
        if eabs < 10 {
            s.push('0');
        }
        s.push_str(&eabs.to_string());
        s
    } else {
        // %f style with precision p-1-exp, trailing zeros removed.
        let mut s = String::new();
        if exp >= 0 {
            let int_len = (exp as usize) + 1;
            s.push_str(std::str::from_utf8(&digits[..int_len]).unwrap());
            let mut frac: Vec<u8> = digits[int_len..p].to_vec();
            while frac.last() == Some(&b'0') {
                frac.pop();
            }
            if !frac.is_empty() {
                s.push('.');
                s.push_str(std::str::from_utf8(&frac).unwrap());
            }
        } else {
            let mut frac: Vec<u8> = Vec::new();
            for _ in 0..(-exp - 1) {
                frac.push(b'0');
            }
            frac.extend_from_slice(&digits[..p]);
            while frac.last() == Some(&b'0') {
                frac.pop();
            }
            s.push('0');
            if !frac.is_empty() {
                s.push('.');
                s.push_str(std::str::from_utf8(&frac).unwrap());
            }
        }
        s
    };

    if neg {
        format!("-{}", body)
    } else {
        body
    }
}

/// Returns the exact decimal digits of `mag` (>= 0, finite) together with the
/// decimal exponent `e` such that `mag == 0.d0 d1 ... * 10^(e+1)`, i.e. the
/// value is `d0.d1 d2 ... * 10^e`.
fn exact_decimal(mag: f64) -> (Vec<u8>, i32) {
    if mag == 0.0 {
        return (vec![b'0'; EXACT_FRAC_DIGITS + 1], 0);
    }
    // Rust's `{:.*e}` uses an exact (big-integer) algorithm, so the digits are
    // the true decimal expansion, zero padded once it terminates.
    let s = format!("{:.*e}", EXACT_FRAC_DIGITS, mag);
    let (mantissa, exponent) = s.split_once('e').expect("exponential form");
    let digits: Vec<u8> = mantissa.bytes().filter(|b| *b != b'.').collect();
    let exp: i32 = exponent.parse().expect("decimal exponent");
    (digits, exp)
}

/// Rounds `digits` (value `digits[0].digits[1..] * 10^exp`) to `p` significant
/// digits using round-to-nearest, ties-to-even, as glibc does.
fn round_significant(digits: &[u8], exp: i32, p: usize) -> (Vec<u8>, i32) {
    let mut out: Vec<u8> = Vec::with_capacity(p + 1);
    out.extend_from_slice(&digits[..digits.len().min(p)]);
    while out.len() < p {
        out.push(b'0');
    }
    if digits.len() <= p {
        return (out, exp);
    }

    let round_up = {
        let first = digits[p];
        match first.cmp(&b'5') {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => {
                if digits[p + 1..].iter().any(|d| *d != b'0') {
                    true
                } else {
                    // Exact tie: round so that the last kept digit is even.
                    (out[p - 1] - b'0') % 2 == 1
                }
            }
        }
    };

    if !round_up {
        return (out, exp);
    }

    let mut i = p;
    loop {
        if i == 0 {
            // Carry out of the most significant digit: 999.. -> 1000..
            out.insert(0, b'1');
            out.truncate(p);
            return (out, exp + 1);
        }
        i -= 1;
        if out[i] == b'9' {
            out[i] = b'0';
        } else {
            out[i] += 1;
            return (out, exp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_g;

    /// `(f32 bit pattern, printf("%%.9g", (double) value))` — every expected
    /// string was produced by glibc's `printf` itself, so this pins the
    /// `%%e`/`%%f` style switch, the trailing-zero trimming, the two-digit
    /// exponent field, the round-half-to-even ties and the nan/inf spellings.
    #[rustfmt::skip]
    const CASES: &[(u32, &str)] = &[
    (0x00000000, "0"),
    (0x80000000, "-0"),
    (0x3f800000, "1"),
    (0xbf800000, "-1"),
    (0x3f000000, "0.5"),
    (0xbf000000, "-0.5"),
    (0x40000000, "2"),
    (0x7f800000, "inf"),
    (0xff800000, "-inf"),
    (0x7fc00000, "nan"),
    (0xffc00000, "-nan"),
    (0x7fc00001, "nan"),
    (0xffc00001, "-nan"),
    (0x7f800001, "nan"),
    (0xffffffff, "-nan"),
    (0x7fffffff, "nan"),
    (0x00000001, "1.40129846e-45"),
    (0x80000001, "-1.40129846e-45"),
    (0x007fffff, "1.17549421e-38"),
    (0x00800000, "1.17549435e-38"),
    (0x7f7fffff, "3.40282347e+38"),
    (0xff7fffff, "-3.40282347e+38"),
    (0x4b000000, "8388608"),
    (0x4b7fffff, "16777215"),
    (0x497423ff, "999999.938"),
    (0x3dcccccd, "0.100000001"),
    (0x3f8ccccd, "1.10000002"),
    (0x322bcc77, "9.99999994e-09"),
    (0x0da24260, "1e-30"),
    (0x71c37937, "1.93587573e+30"),
    (0x4cbebc20, "100000000"),
    (0x501502f9, "1e+10"),
    (0x3f7ffffe, "0.999999881"),
    (0x3f7fffff, "0.99999994"),
    (0x477fff00, "65535"),
    (0x477fff80, "65535.5"),
    (0x4b189680, "10000000"),
    (0x4b18967f, "9999999"),
    (0x3c23d70a, "0.00999999978"),
    (0x2edbe6ff, "1.00000001e-10"),
    (0x15798ee2, "5.03978642e-26"),
    (0x66ffffff, "6.04462874e+23"),
    (0x0a4fb11f, "1.00000002e-32"),
    (0x38d1b717, "9.99999975e-05"),
    (0x358637bd, "9.99999997e-07"),
    (0x3d4ccccd, "0.0500000007"),
    (0xbe99999a, "-0.300000012"),
    (0x41200000, "10"),
    (0x447a0000, "1000"),
    (0x461c4000, "10000"),
    (0x48f42400, "500000"),
    (0x4e6e6b28, "1e+09"),
    (0x51ba43b7, "9.9999998e+10"),
    (0x5368d4a5, "9.99999996e+11"),
    (0x551184e7, "9.99999983e+12"),
    (0x56b5e621, "1e+14"),
    (0x585e1bc9, "9.76843671e+14"),
    (0x5a0e1bca, "1.00000003e+16"),
    (0x5bb1a2bc, "9.99999984e+16"),
    (0x5d5e0b6b, "9.99999984e+17"),
    (0x5f0ac723, "9.99999998e+18"),
    (0x60ad78ec, "1.00000002e+20"),
    (0x6258d727, "1.00000002e+21"),
    (0x64078678, "9.99999978e+21"),
    (0x65a96816, "9.99999978e+22"),
    (0x6753c21c, "1.00000001e+24"),
    (0x69045951, "9.99999956e+24"),
    (0x6aa56fa6, "1.00000003e+26"),
    (0x6c4ecb8f, "9.99999988e+26"),
    (0x6e013f39, "9.99999944e+27"),
    (0x6fa18f08, "1.00000002e+29"),
    (0x714b1ae6, "1.0057276e+30"),
    (0x72fe80fe, "1.00819372e+31"),
    (0x749f1d10, "1.00850178e+32"),
    (0x7648e6d4, "1.0186925e+33"),
    (0x77f684df, "9.99999979e+33"),
    (0x799a130c, "1.00000004e+35"),
    (0x7b4097ce, "9.99999962e+35"),
    (0x7cf0bdc2, "9.99999993e+36"),
    (0x7e967699, "9.99999968e+37"),
    (0x00000002, "2.80259693e-45"),
    (0x00000003, "4.20389539e-45"),
    (0x000003ff, "1.43352833e-42"),
    (0x00001000, "5.73971851e-42"),
    (0x33d6bf95, "1.00000001e-07"),
    (0x24e69595, "1.00000002e-16"),
    (0x1e3ce508, "9.99999968e-21"),
    (0x7857dd86, "1.75130758e+34"),
    (0x2e84496e, "6.01570876e-11"),
    (0xba6f875c, "-0.000913729658"),
    (0x940eee3c, "-7.21615135e-27"),
    (0x4dc2a627, "408208608"),
    (0x33406bc4, "4.48014958e-08"),
    (0xe325faa6, "-3.0617739e+21"),
    (0xb938451e, "-0.000175733556"),
    (0x68fb90d7, "9.50388649e+24"),
    (0xc1d8fac1, "-27.1224384"),
    (0xb7740a63, "-1.45459517e-05"),
    (0xc2354e2b, "-45.3263359"),
    (0x43e58844, "459.064575"),
    (0x887e8400, "-7.65905099e-34"),
    (0x3ec33dd6, "0.381331146"),
    (0xa2da95a8, "-5.92474731e-18"),
    (0xd0055979, "-8.94893363e+09"),
    (0xbc38d756, "-0.0112818088"),
    (0x7f90ade7, "nan"),
    (0x5aab0a37, "2.40717262e+16"),
    (0x6a769806, "7.45284155e+25"),
    (0x86ff0de2, "-9.5940738e-35"),
    (0xba4e6c36, "-0.000787440105"),
    (0x9d9b532a, "-4.1114148e-21"),
    (0xf6978770, "-1.53668716e+33"),
    (0x37d72e4a, "2.56515523e-05"),
    (0x4f3d4e7b, "3.17603712e+09"),
    (0x8b389064, "-3.55457384e-32"),
    (0xb43d4318, "-1.76264052e-07"),
    (0x548a84a5, "4.75944714e+12"),
    (0x84f42b4b, "-5.74038826e-36"),
    (0x131db618, "1.99059651e-27"),
    (0xbb3a6a06, "-0.00284445425"),
    (0xc63d5f77, "-12119.8662"),
    (0xffe9ec11, "-nan"),
    (0xddfa7fa4, "-2.25629077e+18"),
    (0x34d57084, "3.97562076e-07"),
    (0xb070e384, "-8.76348105e-10"),
    (0xc0a9c8be, "-5.30575466"),
    (0xba49c19f, "-0.00076963933"),
    (0x77f1caf0, "9.80827816e+33"),
    (0xecb736d8, "-1.77193934e+27"),
    (0xb5834f4c, "-9.78333901e-07"),
    (0xd4dd79d3, "-7.60985315e+12"),
    (0xdf354788, "-1.30625588e+19"),
    (0xa70b967a, "-1.93716986e-15"),
    (0x25fbab1b, "4.36575048e-16"),
    (0x88177abd, "-4.55841878e-34"),
    (0x366dadc0, "3.54168878e-06"),
    (0xde11ee00, "-2.62883554e+18"),
    (0x6972f683, "1.83577604e+25"),
    (0xfaaced22, "-4.48942342e+35"),
    (0x0ef8e010, "6.13524845e-30"),
    (0xc54be01c, "-3262.00684"),
    (0x597538cb, "4.31398836e+15"),
    (0xa0f09780, "-4.07578358e-19"),
    (0x6aa45fe0, "9.93582957e+25"),
    (0x7744001a, "3.97536033e+33"),
    (0x1fca7da2, "8.57581697e-20"),
    (0xb9ae5c8f, "-0.000332568277"),
    (0xbe564059, "-0.209229842"),
    (0x2393446a, "1.59667464e-17"),
    (0xc34797c4, "-199.592834"),
    (0x53bdf64d, "1.63176212e+12"),
    (0x63eb18aa, "8.67352418e+21"),
    (0x54b1070f, "6.08262108e+12"),
    (0x58655852, "1.00866998e+15"),
    (0xcb2d34ea, "-11351274"),
    (0x337e6853, "5.92338658e-08"),
    (0x5352d63f, "9.05537585e+11"),
    (0x6d433715, "3.77601044e+27"),
    (0x6a151044, "4.50516898e+25"),
    (0x51032369, "3.5202175e+10"),
    (0x915405c0, "-1.6725623e-28"),
    (0x36cc71a5, "6.09290373e-06"),
    (0xe7b0dfa4, "-1.67052388e+24"),
    (0xde282d59, "-3.02961001e+18"),
    (0x682f860e, "3.31554803e+24"),
    (0x3aa67f52, "0.00127027393"),
    (0x345986d3, "2.02587486e-07"),
    (0x0a5d5bfe, "1.06580648e-32"),
    (0xbffa0775, "-1.95335257"),
    (0x39a7ff57, "0.000320429652"),
    (0xc30d0ea3, "-141.057175"),
    (0x04f8c31c, "5.84836995e-36"),
    (0xf8971813, "-2.45164097e+34"),
    (0xdd8ba9d4, "-1.25797599e+18"),
    (0x4245dc03, "49.4648552"),
    (0xd80dad42, "-6.23101105e+14"),
    (0x8171a4c3, "-4.43829199e-38"),
    (0x51b8a7af, "9.91359058e+10"),
    (0xc5bc543b, "-6026.52881"),
    (0xf5d93c67, "-5.50758556e+32"),
    (0x91f68f88, "-3.89004145e-28"),
    (0xcdebefa7, "-494793952"),
    (0xb4852afb, "-2.48044529e-07"),
    (0x6bc9e141, "4.88115642e+26"),
    (0x9d1a8c83, "-2.04543731e-21"),
    (0x1cae4e65, "1.15346186e-21"),
    (0x54c4c0c3, "6.76038076e+12"),
    (0xd9f181ea, "-8.49728893e+15"),
    (0xa7193cf4, "-2.12660581e-15"),
    (0x9b509fbe, "-1.72569722e-22"),
    (0xc7c64d55, "-101530.664"),
    (0x3b08c157, "0.00208671927"),
    (0x3c6fc173, "0.0146335242"),
    (0xfebdf93a, "-1.26259075e+38"),
    (0x76da03a0, "2.21092625e+33"),
    (0x5d9f3480, "1.43399186e+18"),
    (0x22d7bd7f, "5.84765036e-18"),
    (0x33fddcd9, "1.18214025e-07"),
    (0x5e684f96, "4.18494082e+18"),
    (0x7e4ba594, "6.76732535e+37"),
    (0x9871d769, "-3.12572743e-24"),
    (0xd625c18a, "-4.55626656e+13"),
    (0xa0cb0b17, "-3.43968763e-19"),
    (0x24329a26, "3.87281661e-17"),
    (0x430e07f5, "142.031082"),
    (0x63243c73, "3.02962185e+21"),
    (0x9fa71a59, "-7.07708396e-20"),
    (0x5656a72a, "5.90033631e+13"),
    (0x55e3679f, "3.12542736e+13"),
    (0xf0a737c3, "-4.14011267e+29"),
    (0xb56211cc, "-8.42174586e-07"),
    (0xffc9492c, "-nan"),
    (0xb6770b11, "-3.68123096e-06"),
    (0x7707af4d, "2.75201406e+33"),
    (0xdc79e8e8, "-2.81373409e+17"),
    (0x2b790602, "8.84709081e-13"),
    (0xaf1f7fa6, "-1.45063156e-10"),
    (0xb2122b13, "-8.50811244e-09"),
    (0xc48ff480, "-1151.64062"),
    (0xdc769927, "-2.77644948e+17"),
    (0x21cb664a, "1.37828908e-18"),
    (0x54f1b974, "8.30558804e+12"),
    (0xecad86a1, "-1.67823947e+27"),
    (0x34f81895, "4.62114855e-07"),
    (0x94bdca5e, "-1.91639428e-26"),
    (0xafc3434b, "-3.55181079e-10"),
    (0x10c1525a, "7.625196e-29"),
    (0x1915ec28, "7.75081083e-24"),
    (0xf0edeb0c, "-5.89056813e+29"),
    (0xd7f35ceb, "-5.3516081e+14"),
    (0x2c7a15be, "3.55392061e-12"),
    (0x011ae8e6, "2.84524293e-38"),
    (0xe65ddc49, "-2.61926634e+23"),
    (0x2691949c, "1.01016768e-15"),
    (0x7029d03d, "2.10218845e+29"),
    (0xb42eddd2, "-1.62857106e-07"),
    (0x3c55de74, "0.0130535252"),
    (0x59ac4f53, "6.06261424e+15"),
    (0x3a81ea0b, "0.000991166919"),
    (0x14d92cf9, "2.19291391e-26"),
    (0x3215e848, "8.72575612e-09"),
    (0xcd9201ff, "-306200544"),
    (0x48c3fd37, "401385.719"),
    (0x3d41c6df, "0.0473087989"),
    (0xb1ee4a0a, "-6.93512892e-09"),
    (0x760ee1c3, "7.24497212e+32"),
    (0x9a094dea, "-2.83938745e-23"),
    (0xbff20c11, "-1.89099324"),
    (0x3e422378, "0.189588428"),
    (0xbf03097d, "-0.51186353"),
    (0x4071abeb, "3.77611804"),
    (0xc0191ef2, "-2.39251375"),
    (0xc05bb9d9, "-3.43321824"),
    (0xbfca3d13, "-1.57998884"),
    (0xc03a7cbc, "-2.91386318"),
    (0x3fa57b99, "1.2928344"),
    (0xbffff8d8, "-1.99978161"),
    (0xc04c7ca5, "-3.1951077"),
    (0xc0100857, "-2.25050902"),
    (0xc01bef3a, "-2.43647623"),
    (0xbf64263b, "-0.891208351"),
    (0x3dfa26a7, "0.122144036"),
    (0xc00db3e5, "-2.21410489"),
    (0xbf94e0cb, "-1.16311014"),
    (0x3fcf6810, "1.62036324"),
    (0x40197c7e, "2.3982234"),
    (0x3f2180a3, "0.630869091"),
    (0x405d3380, "3.45626831"),
    (0x3eb4e09d, "0.353276163"),
    (0x405f4cad, "3.48905492"),
    (0x3fe1c0e1, "1.7636987"),
    (0x3f88a117, "1.06741607"),
    (0xc0434d56, "-3.05159521"),
    (0xc0637296, "-3.55386877"),
    (0xc03fc1f4, "-2.99621296"),
    (0x4028ba22, "2.63636065"),
    (0x40503740, "3.25337219"),
    (0x3f6b68f9, "0.919570506"),
    (0xc0530240, "-3.29701233"),
    (0x3d07aac3, "0.0331218354"),
    (0xc00709f6, "-2.10998297"),
    (0x3f2be6b0, "0.671488762"),
    (0x3fed6795, "1.85472357"),
    (0xc02d09ab, "-2.70371509"),
    (0xc03fd6b6, "-2.99747992"),
    (0xc04a998a, "-3.16562128"),
    (0x40631f2f, "3.5487783"),
    (0x3efca026, "0.493409336"),
    (0x407d3f4e, "3.95698881"),
    (0x3f86aac8, "1.05208683"),
    (0x3f435946, "0.763080955"),
    (0x3ef5e554, "0.48026526"),
    (0x3eb5646d, "0.354281813"),
    (0xbfbdb4ea, "-1.48208356"),
    (0xc04117ea, "-3.0170846"),
    (0xc071316f, "-3.76864219"),
    (0xc060856d, "-3.50814366"),
    (0x3f8a98e8, "1.08279133"),
    (0x3f7fc9f4, "0.99917531"),
    (0x3f757b38, "0.958911419"),
    (0xc0146427, "-2.31861281"),
    (0xc01593f9, "-2.33715653"),
    (0xc01eabca, "-2.47923517"),
    (0xc020afe5, "-2.51073575"),
    (0xbfbecf4a, "-1.49070096"),
    (0x3fee2e2a, "1.86078382"),
    (0x3f89860e, "1.07440352"),
    (0x402b5380, "2.67697144"),
    (0x401c946e, "2.44655943"),
    (0xc010c090, "-2.26175308"),
    (0x3f30cde8, "0.69064188"),
    (0xbed1fd7a, "-0.410136998"),
    (0xbf271244, "-0.652622461"),
    (0x3fcfe797, "1.62425506"),
    (0x4043c80e, "3.05908537"),
    (0xc02de33b, "-2.71699405"),
    (0xbf5119d4, "-0.816800356"),
    (0x4001586d, "2.02102208"),
    (0xbf92c278, "-1.14655972"),
    (0x3fbf1bf1, "1.4930402"),
    (0xc042f1a8, "-3.04599953"),
    (0x3fb2e57c, "1.39762831"),
    (0x400bb000, "2.18261719"),
    (0x400114aa, "2.01688623"),
    (0xbf256fc2, "-0.646236539"),
    (0x4073e439, "3.81080461"),
    (0xc01256c1, "-2.28654504"),
    ];

    #[test]
    fn matches_glibc_printf() {
        for (bits, expected) in CASES {
            let v = f32::from_bits(*bits);
            assert_eq!(
                format_g(v as f64, 9),
                *expected,
                "%.9g of f32 bits {bits:#010x}"
            );
        }
    }

    /// glibc treats a precision of 0 as 1.
    #[test]
    fn precision_zero_behaves_like_one() {
        assert_eq!(format_g(0.0, 0), format_g(0.0, 1));
        assert_eq!(format_g(1.5, 0), "2");
    }
}
