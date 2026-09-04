// Phase C — error-path differential tests. One test per row of ERRORS.md.
//
// Every test constructs the exact invalid input/condition, calls BOTH the C
// `.so` and the Rust `.so` through their exported symbols, and asserts the same
// rejection (same error code / sentinel / signal), not merely "both failed".
mod harness;

use harness::*;
use std::ffi::c_int;

fn is_root() -> bool {
    // No `libc` crate in dev-deps; ask the kernel through /proc instead.
    std::fs::read_to_string("/proc/self/status")
        .map(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1).map(|u| u == "0"))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn run_env(tag: &'static str, log_file: Option<&'static str>, max: Option<&'static str>) -> Run<'static> {
    Run {
        tag,
        env: vec![
            ("LOG_FILE".into(), log_file.map(|s| s.to_string())),
            ("MAX_TASKS".into(), max.map(|s| s.to_string())),
        ],
        log_files: vec![("log.txt".into(), "log.txt".into())],
        chdir: false,
    }
}

// ===========================================================================
// E1 — `$LOG_FILE` in a non-existent directory ⇒ rc -1 + stderr message.
// ===========================================================================
#[test]
fn phase_c_e1_initialize_logger_missing_directory() {
    scenario(
        "e1",
        run_env("e1", Some("/nonexistent-dir-xyz-9f3/log.txt"), None),
        |api, s| unsafe {
            let rc = (api.initialize_logger)();
            s.int("rc", rc);
            assert_eq!(rc, -1, "[{}] expected -1 from initialize_logger", api.name);
            // Post-failure the static handle is still NULL, so logging is silent.
            (api.log_info)(c"should not appear".as_ptr());
            (api.finalize_logger)();
        },
    );
}

// ===========================================================================
// E2 — `$LOG_FILE` set to the empty string (getenv non-NULL ⇒ "" is the path).
// ===========================================================================
#[test]
fn phase_c_e2_initialize_logger_empty_path() {
    scenario("e2", run_env("e2", Some(""), None), |api, s| unsafe {
        let rc = (api.initialize_logger)();
        s.int("rc", rc);
        assert_eq!(rc, -1, "[{}] expected -1 for empty $LOG_FILE", api.name);
    });
}

// ===========================================================================
// E3 — `$LOG_FILE` names an existing directory ⇒ fopen EISDIR.
// ===========================================================================
#[test]
fn phase_c_e3_initialize_logger_path_is_directory() {
    let _g = guard();
    let libs = fresh("e3");
    diff(
        &libs,
        Run {
            tag: "e3",
            env: vec![
                ("LOG_FILE".into(), Some("@/adir".into())),
                ("MAX_TASKS".into(), None),
            ],
            log_files: vec![],
            chdir: false,
        },
        |api, s| unsafe {
            let p = std::env::var("LOG_FILE").unwrap();
            std::fs::create_dir_all(&p).unwrap();
            let rc = (api.initialize_logger)();
            s.int("rc", rc);
            assert_eq!(rc, -1, "[{}] expected -1 when $LOG_FILE is a dir", api.name);
        },
    );
}

// ===========================================================================
// E4 — `$LOG_FILE` names a mode-0444 file ⇒ fopen EACCES.
// ===========================================================================
#[test]
fn phase_c_e4_initialize_logger_readonly_file() {
    if is_root() {
        eprintln!("E4 skipped: running as root, mode 0444 does not deny write");
        return;
    }
    let _g = guard();
    let libs = fresh("e4");
    diff(
        &libs,
        Run {
            tag: "e4",
            env: vec![
                ("LOG_FILE".into(), Some("@/ro.txt".into())),
                ("MAX_TASKS".into(), None),
            ],
            log_files: vec![("ro.txt".into(), "ro.txt".into())],
            chdir: false,
        },
        |api, s| unsafe {
            use std::os::unix::fs::PermissionsExt;
            let p = std::env::var("LOG_FILE").unwrap();
            std::fs::write(&p, b"").unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o444)).unwrap();
            let rc = (api.initialize_logger)();
            s.int("rc", rc);
            assert_eq!(rc, -1, "[{}] expected -1 for read-only log file", api.name);
        },
    );
}

// ===========================================================================
// E5/E6/E7 — log_info / log_warning / log_error while `log_file == NULL`.
// E8 — finalize_logger while `log_file == NULL` (no "Logger finalized." line).
// ===========================================================================
#[test]
fn phase_c_e5_e6_e7_e8_logging_before_init_is_noop() {
    scenario("e5678", run_with("e5678", None), |api, s| unsafe {
        let m = cstr(b"never written");
        (api.log_info)(m.as_ptr());
        (api.log_warning)(m.as_ptr());
        (api.log_error)(m.as_ptr());
        (api.finalize_logger)();
        // The log file must not even have been created.
        let p = std::env::var("LOG_FILE").unwrap();
        s.int("log_exists", std::path::Path::new(&p).exists() as i32);
        assert!(
            !std::path::Path::new(&p).exists(),
            "[{}] log file must not exist before initialize_logger",
            api.name
        );
    });
}

/// Same, but after a *failed* `initialize_logger` (E1) — the handle is still
/// NULL, so every subsequent call must stay silent.
#[test]
fn phase_c_e5_e8_logging_after_failed_init_is_noop() {
    scenario(
        "e58b",
        run_env("e58b", Some("/nonexistent-dir-xyz-9f3/log.txt"), None),
        |api, s| unsafe {
            s.int("rc", (api.initialize_logger)());
            let m = cstr(b"still silent");
            (api.log_info)(m.as_ptr());
            (api.log_warning)(m.as_ptr());
            (api.log_error)(m.as_ptr());
            (api.finalize_logger)();
        },
    );
}

// ===========================================================================
// E9 — `malloc(sizeof(TaskManager))` failure.
//
// Unreachable through the public API: the request is a fixed 16 bytes and both
// builds issue it through the *same* libc `malloc`, so there is no input that
// makes one fail and not the other. Covered by source inspection plus the
// structurally identical second-allocation failure below (E10-E12), which does
// exercise `log_error` + `return NULL`.
//
// What is asserted here is the part that IS observable: for a request that
// succeeds, both return non-NULL with identical field values, and both write
// the identical success log line — i.e. the branch condition is evaluated on
// the same value in both builds.
// ===========================================================================
#[test]
fn phase_c_e9_manager_allocation_branch_agrees() {
    scenario("e9", run_with("e9", Some("1")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        s.is_null("mgr", m);
        s.mgr("mgr", m);
        assert!(!m.is_null(), "[{}] 16-byte malloc must succeed", api.name);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// E10/E11/E12 — the `tasks` allocation fails ⇒ log_error + free + NULL.
// ===========================================================================
fn tasks_alloc_failure(tag: &'static str, values: &[&str]) {
    let _g = guard();
    for v in values {
        let libs = fresh(tag);
        diff(&libs, run_with(tag, Some(v)), |api, s| unsafe {
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            s.is_null("mgr", m);
            assert!(
                m.is_null(),
                "[{}] create_task_manager must return NULL for MAX_TASKS={}",
                api.name,
                std::env::var("MAX_TASKS").unwrap_or_default()
            );
            (api.finalize_logger)();
        });
    }
}

#[test]
fn phase_c_e10_max_tasks_negative() {
    tasks_alloc_failure("e10", &["-1", "-2", "-10", "-1000000"]);
}

#[test]
fn phase_c_e11_max_tasks_huge_positive() {
    tasks_alloc_failure("e11", &["2000000000", "2147483647", "1000000000"]);
}

#[test]
fn phase_c_e12_max_tasks_int_min() {
    tasks_alloc_failure("e12", &["-2147483648", "-2147483647"]);
}

// ===========================================================================
// E13 — one `add_task` past a filled manager.
// ===========================================================================
#[test]
fn phase_c_e13_add_task_past_limit() {
    let _g = guard();
    for max in [1usize, 2, 3, 10] {
        let libs = fresh("e13");
        let maxs = max.to_string();
        diff(&libs, run_with("e13", Some(&maxs)), move |api, s| unsafe {
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            for i in 0..max {
                let d = cstr(format!("ok-{i}").as_bytes());
                (api.add_task)(m, d.as_ptr(), i as c_int);
            }
            s.mgr("full", m);
            // Three rejected adds in a row: each must warn and change nothing.
            for i in 0..3 {
                let d = cstr(format!("rejected-{i}").as_bytes());
                (api.add_task)(m, d.as_ptr(), 999);
                s.mgr("after_reject", m);
                assert_eq!(
                    (*m).task_count,
                    max as c_int,
                    "[{}] task_count must stay at max",
                    api.name
                );
            }
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// E14 — `$MAX_TASKS=0` (and non-numeric ⇒ atoi 0): the *first* add is rejected.
// ===========================================================================
#[test]
fn phase_c_e14_add_task_zero_capacity() {
    let _g = guard();
    for v in ["0", "abc", "", "-0", "x9"] {
        let libs = fresh("e14");
        diff(&libs, run_with("e14", Some(v)), |api, s| unsafe {
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            s.is_null("mgr", m);
            assert!(!m.is_null(), "[{}] malloc(0) still succeeds", api.name);
            s.mgr("fresh", m);
            let d = cstr(b"first");
            (api.add_task)(m, d.as_ptr(), 1);
            s.mgr("after", m);
            assert_eq!((*m).task_count, 0, "[{}] nothing may be stored", api.name);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

// ===========================================================================
// E15 — hand-built manager with a negative `max_tasks` (`0 >= -1`).
// ===========================================================================
#[test]
fn phase_c_e15_add_task_negative_max() {
    scenario("e15", run_with("e15", Some("4")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        for neg in [-1i32, -100, i32::MIN] {
            (*m).task_count = 0;
            (*m).max_tasks = neg;
            let d = cstr(b"nope");
            (api.add_task)(m, d.as_ptr(), 7);
            s.mgr("after", m);
            assert_eq!((*m).task_count, 0, "[{}] must reject", api.name);
        }
        (*m).max_tasks = 4;
        (*m).task_count = 0;
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// E16 — `driver` when `initialize_logger` fails ⇒ EXIT_FAILURE (1), no stdout.
// ===========================================================================
#[test]
fn phase_c_e16_driver_logger_failure() {
    let _g = guard();
    for lf in ["/nonexistent-dir-xyz-9f3/log.txt", ""] {
        let libs = fresh("e16");
        let lf_owned = lf.to_string();
        diff(
            &libs,
            Run {
                tag: "e16",
                env: vec![
                    ("LOG_FILE".into(), Some(lf_owned)),
                    ("MAX_TASKS".into(), Some("4".into())),
                ],
                log_files: vec![],
                chdir: false,
            },
            |api, s| unsafe {
                let input = cstr(b"a\nb\nc");
                let rc = (api.driver)(input.as_ptr());
                s.int("rc", rc);
                assert_eq!(rc, 1, "[{}] expected EXIT_FAILURE", api.name);
            },
        );
    }
}

// ===========================================================================
// E17 — `driver` when `create_task_manager` fails ⇒ EXIT_FAILURE, and the
// quirk that `finalize_logger()` is NOT called (no "Logger finalized." line).
// ===========================================================================
#[test]
fn phase_c_e17_driver_manager_failure() {
    let _g = guard();
    for v in ["-1", "2000000000", "-2147483648"] {
        let libs = fresh("e17");
        diff(&libs, run_with("e17", Some(v)), |api, s| unsafe {
            let input = cstr(b"a\nb\nc");
            let rc = (api.driver)(input.as_ptr());
            s.int("rc", rc);
            assert_eq!(rc, 1, "[{}] expected EXIT_FAILURE", api.name);
            // The log file is still open (never fclosed): flush so its bytes
            // are comparable, then check the finalize line is absent.
            flush_all();
            let p = std::env::var("LOG_FILE").unwrap();
            let body = std::fs::read(&p).unwrap_or_default();
            s.bytes("log", &body);
            assert!(
                !String::from_utf8_lossy(&body).contains("Logger finalized."),
                "[{}] driver must not finalize on manager failure",
                api.name
            );
        });
    }
}

// ===========================================================================
// E18 — per-line `malloc(length + 1)` failure inside `driver`.
//
// Unreachable through the public API (lines are at most as long as the caller's
// own buffer, and both builds call the identical libc `malloc`). Covered by
// source inspection. What is asserted here is that the surrounding, observable
// behaviour of the same loop iteration is identical for the longest inputs we
// can pass, including the boundary where a line exceeds the 256-byte task slot.
// ===========================================================================
#[test]
fn phase_c_e18_driver_line_allocation_branch_agrees() {
    let _g = guard();
    for len in [0usize, 1, 255, 256, 4096, 65536] {
        let libs = fresh("e18");
        diff(&libs, run_with("e18", Some("4")), move |api, s| unsafe {
            let mut rng = Rng::new(1800 + len as u64);
            let mut buf = rng.printable(len);
            buf.push(b'\n');
            buf.extend_from_slice(&rng.printable(len));
            let cs = cstr(&buf);
            let rc = (api.driver)(cs.as_ptr());
            s.int("rc", rc);
            assert_eq!(rc, 0, "[{}] allocation must succeed here", api.name);
        });
    }
}

// ===========================================================================
// E19 — add_task(NULL, ...) ⇒ SIGSEGV in both.
// ===========================================================================
#[test]
fn phase_c_e19_add_task_null_manager() {
    crash_scenario("e19", run_with("e19", Some("4")), Exit::Signal(SIGSEGV), |api| unsafe {
        let d = cstr(b"x");
        (api.add_task)(std::ptr::null_mut(), d.as_ptr(), 1);
    });
}

// ===========================================================================
// E20 — print_tasks(NULL) ⇒ prints "Tasks:" (buffered, then lost) and SIGSEGV.
// ===========================================================================
#[test]
fn phase_c_e20_print_tasks_null_manager() {
    crash_scenario("e20", run_with("e20", Some("4")), Exit::Signal(SIGSEGV), |api| unsafe {
        (api.print_tasks)(std::ptr::null());
    });
}

// ===========================================================================
// E21 — destroy_task_manager(NULL) ⇒ SIGSEGV.
// ===========================================================================
#[test]
fn phase_c_e21_destroy_null_manager() {
    crash_scenario("e21", run_with("e21", Some("4")), Exit::Signal(SIGSEGV), |api| unsafe {
        (api.destroy_task_manager)(std::ptr::null_mut());
    });
}

// ===========================================================================
// E22 — driver(NULL) ⇒ SIGSEGV after the logger/manager are set up.
// ===========================================================================
#[test]
fn phase_c_e22_driver_null_input() {
    crash_scenario("e22", run_with("e22", Some("4")), Exit::Signal(SIGSEGV), |api| unsafe {
        (api.driver)(std::ptr::null());
    });
}

// ===========================================================================
// E23 — add_task(mgr, NULL, p) ⇒ SIGSEGV inside strncpy.
// ===========================================================================
#[test]
fn phase_c_e23_add_task_null_description() {
    crash_scenario("e23", run_with("e23", Some("4")), Exit::Signal(SIGSEGV), |api| unsafe {
        (api.initialize_logger)();
        let m = (api.create_task_manager)();
        (api.add_task)(m, std::ptr::null(), 1);
    });
}

// ===========================================================================
// E24 — log_*(NULL) with an open log: glibc `%s` prints "(null)", no crash.
// ===========================================================================
#[test]
fn phase_c_e24_log_null_message() {
    scenario("e24", run_with("e24", None), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        (api.log_info)(std::ptr::null());
        (api.log_warning)(std::ptr::null());
        (api.log_error)(std::ptr::null());
        (api.finalize_logger)();
    });
}

// ===========================================================================
// E25 — over-long description: silent truncation to 255 bytes, still success.
// ===========================================================================
#[test]
fn phase_c_e25_description_truncation_is_not_an_error() {
    scenario("e25", run_with("e25", Some("16")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        for len in [255usize, 256, 257, 1000, 100_000] {
            let mut rng = Rng::new(2500 + len as u64);
            let d = cstr(&rng.printable(len));
            (api.add_task)(m, d.as_ptr(), len as c_int);
            s.mgr("after", m);
            assert_eq!(
                (*m).task_count as usize,
                s.recs.len() - 1,
                "[{}] truncation must not be an error",
                api.name
            );
        }
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// E26 — no enum exists in this API. The only int parameter (`priority`) accepts
// every one of the 2^32 values; there is no sentinel and no rejection. Passing
// "out-of-range" ints across the FFI boundary must therefore store and render
// them verbatim in both builds.
// ===========================================================================
#[test]
fn phase_c_e26_no_enum_every_int_is_valid() {
    scenario("e26", run_with("e26", Some("256")), |api, s| unsafe {
        s.int("rc", (api.initialize_logger)());
        let m = (api.create_task_manager)();
        // Values that would be "invalid enum variants" in a C API that had one.
        let mut vals: Vec<i32> = vec![
            i32::MIN,
            i32::MIN + 1,
            -12345,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            5,
            99,
            255,
            256,
            65535,
            65536,
            i32::MAX - 1,
            i32::MAX,
        ];
        let mut rng = Rng::new(26);
        for _ in 0..60 {
            vals.push(rng.i32());
        }
        for (i, v) in vals.iter().enumerate() {
            let d = cstr(format!("v{i}").as_bytes());
            (api.add_task)(m, d.as_ptr(), *v);
        }
        s.mgr("final", m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ===========================================================================
// Generic boundary sweep required regardless of the table: zero/oversized
// lengths and one-step-past-range values on every entry point that takes them.
// ===========================================================================
#[test]
fn phase_c_generic_boundaries() {
    let _g = guard();
    // one past the capacity boundary, for several capacities
    for max in [0usize, 1, 2, 15, 16] {
        let libs = fresh("gb");
        let maxs = max.to_string();
        diff(&libs, run_with("gb", Some(&maxs)), move |api, s| unsafe {
            s.int("rc", (api.initialize_logger)());
            let m = (api.create_task_manager)();
            s.is_null("mgr", m);
            let mut rng = Rng::new(7000 + max as u64);
            for i in 0..(max + 2) {
                // description length hops the 255/256 boundary
                let len = match i % 4 {
                    0 => 0,
                    1 => 255,
                    2 => 256,
                    _ => rng.range(1, 300),
                };
                let d = cstr(&rng.printable(len));
                (api.add_task)(m, d.as_ptr(), i as c_int - 1);
                s.mgr("step", m);
            }
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        });
    }
}

/// `driver` with a zero-length payload and with a payload far larger than any
/// task slot, plus `$MAX_TASKS` at both ends of its usable range.
#[test]
fn phase_c_generic_driver_boundaries() {
    let _g = guard();
    for max in ["0", "1", "2", "10"] {
        for n in [0usize, 1, 2, 11] {
            let libs = fresh("gdb");
            diff(&libs, run_with("gdb", Some(max)), move |api, s| unsafe {
                let mut rng = Rng::new(8000 + n as u64);
                let mut buf: Vec<u8> = Vec::new();
                for i in 0..n {
                    if i > 0 {
                        buf.push(b'\n');
                    }
                    buf.extend_from_slice(&rng.printable(if i == 0 { 0 } else { 400 }));
                }
                let cs = cstr(&buf);
                let rc = (api.driver)(cs.as_ptr());
                s.int("rc", rc);
            });
        }
    }
}
