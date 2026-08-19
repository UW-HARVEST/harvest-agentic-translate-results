//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both artifacts are driven as processes through the only boundary the C target
//! exposes (stdin/stdout/exit status — see `SYMBOLS.md`), and every row is
//! exercised with many randomized inputs from a fixed seed rather than a single
//! hand-picked value.

mod common;

use common::{assert_same, assert_same_prefix, assert_same_with, pair, In, Out, Rng};

/// Samples per randomized row.
const N: usize = 24;

/// Whether a value pair must be compared over an output *prefix* rather than in
/// full, because its stdout is too large to capture.
///
/// Two distinct reasons:
///  * `x>0 && y<0` never terminates in practice — `foo` decrements `y` on every
///    pass where `y != 0`, so a negative `y` runs until it wraps around through
///    `INT_MIN` (~2^32 lines).  Row C15 covers that class.
///  * a *terminating* run still emits on the order of `x + y` lines, so operands
///    near `INT_MAX` produce gigabytes.  Rows C13/C24 cover those.
///
/// A pair that fails the loop guard (`x<=0 && y<=0`) prints nothing at all
/// regardless of magnitude, so it is always safe to compare in full.
fn needs_prefix(x: i64, y: i64) -> bool {
    if !(x > 0 || y > 0) {
        return false; // guard false: empty output
    }
    if y < 0 {
        return true; // signed-overflow wrap: unbounded
    }
    // Bounded, but keep the captured output well under a megabyte.
    x > 100_000 || y > 100_000
}

// ---------------------------------------------------------------- C1 .. C15
// `foo`'s control flow.

/// C1: `x<=0 && y<=0` — the `while (x > 0 || y > 0)` guard is false, so the body
/// never runs and stdout is empty.
#[test]
fn c1_guard_false_no_iterations() {
    let mut rng = Rng::new(0xC1);
    assert_same("c1 (0,0)", &pair(0, 0));
    for i in 0..N {
        let x = rng.range(-4096, 0);
        let y = rng.range(-4096, 0);
        assert_same(&format!("c1 #{i} ({x},{y})"), &pair(x, y));
    }
}

/// C2: `x>0, y==0` — guard true through the first disjunct only; `label1` prints
/// and decrements `x`, then `label2` takes the `continue` on every pass.
#[test]
fn c2_xpos_yzero() {
    let mut rng = Rng::new(0xC2);
    for i in 0..N {
        let x = rng.range(1, 3000);
        assert_same(&format!("c2 #{i} ({x},0)"), &pair(x, 0));
    }
}

/// C3: `x==0, y>0` — guard true through the second disjunct; `label1` is a no-op
/// and the `x<3` back-edge spins the inner loop until `y` hits 0.
#[test]
fn c3_xzero_ypos() {
    let mut rng = Rng::new(0xC3);
    for i in 0..N {
        let y = rng.range(1, 3000);
        assert_same(&format!("c3 #{i} (0,{y})"), &pair(0, y));
    }
}

/// C4: `x<0, y>0` — `x` is never decremented (guarded by `if (x > 0)`) yet keeps
/// the `x<3` back-edge permanently taken.
#[test]
fn c4_xneg_ypos() {
    let mut rng = Rng::new(0xC4);
    for i in 0..N {
        let x = rng.range(-3000, -1);
        let y = rng.range(1, 3000);
        assert_same(&format!("c4 #{i} ({x},{y})"), &pair(x, y));
    }
}

/// C5: `1 <= x < 3` with `y>0` — the back-edge is always taken and both counters
/// drain inside the body.
#[test]
fn c5_x_below_three_ypos() {
    let mut rng = Rng::new(0xC5);
    for i in 0..N {
        let x = rng.range(1, 2);
        let y = rng.range(1, 3000);
        assert_same(&format!("c5 #{i} ({x},{y})"), &pair(x, y));
    }
}

/// C6: `x==3` exactly — the boundary of `if (x < 3)`.  `x` is decremented to 2 at
/// `label1` *before* the test, so the back-edge is taken on the very first pass.
#[test]
fn c6_x_equals_three_boundary() {
    let mut rng = Rng::new(0xC6);
    for y in [0i64, 1, 2, 3, 4, 5, 6, 17] {
        assert_same(&format!("c6 (3,{y})"), &pair(3, y));
        assert_same(&format!("c6 (4,{y})"), &pair(4, y));
    }
    for i in 0..N {
        let y = rng.range(0, 3000);
        assert_same(&format!("c6 #{i} (3,{y})"), &pair(3, y));
    }
}

/// C7: `x==1 && y==4` — the only input that reaches `goto label2`, skipping the
/// `x--` block on the first iteration (the jump is unreachable on later
/// iterations; see the control-flow note in `CONFIGS.md`).
#[test]
fn c7_goto_label2_special_case() {
    assert_same("c7 (1,4)", b"1 4");
    // Same value pair, reached through every layout the parser accepts.
    for layout in [
        &b"1 4"[..],
        b"1\t4",
        b"1\n4",
        b"  +1   +4  ",
        b"01 04",
        b"1 4 9",
        b"1 4junk",
    ] {
        assert_same("c7 layout", layout);
    }
}

/// C8: near misses of C7 must *not* take the jump.
#[test]
fn c8_goto_label2_near_misses() {
    for (x, y) in [
        (1i64, 3i64),
        (1, 5),
        (2, 4),
        (0, 4),
        (1, 0),
        (4, 4),
        (-1, 4),
        (1, -4),
        (2, 5),
        (3, 4),
        (1, 40),
        (11, 4),
    ] {
        let label = format!("c8 ({x},{y})");
        if needs_prefix(x, y) {
            assert_same_prefix(&label, &pair(x, y), 64 * 1024);
        } else {
            assert_same(&label, &pair(x, y));
        }
    }
}

/// C9: `x>3 && y>0` — the first pass falls out of the body to the guard without
/// taking the back-edge, and re-enters until `x` drops below 3.
#[test]
fn c9_x_above_three_ypos() {
    let mut rng = Rng::new(0xC9);
    for i in 0..N {
        let x = rng.range(4, 3000);
        let y = rng.range(1, 3000);
        assert_same(&format!("c9 #{i} ({x},{y})"), &pair(x, y));
    }
}

/// C10: `x>3 && y==0` — repeated guard re-entry with the `continue` path.
#[test]
fn c10_x_above_three_yzero() {
    let mut rng = Rng::new(0xCA);
    for i in 0..N {
        let x = rng.range(4, 3000);
        assert_same(&format!("c10 #{i} ({x},0)"), &pair(x, 0));
    }
}

/// C11: exhaustive sweep of the small-value cross product, which covers every
/// combination of the four `if` branches reachable with bounded output.
#[test]
fn c11_exhaustive_small_grid() {
    for x in -6i64..=12 {
        for y in 0i64..=12 {
            assert_same(&format!("c11 ({x},{y})"), &pair(x, y));
        }
    }
}

/// C12: large bounded workloads — tens of thousands of lines, crossing the stdio
/// buffer boundary many times.
#[test]
fn c12_large_bounded_workloads() {
    let mut rng = Rng::new(0xCC);
    for i in 0..12 {
        let x = rng.range(3000, 9000);
        let y = rng.range(3000, 9000);
        assert_same(&format!("c12 #{i} ({x},{y})"), &pair(x, y));
    }
    assert_same("c12 (9000,0)", &pair(9000, 0));
    assert_same("c12 (0,9000)", &pair(0, 9000));
}

/// C13: `INT_MAX` operands.  The output is ~2^31 lines, so compare a prefix.
#[test]
fn c13_int_max_prefix() {
    let n = 64 * 1024;
    assert_same_prefix("c13 (INT_MAX,0)", b"2147483647 0", n);
    assert_same_prefix("c13 (0,INT_MAX)", b"0 2147483647", n);
    assert_same_prefix("c13 (INT_MAX,INT_MAX)", b"2147483647 2147483647", n);
    assert_same_prefix("c13 (INT_MAX,1)", b"2147483647 1", n);
    assert_same_prefix("c13 (1,INT_MAX)", b"1 2147483647", n);
}

/// C14: `INT_MIN` operands.  `x==INT_MIN` must never print an `"x"` line because
/// the decrement is guarded by `if (x > 0)`.
#[test]
fn c14_int_min_combinations() {
    assert_same("c14 (INT_MIN,0)", b"-2147483648 0");
    assert_same("c14 (INT_MIN,INT_MIN)", b"-2147483648 -2147483648");
    assert_same("c14 (0,INT_MIN)", b"0 -2147483648");
    assert_same("c14 (INT_MIN,1)", b"-2147483648 1");
    assert_same("c14 (INT_MIN,7)", b"-2147483648 7");
    assert_same("c14 (INT_MIN,2000)", b"-2147483648 2000");
    assert_same("c14 (INT_MIN+1,5)", b"-2147483647 5");
    // `x>0 && y==INT_MIN` is the unbounded overflow class.
    assert_same_prefix("c14 (1,INT_MIN)", b"1 -2147483648", 64 * 1024);
}

/// C15: the unbounded class `x>0 && y<0` (signed-overflow wrap at `y--`),
/// compared over a 256 KiB stdout prefix.
#[test]
fn c15_unbounded_prefix_compare() {
    let n = 256 * 1024;
    let mut rng = Rng::new(0xCF);
    assert_same_prefix("c15 (1,-1)", b"1 -1", n);
    assert_same_prefix("c15 (2,-100)", b"2 -100", n);
    assert_same_prefix("c15 (5,-6)", b"5-6", n);
    assert_same_prefix("c15 (4000,-1)", b"4000 -1", n);
    for i in 0..8 {
        let x = rng.range(1, 4000);
        let y = rng.range(-4000, -1);
        assert_same_prefix(&format!("c15 #{i} ({x},{y})"), &pair(x, y), 64 * 1024);
    }
}

// --------------------------------------------------------------- C16 .. C24
// The `scanf("%d %d", &x, &y)` layer.

/// C16: canonical `"<x> <y>"` layout over randomized values.
#[test]
fn c16_canonical_layout() {
    let mut rng = Rng::new(0xD0);
    let mut done = 0;
    while done < 64 {
        let x = rng.range(-4096, 4096);
        let y = rng.range(-4096, 4096);
        if needs_prefix(x, y) {
            continue; // covered by C15
        }
        assert_same(&format!("c16 #{done} ({x},{y})"), &pair(x, y));
        done += 1;
    }
}

/// C17: every whitespace separator `%d` accepts between the two integers.
#[test]
fn c17_separator_variants() {
    let seps: [&[u8]; 10] = [
        b" ", b"  ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r", b"\r\n", b" \t\n\x0b\x0c\r ", b"\n\n\n",
    ];
    let mut rng = Rng::new(0xD1);
    for sep in seps {
        for i in 0..6 {
            let x = rng.range(0, 40);
            let y = rng.range(0, 40);
            let mut input = x.to_string().into_bytes();
            input.extend_from_slice(sep);
            input.extend_from_slice(y.to_string().as_bytes());
            assert_same(&format!("c17 sep={sep:?} #{i} ({x},{y})"), &input);
        }
    }
}

/// C18: leading whitespace before the first integer and trailing whitespace
/// after the second.
#[test]
fn c18_leading_and_trailing_whitespace() {
    let pads: [&[u8]; 6] = [b"", b" ", b"   ", b"\n", b"\t\t", b" \t\n\x0b\x0c\r"];
    let mut rng = Rng::new(0xD2);
    for lead in pads {
        for trail in pads {
            let x = rng.range(0, 30);
            let y = rng.range(0, 30);
            let mut input = lead.to_vec();
            input.extend_from_slice(x.to_string().as_bytes());
            input.push(b' ');
            input.extend_from_slice(y.to_string().as_bytes());
            input.extend_from_slice(trail);
            assert_same(&format!("c18 ({x},{y}) lead={lead:?} trail={trail:?}"), &input);
        }
    }
}

/// C19: explicit `+`/`-` signs in every combination, including signed zero.
#[test]
fn c19_explicit_signs() {
    let mut rng = Rng::new(0xD3);
    for sx in ["", "+", "-"] {
        for sy in ["", "+", "-"] {
            for i in 0..6 {
                let x = rng.range(0, 60);
                let y = rng.range(0, 60);
                let vx: i64 = if sx == "-" { -x } else { x };
                let vy: i64 = if sy == "-" { -y } else { y };
                let input = format!("{sx}{x} {sy}{y}").into_bytes();
                let label = format!("c19 #{i} [{sx}{x} {sy}{y}]");
                if needs_prefix(vx, vy) {
                    assert_same_prefix(&label, &input, 64 * 1024);
                } else {
                    assert_same(&label, &input);
                }
            }
        }
    }
    for input in [&b"+0 -0"[..], b"-0 +0", b"-0 -0", b"+0 +0"] {
        assert_same("c19 signed zero", input);
    }
}

/// C20: no separator at all — the sign of the second integer delimits the first.
#[test]
fn c20_sign_as_separator() {
    // "5-6" parses as x=5, y=-6, which is the unbounded class.
    for input in [&b"5-6"[..], b"12-34", b"1-1", b"2-3"] {
        assert_same_prefix("c20 minus separator", input, 64 * 1024);
    }
    // "+" keeps y positive, so these terminate.
    for input in [&b"5+6"[..], b"12+34", b"0+7", b"3+3"] {
        assert_same("c20 plus separator", input);
    }
    // A negative first operand with a signed second operand.
    assert_same("c20 -5-6", b"-5-6");
    assert_same("c20 -5+6", b"-5+6");
}

/// C21: leading zeros and long digit runs (the `strtol` accumulation path).
#[test]
fn c21_leading_zeros_and_long_runs() {
    assert_same("c21 zeros", b"0 0");
    assert_same("c21 padded", b"0000005 000006");
    assert_same("c21 many zeros", b"000000000000000000005 00000000000000000000007");
    // 40 and 1000 digit runs: both saturate `long` and truncate to `int`.
    let long40 = "1".repeat(40);
    let long1000 = "9".repeat(1000);
    assert_same("c21 40 digits x", format!("{long40} 3").as_bytes());
    // 40 digits saturate to -1, so y<0 with x>0: the unbounded overflow class.
    assert_same_prefix("c21 40 digits y", format!("3 {long40}").as_bytes(), 64 * 1024);
    assert_same("c21 1000 digits x", format!("{long1000} 3").as_bytes());
    assert_same("c21 1000 digits y", format!("0 {long1000}").as_bytes());
    // Long run of zeros before a significant digit.
    let zeros = "0".repeat(500);
    assert_same("c21 500 zeros", format!("{zeros}4 {zeros}2").as_bytes());
}

/// C22: tokens after the second integer are never consumed.
#[test]
fn c22_extra_tokens_ignored() {
    for input in [
        &b"5 6 7 8 9"[..],
        b"5 6junk",
        b"5 6 junk",
        b"2 3\n4 5\n6 7\n",
        b"1 2 -9999999999999999999999",
        b"0 4 the rest is ignored",
    ] {
        assert_same("c22 extra tokens", input);
    }
}

/// C23: only one token — `y` keeps its `int y = 0` default.
#[test]
fn c23_single_token() {
    let mut rng = Rng::new(0xD7);
    for i in 0..N {
        let x = rng.range(-200, 200);
        for suffix in ["", " ", "\n", "\t", " \n "] {
            assert_same(
                &format!("c23 #{i} ({x}) suffix={suffix:?}"),
                format!("{x}{suffix}").as_bytes(),
            );
        }
    }
}

/// C24: boundary magnitudes as *valid* input on both operands.
#[test]
fn c24_magnitude_boundaries_both_operands() {
    let mags = [
        "0",
        "1",
        "2",
        "3",
        "2147483646",
        "2147483647",  // INT_MAX
        "2147483648",  // INT_MAX+1 -> INT_MIN
        "-2147483647",
        "-2147483648", // INT_MIN
        "-2147483649", // INT_MIN-1 -> INT_MAX
        "4294967295",  // UINT_MAX  -> -1
        "4294967296",  // UINT_MAX+1 -> 0
        "9223372036854775807", // LONG_MAX -> -1
        "9223372036854775808", // LONG_MAX+1, saturates -> -1
        "-9223372036854775808", // LONG_MIN -> 0
        "-9223372036854775809", // LONG_MIN-1, saturates -> 0
    ];
    for a in mags {
        for b in mags {
            let input = format!("{a} {b}");
            // Values that land in the unbounded class need prefix comparison;
            // parse the way the C does to decide (saturate to i64, truncate to i32).
            let (x, y) = (as_c_int(a), as_c_int(b));
            let label = format!("c24 [{a} {b}] -> ({x},{y})");
            if needs_prefix(x as i64, y as i64) {
                assert_same_prefix(&label, input.as_bytes(), 64 * 1024);
            } else {
                assert_same(&label, input.as_bytes());
            }
        }
    }
}

/// Mirror of glibc's `%d`: `strtol` saturates at the `long` limits, then the
/// store truncates to `int`.
fn as_c_int(s: &str) -> i32 {
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let mut v: i128 = 0;
    for b in digits.bytes().take_while(|b| b.is_ascii_digit()) {
        if v <= i64::MAX as i128 {
            v = v * 10 + i128::from(b - b'0');
        }
    }
    let signed = if neg { -v } else { v };
    signed.clamp(i64::MIN as i128, i64::MAX as i128) as i64 as i32
}

// --------------------------------------------------------------- C25 .. C28
// The stdio environment.

/// C25: stdin from a regular file must behave exactly like stdin from a pipe.
#[test]
fn c25_stdin_file_vs_pipe() {
    let mut rng = Rng::new(0xD9);
    for i in 0..16 {
        let x = rng.range(-50, 200);
        let y = rng.range(0, 200);
        let input = pair(x, y);
        assert_same_with(
            &format!("c25 pipe #{i} ({x},{y})"),
            In::Bytes(&input),
            Out::Pipe,
            common::DEFAULT_TIMEOUT_SECS,
        );
        assert_same_with(
            &format!("c25 file #{i} ({x},{y})"),
            In::File(&input),
            Out::Pipe,
            common::DEFAULT_TIMEOUT_SECS,
        );
    }
}

/// C26: stdin at immediate EOF.
#[test]
fn c26_stdin_dev_null() {
    assert_same_with(
        "c26 /dev/null",
        In::Path("/dev/null"),
        Out::Pipe,
        common::DEFAULT_TIMEOUT_SECS,
    );
    assert_same_with(
        "c26 empty pipe",
        In::Bytes(b""),
        Out::Pipe,
        common::DEFAULT_TIMEOUT_SECS,
    );
    assert_same_with(
        "c26 empty file",
        In::File(b""),
        Out::Pipe,
        common::DEFAULT_TIMEOUT_SECS,
    );
}

/// C27: stdout to a regular file, a pipe, and `/dev/null` — the bytes written to
/// disk must equal the bytes delivered to a pipe.
#[test]
fn c27_stdout_file_vs_pipe() {
    let mut rng = Rng::new(0xDB);
    for i in 0..12 {
        let x = rng.range(0, 400);
        let y = rng.range(0, 400);
        let input = pair(x, y);
        assert_same_with(
            &format!("c27 pipe #{i} ({x},{y})"),
            In::Bytes(&input),
            Out::Pipe,
            common::DEFAULT_TIMEOUT_SECS,
        );
        assert_same_with(
            &format!("c27 file #{i} ({x},{y})"),
            In::Bytes(&input),
            Out::File,
            common::DEFAULT_TIMEOUT_SECS,
        );
        assert_same_with(
            &format!("c27 devnull #{i} ({x},{y})"),
            In::Bytes(&input),
            Out::Path("/dev/null"),
            common::DEFAULT_TIMEOUT_SECS,
        );

        // Cross-check the two capture routes against each other.
        let f = common::run_with(
            &common::c_bin(),
            In::Bytes(&input),
            Out::File,
            common::DEFAULT_TIMEOUT_SECS,
        );
        let p = common::run_with(
            &common::rust_bin(),
            In::Bytes(&input),
            Out::Pipe,
            common::DEFAULT_TIMEOUT_SECS,
        );
        assert_eq!(
            f.stdout, p.stdout,
            "c27 #{i} ({x},{y}): C-to-file bytes differ from Rust-to-pipe bytes"
        );
    }
}

/// C28: a large whitespace prefix, so the whitespace skip inside `%d` spans
/// several stdio buffer refills before any digit appears.
#[test]
fn c28_large_whitespace_prefix() {
    for pad_len in [4095usize, 4096, 4097, 8192, 64 * 1024] {
        for pad_byte in [b' ', b'\n', b'\t'] {
            let mut input = vec![pad_byte; pad_len];
            input.extend_from_slice(b"7 5");
            assert_same(&format!("c28 pad={pad_len} byte={pad_byte}"), &input);
        }
    }
    // A long whitespace run *between* the two integers as well.
    let mut input = vec![b' '; 70_000];
    input.extend_from_slice(b"4");
    input.extend(std::iter::repeat(b'\n').take(70_000));
    input.extend_from_slice(b"6");
    input.extend(std::iter::repeat(b'\t').take(70_000));
    assert_same("c28 long interior run", &input);
}

/// C29: locale and environment invariance.  The C never calls `setlocale`, so it
/// stays in the "C" locale: `%d` accepts only ASCII digits, `isspace` only the
/// six ASCII space characters, and no thousands separator is ever recognised —
/// regardless of `LC_ALL`/`LANG`.
#[test]
fn c29_locale_and_env_invariance() {
    let envs: [&[(&str, &str)]; 6] = [
        &[],
        &[("LC_ALL", "C"), ("LANG", "C")],
        &[("LC_ALL", "POSIX"), ("LANG", "POSIX")],
        &[("LC_ALL", "en_US.UTF-8"), ("LANG", "en_US.UTF-8")],
        &[("LC_ALL", "de_DE.UTF-8"), ("LANG", "de_DE.UTF-8")],
        &[("LC_NUMERIC", "de_DE.UTF-8"), ("LC_CTYPE", "tr_TR.UTF-8")],
    ];
    let inputs: [&[u8]; 8] = [
        b"3 2",
        b"1,000 5",
        b"1.000 5",
        b"  12  34",
        "١٢٣ 4".as_bytes(),
        "５ ６".as_bytes(),
        b"\xc2\xa0 5 6",
        b"0 7",
    ];
    let baseline_bin = common::c_bin();
    for input in inputs {
        let baseline = common::run(&baseline_bin, input);
        for env in envs {
            let c = common::run_with_env(&baseline_bin, env, input, common::DEFAULT_TIMEOUT_SECS);
            let r = common::run_with_env(
                &common::rust_bin(),
                env,
                input,
                common::DEFAULT_TIMEOUT_SECS,
            );
            assert_eq!(
                c.stdout, baseline.stdout,
                "c29: the C reference changed behaviour under {env:?} for {:?}",
                String::from_utf8_lossy(input)
            );
            assert_eq!(
                c.stdout,
                r.stdout,
                "c29: stdout differs under {env:?} for {:?}",
                String::from_utf8_lossy(input)
            );
            assert_eq!(c.stderr, r.stderr, "c29: stderr differs under {env:?}");
            assert_eq!(c.code, r.code, "c29: exit status differs under {env:?}");
        }
    }
}
