//! Phase B — differential tests for `int main(int argc, char **argv)` called
//! through the `.so` exports of both builds (CONFIGS.md rows 9–26).
//!
//! This binary intentionally contains a SINGLE `#[test]` function: the harness
//! redirects the process-wide fd 1 to capture what `main` prints, so no other
//! test may run concurrently (see `common::capture_fd1`).

mod common;

use common::{argv1, fresh_pair, Pair, Rng, SEED};

const TAG: &str = "ffi_main";

fn check(pair: &Pair, arg: &[u8], ctx: &str) {
    pair.assert_main_same_auto(&argv1(arg), ctx);
}

#[test]
fn ffi_main_differential_all_configurations() {
    // ---------------------------------------------------------- row 9 -------
    // `main` observes the same `sum` that direct `static_sum` calls updated.
    for pre in [1i32, -5, 1_000_000, i32::MAX, i32::MIN] {
        let pair = fresh_pair(TAG);
        assert_eq!(
            pair.c.static_sum(pre),
            pair.rust.static_sum(pre),
            "static_sum({pre}) differs"
        );
        pair.assert_main_same_auto(&argv1(b"3"), &format!("row 9, pre = {pre}"));
        assert_eq!(
            pair.c.static_sum(0),
            pair.rust.static_sum(0),
            "running total after main differs (pre = {pre})"
        );
    }

    let pair = fresh_pair(TAG);

    // --------------------------------------------------------- row 10 -------
    for arg in [
        &b"0"[..], b"1", b"2", b"7", b"-1", b"-3", b"10", b"-10", b"99", b"-99",
    ] {
        check(&pair, arg, "row 10 small strides");
    }

    // --------------------------------------------------------- row 11 -------
    for arg in [
        &b"+2"[..],
        b"-0",
        b"+0",
        b"0000",
        b" 7",
        b"\t-4",
        b"\n\x0b\x0c\r 12",
        b"   +000123",
        b"\r\n-000000000000000005",
        b"          8",
        b"+00000000000000000009",
    ] {
        check(&pair, arg, "row 11 whitespace/sign/zeros");
    }
    // randomized whitespace/sign/leading-zero decorations
    let mut rng = Rng::new(SEED ^ 0xA1);
    let spaces: [&[u8]; 7] = [b"", b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r"];
    let signs: [&[u8]; 3] = [b"", b"+", b"-"];
    for _ in 0..200 {
        let mut s = Vec::new();
        for _ in 0..rng.below(4) {
            s.extend_from_slice(*rng.pick(&spaces));
        }
        s.extend_from_slice(*rng.pick(&signs));
        for _ in 0..rng.below(4) {
            s.push(b'0');
        }
        let n = rng.below(1_000_000);
        s.extend_from_slice(n.to_string().as_bytes());
        check(&pair, &s, "row 11 randomized decoration");
    }

    // --------------------------------------------------------- row 12 -------
    for arg in [
        &b"5abc"[..], b"0x10", b"0X10", b"3 4", b"7\n", b"12.", b"9,9", b"1e5", b"6-", b"2+3",
        b"42/", b"8:", b"-7xyz", b"+9 ", b"0b101",
    ] {
        check(&pair, arg, "row 12 trailing garbage");
    }
    let suffix_bytes: &[u8] = b"abcxyzXYZ .,;:/-+*_'\"\\|()[]{}\t\n\r\x0b\x0c#$%&!?@^~`=<>";
    for _ in 0..200 {
        let mut s = Vec::new();
        if rng.below(2) == 0 {
            s.extend_from_slice(*rng.pick(&signs));
        }
        let n = rng.below(100_000);
        s.extend_from_slice(n.to_string().as_bytes());
        let extra = 1 + rng.below(4);
        for _ in 0..extra {
            s.push(suffix_bytes[rng.below(suffix_bytes.len() as u64) as usize]);
        }
        check(&pair, &s, "row 12 randomized garbage suffix");
    }

    // --------------------------------------------------------- row 13 -------
    for digits in 1..=19u32 {
        for sign in ["", "+", "-"] {
            // smallest and largest value of this width, plus randoms
            let lo = if digits == 1 {
                0u128
            } else {
                10u128.pow(digits - 1)
            };
            let hi = 10u128.pow(digits) - 1;
            for v in [lo, hi] {
                check(&pair, format!("{sign}{v}").as_bytes(), "row 13 digit sweep");
            }
            for _ in 0..6 {
                let span = (hi - lo + 1) as u64 as u128;
                let v = lo + (rng.next_u64() as u128) % span.max(1);
                check(
                    &pair,
                    format!("{sign}{v}").as_bytes(),
                    "row 13 digit sweep random",
                );
            }
        }
    }

    // --------------------------------------------------------- row 14 -------
    for len in [20usize, 21, 25, 39, 40, 63, 64, 100, 199, 200] {
        for sign in ["", "+", "-"] {
            let all9: String = "9".repeat(len);
            let all1: String = "1".repeat(len);
            let mut zeros = String::from("0").repeat(len - 1);
            zeros.push('7');
            for body in [all9, all1, zeros] {
                check(
                    &pair,
                    format!("{sign}{body}").as_bytes(),
                    "row 14 long digit run",
                );
            }
            let random: String = (0..len)
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect();
            check(
                &pair,
                format!("{sign}{random}").as_bytes(),
                "row 14 long random digit run",
            );
        }
    }

    // --------------------------------------------------------- row 15 -------
    for _ in 0..400 {
        let v = rng.next_i32();
        check(&pair, v.to_string().as_bytes(), "row 15 random i32 stride");
    }

    // ----------------------------------------------------- rows 16, 17 ------
    for arg in [
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "-4294967296",
        "8589934592",
        "-8589934592",
        "6442450944",
        "2147483646",
    ] {
        check(&pair, arg.as_bytes(), "rows 16/17 int truncation");
    }

    // ----------------------------------------------------- rows 18–21 ------
    for arg in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "-18446744073709551616",
        "+9223372036854775807",
        "+9223372036854775808",
    ] {
        check(&pair, arg.as_bytes(), "rows 18-21 long saturation");
    }

    // --------------------------------------------------------- row 22 -------
    for _ in 0..300 {
        let v = rng.next_u64() as i64;
        check(&pair, v.to_string().as_bytes(), "row 22 random i64 string");
        let shift = rng.below(64) as u32;
        let w = (rng.next_u64() >> shift) as i64;
        check(&pair, w.to_string().as_bytes(), "row 22 random shifted i64");
    }

    // --------------------------------------------------------- row 23 -------
    for _ in 0..400 {
        let len = rng.below(12) as usize;
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            // Mix of digits, signs, spaces, letters and high-bit bytes.
            let class = rng.below(10);
            let b = match class {
                0..=4 => b'0' + rng.below(10) as u8,
                5 => *rng.pick(&[b'+', b'-']),
                6 => *rng.pick(&[b' ', b'\t', b'\n', b'\x0b', b'\x0c', b'\r']),
                7 => b'a' + rng.below(26) as u8,
                8 => 0x80 + rng.below(0x80) as u8,
                _ => 0x21 + rng.below(0x5e) as u8,
            };
            s.push(b);
        }
        check(&pair, &s, "row 23 random byte soup");
    }

    // --------------------------------------------------------- row 24 -------
    for arg in [
        &b"\xff7"[..],
        b"7\xff",
        b"\xc3\xa9",
        b"\xc3\xa95",
        b"5\xc3\xa9",
        b"\x80\x81",
        b"-\xff3",
        b" \xffx",
    ] {
        check(&pair, arg, "row 24 non-UTF-8 argument");
    }

    // --------------------------------------------------------- row 25 -------
    // argc == 2 but argv carries further entries: they must be ignored.
    for extra in [1usize, 2, 5] {
        let mut args = argv1(b"4");
        for i in 0..extra {
            args.push(format!("ignored{i}").into_bytes());
        }
        pair.assert_main_same(2, &args, "row 25 extra argv entries");
    }

    // --------------------------------------------------------- row 26 -------
    // Repeated `main` calls on the same instance keep accumulating `sum`.
    let repeat_pair = fresh_pair(TAG);
    for round in 0..3 {
        repeat_pair.assert_main_same_auto(&argv1(b"6"), &format!("row 26 round {round}"));
    }
    for round in 0..3 {
        repeat_pair.assert_main_same_auto(
            &argv1(b"-1000000000"),
            &format!("row 26 wrapping round {round}"),
        );
    }
    assert_eq!(
        repeat_pair.c.static_sum(0),
        repeat_pair.rust.static_sum(0),
        "row 26: accumulated state differs after repeated main() calls"
    );

    // A last sanity check that both libraries are still in lock-step.
    assert_eq!(pair.c.static_sum(0), pair.rust.static_sum(0));
}
