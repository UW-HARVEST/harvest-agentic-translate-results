// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Each test drives BOTH `.so`s through their
// exported `sieve` symbol (via libloading) and compares the stdout bytes.
// Randomized rows use the fixed SplitMix64 seed in `common::SEED` so failures
// reproduce exactly.

mod common;
use common::*;

/// Row 1 — val == 9: smallest immediate-break value.
#[test]
fn cfg_row01_nine_exactly() {
    assert_match(9);
    let c = run(Impl::C, 9);
    assert_eq!(c, b"9\n".to_vec(), "C reference shape for sieve(9)");
}

/// Row 2 — positive, ends in 9, small multi-digit (1 iteration).
#[test]
fn cfg_row02_positive_ends_in_nine_small() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut vals = Vec::new();
    for _ in 0..200 {
        let v = rng.range(1, 99) * 10 + 9; // 19 .. 999
        vals.push(v as i32);
    }
    assert_match_all("row02", &vals);
}

/// Row 3 — positive, ends in 9, large (7..10 digit field width).
#[test]
fn cfg_row03_positive_ends_in_nine_large() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut vals = Vec::new();
    for _ in 0..200 {
        let v = rng.range(100_000, 99_999_999) * 10 + 9; // 1e6 .. ~1e9
        vals.push(v as i32);
    }
    assert_match_all("row03", &vals);
}

/// Row 4 — every single-digit non-9 start, exhaustively.
#[test]
fn cfg_row04_single_digit_exhaustive() {
    let vals: Vec<i32> = (0..=8).collect();
    assert_match_all("row04", &vals);
}

/// Row 5 — positive, does not end in 9, small (2..10 iterations).
#[test]
fn cfg_row05_positive_small_multi_iteration() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut vals = Vec::new();
    while vals.len() < 300 {
        let v = rng.range(10, 9999) as i32;
        if v % 10 != 9 {
            vals.push(v);
        }
    }
    assert_match_all("row05", &vals);
}

/// Row 6 — positive, does not end in 9, large (9..10 digits, near INT_MAX but
/// still terminating).
#[test]
fn cfg_row06_positive_large_multi_iteration() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut vals = Vec::new();
    while vals.len() < 300 {
        let v = rng.range(1_000_000, 2_000_000_000) as i32;
        if v % 10 != 9 && line_count(v).is_some() {
            vals.push(v);
        }
    }
    assert_match_all("row06", &vals);
}

/// Row 7 — runs that change decimal field width mid-loop: 10^k - 2 counts
/// 8,9 / 98,99 / 998,999 ... crossing the digit boundary.
#[test]
fn cfg_row07_digit_width_change_during_run() {
    let mut vals = Vec::new();
    let mut p: i64 = 10;
    while p <= 1_000_000_000 {
        vals.push((p - 2) as i32);
        p *= 10;
    }
    // also the plain 10^k values themselves
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        vals.push(p as i32);
        p *= 10;
    }
    assert_match_all("row07", &vals);
}

/// Row 8 — val == 0.
#[test]
fn cfg_row08_zero() {
    assert_match(0);
    assert_eq!(run(Impl::C, 0), b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n".to_vec());
}

/// Row 9 — negative values whose last digit is 9 (C's `%` yields -9, so they do
/// NOT break early).
#[test]
fn cfg_row09_negative_ends_in_nine() {
    let mut rng = Rng::new(SEED ^ 9);
    let mut vals = vec![-9, -19, -29, -99, -109, -999, -9999];
    for _ in 0..200 {
        let v = -(rng.range(0, 999) * 10 + 9); // -9 .. -9999
        vals.push(v as i32);
    }
    assert_match_all("row09", &vals);
}

/// Row 10 — small negatives; output crosses the sign transition through 0.
#[test]
fn cfg_row10_small_negatives() {
    let mut rng = Rng::new(SEED ^ 10);
    let mut vals = Vec::new();
    for _ in 0..300 {
        vals.push(rng.range(-999, -1) as i32);
    }
    assert_match_all("row10", &vals);
}

/// Row 11 — large negatives: 50k..200k lines, > 1 MiB of stdout, crossing
/// printf's internal buffer hundreds of times.
#[test]
fn cfg_row11_large_negatives_many_lines() {
    let mut rng = Rng::new(SEED ^ 11);
    let mut vals = Vec::new();
    for _ in 0..20 {
        vals.push(rng.range(-200_000, -50_000) as i32);
    }
    for &v in &vals {
        let c = run(Impl::C, v);
        let r = run(Impl::Rust, v);
        assert_same_bytes(&format!("[row11] sieve({v})"), &c, &r);
        // sanity: the run really is large
        assert!(c.len() > 300_000, "expected a large output for {v}");
    }
}

/// Row 12 — INT_MAX - 8 == 2147483639: the largest value that terminates.
#[test]
fn cfg_row12_last_safe_value() {
    let v = i32::MAX - 8;
    assert_eq!(v, 2_147_483_639);
    assert_match(v);
    assert_eq!(run(Impl::C, v), b"2147483639\n".to_vec());
}

/// Row 13 — the 8-value signed-overflow window [INT_MAX-7, INT_MAX].
/// Output is unbounded, so compare a 256 KiB prefix from a forked child.
#[test]
fn cfg_row13_overflow_window() {
    const LIMIT: usize = 256 * 1024;
    for v in (i32::MAX - 7)..=i32::MAX {
        let c = capture_prefix(Impl::C, v, LIMIT);
        let r = capture_prefix(Impl::Rust, v, LIMIT);
        assert_eq!(
            c.prefix.len(),
            LIMIT,
            "[row13] C produced only {} bytes for sieve({v}); the child may have \
             stalled, so the comparison would be vacuous",
            c.prefix.len()
        );
        assert_eq!(r.prefix.len(), LIMIT, "[row13] Rust prefix short for {v}");
        assert_same_bytes(&format!("[row13] sieve({v}) prefix"), &c.prefix, &r.prefix);
        assert_eq!(
            c.finished, r.finished,
            "[row13] termination behaviour differs for sieve({v})"
        );
    }
}

/// Row 14 — INT_MIN .. INT_MIN+3: unbounded upward count, 256 KiB prefix.
#[test]
fn cfg_row14_int_min_window() {
    const LIMIT: usize = 256 * 1024;
    for k in 0..4 {
        let v = i32::MIN + k;
        let c = capture_prefix(Impl::C, v, LIMIT);
        let r = capture_prefix(Impl::Rust, v, LIMIT);
        assert_eq!(c.prefix.len(), LIMIT, "[row14] C prefix short for {v}");
        assert_eq!(r.prefix.len(), LIMIT, "[row14] Rust prefix short for {v}");
        assert_same_bytes(&format!("[row14] sieve({v}) prefix"), &c.prefix, &r.prefix);
    }
    // The very first line must be the full 11-character INT_MIN literal.
    let c = capture_prefix(Impl::C, i32::MIN, 4096);
    assert!(
        c.prefix.starts_with(b"-2147483648\n"),
        "unexpected C first line: {:?}",
        String::from_utf8_lossy(&c.prefix[..20.min(c.prefix.len())])
    );
}

/// Row 15 — uniformly random int32 over the whole domain, restricted to the
/// terminating classes so the output stays bounded.
#[test]
fn cfg_row15_random_full_domain() {
    let mut rng = Rng::new(SEED ^ 15);
    let mut vals = Vec::new();
    while vals.len() < 500 {
        let v = rng.next_i32();
        match line_count(v) {
            // keep runs bounded: skip the unbounded overflow window (row 13)
            // and the multi-billion-line deep negatives (row 14 covers those
            // via prefix comparison).
            Some(n) if n <= 100_000 => vals.push(v),
            _ => {
                // fold huge/unbounded draws into a comparable bounded value so
                // no draw is wasted
                let folded = (v % 100_000) as i32;
                if line_count(folded).map(|n| n <= 100_000).unwrap_or(false) {
                    vals.push(folded);
                }
            }
        }
    }
    assert_match_all("row15", &vals);
}

/// Row 16 — many back-to-back calls on the same loaded handle: the library is
/// stateless, so N calls must equal the concatenation of N single calls.
#[test]
fn cfg_row16_many_sequential_calls() {
    let mut rng = Rng::new(SEED ^ 16);
    for rep in 0..10 {
        let mut vals = Vec::new();
        while vals.len() < 50 {
            let v = rng.range(-500, 5000) as i32;
            vals.push(v);
        }
        let c = run_seq(Impl::C, &vals);
        let r = run_seq(Impl::Rust, &vals);
        assert_same_bytes(&format!("[row16 rep{rep}] batch of 50"), &c, &r);

        // and the batch must equal the concatenation of the individual runs
        let mut concat = Vec::new();
        for &v in &vals {
            concat.extend_from_slice(&run(Impl::Rust, v));
        }
        assert_same_bytes(
            &format!("[row16 rep{rep}] Rust batch vs concatenated singles"),
            &c,
            &concat,
        );
    }
}

/// Row 17 — interleaved C and Rust calls sharing one process `FILE *stdout`:
/// neither library may perturb the buffer state the other relies on.
#[test]
fn cfg_row17_interleaved_c_and_rust() {
    let mut rng = Rng::new(SEED ^ 17);
    let vals: Vec<i32> = (0..40).map(|_| rng.range(-300, 3000) as i32).collect();
    let l = libs();

    let interleaved = capture("interleaved", || {
        for &v in &vals {
            l.call(Impl::C, v);
            l.call(Impl::Rust, v);
        }
    });
    // Every value's output must appear exactly twice, back to back.
    let mut expected = Vec::new();
    for &v in &vals {
        let one = run(Impl::C, v);
        expected.extend_from_slice(&one);
        expected.extend_from_slice(&one);
    }
    assert_same_bytes("[row17] interleaved C/Rust", &expected, &interleaved);
}

/// Row 18 — stdout is a pipe rather than a regular file (different buffering).
#[test]
fn cfg_row18_stdout_is_a_pipe() {
    let mut rng = Rng::new(SEED ^ 18);
    let mut vals: Vec<i32> = vec![9, 0, -1, -9, 12345];
    while vals.len() < 30 {
        vals.push(rng.range(-2000, 20000) as i32);
    }
    for v in vals {
        let c = run_through_pipe(Impl::C, v);
        let r = run_through_pipe(Impl::Rust, v);
        assert_same_bytes(&format!("[row18] sieve({v}) via pipe"), &c, &r);
        // the pipe result must also agree with the regular-file capture
        let f = run(Impl::C, v);
        assert_same_bytes(&format!("[row18] sieve({v}) pipe vs file"), &f, &r);
    }
}

/// Row 19 — stdout closed: every printf fails, C ignores the return value, so
/// the loop must still terminate normally.
#[test]
fn cfg_row19_stdout_closed_still_terminates() {
    for v in [9i32, 0, -5, 12345, 2_147_483_639] {
        let c = run_with_closed_stdout(Impl::C, v);
        let r = run_with_closed_stdout(Impl::Rust, v);
        assert!(!c.timed_out, "[row19] C hung for sieve({v})");
        assert_eq!(
            c,
            ChildOutcome {
                exit_code: Some(7),
                signal: None,
                timed_out: false
            },
            "[row19] C did not return normally for sieve({v})"
        );
        assert_eq!(c, r, "[row19] C and Rust differ for sieve({v}) with fd1 closed");
    }
}

/// Row 20 — the pruned A1 x A2 cross product: every last digit, both signs.
#[test]
fn cfg_row20_digit_by_sign_cross_product() {
    let mut rng = Rng::new(SEED ^ 20);
    for digit in 0..10i64 {
        for sign in [1i64, -1] {
            let mut vals = Vec::new();
            for _ in 0..25 {
                let mag = rng.range(0, 900) * 10 + digit;
                let v = sign * mag;
                vals.push(v as i32);
            }
            assert_match_all(&format!("row20 d{digit} s{sign}"), &vals);
        }
    }
}

/// Row 21 (exhaustive cross-check) — EVERY value in a dense band around zero,
/// one by one. This leaves no gap in the sign/last-digit/carry space for small
/// magnitudes: 6001 consecutive inputs, ~9M output lines total.
#[test]
fn cfg_row21_exhaustive_dense_band() {
    let vals: Vec<i32> = (-3000..=3000).collect();
    assert_match_all("row21-exhaustive", &vals);
}

/// Row 22 (exhaustive cross-check) — every value in a dense band at the top of
/// the terminating range, i.e. right up to the overflow boundary.
#[test]
fn cfg_row22_exhaustive_band_below_int_max() {
    let vals: Vec<i32> = ((i32::MAX - 2008)..=(i32::MAX - 8)).collect();
    assert_match_all("row22-exhaustive", &vals);
}
