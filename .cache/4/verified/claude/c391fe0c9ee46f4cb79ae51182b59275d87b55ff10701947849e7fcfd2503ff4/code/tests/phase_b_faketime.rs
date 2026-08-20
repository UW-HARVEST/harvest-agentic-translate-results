// Phase B — CONFIGS.md axis L: the `time_t` values that reach
// `get_computation_timestamp` (`time() >> 29`) and `mathop`'s
// `(int)(computation_time % 100)` modifier.
//
// Wall-clock time pins that value to a single number, which hides whole classes
// of behaviour: the *arithmetic* right shift of a negative `time_t`, the `% 100`
// (rather than `% 10`) modifier, the narrowing `(int)` cast of a large
// remainder, and `printf("%ld")` of a big value. This test re-executes itself
// with an LD_PRELOAD shim (tests/support/faketime.c) that makes BOTH libraries'
// `time()` return a chosen value, then compares them exactly as the other
// phases do.
//
// If a C compiler is unavailable the test skips (loudly) rather than failing.

mod common;

use common::*;
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;

/// The `time()` values driven through both libraries.
const FAKE_TIMES: &[i64] = &[
    0,
    1,
    536_870_911,          // 2^29 - 1        -> ts = 0
    536_870_912,          // 2^29            -> ts = 1
    1_787_000_000,        // "now"           -> ts = 3
    5_368_709_120,        // 10 * 2^29       -> ts = 10  (% 100 = 10, % 10 = 0)
    24_159_191_040,       // 45 * 2^29       -> ts = 45
    73_551_675_392,       // 137 * 2^29      -> ts = 137 (% 100 = 37)
    -1,                   // arithmetic shift-> ts = -1
    -536_870_912,         // -2^29           -> ts = -1
    -536_870_913,         //                 -> ts = -2
    -73_551_675_392,      //                 -> ts = -137
    i64::MAX,             //                 -> ts = 17179869183 (% 100 = 83)
    i64::MIN,             //                 -> ts = -17179869184 (% 100 = -84)
];

fn shim_path() -> PathBuf {
    std::env::temp_dir().join(format!("libfaketime_{}.so", std::process::id()))
}

// ---------------------------------------------------------------------------
// Parent side: build the shim, re-exec this test once per fake clock value.
// ---------------------------------------------------------------------------

#[test]
fn axis_l_timestamp_values() {
    if std::env::var_os("FAKETIME_CHILD").is_some() {
        child_body();
        return;
    }

    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("support")
        .join("faketime.c");
    assert!(src.exists(), "missing fixture {}", src.display());
    let shim = shim_path();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let built = Command::new(&cc)
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&shim)
        .arg(&src)
        .status();
    match built {
        Ok(s) if s.success() => {}
        other => {
            eprintln!(
                "SKIPPING axis-L timestamp sweep: cannot build the LD_PRELOAD shim \
                 with {cc:?} ({other:?})"
            );
            return;
        }
    }

    let exe = std::env::current_exe().expect("current_exe");
    for &t in FAKE_TIMES {
        let out = Command::new(&exe)
            .args(["--exact", "axis_l_timestamp_values", "--nocapture"])
            .env("FAKETIME_CHILD", "1")
            .env("FAKE_TIME", t.to_string())
            .env("LD_PRELOAD", &shim)
            .output()
            .expect("re-exec self");
        if !out.status.success() {
            panic!(
                "FAKE_TIME={t} child failed ({:?})\n--- stdout ---\n{}\n--- stderr ---\n{}",
                out.status,
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    let _ = std::fs::remove_file(&shim);
}

// ---------------------------------------------------------------------------
// Child side: runs with the shim active, so `time()` is `$FAKE_TIME`.
// ---------------------------------------------------------------------------

fn child_body() {
    let want: i64 = std::env::var("FAKE_TIME")
        .expect("FAKE_TIME")
        .parse()
        .expect("i64");
    let (c, r) = both();

    // The shim must actually be in effect, otherwise this test would silently
    // degrade into a duplicate of the wall-clock one.
    assert_eq!(
        now(),
        want,
        "the LD_PRELOAD shim is not active — refusing to report a vacuous pass"
    );
    let expected_ts = want >> 29;

    // ---- get_computation_timestamp -----------------------------------------
    let cv = unsafe { (c.get_computation_timestamp)() };
    let rv = unsafe { (r.get_computation_timestamp)() };
    assert_eq!(cv, rv, "get_computation_timestamp @ time={want}");
    assert_eq!(
        cv, expected_ts,
        "time()>>29 must be an arithmetic shift @ time={want}"
    );

    // ---- the timestamp written into history records -------------------------
    let mut cbuf = [ComputationResult::default(); 10];
    let mut rbuf = [ComputationResult::default(); 10];
    let mut ch = cbuf.as_mut_ptr();
    let mut rh = rbuf.as_mut_ptr();
    let mut cn: c_int = 0;
    let mut rn: c_int = 0;
    for op in 1..=5 {
        let a = 100 * op;
        let b = op + 1;
        let x = unsafe { (c.perform_computation_with_history)(a, b, op, &mut ch, &mut cn) };
        let y = unsafe { (r.perform_computation_with_history)(a, b, op, &mut rh, &mut rn) };
        assert_eq!(x, y, "record op={op} @ time={want}");
    }
    assert_eq!(cn, rn);
    assert_eq!(cbuf, rbuf, "records @ time={want}");
    for i in 0..cn as usize {
        assert_eq!(
            cbuf[i].timestamp, expected_ts,
            "record {i} timestamp @ time={want}"
        );
    }

    // ---- mathop: return value AND the four printed lines -------------------
    // (`% 100`, the narrowing `(int)` cast and `printf("%ld")` all in play.)
    let mut rng = Rng::new(0xFACE_71ED_u64 ^ (want as u64));
    let mut quads: Vec<(c_int, c_int, c_int, c_int)> = vec![
        (49, 3, 0, 0),
        (50, 7, 1, 1),
        (51, -9, 2, 2),
        (52, 11, 3, 3),
        (53, 0, 4, 4),
        (0, 0, -1, -1),
        (i32::MAX, 1, -2, -5),
        (i32::MIN, 3, -4, 3),
    ];
    for _ in 0..64 {
        quads.push((
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        ));
    }
    for (p1, p2, p3, p4) in quads {
        if mathop_is_ub(p1, p2, p3, p4) {
            continue;
        }
        let _g = serial();
        let mut cr: c_int = 0;
        let cout = capture_stdout(|| cr = unsafe { (c.mathop)(p1, p2, p3, p4) });
        let mut rr: c_int = 0;
        let rout = capture_stdout(|| rr = unsafe { (r.mathop)(p1, p2, p3, p4) });
        assert_eq!(
            cr, rr,
            "mathop({p1},{p2},{p3},{p4}) @ time={want}: C={cr} Rust={rr}"
        );
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "mathop({p1},{p2},{p3},{p4}) @ time={want}: stdout"
        );
        // The printed timestamp is `(long)computation_time`, not the raw clock.
        let head = format!("Computation performed at timestamp: {expected_ts}\n");
        assert!(
            String::from_utf8_lossy(&cout).starts_with(&head),
            "@ time={want}: expected transcript to start with {head:?}, got {:?}",
            String::from_utf8_lossy(&cout)
        );
        // The modifier really is `% 100` (and keeps the sign of the dividend).
        let modifier = (expected_ts % 100) as i32;
        assert_eq!(
            modifier as i64,
            expected_ts % 100,
            "sanity: modifier fits in an int"
        );
    }
}
