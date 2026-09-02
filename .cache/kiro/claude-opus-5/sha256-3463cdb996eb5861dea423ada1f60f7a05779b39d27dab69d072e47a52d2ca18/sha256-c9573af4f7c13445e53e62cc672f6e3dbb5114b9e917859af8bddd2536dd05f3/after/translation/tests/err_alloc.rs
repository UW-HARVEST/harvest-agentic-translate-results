//! ERRORS.md rows E1–E3 (+ note (a)): the `shared.h` helpers that
//! `exit(EXIT_FAILURE)`. Run out-of-process via `tests/exit_helper.rs`.

mod common;

use common::{assert_helper_same, run_case};

/// ERRORS.md E1 — `os_calloc` with a size that makes `calloc` return NULL.
#[test]
fn e1_os_calloc_oom() {
    let c = assert_helper_same("calloc_oom");
    assert_eq!(c.code, Some(1), "C must exit(EXIT_FAILURE)");
    assert_eq!(
        c.stderr,
        b"Memory allocation failed in os_calloc".to_vec(),
        "exact C message (note: no trailing newline)"
    );
    // And the Rust really did produce those same bytes.
    assert_eq!(run_case("rust", "calloc_oom").stderr, c.stderr);
}

/// ERRORS.md E2 — `os_realloc` returning NULL, from a NULL pointer and from a
/// live allocation.
#[test]
fn e2_os_realloc_oom() {
    let c = assert_helper_same("realloc_oom");
    assert_eq!(c.code, Some(1));
    assert_eq!(c.stderr, b"Memory allocation failed in os_realloc".to_vec());
    let c2 = assert_helper_same("realloc_oom_live");
    assert_eq!(c2.code, Some(1));
    assert_eq!(c2.stderr, b"Memory allocation failed in os_realloc".to_vec());
}

/// ERRORS.md E3 — `os_strdup(NULL)`.
#[test]
fn e3_os_strdup_null() {
    let c = assert_helper_same("strdup_null");
    assert_eq!(c.code, Some(1));
    assert_eq!(c.stderr, b"NULL string passed to os_strdup".to_vec());
}

/// Control: the happy paths must not exit, in either implementation.
#[test]
fn e0_helpers_happy_path_does_not_exit() {
    let c = assert_helper_same("ok");
    assert_eq!(c.code, Some(0));
    assert_eq!(c.stdout, b"OK\n".to_vec());
}
