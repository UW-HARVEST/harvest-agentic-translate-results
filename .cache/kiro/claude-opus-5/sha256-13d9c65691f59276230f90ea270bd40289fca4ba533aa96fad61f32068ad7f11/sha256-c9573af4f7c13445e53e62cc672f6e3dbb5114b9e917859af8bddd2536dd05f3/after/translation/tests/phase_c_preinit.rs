//! Phase C — `ERRORS.md` rows 7-10: the `if (log_file)` guards.
//!
//! These four rows can only be observed while `log_file` is still NULL, i.e.
//! before *any* `initialize_logger` call in the process.  Because `log_file` is
//! a `static` that the C never resets, this file deliberately contains a single
//! `#[test]` so the pristine state is guaranteed.

mod common;

use common::{assert_same, cstring, Config, LogTarget};
use std::ffi::c_char;

#[test]
fn err07_preinit_guards() {
    let msg = cstring(b"pre-init message");
    let p = msg.as_ptr() as *const c_char;

    // Row 7/8/9: log_info / log_warning / log_error with log_file == NULL.
    // The guard fails, so nothing is written and no file is even created.
    for (row, which) in [(7, 0usize), (8, 1), (9, 2)] {
        let out = assert_same(
            &format!("err{row:02}-preinit"),
            &Config::new().log(LogTarget::Relative("guard.log")),
            |api| unsafe {
                match which {
                    0 => (api.log_info)(p),
                    1 => (api.log_warning)(p),
                    _ => (api.log_error)(p),
                }
                0
            },
        );
        assert!(
            out.log.is_empty() && out.stdout.is_empty() && out.stderr.is_empty(),
            "row {row}: expected a complete no-op, got {out:?}"
        );
    }

    // Row 10: finalize_logger with log_file == NULL -> no "Logger finalized."
    // line and no fclose.
    let out = assert_same(
        "err10-preinit-finalize",
        &Config::new().log(LogTarget::Relative("guard.log")),
        |api| unsafe {
            (api.finalize_logger)();
            0
        },
    );
    assert!(
        out.log.is_empty() && out.stdout.is_empty() && out.stderr.is_empty(),
        "row 10: expected a complete no-op, got {out:?}"
    );

    // Positive control: the same calls after initialize_logger DO write, which
    // proves the assertions above were not vacuous (wrong path, missing file,
    // ...).  Also covers ERRORS.md row 29 partially and CONFIGS rows 6-8.
    let out = assert_same(
        "err07-positive-control",
        &Config::new().log(LogTarget::Relative("guard.log")),
        |api| unsafe {
            let r = (api.initialize_logger)();
            (api.log_info)(p);
            (api.log_warning)(p);
            (api.log_error)(p);
            (api.finalize_logger)();
            r as i64
        },
    );
    assert_eq!(out.ret, 0);
    let text = String::from_utf8_lossy(&out.log);
    assert!(text.contains("[INFO] Logger initialized."), "{text}");
    assert!(text.contains("[INFO] pre-init message"), "{text}");
    assert!(text.contains("[WARNING] pre-init message"), "{text}");
    assert!(text.contains("[ERROR] pre-init message"), "{text}");
    assert!(text.contains("[INFO] Logger finalized."), "{text}");
}
