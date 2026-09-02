//! Phase C — error-path differential tests for `driver`
//! (rows 15, 16, 17 and 19 of `ERRORS.md`).
//!
//! `driver` writes to `OUT_FILE = "matrix.txt"` relative to the process cwd, so
//! this binary chdirs into a private temp directory and serialises access.

mod common;

use common::*;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

static CWD_LOCK: Mutex<()> = Mutex::new(());
static DIR: OnceLock<PathBuf> = OnceLock::new();
const OUT: &str = "matrix.txt";

fn enter() -> MutexGuard<'static, ()> {
    let g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!("difftest-driver-c-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    });
    std::env::set_current_dir(d).unwrap();
    g
}

fn check_driver_err(
    b: &Both,
    wa: c_int,
    ha: c_int,
    a: &str,
    wb: c_int,
    hb: c_int,
    bb: &str,
    expect_rc: c_int,
    expect_err: &str,
) {
    let sa = cs(a);
    let sb = cs(bb);
    let run = |api: &Api| {
        let _ = std::fs::remove_file(OUT);
        let (rc, err) =
            capture_stderr(|| unsafe { (api.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr()) });
        let existed = std::fs::read(OUT).ok();
        let _ = std::fs::remove_file(OUT);
        (rc, existed, err)
    };
    let (rc_c, out_c, ec) = run(&b.c);
    let (rc_r, out_r, er) = run(&b.rs);
    let ctx = format!("driver({wa},{ha},{a:?},{wb},{hb},{bb:?})");
    assert_eq!(rc_c, rc_r, "{ctx} return code mismatch");
    assert_eq!(out_c, out_r, "{ctx} matrix.txt mismatch");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er),
        "{ctx} stderr mismatch"
    );
    assert_eq!(rc_c, expect_rc, "{ctx} expected rc {expect_rc}");
    assert_eq!(String::from_utf8_lossy(&ec), expect_err, "{ctx} diagnostic");
}

/// ERRORS.md row 15 — matrix A fails to parse -> EXIT_FAILURE, nothing written.
#[test]
fn err_driver_bad_a() {
    let _g = enter();
    let b = load_both();
    check_driver_err(
        &b,
        2,
        2,
        "1 2\n",
        2,
        2,
        "5 6\n7 8\n",
        1,
        "Insufficient rows in input string.\n",
    );
    check_driver_err(
        &b,
        3,
        1,
        "1 2\n",
        1,
        3,
        "1\n2\n3\n",
        1,
        "Insufficient columns in row 1.\n",
    );
    check_driver_err(&b, 1, 1, "", 1, 1, "1\n", 1, "Insufficient rows in input string.\n");
}

/// ERRORS.md row 16 — A parses, B fails to parse -> `free_matrix(mat_a)` then
/// EXIT_FAILURE.
#[test]
fn err_driver_bad_b() {
    let _g = enter();
    let b = load_both();
    check_driver_err(
        &b,
        2,
        2,
        "1 2\n3 4\n",
        2,
        2,
        "5 6\n",
        1,
        "Insufficient rows in input string.\n",
    );
    check_driver_err(
        &b,
        2,
        2,
        "1 2\n3 4\n",
        3,
        2,
        "5 6\n7 8\n",
        1,
        "Insufficient columns in row 1.\n",
    );
    check_driver_err(&b, 1, 1, "9\n", 1, 1, "", 1, "Insufficient rows in input string.\n");
}

/// ERRORS.md row 17 — both parse but `width_a != height_b`.
#[test]
fn err_driver_dim_mismatch() {
    let _g = enter();
    let b = load_both();
    const MSG: &str = "Matrix dimensions do not allow multiplication.\n";
    check_driver_err(&b, 2, 2, "1 2\n3 4\n", 2, 3, "1 2\n3 4\n5 6\n", 1, MSG);
    check_driver_err(&b, 3, 1, "1 2 3\n", 1, 2, "4\n5\n", 1, MSG);
    check_driver_err(&b, 1, 1, "1\n", 1, 2, "4\n5\n", 1, MSG);
    // width_a = 0 vs height_b = 2.
    check_driver_err(&b, 0, 1, "x", 1, 2, "1\n2\n", 1, MSG);
}

/// ERRORS.md row 19 — the pipeline succeeds but `write_to_file` fails because
/// `matrix.txt` is a directory -> EISDIR reported, driver returns EXIT_FAILURE.
#[test]
fn err_driver_write_fails() {
    let _g = enter();
    let b = load_both();
    let sa = cs("1 2\n3 4\n");
    let sb = cs("5 6\n7 8\n");

    let run = |api: &Api| {
        let _ = std::fs::remove_file(OUT);
        let _ = std::fs::remove_dir_all(OUT);
        std::fs::create_dir(OUT).unwrap();
        let (rc, err) =
            capture_stderr(|| unsafe { (api.driver)(2, 2, sa.as_ptr(), 2, 2, sb.as_ptr()) });
        let _ = std::fs::remove_dir_all(OUT);
        (rc, err)
    };
    let (rc_c, ec) = run(&b.c);
    let (rc_r, er) = run(&b.rs);
    assert_eq!(rc_c, rc_r, "driver write-failure return code mismatch");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er),
        "driver write-failure stderr mismatch"
    );
    assert_eq!(rc_c, 1, "driver must return EXIT_FAILURE");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        "Error opening file 'matrix.txt': Is a directory\n"
    );
}

/// The success sentinel, for contrast: EXIT_SUCCESS is 0 and nothing is printed.
#[test]
fn driver_success_sentinel() {
    let _g = enter();
    let b = load_both();
    check_driver_err(&b, 2, 2, "1 2\n3 4\n", 2, 2, "5 6\n7 8\n", 0, "");
}
