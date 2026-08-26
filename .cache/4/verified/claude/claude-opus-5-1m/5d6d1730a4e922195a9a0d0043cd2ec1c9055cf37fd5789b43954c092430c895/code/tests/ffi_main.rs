// Phase B — differential tests for the composed entry point, `main`
// (`scanf("%lf", &f)` followed by `driver(f)`).
//
// CONFIGS.md rows 16-35.  Both shared objects are loaded with `libloading`,
// standard input is redirected to a file holding the exact test bytes, and only
// the exported `main` symbol is called.

mod common;

use common::{
    diff_driver_bits, diff_main, diff_main_strs, push_digits, push_n_digits, push_sign, push_ws,
    random_case, Rng,
    DIGITS, HEXDIGITS, SEED, WS,
};

const N: usize = 3000;

fn gen<F: FnMut(&mut Rng, &mut Vec<u8>)>(seed: u64, n: usize, mut f: F) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut v = Vec::new();
        f(&mut rng, &mut v);
        out.push(v);
    }
    out
}

fn strs(list: &[&str]) -> Vec<Vec<u8>> {
    list.iter().map(|s| s.as_bytes().to_vec()).collect()
}

/// CONFIGS row 16 — decimal, integer digits only, no exponent
fn row_16_decimal_integer_only() {
    let inputs = gen(SEED ^ 16, N, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        let n = 1 + rng.below(20) as usize;
        push_digits(rng, v, n, DIGITS);
    });
    diff_main("row16 decimal int only", &inputs);
}

/// CONFIGS row 17 — decimal with sign, integer and fraction digits
fn row_17_decimal_with_fraction() {
    let inputs = gen(SEED ^ 17, N, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        let a = 1 + rng.below(18) as usize;
        push_digits(rng, v, a, DIGITS);
        v.push(b'.');
        let b = 1 + rng.below(18) as usize;
        push_digits(rng, v, b, DIGITS);
    });
    diff_main("row17 decimal int+frac", &inputs);
}

/// CONFIGS row 18 — fraction only, and a trailing decimal point
fn row_18_fraction_only_and_trailing_dot() {
    let mut inputs = gen(SEED ^ 18, N, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        if rng.flip() {
            v.push(b'.');
            let b = 1 + rng.below(20) as usize;
            push_digits(rng, v, b, DIGITS);
        } else {
            let a = 1 + rng.below(20) as usize;
            push_digits(rng, v, a, DIGITS);
            v.push(b'.');
        }
    });
    inputs.extend(strs(&[
        ".5", "-.5", "+.5", "5.", "-5.", "+5.", ".0", "-.0", "0.", "-0.", ".00000", "000.",
        ".5e3", "5.e3", "-.5E-3", ".5p3",
    ]));
    diff_main("row18 frac only / trailing dot", &inputs);
}

/// CONFIGS row 19 — decimal with an `e`/`E` exponent
fn row_19_decimal_exponent() {
    let inputs = gen(SEED ^ 19, N, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        let a = rng.below(12) as usize;
        push_digits(rng, v, a, DIGITS);
        if rng.flip() || a == 0 {
            v.push(b'.');
            let b = 1 + rng.below(12) as usize;
            push_digits(rng, v, b, DIGITS);
        }
        v.push(if rng.flip() { b'e' } else { b'E' });
        push_sign(rng, v);
        let n = 1 + rng.below(3) as usize;
        push_digits(rng, v, n, DIGITS);
    });
    diff_main("row19 decimal exponent", &inputs);
}

/// CONFIGS row 20 — extreme and malformed-looking exponents
fn row_20_extreme_exponents() {
    let mut inputs = gen(SEED ^ 20, N, |rng, v| {
        push_sign(rng, v);
        let a = 1 + rng.below(6) as usize;
        push_digits(rng, v, a, DIGITS);
        if rng.flip() {
            v.push(b'.');
            push_n_digits(rng, v, 1, 6, DIGITS);
        }
        v.push(if rng.flip() { b'e' } else { b'E' });
        let neg = rng.flip();
        v.push(if neg { b'-' } else { b'+' });
        match rng.below(6) {
            0 => v.extend_from_slice(format!("{}", rng.range(280, 340)).as_bytes()),
            1 => v.extend_from_slice(format!("{}", rng.range(300, 320)).as_bytes()),
            2 => v.extend_from_slice(b"999"),
            3 => v.extend_from_slice(b"1000000000"),
            4 => {
                // leading zeros, then a normal exponent
                for _ in 0..rng.below(20) {
                    v.push(b'0');
                }
                v.extend_from_slice(format!("{}", rng.range(0, 400)).as_bytes());
            }
            _ => {
                // absurdly long exponent digit string
                for _ in 0..(rng.below(400) + 20) {
                    v.push(*rng.pick(DIGITS));
                }
            }
        }
    });
    inputs.extend(strs(&[
        "1e309", "1e308", "-1e309", "1e-323", "1e-324", "1e-325", "1e-400", "1e99999999999999",
        "1e-99999999999999", "1e+00000000000000000000000000000005", "0e999999999999999999",
        "0e-999999999999999999", "1e2147483647", "1e2147483648", "1e-2147483648", "1e4294967296",
        "1e9223372036854775807", "1e-9223372036854775809", "1e18446744073709551616",
    ]));
    diff_main("row20 extreme exponents", &inputs);
}

/// CONFIGS row 21 — 17-digit mantissas at the representability boundary
fn row_21_boundary_mantissas() {
    let mut inputs = gen(SEED ^ 21, 4000, |rng, v| {
        push_sign(rng, v);
        // first digit non-zero so the exponent means what it says
        v.push(*rng.pick(b"123456789"));
        push_n_digits(rng, v, 16, 19, DIGITS);
        v.push(b'e');
        let e = match rng.below(4) {
            0 => rng.range(-340, -290),
            1 => rng.range(290, 320),
            2 => rng.range(-30, 30),
            _ => rng.range(-330, 310),
        };
        v.extend_from_slice(format!("{e}").as_bytes());
    });
    inputs.extend(strs(&[
        // classic strtod stress values
        "2.2250738585072011e-308",
        "2.2250738585072012e-308",
        "2.2250738585072013e-308",
        "2.2250738585072014e-308",
        "1.7976931348623157e308",
        "1.7976931348623158e308",
        "1.7976931348623159e308",
        "2.4703282292062327e-324",
        "2.4703282292062328e-324",
        "7.4109846876186981e-323",
        "9007199254740992",
        "9007199254740993",
        "9007199254740994",
        "9007199254740995",
        "1.000000000000000055511151231257827021181583404541015625",
        "0.500000000000000166533453693773481063544750213623046875",
        "4503599627370496.5",
        "4503599627370497.5",
        "1e23",
        "8.98846567431158e307",
        "5e-324",
        "3e-324",
        "2e-324",
        "1e-323",
        "0.000000000000000000000000000000000000000000000000000000000000000000000000000000001",
    ]));
    diff_main("row21 boundary mantissas", &inputs);
}

/// CONFIGS row 22 — very long digit strings (`strtod` slow path)
fn row_22_long_digit_strings() {
    let inputs = gen(SEED ^ 22, 600, |rng, v| {
        push_sign(rng, v);
        let total = 100 + rng.below(700) as usize;
        let dot = rng.below(total as u64 + 1) as usize;
        for i in 0..total {
            if i == dot {
                v.push(b'.');
            }
            v.push(*rng.pick(DIGITS));
        }
        if rng.flip() {
            v.push(b'e');
            push_sign(rng, v);
            v.extend_from_slice(format!("{}", rng.range(0, 400)).as_bytes());
        }
    });
    diff_main("row22 long digit strings", &inputs);
}

/// CONFIGS row 23 — leading white space in every form
fn row_23_leading_whitespace() {
    let mut inputs = gen(SEED ^ 23, 1500, |rng, v| {
        let n = rng.below(12);
        for _ in 0..n {
            v.push(*rng.pick(&WS));
        }
        push_sign(rng, v);
        push_n_digits(rng, v, 1, 8, DIGITS);
        if rng.flip() {
            v.push(b'.');
            push_n_digits(rng, v, 1, 8, DIGITS);
        }
    });
    // one input per white-space byte, alone in front of the number
    for w in WS {
        inputs.push([&[w][..], b"12.5"].concat());
        inputs.push([&[w, w, w][..], b"-7"].concat());
        inputs.push(vec![w]);
    }
    // a very long run of white space
    let mut long = vec![b' '; 4096];
    long.extend_from_slice(b"3.5");
    inputs.push(long);
    let mut long2 = vec![b'\n'; 4096];
    long2.extend_from_slice(b"-0x1.8p3");
    inputs.push(long2);
    inputs.push(vec![b' '; 100_000]);
    diff_main("row23 leading whitespace", &inputs);
}

fn push_hex_prefix(rng: &mut Rng, v: &mut Vec<u8>) {
    v.push(b'0');
    v.push(if rng.flip() { b'x' } else { b'X' });
}

/// CONFIGS row 24 — hexadecimal, integer hex digits only, no `p`
fn row_24_hex_integer_only() {
    let inputs = gen(SEED ^ 24, N, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        push_hex_prefix(rng, v);
        push_n_digits(rng, v, 1, 14, HEXDIGITS);
    });
    diff_main("row24 hex int only", &inputs);
}

/// CONFIGS row 25 — hexadecimal with a fraction and no `p`
fn row_25_hex_fraction() {
    let mut inputs = gen(SEED ^ 25, N, |rng, v| {
        push_sign(rng, v);
        push_hex_prefix(rng, v);
        let a = rng.below(10) as usize;
        push_digits(rng, v, a, HEXDIGITS);
        v.push(b'.');
        let b = if a == 0 { 1 + rng.below(10) as usize } else { rng.below(10) as usize };
        push_digits(rng, v, b, HEXDIGITS);
    });
    inputs.extend(strs(&[
        "0x1.8", "0x.8", "0xa.", "-0x.8", "+0X.F", "0x0.0", "0x0.", "0x.0", "0X.",
    ]));
    diff_main("row25 hex fraction", &inputs);
}

/// CONFIGS row 26 — hexadecimal with a `p` exponent in the normal range
fn row_26_hex_p_exponent() {
    let inputs = gen(SEED ^ 26, N, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        push_hex_prefix(rng, v);
        let a = 1 + rng.below(8) as usize;
        push_digits(rng, v, a, HEXDIGITS);
        if rng.flip() {
            v.push(b'.');
            push_n_digits(rng, v, 0, 7, HEXDIGITS);
        }
        v.push(if rng.flip() { b'p' } else { b'P' });
        push_sign(rng, v);
        v.extend_from_slice(format!("{}", rng.below(60)).as_bytes());
    });
    diff_main("row26 hex p exponent", &inputs);
}

/// CONFIGS row 27 — more than 14 significant hex digits: rounding, sticky bits
/// and exact ties on the 53rd significand bit
fn row_27_hex_rounding() {
    let mut inputs = gen(SEED ^ 27, 4000, |rng, v| {
        push_sign(rng, v);
        push_hex_prefix(rng, v);
        match rng.below(3) {
            0 => {
                // long random digit string
                push_n_digits(rng, v, 14, 40, HEXDIGITS);
                if rng.flip() {
                    v.push(b'.');
                    push_n_digits(rng, v, 0, 19, HEXDIGITS);
                }
            }
            1 => {
                // exact tie: 53 significant bits then a single 1 bit
                v.push(*rng.pick(b"89abcdef"));
                v.push(b'.');
                push_digits(rng, v, 12, HEXDIGITS);
                v.push(b'8');
                for _ in 0..rng.below(8) {
                    v.push(b'0');
                }
            }
            _ => {
                // tie plus a sticky bit somewhere further out
                v.push(*rng.pick(b"89abcdef"));
                v.push(b'.');
                push_digits(rng, v, 12, HEXDIGITS);
                v.push(b'8');
                for _ in 0..rng.below(8) {
                    v.push(b'0');
                }
                v.push(*rng.pick(b"123456789abcdef"));
            }
        }
        if rng.flip() {
            v.push(b'p');
            push_sign(rng, v);
            v.extend_from_slice(format!("{}", rng.below(40)).as_bytes());
        }
    });
    inputs.extend(strs(&[
        "0x1.0000000000000p0",
        "0x1.0000000000008p0",
        "0x1.0000000000008000001p0",
        "0x1.0000000000018p0",
        "0x1.fffffffffffff8p0",
        "0x1.ffffffffffffffffffffp0",
        "0x1fffffffffffff",
        "0x1fffffffffffff8",
        "0x1fffffffffffff9",
        "0x20000000000000",
        "0x20000000000001",
        "0x1p53",
        "0x1.0000000000000000000000000000001p0",
        "0xffffffffffffffffffffffffffffffffffffffff",
    ]));
    diff_main("row27 hex rounding", &inputs);
}

/// CONFIGS row 28 — `p` exponents that land in the subnormal range or overflow
fn row_28_hex_subnormal_and_overflow() {
    let mut inputs = gen(SEED ^ 28, 4000, |rng, v| {
        push_sign(rng, v);
        push_hex_prefix(rng, v);
        match rng.below(3) {
            0 => v.push(b'1'),
            1 => {
                v.push(*rng.pick(b"123456789abcdef"));
                v.push(b'.');
                push_n_digits(rng, v, 1, 16, HEXDIGITS);
            }
            _ => push_n_digits(rng, v, 1, 16, HEXDIGITS),
        }
        v.push(b'p');
        let e = if rng.flip() {
            rng.range(-1090, -1000)
        } else {
            rng.range(1000, 1040)
        };
        v.extend_from_slice(format!("{e}").as_bytes());
    });
    for e in -1090..=-1000i64 {
        inputs.push(format!("0x1p{e}").into_bytes());
        inputs.push(format!("0x1.8p{e}").into_bytes());
        inputs.push(format!("0x1.fffffffffffffp{e}").into_bytes());
        inputs.push(format!("-0x1p{e}").into_bytes());
    }
    for e in 1000..=1040i64 {
        inputs.push(format!("0x1p{e}").into_bytes());
        inputs.push(format!("0x1.fffffffffffffp{e}").into_bytes());
        inputs.push(format!("0x1.fffffffffffff8p{e}").into_bytes());
        inputs.push(format!("-0x1p{e}").into_bytes());
    }
    diff_main("row28 hex subnormal/overflow", &inputs);
}

/// CONFIGS row 29 — huge and very long `p` exponents
fn row_29_hex_huge_exponents() {
    let mut inputs = gen(SEED ^ 29, 800, |rng, v| {
        push_sign(rng, v);
        push_hex_prefix(rng, v);
        push_n_digits(rng, v, 1, 4, HEXDIGITS);
        v.push(b'p');
        v.push(if rng.flip() { b'-' } else { b'+' });
        let n = 1 + rng.below(40) as usize;
        push_digits(rng, v, n, DIGITS);
    });
    inputs.extend(strs(&[
        "0x1p+99999999999",
        "0x1p-99999999999",
        "-0x1p+99999999999",
        "-0x1p-99999999999",
        "0x1p2147483647",
        "0x1p-2147483648",
        "0x1p9223372036854775807",
        "0x1p-9223372036854775808",
        "0x1p18446744073709551617",
        "0x0p99999999999999999999999999",
        "0x0p-99999999999999999999999999",
        "0x1p00000000000000000000000000000000005",
    ]));
    diff_main("row29 hex huge exponents", &inputs);
}

/// CONFIGS row 30 — `inf` / `infinity` / `nan` in every case permutation
fn row_30_specials() {
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for base in ["inf", "infinity", "nan"] {
        let n = base.len();
        for mask in 0u32..(1u32 << n) {
            let word: Vec<u8> = base
                .bytes()
                .enumerate()
                .map(|(i, c)| if mask >> i & 1 == 1 { c ^ 0x20 } else { c })
                .collect();
            for sign in ["", "-", "+"] {
                let mut v = sign.as_bytes().to_vec();
                v.extend_from_slice(&word);
                inputs.push(v.clone());
                // with trailing junk, and with an n-char-sequence
                let mut w = v.clone();
                w.extend_from_slice(b"(1234)");
                inputs.push(w);
                let mut w = v.clone();
                w.extend_from_slice(b"()");
                inputs.push(w);
                let mut w = v.clone();
                w.extend_from_slice(b"xyz");
                inputs.push(w);
                let mut w = b"   ".to_vec();
                w.extend_from_slice(&v);
                inputs.push(w);
            }
        }
    }
    inputs.extend(strs(&[
        "nan(", "nan()", "nan(0x1)", "nan(abc", "-nan(a)", "infin", "infinityy", "INFINITY",
        "Infinity", "iNfInItY", "inf inity", "in", "-inf", "+inf", "nan1", "nanny",
    ]));
    diff_main("row30 specials", &inputs);
}

/// CONFIGS row 31 — dangling exponent characters
fn row_31_dangling_exponents() {
    let mut inputs = strs(&[
        "1e", "1e+", "1e-", "1E", "1E+", "1E-", ".5e", "0.5e+", "1e+x", "1e-x", "1ee5", "1e5e5",
        "0x1p", "0x1p+", "0x1p-", "0x1P", "0X1p+", "0x1pa", "0x1pf", "0x1p2f3", "0x1p1e",
        "0x1p-a1", "0x1pp1", "0x1p+p1", "0x1.8p", "0x1.8p-", "0xap", "0x1p+2p3", "1e1.5",
        "0x1p1.5", "1.e", "1.e5", "0x.p1", "0x1e5", "0x1E5", "0x1e+5", "0xe1",
    ]);
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..600 {
        let mut v = Vec::new();
        push_sign(&mut rng, &mut v);
        let hex = rng.flip();
        if hex {
            push_hex_prefix(&mut rng, &mut v);
            push_n_digits(&mut rng, &mut v, 1, 4, HEXDIGITS);
            v.push(if rng.flip() { b'p' } else { b'P' });
        } else {
            push_n_digits(&mut rng, &mut v, 1, 4, DIGITS);
            v.push(if rng.flip() { b'e' } else { b'E' });
        }
        match rng.below(4) {
            0 => {}
            1 => v.push(if rng.flip() { b'+' } else { b'-' }),
            2 => v.push(*rng.pick(b"abcdefxyzXYZ")),
            _ => {
                v.push(if rng.flip() { b'+' } else { b'-' });
                v.push(*rng.pick(b"abcdefxyz.+-"));
            }
        }
        inputs.push(v);
    }
    diff_main("row31 dangling exponents", &inputs);
}

/// CONFIGS row 32 — repeated decimal points and separator characters
fn row_32_dots_and_separators() {
    let mut inputs = strs(&[
        "1.2.3", "1..2", "..1", ".1.", "1.2.", "0x1.8.8", "0x..8", "1,000", "1,5", "1'0",
        "1 000", "1_000", "1;5", "1:5", "-1.2.3", "0x1.8.8p3", "1.2e3.4", "..", "...", ".e",
        "1.2,3", "0x.8.8",
    ]);
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..800 {
        let mut v = Vec::new();
        push_sign(&mut rng, &mut v);
        let n = 1 + rng.below(6) as usize;
        for _ in 0..n {
            match rng.below(4) {
                0 => v.push(b'.'),
                1 => v.push(*rng.pick(b",'_ ;:")),
                _ => push_n_digits(&mut rng, &mut v, 1, 3, DIGITS),
            }
        }
        inputs.push(v);
    }
    diff_main("row32 dots and separators", &inputs);
}

/// CONFIGS row 33 — a valid number followed by arbitrary trailing input
fn row_33_trailing_input() {
    let inputs = gen(SEED ^ 33, 2000, |rng, v| {
        push_ws(rng, v);
        push_sign(rng, v);
        if rng.flip() {
            push_n_digits(rng, v, 1, 6, DIGITS);
            v.push(b'.');
            push_n_digits(rng, v, 0, 5, DIGITS);
        } else {
            push_hex_prefix(rng, v);
            push_n_digits(rng, v, 1, 6, HEXDIGITS);
        }
        // trailing junk that must never influence the first conversion
        let n = rng.below(12) as usize;
        for _ in 0..n {
            v.push(*rng.pick(b"0123456789abcdefxXpPeE+-. \t\n,()"));
        }
    });
    diff_main("row33 trailing input", &inputs);
}

/// CONFIGS row 34 — random byte soup
fn row_34_random_soup() {
    const ALPHABET: &[u8] = b"0123456789abcdefABCDEFxXpPeE+-.,_ \t\n\r\x0b\x0cnNiIfFtTyY()'\0\x7f\xff";
    let mut inputs = gen(SEED ^ 34, 12000, |rng, v| {
        let n = rng.below(13) as usize;
        for _ in 0..n {
            v.push(*rng.pick(ALPHABET));
        }
    });
    // fully random bytes
    let mut rng = Rng::new(SEED ^ 0x34_34);
    for _ in 0..4000 {
        let n = rng.below(10) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push((rng.next_u64() & 0xff) as u8);
        }
        inputs.push(v);
    }
    // Token soup: short concatenations of the *syntactic* pieces glibc's `%lf`
    // scanner branches on.  A flat byte alphabet almost never produces a short
    // structured prefix such as "-0x", which is exactly where the interesting
    // rejection boundaries live.
    const TOKENS: &[&[u8]] = &[
        b"-", b"+", b"0", b"0x", b"0X", b".", b"e", b"E", b"p", b"P", b"1", b"9", b"a", b"f",
        b"g", b"x", b"n", b"na", b"nan", b"i", b"in", b"inf", b"infi", b"infinity", b" ",
        b"\n", b"(", b")", b"12", b"ff", b"00", b"1024", b"1075", b"8", b",", b"\0",
    ];
    for _ in 0..12000 {
        let n = 1 + rng.below(5) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            v.extend_from_slice(*rng.pick(TOKENS));
        }
        inputs.push(v);
    }
    // random case variants of number-ish words
    for _ in 0..2000 {
        let base: &[u8] = *rng.pick(&[
            &b"0x1p3"[..],
            &b"inf"[..],
            &b"nan"[..],
            &b"infinity"[..],
            &b"1e5"[..],
            &b"0X.8P-2"[..],
        ]);
        inputs.push(random_case(&mut rng, base));
    }
    diff_main("row34 random soup", &inputs);
}

/// CONFIGS row 35 — the low-level and the composed entry point interleaved in
/// one process: no buffered state may leak from one call to the next
fn row_35_interleaved_entry_points() {
    let mut rng = Rng::new(SEED ^ 35);
    for _ in 0..40 {
        let mut inputs = Vec::new();
        for _ in 0..8 {
            let mut v = Vec::new();
            push_ws(&mut rng, &mut v);
            push_sign(&mut rng, &mut v);
            push_n_digits(&mut rng, &mut v, 1, 6, DIGITS);
            if rng.flip() {
                v.push(b'.');
                push_n_digits(&mut rng, &mut v, 0, 5, DIGITS);
            }
            inputs.push(v);
        }
        diff_main("row35 interleaved main", &inputs);
        let bits: Vec<u64> = (0..8).map(|_| rng.next_u64()).collect();
        diff_driver_bits("row35 interleaved driver", &bits);
    }
    // multi-number inputs: only the first number may ever be consumed
    diff_main_strs(
        "row35 repeat reads",
        &["1 2 3", "  4.5\n6.5\n", "0x1p1 0x2p2", "inf nan", "\n\n\n7"],
    );
}

fn main() {
    common::run_suite(
        "ffi_main",
        &[
            ("row_16_decimal_integer_only", row_16_decimal_integer_only),
            ("row_17_decimal_with_fraction", row_17_decimal_with_fraction),
            (
                "row_18_fraction_only_and_trailing_dot",
                row_18_fraction_only_and_trailing_dot,
            ),
            ("row_19_decimal_exponent", row_19_decimal_exponent),
            ("row_20_extreme_exponents", row_20_extreme_exponents),
            ("row_21_boundary_mantissas", row_21_boundary_mantissas),
            ("row_22_long_digit_strings", row_22_long_digit_strings),
            ("row_23_leading_whitespace", row_23_leading_whitespace),
            ("row_24_hex_integer_only", row_24_hex_integer_only),
            ("row_25_hex_fraction", row_25_hex_fraction),
            ("row_26_hex_p_exponent", row_26_hex_p_exponent),
            ("row_27_hex_rounding", row_27_hex_rounding),
            (
                "row_28_hex_subnormal_and_overflow",
                row_28_hex_subnormal_and_overflow,
            ),
            ("row_29_hex_huge_exponents", row_29_hex_huge_exponents),
            ("row_30_specials", row_30_specials),
            ("row_31_dangling_exponents", row_31_dangling_exponents),
            ("row_32_dots_and_separators", row_32_dots_and_separators),
            ("row_33_trailing_input", row_33_trailing_input),
            ("row_34_random_soup", row_34_random_soup),
            (
                "row_35_interleaved_entry_points",
                row_35_interleaved_entry_points,
            ),
        ],
    );
}
