// Phase B — valid-path differential tests. One test per row of CONFIGS.md.
//
// Every test loads BOTH the C `.so` and the Rust `.so` with `libloading` and
// calls them only through their exported symbols. Randomised rows use a fixed
// seed so a failure is reproducible.
mod harness;

use harness::*;
use std::ffi::c_int;

// ===========================================================================
// C1 — every logger entry point while the static `log_file` is still NULL.
// ===========================================================================
#[test]
fn phase_b_c1_logger_uninitialised_is_silent() {
    scenario(
        "c1",
        Run {
            tag: "c1",
            env: vec![
                ("LOG_FILE".into(), Some("@/log.txt".into())),
                ("MAX_TASKS".into(), None),
            ],
            log_files: vec![("log.txt".into(), "log.txt".into())],
            chdir: false,
        },
        |api, s| unsafe {
            let mut rng = Rng::new(1);
            for _ in 0..16 {
                let m = cstr(&rng.printable_range(0, 40));
                (api.log_info)(m.as_ptr());
                (api.log_warning)(m.as_ptr());
                (api.log_error)(m.as_ptr());
            }
            (api.finalize_logger)();
            s.int("after_finalize_marker", 0);
        },
    );
}

// ===========================================================================
// C2 — `$LOG_FILE` unset ⇒ `default.log` in the CWD.
// ===========================================================================
#[test]
fn phase_b_c2_initialize_logger_default_path() {
    scenario("c2", run_default_log("c2", None), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
    });
}

// ===========================================================================
// C3 — `$LOG_FILE` set to a fresh writable path.
// ===========================================================================
#[test]
fn phase_b_c3_initialize_logger_explicit_path() {
    scenario("c3", run_with("c3", None), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
    });
}

// ===========================================================================
// C4 — append mode: pre-existing bytes must be preserved.
// ===========================================================================
#[test]
fn phase_b_c4_append_to_existing_log() {
    let _g = guard();
    let libs = fresh("c4");
    // The scenario body itself seeds the file, so both runs see the same
    // pre-state in their own private directory.
    diff(&libs, run_with("c4", None), |api, s| unsafe {
        let path = std::env::var("LOG_FILE").unwrap();
        std::fs::write(&path, b"PRE-EXISTING\n").unwrap();
        s.int("rc", (api.initialize_logger)());
        (api.log_info)(c"after".as_ptr());
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C5 — re-initialise without finalising (handle overwritten, no fclose).
// ===========================================================================
#[test]
fn phase_b_c5_double_initialize_same_path() {
    scenario("c5", run_with("c5", None), |api, s| unsafe {
        s.int("rc1", (api.initialize_logger)());
        s.int("rc2", (api.initialize_logger)());
        (api.log_info)(c"between".as_ptr());
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C6 — re-initialise onto a *different* path.
// ===========================================================================
#[test]
fn phase_b_c6_reinitialize_different_path() {
    let _g = guard();
    let libs = fresh("c6");
    diff(
        &libs,
        Run {
            tag: "c6",
            env: vec![
                ("LOG_FILE".into(), Some("@/first.txt".into())),
                ("MAX_TASKS".into(), None),
            ],
            log_files: vec![
                ("first.txt".into(), "first.txt".into()),
                ("second.txt".into(), "second.txt".into()),
            ],
            chdir: false,
        },
        |api, s| unsafe {
            s.int("rc1", (api.initialize_logger)());
            (api.log_info)(c"to-first".as_ptr());
            let first = std::env::var("LOG_FILE").unwrap();
            let second = std::path::Path::new(&first).with_file_name("second.txt");
            std::env::set_var("LOG_FILE", &second);
            s.int("rc2", (api.initialize_logger)());
            (api.log_warning)(c"to-second".as_ptr());
            (api.finalize_logger)();
            std::env::set_var("LOG_FILE", &first);
        },
    );
}

// ===========================================================================
// C7 — all three levels interleaved, randomised messages.
// ===========================================================================
#[test]
fn phase_b_c7_all_levels_interleaved() {
    scenario("c7", run_with("c7", None), |api, s| unsafe {
        let mut rng = Rng::new(7);
        s.int("rc", (api.initialize_logger)());
        for _ in 0..64 {
            let m = cstr(&rng.printable_range(0, 80));
            match rng.below(3) {
                0 => (api.log_info)(m.as_ptr()),
                1 => (api.log_warning)(m.as_ptr()),
                _ => (api.log_error)(m.as_ptr()),
            }
        }
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C8 — boundary / hostile message payloads.
// ===========================================================================
#[test]
fn phase_b_c8_boundary_messages() {
    scenario("c8", run_with("c8", None), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let mut msgs: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"x".to_vec(),
            b"line1\nline2".to_vec(),
            b"%s".to_vec(),
            b"%d %i %u".to_vec(),
            b"%n".to_vec(),
            b"%%".to_vec(),
            b"%1000d".to_vec(),
            b"tab\there\rcr".to_vec(),
            vec![b'A'; 4096],
        ];
        // every non-NUL byte value
        msgs.push((1u8..=255).collect());
        let mut rng = Rng::new(8);
        for _ in 0..24 {
            msgs.push(rng.cbytes_range(0, 300));
        }
        for m in &msgs {
            let cs = cstr(m);
            (api.log_info)(cs.as_ptr());
            (api.log_warning)(cs.as_ptr());
            (api.log_error)(cs.as_ptr());
        }
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C9 — finalize appends its own line and closes.
// ===========================================================================
#[test]
fn phase_b_c9_finalize_line() {
    scenario("c9", run_with("c9", None), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C10 — create/destroy with `$MAX_TASKS` unset and the logger *not* initialised.
// ===========================================================================
#[test]
fn phase_b_c10_create_destroy_default_no_logger() {
    scenario("c10", run_with("c10", None), |api, s| unsafe {
        let m = (api.create_task_manager)();
        s.is_null("mgr", m);
        s.mgr("mgr", m);
        (api.destroy_task_manager)(m);
    });
}

// ===========================================================================
// C11 — same, but with the logger initialised (log lines must appear).
// ===========================================================================
#[test]
fn phase_b_c11_create_destroy_with_logger() {
    scenario("c11", run_with("c11", None), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        s.mgr("mgr", m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C12 — `$MAX_TASKS` numeric sweep.
// ===========================================================================
#[test]
fn phase_b_c12_max_tasks_numeric_sweep() {
    let _g = guard();
    let mut rng = Rng::new(12);
    let mut vals: Vec<String> = ["1", "2", "7", "10", "64", "1000"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    for _ in 0..12 {
        vals.push(rng.range(1, 4096).to_string());
    }
    for v in vals {
        let libs = fresh("c12");
        diff(&libs, run_with("c12", Some(&v)), |api, s| unsafe {
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            s.is_null("mgr", m);
            s.mgr("mgr", m);
            if !m.is_null() {
                (api.destroy_task_manager)(m);
            }
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// C13 — `$MAX_TASKS=0`: `malloc(0)` still succeeds, manager is returned.
// ===========================================================================
#[test]
fn phase_b_c13_max_tasks_zero() {
    scenario("c13", run_with("c13", Some("0")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        s.is_null("mgr", m);
        s.mgr("mgr", m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C14 — `$MAX_TASKS` non-numeric / partially numeric: whatever `atoi` yields.
// ===========================================================================
#[test]
fn phase_b_c14_max_tasks_atoi_semantics() {
    let _g = guard();
    let vals = [
        "abc",
        "",
        "  12",
        "\t9",
        "+5",
        "3x",
        "0x10",
        "1e3",
        "99999999999999999999",
        "2147483647",
        "007",
        " -0",
        "12 34",
        ".5",
        "-",
    ];
    for v in vals {
        let libs = fresh("c14");
        diff(&libs, run_with("c14", Some(v)), |api, s| unsafe {
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            s.is_null("mgr", m);
            s.mgr("mgr", m);
            if !m.is_null() {
                let d = cstr(b"probe");
                (api.add_task)(m, d.as_ptr(), 42);
                s.mgr("mgr_after_add", m);
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
            }
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// C15 — the happy manual pipeline, randomised.
// ===========================================================================
#[test]
fn phase_b_c15_manual_pipeline_random() {
    let _g = guard();
    for iter in 0..24u64 {
        let libs = fresh("c15");
        let max = 1 + (iter % 12) as usize;
        let maxs = max.to_string();
        diff(&libs, run_with("c15", Some(&maxs)), move |api, s| unsafe {
            let mut rng = Rng::new(1500 + iter);
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            s.is_null("mgr", m);
            let n = rng.range(1, max);
            for _ in 0..n {
                let d = cstr(&rng.printable_range(0, 60));
                (api.add_task)(m, d.as_ptr(), rng.i32());
                s.mgr("step", m);
            }
            (api.print_tasks)(m);
            s.mgr("final", m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// C16 — description length sweep across the 255-byte `strncpy` boundary.
// ===========================================================================
#[test]
fn phase_b_c16_description_length_boundary() {
    scenario("c16", run_with("c16", Some("64")), |api, s| unsafe {
        let mut rng = Rng::new(16);
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        for len in [0usize, 1, 2, 127, 253, 254, 255, 256, 257, 300, 1024] {
            let d = cstr(&rng.printable(len));
            (api.add_task)(m, d.as_ptr(), len as c_int);
            s.mgr("after", m);
        }
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C17 — hostile description bytes rendered through `%s`.
// ===========================================================================
#[test]
fn phase_b_c17_description_hostile_bytes() {
    scenario("c17", run_with("c17", Some("128")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        let mut descs: Vec<Vec<u8>> = vec![
            b"%s".to_vec(),
            b"%n".to_vec(),
            b"%%".to_vec(),
            b"%1000d".to_vec(),
            b"%.*s".to_vec(),
            b"tab\there".to_vec(),
            b"cr\rhere".to_vec(),
            b"nl\nhere".to_vec(),
            (0x80u8..=0xFF).collect(),
            (1u8..=0x1F).collect(),
        ];
        let mut rng = Rng::new(17);
        for _ in 0..40 {
            descs.push(rng.cbytes_range(0, 400));
        }
        for (i, d) in descs.iter().enumerate() {
            let cs = cstr(d);
            (api.add_task)(m, cs.as_ptr(), i as c_int);
        }
        s.mgr("final", m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C18 — `priority` extremes (no enum exists; every `int` is valid).
// ===========================================================================
#[test]
fn phase_b_c18_priority_extremes() {
    scenario("c18", run_with("c18", Some("128")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        let mut prios: Vec<i32> = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
        let mut rng = Rng::new(18);
        for _ in 0..40 {
            prios.push(rng.i32());
        }
        for p in &prios {
            let d = cstr(format!("p{p}").as_bytes());
            (api.add_task)(m, d.as_ptr(), *p);
        }
        s.mgr("final", m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C19 — fill exactly to `max_tasks` (the accepted boundary).
// ===========================================================================
#[test]
fn phase_b_c19_fill_exactly_to_max() {
    let _g = guard();
    let mut rng = Rng::new(19);
    let mut maxes = vec![1usize, 2, 3, 10];
    for _ in 0..6 {
        maxes.push(rng.range(1, 40));
    }
    for max in maxes {
        let libs = fresh("c19");
        let maxs = max.to_string();
        diff(&libs, run_with("c19", Some(&maxs)), move |api, s| unsafe {
            let mut rng = Rng::new(1900 + max as u64);
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            for i in 0..max {
                let d = cstr(&rng.printable_range(0, 30));
                (api.add_task)(m, d.as_ptr(), i as c_int + 1);
                s.mgr("step", m);
            }
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// C20 — `print_tasks` row counts (0 / 1 / 2 / max), 1-based index.
// ===========================================================================
#[test]
fn phase_b_c20_print_tasks_row_counts() {
    let _g = guard();
    for n in [0usize, 1, 2, 5, 10] {
        let libs = fresh("c20");
        diff(&libs, run_with("c20", Some("10")), move |api, s| unsafe {
            let mut rng = Rng::new(2000 + n as u64);
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            for i in 0..n {
                let d = cstr(&rng.printable_range(1, 20));
                (api.add_task)(m, d.as_ptr(), (i as c_int) * 3 - 1);
            }
            (api.print_tasks)(m);
            s.mgr("final", m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// C21 — `print_tasks` twice: no state change.
// ===========================================================================
#[test]
fn phase_b_c21_print_tasks_twice() {
    scenario("c21", run_with("c21", Some("4")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        for i in 0..4 {
            let d = cstr(format!("task-{i}").as_bytes());
            (api.add_task)(m, d.as_ptr(), i);
        }
        (api.print_tasks)(m);
        s.mgr("after_first", m);
        (api.print_tasks)(m);
        s.mgr("after_second", m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C22 — hand-lowered `task_count`: only the first N rows are printed.
// ===========================================================================
#[test]
fn phase_b_c22_print_tasks_truncated_count() {
    scenario("c22", run_with("c22", Some("8")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        for i in 0..8 {
            let d = cstr(format!("row-{i}").as_bytes());
            (api.add_task)(m, d.as_ptr(), 100 + i);
        }
        for keep in [0, 1, 3, 8] {
            (*m).task_count = keep;
            (api.print_tasks)(m);
        }
        (*m).task_count = 8;
        s.mgr("final", m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C23–C29 — `driver` input shapes.
// ===========================================================================
fn driver_shape(tag: &'static str, max: Option<&'static str>, inputs: &[&[u8]]) {
    let _g = guard();
    for input in inputs {
        let libs = fresh(tag);
        let owned = input.to_vec();
        diff(&libs, run_with(tag, max), move |api, s| unsafe {
            let cs = cstr(&owned);
            s.int("rc", (api.driver)(cs.as_ptr()));
        });
    }
}

#[test]
fn phase_b_c23_driver_empty_string() {
    driver_shape("c23", None, &[b""]);
}

#[test]
fn phase_b_c24_driver_single_line_no_newline() {
    driver_shape("c24", None, &[b"only task", b"x", b"a b c  d"]);
}

#[test]
fn phase_b_c25_driver_single_line_trailing_newline() {
    driver_shape("c25", None, &[b"only task\n", b"x\n"]);
}

#[test]
fn phase_b_c26_driver_many_lines_no_trailing_newline() {
    driver_shape(
        "c26",
        None,
        &[b"a\nb\nc", b"one\ntwo\nthree\nfour\nfive", b"1\n2"],
    );
}

#[test]
fn phase_b_c27_driver_many_lines_trailing_newline() {
    driver_shape("c27", None, &[b"a\nb\nc\n", b"one\ntwo\n"]);
}

#[test]
fn phase_b_c28_driver_consecutive_newlines() {
    driver_shape(
        "c28",
        None,
        &[b"a\n\nb", b"a\n\n\n", b"\n\na", b"a\n\n\n\nb\n\n"],
    );
}

#[test]
fn phase_b_c29_driver_only_newlines() {
    driver_shape("c29", None, &[b"\n", b"\n\n", b"\n\n\n", b"\nx"]);
}

// ===========================================================================
// C30 — a `driver` line at/over the 255-byte truncation boundary.
// ===========================================================================
#[test]
fn phase_b_c30_driver_overlong_line() {
    let _g = guard();
    for len in [254usize, 255, 256, 257, 512, 1000] {
        let libs = fresh("c30");
        diff(&libs, run_with("c30", Some("8")), move |api, s| unsafe {
            let mut rng = Rng::new(3000 + len as u64);
            let mut buf = rng.printable(len);
            buf.push(b'\n');
            buf.extend_from_slice(&rng.printable(len));
            let cs = cstr(&buf);
            s.int("rc", (api.driver)(cs.as_ptr()));
        });
    }
}

// ===========================================================================
// C31 — line count exactly `$MAX_TASKS`.
// C32 — line count greater than `$MAX_TASKS`.
// ===========================================================================
#[test]
fn phase_b_c31_driver_lines_equal_max() {
    let _g = guard();
    for max in [1usize, 2, 3, 10, 17] {
        let libs = fresh("c31");
        let maxs = max.to_string();
        diff(&libs, run_with("c31", Some(&maxs)), move |api, s| unsafe {
            let lines: Vec<String> = (0..max).map(|i| format!("t{i}")).collect();
            let cs = cstr(lines.join("\n").as_bytes());
            s.int("rc", (api.driver)(cs.as_ptr()));
        });
    }
}

#[test]
fn phase_b_c32_driver_lines_over_max() {
    let _g = guard();
    for (max, n) in [(1usize, 4usize), (2, 3), (3, 10), (5, 6), (10, 25)] {
        let libs = fresh("c32");
        let maxs = max.to_string();
        diff(&libs, run_with("c32", Some(&maxs)), move |api, s| unsafe {
            let lines: Vec<String> = (0..n).map(|i| format!("task-{i}")).collect();
            let cs = cstr(lines.join("\n").as_bytes());
            s.int("rc", (api.driver)(cs.as_ptr()));
        });
    }
}

// ===========================================================================
// C33 — tightest limit.
// ===========================================================================
#[test]
fn phase_b_c33_driver_max_one() {
    driver_shape("c33", Some("1"), &[b"a\nb\nc\nd", b"single", b""]);
}

// ===========================================================================
// C34 — `$MAX_TASKS=0` with non-empty input.
// ===========================================================================
#[test]
fn phase_b_c34_driver_max_zero() {
    driver_shape("c34", Some("0"), &[b"a\nb\nc", b"one", b""]);
}

// ===========================================================================
// C35 — `driver` with `$LOG_FILE` unset ⇒ `default.log`.
// ===========================================================================
#[test]
fn phase_b_c35_driver_default_log() {
    scenario(
        "c35",
        run_default_log("c35", Some("4")),
        |api, s| unsafe {
            let cs = cstr(b"alpha\nbeta\ngamma");
            s.int("rc", (api.driver)(cs.as_ptr()));
        },
    );
}

// ===========================================================================
// C36 — `driver` appends to a non-empty log file.
// ===========================================================================
#[test]
fn phase_b_c36_driver_appends_to_existing_log() {
    scenario("c36", run_with("c36", Some("4")), |api, s| unsafe {
        let path = std::env::var("LOG_FILE").unwrap();
        std::fs::write(&path, b"HEADER-BYTES\n").unwrap();
        let cs = cstr(b"alpha\nbeta");
        s.int("rc", (api.driver)(cs.as_ptr()));
    });
}

// ===========================================================================
// C37 — `driver` twice in the same process.
// ===========================================================================
#[test]
fn phase_b_c37_driver_twice() {
    scenario("c37", run_with("c37", Some("3")), |api, s| unsafe {
        let a = cstr(b"first\nsecond");
        let b = cstr(b"third\nfourth\nfifth\nsixth");
        s.int("rc1", (api.driver)(a.as_ptr()));
        s.int("rc2", (api.driver)(b.as_ptr()));
    });
}

// ===========================================================================
// C38 — logger reusable after `driver` finalised it.
// ===========================================================================
#[test]
fn phase_b_c38_logger_reusable_after_driver() {
    scenario("c38", run_with("c38", Some("3")), |api, s| unsafe {
        let a = cstr(b"one\ntwo");
        s.int("rc", (api.driver)(a.as_ptr()));
        s.int("rc_reinit", (api.initialize_logger)());
        (api.log_info)(c"post-driver info".as_ptr());
        (api.log_warning)(c"post-driver warning".as_ptr());
        (api.log_error)(c"post-driver error".as_ptr());
        (api.finalize_logger)();
    });
}

// ===========================================================================
// C39 — randomised `driver` property sweep (fixed seed).
// ===========================================================================
#[test]
fn phase_b_c39_driver_property_sweep() {
    let _g = guard();
    let mut seed_rng = Rng::new(0xC39);
    for iter in 0..200u64 {
        let max = seed_rng.range(0, 25);
        let nlines = seed_rng.range(0, 20);
        let libs = fresh("c39");
        let maxs = max.to_string();
        diff(&libs, run_with("c39", Some(&maxs)), move |api, s| unsafe {
            let mut rng = Rng::new(0x39000 + iter);
            let mut buf: Vec<u8> = Vec::new();
            for i in 0..nlines {
                if i > 0 {
                    buf.push(b'\n');
                }
                let l = rng.range(0, 300);
                buf.extend_from_slice(&rng.printable(l));
            }
            if rng.below(2) == 0 && nlines > 0 {
                buf.push(b'\n');
            }
            let cs = cstr(&buf);
            s.int("rc", (api.driver)(cs.as_ptr()));
            s.bytes("input", cs.to_bytes());
        });
    }
}

// ===========================================================================
// C40 — randomised full manual pipeline (fixed seed).
// ===========================================================================
#[test]
fn phase_b_c40_manual_pipeline_property_sweep() {
    let _g = guard();
    let mut seed_rng = Rng::new(0xC40);
    for iter in 0..200u64 {
        let max = seed_rng.range(0, 16);
        let nadds = seed_rng.range(0, 20);
        let libs = fresh("c40");
        let maxs = max.to_string();
        diff(&libs, run_with("c40", Some(&maxs)), move |api, s| unsafe {
            let mut rng = Rng::new(0x40000 + iter);
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            s.is_null("mgr", m);
            if m.is_null() {
                (api.finalize_logger)();
                return;
            }
            s.mgr("fresh", m);
            for _ in 0..nadds {
                let len = rng.range(0, 320);
                let d = cstr(&rng.cbytes(len));
                (api.add_task)(m, d.as_ptr(), rng.i32());
                s.mgr("step", m);
            }
            (api.print_tasks)(m);
            s.mgr("final", m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// C41 — `driver` fuzz over *arbitrary* non-NUL bytes with newlines placed at
// random positions (C39 restricts itself to printable line bodies).
// ===========================================================================
#[test]
fn phase_b_c41_driver_raw_byte_fuzz() {
    let _g = guard();
    let mut seed_rng = Rng::new(0xC41);
    for iter in 0..250u64 {
        let max = seed_rng.range(0, 12);
        let len = seed_rng.range(0, 900);
        let libs = fresh("c41");
        let maxs = max.to_string();
        diff(&libs, run_with("c41", Some(&maxs)), move |api, s| unsafe {
            let mut rng = Rng::new(0x41000 + iter);
            // Arbitrary bytes 1..=255, then sprinkle newlines.
            let mut buf = rng.cbytes(len);
            let nl = if len == 0 { 0 } else { rng.range(0, len) };
            for _ in 0..nl {
                if len > 0 {
                    let at = rng.below(len);
                    buf[at] = b'\n';
                }
            }
            let cs = cstr(&buf);
            s.int("rc", (api.driver)(cs.as_ptr()));
        });
    }
}

// ===========================================================================
// C42 — randomised *sequences* of API calls (the composed pipeline, not one
// wrapper at a time). Sequences respect the one real precondition of the C
// code: after `finalize_logger` the static handle is dangling, so the next
// logger-touching call must be `initialize_logger`.
// ===========================================================================
#[test]
fn phase_b_c42_random_call_sequences() {
    let _g = guard();
    let mut seed_rng = Rng::new(0xC42);
    for iter in 0..150u64 {
        let max = seed_rng.range(0, 8);
        let nops = seed_rng.range(1, 30);
        let libs = fresh("c42");
        let maxs = max.to_string();
        diff(&libs, run_with("c42", Some(&maxs)), move |api, s| unsafe {
            let mut rng = Rng::new(0x42000 + iter);
            let mut mgrs: Vec<*mut TaskManager> = Vec::new();
            let mut inited = false;
            let mut finalized = false;

            for step in 0..nops {
                if finalized {
                    // only legal continuation
                    s.int("reinit", (api.initialize_logger)());
                    finalized = false;
                    inited = true;
                    continue;
                }
                match rng.below(7) {
                    0 => {
                        s.int("init", (api.initialize_logger)());
                        inited = true;
                    }
                    1 => {
                        let m = cstr(&rng.cbytes_range(0, 120));
                        match rng.below(3) {
                            0 => (api.log_info)(m.as_ptr()),
                            1 => (api.log_warning)(m.as_ptr()),
                            _ => (api.log_error)(m.as_ptr()),
                        }
                    }
                    2 => {
                        let m = (api.create_task_manager)();
                        s.is_null("created", m);
                        if !m.is_null() {
                            s.mgr("created", m);
                            mgrs.push(m);
                        }
                    }
                    3 => {
                        if let Some(&m) = mgrs.get(rng.below(mgrs.len().max(1)).min(
                            mgrs.len().saturating_sub(1),
                        )) {
                            let d = cstr(&rng.cbytes_range(0, 320));
                            (api.add_task)(m, d.as_ptr(), rng.i32());
                            s.mgr("added", m);
                        }
                    }
                    4 => {
                        if !mgrs.is_empty() {
                            let i = rng.below(mgrs.len());
                            (api.print_tasks)(mgrs[i]);
                            s.mgr("printed", mgrs[i]);
                        }
                    }
                    5 => {
                        if !mgrs.is_empty() {
                            let i = rng.below(mgrs.len());
                            let m = mgrs.remove(i);
                            (api.destroy_task_manager)(m);
                            s.int("destroyed", i as i32);
                        }
                    }
                    _ => {
                        if inited {
                            (api.finalize_logger)();
                            finalized = true;
                        }
                    }
                }
                s.int("step", step as i32);
            }

            // Tear everything down so nothing leaks between iterations.
            // `destroy_task_manager` logs, so the handle must be live again:
            // after `finalize_logger` the C static is closed-but-not-NULL, and
            // using it would be UB in *both* builds.
            if finalized {
                s.int("teardown_reinit", (api.initialize_logger)());
                inited = true;
            }
            for m in mgrs {
                (api.destroy_task_manager)(m);
            }
            if inited {
                (api.finalize_logger)();
            }
        });
    }
}
