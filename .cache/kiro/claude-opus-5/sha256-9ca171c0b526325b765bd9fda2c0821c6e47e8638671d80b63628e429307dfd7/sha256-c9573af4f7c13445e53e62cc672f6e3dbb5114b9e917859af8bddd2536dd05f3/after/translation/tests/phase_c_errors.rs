//! Phase C — error / rejection-path differential tests, one per `ERRORS.md` row.
//!
//! `driver` is `void` and validates nothing, so "same error" means: the same
//! rejection *behaviour* — the same (often empty) stdout byte stream, the same
//! suppressed blocks, and, for the one divergent input class, the same
//! non-termination with a byte-identical output prefix.

mod support;

use std::ffi::CString;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::{assert_same, assert_same_all, rng_for, run, terminates, Impl};

const N: usize = 64;

/// Every row that expects the loop guard (`src/driver.c:30`) to be false must
/// produce a completely empty stdout.
#[track_caller]
fn assert_rejected_silently(row: &str, x: i32, y: i32) {
    let out = assert_same(row, x, y);
    assert!(
        out.is_empty(),
        "{row}: driver({x}, {y}) should write nothing, wrote {:?}",
        String::from_utf8_lossy(&out)
    );
}

// ------------------------------------------------------- rows 1..6: guard false

#[test]
fn err_row_01_guard_zero_zero() {
    assert_rejected_silently("ERRORS row 1 (x==0 && y==0)", 0, 0);
}

#[test]
fn err_row_02_guard_both_negative() {
    let row = "ERRORS row 2 (x<0 && y<0)";
    let mut rng = rng_for(row);
    for _ in 0..N {
        let x = rng.range(-100_000, -1);
        let y = rng.range(-100_000, -1);
        assert_rejected_silently(row, x, y);
    }
}

#[test]
fn err_row_03_guard_negx_zeroy() {
    let row = "ERRORS row 3 (x<0 && y==0)";
    let mut rng = rng_for(row);
    for _ in 0..N {
        let x = rng.range(-100_000, -1);
        assert_rejected_silently(row, x, 0);
    }
}

#[test]
fn err_row_04_guard_zerox_negy() {
    let row = "ERRORS row 4 (x==0 && y<0)";
    let mut rng = rng_for(row);
    for _ in 0..N {
        let y = rng.range(-100_000, -1);
        assert_rejected_silently(row, 0, y);
    }
}

#[test]
fn err_row_05_guard_int_min_both() {
    assert_rejected_silently("ERRORS row 5 (INT_MIN, INT_MIN)", i32::MIN, i32::MIN);
}

#[test]
fn err_row_06_guard_extreme_mixed() {
    let row = "ERRORS row 6 (extreme mixed, guard false)";
    for (x, y) in [(i32::MIN, 0), (0, i32::MIN), (i32::MIN, -1), (-1, i32::MIN)] {
        assert_rejected_silently(row, x, y);
    }
}

// ------------------------------------- row 7: `if (x > 0)` false, label1 skipped

#[test]
fn err_row_07_skip_label1_x_not_positive() {
    let row = "ERRORS row 7 (x<=0 && y>0 — label1 suppressed)";
    let mut rng = rng_for(row);
    let mut n = 0;
    for _ in 0..N {
        let x = rng.range(-500, 0);
        let y = rng.range(1, 200);
        let out = assert_same(row, x, y);
        let text = String::from_utf8(out).expect("ascii");
        assert!(
            !text.lines().any(|l| l == "x"),
            "{row}: driver({x}, {y}) emitted an \"x\" line although x <= 0"
        );
        // One "loop" (the guard is only re-tested after the y-drain finishes via
        // the `continue`), then exactly `y` "y" lines.
        assert_eq!(
            text.lines().filter(|l| *l == "y").count(),
            y as usize,
            "{row}: driver({x}, {y}) wrong number of y lines"
        );
        n += 1;
    }
    assert_eq!(n, N);
}

// ------------------------------ row 8: `if (y == 0) continue;` on the first pass

#[test]
fn err_row_08_reject_y_zero_continue() {
    let row = "ERRORS row 8 (x>0 && y==0 — y-block rejected every pass)";
    let mut rng = rng_for(row);
    let mut n = 0;
    for _ in 0..N {
        let x = rng.range(1, 300);
        let out = assert_same(row, x, 0);
        let text = String::from_utf8(out).expect("ascii");
        assert!(
            !text.lines().any(|l| l == "y"),
            "{row}: driver({x}, 0) emitted a \"y\" line although y == 0"
        );
        assert_eq!(
            text.lines().filter(|l| *l == "loop").count(),
            x as usize,
            "{row}: driver({x}, 0) wrong number of outer passes"
        );
        n += 1;
    }
    assert_eq!(n, N);
}

// ------------------------- row 9: the `continue` fires only after y drains to 0

#[test]
fn err_row_09_reject_y_zero_after_drain() {
    let row = "ERRORS row 9 (y reaches 0 mid-run)";
    let mut rng = rng_for(row);
    let inputs: Vec<(i32, i32)> =
        (0..N).map(|_| (rng.range(1, 200), rng.range(1, 200))).collect();
    assert_same_all(row, inputs);
}

// ---------------------------- row 10: `if (x < 3)` false, back-edge not taken

#[test]
fn err_row_10_no_backedge_x_ge_3() {
    let row = "ERRORS row 10 (x>=3 && y>0 — back-edge declined)";
    let mut rng = rng_for(row);
    let mut n = 0;
    for _ in 0..N {
        let x = rng.range(3, 300);
        let y = rng.range(1, 200);
        assert_same(row, x, y);
        n += 1;
    }
    // Pin the exact boundary: x == 3 declines, x == 2 takes the back-edge.
    let at3 = String::from_utf8(assert_same(row, 3, 1)).unwrap();
    let at2 = String::from_utf8(assert_same(row, 2, 1)).unwrap();
    assert_ne!(at3, at2, "{row}: x==3 and x==2 must differ at the S5 boundary");
    assert_eq!(n, N);
}

// ---------------------- row 11: `x == 1 && y == 4` forward goto, applied once

#[test]
fn err_row_11_goto_label2_skip_once() {
    let row = "ERRORS row 11 (x==1 && y==4 — skip label1 once)";
    let out = assert_same(row, 1, 4);
    let text = String::from_utf8(out).expect("ascii");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(&lines[..2], &["loop", "y"], "{row}: label1 was not skipped on entry");
    assert!(lines.contains(&"x"), "{row}: the skip must not persist");
    // Neighbours that must NOT take the skip.
    for (x, y) in [(1, 3), (1, 5), (2, 4), (0, 4), (4, 4), (-1, 4)] {
        let t = String::from_utf8(assert_same(row, x, y)).unwrap();
        let l: Vec<&str> = t.lines().collect();
        if x > 0 {
            assert_eq!(&l[..2], &["loop", "x"], "{row}: driver({x}, {y}) must not skip label1");
        }
    }
}

// --------------------------------------- row 12: the non-terminating input class

/// Child-side worker for `err_row_12_nonterminating_x_pos_y_neg`.
///
/// Ignored by default; the parent re-executes this same test binary with
/// `--ignored --exact` and the `DRIVER_HANG_*` environment variables set. The
/// child points fd 1 at the file it is told to use (so the libtest harness's own
/// chatter, which went to the inherited/null stdout, cannot pollute it) and then
/// calls `driver`, which never returns.
#[test]
#[ignore = "spawned as a child process by err_row_12_nonterminating_x_pos_y_neg"]
fn hang_child_worker() {
    let (which, x, y, out) = match (
        std::env::var("DRIVER_HANG_IMPL"),
        std::env::var("DRIVER_HANG_X"),
        std::env::var("DRIVER_HANG_Y"),
        std::env::var("DRIVER_HANG_OUT"),
    ) {
        (Ok(i), Ok(x), Ok(y), Ok(o)) => (
            i,
            x.parse::<i32>().expect("DRIVER_HANG_X"),
            y.parse::<i32>().expect("DRIVER_HANG_Y"),
            o,
        ),
        // Not spawned as a worker (e.g. someone ran `cargo test -- --ignored`
        // directly): do nothing.
        _ => return,
    };

    let which = match which.as_str() {
        "c" => Impl::C,
        "rust" => Impl::Rust,
        other => panic!("unknown DRIVER_HANG_IMPL {other:?}"),
    };

    let path = CString::new(out).unwrap();
    unsafe {
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o600);
        assert!(fd >= 0, "child: open failed");
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(fd, 1) >= 0, "child: dup2 failed");
        libc::close(fd);
    }

    let p = support::pair();
    let f = match which {
        Impl::C => &p.c.driver,
        Impl::Rust => &p.rust.driver,
    };
    unsafe { f(x, y) };
    unreachable!("driver({x}, {y}) returned but this input class must diverge");
}

/// Bytes of output prefix each implementation must agree on.
const PREFIX: usize = 16 * 1024;

struct HangResult {
    prefix: Vec<u8>,
    still_running: bool,
}

fn run_hanging(which: Impl, x: i32, y: i32) -> HangResult {
    let tag = match which {
        Impl::C => "c",
        Impl::Rust => "rust",
    };
    let out_path = std::env::temp_dir().join(format!(
        "driver-hang-{}-{}-{}-{}.out",
        std::process::id(),
        tag,
        x,
        y.unsigned_abs()
    ));
    let _ = std::fs::remove_file(&out_path);

    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(exe)
        .args(["--exact", "hang_child_worker", "--ignored", "--test-threads=1"])
        .env("DRIVER_HANG_IMPL", tag)
        .env("DRIVER_HANG_X", x.to_string())
        .env("DRIVER_HANG_Y", y.to_string())
        .env("DRIVER_HANG_OUT", &out_path)
        .env("DRIVER_C_SO", support::c_so_path())
        .env("DRIVER_RUST_SO", support::rust_so_path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn hang worker");

    // Wait for enough output, or for the child to (unexpectedly) exit.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut exited_early = false;
    loop {
        let len = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
        if len as usize >= PREFIX {
            break;
        }
        if let Some(_status) = child.try_wait().expect("try_wait") {
            exited_early = true;
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let still_running = child.try_wait().expect("try_wait").is_none();
    let _ = child.kill();
    let _ = child.wait();

    let mut bytes = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);
    assert!(
        !exited_early || bytes.len() >= PREFIX,
        "{tag}: driver({x}, {y}) exited instead of diverging (produced {} bytes)",
        bytes.len()
    );
    bytes.truncate(PREFIX);
    HangResult { prefix: bytes, still_running }
}

#[test]
fn err_row_12_nonterminating_x_pos_y_neg() {
    let row = "ERRORS row 12 (x>0 && y<0 — never returns)";
    // Representatives of every x class that can reach the divergent path.
    for (x, y) in [(1, -1), (2, -1), (3, -1), (4, -7), (9, -3), (1, i32::MIN + 1)] {
        assert!(!terminates(x, y), "{row}: bad test input");

        let c = run_hanging(Impl::C, x, y);
        let r = run_hanging(Impl::Rust, x, y);

        assert!(c.still_running, "{row}: C driver({x}, {y}) terminated; expected divergence");
        assert!(r.still_running, "{row}: Rust driver({x}, {y}) terminated; expected divergence");
        assert_eq!(
            c.prefix.len(),
            PREFIX,
            "{row}: C driver({x}, {y}) produced too little output to compare"
        );
        assert_eq!(
            r.prefix.len(),
            PREFIX,
            "{row}: Rust driver({x}, {y}) produced too little output to compare"
        );
        if c.prefix != r.prefix {
            let at = c.prefix.iter().zip(&r.prefix).position(|(a, b)| a != b).unwrap();
            let mut err = Vec::new();
            let _ = writeln!(
                err,
                "{row}: driver({x}, {y}) output prefixes diverge at byte {at}"
            );
            panic!("{}", String::from_utf8_lossy(&err));
        }
    }
}

// ------------------------------------------------- rows 13..17: N/A boundaries
//
// Recorded here so the "generic C-API boundaries" checklist is executed rather
// than merely asserted in prose. `void driver(int, int)` has no pointer, no
// length, no enum, and no return value, so the nearest reachable analogues are
// the extreme scalar values and the full one-step-past-boundary neighbourhood of
// every constant the C compares against (0, 1, 3, 4).

#[test]
fn err_rows_13_to_17_generic_boundaries() {
    let row = "ERRORS rows 13-17 (generic FFI boundaries: extremes + off-by-one)";
    // Every constant in the C source and its immediate neighbours, plus the
    // representable extremes of `int` — i.e. the whole "out-of-range value
    // crossing the FFI boundary" class for a plain-`int` API.
    let interesting = [
        i32::MIN,
        i32::MIN + 1,
        -5,
        -4,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        1_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut compared = 0;
    let mut divergent = 0;
    for &x in &interesting {
        for &y in &interesting {
            if !terminates(x, y) {
                divergent += 1;
                continue;
            }
            // `x == i32::MAX` with a terminating y would emit ~2^31 lines; that
            // magnitude class is covered by CONFIGS row 33. Skip only those.
            if x > 100_000 {
                continue;
            }
            if y > 100_000 {
                continue;
            }
            assert_same(row, x, y);
            compared += 1;
        }
    }
    assert!(compared >= 100, "{row}: only {compared} pairs compared");
    assert!(divergent > 0, "{row}: expected some divergent pairs to be classified");
}

/// The Rust export must be reachable and callable under the exact C ABI for the
/// full `int` domain — including values that would be "invalid enum" in a
/// richer API. Here we simply confirm the symbol is the C ABI one and that a
/// no-op call pair agrees, catching ABI/mangling regressions directly.
#[test]
fn err_abi_export_is_callable_as_c() {
    let row = "ERRORS rows 15-17 (exported C ABI symbol)";
    let a = run(Impl::C, 0, 0);
    let b = run(Impl::Rust, 0, 0);
    assert_eq!(a, b, "{row}: exported symbols disagree on the trivial call");
    assert!(a.is_empty(), "{row}: trivial call must be silent");
}
