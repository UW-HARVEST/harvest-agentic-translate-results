//! Phase C — error-path differential tests, one test per `ERRORS.md` row
//! (E1..E20) plus the generic FFI boundary rows (G1..G7).
//!
//! Every call goes through `dlopen`/`dlsym` on the two `.so` files.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::ptr;

/// Result of running one error scenario against one implementation.
#[derive(Debug, PartialEq, Eq)]
struct Out {
    /// Every value the C API returned, in order (`initialize_logger` codes,
    /// `driver` codes, `create_task_manager` NULL-ness encoded as 0/1).
    rets: Vec<c_int>,
    /// `(max_tasks, task_count)` snapshots plus full task bytes.
    state: Vec<(c_int, c_int, Vec<Vec<u8>>)>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    log: Vec<u8>,
}

fn env(log: Option<&Path>, max_tasks: Option<&str>) {
    match log {
        Some(p) => set_env("LOG_FILE", p.to_str().unwrap()),
        None => unset_env("LOG_FILE"),
    }
    match max_tasks {
        Some(v) => set_env("MAX_TASKS", v),
        None => unset_env("MAX_TASKS"),
    }
}

/// Runs `body` against one implementation, capturing returns, stdout and log.
fn run<F>(lib: &Lib, log: Option<&Path>, max_tasks: Option<&str>, body: F) -> Out
where
    F: FnOnce(&Lib, &mut Vec<c_int>, &mut Vec<(c_int, c_int, Vec<Vec<u8>>)>),
{
    env(log, max_tasks);
    let mut rets = Vec::new();
    let mut state = Vec::new();
    let ((), stdout, stderr) = capture_out_err(|| body(lib, &mut rets, &mut state));
    let log_bytes = match log {
        Some(p) => read_log(&p.to_path_buf()),
        None => Vec::new(),
    };
    Out {
        rets,
        state,
        stdout,
        stderr,
        log: log_bytes,
    }
}

fn diff_err<F>(ctx: &str, tag: &str, max_tasks: Option<&str>, use_log: bool, body: F)
where
    F: Fn(&Lib, &mut Vec<c_int>, &mut Vec<(c_int, c_int, Vec<Vec<u8>>)>) + Copy,
{
    let (libs, _g) = libs();
    let logs = fresh_logs(tag);
    let (lc, lr) = if use_log {
        (Some(logs.c.as_path()), Some(logs.rust.as_path()))
    } else {
        (None, None)
    };
    let c = run(&libs.c, lc, max_tasks, body);
    let r = run(&libs.rust, lr, max_tasks, body);
    if c.rets != r.rets {
        panic!(
            "return codes differ [{ctx}]\n  C   : {:?}\n  Rust: {:?}",
            c.rets, r.rets
        );
    }
    if c.state != r.state {
        panic!(
            "manager state differs [{ctx}]\n  C   : {:?}\n  Rust: {:?}",
            c.state, r.state
        );
    }
    assert_bytes_eq("stdout", ctx, &c.stdout, &r.stdout);
    assert_bytes_eq("stderr", ctx, &c.stderr, &r.stderr);
    assert_bytes_eq("log file", ctx, &c.log, &r.log);
    let _ = std::fs::remove_file(&logs.c);
    let _ = std::fs::remove_file(&logs.rust);
}

/// Variant for scenarios where `LOG_FILE` must be one specific (unopenable)
/// value for BOTH implementations. No log file can be produced in that case, so
/// only returns / stdout / stderr / manager state are compared — and stderr is
/// exactly what these rows are about (`Failed to open log file: %s`).
fn diff_err_fixed_env<F>(ctx: &str, log_env: Option<&str>, max_tasks: Option<&str>, body: F)
where
    F: Fn(&Lib, &mut Vec<c_int>, &mut Vec<(c_int, c_int, Vec<Vec<u8>>)>) + Copy,
{
    let (libs, _g) = libs();
    let lp = log_env.map(PathBuf::from);
    let c = run(&libs.c, lp.as_deref(), max_tasks, body);
    let r = run(&libs.rust, lp.as_deref(), max_tasks, body);
    if c.rets != r.rets {
        panic!(
            "return codes differ [{ctx}]\n  C   : {:?}\n  Rust: {:?}",
            c.rets, r.rets
        );
    }
    if c.state != r.state {
        panic!(
            "manager state differs [{ctx}]\n  C   : {:?}\n  Rust: {:?}",
            c.state, r.state
        );
    }
    assert_bytes_eq("stdout", ctx, &c.stdout, &r.stdout);
    assert_bytes_eq("stderr", ctx, &c.stderr, &r.stderr);
}

/// Forces the module-level `log_file` back to NULL in *both* libraries, exactly
/// the way the C does it: an `initialize_logger` whose `fopen` fails assigns
/// NULL. Needed by the "called before init" rows.
fn force_log_file_null(lib: &Lib) {
    set_env("LOG_FILE", "/nonexistent-dir-xyz-42/f.log");
    let rc = unsafe { (lib.initialize_logger())() };
    assert_eq!(rc, -1, "expected fopen failure to reset log_file to NULL");
}

fn snap(mgr: *const TaskManager) -> (c_int, c_int, Vec<Vec<u8>>) {
    unsafe {
        if mgr.is_null() {
            (-999, -999, Vec::new())
        } else {
            snapshot(mgr)
        }
    }
}

// ============================================== E1..E3 initialize_logger fopen ==

fn unopenable_paths() -> Vec<String> {
    let mut v = vec![
        "/nonexistent-dir-xyz-42/some.log".to_string(),
        "/proc/self/no/such/place.log".to_string(),
    ];
    // A read-only regular file: fopen(..., "a") fails with EACCES. Different
    // errno from the ENOENT/EISDIR cases, same -1 result.
    let ro = tmp_path("e1-readonly.log");
    if std::fs::write(&ro, b"").is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&ro).unwrap().permissions();
        perms.set_mode(0o444);
        if std::fs::set_permissions(&ro, perms).is_ok() {
            // Root ignores the mode bits; only use it if it really is unopenable.
            if std::fs::OpenOptions::new().append(true).open(&ro).is_err() {
                v.push(ro.to_str().unwrap().to_string());
            }
        }
    }
    assert!(
        v.len() >= 3,
        "expected the read-only-file fopen failure mode to be usable; running as \
         root disables it (uid must not be 0)"
    );
    v
}

#[test]
fn err_e1_initialize_logger_fopen_fails() {
    for p in unopenable_paths() {
        // Compares the -1 return AND the exact `Failed to open log file: %s`
        // bytes on stderr, plus the fact that the failure leaves log_file NULL
        // so the following log_info is a silent no-op.
        diff_err_fixed_env(
            &format!("E1 LOG_FILE={p}"),
            Some(&p),
            None,
            |lib, rets, _st| unsafe {
                rets.push((lib.initialize_logger())());
                (lib.log_info())(cstr_of("after failure").as_ptr());
                (lib.log_warning())(cstr_of("after failure").as_ptr());
                (lib.log_error())(cstr_of("after failure").as_ptr());
                (lib.finalize_logger())();
            },
        );
        // And the sentinel really is -1, not just "equal on both sides".
        let (libs, _g) = libs();
        set_env("LOG_FILE", &p);
        let (rc, _o, _e) = capture_out_err(|| unsafe { (libs.c.initialize_logger())() });
        assert_eq!(rc, -1, "C must return -1 for {p}");
        set_env("LOG_FILE", &p);
        let (rc2, _o, _e) = capture_out_err(|| unsafe { (libs.rust.initialize_logger())() });
        assert_eq!(rc2, -1, "Rust must return -1 for {p}");
    }
}

#[test]
fn err_e2_initialize_logger_empty_path() {
    diff_err_fixed_env("E2 LOG_FILE=\"\"", Some(""), None, |lib, rets, _st| unsafe {
        rets.push((lib.initialize_logger())());
    });
    let (libs, _g) = libs();
    set_env("LOG_FILE", "");
    let (rc, _o, _e) = capture_out_err(|| unsafe { (libs.c.initialize_logger())() });
    assert_eq!(rc, -1, "fopen(\"\") must fail");
    set_env("LOG_FILE", "");
    let (rc2, _o, _e) = capture_out_err(|| unsafe { (libs.rust.initialize_logger())() });
    assert_eq!(rc2, -1);
}

#[test]
fn err_e3_initialize_logger_path_is_dir() {
    let d = tmp_path("e3dir");
    std::fs::create_dir_all(&d).unwrap();
    let p = d.to_str().unwrap().to_string();
    diff_err_fixed_env(&format!("E3 LOG_FILE={p} (dir)"), Some(&p), None, |lib, rets, _st| unsafe {
        rets.push((lib.initialize_logger())());
    });
    let (libs, _g) = libs();
    set_env("LOG_FILE", &p);
    let (rc, _o, _e) = capture_out_err(|| unsafe { (libs.c.initialize_logger())() });
    assert_eq!(rc, -1, "fopen(<dir>, \"a\") must fail");
    set_env("LOG_FILE", &p);
    let (rc2, _o, _e) = capture_out_err(|| unsafe { (libs.rust.initialize_logger())() });
    assert_eq!(rc2, -1);
    let _ = std::fs::remove_dir_all(&d);
}

// ================================= E4/E5/E6 log_* with log_file == NULL, E7 finalize ==

#[test]
fn err_e4_log_fns_before_init() {
    let (libs, _g) = libs();
    let logs = fresh_logs("e4");
    // Bring both libraries to the log_file == NULL state.
    force_log_file_null(&libs.c);
    force_log_file_null(&libs.rust);

    let msgs = ["", "x", "a longer message with spaces"];
    let (_, out_c) = capture_stdout(|| unsafe {
        for m in msgs {
            (libs.c.log_info())(cstr_of(m).as_ptr());
            (libs.c.log_warning())(cstr_of(m).as_ptr());
            (libs.c.log_error())(cstr_of(m).as_ptr());
        }
    });
    let (_, out_r) = capture_stdout(|| unsafe {
        for m in msgs {
            (libs.rust.log_info())(cstr_of(m).as_ptr());
            (libs.rust.log_warning())(cstr_of(m).as_ptr());
            (libs.rust.log_error())(cstr_of(m).as_ptr());
        }
    });
    assert_bytes_eq("stdout", "E4/E5/E6", &out_c, &out_r);
    assert!(out_c.is_empty(), "no output expected, got {}", show(&out_c));
    // Nothing must have been created at either log path either.
    assert!(read_log(&logs.c).is_empty());
    assert!(read_log(&logs.rust).is_empty());
}

#[test]
fn err_e7_finalize_without_init() {
    let (libs, _g) = libs();
    force_log_file_null(&libs.c);
    force_log_file_null(&libs.rust);
    let (_, out_c) = capture_stdout(|| unsafe { (libs.c.finalize_logger())() });
    let (_, out_r) = capture_stdout(|| unsafe { (libs.rust.finalize_logger())() });
    assert_bytes_eq("stdout", "E7", &out_c, &out_r);
    assert!(out_c.is_empty());
    // A second finalize must also be a no-op in both.
    let (_, out_c2) = capture_stdout(|| unsafe { (libs.c.finalize_logger())() });
    let (_, out_r2) = capture_stdout(|| unsafe { (libs.rust.finalize_logger())() });
    assert_bytes_eq("stdout (2nd)", "E7", &out_c2, &out_r2);
}

// ============================== E8/E9/E10 create_task_manager allocation failure ==

/// `MAX_TASKS` values whose `max_tasks * sizeof(Task)` cannot be allocated.
/// Includes negative values, which C converts to a huge `size_t`.
const HUGE_MAX_TASKS: &[&str] = &[
    "1000000000",   // 1e9 * 260 = 260 GB
    "2000000000",   // 520 GB
    "2147483647",   // INT_MAX * 260
    "-1",           // (size_t)(-1) * 260, wraps
    "-2",
    "-1000",
    "-2147483648",  // INT_MIN
];

#[test]
fn err_e9_create_tasks_alloc_fails() {
    for v in ["1000000000", "2000000000", "2147483647"] {
        diff_err(&format!("E9 MAX_TASKS={v}"), "e9", Some(v), true, |lib, rets, state| unsafe {
            rets.push((lib.initialize_logger())());
            let m = (lib.create_task_manager())();
            rets.push(if m.is_null() { 1 } else { 0 });
            state.push(snap(m));
            if !m.is_null() {
                (lib.destroy_task_manager())(m);
            }
            (lib.finalize_logger())();
        });
    }
}

#[test]
fn err_e10_create_negative_max_tasks() {
    for v in ["-1", "-2", "-1000", "-2147483648"] {
        diff_err(&format!("E10 MAX_TASKS={v}"), "e10", Some(v), true, |lib, rets, state| unsafe {
            rets.push((lib.initialize_logger())());
            let m = (lib.create_task_manager())();
            rets.push(if m.is_null() { 1 } else { 0 });
            state.push(snap(m));
            if !m.is_null() {
                (lib.destroy_task_manager())(m);
            }
            (lib.finalize_logger())();
        });
    }
}

/// E8 is unreachable (a 16-byte malloc cannot be made to fail), but the *shape*
/// of the whole `HUGE_MAX_TASKS` set is still asserted here so no value in that
/// set diverges.
#[test]
fn err_e8_e9_e10_all_huge_values() {
    for v in HUGE_MAX_TASKS {
        diff_err(&format!("E8/9/10 MAX_TASKS={v}"), "e8", Some(v), true, |lib, rets, state| unsafe {
            rets.push((lib.initialize_logger())());
            let m = (lib.create_task_manager())();
            rets.push(if m.is_null() { 1 } else { 0 });
            state.push(snap(m));
            if !m.is_null() {
                (lib.destroy_task_manager())(m);
            }
            (lib.finalize_logger())();
        });
    }
}

// ================================================ E11..E15 add_task rejections ==

fn add_many(n: usize) -> impl Fn(&Lib, &mut Vec<c_int>, &mut Vec<(c_int, c_int, Vec<Vec<u8>>)>) + Copy {
    move |lib, rets, state| unsafe {
        rets.push((lib.initialize_logger())());
        let m = (lib.create_task_manager())();
        rets.push(if m.is_null() { 1 } else { 0 });
        if !m.is_null() {
            for i in 0..n {
                let d = cstr_of(&format!("task-{i}"));
                (lib.add_task())(m, d.as_ptr(), i as c_int + 1);
                state.push(snap(m));
            }
            (lib.print_tasks())(m);
            state.push(snap(m));
            (lib.destroy_task_manager())(m);
        }
        (lib.finalize_logger())();
    }
}

#[test]
fn err_e11_add_task_full() {
    diff_err("E11 MAX_TASKS=4, add 9", "e11", Some("4"), true, add_many(9));
    diff_err("E11 MAX_TASKS=10, add 13", "e11", Some("10"), true, add_many(13));
    diff_err("E11 MAX_TASKS=1, add 5", "e11", Some("1"), true, add_many(5));
}

#[test]
fn err_e12_add_task_max_zero() {
    diff_err("E12 MAX_TASKS=0, add 3", "e12", Some("0"), true, add_many(3));
}

#[test]
fn err_e13_add_task_max_nonnumeric() {
    for v in ["abc", "", "x1", "-", "+", " ", "\t", "null"] {
        diff_err(
            &format!("E13 MAX_TASKS={v:?}"),
            "e13",
            Some(v),
            true,
            add_many(3),
        );
    }
}

#[test]
fn err_e14_add_task_truncation() {
    for len in [255usize, 256, 257, 300, 1000, 4096] {
        let long: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
        let s = cstr(&long);
        let ctx = format!("E14 len={len}");
        let sp = s.as_ptr();
        diff_err(&ctx, "e14", Some("2"), true, move |lib, rets, state| unsafe {
            rets.push((lib.initialize_logger())());
            let m = (lib.create_task_manager())();
            rets.push(if m.is_null() { 1 } else { 0 });
            if !m.is_null() {
                (lib.add_task())(m, sp, 5);
                state.push(snap(m));
                (lib.print_tasks())(m);
                (lib.destroy_task_manager())(m);
            }
            (lib.finalize_logger())();
        });
        drop(s);
    }
}

#[test]
fn err_e15_add_task_empty_desc() {
    diff_err("E15 empty description", "e15", Some("3"), true, |lib, rets, state| unsafe {
        rets.push((lib.initialize_logger())());
        let m = (lib.create_task_manager())();
        rets.push(if m.is_null() { 1 } else { 0 });
        if !m.is_null() {
            let e = cstr_of("");
            (lib.add_task())(m, e.as_ptr(), 0);
            (lib.add_task())(m, e.as_ptr(), -1);
            state.push(snap(m));
            (lib.print_tasks())(m);
            (lib.destroy_task_manager())(m);
        }
        (lib.finalize_logger())();
    });
}

// ==================================================== E16..E20 driver failures ==

#[test]
fn err_e16_driver_logger_fails() {
    for p in unopenable_paths() {
        diff_err_fixed_env(
            &format!("E16 LOG_FILE={p}"),
            Some(&p),
            None,
            |lib, rets, _st| unsafe {
                let input = cstr_of("a\nb\nc");
                rets.push((lib.driver())(input.as_ptr()));
            },
        );
        // The sentinel is EXIT_FAILURE == 1 and stdout stays empty.
        let (libs, _g) = libs();
        let input = cstr_of("a\nb\nc");
        set_env("LOG_FILE", &p);
        unset_env("MAX_TASKS");
        let (rc_c, out_c, err_c) = capture_out_err(|| unsafe { (libs.c.driver())(input.as_ptr()) });
        set_env("LOG_FILE", &p);
        let (rc_r, out_r, err_r) =
            capture_out_err(|| unsafe { (libs.rust.driver())(input.as_ptr()) });
        assert_eq!(rc_c, 1, "C driver must return EXIT_FAILURE for {p}");
        assert_eq!(rc_c, rc_r, "E16 return differs for {p}");
        assert_bytes_eq("stdout", "E16", &out_c, &out_r);
        assert_bytes_eq("stderr", "E16", &err_c, &err_r);
        assert!(out_c.is_empty(), "nothing should reach stdout: {}", show(&out_c));
        assert!(
            !err_c.is_empty() && err_c.starts_with(b"Failed to open log file: "),
            "expected the C's stderr diagnostic, got {}",
            show(&err_c)
        );
    }
}

#[test]
fn err_e17_driver_manager_fails() {
    for v in HUGE_MAX_TASKS {
        diff_err(
            &format!("E17 MAX_TASKS={v}"),
            "e17",
            Some(v),
            true,
            |lib, rets, _state| unsafe {
                let input = cstr_of("one\ntwo\nthree");
                rets.push((lib.driver())(input.as_ptr()));
            },
        );
    }
}

#[test]
fn err_e19_driver_empty_input() {
    diff_err("E19 empty input", "e19", None, true, |lib, rets, _s| unsafe {
        let e = cstr_of("");
        rets.push((lib.driver())(e.as_ptr()));
    });
}

#[test]
fn err_e20_driver_overflow_lines() {
    for (max, lines) in [("1", 6usize), ("2", 7), ("3", 10), ("5", 5), ("5", 6)] {
        let input: String = (0..lines)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let s = cstr_of(&input);
        let sp = s.as_ptr();
        diff_err(
            &format!("E20 MAX_TASKS={max} lines={lines}"),
            "e20",
            Some(max),
            true,
            move |lib, rets, _st| unsafe {
                rets.push((lib.driver())(sp));
            },
        );
        drop(s);
    }
}

// ======================================================= G1..G7 generic bounds ==

#[test]
fn bnd_g1_priority_extremes() {
    for p in [
        0i32,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        -2147483647,
        123456789,
    ] {
        diff_err(
            &format!("G1 priority={p}"),
            "g1",
            Some("3"),
            true,
            move |lib, rets, state| unsafe {
                rets.push((lib.initialize_logger())());
                let m = (lib.create_task_manager())();
                rets.push(if m.is_null() { 1 } else { 0 });
                if !m.is_null() {
                    let d = cstr_of("prio");
                    (lib.add_task())(m, d.as_ptr(), p);
                    state.push(snap(m));
                    (lib.print_tasks())(m);
                    (lib.destroy_task_manager())(m);
                }
                (lib.finalize_logger())();
            },
        );
    }
}

#[test]
fn bnd_g2_log_format_specifiers() {
    // The message is an *argument* to fprintf, never the format string, so all
    // of these must appear literally in the log.
    const NASTY: &[&str] = &[
        "%s", "%d", "%n", "%p", "%%", "%1000000d", "%s%s%s%s%s%s%s%s", "100% done", "%.*s",
    ];
    diff_err("G2 format specifiers", "g2", None, true, |lib, rets, _s| unsafe {
        rets.push((lib.initialize_logger())());
        for m in NASTY {
            let c = cstr_of(m);
            (lib.log_info())(c.as_ptr());
            (lib.log_warning())(c.as_ptr());
            (lib.log_error())(c.as_ptr());
        }
        (lib.finalize_logger())();
    });
    // …and the same through print_tasks (printf with a %s argument).
    diff_err("G2 via print_tasks", "g2b", Some("16"), true, |lib, rets, state| unsafe {
        rets.push((lib.initialize_logger())());
        let m = (lib.create_task_manager())();
        rets.push(if m.is_null() { 1 } else { 0 });
        if !m.is_null() {
            for (i, s) in NASTY.iter().enumerate() {
                let c = cstr_of(s);
                (lib.add_task())(m, c.as_ptr(), i as c_int);
            }
            state.push(snap(m));
            (lib.print_tasks())(m);
            (lib.destroy_task_manager())(m);
        }
        (lib.finalize_logger())();
    });
}

#[test]
fn bnd_g3_log_lengths() {
    for len in [0usize, 1, 2, 255, 256, 257, 4095, 4096, 65535, 100_000] {
        let msg: Vec<u8> = std::iter::repeat(b'z').take(len).collect();
        let s = cstr(&msg);
        let sp = s.as_ptr();
        diff_err(
            &format!("G3 len={len}"),
            "g3",
            None,
            true,
            move |lib, rets, _st| unsafe {
                rets.push((lib.initialize_logger())());
                (lib.log_info())(sp);
                (lib.log_warning())(sp);
                (lib.log_error())(sp);
                (lib.finalize_logger())();
            },
        );
        drop(s);
    }
}

#[test]
fn bnd_g4_desc_boundary_lengths() {
    for len in [0usize, 1, 2, 253, 254, 255, 256, 257, 258, 511, 512, 513] {
        let d: Vec<u8> = (0..len).map(|i| b'0' + (i % 10) as u8).collect();
        let s = cstr(&d);
        let sp = s.as_ptr();
        diff_err(
            &format!("G4 len={len}"),
            "g4",
            Some("2"),
            true,
            move |lib, rets, state| unsafe {
                rets.push((lib.initialize_logger())());
                let m = (lib.create_task_manager())();
                rets.push(if m.is_null() { 1 } else { 0 });
                if !m.is_null() {
                    (lib.add_task())(m, sp, 1);
                    state.push(snap(m));
                    (lib.print_tasks())(m);
                    (lib.destroy_task_manager())(m);
                }
                (lib.finalize_logger())();
            },
        );
        drop(s);
    }
}

#[test]
fn bnd_g5_driver_newline_shapes() {
    for s in [
        "", "\n", "\n\n", "\n\n\n", "\n\n\n\n", "a", "a\n", "\na", "a\n\n", "\n\na", "a\nb",
        "a\n\nb", "\na\n", "\n\na\n\n", "x\n\n\n\ny", " \n \n ", "\t\n\t",
    ] {
        let cs = cstr_of(s);
        let sp = cs.as_ptr();
        diff_err(
            &format!("G5 {s:?}"),
            "g5",
            Some("6"),
            true,
            move |lib, rets, _st| unsafe {
                rets.push((lib.driver())(sp));
            },
        );
        drop(cs);
    }
}

#[test]
fn bnd_g6_max_tasks_parsing() {
    for v in [
        "0",
        "",
        "abc",
        "7x",
        " 9",
        "  -4",
        "+3",
        "-0",
        "0x10",
        "010",
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "99999999999999999999",
        "9.9",
        "1e3",
        "\n5",
        "5\n",
    ] {
        diff_err(
            &format!("G6 MAX_TASKS={v:?}"),
            "g6",
            Some(v),
            true,
            |lib, rets, state| unsafe {
                rets.push((lib.initialize_logger())());
                let m = (lib.create_task_manager())();
                rets.push(if m.is_null() { 1 } else { 0 });
                state.push(snap(m));
                if !m.is_null() {
                    let d = cstr_of("probe");
                    (lib.add_task())(m, d.as_ptr(), 1);
                    state.push(snap(m));
                    (lib.print_tasks())(m);
                    (lib.destroy_task_manager())(m);
                }
                (lib.finalize_logger())();
            },
        );
    }
}

#[test]
fn bnd_g7_log_null_message() {
    // log_* only *forwards* the pointer to fprintf's %s, so NULL is well defined
    // for glibc ("(null)") and both sides must produce the same bytes.
    diff_err("G7 NULL message", "g7", None, true, |lib, rets, _s| unsafe {
        rets.push((lib.initialize_logger())());
        (lib.log_info())(ptr::null());
        (lib.log_warning())(ptr::null());
        (lib.log_error())(ptr::null());
        (lib.finalize_logger())();
    });
}

/// Documents the unchecked-deref rows: the C has no null checks in `add_task`,
/// `print_tasks`, `destroy_task_manager`, nor on `driver`'s `tasks` argument, and
/// neither does the Rust. Asserted by grepping the translated sources instead of
/// by faulting the test process.
///
/// The C performs exactly twelve null tests; each must have exactly one Rust
/// counterpart and there must be no thirteenth:
///
/// | C site | check | Rust site |
/// |--------|-------|-----------|
/// | `logger.c:35`       | `log_file_env ? :`      | `logger.rs` `log_file_env.is_null()` |
/// | `logger.c:38`       | `!log_file`             | `logger.rs` `LOG_FILE.is_null()` |
/// | `logger.c:48`       | `if (log_file)`         | `logger.rs` `!LOG_FILE.is_null()` (log_info) |
/// | `logger.c:54`       | `if (log_file)`         | `logger.rs` `!LOG_FILE.is_null()` (log_warning) |
/// | `logger.c:60`       | `if (log_file)`         | `logger.rs` `!LOG_FILE.is_null()` (log_error) |
/// | `logger.c:66`       | `if (log_file)`         | `logger.rs` `!LOG_FILE.is_null()` (finalize) |
/// | `task_manager.c:34` | `!manager` (malloc)     | `task_manager.rs` `manager.is_null()` |
/// | `task_manager.c:40` | `max_tasks_env ? :`     | `task_manager.rs` `max_tasks_env.is_null()` |
/// | `task_manager.c:43` | `!manager->tasks`       | `task_manager.rs` `tasks.is_null()` |
/// | `driver.c:39`       | `!manager`              | `driver.rs` `manager.is_null()` |
/// | `driver.c:47`       | `end == NULL` (strchr)  | `driver.rs` `end.is_null()` |
/// | `driver.c:54`       | `!task` (malloc)        | `driver.rs` `task.is_null()` |
#[test]
fn bnd_g7_no_extra_null_guards_in_rust() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut per_file: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "rs") != Some(true) {
            continue;
        }
        let txt = std::fs::read_to_string(&p).unwrap();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        for line in txt.lines() {
            let t = line.trim();
            // Ignore doc comments / normal comments.
            if t.starts_with("//") || t.starts_with("*") || t.starts_with("/*") {
                continue;
            }
            if t.contains("is_null()") {
                per_file.entry(name.clone()).or_default().push(t.to_string());
            }
        }
    }

    // Exactly the twelve counterparts of the C's twelve null tests.
    let expect: &[(&str, usize, &[&str])] = &[
        (
            "logger.rs",
            6,
            &["log_file_env.is_null()", "LOG_FILE.is_null()"],
        ),
        (
            "task_manager.rs",
            3,
            &["manager.is_null()", "max_tasks_env.is_null()", "tasks.is_null()"],
        ),
        (
            "driver.rs",
            3,
            &["manager.is_null()", "end.is_null()", "task.is_null()"],
        ),
    ];

    let total: usize = per_file.values().map(|v| v.len()).sum();
    assert_eq!(
        total, 12,
        "Rust must contain exactly the C's 12 null tests, found {total}:\n{:#?}",
        per_file
    );

    for (file, n, allowed) in expect {
        let got = per_file
            .get(*file)
            .unwrap_or_else(|| panic!("no null checks found in {file}"));
        assert_eq!(
            got.len(),
            *n,
            "{file}: expected {n} null checks, got {}:\n{}",
            got.len(),
            got.join("\n")
        );
        for line in got {
            assert!(
                allowed.iter().any(|a| line.contains(a)),
                "{file}: unexpected null check (no C counterpart): {line}"
            );
        }
    }

    // cbind.rs is pure declarations and must contain none.
    assert!(
        per_file.get("cbind.rs").is_none() && per_file.get("lib.rs").is_none(),
        "cbind.rs/lib.rs should contain no null checks: {per_file:#?}"
    );

    // The three functions the C leaves completely unguarded must stay unguarded.
    let tm = std::fs::read_to_string(dir.join("task_manager.rs")).unwrap();
    for fname in ["add_task", "print_tasks", "destroy_task_manager"] {
        let start = tm
            .find(&format!("fn {fname}("))
            .unwrap_or_else(|| panic!("{fname} not found in task_manager.rs"));
        // Body ends at the next `#[unsafe(no_mangle)]` or EOF.
        let rest = &tm[start..];
        let end = rest[1..]
            .find("#[unsafe(no_mangle)]")
            .map(|i| i + 1)
            .unwrap_or(rest.len());
        let body = &rest[..end];
        assert!(
            !body.contains("is_null()"),
            "{fname} must dereference its pointer argument unchecked, like the C does"
        );
    }
}

#[allow(unused)]
fn _unused(_: &Path, _: *const c_char) {}
