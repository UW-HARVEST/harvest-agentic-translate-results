// CONFIGS.md row 37 -- byte-for-byte comparison of everything the two
// libraries write to stdout. `overunder` emits 9 printf lines, including a
// `%.2f` of a double and a `%s` of the memcpy-copied `label` buffer.
//
// Both `.so`s print with libc `printf`, so fd 1 is redirected with dup2 and a
// single fflush(NULL) drains both. This runs as its own test binary and in one
// test function, so nothing else races on fd 1.

mod common;

use common::*;

const CORNERS: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];

fn cmp_stdout(p: &Pair, a: i32, b: i32, c: i32, d: i32) {
    let out_c = capture_stdout("c", || {
        unsafe { (p.c.overunder)(a, b, c, d) };
    });
    let out_r = capture_stdout("r", || {
        unsafe { (p.r.overunder)(a, b, c, d) };
    });
    if out_c != out_r {
        panic!(
            "stdout mismatch for overunder({a}, {b}, {c}, {d})\n\
             --- C ({} bytes) ---\n{}\n--- RUST ({} bytes) ---\n{}\n--- raw C ---\n{:02x?}\n--- raw RUST ---\n{:02x?}",
            out_c.len(),
            String::from_utf8_lossy(&out_c),
            out_r.len(),
            String::from_utf8_lossy(&out_r),
            out_c,
            out_r
        );
    }
    assert!(
        !out_c.is_empty(),
        "capture produced nothing -- the harness is not observing printf output"
    );
}

#[test]
fn row37_stdout_byte_identical() {
    let p = load_pair();

    // Sanity: the capture mechanism really sees the library's printf output.
    let probe = capture_stdout("probe", || {
        unsafe { (p.c.overunder)(7, 2, 3, 4) };
    });
    let text = String::from_utf8_lossy(&probe).to_string();
    for needle in [
        "result_1 = 7",
        "result_2 = 2",
        "Converted values:",
        "Switch fall-through result:",
        "Copied block:",
        "label=Source",
        "Pointer operation result:",
        "Overflow protected conversion: 2147483647",
        "Underflow protected conversion: -2147483648",
        "Array copied via memcpy:",
    ] {
        assert!(
            text.contains(needle),
            "capture missing {needle:?}; got:\n{text}"
        );
    }
    assert_eq!(
        text.lines().count(),
        9,
        "expected 9 printf lines, got:\n{text}"
    );

    // Hand-picked values: each `a % 6` residue, negative a, zero, saturation.
    for a in [0, 1, 2, 3, 4, 5, 6, 7, -1, -2, -3, -4, -5, -6, -7] {
        cmp_stdout(&p, a, 11, -13, 5);
    }
    // `%.2f` of temp1 across magnitudes and signs, including huge values where
    // printf must render many digits.
    for a in [
        123, -123, 1_000_000, -1_000_000, 1_431_655_765, -1_431_655_765, i32::MAX, i32::MIN,
    ] {
        cmp_stdout(&p, a, 3, 3, 3);
    }
    // Saturating / NaN paths visible in "Converted values:".
    for (a, b, c, d) in [
        (2, i32::MAX, i32::MAX, 46341),      // conv2 saturates, sqrt NaN
        (3, i32::MIN, i32::MIN, -46341),     // conv2 saturates low
        (4, 0, 0, 65536),                    // sqrt of wrapped-to-0
        (5, 1, -1, 0),
    ] {
        cmp_stdout(&p, a, b, c, d);
    }
    // Full corner grid.
    for &a in &CORNERS {
        for &b in &CORNERS {
            for &c in &CORNERS {
                for &d in &CORNERS {
                    cmp_stdout(&p, a, b, c, d);
                }
            }
        }
    }
    // Randomized.
    let mut rng = Rng::new(SEED ^ 37);
    for _ in 0..600 {
        cmp_stdout(
            &p,
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
    // Small magnitudes, where %.2f rounding of temp1 = a*1.5 is most delicate.
    for _ in 0..400 {
        cmp_stdout(
            &p,
            rng.range_i32(-2000, 2000),
            rng.range_i32(-2000, 2000),
            rng.range_i32(-2000, 2000),
            rng.range_i32(-2000, 2000),
        );
    }
}
