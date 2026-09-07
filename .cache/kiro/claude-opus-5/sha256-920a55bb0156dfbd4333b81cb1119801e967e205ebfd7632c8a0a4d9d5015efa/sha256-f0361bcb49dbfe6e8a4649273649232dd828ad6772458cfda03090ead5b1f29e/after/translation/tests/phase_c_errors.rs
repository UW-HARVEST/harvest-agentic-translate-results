//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input, calls BOTH `.so`s through
//! `dlsym`, and asserts the SAME sentinel / error code (and the same `stderr`
//! diagnostic), not merely "both failed".

mod common;

use common::*;
use std::ffi::{CString, c_char, c_int};
use std::os::unix::fs::PermissionsExt;
use std::ptr;

const EINVAL: c_int = 22;
const ENOENT: c_int = 2;
const EACCES: c_int = 13;
const EFAULT: c_int = 14;
const EISDIR: c_int = 21;
const ENOSPC: c_int = 28;
const EXIT_FAILURE: c_int = 1;

const INT_MIN: c_int = c_int::MIN;
const INT_MAX: c_int = c_int::MAX;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Shape-only comparison: never touches element memory, so it is safe for dims
/// that make the allocations absurd.
fn diff_allocate_shape(label: &str, w: c_int, h: c_int) -> bool {
    let p = pair();
    let probe = |imp: &Impl| unsafe {
        let m = imp.allocate_matrix(w, h);
        let out = if m.is_null() {
            (true, 0, 0, true)
        } else {
            (false, (*m).width, (*m).height, (*m).matrix.is_null())
        };
        imp.free_matrix(m);
        out
    };
    let (c_out, c_err) = capture_stderr(|| probe(&p.c));
    let (r_out, r_err) = capture_stderr(|| probe(&p.rs));
    assert_eq!(
        c_out, r_out,
        "[{label}] allocate_matrix({w},{h}) diverged: C={c_out:?} RS={r_out:?}"
    );
    assert_eq!(
        show(&c_err),
        show(&r_err),
        "[{label}] allocate_matrix({w},{h}) stderr diverged"
    );
    c_out.0
}

/// `initialize_matrix_from_string` shape-only comparison (no element reads).
fn diff_init_shape(label: &str, input: &str, w: c_int, h: c_int) -> bool {
    let p = pair();
    let cstr = cs(input);
    let probe = |imp: &Impl| unsafe {
        let m = imp.initialize_matrix_from_string(cstr.as_ptr(), w, h);
        let out = if m.is_null() {
            (true, 0, 0)
        } else {
            (false, (*m).width, (*m).height)
        };
        imp.free_matrix(m);
        out
    };
    let (c_out, c_err) = capture_stderr(|| probe(&p.c));
    let (r_out, r_err) = capture_stderr(|| probe(&p.rs));
    assert_eq!(
        c_out, r_out,
        "[{label}] init({input:?},{w},{h}) diverged: C={c_out:?} RS={r_out:?}"
    );
    assert_eq!(
        show(&c_err),
        show(&r_err),
        "[{label}] init({input:?},{w},{h}) stderr diverged"
    );
    c_out.0
}

/// `write_to_file` against the SAME path for both implementations (used for the
/// failure rows, where no file is produced, so `stderr` — which embeds the
/// filename — is directly comparable).
fn diff_write_err(
    label: &str,
    filename: Option<&str>,
    content: Option<&str>,
    expect_rc: c_int,
) {
    let p = pair();
    let fname: Option<CString> = filename.map(cs);
    let cont: Option<CString> = content.map(cs);
    let fp: *const c_char = fname.as_ref().map_or(ptr::null(), |c| c.as_ptr());
    let cp: *const c_char = cont.as_ref().map_or(ptr::null(), |c| c.as_ptr());

    let (c_rc, c_err) = capture_stderr(|| unsafe { p.c.write_to_file(fp, cp) });
    let (r_rc, r_err) = capture_stderr(|| unsafe { p.rs.write_to_file(fp, cp) });

    assert_eq!(
        c_rc,
        r_rc,
        "[{label}] write_to_file rc diverged: C={c_rc} ({}) RS={r_rc} ({})",
        errno_name(c_rc),
        errno_name(r_rc)
    );
    assert_eq!(
        c_rc,
        expect_rc,
        "[{label}] expected {} ({expect_rc}) but C returned {} ({c_rc})",
        errno_name(expect_rc),
        errno_name(c_rc)
    );
    assert_eq!(
        show(&c_err),
        show(&r_err),
        "[{label}] write_to_file stderr diverged"
    );
    assert!(!c_err.is_empty(), "[{label}] expected a diagnostic on stderr");
}

// ===========================================================================
// ERRORS.md rows 2 & 3 — allocate_matrix allocation failures
// ===========================================================================

/// Row 2: `malloc(height * sizeof(int*))` fails (height < 0 ⇒ huge size_t).
#[test]
fn err_01_allocate_matrix_negative_height() {
    let _g = lock();
    for h in [-1, -2, -7, INT_MIN, INT_MIN + 1] {
        for w in [0, 1, 4] {
            let was_null = diff_allocate_shape("err01", w, h);
            assert!(was_null, "err01 expected NULL for height={h}, width={w}");
        }
    }
}

/// Row 3: a row `malloc(width * sizeof(int))` fails (width < 0, height > 0).
#[test]
fn err_02_allocate_matrix_negative_width() {
    let _g = lock();
    for w in [-1, -2, -9, INT_MIN, INT_MIN + 1] {
        for h in [1, 3] {
            let was_null = diff_allocate_shape("err02", w, h);
            assert!(was_null, "err02 expected NULL for width={w}, height={h}");
        }
    }
}

// ===========================================================================
// ERRORS.md row 4 — free_matrix(NULL)
// ===========================================================================

#[test]
fn err_03_free_matrix_null() {
    let _g = lock();
    let p = pair();
    let (_, c_err) = capture_stderr(|| unsafe {
        for _ in 0..100 {
            p.c.free_matrix(ptr::null_mut());
        }
    });
    let (_, r_err) = capture_stderr(|| unsafe {
        for _ in 0..100 {
            p.rs.free_matrix(ptr::null_mut());
        }
    });
    assert_eq!(show(&c_err), show(&r_err), "err03 stderr diverged");
    assert!(c_err.is_empty(), "err03 free_matrix(NULL) must be silent");
}

// ===========================================================================
// ERRORS.md rows 6, 7, 8 — tokenisation shortfalls
// ===========================================================================

/// Row 6: fewer rows than `height`.
#[test]
fn err_04_insufficient_rows() {
    let _g = lock();
    diff_init("err04-a", "1 2\n3 4", 2, 3);
    diff_init("err04-b", "1 2\n3 4\n", 2, 5);
    diff_init("err04-c", "1", 1, 2);
    diff_init("err04-d", "\n\n\n", 1, 1);
    diff_init("err04-e", "   ", 0, 2);
    // stderr text must match the C exactly.
    let p = pair();
    let input = cs("1 2\n3 4");
    let (_, c_err) = capture_stderr(|| unsafe {
        let m = p.c.initialize_matrix_from_string(input.as_ptr(), 2, 3);
        p.c.free_matrix(m);
    });
    assert_stderr_is("err04", &c_err, "Insufficient rows in input string.\n");
}

/// Row 7: fewer columns than `width`, and the 1-based row number in the message.
#[test]
fn err_05_insufficient_cols() {
    let _g = lock();
    diff_init("err05-a", "1 2\n3", 2, 2);
    diff_init("err05-b", "1 2 3\n4 5 6", 4, 2);
    diff_init("err05-c", "1", 2, 1);
    diff_init("err05-d", "1 2 3\n4 5 6\n7 8", 3, 3);
    let p = pair();
    // Failure on row 3 ⇒ message must say "row 3".
    let input = cs("1 2 3\n4 5 6\n7 8");
    let (_, c_err) = capture_stderr(|| unsafe {
        let m = p.c.initialize_matrix_from_string(input.as_ptr(), 3, 3);
        p.c.free_matrix(m);
    });
    assert_stderr_is("err05", &c_err, "Insufficient columns in row 3.\n");
    let (_, r_err) = capture_stderr(|| unsafe {
        let m = p.rs.initialize_matrix_from_string(input.as_ptr(), 3, 3);
        p.rs.free_matrix(m);
    });
    assert_stderr_is("err05-rs", &r_err, "Insufficient columns in row 3.\n");
}

/// Row 8: empty input string with height > 0.
#[test]
fn err_06_empty_input() {
    let _g = lock();
    diff_init("err06-empty", "", 1, 1);
    diff_init("err06-empty-3x3", "", 3, 3);
    diff_init("err06-nl-only", "\n", 2, 2);
    diff_init("err06-spaces", "     ", 2, 2);
}

// ===========================================================================
// ERRORS.md rows 9, 10 — unchecked allocate_matrix NULL inside init
// ===========================================================================

/// Row 9: `height < 0` ⇒ `allocate_matrix` NULL, unchecked, loop skipped.
#[test]
fn err_07_init_negative_height() {
    let _g = lock();
    for h in [-1, -3, INT_MIN, INT_MIN + 1] {
        for w in [0, 1, 3] {
            let was_null = diff_init_shape("err07", "1 2 3\n4 5 6\n", w, h);
            assert!(was_null, "err07 expected NULL for w={w} h={h}");
        }
    }
}

/// Row 10: `width < 0 && height > 0` ⇒ NULL `mat` returned without a NULL deref.
#[test]
fn err_08_init_negative_width() {
    let _g = lock();
    for w in [-1, -5, INT_MIN, INT_MIN + 1] {
        for h in [1, 2, 3] {
            let was_null = diff_init_shape("err08", "1 2 3\n4 5 6\n7 8 9\n", w, h);
            assert!(was_null, "err08 expected NULL for w={w} h={h}");
        }
    }
    // Also the sub-case where the row tokens run out first.
    diff_init_shape("err08-short", "1 2", -1, 4);
}

// ===========================================================================
// ERRORS.md row 11 — multiply_matrices dimension mismatch
// ===========================================================================

#[test]
fn err_09_dim_mismatch() {
    let _g = lock();
    // A width 2 vs B height 3
    diff_multiply("err09-a", "1 2\n3 4", 2, 2, "1 2\n3 4\n5 6", 2, 3);
    // A width 3 vs B height 1
    diff_multiply("err09-b", "1 2 3", 3, 1, "9", 1, 1);
    // one-step-off: width 2 vs height 1 and width 2 vs height 3
    diff_multiply("err09-c", "1 2\n3 4", 2, 2, "5 6", 2, 1);
    // zero vs non-zero inner dim
    diff_multiply("err09-d", "x\ny", 0, 2, "1 2\n3 4", 2, 2);

    let p = pair();
    let a = cs("1 2\n3 4");
    let b = cs("1 2\n3 4\n5 6");
    let run = |imp: &Impl| unsafe {
        let ma = imp.initialize_matrix_from_string(a.as_ptr(), 2, 2);
        let mb = imp.initialize_matrix_from_string(b.as_ptr(), 2, 3);
        let r = imp.multiply_matrices(ma, mb);
        let is_null = r.is_null();
        imp.free_matrix(r);
        imp.free_matrix(ma);
        imp.free_matrix(mb);
        is_null
    };
    let (c_null, c_err) = capture_stderr(|| run(&p.c));
    let (r_null, r_err) = capture_stderr(|| run(&p.rs));
    assert!(c_null && r_null, "err09 both must return NULL");
    assert_stderr_is(
        "err09-c-msg",
        &c_err,
        "Matrix dimensions do not allow multiplication.\n",
    );
    assert_eq!(show(&c_err), show(&r_err), "err09 stderr diverged");
}

// ===========================================================================
// ERRORS.md rows 12, 13 — matrix_to_string
// ===========================================================================

/// Row 12 / G1: `matrix_to_string(NULL)`.
#[test]
fn err_10_to_string_null() {
    let _g = lock();
    let p = pair();
    let (c_out, c_err) =
        capture_stderr(|| unsafe { take_c_string(p.c.matrix_to_string(ptr::null_mut())) });
    let (r_out, r_err) =
        capture_stderr(|| unsafe { take_c_string(p.rs.matrix_to_string(ptr::null_mut())) });
    assert!(c_out.is_none(), "err10 C must return NULL");
    assert_eq!(c_out, r_out, "err10 return diverged");
    assert_stderr_is("err10", &c_err, "Error: Matrix is NULL.\n");
    assert_eq!(show(&c_err), show(&r_err), "err10 stderr diverged");
}

/// Row 13: `buffer_size` overflows `int` negative ⇒ `malloc` fails ⇒ NULL.
///
/// The C returns before touching `mat->matrix`, so a hand-built `matrix_t` with
/// a NULL row array is a legitimate input here.
#[test]
fn err_11_to_string_size_overflow() {
    let _g = lock();
    let p = pair();
    // height * (width*10 + width) + height + 1, computed in `int`:
    //   width = 200_000_000, height = 1 -> 2_200_000_000 wraps to -2_094_967_296
    //
    // Only dimension pairs whose wrapped `buffer_size` comes out NEGATIVE may be
    // used: when it wraps back to a large POSITIVE value (e.g. INT_MAX x 1 ->
    // 2_147_483_639, INT_MAX x 3 -> 2_147_483_619) `malloc` succeeds and the C
    // goes on to dereference `mat->matrix`, which faults for any matrix that
    // does not really have that many columns. Both implementations fault
    // identically there; it is simply not observable through a test. The
    // assertion below enforces the precondition instead of trusting the table.
    let buffer_size = |w: c_int, h: c_int| -> c_int {
        h.wrapping_mul(w.wrapping_mul(10).wrapping_add(w))
            .wrapping_add(h)
            .wrapping_add(1)
    };
    for (w, h) in [
        (200_000_000, 1),
        (INT_MAX, 2),
        (1_000_000_000, 3),
        (300_000_000, 1),
        (250_000_000, 1),
        (INT_MAX, 4),
    ] {
        assert!(
            buffer_size(w, h) < 0,
            "err11 precondition: buffer_size({w},{h}) = {} must be negative",
            buffer_size(w, h)
        );
        let mut m = MatrixT {
            matrix: ptr::null_mut(),
            width: w,
            height: h,
        };
        let mp: *mut MatrixT = &raw mut m;
        let (c_out, c_err) = capture_stderr(|| unsafe { take_c_string(p.c.matrix_to_string(mp)) });
        let (r_out, r_err) = capture_stderr(|| unsafe { take_c_string(p.rs.matrix_to_string(mp)) });
        assert_eq!(
            c_out.is_none(),
            r_out.is_none(),
            "err11 ({w}x{h}) NULL-ness diverged: C={} RS={}",
            show_opt(&c_out),
            show_opt(&r_out)
        );
        assert_eq!(c_out, r_out, "err11 ({w}x{h}) return diverged");
        assert_eq!(show(&c_err), show(&r_err), "err11 ({w}x{h}) stderr diverged");
    }
    // Confirm at least the canonical case really does hit the malloc-failure
    // branch in the C (so the row is genuinely exercised, not vacuous).
    let mut m = MatrixT {
        matrix: ptr::null_mut(),
        width: 200_000_000,
        height: 1,
    };
    let mp: *mut MatrixT = &raw mut m;
    let (c_out, c_err) = capture_stderr(|| unsafe { take_c_string(p.c.matrix_to_string(mp)) });
    assert!(c_out.is_none(), "err11 C must return NULL");
    assert!(
        show(&c_err).starts_with("Failed to allocate memory for matrix string"),
        "err11 expected the perror diagnostic, got {:?}",
        show(&c_err)
    );
}

// ===========================================================================
// ERRORS.md rows 14..21 — write_to_file
// ===========================================================================

/// Row 14: `content == NULL` ⇒ `EINVAL`.
#[test]
fn err_12_write_null_content() {
    let _g = lock();
    let path = unique_tmp_path("nullcontent");
    diff_write_err(
        "err12",
        Some(path.to_str().unwrap()),
        None,
        EINVAL,
    );
    assert!(!path.exists(), "err12 must not create the file");
    let p = pair();
    let name = cs(path.to_str().unwrap());
    let (_, c_err) =
        capture_stderr(|| unsafe { p.c.write_to_file(name.as_ptr(), ptr::null()) });
    assert_stderr_is("err12-msg", &c_err, "Error: Content is NULL.\n");
}

/// Row 15: `fopen` ⇒ `ENOENT` (missing directory component).
#[test]
fn err_13_write_fopen_enoent() {
    let _g = lock();
    diff_write_err(
        "err13",
        Some("/tmp/definitely-not-here-cdiff-xyz/inner/out.txt"),
        Some("payload"),
        ENOENT,
    );
}

/// Row 16: `fopen` ⇒ `EISDIR` (target is a directory).
#[test]
fn err_14_write_fopen_eisdir() {
    let _g = lock();
    let dir = unique_tmp_path("isdir");
    std::fs::create_dir_all(&dir).unwrap();
    diff_write_err("err14", Some(dir.to_str().unwrap()), Some("payload"), EISDIR);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Row 17: `fopen("")` ⇒ `ENOENT`.
#[test]
fn err_15_write_fopen_empty_name() {
    let _g = lock();
    diff_write_err("err15", Some(""), Some("payload"), ENOENT);
}

/// Row 18 / null-pointer boundary: `filename == NULL` ⇒ `EFAULT`.
#[test]
fn err_16_write_fopen_null_name() {
    let _g = lock();
    diff_write_err("err16", None, Some("payload"), EFAULT);
}

/// Row 19: `fopen` ⇒ `EACCES` (unwritable directory).
#[test]
fn err_17_write_fopen_eacces() {
    let _g = lock();
    let dir = unique_tmp_path("ro");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let target = dir.join("out.txt");
    diff_write_err("err17", Some(target.to_str().unwrap()), Some("payload"), EACCES);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Row 20: `fprintf(file, "%s", content) < 0` ⇒ `ENOSPC`.
/// Content larger than the stdio buffer forces the flush inside `fprintf`.
#[test]
fn err_18_write_fprintf_enospc_large() {
    let _g = lock();
    let big = "a".repeat(300_000);
    diff_write_err("err18", Some("/dev/full"), Some(&big), ENOSPC);
    // Confirm it is the *writing* branch, not the closing branch.
    let p = pair();
    let name = cs("/dev/full");
    let content = cs(&big);
    let (_, c_err) =
        capture_stderr(|| unsafe { p.c.write_to_file(name.as_ptr(), content.as_ptr()) });
    assert!(
        show(&c_err).starts_with("Error writing to file '/dev/full'"),
        "err18 expected the write-branch diagnostic, got {:?}",
        show(&c_err)
    );
}

/// Row 21: `fclose(file) != 0` ⇒ `ENOSPC`. Small content stays buffered, so the
/// failure only surfaces at close time.
#[test]
fn err_19_write_fclose_enospc_small() {
    let _g = lock();
    diff_write_err("err19", Some("/dev/full"), Some("hi"), ENOSPC);
    let p = pair();
    let name = cs("/dev/full");
    let content = cs("hi");
    let (_, c_err) =
        capture_stderr(|| unsafe { p.c.write_to_file(name.as_ptr(), content.as_ptr()) });
    assert!(
        show(&c_err).starts_with("Error closing file '/dev/full'"),
        "err19 expected the close-branch diagnostic, got {:?}",
        show(&c_err)
    );
}

// ===========================================================================
// ERRORS.md rows 22..24, 26 — driver
// ===========================================================================

/// Row 22: matrix A fails to initialise ⇒ `EXIT_FAILURE`.
#[test]
fn err_20_driver_bad_a() {
    let _g = lock();
    diff_driver("err20-rows", 2, 3, "1 2\n3 4", 2, 2, "1 2\n3 4");
    diff_driver("err20-cols", 3, 2, "1 2\n3 4", 2, 2, "1 2\n3 4");
    diff_driver("err20-empty", 2, 2, "", 2, 2, "1 2\n3 4");
    diff_driver("err20-negw", -1, 2, "1 2\n3 4", 2, 2, "1 2\n3 4");
    diff_driver("err20-negh", 2, -1, "1 2\n3 4", 2, 2, "1 2\n3 4");
    // and it really is EXIT_FAILURE
    let p = pair();
    let a = cs("1 2\n3 4");
    let b = cs("1 2\n3 4");
    let (rc, _) = capture_stderr(|| unsafe {
        p.c.driver(2, 3, a.as_ptr(), 2, 2, b.as_ptr())
    });
    assert_eq!(rc, EXIT_FAILURE, "err20 expected EXIT_FAILURE");
}

/// Row 23: matrix B fails to initialise ⇒ `EXIT_FAILURE` after freeing A.
#[test]
fn err_21_driver_bad_b() {
    let _g = lock();
    diff_driver("err21-rows", 2, 2, "1 2\n3 4", 2, 3, "1 2\n3 4");
    diff_driver("err21-cols", 2, 2, "1 2\n3 4", 3, 2, "1 2\n3 4");
    diff_driver("err21-empty", 2, 2, "1 2\n3 4", 2, 2, "");
    diff_driver("err21-negw", 2, 2, "1 2\n3 4", -1, 2, "1 2\n3 4");
    diff_driver("err21-negh", 2, 2, "1 2\n3 4", 2, -1, "1 2\n3 4");
}

/// Row 24: `width_a != height_b` ⇒ `multiply_matrices` NULL ⇒ `EXIT_FAILURE`.
#[test]
fn err_22_driver_dim_mismatch() {
    let _g = lock();
    diff_driver("err22-a", 2, 2, "1 2\n3 4", 2, 3, "1 2\n3 4\n5 6");
    diff_driver("err22-b", 3, 1, "1 2 3", 1, 1, "9");
    diff_driver("err22-c", 1, 2, "1\n2", 2, 2, "1 2\n3 4");
    diff_driver("err22-zero", 0, 2, "x\ny", 2, 2, "1 2\n3 4");
}

/// Row 26: `write_to_file("matrix.txt", …)` fails because the cwd is read-only.
#[test]
fn err_23_driver_write_fails() {
    let _g = lock();
    let p = pair();
    let dir = unique_tmp_path("drv-ro");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let orig = std::env::current_dir().unwrap();
    let a = cs("1 2\n3 4\n");
    let b = cs("5 6\n7 8\n");

    let run = |imp: &Impl| {
        std::env::set_current_dir(&dir).unwrap();
        let out = capture_stderr(|| unsafe { imp.driver(2, 2, a.as_ptr(), 2, 2, b.as_ptr()) });
        std::env::set_current_dir(&orig).unwrap();
        out
    };
    let (c_rc, c_err) = run(&p.c);
    let (r_rc, r_err) = run(&p.rs);

    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(c_rc, r_rc, "err23 driver rc diverged (C={c_rc} RS={r_rc})");
    assert_eq!(c_rc, EXIT_FAILURE, "err23 expected EXIT_FAILURE from the C");
    assert_eq!(show(&c_err), show(&r_err), "err23 stderr diverged");
    assert!(
        show(&c_err).contains("Error opening file 'matrix.txt'"),
        "err23 expected the fopen diagnostic, got {:?}",
        show(&c_err)
    );
}

// ===========================================================================
// Generic FFI-boundary cases (ERRORS.md G-rows)
// ===========================================================================

/// G3: both pointers NULL — the content check runs first, so `EINVAL` wins.
#[test]
fn err_24_write_both_null() {
    let _g = lock();
    diff_write_err("err24", None, None, EINVAL);
    let p = pair();
    let (_, c_err) = capture_stderr(|| unsafe { p.c.write_to_file(ptr::null(), ptr::null()) });
    assert_stderr_is("err24-msg", &c_err, "Error: Content is NULL.\n");
}

/// G5/G6/G7: the full `int` range in the `width`/`height` parameters, including
/// values that have no meaningful interpretation. (There is no `enum` in this
/// API — see ERRORS.md G7 — so arbitrary `int` is the equivalent input class.)
///
/// `height` is deliberately never a large positive value: the C would then try
/// to perform billions of row allocations, whose success depends on the host's
/// overcommit state rather than on the translation.
#[test]
fn err_25_extreme_dims() {
    let _g = lock();
    let cases: &[(c_int, c_int)] = &[
        (INT_MAX, 0),
        (INT_MIN, 0),
        (INT_MIN + 1, 0),
        (0, INT_MIN),
        (0, -1),
        (-1, 0),
        (-1, -1),
        (INT_MIN, INT_MIN),
        (INT_MAX, -1),
        (INT_MIN, INT_MAX),
        (-1, 1),
        (1, -1),
        (-1, INT_MIN),
        (INT_MIN, 1),
        (1, INT_MIN),
        (INT_MIN + 1, INT_MIN + 1),
    ];
    for &(w, h) in cases {
        diff_allocate_shape("err25-alloc", w, h);
        diff_init_shape("err25-init", "1 2 3\n4 5 6\n", w, h);
    }
}

/// Randomised sweep over the out-of-range / boundary dimension space.
#[test]
fn cfg_fuzz_dims() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xF1);
    let interesting: &[c_int] = &[
        INT_MIN,
        INT_MIN + 1,
        -1_000_000,
        -1000,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        7,
        16,
        INT_MAX,
    ];
    for i in 0..400 {
        let mut w = *rng.pick(interesting);
        let mut h = *rng.pick(interesting);
        // Keep `height` out of the "huge positive" region (see err_25 note).
        if h > 16 {
            h = *rng.pick(&[0, -1, 1, 3]);
        }
        if w > 16 && h > 0 {
            w = *rng.pick(&[0, -1, 1, 3, 16]);
        }
        diff_allocate_shape(&format!("fuzzdims#{i}-alloc"), w, h);
        diff_init_shape(&format!("fuzzdims#{i}-init"), "1 2 3\n4 5 6\n7 8 9\n", w, h);
    }
}

/// Extra boundary: `multiply_matrices` where one operand has a negative
/// dimension recorded in a hand-built struct but conformable fields.
#[test]
fn err_26_multiply_negative_dims_handbuilt() {
    let _g = lock();
    let p = pair();
    // width_a == height_b == 0 but heights/widths negative: every loop is
    // skipped, so `allocate_matrix(mat_b->width, mat_a->height)` decides.
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        // (w_a, h_a, w_b, h_b) with w_a == h_b so the guard passes
        (0, -1, 0, 0),
        (0, 0, -1, 0),
        (0, -1, -1, 0),
        (0, 0, 0, 0),
    ];
    for &(wa, ha, wb, hb) in cases {
        let mut a = MatrixT {
            matrix: ptr::null_mut(),
            width: wa,
            height: ha,
        };
        let mut b = MatrixT {
            matrix: ptr::null_mut(),
            width: wb,
            height: hb,
        };
        let ap: *mut MatrixT = &raw mut a;
        let bp: *mut MatrixT = &raw mut b;
        let run = |imp: &Impl| unsafe {
            let r = imp.multiply_matrices(ap, bp);
            let out = if r.is_null() {
                (true, 0, 0)
            } else {
                (false, (*r).width, (*r).height)
            };
            imp.free_matrix(r);
            out
        };
        let (c_out, c_err) = capture_stderr(|| run(&p.c));
        let (r_out, r_err) = capture_stderr(|| run(&p.rs));
        assert_eq!(
            c_out, r_out,
            "err26 multiply({wa}x{ha} , {wb}x{hb}) diverged: C={c_out:?} RS={r_out:?}"
        );
        assert_eq!(show(&c_err), show(&r_err), "err26 stderr diverged");
    }
}

/// Extra boundary: `matrix_to_string` on a hand-built struct whose dimensions
/// are zero/negative (loops skipped, no element access).
#[test]
fn err_27_to_string_degenerate_dims() {
    let _g = lock();
    let p = pair();
    for (w, h) in [
        (0, 0),
        (5, 0),
        (0, -1),
        (-1, 0),
        (-1, -1),
        (INT_MIN, 0),
        (INT_MAX, 0),
        (0, INT_MIN),
        (-5, -5),
    ] {
        let mut m = MatrixT {
            matrix: ptr::null_mut(),
            width: w,
            height: h,
        };
        let mp: *mut MatrixT = &raw mut m;
        let (c_out, c_err) = capture_stderr(|| unsafe { take_c_string(p.c.matrix_to_string(mp)) });
        let (r_out, r_err) = capture_stderr(|| unsafe { take_c_string(p.rs.matrix_to_string(mp)) });
        assert_eq!(
            c_out,
            r_out,
            "err27 to_string({w}x{h}) diverged: C={} RS={}",
            show_opt(&c_out),
            show_opt(&r_out)
        );
        assert_eq!(show(&c_err), show(&r_err), "err27 stderr diverged for ({w}x{h})");
    }
}

/// Extra boundary: `driver` with every dimension at a degenerate/negative value.
#[test]
fn err_28_driver_degenerate_dims() {
    let _g = lock();
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (-1, -1, -1, -1),
        (0, -1, 0, 0),
        (-1, 2, 2, -1),
        (INT_MIN, 1, 1, INT_MIN),
        (0, 0, 0, 0),
        (1, 0, 0, 1),
        (0, 1, 1, 0),
    ];
    for &(wa, ha, wb, hb) in cases {
        diff_driver(
            &format!("err28-{wa}x{ha}-{wb}x{hb}"),
            wa,
            ha,
            "1 2\n3 4\n",
            wb,
            hb,
            "5 6\n7 8\n",
        );
    }
}
