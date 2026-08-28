// Phase B (continued) -- CONFIGS.md rows C64..C69: the `time(NULL)` axis.
//
// `get_modified_time` does `time_t current = time(NULL); current >>= 29;`.  With
// the real clock that base value is a constant (`3` until 2038), which leaves the
// following untested:
//
//   * arithmetic vs logical `>> 29` (indistinguishable while the clock is > 0);
//   * the `current + offset` addition near the `time_t` extremes;
//   * `hash_time_value` over `time_t` values with a different byte layout;
//   * the `Modified time: %ld` field for negative values.
//
// This file therefore re-runs the whole `get_modified_time` / `hash_time_value` /
// `modeselect` differential suite inside a child process that has an LD_PRELOAD
// `time()` interposer (examples/faketime.rs) forcing a chosen clock value.
mod common;

use common::*;
use std::process::Command;

const ENV_VAR: &str = "DIFFTEST_FAKE_TIME";

/// The clock values the child is run with.
fn fake_times() -> Vec<i64> {
    let mut v = vec![
        0i64,
        1,
        -1,
        2,
        -2,
        536_870_911,  // 2^29 - 1
        536_870_912,  // 2^29
        536_870_913,
        -536_870_911,
        -536_870_912,
        -536_870_913,
        1_756_000_000, // roughly "now"
        2_147_483_647,
        2_147_483_648,
        -2_147_483_648,
        4_294_967_296,
        i64::MAX,
        i64::MIN,
        i64::MIN + 1,
    ];
    let mut rng = Rng::new(SEED ^ 0xFACE);
    for _ in 0..6 {
        v.push(rng.next_i64());
    }
    v
}

// ===========================================================================
// Driver: spawn this same test binary as a worker, once per clock value.
// ===========================================================================

#[test]
fn faketime_driver_runs_worker_for_every_clock_value() {
    if std::env::var_os(ENV_VAR).is_some() {
        return; // we *are* the worker
    }
    let exe = std::env::current_exe().expect("current_exe");
    let preload = faketime_so_path();
    eprintln!("faketime interposer: {}", preload.display());

    // Sanity: the interposer must really override libc's `time`.
    let probe = Command::new(&exe)
        .args(["--exact", "faketime_worker_probe", "--nocapture", "--test-threads=1"])
        .env(ENV_VAR, "-536870913")
        .env("LD_PRELOAD", &preload)
        .output()
        .expect("spawn probe");
    assert!(
        probe.status.success(),
        "faketime interposition probe failed\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&probe.stdout),
        String::from_utf8_lossy(&probe.stderr)
    );

    for t in fake_times() {
        let out = Command::new(&exe)
            .args(["--exact", "faketime_worker", "--nocapture", "--test-threads=1"])
            .env(ENV_VAR, t.to_string())
            .env("LD_PRELOAD", &preload)
            .output()
            .unwrap_or_else(|e| panic!("spawning worker for clock {t}: {e}"));
        assert!(
            out.status.success(),
            "faketime worker FAILED for clock value {t}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

// ===========================================================================
// Probe: verifies the interposer is actually in effect (otherwise every worker
// run would silently degrade into a duplicate of the normal test suite).
// ===========================================================================

#[test]
fn faketime_worker_probe() {
    let Some(raw) = std::env::var_os(ENV_VAR) else {
        return;
    };
    let want: i64 = raw.to_string_lossy().parse().expect("clock value");
    let l = libs();
    // get_modified_time(0, 0) == (time(NULL) >> 29) exactly.
    let c = unsafe { (l.c.get_modified_time)(0, 0) };
    let rs = unsafe { (l.rs.get_modified_time)(0, 0) };
    eq_i64("faketime/probe", want, c, rs);
    assert_eq!(
        c,
        want >> 29,
        "LD_PRELOAD time() interposition is NOT in effect: get_modified_time(0,0) = {c}, \
         expected {} for clock {want}",
        want >> 29
    );
    // And the shift must be ARITHMETIC: for a negative clock the base is negative.
    if want < 0 {
        assert!(
            c < 0,
            "faketime/probe: clock {want} gave base {c}; a logical >> 29 would be positive"
        );
    }
}

// ===========================================================================
// Worker: the actual differential rows, run under the forced clock.
// ===========================================================================

#[test]
fn faketime_worker() {
    let Some(raw) = std::env::var_os(ENV_VAR) else {
        return;
    };
    let clock: i64 = raw.to_string_lossy().parse().expect("clock value");
    let base = clock >> 29;
    let l = libs();
    let mut rng = Rng::new(SEED ^ (clock as u64));

    // ---- C64: get_modified_time under the forced clock ----------------------
    let mut offsets: Vec<(i32, i32)> = vec![
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (24855, 596523),
        (-24855, -596523),
        (24856, 0),
        (-24856, 0),
        (0, 596524),
        (0, -596524),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
    ];
    for _ in 0..1500 {
        offsets.push((rng.next_i32(), rng.next_i32()));
    }
    for _ in 0..500 {
        offsets.push((rng.range_i32(-24855, 24855), rng.range_i32(-23, 23)));
    }
    for &(d, h) in &offsets {
        let c = unsafe { (l.c.get_modified_time)(d, h) };
        let rs = unsafe { (l.rs.get_modified_time)(d, h) };
        eq_i64("C64", (clock, d, h), c, rs);
        let expect = base.wrapping_add(d.wrapping_mul(86400).wrapping_add(h.wrapping_mul(3600)) as i64);
        assert_eq!(
            c, expect,
            "C64: clock={clock} base={base} ({d},{h}) -> {c}, expected {expect}"
        );

        // ---- C65: composed get_modified_time -> hash_time_value ------------
        let hc = unsafe { (l.c.hash_time_value)(c) };
        let hrs = unsafe { (l.rs.hash_time_value)(rs) };
        eq_int("C65", (clock, d, h, c), hc, hrs);
    }

    // ---- C66: modeselect under the forced clock ----------------------------
    // Full 4 x 5 cross product of mode index and complexity level.
    for mi in 0..4i32 {
        for cl in 0..5i32 {
            for i in 0..6 {
                let ms = mi + 4 * rng.range_i32(0, 1_000_000);
                let cx = cl + 5 * rng.range_i32(0, 1_000_000);
                let (to, sd) = match i {
                    0 => (0, 0),
                    1 => (1, 1),
                    2 => (-1, -1),
                    3 => (i32::MAX, i32::MIN),
                    4 => (rng.next_i32(), rng.next_i32()),
                    _ => (rng.range_i32(-30000, 30000), rng.range_i32(-100, 100)),
                };
                let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(ms, to, cx, sd) });
                let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(ms, to, cx, sd) });
                eq_int("C66", (clock, ms, to, cx, sd), cr, rr);
                eq_bytes("C66", (clock, ms, to, cx, sd), &cout, &rout);
            }
        }
    }

    // ---- C67: the `Modified time: %ld` field for a negative clock ----------
    let (_, cout) = capture(|| unsafe { (l.c.modeselect)(0, 0, 0, 0) });
    let (_, rout) = capture(|| unsafe { (l.rs.modeselect)(0, 0, 0, 0) });
    eq_bytes("C67", clock, &cout, &rout);
    let text = String::from_utf8_lossy(&cout).to_string();
    let line = text
        .lines()
        .find(|l| l.starts_with("Modified time: "))
        .unwrap_or_else(|| panic!("C67: no 'Modified time' line in {}", show(&cout)));
    assert!(
        line.starts_with(&format!("Modified time: {base},")),
        "C67: clock={clock} expected base {base} in {line:?}"
    );

    // ---- C68: negative mode_selector that is a multiple of 4 ---------------
    for ms in [-4i32, -8, i32::MIN] {
        let (cr, cout) = capture(|| unsafe { (l.c.modeselect)(ms, 3, 2, 1) });
        let (rr, rout) = capture(|| unsafe { (l.rs.modeselect)(ms, 3, 2, 1) });
        eq_int("C68", (clock, ms), cr, rr);
        eq_bytes("C68", (clock, ms), &cout, &rout);
    }

    // ---- C69: hash_time_value over the exact time_t values this clock yields
    for k in -4..=4i64 {
        for t in [
            base.wrapping_add(k),
            clock.wrapping_add(k),
            (clock >> 29).wrapping_mul(2).wrapping_add(k),
        ] {
            let hc = unsafe { (l.c.hash_time_value)(t) };
            let hrs = unsafe { (l.rs.hash_time_value)(t) };
            eq_int("C69", (clock, t), hc, hrs);
            assert!(hc >= 0, "C69: hash_time_value({t}) = {hc}");
        }
    }

    eprintln!("faketime worker: clock={clock} base={base} -- all rows matched");
}
