// Phase B — valid-path differential tests, one test per CONFIGS.md row (C1–C17).
//
// Both compiled programs are run as external processes with identical stdin and
// argv; stdout, stderr and the wait status must be byte-for-byte identical.
// Every row uses many randomized inputs from a fixed-seed PRNG.

mod common;

use common::*;

/// C1 — the only success path: x == 1, y == 2, z == 3.
#[test]
fn c1_success_path() {
    assert_same_and_expect(b"1 2 3", "Ok!\nResult: 0\n", "C1 canonical");

    let mut rng = Rng::new(0xC1);
    // Spellings of the exact triple that still convert to 1/2/3.
    let spellings: [(&str, &str, &str); 6] = [
        ("1", "2", "3"),
        ("+1", "+2", "+3"),
        ("01", "02", "03"),
        ("0000000001", "000000002", "0000003"),
        ("+00001", "2", "+0003"),
        ("1", "+0000000000000000002", "3"),
    ];
    for (x, y, z) in spellings {
        for _ in 0..8 {
            let lead = if rng.below(2) == 0 {
                String::new()
            } else {
                ws(&mut rng, 4)
            };
            let s = format!(
                "{lead}{x}{}{y}{}{z}{}",
                ws(&mut rng, 4),
                ws(&mut rng, 4),
                if rng.below(2) == 0 { "\n" } else { "" }
            );
            assert_same_str(&s, "C1 success spellings");
        }
    }
}

/// C2 — stage 1 rejection: x != 1, random y and z over the whole i32 range.
#[test]
fn c2_stage1_random() {
    let mut rng = Rng::new(0xC2);
    for _ in 0..300 {
        let mut x = rng.next_i32();
        if x == 1 {
            x = 7;
        }
        let (y, z) = (rng.next_i32(), rng.next_i32());
        assert_same_str(&format!("{x} {y} {z}"), "C2 x!=1 random");
    }
}

/// C3 — stage 2 rejection: x == 1, y != 2, random z.
#[test]
fn c3_stage2_random() {
    let mut rng = Rng::new(0xC3);
    for _ in 0..300 {
        let mut y = rng.next_i32();
        if y == 2 {
            y = 5;
        }
        let z = rng.next_i32();
        assert_same_str(&format!("1 {y} {z}"), "C3 y!=2 random");
    }
}

/// C4 — stage 3 rejection: x == 1, y == 2, z != 3.
#[test]
fn c4_stage3_random() {
    let mut rng = Rng::new(0xC4);
    for _ in 0..300 {
        let mut z = rng.next_i32();
        if z == 3 {
            z = -3;
        }
        assert_same_str(&format!("1 2 {z}"), "C4 z!=3 random");
    }
}

/// C5 — fully random triples across the whole i32 range.
#[test]
fn c5_fully_random_triples() {
    let mut rng = Rng::new(0xC5);
    for _ in 0..1000 {
        let (x, y, z) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_same_str(&format!("{x} {y} {z}"), "C5 random triple");
    }
    // Also bias towards the magic constants so all 8 B×C×D combinations and
    // their near-misses show up frequently.
    for _ in 0..600 {
        let pick = |r: &mut Rng, magic: i32| -> i32 {
            match r.below(4) {
                0 => magic,
                1 => magic + 1,
                2 => magic - 1,
                _ => *r.choose(INTERESTING),
            }
        };
        let x = pick(&mut rng, 1);
        let y = pick(&mut rng, 2);
        let z = pick(&mut rng, 3);
        assert_same_str(&format!("{x} {y} {z}"), "C5 near-magic triple");
    }
}

/// C6 — all 8 combinations of (x==1?, y==2?, z==3?) with boundary values.
#[test]
fn c6_all_stage_combinations() {
    let xs: [i32; 2] = [1, 0];
    let ys: [i32; 2] = [2, 5];
    let zs: [i32; 2] = [3, -3];
    for x in xs {
        for y in ys {
            for z in zs {
                assert_same_str(&format!("{x} {y} {z}"), "C6 combination");
            }
        }
    }
    // Same truth table, driven with extreme values in the "not equal" slots.
    let bad: [i32; 5] = [0, -1, i32::MIN, i32::MAX, 123];
    for &b in &bad {
        assert_same_str(&format!("{b} 2 3"), "C6 x extreme");
        assert_same_str(&format!("1 {b} 3"), "C6 y extreme");
        assert_same_str(&format!("1 2 {b}"), "C6 z extreme");
        assert_same_str(&format!("{b} {b} {b}"), "C6 all extreme");
    }
}

/// C7 — every whitespace byte as separator, random whitespace runs, and no
/// leading whitespace at all.
#[test]
fn c7_whitespace_forms() {
    for &s in SPACES {
        let sep = s as char;
        assert_same_str(&format!("1{sep}2{sep}3"), "C7 single ws byte");
        assert_same_str(&format!("{sep}1{sep}2{sep}3{sep}"), "C7 ws around");
        assert_same_str(&format!("9{sep}8{sep}7"), "C7 single ws byte, failing");
    }
    let mut rng = Rng::new(0xC7);
    for _ in 0..200 {
        let (x, y, z) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let s = format!(
            "{}{x}{}{y}{}{z}{}",
            if rng.below(2) == 0 { ws(&mut rng, 6) } else { String::new() },
            ws(&mut rng, 6),
            ws(&mut rng, 6),
            if rng.below(2) == 0 { ws(&mut rng, 6) } else { String::new() },
        );
        assert_same_str(&s, "C7 random ws runs");
    }
}

/// C8 — all 27 sign-prefix combinations, randomized magnitudes.
#[test]
fn c8_sign_combinations() {
    let signs = ["", "+", "-"];
    let mut rng = Rng::new(0xC8);
    for sx in signs {
        for sy in signs {
            for sz in signs {
                for _ in 0..4 {
                    let a = rng.below(4_294_967_296);
                    let b = rng.below(1000);
                    let c = rng.below(10);
                    assert_same_str(
                        &format!("{sx}{a} {sy}{b} {sz}{c}"),
                        "C8 signs",
                    );
                }
                // Also with the magic values so success/failure both appear.
                assert_same_str(&format!("{sx}1 {sy}2 {sz}3"), "C8 signs magic");
            }
        }
    }
}

/// C9 — leading zeros with random padding widths.
#[test]
fn c9_leading_zeros() {
    assert_same_and_expect(b"0000000001 000002 0003", "Ok!\nResult: 0\n", "C9 canonical");
    let mut rng = Rng::new(0xC9);
    for _ in 0..120 {
        let pad = |r: &mut Rng, v: i64| -> String {
            let n = 1 + r.below(40) as usize;
            if v < 0 {
                format!("-{}{}", "0".repeat(n), -v)
            } else {
                format!("{}{}", "0".repeat(n), v)
            }
        };
        let (x, y, z) = (
            rng.next_i32() as i64 % 1000,
            rng.next_i32() as i64 % 1000,
            rng.next_i32() as i64 % 1000,
        );
        let s = format!("{} {} {}", pad(&mut rng, x), pad(&mut rng, y), pad(&mut rng, z));
        assert_same_str(&s, "C9 zero padded");
    }
    // Pure zeros of many widths.
    for n in [1usize, 2, 8, 20, 64, 200] {
        let z = "0".repeat(n);
        assert_same_str(&format!("{z} {z} {z}"), "C9 all zeros");
    }
}

/// C10 — int-boundary magnitudes in each of the three positions.
#[test]
fn c10_int_boundaries() {
    let vals: [&str; 10] = [
        "2147483647",  // INT_MAX
        "-2147483648", // INT_MIN
        "2147483648",  // INT_MAX+1  -> narrows to INT_MIN
        "-2147483649", // INT_MIN-1  -> narrows to INT_MAX
        "4294967295",  // UINT_MAX   -> -1
        "4294967296",  // 2^32       -> 0
        "-4294967296", // -2^32      -> 0
        "2147483649",
        "-2147483647",
        "0",
    ];
    for v in vals {
        assert_same_str(&format!("{v} 2 3"), "C10 x boundary");
        assert_same_str(&format!("1 {v} 3"), "C10 y boundary");
        assert_same_str(&format!("1 2 {v}"), "C10 z boundary");
        assert_same_str(&format!("{v} {v} {v}"), "C10 all boundary");
    }
}

/// C11 — values that narrow (mod 2^32) onto the magic constants and therefore
/// *pass* a stage even though the text is not "1"/"2"/"3".
#[test]
fn c11_narrowing_onto_magic() {
    assert_same_and_expect(
        b"4294967297 4294967298 4294967299",
        "Ok!\nResult: 0\n",
        "C11 all three narrow onto 1/2/3",
    );
    assert_same_and_expect(b"-4294967295 2 3", "Ok!\nResult: 0\n", "C11 negative narrows to 1");
    for k in 1..=6i64 {
        let x = 4_294_967_296i64 * k + 1;
        let y = 4_294_967_296i64 * k + 2;
        let z = 4_294_967_296i64 * k + 3;
        assert_same_str(&format!("{x} {y} {z}"), "C11 k*2^32 + magic");
        assert_same_str(&format!("{} 2 3", -x + 2), "C11 negative narrow");
    }
}

/// C12 — long/strtol saturation territory, up to a 100 000-digit number.
#[test]
fn c12_long_saturation() {
    let vals: Vec<String> = vec![
        "9223372036854775807".into(),  // LONG_MAX
        "-9223372036854775808".into(), // LONG_MIN
        "9223372036854775808".into(),  // LONG_MAX+1 -> saturates
        "-9223372036854775809".into(), // LONG_MIN-1 -> saturates
        "18446744073709551615".into(), // 2^64-1
        "18446744073709551616".into(), // 2^64
        "-18446744073709551616".into(),
        "1".to_string() + &"0".repeat(40),
        "-1".to_string() + &"0".repeat(40),
        "9".repeat(100),
        "-".to_string() + &"9".repeat(100),
        "9".repeat(100_000),
        "-".to_string() + &"9".repeat(100_000),
        "0".repeat(100_000) + "1", // huge but tiny value
    ];
    for v in &vals {
        assert_same_str(&format!("{v} 2 3"), "C12 x saturate");
        assert_same_str(&format!("1 {v} 3"), "C12 y saturate");
        assert_same_str(&format!("1 2 {v}"), "C12 z saturate");
    }
    // Random 19..25 digit numbers straddling the LONG_MAX boundary.
    let mut rng = Rng::new(0xC12);
    for _ in 0..120 {
        let digits = 18 + rng.below(8) as usize;
        let mut s = String::new();
        if rng.below(2) == 0 {
            s.push('-');
        }
        s.push((b'1' + rng.below(9) as u8) as char);
        for _ in 1..digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        assert_same_str(&format!("{s} {s} {s}"), "C12 random big");
    }
}

/// C13 — only two conversions succeed (EOF after the 2nd); z keeps 0.
#[test]
fn c13_two_conversions() {
    assert_same_and_expect(b"1 2", "Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n", "C13 canonical");
    let mut rng = Rng::new(0xC13);
    for _ in 0..200 {
        let (x, y) = (rng.next_i32(), rng.next_i32());
        assert_same_str(&format!("{x} {y}"), "C13 two ints then EOF");
        assert_same_str(&format!("{x} {y}\n"), "C13 two ints, newline, EOF");
        assert_same_str(&format!("  {x}\t{y}  \n\n"), "C13 two ints, trailing ws");
    }
}

/// C14 — only one conversion succeeds; y keeps 123 and z keeps 0.
#[test]
fn c14_one_conversion() {
    assert_same_and_expect(b"1", "Error: x == 1 but y != 2\nOperation failed\nResult: 2\n", "C14 canonical");
    let mut rng = Rng::new(0xC14);
    for _ in 0..200 {
        let x = rng.next_i32();
        assert_same_str(&format!("{x}"), "C14 one int then EOF");
        assert_same_str(&format!("{x}\n"), "C14 one int, newline");
        assert_same_str(&format!("\t{x} \r\n"), "C14 one int, ws");
    }
    // x == 1 is the interesting one: it reaches the y stage with y still 123.
    assert_same_str("1", "C14 x==1");
    assert_same_str("+000001\n", "C14 x==1 padded");
}

/// C15 — zero conversions.
#[test]
fn c15_zero_conversions() {
    assert_same_and_expect(b"", "Error: x != 1\nOperation failed\nResult: 1\n", "C15 empty");
    for s in ["", " ", "\n", "\t", "\r\n", "   \n\n\t\t  ", "\x0b\x0c"] {
        assert_same_str(s, "C15 whitespace only");
    }
    let mut rng = Rng::new(0xC15);
    for _ in 0..60 {
        let s = ws(&mut rng, 30);
        assert_same_str(&s, "C15 random whitespace only");
    }
}

/// C16 — trailing content after the third token must never be read.
#[test]
fn c16_trailing_content() {
    let tails: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"\n".to_vec(),
        b"   ".to_vec(),
        b" 4 5 6".to_vec(),
        b"garbage".to_vec(),
        b"\n\n\nmore lines\n".to_vec(),
        vec![0u8; 16],
        b"\x00\x01\xff\xfe".to_vec(),
        vec![b'x'; 65536],
        b"9".repeat(70000),
    ];
    for tail in &tails {
        for head in ["1 2 3", "0 2 3", "1 5 3", "1 2 9"] {
            let mut input = head.as_bytes().to_vec();
            input.extend_from_slice(tail);
            assert_same(&input, "C16 trailing content");
        }
    }
}

/// C17 — tokens spread over lines, CRLF endings, no final newline.
#[test]
fn c17_multiline() {
    let cases = [
        "1\n2\n3",
        "1\n2\n3\n",
        "1\r\n2\r\n3\r\n",
        "\r\n\r\n1\r\n\r\n2\r\n3",
        "  1\n\n\n  2\n\n\n  3\n",
        "1\n2\n3\n4\n5\n",
        "0\n5\n9\n",
        "1\n2\n",
        "1\n",
        "\n\n\n",
    ];
    for c in cases {
        assert_same_str(c, "C17 multiline");
    }
    let mut rng = Rng::new(0xC17);
    for _ in 0..200 {
        let (x, y, z) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        let nl = |r: &mut Rng| if r.below(2) == 0 { "\r\n" } else { "\n" };
        let s = format!(
            "{x}{}{y}{}{z}{}",
            nl(&mut rng),
            nl(&mut rng),
            if rng.below(2) == 0 { nl(&mut rng) } else { "" }
        );
        assert_same_str(&s, "C17 random line endings");
    }
}
