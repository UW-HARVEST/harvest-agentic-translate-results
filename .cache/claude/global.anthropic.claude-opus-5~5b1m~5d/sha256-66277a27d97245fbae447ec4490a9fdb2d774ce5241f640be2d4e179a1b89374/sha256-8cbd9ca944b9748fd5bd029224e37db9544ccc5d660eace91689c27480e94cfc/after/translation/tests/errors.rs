//! Phase C — error / rejection-path differential tests, one per `ERRORS.md` row.
//!
//! `driver` has no return value and no error code, so "same rejection" means
//! "the same bytes on stdout (often: none at all)" and, for the non-terminating
//! inputs, "the same failure to return".

mod common;

use common::{assert_same, assert_same_all, c_driver, rust_driver, Rng};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// E1..E4 — the `while (x > 0 || y > 0)` guard rejects the whole computation.
// ---------------------------------------------------------------------------

/// E1 — `(0, 0)`: guard false, zero bytes of output.
#[allow(dead_code)]
fn test_e1_zero_zero_no_output() {
    let out = assert_same(0, 0);
    assert_eq!(out, b"", "driver(0, 0) must print nothing");
}

/// E2 — both strictly negative.
#[allow(dead_code)]
fn test_e2_both_negative_no_output() {
    for &(x, y) in &[(-1, -1), (-7, -3), (-100, -100), (-1, -2147483647)] {
        let out = assert_same(x, y);
        assert_eq!(out, b"", "driver({x}, {y}) must print nothing");
    }
    let mut rng = Rng::new(0xE2);
    for _ in 0..200 {
        let out = assert_same(rng.range_i32(i32::MIN, -1), rng.range_i32(i32::MIN, -1));
        assert_eq!(out, b"");
    }
}

/// E3 — the negative extreme `INT_MIN`.
#[allow(dead_code)]
fn test_e3_int_min_no_output() {
    for &(x, y) in &[
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MIN + 1),
        (i32::MIN + 1, i32::MIN),
    ] {
        assert_eq!(assert_same(x, y), b"");
    }
}

/// E4 — mixed sign but both non-positive.
#[allow(dead_code)]
fn test_e4_mixed_nonpositive_no_output() {
    for &(x, y) in &[
        (0, -1),
        (-1, 0),
        (i32::MIN, 0),
        (0, i32::MIN),
        (-2147483647, 0),
        (0, -2147483647),
    ] {
        assert_eq!(assert_same(x, y), b"", "driver({x}, {y})");
    }
}

// ---------------------------------------------------------------------------
// E5..E8 — the four inner rejections.
// ---------------------------------------------------------------------------

/// E5 — `if (y == 0) continue;`: the `y` half of the body is rejected, so no
/// `"y\n"` is ever printed.
#[allow(dead_code)]
fn test_e5_y_zero_continue() {
    for x in 1..=25 {
        let out = assert_same(x, 0);
        assert!(
            !out.windows(2).any(|w| w == b"y\n"),
            "driver({x}, 0) printed \"y\\n\" but y was already 0"
        );
        assert!(out.starts_with(b"loop\nx\n"));
    }
}

/// E6 — `if (x > 0)` false: the `x` half of the body is rejected, so no
/// `"x\n"` is ever printed.
#[allow(dead_code)]
fn test_e6_x_nonpositive() {
    for x in [i32::MIN, -5, -1, 0] {
        for y in 1..=25 {
            let out = assert_same(x, y);
            assert!(
                !out.windows(2).any(|w| w == b"x\n"),
                "driver({x}, {y}) printed \"x\\n\" but x <= 0"
            );
        }
    }
}

/// E7 — `if (x < 3)` false: the backwards `goto label1` is rejected, so each
/// `"loop\n"` is followed by exactly one `x`/`y` pass.
#[allow(dead_code)]
fn test_e7_x_ge_3_no_backward_goto() {
    // With x >= 3 and y small enough that x stays >= 3 for the whole run, the
    // body must never re-loop: output is a strict repetition of "loop\nx\ny\n".
    for (x, y) in [(5, 2), (10, 3), (20, 5), (40, 10)] {
        let out = assert_same(x, y);
        let prefix: Vec<u8> = b"loop\nx\ny\n".repeat(y as usize);
        assert!(
            out.starts_with(&prefix),
            "driver({x}, {y}) took the goto label1 branch while x >= 3: {:?}",
            String::from_utf8_lossy(&out)
        );
    }
}

/// E8 — `if (x == 1 && y == 4)` only partially satisfied: `goto label2` is
/// rejected, so `label1` runs and `"x\n"` precedes `"y\n"`.
#[allow(dead_code)]
fn test_e8_special_case_partially_matched() {
    for &(x, y) in &[(1, 3), (1, 5), (2, 4), (0, 4), (1, 40), (4, 4), (-1, 4)] {
        let out = assert_same(x, y);
        if x > 0 {
            assert!(
                out.starts_with(b"loop\nx\n"),
                "driver({x}, {y}) skipped label1 although the special case does not hold: {:?}",
                String::from_utf8_lossy(&out)
            );
        }
    }
    // And the positive control: (1,4) *does* skip label1.
    assert!(assert_same(1, 4).starts_with(b"loop\ny\n"));
}

// ---------------------------------------------------------------------------
// E11 / E12 — every 32-bit pattern is a valid `int`, including the values a
// foreign caller might push across the FFI boundary.
// ---------------------------------------------------------------------------

/// E11 — extreme `int` values in every pairwise combination that terminates.
/// (Combinations with `x > 0 && y < 0` never return in C — that is E9.)
#[allow(dead_code)]
fn test_e11_extreme_int_matrix() {
    let extremes = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, 3, 4, 5];
    for &x in &extremes {
        for &y in &extremes {
            if x > 0 && y < 0 {
                continue; // non-terminating in C — see E9
            }
            assert_same(x, y);
        }
    }
    // The positive extremes of `y` are safe as long as they are small enough to
    // finish; `INT_MAX` itself is covered by E10.
    for &x in &[i32::MIN, -1, 0] {
        for &y in &[1, 2, 3, 4, 100, 1000] {
            assert_same(x, y);
        }
    }
}

/// E12 — a foreign caller passing over-wide values: the ABI truncates to the
/// low 32 bits, and both `.so`s must observe the identical truncated `int`.
#[allow(dead_code)]
fn test_e12_wide_argument_truncation() {
    let patterns: [u64; 8] = [
        0x0000_0000_0000_0003,
        0xFFFF_FFFF_0000_0003, // high garbage, low 32 bits = 3
        0x1234_5678_0000_0002,
        0xDEAD_BEEF_0000_0004,
        0x0000_0000_FFFF_FFFF, // -1
        0xFFFF_FFFF_FFFF_FFFF, // -1
        0x0000_0000_8000_0000, // INT_MIN
        0xCAFE_BABE_8000_0000, // INT_MIN
    ];
    for &px in &patterns {
        for &py in &patterns {
            let x = px as u32 as i32;
            let y = py as u32 as i32;
            if x > 0 && y < 0 {
                continue; // non-terminating in C — see E9
            }
            assert_same(x, y);
        }
    }
}

// ---------------------------------------------------------------------------
// Child-process helper, used by E9 (non-termination) and E10 (bounded prefix).
// ---------------------------------------------------------------------------

const MODE: &str = "DRIVER_CHILD_MODE";
const LIB: &str = "DRIVER_CHILD_LIB";
const ENV_X: &str = "DRIVER_CHILD_X";
const ENV_Y: &str = "DRIVER_CHILD_Y";

/// Not a real test: re-executed as a child process so that inputs which never
/// return (or never stop printing) can be observed and then killed without
/// wedging the test runner.
#[test]
#[ignore = "internal child-process helper, re-executed by E9/E10"]
fn child_helper() {
    let mode = std::env::var(MODE).expect("child helper invoked without DRIVER_CHILD_MODE");
    let x: i32 = std::env::var(ENV_X).unwrap().parse().unwrap();
    let y: i32 = std::env::var(ENV_Y).unwrap().parse().unwrap();
    let which = std::env::var(LIB).unwrap();

    let f = if which == "c" { c_driver() } else { rust_driver() };
    match mode.as_str() {
        // Writes straight to the (piped) stdout so the parent can read a prefix.
        "prefix" | "spin" => unsafe { f(x, y) },
        other => panic!("unknown child mode {other:?}"),
    }
}

fn spawn_child(mode: &str, which: &str, x: i32, y: i32, stdout: Stdio) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["child_helper", "--ignored", "--exact", "--test-threads=1"])
        .env(MODE, mode)
        .env(LIB, which)
        .env(ENV_X, x.to_string())
        .env(ENV_Y, y.to_string())
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child helper")
}

/// True if the child was still running after `window`.
fn still_running_after(child: &mut std::process::Child, window: Duration) -> bool {
    let deadline = Instant::now() + window;
    while Instant::now() < deadline {
        if child.try_wait().expect("try_wait").is_some() {
            return false;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    true
}

/// E9 — `y < 0` while `x > 0`: `y == 0` is never reached, so C loops forever.
/// Both implementations must reject the input the same way, i.e. neither
/// returns. A terminating control input proves the detector is meaningful.
#[allow(dead_code)]
fn test_e9_nonterminating_both() {
    for &(x, y) in &[(1, -1), (3, -2), (7, i32::MIN + 1)] {
        let mut c = spawn_child("spin", "c", x, y, Stdio::null());
        let mut r = spawn_child("spin", "rust", x, y, Stdio::null());
        let c_hung = still_running_after(&mut c, Duration::from_millis(2500));
        let r_hung = still_running_after(&mut r, Duration::from_millis(2500));
        let _ = c.kill();
        let _ = r.kill();
        let _ = c.wait();
        let _ = r.wait();
        assert!(
            c_hung && r_hung,
            "driver({x}, {y}): C hung = {c_hung}, Rust hung = {r_hung} — they must agree"
        );
    }

    // Control: a terminating input must be seen to terminate, otherwise the
    // assertions above would be vacuous.
    let mut c = spawn_child("spin", "c", 4, 4, Stdio::null());
    let mut r = spawn_child("spin", "rust", 4, 4, Stdio::null());
    let c_hung = still_running_after(&mut c, Duration::from_millis(20_000));
    let r_hung = still_running_after(&mut r, Duration::from_millis(20_000));
    let _ = c.kill();
    let _ = r.kill();
    assert!(
        !c_hung && !r_hung,
        "control driver(4, 4) should terminate (C hung = {c_hung}, Rust hung = {r_hung})"
    );
}

/// E10 — `x == INT_MAX`, `y == 0`: terminates only after ~2^31 iterations, so
/// compare a bounded stdout PREFIX from each implementation and then kill both.
#[allow(dead_code)]
fn test_e10_huge_x_bounded_prefix() {
    const N: usize = 64 * 1024;

    fn prefix(which: &str, x: i32, y: i32) -> Vec<u8> {
        let mut child = spawn_child("prefix", which, x, y, Stdio::piped());
        let mut out = child.stdout.take().unwrap();
        let mut buf = vec![0u8; N];
        let mut filled = 0;
        while filled < N {
            match out.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) => panic!("read from {which} child: {e}"),
            }
        }
        buf.truncate(filled);
        drop(out);
        let _ = child.kill();
        let _ = child.wait();
        buf
    }

    /// The child is a libtest binary, so it emits its own "running 1 test"
    /// banner on stdout first. Drop everything before the driver's first
    /// `"loop\n"` so only library output is compared.
    fn driver_output(mut raw: Vec<u8>) -> Vec<u8> {
        let start = raw
            .windows(5)
            .position(|w| w == b"loop\n")
            .expect("child produced no driver output");
        raw.drain(..start);
        raw
    }

    for &(x, y) in &[(i32::MAX, 0), (i32::MAX, 1), (i32::MAX - 1, 0), (2, i32::MAX)] {
        let c = driver_output(prefix("c", x, y));
        let r = driver_output(prefix("rust", x, y));
        let n = c.len().min(r.len());
        assert!(
            n >= 32 * 1024,
            "too little output to compare for ({x},{y}): C {} bytes, Rust {} bytes",
            c.len(),
            r.len()
        );
        assert!(
            c[..n] == r[..n],
            "driver({x}, {y}) prefixes diverge at byte {}",
            c[..n]
                .iter()
                .zip(r[..n].iter())
                .position(|(a, b)| a != b)
                .unwrap()
        );
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries required by Phase C even though the table has no rows for
// them: `driver` takes no pointers and no enums, so there is nothing to pass a
// NULL or an out-of-range variant for; the closest analogue is the full set of
// int boundary values, asserted here in one place.
// ---------------------------------------------------------------------------

/// One step past every documented boundary in either direction.
#[allow(dead_code)]
fn test_boundaries_one_step_past() {
    let interesting = [
        i32::MIN,
        i32::MIN + 1,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        63,
        64,
        65,
    ];
    for &x in &interesting {
        for &y in &interesting {
            if x > 0 && y < 0 {
                continue; // E9
            }
            assert_same(x, y);
        }
    }
}

/// The `driver` symbol must be resolvable from both `.so`s (a missing export is
/// itself a rejection difference).
#[allow(dead_code)]
fn test_symbol_resolves_in_both() {
    let _c = c_driver();
    let _r = rust_driver();
    assert_same_all([(0, 0), (1, 1)]);
}

// ---------------------------------------------------------------------------
// Aggregator — see the note in configs.rs: fd 1 is process-global, so the rows
// must run sequentially in a single test.
// ---------------------------------------------------------------------------

macro_rules! rows {
    ($($f:ident),* $(,)?) => {
        #[test]
        fn phase_c_all_error_rows() {
            $(
                eprintln!("ERRORS.md row: {}", stringify!($f));
                $f();
            )*
            eprintln!("Phase C: all ERRORS.md rows passed");
        }
    };
}

rows!(
    test_e1_zero_zero_no_output,
    test_e2_both_negative_no_output,
    test_e3_int_min_no_output,
    test_e4_mixed_nonpositive_no_output,
    test_e5_y_zero_continue,
    test_e6_x_nonpositive,
    test_e7_x_ge_3_no_backward_goto,
    test_e8_special_case_partially_matched,
    test_e11_extreme_int_matrix,
    test_e12_wide_argument_truncation,
    test_boundaries_one_step_past,
    test_symbol_resolves_in_both,
    // Child-process rows last: they are the slowest.
    test_e10_huge_x_bounded_prefix,
    test_e9_nonterminating_both,
);
