//! Phase C — non-crashing error paths of `ERRORS.md`:
//! rows 1-6 (`fopen` failures), 11-14 (allocation failures), 15-17 (capacity
//! rejection), 23-25 (`driver` failure returns) and 29 (double init).
//!
//! Each test asserts two things:
//!   * C and Rust produce identical return value / stdout / stderr / log, and
//!   * the C outcome really is the error outcome (so the row is not vacuous).

mod common;

use common::{
    arm_malloc_failure, assert_same, cstring, disarm_malloc_failure, is_root, Config, LogTarget,
    Rng, TaskManager,
};
use std::ffi::{c_char, c_int};

const SEED: u64 = 0xE7707_0BAD_5EED;

// ---------------------------------------------------------------------------
// Rows 1-6: initialize_logger -> fopen fails -> stderr + return -1
// ---------------------------------------------------------------------------

/// `initialize_logger` alone, then a log call (which must stay a no-op because
/// `log_file` is still NULL) and `finalize_logger` (likewise a no-op).
fn init_only(api: &common::Api) -> i64 {
    unsafe {
        let msg = c"after failed init";
        let r = (api.initialize_logger)();
        (api.log_info)(msg.as_ptr());
        (api.finalize_logger)();
        r as i64
    }
}

fn expect_fopen_failure(tag: &str, log: LogTarget, path_shown: &str) {
    let cfg = Config::new().log(log);
    let out = assert_same(tag, &cfg, init_only);
    assert_eq!(out.ret, -1, "{tag}: initialize_logger should return -1");
    let expected = format!("Failed to open log file: {path_shown}\n");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        expected,
        "{tag}: stderr"
    );
    assert!(out.stdout.is_empty(), "{tag}: stdout should be empty");
}

#[test]
fn err01_fopen_directory() {
    // "." is the scratch directory the harness chdir'ed into: fopen(dir,"a")
    // fails with EISDIR.  Using a relative name keeps the string (and hence the
    // stderr message) identical on both sides.
    expect_fopen_failure("err01", LogTarget::Raw(".".into()), ".");
    expect_fopen_failure("err01-slash", LogTarget::Raw("/".into()), "/");
    expect_fopen_failure("err01-tmp", LogTarget::Raw("/tmp".into()), "/tmp");
}

#[test]
fn err02_fopen_empty_path() {
    expect_fopen_failure("err02", LogTarget::Raw(String::new()), "");
}

#[test]
fn err03_fopen_missing_parent() {
    expect_fopen_failure(
        "err03",
        LogTarget::Raw("nope/deeper/x.log".into()),
        "nope/deeper/x.log",
    );
    expect_fopen_failure(
        "err03-abs",
        LogTarget::Raw("/nonexistent-dir-9f3a/x.log".into()),
        "/nonexistent-dir-9f3a/x.log",
    );
}

#[test]
fn err04_fopen_readonly() {
    if is_root() {
        eprintln!("err04: running as root, EACCES cannot be provoked - skipped");
        return;
    }
    let cfg = Config::new().log(LogTarget::Raw("ro.log".into()));
    let out = assert_same("err04", &cfg, |api| {
        // The harness has chdir'ed into this side's private scratch dir.
        std::fs::write("ro.log", b"").unwrap();
        let mut perm = std::fs::metadata("ro.log").unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o444);
        std::fs::set_permissions("ro.log", perm).unwrap();
        init_only(api)
    });
    assert_eq!(out.ret, -1);
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "Failed to open log file: ro.log\n"
    );
}

#[test]
fn err05_fopen_notdir() {
    let cfg = Config::new().log(LogTarget::Raw("f.txt/x.log".into()));
    let out = assert_same("err05", &cfg, |api| {
        std::fs::write("f.txt", b"not a directory").unwrap();
        init_only(api)
    });
    assert_eq!(out.ret, -1);
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "Failed to open log file: f.txt/x.log\n"
    );
}

#[test]
fn err06_fopen_name_too_long() {
    let long = "a".repeat(5000);
    expect_fopen_failure("err06", LogTarget::Raw(long.clone()), &long);
    // One component longer than NAME_MAX (255) also fails, with ENAMETOOLONG.
    let comp = "b".repeat(300);
    expect_fopen_failure("err06-component", LogTarget::Raw(comp.clone()), &comp);
}

// ---------------------------------------------------------------------------
// Rows 11-14: malloc failures inside create_task_manager
// ---------------------------------------------------------------------------

#[test]
fn err11_manager_malloc_fails() {
    // sizeof(TaskManager) == 16.  The C logs "Failed to allocate memory for
    // TaskManager." and returns NULL *without* freeing anything.
    let out = assert_same("err11", &Config::new(), |api| unsafe {
        let r = (api.initialize_logger)();
        // No Rust allocation between arming and the call under test.
        let before = arm_malloc_failure(16);
        let m = (api.create_task_manager)();
        let fired = disarm_malloc_failure(before);
        let code = (m.is_null() as i64) | ((fired as i64) << 1) | ((r as i64) << 2);
        if !m.is_null() {
            (api.destroy_task_manager)(m);
        }
        (api.finalize_logger)();
        code
    });
    assert_eq!(
        out.ret, 0b011,
        "expected the 16-byte malloc to fail and create_task_manager to return NULL"
    );
    let log = String::from_utf8_lossy(&out.log);
    assert!(
        log.contains("[ERROR] Failed to allocate memory for TaskManager."),
        "{log}"
    );
    assert!(
        !log.contains("TaskManager created successfully"),
        "{log}"
    );
}

/// `initialize_logger` -> `create_task_manager` -> (destroy if non-NULL) ->
/// `finalize_logger`; returns 1 when the manager came back NULL.
fn create_only(api: &common::Api) -> i64 {
    unsafe {
        let r = (api.initialize_logger)();
        let m = (api.create_task_manager)();
        let null = m.is_null() as i64;
        if !m.is_null() {
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        }
        (api.finalize_logger)();
        null | ((r as i64) << 1)
    }
}

fn expect_tasks_alloc_failure(tag: &str, max_tasks: &str) {
    let cfg = Config::new().max_tasks(max_tasks);
    let out = assert_same(tag, &cfg, create_only);
    assert_eq!(
        out.ret, 1,
        "{tag}: create_task_manager should have returned NULL"
    );
    let log = String::from_utf8_lossy(&out.log);
    assert!(
        log.contains("[ERROR] Failed to allocate memory for tasks."),
        "{tag}: {log}"
    );
    assert!(out.stdout.is_empty(), "{tag}: stdout should be empty");
}

#[test]
fn err12_tasks_size_wraps() {
    // max_tasks == -1 -> (size_t)(-1) * 260 wraps to 0xFFFFFFFFFFFFFEFC.
    assert_eq!(common::tasks_alloc_bytes(-1), 0xFFFF_FFFF_FFFF_FEFC);
    for v in ["-1", "-2", "-1000", "-2147483648", "2147483648", "-2147483649"] {
        expect_tasks_alloc_failure(&format!("err12-{v}"), v);
    }
}

#[test]
fn err13_tasks_too_big() {
    // 2_000_000_000 * 260 == 520 GB.
    for v in ["2000000000", "1000000000", "99999999999999999999"] {
        expect_tasks_alloc_failure(&format!("err13-{v}"), v);
    }
}

#[test]
fn err14_tasks_malloc_fails() {
    // Same branch, provoked with an ordinary capacity: 10 * 260 == 2600.
    let out = assert_same("err14", &Config::new(), |api| unsafe {
        let r = (api.initialize_logger)();
        let before = arm_malloc_failure(2600);
        let m = (api.create_task_manager)();
        let fired = disarm_malloc_failure(before);
        let code = (m.is_null() as i64) | ((fired as i64) << 1) | ((r as i64) << 2);
        if !m.is_null() {
            (api.destroy_task_manager)(m);
        }
        (api.finalize_logger)();
        code
    });
    assert_eq!(out.ret, 0b011);
    let log = String::from_utf8_lossy(&out.log);
    assert!(
        log.contains("[ERROR] Failed to allocate memory for tasks."),
        "{log}"
    );
}

// ---------------------------------------------------------------------------
// Rows 15-17: add_task capacity rejection
// ---------------------------------------------------------------------------

fn fill(api: &common::Api, cap_hint: usize, extra: usize, rng_seed: u64) -> i64 {
    let mut rng = Rng::new(rng_seed);
    let items: Vec<(Vec<u8>, i32)> = (0..cap_hint + extra)
        .map(|_| {
            let n = rng.below(60);
            let body = rng.cstr_body(n);
            (cstring(&body), rng.priority())
        })
        .collect();
    unsafe {
        let r = (api.initialize_logger)();
        let m = (api.create_task_manager)();
        if m.is_null() {
            (api.finalize_logger)();
            return -1;
        }
        for (d, p) in &items {
            (api.add_task)(m, d.as_ptr() as *const c_char, *p as c_int);
        }
        (api.print_tasks)(m);
        let tc = (*m).task_count as i64;
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
        tc | ((r as i64) << 32)
    }
}

#[test]
fn err15_limit_reached() {
    for cap in [1usize, 2, 3, 10] {
        for extra in [1usize, 2, 9] {
            let cfg = Config::new().max_tasks(cap.to_string());
            let tag = format!("err15-{cap}+{extra}");
            let out = assert_same(&tag, &cfg, |api| fill(api, cap, extra, SEED + cap as u64));
            assert_eq!(out.ret, cap as i64, "{tag}: task_count must stop at max_tasks");
            let log = String::from_utf8_lossy(&out.log);
            let warnings = log
                .matches("[WARNING] Cannot add task: Maximum task limit reached.")
                .count();
            assert_eq!(warnings, extra, "{tag}: one warning per rejected add");
        }
    }
}

#[test]
fn err16_zero_capacity() {
    // $MAX_TASKS=0: malloc(0) returns a non-NULL pointer, so the manager is
    // created, but 0 >= 0 rejects every single add.
    let cfg = Config::new().max_tasks("0");
    let out = assert_same("err16", &cfg, |api| fill(api, 0, 5, SEED + 100));
    assert_eq!(out.ret, 0);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "Tasks:\n",
        "only the header should be printed"
    );
    let log = String::from_utf8_lossy(&out.log);
    assert_eq!(
        log.matches("[WARNING] Cannot add task: Maximum task limit reached.")
            .count(),
        5
    );
    assert!(!log.contains("[INFO] Task added successfully."), "{log}");
}

#[test]
fn err17_negative_capacity() {
    // A hand-built manager with max_tasks < 0: `0 >= -1` rejects the add before
    // the (NULL) tasks array is ever touched.
    for max_tasks in [-1i32, -100, i32::MIN] {
        let desc = cstring(b"rejected");
        let mut tm = TaskManager {
            tasks: std::ptr::null_mut(),
            max_tasks,
            task_count: 0,
        };
        let p: *mut TaskManager = &mut tm;
        let out = assert_same(&format!("err17-{max_tasks}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            (api.add_task)(p, desc.as_ptr() as *const c_char, 7);
            (api.print_tasks)(p);
            let tc = (*p).task_count as i64;
            (api.finalize_logger)();
            tc | ((r as i64) << 32)
        });
        assert_eq!(out.ret, 0, "task_count must be untouched");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "Tasks:\n");
        let log = String::from_utf8_lossy(&out.log);
        assert!(
            log.contains("[WARNING] Cannot add task: Maximum task limit reached."),
            "{log}"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 23-25: driver failure returns
// ---------------------------------------------------------------------------

#[test]
fn err23_driver_logger_fails() {
    let blob = cstring(b"one\ntwo\nthree");
    let cfg = Config::new().log(LogTarget::Raw(".".into()));
    let out = assert_same("err23", &cfg, |api| unsafe {
        (api.driver)(blob.as_ptr() as *const c_char) as i64
    });
    assert_eq!(out.ret, 1, "driver must return EXIT_FAILURE");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "Failed to open log file: .\n"
    );
    assert!(
        out.stdout.is_empty(),
        "driver must bail out before print_tasks"
    );
}

#[test]
fn err24_driver_manager_fails() {
    let blob = cstring(b"one\ntwo");
    let cfg = Config::new().max_tasks("-1");
    let out = assert_same("err24", &cfg, |api| unsafe {
        (api.driver)(blob.as_ptr() as *const c_char) as i64
    });
    assert_eq!(out.ret, 1, "driver must return EXIT_FAILURE");
    assert!(out.stdout.is_empty(), "nothing should reach stdout");
    let log = String::from_utf8_lossy(&out.log);
    assert!(log.contains("[INFO] Logger initialized."), "{log}");
    assert!(
        log.contains("[ERROR] Failed to allocate memory for tasks."),
        "{log}"
    );
    // This early-return path skips finalize_logger, so the C never writes the
    // "Logger finalized." line (the handle is simply leaked).
    assert!(!log.contains("Logger finalized."), "{log}");
}

#[test]
fn err25_driver_task_malloc_fails() {
    // One 100-byte line -> driver calls malloc(101) for the copy.  Sizes 16 and
    // 2600 are consumed by create_task_manager first, so arming 101 targets
    // exactly the per-line allocation.
    let line: Vec<u8> = vec![b'z'; 100];
    let blob = cstring(&line);
    let out = assert_same("err25", &Config::new(), |api| unsafe {
        let before = arm_malloc_failure(101);
        let r = (api.driver)(blob.as_ptr() as *const c_char) as i64;
        let fired = disarm_malloc_failure(before);
        r | ((fired as i64) << 8)
    });
    assert_eq!(
        out.ret,
        1 | (1 << 8),
        "the 101-byte malloc must fail and driver must return EXIT_FAILURE"
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "Error: Failed to allocate memory for task.\n"
    );
    assert!(
        out.stdout.is_empty(),
        "driver must bail out before print_tasks"
    );
    let log = String::from_utf8_lossy(&out.log);
    // This path *does* clean up.
    assert!(
        log.contains("[INFO] TaskManager destroyed successfully."),
        "{log}"
    );
    assert!(log.contains("[INFO] Logger finalized."), "{log}");
    assert!(!log.contains("[INFO] Task added successfully."), "{log}");

    // Same thing on a later line: the first two lines are added, the third
    // allocation fails.
    let blob2 = cstring(b"aa\nbbb\ncccc");
    let out = assert_same("err25-third-line", &Config::new(), |api| unsafe {
        let before = arm_malloc_failure(5); // strlen("cccc") + 1
        let r = (api.driver)(blob2.as_ptr() as *const c_char) as i64;
        let fired = disarm_malloc_failure(before);
        r | ((fired as i64) << 8)
    });
    assert_eq!(out.ret, 1 | (1 << 8));
    let log = String::from_utf8_lossy(&out.log);
    assert_eq!(log.matches("[INFO] Task added successfully.").count(), 2);
}

// ---------------------------------------------------------------------------
// Row 29: initialize_logger called twice
// ---------------------------------------------------------------------------

#[test]
fn err29_double_initialize() {
    // The C overwrites `log_file` without closing the previous handle: the
    // first stream stays open and its buffered bytes are only written when the
    // process (or an fflush) gets around to it.
    let a = cstring(b"before-second-init");
    let b = cstring(b"after-second-init");
    let out = assert_same("err29", &Config::new(), |api| unsafe {
        let r1 = (api.initialize_logger)();
        (api.log_info)(a.as_ptr() as *const c_char);
        let r2 = (api.initialize_logger)();
        (api.log_info)(b.as_ptr() as *const c_char);
        (api.finalize_logger)();
        (r1 as i64) * 10 + r2 as i64
    });
    assert_eq!(out.ret, 0, "both initialize_logger calls must succeed");
    let log = String::from_utf8_lossy(&out.log);
    assert!(log.contains("before-second-init"), "{log}");
    assert!(log.contains("after-second-init"), "{log}");
    // Two successful opens => two "Logger initialized." lines.
    assert_eq!(log.matches("[INFO] Logger initialized.").count(), 2, "{log}");
}
