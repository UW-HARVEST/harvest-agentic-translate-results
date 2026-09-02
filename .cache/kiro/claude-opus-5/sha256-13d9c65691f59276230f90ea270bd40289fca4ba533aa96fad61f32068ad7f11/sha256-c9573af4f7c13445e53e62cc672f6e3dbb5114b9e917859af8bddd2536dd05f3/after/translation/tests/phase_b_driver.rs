//! Phase B — valid-path differential tests for the `driver` one-shot wrapper
//! (`CONFIGS.md` rows 25-40).
//!
//! `driver` composes all nine low-level functions, so this is the "composed
//! pipeline" half of Phase B: it exercises the line-splitting loop in
//! `c_src/src/driver.c:45-66` together with the capacity check in
//! `add_task` and the truncation in `strncpy`.

mod common;

use common::{assert_same, cstring, Config, LogTarget, Rng};
use std::ffi::c_char;

const SEED: u64 = 0xD1_1E_5A_FE_00_11;
const N: usize = 48;

fn run_driver(tag: &str, cfg: &Config, blob: &[u8]) {
    let s = cstring(blob);
    assert_same(tag, cfg, |api| unsafe {
        (api.driver)(s.as_ptr() as *const c_char) as i64
    });
}

// ---------------------------------------------------------------------------
// Rows 25-30: the exact newline shapes the splitting loop distinguishes
// ---------------------------------------------------------------------------

#[test]
fn cfg25_driver_empty() {
    // `while (*start != '\0')` never runs -> header only, no task logged.
    run_driver("cfg25", &Config::new(), b"");
}

#[test]
fn cfg26_driver_single_line() {
    let mut rng = Rng::new(SEED);
    for i in 0..N {
        let len = rng.range(1, 120);
        let mut body = rng.cstr_body(len);
        body.retain(|&b| b != b'\n');
        if body.is_empty() {
            body.push(b'x');
        }
        run_driver(&format!("cfg26-{i}"), &Config::new(), &body);
    }
}

#[test]
fn cfg27_driver_trailing_newline() {
    // After the last line, `start = end + 1` lands on the NUL, so a trailing
    // '\n' must NOT create an extra empty task.
    let mut rng = Rng::new(SEED + 1);
    for i in 0..N {
        let nlines = rng.range(1, 6);
        let mut blob = rng.blob(nlines, 40);
        blob.push(b'\n');
        run_driver(&format!("cfg27-{i}"), &Config::new(), &blob);
    }
}

#[test]
fn cfg28_driver_only_newline() {
    run_driver("cfg28-one", &Config::new(), b"\n");
    run_driver("cfg28-two", &Config::new(), b"\n\n");
    run_driver("cfg28-three", &Config::new(), b"\n\n\n");
    run_driver("cfg28-twelve", &Config::new(), &vec![b'\n'; 12]);
    // More newlines than the capacity.
    run_driver(
        "cfg28-thirty",
        &Config::new().max_tasks("4"),
        &vec![b'\n'; 30],
    );
}

#[test]
fn cfg29_driver_leading_newline() {
    let mut rng = Rng::new(SEED + 2);
    for i in 0..N {
        let nlines = rng.range(1, 6);
        let mut blob = vec![b'\n'];
        blob.extend(rng.blob(nlines, 40));
        run_driver(&format!("cfg29-{i}"), &Config::new(), &blob);
    }
}

#[test]
fn cfg30_driver_empty_middle_line() {
    run_driver("cfg30-ab", &Config::new(), b"a\n\nb");
    run_driver("cfg30-abc", &Config::new(), b"a\n\n\nb\n\nc");
    let mut rng = Rng::new(SEED + 3);
    for i in 0..N {
        // Random blob with a high density of empty lines.
        let mut blob = Vec::new();
        for _ in 0..rng.range(1, 14) {
            if rng.below(3) == 0 {
                blob.push(b'\n');
            } else {
                let len = rng.range(1, 30);
                let mut l = rng.cstr_body(len);
                l.retain(|&b| b != b'\n');
                blob.extend(l);
                blob.push(b'\n');
            }
        }
        blob.pop();
        run_driver(&format!("cfg30-{i}"), &Config::new(), &blob);
    }
}

// ---------------------------------------------------------------------------
// Rows 31-34: line count vs capacity
// ---------------------------------------------------------------------------

#[test]
fn cfg31_driver_many_lines() {
    let mut rng = Rng::new(SEED + 4);
    for i in 0..N {
        let nlines = rng.range(1, 30);
        let blob = rng.blob(nlines, 90);
        run_driver(&format!("cfg31-{i}"), &Config::new(), &blob);
    }
}

#[test]
fn cfg32_driver_exactly_capacity() {
    let mut rng = Rng::new(SEED + 5);
    for cap in [1usize, 2, 5, 10, 17, 64] {
        for i in 0..6 {
            let blob = rng.blob(cap, 60);
            let cfg = Config::new().max_tasks(cap.to_string());
            run_driver(&format!("cfg32-{cap}-{i}"), &cfg, &blob);
        }
    }
}

#[test]
fn cfg33_driver_over_capacity() {
    // Rejected lines still consume a `priority++`, so the priorities printed
    // for the accepted lines must not shift.
    let mut rng = Rng::new(SEED + 6);
    for cap in [1usize, 2, 5, 10] {
        for extra in [1usize, 2, 7, 25] {
            let blob = rng.blob(cap + extra, 50);
            let cfg = Config::new().max_tasks(cap.to_string());
            run_driver(&format!("cfg33-{cap}-plus{extra}"), &cfg, &blob);
        }
    }
}

#[test]
fn cfg34_driver_capacities() {
    let mut rng = Rng::new(SEED + 7);
    for cap in ["1", "2", "5", "10", "64", "0", "007", " 3", "3x"] {
        for i in 0..8 {
            let nlines = rng.range(1, 20);
            let blob = rng.blob(nlines, 70);
            let cfg = Config::new().max_tasks(cap);
            run_driver(&format!("cfg34-{cap:?}-{i}"), &cfg, &blob);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 35-37: line content shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg35_driver_long_lines() {
    // `malloc(length + 1)` + `strncpy(task, start, length)` in driver, then
    // truncation to 255 bytes inside add_task.
    let mut rng = Rng::new(SEED + 8);
    for len in [0usize, 1, 253, 254, 255, 256, 257, 258, 511, 1024, 4096] {
        for i in 0..4 {
            let mut line = rng.cstr_body(len);
            line.retain(|&b| b != b'\n');
            // one long line, then the same line followed by a short one
            run_driver(&format!("cfg35-{len}-{i}a"), &Config::new(), &line);
            let mut blob = line.clone();
            blob.push(b'\n');
            blob.extend(b"tail");
            run_driver(&format!("cfg35-{len}-{i}b"), &Config::new(), &blob);
        }
    }
}

#[test]
fn cfg36_driver_non_utf8() {
    let mut rng = Rng::new(SEED + 9);
    for i in 0..N {
        let nlines = rng.range(1, 12);
        let mut blob = Vec::new();
        for j in 0..nlines {
            if j > 0 {
                blob.push(b'\n');
            }
            let len = rng.range(1, 300);
            for _ in 0..len {
                blob.push(rng.range(0x80, 0xff) as u8);
            }
        }
        run_driver(&format!("cfg36-{i}"), &Config::new(), &blob);
    }
}

#[test]
fn cfg37_driver_format_specifiers() {
    let pieces: &[&[u8]] = &[b"%s", b"%d", b"%n", b"%p", b"%%", b"%999999d", b"%.*s"];
    let mut rng = Rng::new(SEED + 10);
    for i in 0..N {
        let mut blob = Vec::new();
        for j in 0..rng.range(1, 10) {
            if j > 0 {
                blob.push(b'\n');
            }
            for _ in 0..rng.range(1, 12) {
                blob.extend_from_slice(pieces[rng.below(pieces.len())]);
            }
        }
        run_driver(&format!("cfg37-{i}"), &Config::new(), &blob);
    }
}

// ---------------------------------------------------------------------------
// Rows 38-40
// ---------------------------------------------------------------------------

#[test]
fn cfg38_driver_default_log() {
    let mut rng = Rng::new(SEED + 11);
    for i in 0..N {
        let nlines = rng.range(1, 15);
        let blob = rng.blob(nlines, 80);
        let cfg = Config::new().log(LogTarget::Unset);
        run_driver(&format!("cfg38-{i}"), &cfg, &blob);
    }
}

#[test]
fn cfg39_driver_twice() {
    // The second `driver` call re-runs `initialize_logger`, which overwrites
    // the (already `fclose`d) handle, and allocates a second manager.
    let mut rng = Rng::new(SEED + 12);
    for i in 0..N {
        let na = rng.range(1, 8);
        let a = cstring(&rng.blob(na, 50));
        let nb = rng.range(1, 8);
        let b = cstring(&rng.blob(nb, 50));
        assert_same(&format!("cfg39-{i}"), &Config::new(), |api| unsafe {
            let r1 = (api.driver)(a.as_ptr() as *const c_char) as i64;
            let r2 = (api.driver)(b.as_ptr() as *const c_char) as i64;
            r1 * 1000 + r2
        });
    }
}

#[test]
fn cfg40_driver_zero_capacity() {
    // $MAX_TASKS=0 -> malloc(0) succeeds (non-NULL) but every add is rejected.
    let mut rng = Rng::new(SEED + 13);
    for i in 0..N {
        let nlines = rng.range(1, 12);
        let blob = rng.blob(nlines, 60);
        let cfg = Config::new().max_tasks("0");
        run_driver(&format!("cfg40-{i}"), &cfg, &blob);
    }
    run_driver("cfg40-empty", &Config::new().max_tasks("0"), b"");
}
