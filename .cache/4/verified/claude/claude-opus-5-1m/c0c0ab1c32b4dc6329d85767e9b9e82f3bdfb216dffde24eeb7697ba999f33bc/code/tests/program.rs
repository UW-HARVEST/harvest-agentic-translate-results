// Phase B -- CONFIGS.md rows 26..37: the top-level entry point.
//
// Every case is checked four ways: the compiled C program, the compiled Rust
// program, the `main` export of the C `.so` and the `main` export of the Rust
// `.so` (the last two are reached through `examples/so_runner.rs`, which
// `dlopen`s the object and calls its `main` symbol with a real stdin/stdout).

mod common;

use common::{assert_program_matches, Rng};

const ITERS: usize = 50;

// ------------------------------------------------------------ generators ----

const WS: &[u8] = b" \t\n\x0b\x0c\r";

fn sep(rng: &mut Rng) -> Vec<u8> {
    let n = rng.range_incl(1, 3) as usize;
    (0..n).map(|_| *rng.pick(WS)).collect()
}

/// A token that `%d` converts successfully, in a random spelling.
fn valid_token(rng: &mut Rng) -> String {
    let magnitude: i64 = match rng.below(6) {
        0 => rng.range_incl(0, 9),
        1 => rng.range_incl(0, 1000),
        2 => rng.range_incl(0, i32::MAX as i64),
        3 => rng.range_incl(i32::MAX as i64 - 4, i32::MAX as i64 + 4),
        4 => rng.range_incl(2_147_483_648, 9_223_372_036_854_775_806),
        _ => rng.next_u32() as i64,
    };
    let sign = match rng.below(3) {
        0 => "",
        1 => "+",
        _ => "-",
    };
    let zeros = "0".repeat(rng.below(4) as usize);
    format!("{sign}{zeros}{magnitude}")
}

fn join_tokens(rng: &mut Rng, tokens: &[String], trailing_newline: bool) -> Vec<u8> {
    let mut s: Vec<u8> = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 {
            s.extend(sep(rng));
        }
        s.extend(t.as_bytes());
    }
    if trailing_newline {
        s.push(b'\n');
    }
    s
}

fn spaced(tokens: &[String]) -> Vec<u8> {
    let mut s = tokens.join(" ").into_bytes();
    s.push(b'\n');
    s
}

// ------------------------------------------------------------------ rows ----

#[test]
fn row26_empty_stdin() {
    assert_program_matches("row26-empty", b"");
}

#[test]
fn row27_single_token() {
    let mut rng = Rng::new(0x2601);
    for i in 0..ITERS {
        let t = valid_token(&mut rng);
        assert_program_matches(&format!("row27#{i}"), format!("{t}\n").as_bytes());
    }
    // plus the exact i32 boundaries
    for v in [0i64, 1, -1, i32::MAX as i64, i32::MIN as i64] {
        assert_program_matches("row27-boundary", format!("{v}\n").as_bytes());
    }
}

#[test]
fn row28_k_tokens_space_separated() {
    let mut rng = Rng::new(0x2802);
    for i in 0..ITERS {
        let k = rng.range_incl(2, 98) as usize;
        let tokens: Vec<String> = (0..k).map(|_| valid_token(&mut rng)).collect();
        assert_program_matches(&format!("row28#{i}(k={k})"), &spaced(&tokens));
    }
}

#[test]
fn row29_exactly_99_and_100_tokens() {
    let mut rng = Rng::new(0x2903);
    for k in [99usize, 100] {
        for i in 0..10 {
            let tokens: Vec<String> = (0..k).map(|_| valid_token(&mut rng)).collect();
            assert_program_matches(&format!("row29#{i}(k={k})"), &spaced(&tokens));
        }
    }
}

#[test]
fn row30_more_than_capacity() {
    let mut rng = Rng::new(0x3004);
    for i in 0..ITERS {
        let k = rng.range_incl(101, 150) as usize;
        let tokens: Vec<String> = (0..k).map(|_| valid_token(&mut rng)).collect();
        assert_program_matches(&format!("row30#{i}(k={k})"), &spaced(&tokens));
    }
}

#[test]
fn row31_mixed_whitespace_separators() {
    let mut rng = Rng::new(0x3105);
    for i in 0..ITERS {
        let k = rng.range_incl(1, 40) as usize;
        let tokens: Vec<String> = (0..k).map(|_| valid_token(&mut rng)).collect();
        let nl = rng.bool();
        let bytes = join_tokens(&mut rng, &tokens, nl);
        assert_program_matches(&format!("row31#{i}"), &bytes);
    }
}

#[test]
fn row32_leading_whitespace_and_no_trailing_newline() {
    let mut rng = Rng::new(0x3206);
    for i in 0..ITERS {
        let k = rng.range_incl(1, 20) as usize;
        let tokens: Vec<String> = (0..k).map(|_| valid_token(&mut rng)).collect();
        let mut bytes = sep(&mut rng);
        bytes.extend(join_tokens(&mut rng, &tokens, false));
        assert_program_matches(&format!("row32#{i}"), &bytes);
    }
    // whitespace only
    assert_program_matches("row32-ws-only", b" \t\n\x0b\x0c\r  ");
}

#[test]
fn row33_sign_and_leading_zero_spellings() {
    let mut rng = Rng::new(0x3307);
    for i in 0..ITERS {
        let k = rng.range_incl(1, 30) as usize;
        let tokens: Vec<String> = (0..k)
            .map(|_| {
                let sign = match rng.below(3) {
                    0 => "",
                    1 => "+",
                    _ => "-",
                };
                let zeros = "0".repeat(rng.below(20) as usize);
                let v = rng.range_incl(0, 4_294_967_296);
                format!("{sign}{zeros}{v}")
            })
            .collect();
        assert_program_matches(&format!("row33#{i}"), &spaced(&tokens));
    }
    for t in ["0", "-0", "+0", "0000", "-0000", "+0000000000000000000000005"] {
        assert_program_matches("row33-lit", format!("{t}\n").as_bytes());
    }
}

#[test]
fn row34_magnitude_classes() {
    let mut rng = Rng::new(0x3408);
    for i in 0..ITERS {
        let k = rng.range_incl(1, 30) as usize;
        let tokens: Vec<String> = (0..k)
            .map(|_| match rng.below(7) {
                0 => format!("{}", rng.range_incl(-100, 100)),
                1 => format!("{}", i32::MAX as i64 + rng.range_incl(-3, 3)),
                2 => format!("{}", i32::MIN as i64 + rng.range_incl(-3, 3)),
                3 => format!("{}", rng.range_incl(2_147_483_648, 9_223_372_036_854_775_806)),
                4 => format!("-{}", rng.range_incl(2_147_483_649, 9_223_372_036_854_775_807)),
                5 => {
                    // strictly above LONG_MAX
                    let digits = rng.range_incl(20, 45) as usize;
                    let mut s = String::from("9");
                    while s.len() < digits {
                        s.push((b'0' + rng.below(10) as u8) as char);
                    }
                    s
                }
                _ => {
                    // strictly below LONG_MIN
                    let digits = rng.range_incl(20, 45) as usize;
                    let mut s = String::from("-9");
                    while s.len() < digits + 1 {
                        s.push((b'0' + rng.below(10) as u8) as char);
                    }
                    s
                }
            })
            .collect();
        assert_program_matches(&format!("row34#{i}"), &spaced(&tokens));
    }
    for t in [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999999999999999",
        "-99999999999999999999999999999999",
    ] {
        assert_program_matches("row34-lit", format!("{t}\n").as_bytes());
    }
}

#[test]
fn row35_no_separator_sign_glued_tokens() {
    let mut rng = Rng::new(0x3509);
    for i in 0..ITERS {
        let k = rng.range_incl(2, 30) as usize;
        let mut s = String::new();
        for j in 0..k {
            let v = rng.range_incl(0, 100_000);
            if j == 0 && rng.bool() {
                s.push_str(&format!("{v}"));
            } else {
                s.push(if rng.bool() { '+' } else { '-' });
                s.push_str(&format!("{v}"));
            }
        }
        s.push('\n');
        assert_program_matches(&format!("row35#{i}"), s.as_bytes());
    }
    for t in ["1-2+3-4", "1+2", "5-0", "-1-2", "+1+1+1"] {
        assert_program_matches("row35-lit", format!("{t}\n").as_bytes());
    }
}

#[test]
fn row36_invalid_token_after_k_valid() {
    const BAD: &[&str] = &[
        "abc", "zz", ".", "..", "x", "e5", "--5", "+-5", "-", "+", "/", ":", "'", "\"", "%",
        "0x10", "1.5", "1e3", "5x", "nan", "inf",
    ];
    let mut rng = Rng::new(0x360a);
    for i in 0..ITERS {
        let k = rng.range_incl(0, 20) as usize;
        let mut tokens: Vec<String> = (0..k).map(|_| valid_token(&mut rng)).collect();
        tokens.push((*rng.pick(BAD)).to_string());
        let tail = rng.range_incl(0, 5) as usize;
        for _ in 0..tail {
            tokens.push(valid_token(&mut rng));
        }
        let nl = rng.bool();
        let bytes = join_tokens(&mut rng, &tokens, nl);
        assert_program_matches(&format!("row36#{i}"), &bytes);
    }
}

#[test]
fn row37_random_byte_soup() {
    const ALPHABET: &[u8] = b"0123456789+- \t\n\x0b\x0c\r.xa\0\x80\x7f9";
    let mut rng = Rng::new(0x370b);
    for i in 0..600 {
        let n = rng.range_incl(0, 60) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHABET)).collect();
        assert_program_matches(&format!("row37#{i}"), &bytes);
    }
}

#[test]
fn row37b_long_random_byte_soup() {
    // Longer inputs, biased towards digits so that many tokens actually convert
    // (and so that some digit runs get long enough to saturate `strtol`).
    const ALPHABET: &[u8] = b"00112233445566778899+-  \t\n\r.,x\0";
    let mut rng = Rng::new(0x370c);
    for i in 0..300 {
        let n = rng.range_incl(60, 500) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHABET)).collect();
        assert_program_matches(&format!("row37b#{i}"), &bytes);
    }
}

#[test]
fn row38_main_export_called_repeatedly() {
    // C's `stdin` is a global FILE: calling the `main` export again in the same
    // process resumes the stream where the previous call stopped.
    let mut rng = Rng::new(0x380c);
    for i in 0..30 {
        let k = rng.range_incl(1, 260) as usize;
        let tokens: Vec<String> = (0..k).map(|_| valid_token(&mut rng)).collect();
        let stdin = spaced(&tokens);
        for n in ["1", "2", "3"] {
            let c = common::run_so(&common::c_so(), &["main_n", n], &stdin);
            let r = common::run_so(&common::rust_so(), &["main_n", n], &stdin);
            assert_eq!(
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&r.stdout),
                "[row38#{i}] `main` x{n} stdout differs for {} tokens",
                k
            );
            assert_eq!(c.stdout, r.stdout, "[row38#{i}] `main` x{n} bytes differ");
            assert_eq!(c.status, r.status, "[row38#{i}] `main` x{n} status differs");
        }
    }
}

#[test]
fn row39_fma_array_misaligned_pointers() {
    // A C caller may hand `fma_array` a misaligned `int *`; on x86-64 that is
    // just an unaligned load and the C has no check for it.
    let mut rng = Rng::new(0x390d);
    for i in 0..60 {
        let len = rng.range_incl(1, 24) as i32;
        let vals: Vec<i32> = (0..4 * len).map(|_| rng.next_i32()).collect();
        let stdin: Vec<u8> = vals
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .into_bytes();
        let len_s = len.to_string();
        let c = common::run_so(&common::c_so(), &["fma_misaligned", &len_s], &stdin);
        let r = common::run_so(&common::rust_so(), &["fma_misaligned", &len_s], &stdin);
        assert_eq!(c.status, r.status, "[row39#{i}] status differs");
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
            "[row39#{i}] misaligned fma_array result differs (len={len})"
        );
        assert_eq!(c.stdout, r.stdout, "[row39#{i}] stdout differs");
    }
}
