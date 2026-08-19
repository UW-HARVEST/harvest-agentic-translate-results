//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are exercised exclusively through their shared objects
//! (`dlopen` + `dlsym` via `libloading`) or through their linked executables.

#[macro_use]
mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1..15 — the low-level `driver(int)` entry point, called through the `.so`
// ---------------------------------------------------------------------------

/// Row 1: `x = 0` → the loop guard rejects immediately, empty output.
fn cfg_01_driver_zero() {
    assert_driver_eq(&[0]);
}

/// Row 2: `x = 1` → exactly one line.
fn cfg_02_driver_one() {
    assert_driver_eq(&[1]);
}

/// Row 3: `x = 2..=9` exhaustively; `j` crosses ten at `i == 5`.
fn cfg_03_driver_single_digit() {
    for x in 2..=9 {
        assert_driver_eq(&[x]);
    }
}

/// Row 4: `x = 10` → `i` and `j` both reach two digits.
fn cfg_04_driver_ten() {
    assert_driver_eq(&[10]);
    assert_driver_eq(&[11]);
}

/// Row 5: `x = 51` → `j` crosses 100 while `i` is still two digits.
fn cfg_05_driver_j_crosses_100() {
    for x in [49, 50, 51, 52] {
        assert_driver_eq(&[x]);
    }
}

/// Row 6: `i` crosses 100.
fn cfg_06_driver_i_crosses_100() {
    for x in [99, 100, 101] {
        assert_driver_eq(&[x]);
    }
}

/// Row 7: `j` crosses 1000 (`i == 500`) and `i` crosses 1000.
fn cfg_07_driver_crosses_1000() {
    for x in [499, 500, 501, 999, 1000, 1001] {
        assert_driver_eq(&[x]);
    }
}

/// Row 8: every `x` whose output length straddles the 4096-byte boundary.
fn cfg_08_driver_straddles_4096() {
    let xs: Vec<i32> = (1..4000)
        .filter(|&x| (4000..=4200).contains(&driver_output_len(x)))
        .collect();
    assert!(!xs.is_empty(), "no x straddles 4096");
    for x in xs {
        assert_driver_eq(&[x]);
    }
}

/// Row 9: every `x` whose output length straddles the 8192-byte boundary.
fn cfg_09_driver_straddles_8192() {
    let xs: Vec<i32> = (1..8000)
        .filter(|&x| (8100..=8300).contains(&driver_output_len(x)))
        .collect();
    assert!(!xs.is_empty(), "no x straddles 8192");
    for x in xs {
        assert_driver_eq(&[x]);
    }
}

/// Row 10: `j` crosses 10^4, `i` crosses 10^4, output far beyond both buffers.
fn cfg_10_driver_crosses_10k() {
    for x in [4999, 5000, 5001, 9999, 10000, 10001] {
        assert_driver_eq(&[x]);
    }
}

/// Row 11: `j` crosses 10^5.
fn cfg_11_driver_crosses_100k() {
    for x in [49999, 50000, 50001, 100000] {
        assert_driver_eq(&[x]);
    }
}

/// Row 12: randomized positive `x`, fixed seed.
fn cfg_12_driver_random_positive() {
    let mut rng = Rng::new(0x5eed_0012);
    for _ in 0..200 {
        let x = rng.range_i64(0, 3000) as i32;
        assert_driver_eq(&[x]);
    }
    for _ in 0..20 {
        let x = rng.range_i64(3000, 200_000) as i32;
        assert_driver_eq(&[x]);
    }
}

/// Row 13: randomized non-positive `x`, fixed seed.
fn cfg_13_driver_random_non_positive() {
    let mut rng = Rng::new(0x5eed_0013);
    let mut batch = Vec::new();
    for _ in 0..200 {
        batch.push(rng.range_i64(i32::MIN as i64, 0) as i32);
    }
    // Individually …
    for &x in &batch {
        assert_driver_eq(&[x]);
    }
    // … and all in a row in one process.
    assert_driver_eq(&batch);
}

/// Row 14: stdout is a pipe instead of a regular file.
fn cfg_14_driver_stdout_pipe() {
    for x in [0, 1, 7, 1000] {
        assert_driver_eq_pipe(&[x]);
    }
    assert_driver_eq_pipe(&[0, 1, 2, -5, 1000]);
}

/// Row 15: 50 consecutive `driver` calls in one process; the concatenated
/// output must match byte for byte (buffer / lock state reuse).
fn cfg_15_driver_many_calls_one_process() {
    let mut rng = Rng::new(0x5eed_0015);
    let mut xs = Vec::new();
    for k in 0..50 {
        if k % 7 == 0 {
            xs.push(0);
        } else if k % 11 == 0 {
            xs.push(rng.range_i64(i32::MIN as i64, -1) as i32);
        } else {
            xs.push(rng.range_i64(1, 400) as i32);
        }
    }
    assert_driver_eq(&xs);
}

// ---------------------------------------------------------------------------
// Row 16..28 — the `main` entry point, called through the `.so`
// ---------------------------------------------------------------------------

/// Row 16: bare digits, EOF straight after them.
fn cfg_16_main_no_trailing_newline() {
    for s in ["0", "1", "7", "10", "51", "100", "1000"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 17: digits followed by a newline.
fn cfg_17_main_trailing_newline() {
    for s in ["0\n", "1\n", "7\n", "123\n", "2000\n"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 18: each individual leading whitespace byte `%d` must skip.
fn cfg_18_main_each_leading_space() {
    for ws in [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'] {
        let mut v = vec![ws];
        v.extend_from_slice(b"13\n");
        assert_main_so_eq(&v, StdinKind::File);
    }
}

/// Row 19: randomized mixtures of several leading whitespace bytes.
fn cfg_19_main_mixed_leading_space() {
    let ws = [b' ', b'\t', b'\n', 0x0bu8, 0x0c, b'\r'];
    let mut rng = Rng::new(0x5eed_0019);
    for _ in 0..12 {
        let n = rng.range_usize(2, 9);
        let mut v = Vec::new();
        for _ in 0..n {
            v.push(*rng.pick(&ws));
        }
        v.extend_from_slice(format!("{}\n", rng.range_i64(0, 300)).as_bytes());
        assert_main_so_eq(&v, StdinKind::File);
    }
}

/// Row 20: explicit `+` sign.
fn cfg_20_main_plus_sign() {
    for s in ["+0", "+1\n", "+7", "+123\n", "+02000\n"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 21: explicit `-` sign → non-positive `x` → empty output.
fn cfg_21_main_minus_sign() {
    for s in ["-0", "-1\n", "-7", "-2147483648", "-2147483647\n"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 22: all three spellings of zero.
fn cfg_22_main_zero_spellings() {
    for s in ["0", "-0", "+0", "0\n", "-0\n", "+0\n"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 23: leading zeros (1..40 of them).
fn cfg_23_main_leading_zeros() {
    let mut rng = Rng::new(0x5eed_0023);
    for n in [1usize, 2, 5, 18, 19, 20, 40] {
        let mut s = String::new();
        if rng.next_u32() % 2 == 0 {
            s.push('+');
        }
        for _ in 0..n {
            s.push('0');
        }
        s.push_str(&format!("{}\n", rng.range_i64(0, 500)));
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 24: digits immediately followed by non-digit garbage.
fn cfg_24_main_trailing_garbage() {
    for s in ["5abc", "5.9\n", "5-3", "12e4\n", "3)", "9\0junk"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 25: a second number after the first — only the first is converted.
fn cfg_25_main_second_number_ignored() {
    for s in ["5 9", "5\n9\n", "  12\t34\n", "7 -8 9\n", "3\n\n\n1000\n"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::File);
    }
}

/// Row 26: value beyond `INT_MAX` but inside `long`, truncating to a small
/// positive `int` — the output is short but the parse path is the wide one.
fn cfg_26_main_int_truncation_small_positive() {
    // 2^32 + k  ->  k
    for k in [1i64, 2, 5, 17, 300] {
        let v = 4_294_967_296i64 + k;
        assert_main_so_eq(format!("{v}\n").as_bytes(), StdinKind::File);
    }
    // 2^33 + k  ->  k
    for k in [1i64, 9, 250] {
        let v = 8_589_934_592i64 + k;
        assert_main_so_eq(format!("{v}\n").as_bytes(), StdinKind::File);
    }
    // -(2^32 - k) -> k
    for k in [1i64, 4, 99] {
        let v = -(4_294_967_296i64 - k);
        assert_main_so_eq(format!("{v}\n").as_bytes(), StdinKind::File);
    }
}

/// Row 27: randomized decimal strings with randomized whitespace, sign and
/// zero padding, fixed seed.
fn cfg_27_main_randomized() {
    let ws = [b' ', b'\t', b'\n', 0x0bu8, 0x0c, b'\r'];
    let tails: [&[u8]; 6] = [b"", b"\n", b" ", b"xyz", b" 42\n", b"\n\n"];
    let mut rng = Rng::new(0x5eed_0027);
    for _ in 0..60 {
        let mut v = Vec::new();
        for _ in 0..rng.range_usize(0, 3) {
            v.push(*rng.pick(&ws));
        }
        let neg = rng.next_u32() % 4 == 0;
        match rng.next_u32() % 3 {
            0 => v.push(if neg { b'-' } else { b'+' }),
            1 if neg => v.push(b'-'),
            _ => {}
        }
        for _ in 0..rng.range_usize(0, 3) {
            v.push(b'0');
        }
        let n = rng.range_i64(0, 20_000);
        v.extend_from_slice(n.to_string().as_bytes());
        v.extend_from_slice(tails[rng.range_usize(0, tails.len() - 1)]);
        assert_main_so_eq(&v, StdinKind::File);
    }
}

/// Row 28: stdin is a pipe rather than a regular file.
fn cfg_28_main_stdin_pipe() {
    for s in ["7", "7\n", "5 9", "  -3\n", "0", "1000\n"] {
        assert_main_so_eq(s.as_bytes(), StdinKind::Pipe);
    }
}

// ---------------------------------------------------------------------------
// Row 29..32 — the linked executables, end to end
// ---------------------------------------------------------------------------

/// Row 29: end-to-end with pipes on both ends, randomized inputs.
fn cfg_29_exe_pipes_randomized() {
    let ws = [b' ', b'\t', b'\n', 0x0bu8, 0x0c, b'\r'];
    let tails: [&[u8]; 6] = [b"", b"\n", b" ", b"xyz", b" 42\n", b"\n\n"];
    let mut rng = Rng::new(0x5eed_0029);
    for _ in 0..200 {
        let mut v = Vec::new();
        for _ in 0..rng.range_usize(0, 4) {
            v.push(*rng.pick(&ws));
        }
        if rng.next_u32() % 3 == 0 {
            v.push(if rng.next_u32() % 2 == 0 { b'+' } else { b'-' });
        }
        for _ in 0..rng.range_usize(0, 5) {
            v.push(b'0');
        }
        let n = rng.range_i64(0, 8_000);
        v.extend_from_slice(n.to_string().as_bytes());
        v.extend_from_slice(tails[rng.range_usize(0, tails.len() - 1)]);
        assert_exe_eq(&v, ExeIo::Pipes);
    }
}

/// Row 30: end-to-end with regular files on both ends.
fn cfg_30_exe_files() {
    let mut rng = Rng::new(0x5eed_0030);
    for s in ["", "\n", "0", "1", "7\n", "  +51\t", "-9\n", "5 9\n", "abc"] {
        assert_exe_eq(s.as_bytes(), ExeIo::Files);
    }
    for _ in 0..40 {
        let n = rng.range_i64(0, 5_000);
        assert_exe_eq(format!("{n}\n").as_bytes(), ExeIo::Files);
    }
}

/// Row 31: exit status is 0 for every valid input.
fn cfg_31_exe_exit_status() {
    for s in ["", " ", "0", "1", "abc", "-5", "+5\n", "99999999999999999999\n"] {
        let c = run_exe(&c_exe(), s.as_bytes(), ExeIo::Pipes);
        let r = run_exe(&rust_exe(), s.as_bytes(), ExeIo::Pipes);
        assert_eq!(c.code, Some(0), "C exit status for {s:?}");
        assert_eq!(r.code, c.code, "exit status mismatch for {s:?}");
        assert_eq!(r.signal, c.signal, "signal mismatch for {s:?}");
    }
}

/// Row 32: large output through a pipe (≈2.3 MB, many `write(2)`s).
fn cfg_32_exe_large_output_pipe() {
    assert_exe_eq(b"200000\n", ExeIo::Pipes);
    assert_exe_eq(b"200000\n", ExeIo::Files);
}

// ---------------------------------------------------------------------------
// Entry point (this target uses `harness = false`; see common::run_cases)
// ---------------------------------------------------------------------------

fn main() {
    common::run_cases(&[
        case!(cfg_01_driver_zero),
        case!(cfg_02_driver_one),
        case!(cfg_03_driver_single_digit),
        case!(cfg_04_driver_ten),
        case!(cfg_05_driver_j_crosses_100),
        case!(cfg_06_driver_i_crosses_100),
        case!(cfg_07_driver_crosses_1000),
        case!(cfg_08_driver_straddles_4096),
        case!(cfg_09_driver_straddles_8192),
        case!(cfg_10_driver_crosses_10k),
        case!(cfg_11_driver_crosses_100k),
        case!(cfg_12_driver_random_positive),
        case!(cfg_13_driver_random_non_positive),
        case!(cfg_14_driver_stdout_pipe),
        case!(cfg_15_driver_many_calls_one_process),
        case!(cfg_16_main_no_trailing_newline),
        case!(cfg_17_main_trailing_newline),
        case!(cfg_18_main_each_leading_space),
        case!(cfg_19_main_mixed_leading_space),
        case!(cfg_20_main_plus_sign),
        case!(cfg_21_main_minus_sign),
        case!(cfg_22_main_zero_spellings),
        case!(cfg_23_main_leading_zeros),
        case!(cfg_24_main_trailing_garbage),
        case!(cfg_25_main_second_number_ignored),
        case!(cfg_26_main_int_truncation_small_positive),
        case!(cfg_27_main_randomized),
        case!(cfg_28_main_stdin_pipe),
        case!(cfg_29_exe_pipes_randomized),
        case!(cfg_30_exe_files),
        case!(cfg_31_exe_exit_status),
        case!(cfg_32_exe_large_output_pipe),
    ]);
}
