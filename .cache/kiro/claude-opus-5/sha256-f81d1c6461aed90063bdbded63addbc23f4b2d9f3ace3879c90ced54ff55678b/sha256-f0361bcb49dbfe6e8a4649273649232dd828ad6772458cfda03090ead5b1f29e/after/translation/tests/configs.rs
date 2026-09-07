//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1…C14). Every test drives BOTH the C
//! `.so` and the Rust `.so` through their exported `sieve` symbol and compares
//! stdout byte-for-byte over many randomized inputs from a fixed seed. Each
//! comparison additionally checks the C's own output against an independent
//! model of `c_src/src/sieve.c`, so two implementations agreeing on the wrong
//! bytes is still a failure.

mod harness;

use harness::*;

/// Fold a start value into one whose run terminates within a sane output
/// budget. The signed-overflow region `[INT_MAX-7, INT_MAX]` runs for 4.3e9
/// lines and is covered separately by C14 / `ERRORS.md` E7–E9.
fn bounded(v: i64) -> i32 {
    let mut v = v;
    if v >= 2_147_483_640 {
        v -= 10;
    }
    v as i32
}

// --------------------------------------------------------------------- C1
/// C1: exhaustive sweep of every `val` in `-256..=256` — all 10 non-negative
/// residues, all 10 negative residues, zero, the sign boundary and 1/2/3-digit
/// widths, with no sampling gaps.
#[test]
fn c1_exhaustive_small_range() {
    let vals: Vec<i32> = (-256..=256).collect();
    assert_eq!(vals.len(), 513);
    assert_same_batch(&vals, "C1");
}

// --------------------------------------------------------------------- C2
/// C2: positive, residue == 9 — the break-immediately, single-iteration path.
#[test]
fn c2_positive_residue_nine_single_iteration() {
    let mut rng = Rng::new();
    let mut vals = Vec::new();
    while vals.len() < 300 {
        let v = rng.range(0, 100_000);
        let v = v - (v % 10) + 9; // force residue 9
        vals.push(v as i32);
    }
    for &v in &vals {
        assert_eq!(v % 10, 9, "C2 setup");
    }
    assert_same_batch(&vals, "C2");
    // Spot-check the defining property of this class: exactly one line.
    assert_eq!(c_out(vals[0]), format!("{}\n", vals[0]).as_bytes());
    assert_eq!(r_out(vals[0]), format!("{}\n", vals[0]).as_bytes());
}

// --------------------------------------------------------------------- C3
/// C3: positive, residue != 9 — the multi-iteration path, covering the other
/// nine residues.
#[test]
fn c3_positive_other_residues_multi_iteration() {
    let mut rng = Rng::new();
    let mut vals = Vec::new();
    let mut seen = [false; 10];
    while vals.len() < 400 {
        let v = rng.range(0, 100_000);
        if v % 10 != 9 {
            seen[(v % 10) as usize] = true;
            vals.push(v as i32);
        }
    }
    for r in 0..9 {
        assert!(seen[r], "C3 never generated residue {r}");
    }
    assert_same_batch(&vals, "C3");
}

// --------------------------------------------------------------------- C4
/// C4: zero and the immediate neighbourhood of the sign boundary.
#[test]
fn c4_zero_and_sign_boundary() {
    let mut vals: Vec<i32> = (-3..=3).collect();
    let mut rng = Rng::new();
    while vals.len() < 20 {
        vals.push(rng.range(-3, 3) as i32);
    }
    assert_same_batch(&vals, "C4");
    // Zero must print 0..=9 (ten lines), not stop at 0.
    assert_eq!(c_out(0), b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    assert_eq!(r_out(0), b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

// --------------------------------------------------------------------- C5
/// C5: negative values whose last digit is 9. C's `%` truncates, so
/// `-39 % 10 == -9`, never `9`; the predicate is never true and the loop counts
/// all the way up to 9.
#[test]
fn c5_negative_ending_in_nine_never_breaks() {
    let mut rng = Rng::new();
    let mut vals = Vec::new();
    while vals.len() < 300 {
        let v = rng.range(-100_000, -1);
        let v = v - (v % 10) - 9; // force truncating remainder -9
        vals.push(v as i32);
    }
    for &v in &vals {
        assert_eq!(v % 10, -9, "C5 setup: expected truncating remainder -9");
    }
    assert_same_batch(&vals, "C5");
    // Spot-check the trap: it must NOT stop on the first value.
    let v = vals[0];
    for out in [c_out(v), r_out(v)] {
        assert!(
            out.len() > format!("{v}\n").len(),
            "C5: sieve({v}) stopped immediately — a negative remainder was treated as 9"
        );
        assert!(out.ends_with(b"\n9\n"), "C5: sieve({v}) must run up to 9");
    }
}

// --------------------------------------------------------------------- C6
/// C6: negative values with every other residue (-8..=0).
#[test]
fn c6_negative_other_residues() {
    let mut rng = Rng::new();
    let mut vals = Vec::new();
    let mut seen = [false; 10];
    while vals.len() < 400 {
        let v = rng.range(-100_000, -1);
        let r = (-(v % 10)) as usize; // 0..=9
        if r != 9 {
            seen[r] = true;
            vals.push(v as i32);
        }
    }
    for r in 0..9 {
        assert!(seen[r], "C6 never generated negative residue -{r}");
    }
    assert_same_batch(&vals, "C6");
}

// --------------------------------------------------------------------- C7
/// C7: `printf` field width grows mid-run across each power of ten.
#[test]
fn c7_positive_digit_width_transitions() {
    let mut vals = Vec::new();
    let mut p: i64 = 10;
    for _ in 0..8 {
        for off in [1i64, 2, 5, 9] {
            let v = p - off; // 9,8,5,1 then 99,98,95,91 then 999,…
            if v >= 0 {
                vals.push(v as i32);
            }
        }
        p *= 10;
    }
    assert_eq!(vals.len(), 32, "C7 should cover 8 boundaries x 4 offsets");
    assert_same_batch(&vals, "C7");
}

// --------------------------------------------------------------------- C8
/// C8: the minus sign disappears mid-run and the negative field width shrinks.
#[test]
fn c8_negative_digit_width_transitions() {
    let vals: Vec<i32> = vec![
        -1, -2, -9, -10, -11, -99, -100, -101, -999, -1000, -1001, -10000,
    ];
    assert_eq!(vals.len(), 12);
    assert_same_batch(&vals, "C8");
    assert_eq!(c_out(-1), b"-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    assert_eq!(r_out(-1), b"-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

// --------------------------------------------------------------------- C9
/// C9: large positive magnitude (10-digit output), just clear of the overflow
/// region, so every run still terminates normally.
#[test]
fn c9_large_positive_near_int_max() {
    let mut rng = Rng::new();
    let vals: Vec<i32> = (0..200)
        .map(|_| rng.range(2_000_000_000, 2_147_483_600) as i32)
        .collect();
    for &v in &vals {
        assert!(v < 2_147_483_640, "C9 must stay out of the overflow region");
    }
    assert_same_batch(&vals, "C9");
}

// --------------------------------------------------------------------- C10
/// C10: multi-million-line runs from large negative starts (~18 MB of output
/// each), where any per-iteration drift would accumulate visibly.
#[test]
fn c10_large_negative_long_run() {
    let mut rng = Rng::new();
    let vals: Vec<i32> = (0..8)
        .map(|_| rng.range(-2_000_000, -1_000_000) as i32)
        .collect();
    for &v in &vals {
        let c = c_out(v);
        let r = r_out(v);
        assert_eq!(
            c.len(),
            r.len(),
            "[C10] sieve({v}): byte counts differ (C {} vs Rust {})",
            c.len(),
            r.len()
        );
        assert!(c == r, "[C10] sieve({v}) diverged inside a {}-byte stream", c.len());
        // Closed form from the C source: a negative start prints val..=9.
        let lines = c.iter().filter(|&&b| b == b'\n').count() as i64;
        assert_eq!(lines, 10 - v as i64, "[C10] wrong line count for sieve({v})");
        assert!(c.starts_with(format!("{v}\n").as_bytes()));
        assert!(c.ends_with(b"\n9\n"));
    }
}

// --------------------------------------------------------------------- C11
/// C11: uniform over the whole `i32` space, projected into a terminating,
/// budget-sized run (a positive start already runs for at most ten lines;
/// negative starts are folded into `-100_000..=0`).
#[test]
fn c11_full_range_random_projected() {
    let mut rng = Rng::new();
    let mut vals = Vec::new();
    let (mut neg, mut pos) = (0usize, 0usize);
    while vals.len() < 500 {
        let raw = rng.next_u32() as i32;
        let v: i32 = if raw < 0 {
            neg += 1;
            -((raw.unsigned_abs() % 100_001) as i32)
        } else {
            pos += 1;
            bounded(raw as i64)
        };
        vals.push(v);
    }
    assert!(
        neg > 100 && pos > 100,
        "C11 sign coverage too skewed: {neg} negative / {pos} positive"
    );
    assert_same_batch(&vals, "C11");
}

// --------------------------------------------------------------------- C12
/// C12: statelessness. Repeated and interleaved invocations through the SAME
/// loaded handle must not influence one another, and C and Rust must agree on
/// the whole concatenated stream.
#[test]
fn c12_repeat_and_interleaved_invocations() {
    let mut rng = Rng::new();
    for i in 0..100 {
        let a = bounded(rng.range(-500, 2_147_483_647));
        let b = bounded(rng.range(-500, 2_147_483_647));

        // Same value three times: C-only vs Rust-only, and vs 3x the single run.
        let c3 = run(&[step(Side::C, a), step(Side::C, a), step(Side::C, a)]);
        let r3 = run(&[step(Side::Rust, a), step(Side::Rust, a), step(Side::Rust, a)]);
        assert_eq!(c3, r3, "[C12/{i}] 3x sieve({a}) diverged");
        let tripled = expected_output(a).repeat(3);
        assert_eq!(
            c3,
            tripled.as_bytes(),
            "[C12/{i}] repeated call was not stateless"
        );

        // Interleaved a, b, a.
        let cs = run(&[step(Side::C, a), step(Side::C, b), step(Side::C, a)]);
        let rs = run(&[
            step(Side::Rust, a),
            step(Side::Rust, b),
            step(Side::Rust, a),
        ]);
        assert_eq!(cs, rs, "[C12/{i}] sequence ({a},{b},{a}) diverged");
    }
}

// --------------------------------------------------------------------- C13
/// C13: cross-library interleaving on the shared libc `stdout` FILE buffer.
/// Swapping which library serves which call must not change a single byte —
/// this also proves the Rust side writes through the same `printf` buffer
/// rather than a separate Rust-side one that would flush in a different order.
#[test]
fn c13_cross_library_interleaving_shared_stdout() {
    let mut rng = Rng::new();
    for i in 0..50 {
        let a = bounded(rng.range(-200, 2_147_483_647));
        let b = bounded(rng.range(-200, 2_147_483_647));
        let c_then_r = run(&[step(Side::C, a), step(Side::Rust, b)]);
        let r_then_c = run(&[step(Side::Rust, a), step(Side::C, b)]);
        assert_eq!(
            c_then_r, r_then_c,
            "[C13/{i}] swapping which library serves ({a},{b}) changed the byte stream"
        );
        let expected = format!("{}{}", expected_output(a), expected_output(b));
        assert_eq!(
            c_then_r,
            expected.as_bytes(),
            "[C13/{i}] interleaved stream disagrees with the model"
        );
    }
}

// --------------------------------------------------------------------- C14
/// C14: huge-output shapes compared by bounded stdout prefix in child
/// processes — the valid-path view of the overflow / extreme-magnitude starts.
#[test]
fn c14_huge_output_shapes_prefix() {
    const LIMIT: usize = 64 * 1024;
    for arg in [
        "2147483647",  // INT_MAX
        "2147483640",  // first start that cannot terminate without overflow
        "2147483646",
        "-2147483648", // INT_MIN
        "-2147483647", // INT_MIN + 1
    ] {
        assert_same_prefix(arg, LIMIT, "C14");
    }
}

// --------------------------------------------------------- no driver binary
/// The project builds no executable driver (`c_src/CMakeLists.txt` contains
/// only `add_library`), so there is no C-vs-Rust *binary* stdout comparison to
/// make. This test pins that fact so the Phase B gate cannot silently become
/// unmet if a driver is added later.
#[test]
fn no_driver_binary_to_compare() {
    let cmake = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/CMakeLists.txt"),
    )
    .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable driver — Phase B must additionally compare the C and \
         Rust binaries' stdout byte-for-byte"
    );
    assert!(cmake.contains("add_library(Sieve SHARED"));
}
