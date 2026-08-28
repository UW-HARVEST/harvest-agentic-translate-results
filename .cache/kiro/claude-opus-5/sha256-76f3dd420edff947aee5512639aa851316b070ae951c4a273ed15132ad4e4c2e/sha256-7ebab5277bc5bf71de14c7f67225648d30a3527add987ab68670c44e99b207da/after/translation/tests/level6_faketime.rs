//! Level 6: timestamp-dependent paths under a controlled clock.
//!
//! `get_computation_timestamp` is `time() >> 29`, so with the real clock it is
//! currently a small single-digit value. That masks any divergence in the
//! `% 100` time modifier inside `mathop`. This suite re-executes itself in a
//! child process with libc's `time()` interposed via `LD_PRELOAD`, so both
//! libraries see the same chosen epoch and large shifted timestamps become
//! reachable.
mod common;

use common::{both, capture_stdout, global_lock};
use std::path::PathBuf;
use std::process::Command;

const CHILD_ENV: &str = "MATHOP_FAKETIME_CHILD";
const FAKE_TIME_ENV: &str = "MATHOP_FAKE_TIME";

/// Epochs chosen so `epoch >> 29` lands on values that distinguish `% 100` from
/// smaller moduli, and that exercise 0, small, and large shifted stamps.
fn epochs() -> Vec<i64> {
    const S: i64 = 1 << 29;
    vec![
        0,
        1,
        S - 1,          // >> 29 == 0
        S,              // == 1
        3 * S,          // == 3  (roughly "now")
        13 * S,         // == 13 -> distinguishes % 100 from % 10
        99 * S,         // == 99 -> last value below 100
        100 * S,        // == 100 -> wraps the modifier back to 0
        137 * S,        // == 137 -> modifier 37
        1234 * S + 7,   // == 1234 -> modifier 34
        99_999 * S,     // large
    ]
}

fn build_preload_lib() -> PathBuf {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_time.c");
    assert!(src.exists(), "missing {src:?}");
    let out = std::env::temp_dir().join(format!("mathop_fake_time_{}.so", std::process::id()));
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .expect("failed to invoke cc");
    assert!(status.success(), "cc failed to build the time interposer");
    out
}

/// Re-runs the named test in a child process with the interposer preloaded.
fn run_in_child(test_name: &str, epoch: i64) {
    let preload = build_preload_lib();
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args([test_name, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .env(FAKE_TIME_ENV, epoch.to_string())
        .env("LD_PRELOAD", &preload)
        .output()
        .expect("failed to spawn child test process");
    let _ = std::fs::remove_file(&preload);
    assert!(
        out.status.success(),
        "child run for epoch {epoch} failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn in_child() -> bool {
    std::env::var(CHILD_ENV).is_ok()
}

fn fake_epoch() -> i64 {
    std::env::var(FAKE_TIME_ENV)
        .expect("child must be given a fake epoch")
        .parse()
        .expect("bad fake epoch")
}

/// Confirms the interposer is actually in effect; otherwise the whole suite
/// would silently degrade to testing the real clock again.
fn assert_interposed() {
    let b = both();
    let epoch = fake_epoch();
    let expected = epoch >> 29;
    let got = unsafe { (b.c.get_computation_timestamp)() };
    assert_eq!(
        got, expected,
        "LD_PRELOAD interposition of time() is not in effect (epoch {epoch})"
    );
}

// ---------------------------------------------------------------------------
// Parent-side drivers
// ---------------------------------------------------------------------------

#[test]
fn timestamp_matches_under_faked_clock() {
    if in_child() {
        assert_interposed();
        let b = both();
        let expected = fake_epoch() >> 29;
        for _ in 0..8 {
            let (c, r) = unsafe {
                (
                    (b.c.get_computation_timestamp)(),
                    (b.rust.get_computation_timestamp)(),
                )
            };
            assert_eq!(c, r, "get_computation_timestamp differs at faked epoch");
            assert_eq!(c, expected, "unexpected shift result");
        }
        return;
    }
    for e in epochs() {
        run_in_child("timestamp_matches_under_faked_clock", e);
    }
}

#[test]
fn mathop_matches_under_faked_clock() {
    if in_child() {
        assert_interposed();
        let _g = global_lock();
        let b = both();
        // Modest operands: the point here is the timestamp path, and the
        // arithmetic must stay clear of signed overflow.
        let params: [(i32, i32, i32, i32); 24] = [
            (0, 0, 0, 0),
            (1, 2, 1, 1),
            (7, 3, 2, 2),
            (7, 3, 3, 3),
            (7, 3, 4, 4),
            (7, 0, 3, 3),
            (7, 0, 4, 4),
            (-7, 3, 1, 0),
            (49, 5, 0, 1),
            (50, 5, 1, 2),
            (53, 5, 2, 3),
            (54, 5, 3, 4),
            (-1, -1, -1, -1),
            (-5, -5, -5, -5),
            (100, 25, 4, 3),
            (1000, 7, 2, 5),
            (12345, 100, 3, 6),
            (-12345, 100, 4, 7),
            (999, 0, 3, 0),
            (999, 0, 4, 0),
            (31, 17, 8, 9),
            (-31, -17, -8, -9),
            (65, 66, 11, 12),
            (128, 129, 13, 14),
        ];
        for (p1, p2, p3, p4) in params {
            let (cr, cout) = capture_stdout(|| unsafe { (b.c.mathop)(p1, p2, p3, p4) });
            let (rr, rout) = capture_stdout(|| unsafe { (b.rust.mathop)(p1, p2, p3, p4) });
            assert_eq!(
                cr, rr,
                "mathop({p1},{p2},{p3},{p4}) return differs at epoch {}",
                fake_epoch()
            );
            assert_eq!(
                String::from_utf8_lossy(&cout),
                String::from_utf8_lossy(&rout),
                "mathop({p1},{p2},{p3},{p4}) stdout differs at epoch {}",
                fake_epoch()
            );
            assert_eq!(cout, rout);
        }
        return;
    }
    for e in epochs() {
        run_in_child("mathop_matches_under_faked_clock", e);
    }
}

#[test]
fn history_timestamps_match_under_faked_clock() {
    if in_child() {
        assert_interposed();
        let b = both();
        let expected = fake_epoch() >> 29;
        let mut dumps = Vec::new();
        for api in [&b.c, &b.rust] {
            unsafe {
                let mut history: *mut common::ComputationResult = std::ptr::null_mut();
                let mut count: std::ffi::c_int = 0;
                for i in 0..10 {
                    (api.perform_computation_with_history)(
                        i * 7,
                        3,
                        (i % 5) + 1,
                        &mut history,
                        &mut count,
                    );
                }
                let slots: &[common::ComputationResult] = std::slice::from_raw_parts(history, 10);
                for (i, s) in slots.iter().enumerate() {
                    assert_eq!(s.timestamp, expected, "slot {i} stamp at faked epoch");
                }
                dumps.push(common::raw_bytes(history, 10));
                libc_free(history as *mut std::ffi::c_void);
            }
        }
        assert_eq!(dumps[0], dumps[1], "history bytes differ at faked epoch");
        return;
    }
    for e in epochs() {
        run_in_child("history_timestamps_match_under_faked_clock", e);
    }
}

extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut std::ffi::c_void);
}
