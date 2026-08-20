//! Phase B — valid-path differential tests for the composed `driver` pipeline.
//!
//! Covers `CONFIGS.md` rows 21-35. `driver` is the one-shot wrapper, but it is
//! exercised here *in addition to* the low-level tests in `phase_b_logger.rs`
//! and `phase_b_task_manager.rs`, because bugs in the composition (line
//! splitting, `priority++` sequencing, ordering of the log records) are
//! invisible to per-function tests.

mod common;

use common::*;

/// Run `driver(text)` and record the return value.
fn run_driver(api: &Api, rec: &mut Rec, text: &[u8]) {
    let buf = cstr(text);
    unsafe { rec.ret((api.driver)(buf.as_ptr() as *const _)) };
}

/// CONFIGS row 21 — S0: empty input.
fn cfg_21_driver_empty_input() {
    let obs = diff("cfg_21", &Cfg::fresh(), |api, rec| run_driver(api, rec, b""));
    assert_eq!(obs.rets, vec![0]);
    assert_eq!(obs.stdout, b"Tasks:\n".to_vec());
    assert!(obs.stderr.is_empty());
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n\
          [INFO] TaskManager created successfully.\n\
          [INFO] TaskManager destroyed successfully.\n\
          [INFO] Logger finalized.\n"
            .to_vec()
    );
}

/// CONFIGS row 22 — S1: one line, no trailing newline (`strchr` → NULL path).
fn cfg_22_driver_single_no_nl() {
    let obs = diff("cfg_22", &Cfg::fresh(), |api, rec| {
        run_driver(api, rec, b"buy milk")
    });
    assert_eq!(obs.rets, vec![0]);
    assert_eq!(obs.stdout, b"Tasks:\n  [1] buy milk (Priority: 1)\n".to_vec());
}

/// CONFIGS row 23 — S2: one line WITH a trailing newline.
fn cfg_23_driver_single_with_nl() {
    let obs = diff("cfg_23", &Cfg::fresh(), |api, rec| {
        run_driver(api, rec, b"buy milk\n")
    });
    assert_eq!(obs.rets, vec![0]);
    assert_eq!(
        obs.stdout,
        b"Tasks:\n  [1] buy milk (Priority: 1)\n".to_vec(),
        "a trailing newline must not create an extra empty task"
    );
}

/// CONFIGS row 24 — S3/S4: 2..9 lines, with and without a trailing newline.
fn cfg_24_driver_multi_lines() {
    let _g = lock();
    for n in 2..=9usize {
        for trailing in [false, true] {
            let mut text: Vec<u8> = Vec::new();
            for i in 0..n {
                if i > 0 {
                    text.push(b'\n');
                }
                text.extend_from_slice(format!("task number {i}").as_bytes());
            }
            if trailing {
                text.push(b'\n');
            }
            let label = format!("cfg_24 n={n} trailing={trailing}");
            let obs = diff_locked(&label, &Cfg::fresh().max("16"), |api, rec| {
                run_driver(api, rec, &text)
            });
            let printed = String::from_utf8_lossy(&obs.stdout);
            assert_eq!(printed.lines().count(), n + 1, "{label}: wrong row count");
            assert!(printed.contains(&format!("  [{n}] task number {} (Priority: {n})\n", n - 1)));
        }
    }
}

/// CONFIGS row 25 — S5/S6/S7: leading, consecutive and only newlines.
/// The C loop adds an **empty** task for each empty line.
fn cfg_25_driver_empty_lines() {
    let _g = lock();
    let cases: Vec<&[u8]> = vec![
        b"\n",
        b"\n\n",
        b"\n\n\n",
        b"\na",
        b"a\n\nb",
        b"a\n\n\nb\n",
        b"\n\na\n\n",
        b"\na\nb\n\nc",
    ];
    for (i, text) in cases.iter().enumerate() {
        let label = format!("cfg_25 case {i} {:?}", String::from_utf8_lossy(text));
        diff_locked(&label, &Cfg::fresh().max("16"), |api, rec| {
            run_driver(api, rec, text)
        });
    }
    // Absolute expectation for the simplest case: "\n" yields exactly one
    // *empty* task, not zero.
    let obs = diff_locked("cfg_25 \\n", &Cfg::fresh(), |api, rec| {
        run_driver(api, rec, b"\n")
    });
    assert_eq!(obs.stdout, b"Tasks:\n  [1]  (Priority: 1)\n".to_vec());
}

/// CONFIGS row 26 — S8: CRLF input; the `\r` stays inside the description.
fn cfg_26_driver_crlf() {
    let obs = diff("cfg_26", &Cfg::fresh(), |api, rec| {
        run_driver(api, rec, b"alpha\r\nbeta\r\n")
    });
    assert_eq!(
        obs.stdout,
        b"Tasks:\n  [1] alpha\r (Priority: 1)\n  [2] beta\r (Priority: 2)\n".to_vec()
    );
}

/// CONFIGS row 27 — D2..D6: line lengths across the 255-byte truncation limit,
/// inside the composed pipeline.
fn cfg_27_driver_long_lines() {
    let _g = lock();
    for len in [0usize, 1, 254, 255, 256, 257, 300, 1000, 5000] {
        let body: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        for trailing in [false, true] {
            let mut text = body.clone();
            if trailing {
                text.push(b'\n');
            }
            let label = format!("cfg_27 len={len} trailing={trailing}");
            diff_locked(&label, &Cfg::fresh(), |api, rec| run_driver(api, rec, &text));
        }
    }
    // Two long lines in one input, so the second Task slot is also exercised.
    let mut text = vec![b'q'; 300];
    text.push(b'\n');
    text.extend(std::iter::repeat(b'w').take(280));
    let obs = diff_locked("cfg_27 two long", &Cfg::fresh(), |api, rec| {
        run_driver(api, rec, &text)
    });
    let printed = String::from_utf8_lossy(&obs.stdout);
    assert_eq!(printed.matches('q').count(), 255);
    assert_eq!(printed.matches('w').count(), 255);
}

/// CONFIGS row 28 — S9/S10/S11: high bytes, `printf` metacharacters, tabs.
fn cfg_28_driver_odd_bytes() {
    let _g = lock();
    let cases: Vec<Vec<u8>> = vec![
        b"%s\n%d\n%n".to_vec(),
        b"100%\n%%\n".to_vec(),
        b"tab\there\nvt\x0bthere\n".to_vec(),
        "caf\u{e9}\n\u{4e2d}\u{6587}\n\u{1f600}\n".as_bytes().to_vec(),
        vec![0x80, 0x81, b'\n', 0xFE, 0xFF],
        vec![0xFF; 300],
        (1u8..=255).filter(|&b| b != b'\n').collect(),
    ];
    for (i, text) in cases.iter().enumerate() {
        let label = format!("cfg_28 case {i}");
        diff_locked(&label, &Cfg::fresh().max("16"), |api, rec| {
            run_driver(api, rec, text)
        });
    }
}

/// CONFIGS row 29 — `MAX_TASKS=0`: every line is rejected but `driver` still
/// returns 0.
fn cfg_29_driver_max_zero() {
    let obs = diff("cfg_29", &Cfg::fresh().max("0"), |api, rec| {
        run_driver(api, rec, b"a\nb\nc")
    });
    assert_eq!(obs.rets, vec![0]);
    assert_eq!(obs.stdout, b"Tasks:\n".to_vec());
    let text = String::from_utf8_lossy(&obs.log);
    assert_eq!(
        text.matches("[WARNING] Cannot add task: Maximum task limit reached.")
            .count(),
        3
    );
}

/// CONFIGS row 30 — N3/N4: exactly `max_tasks` lines and more than that;
/// `priority` keeps incrementing through the dropped lines.
fn cfg_30_driver_max_boundary() {
    let _g = lock();
    for max in [1usize, 2, 3, 5] {
        for lines in [max.saturating_sub(1), max, max + 1, max + 4] {
            let mut text = Vec::new();
            for i in 0..lines {
                if i > 0 {
                    text.push(b'\n');
                }
                text.extend_from_slice(format!("L{i}").as_bytes());
            }
            let label = format!("cfg_30 max={max} lines={lines}");
            let obs = diff_locked(&label, &Cfg::fresh().max(&max.to_string()), |api, rec| {
                run_driver(api, rec, &text)
            });
            let kept = lines.min(max);
            assert_eq!(
                String::from_utf8_lossy(&obs.stdout).lines().count(),
                kept + 1,
                "{label}"
            );
        }
    }
    // priority numbering: with max=2 and 4 lines, the kept tasks are L0/L1 with
    // priorities 1/2 — the dropped lines still consumed priorities 3 and 4.
    let obs = diff_locked("cfg_30 priority seq", &Cfg::fresh().max("2"), |api, rec| {
        run_driver(api, rec, b"L0\nL1\nL2\nL3")
    });
    assert_eq!(
        obs.stdout,
        b"Tasks:\n  [1] L0 (Priority: 1)\n  [2] L1 (Priority: 2)\n".to_vec()
    );
}

/// CONFIGS row 31 — `driver` called twice in one process: the log is appended,
/// not truncated, and both cycles appear in order.
fn cfg_31_driver_twice_appends() {
    let obs = diff("cfg_31", &Cfg::fresh(), |api, rec| {
        run_driver(api, rec, b"first run\n");
        run_driver(api, rec, b"second run\nthird line");
    });
    assert_eq!(obs.rets, vec![0, 0]);
    let text = String::from_utf8_lossy(&obs.log);
    assert_eq!(text.matches("[INFO] Logger initialized.").count(), 2);
    assert_eq!(text.matches("[INFO] Logger finalized.").count(), 2);
    assert_eq!(
        obs.stdout,
        b"Tasks:\n  [1] first run (Priority: 1)\n\
          Tasks:\n  [1] second run (Priority: 1)\n  [2] third line (Priority: 2)\n"
            .to_vec()
    );
}

/// CONFIGS row 32 — L0: `LOG_FILE` unset, so `driver` logs to `./default.log`.
fn cfg_32_driver_default_log() {
    let cfg = Cfg::fresh().log(LogSetting::UnsetUseCwdDefault);
    let obs = diff("cfg_32", &cfg, |api, rec| {
        run_driver(api, rec, b"alpha\nbeta\ngamma\n")
    });
    assert_eq!(obs.rets, vec![0]);
    assert!(
        obs.log.starts_with(b"[INFO] Logger initialized.\n"),
        "default.log content: {:?}",
        String::from_utf8_lossy(&obs.log)
    );
    assert_eq!(
        String::from_utf8_lossy(&obs.log)
            .matches("[INFO] Task added successfully.")
            .count(),
        3
    );
}

/// CONFIGS row 33 — property-style fuzz of the whole composed pipeline:
/// 200 randomized task lists crossed with four `MAX_TASKS` settings.
fn cfg_33_driver_fuzz_random() {
    let _g = lock();
    const SEED: u64 = 0xB0B1_C0DE_1234_0033;
    let mut rng = Rng::new(SEED);
    let maxes = [None, Some("1"), Some("3"), Some("64")];

    for iter in 0..200usize {
        let nlines = rng.range(0, 30);
        let mut text: Vec<u8> = Vec::new();
        for i in 0..nlines {
            if i > 0 {
                text.push(b'\n');
            }
            let len = if rng.below(8) == 0 {
                rng.range(240, 400) // straddle the 255-byte truncation limit
            } else {
                rng.range(0, 40)
            };
            text.extend_from_slice(&rng.text(len));
        }
        // random leading / trailing newlines
        for _ in 0..rng.below(3) {
            text.push(b'\n');
        }
        if rng.bool() {
            text.insert(0, b'\n');
        }

        let m = maxes[rng.below(maxes.len())];
        let cfg = match m {
            Some(v) => Cfg::fresh().max(v),
            None => Cfg::fresh().max_unset(),
        };
        let label = format!(
            "cfg_33 iter={iter} nlines={nlines} MAX_TASKS={m:?} len={}",
            text.len()
        );
        diff_locked(&label, &cfg, |api, rec| run_driver(api, rec, &text));
    }
}

/// CONFIGS row 34 — the pipeline hand-assembled from the lowest-level exports
/// instead of via `driver`, with randomized payloads (50 seeds).
fn cfg_34_manual_pipeline_random() {
    let _g = lock();
    const SEED: u64 = 0xB0B1_C0DE_1234_0034;
    for iter in 0..50usize {
        let label = format!("cfg_34 iter={iter}");
        diff_locked(&label, &Cfg::fresh().max("3"), |api, rec| unsafe {
            let mut rng = Rng::new(SEED ^ iter as u64);
            rec.ret((api.initialize_logger)());
            let n = rng.range(0, 60);
            let w = cstr(&rng.text(n));
            (api.log_warning)(w.as_ptr() as *const _);
            let m = (api.create_task_manager)();
            rec.ptr_is_null(m as *const u8);
            for _ in 0..4 {
                let n = rng.range(0, 300);
                let d = cstr(&rng.text(n));
                (api.add_task)(m, d.as_ptr() as *const _, rng.i32());
                rec.manager(m);
            }
            (api.print_tasks)(m);
            let n = rng.range(0, 60);
            let e = cstr(&rng.text(n));
            (api.log_error)(e.as_ptr() as *const _);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

/// CONFIGS row 35 — `driver` invoked while a logger handle is already open
/// (manual `initialize_logger` first). C overwrites the static and leaks the
/// first stream; both implementations must produce the same file contents.
fn cfg_35_driver_after_manual_init() {
    let obs = diff("cfg_35", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = cstr(b"manual pre-log");
        (api.log_info)(m.as_ptr() as *const _);
        let buf = cstr(b"one\ntwo\n");
        rec.ret((api.driver)(buf.as_ptr() as *const _));
    });
    assert_eq!(obs.rets, vec![0, 0]);
    let text = String::from_utf8_lossy(&obs.log);
    assert!(text.contains("[INFO] manual pre-log"));
    assert_eq!(text.matches("[INFO] Logger initialized.").count(), 2);
}

// ---------------------------------------------------------------------------
// Single serialized entry point (see phase_b_logger.rs for the rationale).
// ---------------------------------------------------------------------------
#[test]
fn phase_b_driver_all() {
    macro_rules! step {
        ($f:ident) => {{
            eprintln!("--> {}", stringify!($f));
            $f();
        }};
    }
    step!(cfg_21_driver_empty_input);
    step!(cfg_22_driver_single_no_nl);
    step!(cfg_23_driver_single_with_nl);
    step!(cfg_24_driver_multi_lines);
    step!(cfg_25_driver_empty_lines);
    step!(cfg_26_driver_crlf);
    step!(cfg_27_driver_long_lines);
    step!(cfg_28_driver_odd_bytes);
    step!(cfg_29_driver_max_zero);
    step!(cfg_30_driver_max_boundary);
    step!(cfg_31_driver_twice_appends);
    step!(cfg_32_driver_default_log);
    step!(cfg_33_driver_fuzz_random);
    step!(cfg_34_manual_pipeline_random);
    step!(cfg_35_driver_after_manual_init);
}
