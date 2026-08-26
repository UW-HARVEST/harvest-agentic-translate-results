// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Every test loads BOTH shared libraries via `libloading` and compares the raw
// stdout byte streams produced through their exported `sieve` symbol. Runs happen
// in byte-capped, time-limited forked children so that a divergent
// implementation fails instead of hanging (see tests/common/mod.rs).

mod common;

use common::*;

/// Harness self-check: proves the fd-level capture really observes the library's
/// output, so a passing differential assertion cannot mean "both produced
/// nothing". Also exercises an in-process (unforked) call of the exported symbol.
#[test]
fn c0_harness_sanity() {
    let (c, r) = funcs();
    assert_eq!(run(c, 9), b"9\n", "C output for sieve(9)");
    assert_eq!(run(r, 9), b"9\n", "Rust output for sieve(9)");
    assert_eq!(run(c, 5), b"5\n6\n7\n8\n9\n");
    assert_eq!(run(r, 5), b"5\n6\n7\n8\n9\n");
    assert_eq!(run(c, -3), b"-3\n-2\n-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    assert_eq!(run(r, -3), run(c, -3));
    // an independent model of the C loop agrees with the C ground truth
    assert_eq!(run(c, -3), model(-3, 100));
    // and the forked path agrees with the in-process path
    assert_eq!(run_one(c, 5, CAP).bytes, b"5\n6\n7\n8\n9\n");
    assert_eq!(run_one(r, 5, CAP).bytes, b"5\n6\n7\n8\n9\n");
}

/// C1: `val = 9` — positive, residue 9, exactly one iteration.
#[test]
fn c1_single_iteration_exact() {
    assert_same(9);
    let (c, r) = funcs();
    assert_eq!(run_one(c, 9, CAP).bytes, b"9\n");
    assert_eq!(run_one(r, 9, CAP).bytes, b"9\n");
}

/// C2: `val = 0` — zero, residue 0, ten lines.
#[test]
fn c2_zero_start() {
    assert_same(0);
    let (c, _) = funcs();
    assert_eq!(run_one(c, 0, CAP).bytes, b"0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n");
}

/// C3: every positive start in the first decade.
#[test]
fn c3_first_decade_all() {
    assert_same_all("first decade", 1..=8);
}

/// C4: randomized large positive values with residue 9 (single line).
#[test]
fn c4_random_positive_residue_nine() {
    let mut rng = Rng::new(0x5EED_0004);
    let vals: Vec<i32> = (0..256)
        .map(|_| rng.range(0, 214_748_362) as i32 * 10 + 9)
        .collect();
    assert_same_all("positive residue 9", vals);
}

/// C5: randomized large positive values with residue != 9 (2..10 lines,
/// exercising the carry into the next decade and every digit width), plus all
/// ten residues of a few random decades.
#[test]
fn c5_random_positive_other_residues() {
    let mut rng = Rng::new(0x5EED_0005);
    let mut vals = Vec::new();
    while vals.len() < 512 {
        let v = rng.range_i32(1, i32::MAX - 16);
        if v % 10 != 9 {
            vals.push(v);
        }
    }
    for _ in 0..16 {
        let base = rng.range(0, 214_748_300) as i32 * 10;
        for d in 0..10 {
            vals.push(base + d);
        }
    }
    assert_same_all("positive misc residues", vals);
}

/// C6: all ten residues of the highest decade that still terminates.
#[test]
fn c6_top_decade_all_residues() {
    assert_same_all("top decade", 2_147_483_630..=2_147_483_639);
}

/// C7: `INT_MAX - 8` = largest input that terminates without overflow.
#[test]
fn c7_largest_terminating() {
    assert_same(2_147_483_639);
    let (c, r) = funcs();
    assert_eq!(run_one(c, 2_147_483_639, CAP).bytes, b"2147483639\n");
    assert_eq!(run_one(r, 2_147_483_639, CAP).bytes, b"2147483639\n");
}

/// C8: `val = -1` — negative residue `-1`, the floor-mod trap.
#[test]
fn c8_negative_one() {
    assert_same(-1);
    let (c, _) = funcs();
    assert_eq!(
        run_one(c, -1, CAP).bytes,
        b"-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n"
    );
}

/// C9: two full negative decades — every negative residue class.
#[test]
fn c9_negative_two_decades_all() {
    assert_same_all("negative decades", -20..=-1);
}

/// C10: randomized small negatives (long runs, sign and width changes).
#[test]
fn c10_random_small_negative() {
    let mut rng = Rng::new(0x5EED_0010);
    let vals: Vec<i32> = (0..192).map(|_| rng.range_i32(-2000, -1)).collect();
    assert_same_all("small negatives", vals);
}

/// C11: randomized large negatives — ~100k-line runs that refill libc's buffer
/// many times and cross every digit-width boundary.
#[test]
fn c11_random_large_negative() {
    let mut rng = Rng::new(0x5EED_0011);
    let vals: Vec<i32> = (0..6).map(|_| rng.range_i32(-100_000, -20_000)).collect();
    assert_same_all("large negatives", vals);
}

/// C12: digit-width transition shapes.
#[test]
fn c12_digit_width_transitions() {
    assert_same_all(
        "digit widths",
        [
            -10, -100, -1000, -10000, -9, -99, -999, -9999, 10, 100, 1000, 10000, 99, 999, 9999,
            100_000, 1_000_000, 10_000_000, 100_000_000, 1_000_000_000,
        ],
    );
}

/// C13: randomized full-domain sweep restricted to bounded outputs.
#[test]
fn c13_random_full_domain_bounded() {
    let mut rng = Rng::new(0x5EED_0013);
    let vals: Vec<i32> = (0..512)
        .map(|_| rng.range_i32(-3000, i32::MAX - 8))
        .collect();
    assert_same_all("full domain (bounded)", vals);
}

/// C14: stdout is a pipe (different write pattern than a regular file), driven
/// in-process so the exported symbol is also exercised without an intervening
/// fork.
#[test]
fn c14_stdout_is_pipe() {
    let (c, r) = funcs();
    const CAP1: usize = 1 << 20;
    for val in [-5000, -1, 0, 7, 9, 1_234_567] {
        let (co, c_capped) = capture_pipe_capped(|| unsafe { c(val) }, CAP1);
        let (ro, r_capped) = capture_pipe_capped(|| unsafe { r(val) }, CAP1);
        assert!(!co.is_empty(), "no C output through pipe for sieve({val})");
        assert!(!c_capped, "C output exceeded the cap for sieve({val})");
        assert_eq!(
            c_capped, r_capped,
            "cap behaviour differs for sieve({val}) (Rust ran away?)"
        );
        assert_eq!(
            co.len(),
            ro.len(),
            "pipe output length differs for sieve({val})"
        );
        assert!(co == ro, "pipe output bytes differ for sieve({val})");
    }
}

/// C15: repeated and C/Rust-interleaved invocations (statelessness, and no
/// residual stream state left behind by either library).
#[test]
fn c15_repeated_and_interleaved() {
    let (c, r) = funcs();
    let c1 = run_one(c, 3, CAP);
    let c2 = run_one(c, 3, CAP);
    assert_eq!(c1, c2, "C is not stateless");
    let r1 = run_one(r, 3, CAP);
    let r2 = run_one(r, 3, CAP);
    assert_eq!(r1, r2, "Rust is not stateless");
    assert_eq!(c1, r1, "sieve(3) differs");

    // Interleave the two libraries in one output stream, in both orders.
    let inter_c_first = fork_capture(Dest::Pipe, CAP, TIMEOUT_MS, move || unsafe {
        c(-3);
        r(-3);
        c(15);
        r(15);
    });
    let inter_r_first = fork_capture(Dest::Pipe, CAP, TIMEOUT_MS, move || unsafe {
        r(-3);
        c(-3);
        r(15);
        c(15);
    });
    assert!(!inter_c_first.bytes.is_empty());
    assert_eq!(
        inter_c_first, inter_r_first,
        "interleaving order changed the byte stream"
    );
}

/// C16: many calls concatenated into one stream, compared as a whole.
#[test]
fn c16_concatenated_stream() {
    let (c, r) = funcs();
    let mut rng = Rng::new(0x5EED_0016);
    let vals: Vec<i32> = (0..40).map(|_| rng.range_i32(-500, 500_000)).collect();
    let co = run_vals(c, &vals, CAP);
    let ro = run_vals(r, &vals, CAP);
    assert!(!co.bytes.is_empty());
    assert!(!co.capped && !co.timed_out, "C run cut short: {co:?}");
    assert_eq!(
        co.bytes.len(),
        ro.bytes.len(),
        "concatenated stream length differs ({:?} vs {:?})",
        co,
        ro
    );
    assert!(co == ro, "concatenated stream differs: {co:?} vs {ro:?}");
}

/// C17: the effectively unbounded runs — `INT_MIN`, `INT_MIN + 1` and the signed
/// overflow range — compared as an 8 KiB output prefix in a forked child (the
/// full runs are ~2^31 lines).
#[test]
fn c17_unbounded_runs_prefix() {
    let (c, r) = funcs();
    const WANT: usize = 8192;
    for val in [
        i32::MIN,
        i32::MIN + 1,
        2_147_483_640,
        2_147_483_641,
        2_147_483_647,
    ] {
        let co = child_prefix(c, val, WANT);
        let ro = child_prefix(r, val, WANT);
        assert!(
            co.len() >= 1024,
            "C child produced only {} bytes for sieve({val})",
            co.len()
        );
        assert_eq!(
            co.len(),
            ro.len(),
            "prefix length differs for sieve({val}) (C {} vs Rust {})",
            co.len(),
            ro.len()
        );
        assert!(
            co == ro,
            "prefix bytes differ for sieve({val}); C starts {:?}, Rust starts {:?}",
            String::from_utf8_lossy(&co[..co.len().min(64)]),
            String::from_utf8_lossy(&ro[..ro.len().min(64)])
        );
    }
}
