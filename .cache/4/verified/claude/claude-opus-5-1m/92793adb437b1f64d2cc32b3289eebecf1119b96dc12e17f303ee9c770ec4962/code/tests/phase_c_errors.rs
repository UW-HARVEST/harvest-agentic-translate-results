//! Phase C — error-path differential tests (in-process rows of `ERRORS.md`).
//!
//! Rows that require a sacrificial process (null-pointer dereferences, forced
//! allocation failures, and the genuinely-uninitialised logger state) live in
//! `phase_c_process.rs`.
//!
//! Rows covered here: 1-4, 5-9 (via the post-failed-init NULL state), 11-14,
//! 15-18, 21, 24, 26, 27, 28, 31, 32, 33, 34.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// initialize_logger rejection paths (ERRORS rows 1-4)
// ---------------------------------------------------------------------------

/// Shared body: `initialize_logger` must fail, and the failure must leave the
/// static `log_file` NULL (C assigns the failed `fopen` result *before* testing
/// it), so the four `log_*` entry points then become silent no-ops.
fn init_failure_case(label: &str, log_file_value: &str) -> Obs {
    let cfg = Cfg::fresh().log(LogSetting::Explicit(log_file_value.to_string()));
    let obs = diff(label, &cfg, |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
    });
    assert_eq!(
        obs.rets,
        vec![-1],
        "{label}: initialize_logger must return -1"
    );
    assert_eq!(
        obs.stderr,
        format!("Failed to open log file: {log_file_value}\n").into_bytes(),
        "{label}: wrong stderr message"
    );
    assert!(obs.stdout.is_empty(), "{label}: nothing may go to stdout");
    obs
}

/// ERRORS row 1 — `LOG_FILE` has a nonexistent directory component.
fn err_01_init_logger_bad_path() {
    init_failure_case(
        "err_01",
        "/nonexistent_dir_xyz_abc_123/subdir/logfile.log",
    );
}

/// ERRORS row 2 — `LOG_FILE` names a directory (`fopen` fails with `EISDIR`).
fn err_02_init_logger_dir_path() {
    let dir = unique_path("a_directory");
    std::fs::create_dir_all(&dir).expect("create dir");
    init_failure_case("err_02", dir.to_str().unwrap());
}

/// ERRORS row 3 — `LOG_FILE` is the empty string.
fn err_03_init_logger_empty_path() {
    init_failure_case("err_03", "");
}

/// ERRORS row 4 — `LOG_FILE` lives in a directory with no write permission.
fn err_04_init_logger_perm_denied() {
    use std::os::unix::fs::PermissionsExt;
    let dir = unique_path("ro_dir");
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500))
        .expect("chmod 0500");
    let target = dir.join("cannot_create.log");

    // Skip only if the test happens to run as a user that bypasses DAC checks
    // (e.g. root) — then the C code itself would not fail either.
    let probe = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&target);
    if probe.is_ok() {
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        eprintln!("    (err_04 skipped: this user can write into a 0500 directory)");
        return;
    }

    init_failure_case("err_04", target.to_str().unwrap());
    let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
}

// ---------------------------------------------------------------------------
// log_* / finalize_logger with a NULL log_file (ERRORS rows 5-8)
// ---------------------------------------------------------------------------

/// ERRORS rows 5, 6, 7 — after a *failed* `initialize_logger` the static is
/// NULL, so `log_info` / `log_warning` / `log_error` must be complete no-ops.
/// (The pristine "never initialised" state is covered out-of-process by
/// `phase_c_process.rs::fresh_log_before_init`.)
fn err_05_log_fns_before_init() {
    let cfg = Cfg::fresh().log(LogSetting::Explicit(
        "/nonexistent_dir_xyz_abc_123/x.log".to_string(),
    ));
    let obs = diff("err_05", &cfg, |api, rec| unsafe {
        rec.ret((api.initialize_logger)()); // fails -> static is NULL
        let m = cstr(b"must vanish");
        (api.log_info)(m.as_ptr() as *const _);
        (api.log_warning)(m.as_ptr() as *const _);
        (api.log_error)(m.as_ptr() as *const _);
    });
    assert_eq!(obs.rets, vec![-1]);
    assert!(obs.stdout.is_empty());
    // Only the single "Failed to open log file" line, nothing from the log_* calls.
    assert_eq!(
        obs.stderr,
        b"Failed to open log file: /nonexistent_dir_xyz_abc_123/x.log\n".to_vec()
    );
    assert!(obs.log.is_empty());
}

/// ERRORS row 8 — `finalize_logger` with a NULL static: no `fclose`, and no
/// `[INFO] Logger finalized.` record anywhere.
fn err_06_finalize_before_init() {
    let cfg = Cfg::fresh().log(LogSetting::Explicit(
        "/nonexistent_dir_xyz_abc_123/y.log".to_string(),
    ));
    let obs = diff("err_06", &cfg, |api, rec| unsafe {
        rec.ret((api.initialize_logger)()); // fails -> static is NULL
        (api.finalize_logger)();
        (api.finalize_logger)(); // twice, still a no-op
    });
    assert_eq!(obs.rets, vec![-1]);
    assert!(
        !String::from_utf8_lossy(&obs.stderr).contains("finalized"),
        "finalize must be silent when the logger was never opened"
    );
    assert!(obs.log.is_empty());
}

/// ERRORS row 9 — `message == NULL`: C has no null check, so `fprintf`'s `%s`
/// renders glibc's literal `(null)`.
fn err_07_log_null_message() {
    let obs = diff("err_07", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        (api.log_info)(std::ptr::null());
        (api.log_warning)(std::ptr::null());
        (api.log_error)(std::ptr::null());
        (api.finalize_logger)();
    });
    assert_eq!(obs.rets, vec![0]);
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n\
          [INFO] (null)\n\
          [WARNING] (null)\n\
          [ERROR] (null)\n\
          [INFO] Logger finalized.\n"
            .to_vec(),
        "actual C log: {:?}",
        String::from_utf8_lossy(&obs.log)
    );
}

// ---------------------------------------------------------------------------
// create_task_manager allocation failures (ERRORS rows 11-14)
// ---------------------------------------------------------------------------

/// Body shared by the `create_task_manager`-returns-NULL rows.
fn create_null_case(label: &str, max_tasks: &str) -> Obs {
    let obs = diff(label, &Cfg::fresh().max(max_tasks), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = (api.create_task_manager)();
        rec.ptr_is_null(m as *const u8);
        rec.ret(m.is_null() as i64);
        (api.finalize_logger)();
    });
    assert_eq!(
        obs.rets,
        vec![0, 1, 1],
        "{label}: create_task_manager must return NULL"
    );
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n\
          [ERROR] Failed to allocate memory for tasks.\n\
          [INFO] Logger finalized.\n"
            .to_vec(),
        "{label}: wrong C log: {:?}",
        String::from_utf8_lossy(&obs.log)
    );
    obs
}

/// ERRORS rows 11 and 14 — `MAX_TASKS` so large that `max_tasks * 260` cannot
/// be allocated (520 GB and 558 GB).
fn err_09_create_tm_huge_max_tasks() {
    create_null_case("err_09 MAX_TASKS=2000000000", "2000000000");
    create_null_case("err_09 MAX_TASKS=2147483647", "2147483647");
}

/// ERRORS rows 12 and 13 — negative `MAX_TASKS`: the `int` is sign-extended
/// into `size_t` before the multiplication, wrapping to a huge byte count.
fn err_10_create_tm_negative_max_tasks() {
    create_null_case("err_10 MAX_TASKS=-1", "-1");
    create_null_case("err_10 MAX_TASKS=-2", "-2");
    create_null_case("err_10 MAX_TASKS=-1000", "-1000");
    create_null_case("err_10 MAX_TASKS=INT_MIN", "-2147483648");
}

// ---------------------------------------------------------------------------
// add_task rejection paths (ERRORS rows 15-18, 21)
// ---------------------------------------------------------------------------

/// ERRORS row 15 — the 11th task with the default limit of 10 is rejected, and
/// `task_count` must NOT be incremented by the rejected call.
fn err_11_add_task_limit_reached() {
    let obs = diff("err_11", &Cfg::fresh().max_unset(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = (api.create_task_manager)();
        for i in 0..13i32 {
            let d = cstr(format!("t{i}").as_bytes());
            (api.add_task)(m, d.as_ptr() as *const _, i);
            rec.manager(m); // task_count observed after every call
        }
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
    let text = String::from_utf8_lossy(&obs.log);
    assert_eq!(
        text.matches("[WARNING] Cannot add task: Maximum task limit reached.")
            .count(),
        3,
        "expected exactly 3 rejections"
    );
    assert_eq!(text.matches("[INFO] Task added successfully.").count(), 10);
    assert_eq!(
        String::from_utf8_lossy(&obs.stdout).lines().count(),
        11,
        "only the 10 accepted tasks may be printed"
    );
}

/// ERRORS row 16 — `max_tasks == 0`, whether from `MAX_TASKS=0` or from a
/// non-numeric value that `atoi` turns into 0: every `add_task` is rejected.
fn err_12_add_task_zero_max() {
    let _g = lock();
    for v in ["0", "abc", "", "x9", "-0", " ", "0x5"] {
        let obs = diff_locked(
            &format!("err_12 MAX_TASKS={v:?}"),
            &Cfg::fresh().max(v),
            |api, rec| unsafe {
                rec.ret((api.initialize_logger)());
                let m = (api.create_task_manager)();
                rec.ptr_is_null(m as *const u8);
                rec.manager(m);
                for i in 0..3i32 {
                    let d = cstr(b"nope");
                    (api.add_task)(m, d.as_ptr() as *const _, i);
                    rec.manager(m);
                }
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            },
        );
        // malloc(0) still returns a non-NULL pointer, so creation SUCCEEDS.
        assert_eq!(obs.rets, vec![0, 0], "MAX_TASKS={v:?}: create must succeed");
        let text = String::from_utf8_lossy(&obs.log);
        assert_eq!(
            text.matches("[WARNING] Cannot add task: Maximum task limit reached.")
                .count(),
            3,
            "MAX_TASKS={v:?}"
        );
        assert!(!text.contains("[INFO] Task added successfully."));
        assert_eq!(obs.stdout, b"Tasks:\n".to_vec());
    }
}

/// ERRORS row 17 — a caller-supplied `TaskManager` with a NEGATIVE `max_tasks`:
/// `0 >= -1` is true, so every `add_task` is rejected.
fn err_13_add_task_negative_max() {
    let _g = lock();
    for max in [-1i32, -2, -1000, i32::MIN] {
        diff_locked(
            &format!("err_13 max_tasks={max}"),
            &Cfg::fresh(),
            |api, rec| unsafe {
                rec.ret((api.initialize_logger)());
                // 0 slots allocated: nothing may be written to them.
                let m = craft_manager(0, 0, 0);
                (*m).max_tasks = max;
                (*m).task_count = 0;
                let d = cstr(b"rejected");
                (api.add_task)(m, d.as_ptr() as *const _, 5);
                rec.manager(m);
                (api.print_tasks)(m);
                free_manager(m);
                (api.finalize_logger)();
            },
        );
    }
    // Also: task_count already greater than max_tasks.
    diff_locked("err_13 count>max", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = craft_manager(4, 0, 0);
        (*m).max_tasks = 2;
        (*m).task_count = 3; // deliberately past the limit
        let d = cstr(b"rejected");
        (api.add_task)(m, d.as_ptr() as *const _, 5);
        rec.manager(m);
        free_manager(m);
        (api.finalize_logger)();
    });
}

/// ERRORS row 18 — silent truncation of over-long descriptions: `strncpy` with
/// n = 255 does not terminate, then `description[255] = '\0'`. No warning is
/// logged and the task IS stored.
fn err_14_add_task_truncation() {
    let _g = lock();
    for len in [256usize, 257, 300, 1000, 65536] {
        let obs = diff_locked(
            &format!("err_14 len={len}"),
            &Cfg::fresh(),
            |api, rec| unsafe {
                rec.ret((api.initialize_logger)());
                let m = (api.create_task_manager)();
                let body: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
                let d = cstr(&body);
                (api.add_task)(m, d.as_ptr() as *const _, 1);
                rec.manager(m);
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            },
        );
        let text = String::from_utf8_lossy(&obs.log);
        assert!(
            text.contains("[INFO] Task added successfully."),
            "len={len}: over-long descriptions must still be stored"
        );
        assert!(
            !text.contains("[WARNING]"),
            "len={len}: truncation must be silent"
        );
        // exactly 255 description bytes survive
        let row = &obs.stdout[b"Tasks:\n  [1] ".len()..];
        let stored = &row[..255];
        assert_eq!(stored.len(), 255);
        assert_eq!(&row[255..], b" (Priority: 1)\n");
        // ...and the unused tail of the 256-byte field is NUL-padded.
        let desc = &obs.extra[9..9 + DESC_LEN];
        assert_eq!(desc[255], 0, "len={len}: byte 255 must be the forced NUL");
    }
}

/// ERRORS row 21 — `description == NULL` **and** the limit already reached: the
/// limit check short-circuits, so the NULL is never dereferenced and there is
/// no crash.
fn err_16_null_desc_short_circuit() {
    let obs = diff("err_16", &Cfg::fresh().max("0"), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = (api.create_task_manager)();
        rec.ptr_is_null(m as *const u8);
        // NULL description, but max_tasks == 0 so the check fires first.
        (api.add_task)(m, std::ptr::null(), 1);
        rec.manager(m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n\
          [INFO] TaskManager created successfully.\n\
          [WARNING] Cannot add task: Maximum task limit reached.\n\
          [INFO] TaskManager destroyed successfully.\n\
          [INFO] Logger finalized.\n"
            .to_vec()
    );
    assert_eq!(obs.stdout, b"Tasks:\n".to_vec());
}

// ---------------------------------------------------------------------------
// print_tasks / destroy_task_manager edge cases (ERRORS rows 24, 26)
// ---------------------------------------------------------------------------

/// ERRORS row 24 — negative `task_count`: `for (i = 0; i < task_count; i++)`
/// never runs, so only the header is printed.
fn err_17_print_negative_count() {
    let _g = lock();
    for tc in [-1i32, -2, -1000, i32::MIN] {
        let obs = diff_locked(
            &format!("err_17 task_count={tc}"),
            &Cfg::fresh(),
            |api, rec| unsafe {
                rec.ret((api.initialize_logger)());
                let m = craft_manager(4, 0, 0);
                (*m).task_count = tc;
                (api.print_tasks)(m);
                rec.manager(m);
                free_manager(m);
                (api.finalize_logger)();
            },
        );
        assert_eq!(obs.stdout, b"Tasks:\n".to_vec(), "task_count={tc}");
    }
}

/// ERRORS row 26 — `manager->tasks == NULL` on destroy: `free(NULL)` is a
/// no-op, so this is not an error at all.
fn err_18_destroy_null_tasks() {
    let obs = diff("err_18", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = libc::malloc(TASKMANAGER_SIZE) as *mut TaskManager;
        (*m).tasks = std::ptr::null_mut();
        (*m).max_tasks = 7;
        (*m).task_count = 0;
        (api.print_tasks)(m); // task_count == 0, so `tasks` is never read
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
    assert_eq!(obs.stdout, b"Tasks:\n".to_vec());
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n\
          [INFO] TaskManager destroyed successfully.\n\
          [INFO] Logger finalized.\n"
            .to_vec()
    );
}

// ---------------------------------------------------------------------------
// driver error paths (ERRORS rows 27, 28, 31)
// ---------------------------------------------------------------------------

/// ERRORS row 27 — `initialize_logger` fails inside `driver`: returns
/// `EXIT_FAILURE` and never touches the task manager.
fn err_19_driver_logger_fail() {
    let _g = lock();
    for bad in [
        "/nonexistent_dir_xyz_abc_123/z.log",
        "",
        "/proc/self/mem/nope",
    ] {
        let cfg = Cfg::fresh().log(LogSetting::Explicit(bad.to_string()));
        let obs = diff_locked(&format!("err_19 {bad:?}"), &cfg, |api, rec| unsafe {
            let text = cstr(b"a\nb\nc\n");
            rec.ret((api.driver)(text.as_ptr() as *const _));
        });
        assert_eq!(
            obs.rets,
            vec![EXIT_FAILURE as i64],
            "{bad:?}: driver must return EXIT_FAILURE"
        );
        assert_eq!(
            obs.stderr,
            format!("Failed to open log file: {bad}\n").into_bytes()
        );
        assert!(
            obs.stdout.is_empty(),
            "{bad:?}: print_tasks must never run — got {:?}",
            String::from_utf8_lossy(&obs.stdout)
        );
    }
}

/// ERRORS row 28 — `create_task_manager` fails inside `driver`: returns
/// `EXIT_FAILURE`, and the logger is left **unfinalized** (C leaks it), so the
/// log must NOT end with `[INFO] Logger finalized.`
fn err_20_driver_create_fail() {
    let _g = lock();
    for max in ["2000000000", "-1", "2147483647", "-2147483648"] {
        let obs = diff_locked(
            &format!("err_20 MAX_TASKS={max}"),
            &Cfg::fresh().max(max),
            |api, rec| unsafe {
                let text = cstr(b"a\nb\n");
                rec.ret((api.driver)(text.as_ptr() as *const _));
            },
        );
        assert_eq!(obs.rets, vec![EXIT_FAILURE as i64], "MAX_TASKS={max}");
        assert!(obs.stdout.is_empty(), "MAX_TASKS={max}");
        assert!(obs.stderr.is_empty(), "MAX_TASKS={max}");
        assert_eq!(
            obs.log,
            b"[INFO] Logger initialized.\n\
              [ERROR] Failed to allocate memory for tasks.\n"
                .to_vec(),
            "MAX_TASKS={max}: the logger must be left open/unfinalized, got {:?}",
            String::from_utf8_lossy(&obs.log)
        );
    }
}

/// ERRORS row 31 — more input lines than `max_tasks`: the overflow lines are
/// warned about and dropped, `priority` keeps counting, and `driver` still
/// returns 0.
fn err_22_driver_more_lines_than_max() {
    let obs = diff("err_22", &Cfg::fresh().max("10"), |api, rec| unsafe {
        let mut text = Vec::new();
        for i in 0..15 {
            if i > 0 {
                text.push(b'\n');
            }
            text.extend_from_slice(format!("line{i}").as_bytes());
        }
        let buf = cstr(&text);
        rec.ret((api.driver)(buf.as_ptr() as *const _));
    });
    assert_eq!(obs.rets, vec![0]);
    let text = String::from_utf8_lossy(&obs.log);
    assert_eq!(
        text.matches("[WARNING] Cannot add task: Maximum task limit reached.")
            .count(),
        5
    );
    let printed = String::from_utf8_lossy(&obs.stdout);
    assert_eq!(printed.lines().count(), 11);
    assert!(printed.ends_with("  [10] line9 (Priority: 10)\n"));
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary rows (ERRORS rows 32, 33, 34)
// ---------------------------------------------------------------------------

/// ERRORS rows 32, 33, 34 — zero-length inputs, one-step-past-the-range values,
/// and extreme `int` arguments crossing the FFI boundary.
///
/// Note on "out-of-range enum values": the public API declares no `enum` at all
/// (see `ERRORS.md`), so the equivalent boundary is an arbitrary `c_int`, which
/// is what is fuzzed here.
fn err_23_generic_empty_and_bounds() {
    let _g = lock();

    // (a) zero-length everything
    let obs = diff_locked("err_23 empty", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let empty = cstr(b"");
        (api.log_info)(empty.as_ptr() as *const _);
        let m = (api.create_task_manager)();
        (api.add_task)(m, empty.as_ptr() as *const _, 0);
        rec.manager(m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        let d = cstr(b"");
        rec.ret((api.driver)(d.as_ptr() as *const _));
    });
    assert_eq!(obs.rets, vec![0, 0]);

    // (b) exactly at, and one step past, the description limit
    for len in [254usize, 255, 256] {
        diff_locked(
            &format!("err_23 desc len={len}"),
            &Cfg::fresh(),
            |api, rec| unsafe {
                rec.ret((api.initialize_logger)());
                let m = (api.create_task_manager)();
                let d = cstr(&vec![b'#'; len]);
                (api.add_task)(m, d.as_ptr() as *const _, len as c_int);
                rec.manager(m);
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            },
        );
    }

    // (c) exactly at, and one step past, the task limit
    for (max, adds) in [(1usize, 1usize), (1, 2), (5, 4), (5, 5), (5, 6)] {
        diff_locked(
            &format!("err_23 max={max} adds={adds}"),
            &Cfg::fresh().max(&max.to_string()),
            |api, rec| unsafe {
                rec.ret((api.initialize_logger)());
                let m = (api.create_task_manager)();
                for i in 0..adds {
                    let d = cstr(format!("x{i}").as_bytes());
                    (api.add_task)(m, d.as_ptr() as *const _, i as c_int);
                    rec.manager(m);
                }
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            },
        );
    }

    // (d) extreme / random `int` values crossing the FFI boundary
    const SEED: u64 = 0xB0B1_C0DE_1234_0023;
    diff_locked("err_23 int extremes", &Cfg::fresh().max("64"), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = (api.create_task_manager)();
        let mut rng = Rng::new(SEED);
        let mut vals: Vec<c_int> = vec![
            0,
            1,
            -1,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            i32::MAX - 1,
            -2147483647,
        ];
        while vals.len() < 64 {
            vals.push(rng.i32());
        }
        for v in &vals {
            let d = cstr(b"v");
            (api.add_task)(m, d.as_ptr() as *const _, *v);
        }
        rec.manager(m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

// ---------------------------------------------------------------------------
#[test]
fn phase_c_errors_all() {
    macro_rules! step {
        ($f:ident) => {{
            eprintln!("--> {}", stringify!($f));
            $f();
        }};
    }
    step!(err_01_init_logger_bad_path);
    step!(err_02_init_logger_dir_path);
    step!(err_03_init_logger_empty_path);
    step!(err_04_init_logger_perm_denied);
    step!(err_05_log_fns_before_init);
    step!(err_06_finalize_before_init);
    step!(err_07_log_null_message);
    step!(err_09_create_tm_huge_max_tasks);
    step!(err_10_create_tm_negative_max_tasks);
    step!(err_11_add_task_limit_reached);
    step!(err_12_add_task_zero_max);
    step!(err_13_add_task_negative_max);
    step!(err_14_add_task_truncation);
    step!(err_16_null_desc_short_circuit);
    step!(err_17_print_negative_count);
    step!(err_18_destroy_null_tasks);
    step!(err_19_driver_logger_fail);
    step!(err_20_driver_create_fail);
    step!(err_22_driver_more_lines_than_max);
    step!(err_23_generic_empty_and_bounds);
}
