//! Phase B — valid-path differential tests.
//!
//! One test per row of CONFIGS.md. Every test drives BOTH real binaries through
//! the process boundary (argv in, stdout/stderr/exit status out) and compares
//! byte-for-byte. Randomised rows use a fixed SplitMix64 seed so any failure is
//! reproducible.
//!
//! `assert_same` compares stdout, stderr, exit code and terminating signal. For
//! start values that make the C program count ~2^31 times (≈25 GB of stdout) the
//! comparison is bounded to the first N bytes of the stream (`.cap(N)`); when a
//! run finishes below the cap the comparison automatically covers the whole
//! output plus the exit status.

mod common;

use common::*;

const CAP64: usize = 64 * 1024;
const CAP32: usize = 32 * 1024;
const CAP16: usize = 16 * 1024;

/// CONFIGS.md #1 — positive numeral ending in 9: single line, immediate break.
#[test]
fn cfg_01_positive_ends_in_nine() {
    let mut rng = Rng::new(0x0000_0001_1111_1111);
    same_arg(b"9");
    same_arg(b"19");
    same_arg(b"2147483639");
    for _ in 0..512 {
        let k = rng.below(214_748_364);
        let v = k * 10 + 9; // <= 2147483649... clamp into i32 range
        let v = if v > i32::MAX as u64 { v - 10 } else { v };
        same_arg(format!("{v}").as_bytes());
    }
}

/// CONFIGS.md #2 — positive numeral with last digit 0..8: 2..10 lines.
#[test]
fn cfg_02_positive_short_loop() {
    let mut rng = Rng::new(0x0000_0002_2222_2222);
    for _ in 0..512 {
        let v = rng.below(i32::MAX as u64 - 10);
        same_arg(format!("{v}").as_bytes());
    }
    for v in 0..=20u32 {
        same_arg(format!("{v}").as_bytes());
    }
}

/// CONFIGS.md #3 — every lexical spelling of zero.
#[test]
fn cfg_03_zero_forms() {
    for s in [
        &b"0"[..],
        b"+0",
        b"-0",
        b"00000",
        b"  -000",
        b"\t+0x",
        b"-0abc",
        b"000000000000000000000000000",
        b"-00000000000000000000000000009",
        b"\n\r\x0b\x0c +0",
    ] {
        same_arg(s);
    }
}

/// CONFIGS.md #4 — exhaustive small sweep across zero.
#[test]
fn cfg_04_exhaustive_small_range() {
    for v in -300i32..=300 {
        same_arg(format!("{v}").as_bytes());
    }
}

/// CONFIGS.md #5 — small negative starts: the negative-`%` quirk plus the walk
/// up through zero to +9.
#[test]
fn cfg_05_small_negative() {
    let mut rng = Rng::new(0x0000_0005_5555_5555);
    for _ in 0..512 {
        let v = rng.range_i64(-2000, -1);
        same_arg(format!("{v}").as_bytes());
    }
}

/// CONFIGS.md #6 — negative numerals whose magnitude ends in 9 must NOT break
/// early (C's `%` truncates toward zero, giving -9).
#[test]
fn cfg_06_negative_ends_in_nine() {
    let mut rng = Rng::new(0x0000_0006_6666_6666);
    for s in [&b"-9"[..], b"-19", b"-29", b"-109", b"-1009", b"-1999"] {
        same_arg(s);
    }
    for _ in 0..256 {
        let k = rng.below(200);
        let v = -((k * 10 + 9) as i64);
        same_arg(format!("{v}").as_bytes());
    }
    // large magnitudes ending in 9: ~2^31 iterations, bounded compare
    for s in [&b"-2000000009"[..], b"-2147483639", b"-999999999"] {
        same_arg_bounded(s, CAP64);
    }
}

/// CONFIGS.md #7 — every C-locale whitespace prefix, repeated.
#[test]
fn cfg_07_whitespace_prefixes() {
    let mut rng = Rng::new(0x0000_0007_7777_7777);
    for ws in C_SPACES {
        for reps in 1..=8 {
            let v = rng.below(i32::MAX as u64);
            let mut arg = vec![ws; reps];
            arg.extend_from_slice(format!("{v}").as_bytes());
            same_arg(&arg[..]);
        }
    }
    // all six mixed, with and without a sign
    for _ in 0..64 {
        let mut arg: Vec<u8> = Vec::new();
        for _ in 0..1 + rng.below(6) {
            arg.push(*rng.pick(&C_SPACES));
        }
        if rng.below(2) == 0 {
            arg.push(*rng.pick(b"+-"));
        }
        let v = rng.below(2000);
        arg.extend_from_slice(format!("{v}").as_bytes());
        same_arg(&arg[..]);
    }
}

/// CONFIGS.md #8 — explicit '+' sign, values in and out of `int` range.
#[test]
fn cfg_08_explicit_plus() {
    let mut rng = Rng::new(0x0000_0008_8888_8888);
    for _ in 0..256 {
        let v = match rng.below(3) {
            0 => rng.below(i32::MAX as u64),
            1 => rng.below(u64::MAX / 4),
            _ => rng.below(100),
        };
        same_arg_bounded(format!("+{v}").as_bytes(), CAP64);
    }
}

/// CONFIGS.md #9 — explicit '-' sign, magnitudes in and out of `int` range.
#[test]
fn cfg_09_explicit_minus() {
    let mut rng = Rng::new(0x0000_0009_9999_9999);
    for _ in 0..256 {
        let v = match rng.below(3) {
            0 => rng.below(i32::MAX as u64),
            1 => rng.below(u64::MAX / 4),
            _ => rng.below(100),
        };
        same_arg_bounded(format!("-{v}").as_bytes(), CAP64);
    }
}

/// CONFIGS.md #10 — leading zeros (1..40) with optional sign.
#[test]
fn cfg_10_leading_zeros() {
    let mut rng = Rng::new(0x0000_000A_AAAA_AAAA);
    for zeros in 1..=40usize {
        let v = rng.below(100_000);
        let sign: &[u8] = match rng.below(3) {
            0 => b"",
            1 => b"+",
            _ => b"-",
        };
        let mut arg = sign.to_vec();
        arg.extend(std::iter::repeat(b'0').take(zeros));
        arg.extend_from_slice(format!("{v}").as_bytes());
        same_arg_bounded(&arg[..], CAP64);
    }
}

/// CONFIGS.md #11 — trailing garbage: partial parse must match exactly.
#[test]
fn cfg_11_trailing_garbage() {
    let mut rng = Rng::new(0x0000_000B_BBBB_BBBB);
    let suffixes: [&[u8]; 14] = [
        b"abc", b"0x1f", b" 9", b"-3", b"+3", b".5", b"e9", b",", b"\xff", b"\x80", b"\t\t",
        b"\n1", b"%", b"999999999999999999999999x",
    ];
    for suf in suffixes {
        for _ in 0..16 {
            let v = rng.range_i64(-500, 5000);
            let mut arg = format!("{v}").into_bytes();
            arg.extend_from_slice(suf);
            same_arg_bounded(&arg[..], CAP64);
        }
    }
}

/// CONFIGS.md #12 — digit-count sweep 1..10 digits, both signs.
#[test]
fn cfg_12_digit_count_sweep() {
    let mut rng = Rng::new(0x0000_000C_CCCC_CCCC);
    for len in 1..=10usize {
        for _ in 0..8 {
            let mut d = rng.digits(len);
            if len > 1 {
                d[0] = b'1' + rng.below(9) as u8; // no leading zero
            }
            same_arg_bounded(&d[..], CAP32);
            let mut neg = vec![b'-'];
            neg.extend_from_slice(&d);
            same_arg_bounded(&neg[..], CAP32);
            let mut pos = vec![b'+'];
            pos.extend_from_slice(&d);
            same_arg_bounded(&pos[..], CAP32);
        }
    }
}

/// CONFIGS.md #13 — INT_MAX neighbourhood, incl. the `val++` overflow wrap.
#[test]
fn cfg_13_int_max_boundary() {
    for v in 2147483639i64..=2147483647 {
        same_arg_bounded(format!("{v}").as_bytes(), CAP64);
    }
    // and just past it (narrowing kicks in)
    for v in 2147483648i64..=2147483660 {
        same_arg_bounded(format!("{v}").as_bytes(), CAP64);
    }
}

/// CONFIGS.md #14 — INT_MIN neighbourhood (INT_MIN formatting, huge run).
#[test]
fn cfg_14_int_min_boundary() {
    for v in -2147483648i64..=-2147483639 {
        same_arg_bounded(format!("{v}").as_bytes(), CAP64);
    }
    for v in -2147483658i64..=-2147483649 {
        same_arg_bounded(format!("{v}").as_bytes(), CAP64);
    }
}

/// CONFIGS.md #15 — outside `int`, inside `long`: narrowing modulo 2^32.
#[test]
fn cfg_15_long_to_int_truncation() {
    let mut rng = Rng::new(0x0000_000F_1234_5678);
    for _ in 0..512 {
        let mut v = rng.next_u64() as i64;
        if (v as i64) >= i32::MIN as i64 && (v as i64) <= i32::MAX as i64 {
            v = v.wrapping_mul(4_294_967_311); // push it out of int range
        }
        same_arg_bounded(format!("{v}").as_bytes(), CAP32);
    }
}

/// CONFIGS.md #16 — power-of-two offsets around the narrowing boundary.
#[test]
fn cfg_16_power_of_two_offsets() {
    let bases: [i128; 6] = [
        1i128 << 32,
        1i128 << 33,
        -(1i128 << 32),
        1i128 << 31,
        -(1i128 << 31),
        1i128 << 34,
    ];
    for b in bases {
        for k in 0..=12i128 {
            same_arg_bounded(format!("{}", b + k).as_bytes(), CAP32);
            same_arg_bounded(format!("{}", b - k).as_bytes(), CAP32);
        }
    }
}

/// CONFIGS.md #17 — outside `long`: ERANGE clamp to LONG_MAX / LONG_MIN.
#[test]
fn cfg_17_erange_clamp() {
    let mut rng = Rng::new(0x0000_0011_1111_2222);
    for s in [
        &b"9223372036854775806"[..],
        b"9223372036854775807",
        b"9223372036854775808",
        b"9223372036854775809",
        b"-9223372036854775807",
        b"-9223372036854775808",
        b"-9223372036854775809",
        b"-9223372036854775810",
    ] {
        same_arg_bounded(s, CAP32);
    }
    for _ in 0..256 {
        let len = 20 + rng.below(21) as usize; // 20..40 digits => beyond LONG_MAX
        let mut d = rng.digits(len);
        d[0] = b'1' + rng.below(9) as u8;
        same_arg_bounded(&d[..], CAP32);
        let mut neg = vec![b'-'];
        neg.extend_from_slice(&d);
        same_arg_bounded(&neg[..], CAP32);
    }
}

/// CONFIGS.md #18 — very long numerals (100 / 1 000 / 100 000 digits).
#[test]
fn cfg_18_very_long_numerals() {
    let mut rng = Rng::new(0x0000_0012_2222_3333);
    for len in [100usize, 1000, 100_000] {
        for sign in [&b""[..], b"+", b"-"] {
            let mut d = rng.digits(len);
            d[0] = b'1' + rng.below(9) as u8;
            let mut arg = sign.to_vec();
            arg.extend_from_slice(&d);
            same_arg_bounded(&arg[..], CAP32);

            // same, but with a long run of leading zeros before real digits
            let mut arg = sign.to_vec();
            arg.extend(std::iter::repeat(b'0').take(len));
            arg.extend_from_slice(b"123");
            same_arg_bounded(&arg[..], CAP32);
        }
    }
}

/// CONFIGS.md #19 — full lexical cross-product property test.
#[test]
fn cfg_19_random_lexical_cross_product() {
    let mut rng = Rng::new(0x0000_0013_3333_4444);
    let suffixes: [&[u8]; 10] = [
        b"", b"abc", b" ", b"\t9", b"x", b".", b"-1", b"+1", b"\xff", b"e10",
    ];
    for _ in 0..2048 {
        let mut arg: Vec<u8> = Vec::new();
        for _ in 0..rng.below(4) {
            arg.push(*rng.pick(&C_SPACES));
        }
        match rng.below(3) {
            0 => {}
            1 => arg.push(b'+'),
            _ => arg.push(b'-'),
        }
        for _ in 0..rng.below(5) {
            arg.push(b'0');
        }
        let ndigits = 1 + rng.below(25) as usize;
        arg.extend_from_slice(&rng.digits(ndigits));
        let suffix: &[u8] = *rng.pick(&suffixes[..]);
        arg.extend_from_slice(suffix);
        same_arg_bounded(&arg[..], CAP16);
    }
}

/// CONFIGS.md #20 — decade sweep: isolates the `val % 10 == 9` branch for every
/// possible last digit, both signs.
#[test]
fn cfg_20_decade_sweep() {
    let mut rng = Rng::new(0x0000_0014_4444_5555);
    for _ in 0..32 {
        let n = rng.below(214_748_364) as i64;
        for d in 0..=9i64 {
            same_arg_bounded(format!("{}", n * 10 + d).as_bytes(), CAP16);
        }
    }
    for _ in 0..32 {
        let n = rng.below(400) as i64; // keep negative runs short
        for d in 0..=9i64 {
            same_arg(format!("{}", -(n * 10 + d)).as_bytes());
        }
    }
}

/// CONFIGS.md #21 — immediate-break value wearing every lexical decoration.
#[test]
fn cfg_21_decorated_immediate_break() {
    for s in [
        &b"  \t000019abc"[..],
        b"+00000009",
        b"-0000000009",
        b"\n\r+0019 xyz",
        b"\x0b\x0c9",
        b"0000000000000000029\xff",
        b"  +2147483639zzz",
    ] {
        same_arg_bounded(s, CAP64);
    }
}

/// CONFIGS.md #22 — stdout redirected to a regular file.
#[test]
fn cfg_22_stdout_to_file() {
    let mut rng = Rng::new(0x0000_0016_6666_7777);
    for _ in 0..32 {
        let v = rng.range_i64(-3000, 3000);
        assert_same(&Spec::one(format!("{v}").as_bytes()).stdout(StdoutTarget::File));
    }
    for s in [&b"abc"[..], b"", b"9"] {
        assert_same(&Spec::one(s).stdout(StdoutTarget::File));
    }
    // large-but-bounded output into a file
    assert_same(
        &Spec::one(b"-200000")
            .stdout(StdoutTarget::File)
            .cap(4 << 20),
    );
}

/// CONFIGS.md #23 — stdout to /dev/null: only status is observable.
#[test]
fn cfg_23_stdout_devnull() {
    let mut rng = Rng::new(0x0000_0017_7777_8888);
    for _ in 0..24 {
        let v = rng.range_i64(-100_000, 100_000);
        assert_same(&Spec::one(format!("{v}").as_bytes()).stdout(StdoutTarget::DevNull));
    }
    for s in [&b"abc"[..], b"", b"9", b"-1"] {
        assert_same(&Spec::one(s).stdout(StdoutTarget::DevNull));
    }
}

/// CONFIGS.md #24 — fd 1 closed before exec: writes fail, status unchanged.
#[test]
fn cfg_24_stdout_closed() {
    for s in [&b"9"[..], b"5", b"-3", b"0", b"abc", b"", b"12abc"] {
        assert_same(&Spec::one(s).stdout(StdoutTarget::Closed));
    }
    let spec = Spec::new([b"a".to_vec(), b"b".to_vec()]).stdout(StdoutTarget::Closed);
    assert_same(&spec);
}

/// CONFIGS.md #25 — reader closes the pipe mid-stream: SIGPIPE parity.
#[test]
fn cfg_25_sigpipe_parity() {
    for arg in [&b"-2000000000"[..], b"-1000000000", b"2147483648"] {
        let (c_bytes, c_code, c_sig) = run_then_close_pipe(&c_bin(), arg, 8192);
        let (r_bytes, r_code, r_sig) = run_then_close_pipe(&rust_bin(), arg, 8192);
        let n = 8192.min(c_bytes.len()).min(r_bytes.len());
        assert_eq!(&c_bytes[..n], &r_bytes[..n], "prefix differs for {arg:?}");
        assert_eq!(c_sig, r_sig, "signal parity for {arg:?}: C={c_sig:?} R={r_sig:?}");
        assert_eq!(c_code, r_code, "code parity for {arg:?}");
        assert_eq!(c_sig, Some(13), "expected SIGPIPE from C for {arg:?}");
    }
}

/// CONFIGS.md #26 — locale variants must not change a single byte.
#[test]
fn cfg_26_locale_variants() {
    let locales = [
        "C",
        "POSIX",
        "en_US.UTF-8",
        "tr_TR.UTF-8",
        "de_DE.UTF-8",
        "ja_JP.UTF-8",
        "C.UTF-8",
    ];
    let mut rng = Rng::new(0x0000_001A_AAAA_BBBB);
    for loc in locales {
        for _ in 0..8 {
            let v = rng.range_i64(-2000, 2_000_000_000);
            let arg = format!("{v}");
            assert_same(
                &Spec::one(arg.as_bytes())
                    .env("LC_ALL", loc)
                    .env("LANG", loc)
                    .cap(CAP64),
            );
        }
        // thousands-separator temptation: a 10-digit value
        assert_same(
            &Spec::one(b"1234567890")
                .env("LC_ALL", loc)
                .env("LANG", loc)
                .cap(CAP64),
        );
        assert_same(
            &Spec::one(b" \t+000123456789xyz")
                .env("LC_ALL", loc)
                .env("LC_NUMERIC", "de_DE.UTF-8")
                .cap(CAP64),
        );
    }
}

/// CONFIGS.md #27 — environment axis: inherited / cleared / huge.
#[test]
fn cfg_27_environment_variants() {
    let mut rng = Rng::new(0x0000_001B_BBBB_CCCC);
    let junk: String = std::iter::repeat('Z').take(64 * 1024).collect();
    for _ in 0..16 {
        let v = rng.range_i64(-500, 500);
        let arg = format!("{v}");
        assert_same(&Spec::one(arg.as_bytes()));
        assert_same(&Spec::one(arg.as_bytes()).env_clear());
        assert_same(&Spec::one(arg.as_bytes()).env("DRIVER_JUNK", &junk));
        assert_same(
            &Spec::one(arg.as_bytes())
                .env_clear()
                .env("PATH", "/nonexistent"),
        );
    }
    for s in [&b"abc"[..], b""] {
        assert_same(&Spec::one(s).env_clear());
    }
}

/// CONFIGS.md #28 — argv[0] is never read, so any spelling must behave alike.
#[test]
fn cfg_28_argv0_variants() {
    let big = vec![b'n'; 4096];
    let variants: [&[u8]; 4] = [b"", b"weird name", b"\xff\xfe", &big];
    for a0 in variants {
        for arg in [&b"7"[..], b"-2", b"abc", b""] {
            assert_same(&Spec::one(arg).arg0(a0));
        }
    }
}

/// CONFIGS.md #29 — stdin is never read: closed / null / a file with data.
#[test]
fn cfg_29_stdin_unused() {
    for st in [
        StdinTarget::Inherit,
        StdinTarget::Null,
        StdinTarget::Closed,
        StdinTarget::FileWithData,
    ] {
        for arg in [&b"5"[..], b"-3", b"9", b"abc", b""] {
            assert_same(&Spec::one(arg).stdin(st));
        }
    }
}

/// CONFIGS.md #30 — non-UTF-8 operand bytes that still parse.
#[test]
fn cfg_30_non_utf8_operands() {
    let mut rng = Rng::new(0x0000_001D_DDDD_EEEE);
    for s in [&b"5\xff"[..], b"\t-7\x80", b"  +12\xc3\x28", b"9\xfe\xff"] {
        same_arg_bounded(s, CAP16);
    }
    for _ in 0..128 {
        let v = rng.range_i64(-300, 3000);
        let mut arg = format!("{v}").into_bytes();
        let n = 1 + rng.below(3) as usize;
        for _ in 0..n {
            arg.push(0x80 + rng.below(0x80) as u8);
        }
        same_arg_bounded(&arg[..], CAP16);
    }
}

/// CONFIGS.md #31 — long-run tail behaviour: the terminal break at +9 after
/// crossing many decade boundaries (and zero).
#[test]
fn cfg_31_long_run_tail() {
    let mut rng = Rng::new(0x0000_001E_EEEE_FFFF);
    for _ in 0..64 {
        let v = rng.range_i64(-2000, -1);
        let out_c = run(&c_bin(), &Spec::one(format!("{v}").as_bytes()));
        assert!(
            out_c.stdout.ends_with(b"\n7\n8\n9\n"),
            "C run from {v} did not end at 9"
        );
        same_arg(format!("{v}").as_bytes());
    }
    // a longer run, still fully compared
    for v in [-50_000i64, -100_000, -123_457] {
        assert_same(&Spec::one(format!("{v}").as_bytes()).cap(4 << 20));
    }
}

/// CONFIGS.md #32 — ten-digit values straddling the narrowing boundary.
#[test]
fn cfg_32_ten_digit_boundary() {
    for v in 2147483640i64..=2147483660 {
        same_arg_bounded(format!("{v}").as_bytes(), CAP32);
        same_arg_bounded(format!("+{v}").as_bytes(), CAP32);
        same_arg_bounded(format!("-{v}").as_bytes(), CAP32);
    }
    for v in [4294967290i64, 4294967295, 4294967296, 4294967297, 4294967304, 4294967305] {
        same_arg_bounded(format!("{v}").as_bytes(), CAP32);
        same_arg_bounded(format!("-{v}").as_bytes(), CAP32);
    }
}

/// CONFIGS.md #33 — deep stream equality. The rows above bound most long runs to
/// the first 16..64 KiB; this row streams and compares far into the output so a
/// divergence that only appears after millions of lines (digit-width changes,
/// sign flip at zero, post-wrap counting) cannot hide.
#[test]
fn cfg_33_deep_stream_equality() {
    // complete streams (both processes run to their natural break at +9)
    assert_same_streaming(b"-10000000", u64::MAX); // 10M lines, ~88 MB, exact
    assert_same_streaming(b"-1", u64::MAX);
    assert_same_streaming(b"2147483639", u64::MAX);

    // ~2^31-iteration runs: compare the first 256 MiB of the stream, which
    // covers the INT_MAX -> INT_MIN wrap and ~20M subsequent lines
    assert_same_streaming(b"2147483647", 256 << 20);
    assert_same_streaming(b"2147483648", 256 << 20);
    assert_same_streaming(b"-2147483648", 256 << 20);
}

/// CONFIGS.md #34 — stdout is a TTY. glibc line-buffers on a terminal while the
/// Rust port uses a block-buffered writer; the byte stream the terminal receives
/// (including the pty's own \n -> \r\n translation) must still be identical, and
/// the final flush must happen.
#[test]
fn cfg_34_stdout_is_a_tty() {
    for arg in [
        &b"5"[..], b"9", b"0", b"-30", b"abc", b"", b"  +19xyz", b"2147483639",
    ] {
        let (c_out, c_code, c_sig) = run_on_pty(&c_bin(), arg);
        let (r_out, r_code, r_sig) = run_on_pty(&rust_bin(), arg);
        assert_eq!(
            c_out,
            r_out,
            "tty output differs for {arg:?}:\n  C   ={:?}\n  RUST={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        assert_eq!(c_code, r_code, "tty exit code differs for {arg:?}");
        assert_eq!(c_sig, r_sig, "tty signal differs for {arg:?}");
    }
}

/// CONFIGS.md #35 — RLIMIT_FSIZE reached while writing: the kernel raises
/// SIGXFSZ. Neither program checks the return value of printf, so the observable
/// contract is "same terminating signal / exit code, same file prefix".
#[test]
fn cfg_35_fsize_limit_sigxfsz() {
    for (arg, limit) in [
        (&b"5"[..], 4u64),
        (b"-300", 64),
        (b"1234567890", 3),
        (b"9", 1),
        (b"abc", 8),
        (b"", 1),
    ] {
        let (c_data, c_code, c_sig) = run_with_fsize_limit(&c_bin(), arg, limit);
        let (r_data, r_code, r_sig) = run_with_fsize_limit(&rust_bin(), arg, limit);
        assert_eq!(
            c_sig, r_sig,
            "RLIMIT_FSIZE({limit}) signal differs for {arg:?}: C={c_sig:?} RUST={r_sig:?}"
        );
        assert_eq!(
            c_code, r_code,
            "RLIMIT_FSIZE({limit}) exit code differs for {arg:?}"
        );
        assert_eq!(
            c_data,
            r_data,
            "RLIMIT_FSIZE({limit}) file contents differ for {arg:?}:\n  C   ={:?}\n  RUST={:?}",
            String::from_utf8_lossy(&c_data),
            String::from_utf8_lossy(&r_data)
        );
        // non-vacuity: these limits are all smaller than the output, so the
        // kernel must have raised SIGXFSZ (25) and truncated at exactly `limit`
        assert_eq!(c_sig, Some(25), "expected SIGXFSZ from C for {arg:?}");
        assert_eq!(
            c_data.len() as u64, limit,
            "expected the C write to be truncated at the limit for {arg:?}"
        );
    }
}

/// Guard for #34: prove the pty helper actually captures output (otherwise
/// cfg_34 would compare two empty buffers and pass vacuously).
#[test]
fn cfg_34b_pty_helper_is_not_vacuous() {
    let (c_out, code, sig) = run_on_pty(&c_bin(), b"7");
    assert_eq!(
        c_out, b"7\r\n8\r\n9\r\n",
        "pty capture unexpected: {:?}",
        String::from_utf8_lossy(&c_out)
    );
    assert_eq!((code, sig), (Some(0), None));
    let (r_out, _, _) = run_on_pty(&rust_bin(), b"7");
    assert_eq!(r_out, c_out);
    let (c_err, code, _) = run_on_pty(&c_bin(), b"zz");
    assert_eq!(c_err, b"Error: first argument must be an integer!\r\n");
    assert_eq!(code, Some(1));
}

/// CONFIGS.md #36 — the exact `acc*10 + digit` overflow-detection boundary of the
/// parser, where the positive limit (LONG_MAX) and the negative limit (|LONG_MIN|,
/// one larger) differ: a full last-digit sweep across LONG_MAX/LONG_MIN, 19-digit
/// all-nines, and the same values reached through long runs of leading zeros or
/// whitespace.
#[test]
fn cfg_36_strtol_overflow_boundary() {
    for d in b'0'..=b'9' {
        for prefix in ["922337203685477580", "922337203685477581", "922337203685477579"] {
            let pos = format!("{prefix}{}", d as char);
            same_arg_bounded(pos.as_bytes(), CAP32);
            same_arg_bounded(format!("+{pos}").as_bytes(), CAP32);
            same_arg_bounded(format!("-{pos}").as_bytes(), CAP32);
            // reached through leading zeros
            same_arg_bounded(format!("-0000000000{pos}").as_bytes(), CAP32);
            // and through whitespace
            same_arg_bounded(format!("  \t\n{pos}").as_bytes(), CAP32);
            // with trailing garbage after the overflowing numeral
            same_arg_bounded(format!("{pos}abc").as_bytes(), CAP32);
        }
    }
    for s in [
        &b"9999999999999999999"[..],  // 19 nines  > LONG_MAX
        b"-9999999999999999999",
        b"999999999999999999",        // 18 nines  < LONG_MAX
        b"-999999999999999999",
        b"10000000000000000000",      // 20 digits > LONG_MAX
        b"-10000000000000000000",
        b"9223372036854775807",       // LONG_MAX exactly
        b"-9223372036854775808",      // LONG_MIN exactly (must NOT clamp)
    ] {
        same_arg_bounded(s, CAP32);
    }
    // 100 000 leading spaces / zeros in front of a boundary value
    let mut ws = vec![b' '; 100_000];
    ws.extend_from_slice(b"-9223372036854775808");
    same_arg_bounded(&ws[..], CAP32);
    let mut zeros = vec![b'-'];
    zeros.extend(std::iter::repeat(b'0').take(100_000));
    zeros.extend_from_slice(b"9223372036854775809");
    same_arg_bounded(&zeros[..], CAP32);
}
