//! Level 3: the top-level `doubleneg` entry point — return value *and* the
//! bytes it writes to stdout must match the C reference exactly.

mod common;

use common::{both, show, stdout_lock};

fn compare(a: i32, b: i32, c: i32, d: i32) {
    let _guard = stdout_lock();
    let (cimpl, rimpl) = both();

    let (crv, cout) = cimpl.doubleneg_capture(a, b, c, d);
    let (rrv, rout) = rimpl.doubleneg_capture(a, b, c, d);

    // Guard against a vacuous comparison of two empty captures.
    assert!(
        cout.len() > 400 && cout.iter().filter(|&&b| b == b'\n').count() > 30,
        "suspiciously small C stdout capture ({} bytes) for doubleneg({a}, {b}, {c}, {d}): {:?}",
        cout.len(),
        show(&cout)
    );

    if cout != rout {
        // Report the first differing line to keep the failure readable.
        let cs = show(&cout);
        let rs = show(&rout);
        let diff = cs
            .lines()
            .zip(rs.lines())
            .enumerate()
            .find(|(_, (x, y))| x != y)
            .map(|(i, (x, y))| format!("first diff at line {i}:\n  C   : {x}\n  Rust: {y}"))
            .unwrap_or_else(|| {
                format!(
                    "line counts differ: C={} Rust={}",
                    cs.lines().count(),
                    rs.lines().count()
                )
            });
        panic!("doubleneg({a}, {b}, {c}, {d}) stdout mismatch\n{diff}");
    }
    assert_eq!(
        crv, rrv,
        "doubleneg({a}, {b}, {c}, {d}) return value: C={crv}, Rust={rrv}"
    );
}

#[test]
fn doubleneg_fixed_cases() {
    let vals: [i32; 17] = [
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        10,
        -10,
        42,
        100,
        255,
        256,
        -256,
        1000,
        i32::MAX,
        i32::MIN,
    ];
    // Vary one parameter at a time around a fixed baseline, then a few
    // all-different combinations.
    for &v in &vals {
        compare(v, 3, 4, 5);
        compare(3, v, 4, 5);
        compare(3, 4, v, 5);
        compare(3, 4, 5, v);
    }
    compare(0, 0, 0, 0);
    compare(1, 1, 1, 1);
    compare(-1, -1, -1, -1);
    compare(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    compare(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    compare(i32::MIN, -1, i32::MIN, -1);
    compare(123456789, -987654321, 1234, -5678);
}

#[test]
fn doubleneg_exponent_sweep() {
    // `c % 10` selects the power of ten, so cover every residue (including the
    // negative ones C's `%` produces) against several a/b ratios.
    for c in -19i32..=19 {
        for (a, b) in [
            (0, 0),
            (1, 1),
            (1, 3),
            (-1, 3),
            (7, -13),
            (i32::MAX, 1),
            (i32::MIN, 1),
            (i32::MAX, -1),
            (1, i32::MAX),
            (1, i32::MIN),
            (999999937, 7),
        ] {
            compare(a, b, c, 1);
        }
    }
}

#[test]
fn doubleneg_signed_zero_and_special_prints() {
    // `a == 0` with a negative `b` yields -0.0, and `%e` must keep the sign.
    for b in [-1i32, -3, -10, i32::MIN, 1, 3, 10, i32::MAX] {
        for c in -19i32..=19 {
            compare(0, b, c, 0);
        }
    }
    // `b == 0` leaves `result` at +0.0 before the multiply.
    for c in -19i32..=19 {
        compare(0, 0, c, 0);
        compare(1, 0, c, 0);
        compare(-1, 0, c, 0);
        compare(i32::MIN, 0, c, 0);
    }
}

#[test]
fn doubleneg_exponent_formatting_fuzz() {
    // The only `%e` conversions with data-dependent digits come from
    // `calculate_with_doubles(param1, param2, param3)`, so hammer a/b ratios
    // across every power of ten to shake out rounding differences.
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 13) as i32
    };
    for _ in 0..1500 {
        let a = next();
        let b = next();
        // Sweep c over all ten residues (both signs) for each ratio.
        let c = next() % 20;
        compare(a, b, c, next());
    }
    // Ratios whose decimal expansion is long / repeating are the most likely
    // to land on a rounding tie at six fraction digits.
    for (a, b) in [
        (1, 3),
        (2, 3),
        (1, 7),
        (1, 9),
        (1, 11),
        (1, 6),
        (5, 6),
        (1, 16),
        (1, 32),
        (1, 64),
        (1, 128),
        (1, 1024),
        (3, 8),
        (1, 2048),
        (65, 128),
        (1, 512),
        (999999999, 7),
        (2147483647, 3),
        (-2147483648, 7),
        (1, 2000000),
        (1, 20000000),
        (15, 16),
        (105, 32),
        (1000005, 2),
        (10000005, 2),
        (100000005, 2),
    ] {
        for c in -19i32..=19 {
            compare(a, b, c, 1);
        }
    }
}

#[test]
fn doubleneg_randomized() {
    let mut state = 0x51ed_270b_9f11_2a37u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 17) as i32
    };
    for _ in 0..1000 {
        let (a, b, c, d) = (next(), next(), next(), next());
        compare(a, b, c, d);
    }
    // Small-magnitude randoms exercise the "value found" branches more often.
    for _ in 0..1000 {
        let (a, b, c, d) = (next() % 512, next() % 512, next() % 512, next() % 512);
        compare(a, b, c, d);
    }
}
