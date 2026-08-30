//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH `libdriver.so` builds via `libloading` and compares the
//! bytes each writes to `stdout`. The Rust crate is never linked or called
//! directly.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1-4 — the four sign quadrants, |x| > |y|, inexact
// ---------------------------------------------------------------------------

/// Builds `n` randomized pairs in a given quadrant with `|x| > |y|` and
/// `x % y != 0`.
fn quadrant_pairs(rng: &mut Rng, n: usize, x_pos: bool, y_pos: bool) -> Vec<(i32, i32)> {
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        // Keep |y| modest so |x| > |y| is easy to satisfy across the range.
        let ay = rng.range_i32(2, 1 << 20);
        let ax = rng.range_i32(ay + 1, i32::MAX);
        if ax % ay == 0 {
            continue; // row 5 covers the exact case
        }
        let x = if x_pos { ax } else { -ax };
        let y = if y_pos { ay } else { -ay };
        out.push((x, y));
    }
    out
}

#[test]
fn cfg_row01_pos_pos_inexact() {
    let mut rng = Rng::new(SEED ^ 1);
    let pairs = quadrant_pairs(&mut rng, 2000, true, true);
    assert_pairs_match_and_nonempty("row01", &pairs);
}

#[test]
fn cfg_row02_neg_pos_inexact() {
    let mut rng = Rng::new(SEED ^ 2);
    let pairs = quadrant_pairs(&mut rng, 2000, false, true);
    assert_pairs_match_and_nonempty("row02", &pairs);
}

#[test]
fn cfg_row03_pos_neg_inexact() {
    let mut rng = Rng::new(SEED ^ 3);
    let pairs = quadrant_pairs(&mut rng, 2000, true, false);
    assert_pairs_match_and_nonempty("row03", &pairs);
}

#[test]
fn cfg_row04_neg_neg_inexact() {
    let mut rng = Rng::new(SEED ^ 4);
    let pairs = quadrant_pairs(&mut rng, 2000, false, false);
    assert_pairs_match_and_nonempty("row04", &pairs);
}

// ---------------------------------------------------------------------------
// Row 5 — exact division in all four quadrants
// ---------------------------------------------------------------------------

#[test]
fn cfg_row05_exact_all_quadrants() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut pairs = Vec::new();
    for _ in 0..2000 {
        let y = rng.range_i32(1, 1 << 15);
        let k = rng.range_i32(1, i32::MAX / y);
        let x = k.saturating_mul(y);
        for &(sx, sy) in &[(1, 1), (-1, 1), (1, -1), (-1, -1)] {
            pairs.push((x * sx, y * sy));
        }
    }
    assert_pairs_match_and_nonempty("row05", &pairs);
}

// ---------------------------------------------------------------------------
// Row 6 — |x| < |y|  (quotient 0, remainder x)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row06_smaller_magnitude() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut pairs = Vec::new();
    for _ in 0..1500 {
        let ay = rng.range_i32(2, i32::MAX);
        let ax = rng.range_i32(0, ay - 1);
        for &(sx, sy) in &[(1, 1), (-1, 1), (1, -1), (-1, -1)] {
            pairs.push((ax * sx, ay * sy));
        }
    }
    assert_pairs_match_and_nonempty("row06", &pairs);
}

// ---------------------------------------------------------------------------
// Row 7 — |x| == |y|  (quotient +/-1, remainder 0)
// ---------------------------------------------------------------------------

#[test]
fn cfg_row07_equal_magnitude() {
    let mut rng = Rng::new(SEED ^ 7);
    let mut pairs = Vec::new();
    for _ in 0..1500 {
        let a = rng.next_positive();
        pairs.push((a, a));
        pairs.push((-a, a));
        pairs.push((a, -a));
        pairs.push((-a, -a));
    }
    // INT_MIN paired with itself: |x| == |y| at the extreme.
    pairs.push((i32::MIN, i32::MIN));
    assert_pairs_match_and_nonempty("row07", &pairs);
}

// ---------------------------------------------------------------------------
// Row 8 — x == 0
// ---------------------------------------------------------------------------

#[test]
fn cfg_row08_zero_numerator() {
    let mut rng = Rng::new(SEED ^ 8);
    let mut pairs = vec![(0, 1), (0, -1), (0, i32::MAX), (0, i32::MIN)];
    for _ in 0..1000 {
        pairs.push((0, rng.next_i32_nonzero()));
    }
    assert_pairs_match_and_nonempty("row08", &pairs);
}

// ---------------------------------------------------------------------------
// Row 9 — y == 1
// ---------------------------------------------------------------------------

#[test]
fn cfg_row09_divisor_one() {
    let mut rng = Rng::new(SEED ^ 9);
    let mut pairs = vec![(i32::MIN, 1), (i32::MIN + 1, 1), (i32::MAX, 1), (0, 1)];
    for _ in 0..1000 {
        pairs.push((rng.next_i32(), 1));
    }
    assert_pairs_match_and_nonempty("row09", &pairs);
}

// ---------------------------------------------------------------------------
// Row 10 — y == -1, excluding the trapping x == INT_MIN
// ---------------------------------------------------------------------------

#[test]
fn cfg_row10_divisor_minus_one() {
    let mut rng = Rng::new(SEED ^ 10);
    let mut pairs = vec![(i32::MIN + 1, -1), (i32::MAX, -1), (0, -1), (1, -1), (-1, -1)];
    for _ in 0..1000 {
        let x = rng.next_i32();
        if x == i32::MIN {
            continue; // ERRORS.md row 3 — traps, covered in tests/trap.rs
        }
        pairs.push((x, -1));
    }
    assert_pairs_match_and_nonempty("row10", &pairs);
}

// ---------------------------------------------------------------------------
// Row 11 / 12 — extreme numerator, extreme divisor
// ---------------------------------------------------------------------------

const EXTREMES: [i32; 4] = [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX];

#[test]
fn cfg_row11_extreme_numerator() {
    let mut rng = Rng::new(SEED ^ 11);
    let mut pairs = Vec::new();
    for &x in &EXTREMES {
        for _ in 0..400 {
            let y = rng.next_i32_nonzero();
            if x == i32::MIN && y == -1 {
                continue; // traps
            }
            pairs.push((x, y));
        }
        for &y in &EXTREMES {
            if !(x == i32::MIN && y == -1) {
                pairs.push((x, y));
            }
        }
    }
    assert_pairs_match_and_nonempty("row11", &pairs);
}

#[test]
fn cfg_row12_extreme_divisor() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut pairs = Vec::new();
    for &y in &EXTREMES {
        for _ in 0..400 {
            pairs.push((rng.next_i32(), y));
        }
    }
    assert_pairs_match_and_nonempty("row12", &pairs);
}

// ---------------------------------------------------------------------------
// Row 13 — full boundary cross-product (also ERRORS.md G4)
// ---------------------------------------------------------------------------

#[test]
fn boundary_extremes_cross_product() {
    const VALS: [i32; 9] = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut pairs = Vec::new();
    for &x in &VALS {
        for &y in &VALS {
            if y == 0 {
                continue; // ERRORS.md rows 1-2
            }
            if x == i32::MIN && y == -1 {
                continue; // ERRORS.md row 3
            }
            pairs.push((x, y));
        }
    }
    assert_pairs_match_and_nonempty("row13", &pairs);
}

// ---------------------------------------------------------------------------
// Row 14 — printf("%d") width and sign sweep for quot and rem independently
// ---------------------------------------------------------------------------

#[test]
fn cfg_row14_printf_width_sweep() {
    let mut pairs = Vec::new();
    // Target quotient digit-counts 1,2,5,9,10 and remainder digit-counts likewise,
    // in every sign combination, by construction: x = q*y + r.
    let widths: [i32; 6] = [7, 42, 12345, 123456789, 2000000000, 1];
    for &q in &widths {
        for &y in &widths {
            for &(sq, sy) in &[(1i64, 1i64), (-1, 1), (1, -1), (-1, -1)] {
                let qq = q as i64 * sq;
                let yy = y as i64 * sy;
                if yy == 0 {
                    continue;
                }
                // remainder strictly smaller in magnitude than |y|, sign of x
                let r = if y > 1 { (q as i64) % (y as i64) } else { 0 };
                let x = qq * yy + if qq < 0 { -r } else { r };
                if x < i32::MIN as i64 || x > i32::MAX as i64 {
                    continue;
                }
                if x == i32::MIN as i64 && yy == -1 {
                    continue;
                }
                pairs.push((x as i32, yy as i32));
            }
        }
    }
    // Explicit widest renderings.
    pairs.push((i32::MAX, 1)); // "2147483647", "0"
    pairs.push((i32::MIN + 1, 1)); // "-2147483647", "0"
    pairs.push((i32::MAX, 2)); // 10 digits / 1 digit
    pairs.push((i32::MIN, 2)); // negative 10 digits
    assert_pairs_match_and_nonempty("row14", &pairs);
}

// ---------------------------------------------------------------------------
// Row 15 — powers of two and neighbours as divisors
// ---------------------------------------------------------------------------

#[test]
fn cfg_row15_power_of_two_divisors() {
    let mut rng = Rng::new(SEED ^ 15);
    let mut divisors: Vec<i32> = Vec::new();
    for bit in 0..31 {
        let p: i64 = 1i64 << bit;
        for cand in [p - 1, p, p + 1] {
            if cand >= 1 && cand <= i32::MAX as i64 {
                divisors.push(cand as i32);
                divisors.push(-(cand as i32));
            }
        }
    }
    divisors.push(i32::MIN); // -2^31 exactly
    divisors.sort_unstable();
    divisors.dedup();

    let mut pairs = Vec::new();
    for &y in &divisors {
        for _ in 0..24 {
            let x = rng.next_i32();
            if x == i32::MIN && y == -1 {
                continue;
            }
            pairs.push((x, y));
        }
    }
    assert_pairs_match_and_nonempty("row15", &pairs);
}

// ---------------------------------------------------------------------------
// Row 16 — unrestricted large-volume fuzz
// ---------------------------------------------------------------------------

#[test]
fn cfg_row16_unrestricted_fuzz() {
    let mut rng = Rng::new(SEED ^ 16);
    let mut pairs = Vec::with_capacity(20_000);
    while pairs.len() < 20_000 {
        let x = rng.next_i32();
        let y = rng.next_i32();
        if y == 0 || (x == i32::MIN && y == -1) {
            continue; // trapping inputs live in tests/trap.rs
        }
        pairs.push((x, y));
    }
    assert_pairs_match_and_nonempty("row16", &pairs);
}

// ---------------------------------------------------------------------------
// Row 17 / ERRORS.md G5 — dirty high bits in the argument registers
// ---------------------------------------------------------------------------

#[test]
fn abi_high_garbage_bits_ignored() {
    let cw = c_driver_wide();
    let rw = rust_driver_wide();
    let mut rng = Rng::new(SEED ^ 17);

    let mut failures = Vec::new();
    {
        let cap = Capture::new("row17");
        for _ in 0..500 {
            let x = rng.next_i32();
            let y = rng.next_i32_nonzero();
            if x == i32::MIN && y == -1 {
                continue;
            }
            // Splice random garbage into the undefined upper halves.
            let gx = ((rng.next_u64() as i64) << 32) | (x as u32 as i64);
            let gy = ((rng.next_u64() as i64) << 32) | (y as u32 as i64);

            let a = cap.mark();
            unsafe { cw(gx, gy) };
            let b = cap.mark();
            unsafe { rw(gx, gy) };
            let d = cap.mark();
            let (cb, rb) = (cap.slice(a, b), cap.slice(b, d));
            if cb != rb {
                failures.push(format!(
                    "  driver(0x{gx:016x}, 0x{gy:016x}) i.e. ({x}, {y}):\n    C    = {:?}\n    \
                     Rust = {:?}",
                    String::from_utf8_lossy(&cb),
                    String::from_utf8_lossy(&rb)
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "[row17] high-garbage-bit calls diverged:\n{}",
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Row 18 — long unflushed run
// ---------------------------------------------------------------------------

#[test]
fn cfg_row18_buffering_long_run() {
    let mut rng = Rng::new(SEED ^ 18);
    let mut inputs = Vec::with_capacity(5000);
    while inputs.len() < 5000 {
        let x = rng.next_i32();
        let y = rng.next_i32_nonzero();
        if x == i32::MIN && y == -1 {
            continue;
        }
        inputs.push((x, y));
    }

    let c = c_driver();
    let r = rust_driver();
    let (c_all, r_all) = {
        let cap = Capture::new("row18a");
        // All C calls back to back, with no intervening mark/flush.
        let a = cap.mark();
        for &(x, y) in &inputs {
            unsafe { c(x, y) };
        }
        let b = cap.mark();
        for &(x, y) in &inputs {
            unsafe { r(x, y) };
        }
        let d = cap.mark();
        (cap.slice(a, b), cap.slice(b, d))
    };

    assert_eq!(
        c_all.len(),
        r_all.len(),
        "[row18] total byte count differs over a 5000-call unflushed run: C={} Rust={}",
        c_all.len(),
        r_all.len()
    );
    if c_all != r_all {
        let at = c_all
            .iter()
            .zip(r_all.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        let lo = at.saturating_sub(80);
        panic!(
            "[row18] streams diverge at byte {at}:\n  C    ...{:?}\n  Rust ...{:?}",
            String::from_utf8_lossy(&c_all[lo..(at + 80).min(c_all.len())]),
            String::from_utf8_lossy(&r_all[lo..(at + 80).min(r_all.len())]),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 19 — interleaved calls sharing one stdout buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg_row19_interleaved_shared_stdout() {
    let mut rng = Rng::new(SEED ^ 19);
    let c = c_driver();
    let r = rust_driver();

    let mut inputs = Vec::with_capacity(2000);
    while inputs.len() < 2000 {
        let x = rng.next_i32();
        let y = rng.next_i32_nonzero();
        if x == i32::MIN && y == -1 {
            continue;
        }
        inputs.push((x, y));
    }

    // C and Rust alternate into the same libc buffer; the result must read as
    // consecutive identical pairs of lines.
    let bytes = {
        let cap = Capture::new("row19");
        let a = cap.mark();
        for &(x, y) in &inputs {
            unsafe {
                c(x, y);
                r(x, y);
            }
        }
        let b = cap.mark();
        cap.slice(a, b)
    };

    let text = String::from_utf8(bytes).expect("driver output must be valid UTF-8/ASCII");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        inputs.len() * 2,
        "[row19] expected {} lines, got {}",
        inputs.len() * 2,
        lines.len()
    );
    for (i, &(x, y)) in inputs.iter().enumerate() {
        let (cl, rl) = (lines[2 * i], lines[2 * i + 1]);
        assert_eq!(cl, rl, "[row19] driver({x}, {y}) interleaved output differs");
        assert_eq!(
            format!("{cl}\n"),
            expected_line(x, y),
            "[row19] driver({x}, {y}) did not match the reference model"
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md G1-G3 — documenting that the ABI admits no pointer/len/enum inputs
// ---------------------------------------------------------------------------

#[test]
fn abi_no_pointer_arguments() {
    // `void driver(int, int)` has no pointer, length, or enum parameters, so
    // null-pointer, zero/oversized-length and out-of-range-enum inputs are not
    // representable at this boundary. What IS representable is the full 2^64
    // space of register contents, and row 17 covers the undefined upper halves.
    //
    // The check that remains meaningful here is that both symbols really are
    // plain two-int functions that return normally and write exactly one line.
    assert!(
        both_export_driver(),
        "both libraries must export `driver` for the ABI to be comparable"
    );

    let (cb, rb) = {
        let cap = Capture::new("gabi");
        run_pair(&cap, 7, 3)
    };
    assert_eq!(cb, rb);
    assert_eq!(cb, b"quotient: 2, remainder: 1\n");
}

// ---------------------------------------------------------------------------
// Negative controls — prove the harness can actually SEE a difference.
//
// Without these, every "ok" above could be a vacuous pass (e.g. a capture that
// always yields zero bytes would make every comparison trivially equal).
// ---------------------------------------------------------------------------

#[test]
fn harness_detects_divergence() {
    let c = c_driver();
    let r = rust_driver();

    // Deliberately feed the two libraries DIFFERENT inputs. The captured byte
    // strings must differ, which proves the capture is really per-call.
    let (cb, rb) = {
        let cap = Capture::new("negctl");
        let a = cap.mark();
        unsafe { c(7, 3) };
        let b = cap.mark();
        unsafe { r(8, 3) };
        let d = cap.mark();
        (cap.slice(a, b), cap.slice(b, d))
    };
    assert_eq!(cb, b"quotient: 2, remainder: 1\n");
    assert_eq!(rb, b"quotient: 2, remainder: 2\n");
    assert_ne!(
        cb, rb,
        "harness failed to distinguish two different outputs — comparisons above are meaningless"
    );
}

#[test]
fn harness_capture_is_not_silently_empty() {
    // Each individual call must contribute a non-empty, newline-terminated line.
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    let cap = Capture::new("negctl2");
    for _ in 0..64 {
        let x = rng.next_i32();
        let y = rng.next_i32_nonzero();
        if x == i32::MIN && y == -1 {
            continue;
        }
        let (cb, rb) = run_pair(&cap, x, y);
        assert!(!cb.is_empty(), "C capture was empty for ({x}, {y})");
        assert!(!rb.is_empty(), "Rust capture was empty for ({x}, {y})");
        assert_eq!(cb.last(), Some(&b'\n'), "C line not newline-terminated");
        assert_eq!(rb.last(), Some(&b'\n'), "Rust line not newline-terminated");
        assert_eq!(cb, rb);
    }
}

#[test]
fn harness_loads_two_distinct_shared_objects() {
    // Guard against both handles accidentally resolving to the same library,
    // which would make every differential assertion compare C against itself.
    let cp = c_so_path().canonicalize().expect("canonicalize C .so");
    let rp = rust_so_path().canonicalize().expect("canonicalize Rust .so");
    assert_ne!(
        cp, rp,
        "the C and Rust shared objects resolved to the same file: {}",
        cp.display()
    );
    let cf = std::fs::read(&cp).expect("read C .so");
    let rf = std::fs::read(&rp).expect("read Rust .so");
    assert_ne!(cf, rf, "the two shared objects have identical contents");

    // And the two `driver` symbols must live at different addresses.
    let ca = c_driver() as usize;
    let ra = rust_driver() as usize;
    assert_ne!(
        ca, ra,
        "C and Rust `driver` resolved to the same address {ca:#x} — only one library is loaded"
    );
}
