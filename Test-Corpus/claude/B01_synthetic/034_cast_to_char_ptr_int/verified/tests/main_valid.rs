//! Phase B — valid-path differential tests for the exported `int main(void)`
//! (rows C10–C24 of CONFIGS.md).
//!
//! `main` is the one-shot wrapper: `scanf("%d", &x)` followed by `driver(x)`.
//! Every case runs the C `.so`'s `main` and the Rust `.so`'s `main` in a forked
//! child with identical stdin/stdout wiring and compares stdout **and** the exit
//! status byte for byte.

mod common;

use common::*;

/// C10 — plain decimal digits, no sign, randomised over the whole `int` range.
#[test]
fn c10_main_unsigned_decimal_random() {
    let mut rng = Rng::new(0xC010);
    for _ in 0..400 {
        let v = rng.next_u64() as u32 & 0x7fff_ffff;
        diff_main_input(format!("{v}\n").as_bytes());
    }
    for v in [0u32, 1, 7, 10, 99, 100, 12345, 1_000_000_000] {
        diff_main_input(format!("{v}").as_bytes());
    }
}

/// C11 — `-` sign, magnitudes up to 2^31 (covers `-0` and `INT_MIN`).
#[test]
fn c11_main_negative_random() {
    let mut rng = Rng::new(0xC011);
    for _ in 0..400 {
        let m = rng.next_u64() % 2_147_483_649;
        diff_main_input(format!("-{m}\n").as_bytes());
    }
    for m in [0u64, 1, 2_147_483_647, 2_147_483_648, 2_147_483_649] {
        diff_main_input(format!("-{m}").as_bytes());
    }
}

/// C12 — explicit `+` sign.
#[test]
fn c12_main_explicit_plus_random() {
    let mut rng = Rng::new(0xC012);
    for _ in 0..400 {
        let m = rng.next_u64() % 4_294_967_296;
        diff_main_input(format!("+{m}\n").as_bytes());
    }
    for m in [0u64, 1, 2_147_483_647, 2_147_483_648, 4_294_967_295] {
        diff_main_input(format!("+{m}\n").as_bytes());
    }
}

/// C13 — every kind of C whitespace, in randomised runs, before the number.
#[test]
fn c13_main_leading_whitespace_kinds() {
    const WS: [u8; 6] = [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'];
    // Each whitespace character on its own first.
    for w in WS {
        for n in 1..=3usize {
            let mut input = vec![w; n];
            input.extend_from_slice(b"12345");
            diff_main_input(&input);
            let mut signed = vec![w; n];
            signed.extend_from_slice(b"-12345");
            diff_main_input(&signed);
        }
    }
    // Then randomised mixtures.
    let mut rng = Rng::new(0xC013);
    for _ in 0..300 {
        let len = 1 + rng.below(20) as usize;
        let mut input: Vec<u8> = (0..len).map(|_| rng.pick(&WS)).collect();
        let sign = rng.pick(&["", "-", "+"]);
        let v = rng.next_u64() as u32;
        input.extend_from_slice(format!("{sign}{v}").as_bytes());
        diff_main_input(&input);
    }
}

/// C14 — leading zeros (glibc's `strtol` skips them; the digit count still
/// grows, which is what the Rust accumulator has to cope with).
#[test]
fn c14_main_leading_zeros() {
    let mut rng = Rng::new(0xC014);
    for _ in 0..300 {
        let zeros = 1 + rng.below(40) as usize;
        let sign = rng.pick(&["", "-", "+"]);
        let v = rng.next_u64() % 10_000_000_000;
        let input = format!("{sign}{}{v}\n", "0".repeat(zeros));
        diff_main_input(input.as_bytes());
    }
    // All-zero digit strings.
    for n in [1usize, 2, 19, 20, 21, 100] {
        diff_main_input(format!("{}\n", "0".repeat(n)).as_bytes());
        diff_main_input(format!("-{}\n", "0".repeat(n)).as_bytes());
    }
}

/// C15 — a valid number followed by junk: only the first conversion happens.
#[test]
fn c15_main_trailing_junk() {
    let tails: [&[u8]; 12] = [
        b"abc",
        b"ABC",
        b".5",
        b",",
        b"x10",
        b"e9",
        b"-7",
        b" 4711",
        b"\n99",
        b"\t\t",
        b"\0rest",
        b"\xff\xfe",
    ];
    for t in tails {
        for head in ["0", "1", "-1", "+2147483647", "42"] {
            let mut input = head.as_bytes().to_vec();
            input.extend_from_slice(t);
            diff_main_input(&input);
        }
    }
    let mut rng = Rng::new(0xC015);
    for _ in 0..200 {
        let v = rng.next_i32();
        let t = rng.pick(&tails);
        let mut input = format!("{v}").into_bytes();
        input.extend_from_slice(t);
        diff_main_input(&input);
    }
}

/// C16 — line-ending shapes and numbers split across lines.
#[test]
fn c16_main_line_ending_shapes() {
    let cases: [&[u8]; 12] = [
        b"12",
        b"12\n",
        b"12\r\n",
        b"12\n\n",
        b"12\r",
        b"12\n34",
        b"12\r34",
        b"\n12\n",
        b"\r\n12",
        b"-12\r\n",
        b"+12\n",
        b"\n\n\n12\n\n\n",
    ];
    for c in cases {
        diff_main_input(c);
    }
}

/// C17 — digit-string length ladder 1…25, both signs: walks `int` → `long` →
/// overflow with randomised digits.
#[test]
fn c17_main_digit_length_ladder() {
    let mut rng = Rng::new(0xC017);
    for len in 1..=25usize {
        for _ in 0..10 {
            let digits = rng.digits(len);
            for sign in ["", "-", "+"] {
                diff_main_input(format!("{sign}{digits}\n").as_bytes());
            }
        }
        // Deterministic extremes for this length: all 9s and 1 followed by 0s.
        diff_main_input(format!("{}\n", "9".repeat(len)).as_bytes());
        diff_main_input(format!("-{}\n", "9".repeat(len)).as_bytes());
        diff_main_input(format!("1{}\n", "0".repeat(len - 1)).as_bytes());
        diff_main_input(format!("-1{}\n", "0".repeat(len - 1)).as_bytes());
    }
}

/// C18 — digit strings that cross glibc's 4 KiB stdio buffer and Rust's 8 KiB
/// `BufReader`.
#[test]
fn c18_main_huge_digit_strings() {
    let mut rng = Rng::new(0xC018);
    for n in [100usize, 1_000, 4_095, 4_096, 4_097, 8_191, 8_192, 8_193, 20_000] {
        let digits = rng.digits(n);
        diff_main_input(format!("{digits}\n").as_bytes());
        diff_main_input(format!("-{digits}\n").as_bytes());
        diff_main_input(format!("{}\n", "9".repeat(n)).as_bytes());
        diff_main_input(format!("-{}\n", "0".repeat(n)).as_bytes());
        // Leading zeros so the accumulated value stays small despite the length.
        diff_main_input(format!("{}7\n", "0".repeat(n)).as_bytes());
    }
}

/// C19 — the exact numeric boundaries, plus randomised 64- and 128-bit
/// magnitudes.
#[test]
fn c19_main_numeric_boundaries() {
    let exact = [
        "2147483646",
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "4294967294",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967295",
        "-4294967296",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-9223372036854775810",
        "18446744073709551615",
        "18446744073709551616",
        "18446744073709551617",
        "-18446744073709551616",
        "10000000000000000000",
        "100000000000000000000",
        "170141183460469231731687303715884105727",
        "-170141183460469231731687303715884105728",
        "340282366920938463463374607431768211456",
    ];
    for s in exact {
        diff_main_input(format!("{s}\n").as_bytes());
        diff_main_input(s.as_bytes());
    }

    let mut rng = Rng::new(0xC019);
    for _ in 0..200 {
        let a = rng.next_u64();
        let b = rng.next_u64();
        for s in [
            format!("{a}\n"),
            format!("-{a}\n"),
            format!("{a}{b:020}\n"),
            format!("-{a}{b:020}\n"),
        ] {
            diff_main_input(s.as_bytes());
        }
    }
}

/// C20 — stdin is an unseekable pipe instead of a regular file.
#[test]
fn c20_main_stdin_is_a_pipe() {
    let mut rng = Rng::new(0xC020);
    for _ in 0..200 {
        let sign = rng.pick(&["", "-", "+"]);
        let v = rng.next_u64() % 100_000_000_000;
        let input = format!("{sign}{v}\n");
        diff_main(Stdin::Pipe(input.as_bytes()), Stdout::File, &input);
    }
    let fixed: [&[u8]; 8] = [
        b"",
        b"   ",
        b"\n",
        b"abc",
        b"-",
        b"0",
        b"2147483648",
        b"9223372036854775808",
    ];
    for f in fixed {
        diff_main(Stdin::Pipe(f), Stdout::File, &preview(f));
        diff_main(Stdin::Pipe(f), Stdout::Pipe, &preview(f));
    }
}

/// C21 — whitespace runs longer than both stdio buffers before the number.
#[test]
fn c21_main_whitespace_across_buffer_boundary() {
    for n in [4_095usize, 4_096, 4_097, 8_191, 8_192, 8_193, 40_000] {
        let mut input = vec![b' '; n];
        input.extend_from_slice(b"-1234\n");
        diff_main_input(&input);

        let mut mixed: Vec<u8> = (0..n)
            .map(|i| [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'][i % 6])
            .collect();
        mixed.extend_from_slice(b"98765\n");
        diff_main_input(&mixed);

        // Whitespace only: input failure after a very long skip.
        diff_main_input(&vec![b'\n'; n]);
    }
}

/// C22 — NUL and non-ASCII bytes around the digits.
#[test]
fn c22_main_nul_and_non_ascii_bytes() {
    let cases: [&[u8]; 14] = [
        b"\0",
        b"\0 12",
        b"12\0",
        b"12\0\0\0",
        b"\x80",
        b"\xff",
        b"\xff12",
        b"12\xff",
        b"\xc3\xa9",
        b"\xc3\xa912",
        b"12\xc3\xa9",
        b"\x7f12",
        b"\x1f12",
        b"-\x0012",
    ];
    for c in cases {
        diff_main_input(c);
    }
    // Randomised byte soup: mostly junk, so mostly matching failures.
    let mut rng = Rng::new(0xC022);
    for _ in 0..300 {
        let len = 1 + rng.below(12) as usize;
        let input: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        diff_main_input(&input);
    }
}

/// C23 — the return value of the exported `main` (it must be 0 for every input,
/// valid or not); the child's exit code carries it.
#[test]
fn c23_main_return_value() {
    for input in [&b""[..], b"42\n", b"abc", b"-", b"9223372036854775808"] {
        let run = diff_main_input(input);
        assert_eq!(
            run.status,
            Status::Exited(0),
            "`main` must return 0 for {}",
            preview(input)
        );
    }
}

/// C24 (main half) — `main` and `driver` driven in the same process, in one
/// child, sharing the stdout buffer.
#[test]
fn c24_main_and_driver_interleaved() {
    let p = pair();
    let input = b"7\n";
    let c = run_child(Stdin::File(input), Stdout::File, || unsafe {
        (p.c.driver)(1);
        let rc = (p.c.main)();
        (p.c.driver)(-1);
        rc
    });
    let r = run_child(Stdin::File(input), Stdout::File, || unsafe {
        (p.rs.driver)(1);
        let rc = (p.rs.main)();
        (p.rs.driver)(-1);
        rc
    });
    assert_eq!(
        (as_text(&c.out), c.status),
        (as_text(&r.out), r.status),
        "C24 main/driver interleaving"
    );
    assert_eq!(as_text(&c.out), "01000000\n07000000\nffffffff\n");
}
