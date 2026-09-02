//! Phase C — the error rows where the C performs *no* check at all
//! (`ERRORS.md` rows 18-22, 26-28, 30-31).
//!
//! `c_src` has no NULL guards on `add_task`, `print_tasks` or
//! `destroy_task_manager`, and `finalize_logger` closes `log_file` without
//! resetting it.  For those inputs "the same error code" does not exist: the C's
//! observable result is how the *process* ends.  So each scenario is run in a
//! forked child and the termination status (signal number / exit code) of the C
//! child is compared with that of the Rust child.  A Rust build that "helpfully"
//! returned instead of faulting would be a divergence and is caught here.

mod common;

use common::{assert_same_term, cstring, Config, Term, TaskManager};
use std::ffi::c_char;

const SIGSEGV: i32 = 11;

fn cfg() -> Config {
    Config::new()
}

// ---------------------------------------------------------------------------
// Rows 18-22: unchecked NULL dereferences
// ---------------------------------------------------------------------------

#[test]
fn err18_add_task_null_manager() {
    let desc = cstring(b"payload");
    let t = assert_same_term("err18", &cfg(), |api| unsafe {
        (api.initialize_logger)();
        (api.add_task)(std::ptr::null_mut(), desc.as_ptr() as *const c_char, 1);
    });
    assert_eq!(t, Term::Signaled(SIGSEGV), "row 18 should fault");
}

#[test]
fn err19_add_task_null_description() {
    // Capacity is available, so the C reaches `strncpy(dst, NULL, 255)`.
    let t = assert_same_term("err19", &cfg(), |api| unsafe {
        (api.initialize_logger)();
        let m = (api.create_task_manager)();
        assert!(!m.is_null());
        (api.add_task)(m, std::ptr::null(), 1);
    });
    assert_eq!(t, Term::Signaled(SIGSEGV), "row 19 should fault");
}

#[test]
fn err20_print_tasks_null_manager() {
    // `printf("Tasks:\n")` happens before the dereference, so the header is
    // emitted and only then does the process fault.
    let t = assert_same_term("err20", &cfg(), |api| unsafe {
        (api.initialize_logger)();
        (api.print_tasks)(std::ptr::null());
    });
    assert_eq!(t, Term::Signaled(SIGSEGV), "row 20 should fault");
}

#[test]
fn err21_print_tasks_null_array() {
    for task_count in [1i32, 3, 1000] {
        let mut tm = TaskManager {
            tasks: std::ptr::null_mut(),
            max_tasks: 10,
            task_count,
        };
        let p: *const TaskManager = &tm;
        let t = assert_same_term(&format!("err21-{task_count}"), &cfg(), |api| unsafe {
            (api.initialize_logger)();
            (api.print_tasks)(p);
        });
        assert_eq!(t, Term::Signaled(SIGSEGV), "row 21 should fault");
        let _ = &mut tm;
    }
}

#[test]
fn err22_destroy_null_manager() {
    let t = assert_same_term("err22", &cfg(), |api| unsafe {
        (api.initialize_logger)();
        (api.destroy_task_manager)(std::ptr::null_mut());
    });
    assert_eq!(t, Term::Signaled(SIGSEGV), "row 22 should fault");
}

// ---------------------------------------------------------------------------
// Row 26: driver(NULL)
// ---------------------------------------------------------------------------

#[test]
fn err26_driver_null_input() {
    // `while (*start != '\0')` dereferences the NULL argument, after the logger
    // has been opened and the manager allocated.
    let t = assert_same_term("err26", &cfg(), |api| unsafe {
        (api.driver)(std::ptr::null());
    });
    assert_eq!(t, Term::Signaled(SIGSEGV), "row 26 should fault");
}

// ---------------------------------------------------------------------------
// Rows 27-28: log_file is fclose'd but never reset to NULL
// ---------------------------------------------------------------------------

#[test]
fn err27_log_after_finalize() {
    let msg = cstring(b"after finalize");
    for (i, which) in [0usize, 1, 2].into_iter().enumerate() {
        // Recorded twice to confirm the behaviour is deterministic rather than
        // an accident of heap layout.
        let mut seen = Vec::new();
        for round in 0..2 {
            seen.push(assert_same_term(
                &format!("err27-{i}-{round}"),
                &cfg(),
                |api| unsafe {
                    (api.initialize_logger)();
                    (api.finalize_logger)();
                    let p = msg.as_ptr() as *const c_char;
                    match which {
                        0 => (api.log_info)(p),
                        1 => (api.log_warning)(p),
                        _ => (api.log_error)(p),
                    }
                },
            ));
        }
        assert_eq!(seen[0], seen[1], "row 27 ({which}) is not deterministic");
        eprintln!("row 27 (level {which}): both builds terminated {:?}", seen[0]);
    }
}

#[test]
fn err28_double_finalize() {
    let mut seen = Vec::new();
    for round in 0..2 {
        seen.push(assert_same_term(
            &format!("err28-{round}"),
            &cfg(),
            |api| unsafe {
                (api.initialize_logger)();
                (api.finalize_logger)();
                (api.finalize_logger)();
            },
        ));
    }
    assert_eq!(seen[0], seen[1], "row 28 is not deterministic");
    eprintln!("row 28: both builds terminated {:?}", seen[0]);
}

// ---------------------------------------------------------------------------
// Rows 30-31: manager use-after-free / double free
// ---------------------------------------------------------------------------

#[test]
fn err30_double_destroy() {
    let mut seen = Vec::new();
    for round in 0..2 {
        seen.push(assert_same_term(
            &format!("err30-{round}"),
            &cfg(),
            |api| unsafe {
                (api.initialize_logger)();
                let m = (api.create_task_manager)();
                assert!(!m.is_null());
                (api.destroy_task_manager)(m);
                (api.destroy_task_manager)(m);
            },
        ));
    }
    assert_eq!(seen[0], seen[1], "row 30 is not deterministic");
    eprintln!("row 30: both builds terminated {:?}", seen[0]);
    assert_ne!(
        seen[0],
        Term::Exited(0),
        "a double free should not complete silently"
    );
}

#[test]
fn err31_add_after_destroy() {
    let desc = cstring(b"use after free");
    let mut seen = Vec::new();
    for round in 0..2 {
        seen.push(assert_same_term(
            &format!("err31-{round}"),
            &cfg(),
            |api| unsafe {
                (api.initialize_logger)();
                let m = (api.create_task_manager)();
                assert!(!m.is_null());
                (api.destroy_task_manager)(m);
                (api.add_task)(m, desc.as_ptr() as *const c_char, 3);
            },
        ));
    }
    assert_eq!(seen[0], seen[1], "row 31 is not deterministic");
    eprintln!("row 31: both builds terminated {:?}", seen[0]);
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep: every function with a NULL / extreme argument.
// ---------------------------------------------------------------------------

#[test]
fn err_generic_null_and_extreme_sweep() {
    let desc = cstring(b"x");
    // A hand-built manager whose fields are extreme but self-consistent enough
    // that the C's behaviour is well defined.
    let mut tm = TaskManager {
        tasks: std::ptr::null_mut(),
        max_tasks: i32::MAX,
        task_count: 0,
    };
    let p: *mut TaskManager = &mut tm;

    // task_count(0) < max_tasks(INT_MAX) -> the C writes through the NULL
    // tasks pointer.
    let t = assert_same_term("errX-add-null-array", &cfg(), |api| unsafe {
        (api.initialize_logger)();
        (api.add_task)(p, desc.as_ptr() as *const c_char, i32::MAX);
    });
    assert_eq!(t, Term::Signaled(SIGSEGV));

    // Both NULL at once.
    let t = assert_same_term("errX-add-both-null", &cfg(), |api| unsafe {
        (api.initialize_logger)();
        (api.add_task)(std::ptr::null_mut(), std::ptr::null(), i32::MIN);
    });
    assert_eq!(t, Term::Signaled(SIGSEGV));

    // A non-NULL but wildly misaligned/unmapped pointer.
    let t = assert_same_term("errX-destroy-bogus", &cfg(), |api| unsafe {
        (api.initialize_logger)();
        (api.destroy_task_manager)(1usize as *mut TaskManager);
    });
    assert!(
        matches!(t, Term::Signaled(_)),
        "an unmapped manager pointer should fault, got {t:?}"
    );

    let _ = &mut tm;
}
