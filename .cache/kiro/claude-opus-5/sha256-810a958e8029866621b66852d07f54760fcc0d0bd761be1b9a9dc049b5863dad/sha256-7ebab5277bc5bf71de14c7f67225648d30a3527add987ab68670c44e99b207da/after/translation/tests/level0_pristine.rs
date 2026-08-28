//! Level 0: behaviour that is only observable while `log_file` is still the
//! initial NULL.  These live in their own test binary (== own process) because
//! `finalize_logger()` in C leaves `log_file` dangling rather than NULL, so no
//! test in this binary may ever call `initialize_logger()`.

mod harness;

use harness::{compare, cstr, default_log_path, Api};

/// `log_*` before any `initialize_logger()`: nothing is emitted, no file made.
#[test]
fn log_before_initialize_is_a_noop() {
    compare("log before init", &[], |api: &Api, t| unsafe {
        let m = cstr(b"orphan message");
        (api.log_info)(m.as_ptr());
        (api.log_warning)(m.as_ptr());
        (api.log_error)(m.as_ptr());
        let n = cstr(b"");
        (api.log_info)(n.as_ptr());
        t.push(format!("log file exists = {}", default_log_path().exists()));
    });
}

/// `finalize_logger()` with `log_file == NULL` must not write or create a file.
#[test]
fn finalize_before_initialize_is_a_noop() {
    compare("finalize before init", &[], |api: &Api, t| unsafe {
        (api.finalize_logger)();
        (api.finalize_logger)();
        t.push(format!("log file exists = {}", default_log_path().exists()));
    });
}

/// `create_task_manager()` logs through the same NULL `log_file`; the log calls
/// must stay silent while the manager itself is still built correctly.
#[test]
fn task_manager_without_logger() {
    compare("task manager, logger never opened", &[], |api: &Api, t| unsafe {
        let m = (api.create_task_manager)();
        harness::record_manager(t, m);
        assert!(!m.is_null());
        let d = cstr(b"silent");
        (api.add_task)(m, d.as_ptr(), 7);
        harness::record_manager(t, m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        t.push(format!("log file exists = {}", default_log_path().exists()));
    });
}
