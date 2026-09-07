//! Phase B — the `doubleneg` driver: return value **and** byte-for-byte stdout.
//!
//! CONFIGS.md rows 34–41. `c_src/CMakeLists.txt` builds no executable, so the
//! "binary driver" comparison is done here by redirecting fd 1 around each call
//! into the C `.so` and the Rust `.so` respectively.
//!
//! fd 1 redirection is process-global, so every test in this file takes a
//! process-wide lock.

mod common;

use std::sync::Mutex;

use common::*;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Call `doubleneg` in both libraries and compare the return value and the
/// captured stdout.
fn check_doubleneg(cases: &[(i32, i32, i32, i32)], row: &str) {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnDoubleneg>(SYM_DOUBLENEG) };

    for &(a, b, cc, d) in cases {
        let (cv, cout) = capture_stdout(|| unsafe { c(a, b, cc, d) });
        let (rv, rout) = capture_stdout(|| unsafe { r(a, b, cc, d) });
        assert_eq!(
            cv, rv,
            "{row}: doubleneg({a}, {b}, {cc}, {d}) return value => C {cv} vs Rust {rv}"
        );
        assert!(
            cout == rout,
            "{}\n{}",
            format!("{row}: doubleneg({a}, {b}, {cc}, {d})"),
            diff_report(row, &cout, &rout)
        );
        assert!(
            !cout.is_empty(),
            "{row}: stdout capture produced nothing -- the harness is broken"
        );
    }
}

#[test]
fn row34_all_truthiness_combinations() {
    let mut cases = Vec::new();
    for m in 0u32..16 {
        let pick = |bit: u32, nonzero: i32| if m & (1 << bit) != 0 { nonzero } else { 0 };
        cases.push((pick(0, 5), pick(1, 3), pick(2, 2), pick(3, 9)));
        cases.push((pick(0, -5), pick(1, -3), pick(2, -2), pick(3, -9)));
    }
    check_doubleneg(&cases, "row34");
}

#[test]
fn row35_zero_divisor() {
    let mut cases = Vec::new();
    for a in [0, 1, -1, 100, -100, 255, 256, i32::MAX, i32::MIN] {
        for cc in [0, 1, -1, 9, -9, 10, -10] {
            cases.push((a, 0, cc, 42));
        }
    }
    check_doubleneg(&cases, "row35");
}

#[test]
fn row36_negative_param1_negative_bytes() {
    let mut cases = Vec::new();
    for a in [-1, -7, -100, -255, -256, -257, -1000, -65536] {
        for b in [-3, -1, 1, 3, 100] {
            cases.push((a, b, -5, -42));
            cases.push((a, b, 5, 42));
        }
    }
    check_doubleneg(&cases, "row36");
}

#[test]
fn row37_overflowing_search_byte_sweep() {
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut cases = Vec::new();
    for &a in &extremes {
        for &b in &extremes {
            cases.push((a, b, 3, 7));
        }
    }
    check_doubleneg(&cases, "row37");
}

#[test]
fn row38_param3_exponent_sweep() {
    let cases: Vec<(i32, i32, i32, i32)> = (-25..=25).map(|cc| (355, 113, cc, 17)).collect();
    check_doubleneg(&cases, "row38");
}

#[test]
fn row39_needle_not_found_configurations() {
    // The generated buffer is `(param1 + i*7) % 256` for i in 0..256. With a
    // stride of 7 (coprime with 256) every byte value appears, so to reach the
    // "not found" branch we rely on the *negative* seed case, where the C `%`
    // yields negative remainders and the buffer only covers part of the byte
    // range. These inputs deliberately probe both sides.
    let mut cases = Vec::new();
    for a in [-1, -2, -128, -129, -255, 1, 127, 128, 255] {
        for b in [0, 1, 128, 200, 255, 256, -128, -200] {
            cases.push((a, b, 100, 42));
            cases.push((a, b, -100, -42));
        }
    }
    check_doubleneg(&cases, "row39");
}

#[test]
fn row40_random_params() {
    let mut rng = Rng::new(0x4001);
    let cases: Vec<(i32, i32, i32, i32)> = (0..600)
        .map(|_| {
            (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            )
        })
        .collect();
    check_doubleneg(&cases, "row40");
}

#[test]
fn row40b_random_small_params() {
    // Small magnitudes make the `%e` formatting, the `%256` wrap and the
    // memchr hit/miss branches all land in interesting places.
    let mut rng = Rng::new(0x400B);
    let cases: Vec<(i32, i32, i32, i32)> = (0..600)
        .map(|_| {
            let s = |r: &mut Rng| (r.next_u32() % 1024) as i32 - 512;
            (s(&mut rng), s(&mut rng), s(&mut rng), s(&mut rng))
        })
        .collect();
    check_doubleneg(&cases, "row40b");
}

/// row41: there is no executable target in `c_src/CMakeLists.txt`; this asserts
/// that fact stays true so the "compare binaries' stdout" obligation cannot be
/// silently missed, and demonstrates the stdout capture actually captures.
#[test]
fn row41_no_binary_target_and_capture_works() {
    let cmake = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/CMakeLists.txt"),
    )
    .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable -- add a binary stdout comparison"
    );

    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnDoubleneg>(SYM_DOUBLENEG) };
    let (_, cout) = capture_stdout(|| unsafe { c(1, 2, 3, 4) });
    let (_, rout) = capture_stdout(|| unsafe { r(1, 2, 3, 4) });
    assert!(cout.starts_with(b"=== Starting foo() execution ==="));
    assert!(cout.ends_with(b"\n"));
    assert_eq!(cout, rout, "{}", diff_report("row41", &cout, &rout));
}
