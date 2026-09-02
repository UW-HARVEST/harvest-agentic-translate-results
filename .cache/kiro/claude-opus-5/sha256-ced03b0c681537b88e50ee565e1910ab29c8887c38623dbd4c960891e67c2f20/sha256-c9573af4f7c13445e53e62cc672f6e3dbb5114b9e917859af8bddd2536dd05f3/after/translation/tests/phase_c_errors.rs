//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (matrix + write rows; the `driver` rows live
//! in `phase_c_driver.rs`, which needs cwd control). Each test builds the exact
//! invalid input the C checks for and asserts BOTH `.so`s return the same
//! sentinel / error code AND emit the same diagnostic on stderr.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("difftest-err-{}-{}", std::process::id(), tag));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ===================== rows 2 & 3: allocate_matrix OOM =====================

/// Returns (was_null, stderr_bytes). Non-NULL results are freed again.
fn alloc_probe(api: &Api, w: c_int, h: c_int) -> (bool, Vec<u8>) {
    let (p, err) = capture_stderr(|| unsafe { (api.allocate_matrix)(w, h) });
    if !p.is_null() {
        unsafe { (api.free_matrix)(p) };
    }
    (p.is_null(), err)
}

fn check_alloc_err(b: &Both, w: c_int, h: c_int, expect_null: bool) {
    let (nc, ec) = alloc_probe(&b.c, w, h);
    let (nr, er) = alloc_probe(&b.rs, w, h);
    assert_eq!(nc, nr, "allocate_matrix({w},{h}) NULL-ness mismatch");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er),
        "allocate_matrix({w},{h}) stderr mismatch"
    );
    assert_eq!(
        nc, expect_null,
        "allocate_matrix({w},{h}) expected NULL={expect_null}, got NULL={nc}"
    );
}

/// ERRORS.md row 2 — `malloc(height * sizeof(int*))` fails.
#[test]
fn err_allocate_matrix_negative_height() {
    let b = load_both();
    for h in [-1, -2, -1000, i32::MIN] {
        check_alloc_err(&b, 4, h, true);
        check_alloc_err(&b, 0, h, true);
    }
}

/// ERRORS.md row 3 — `malloc(width * sizeof(int))` fails for some row.
#[test]
fn err_allocate_matrix_negative_width() {
    let b = load_both();
    for w in [-1, -2, -1000, i32::MIN] {
        for h in [1, 2, 7] {
            check_alloc_err(&b, w, h, true);
        }
        // height == 0: the row loop never runs, so a negative width is NOT an
        // error and both return a live struct.
        check_alloc_err(&b, w, 0, false);
    }
}

// ============================ row 4: free_matrix(NULL) =====================

/// ERRORS.md row 4 — `free_matrix(NULL)` is a silent no-op.
#[test]
fn err_free_matrix_null() {
    let b = load_both();
    let (_, ec) = capture_stderr(|| unsafe { (b.c.free_matrix)(std::ptr::null_mut()) });
    let (_, er) = capture_stderr(|| unsafe { (b.rs.free_matrix)(std::ptr::null_mut()) });
    assert_eq!(ec, er);
    assert!(ec.is_empty(), "free_matrix(NULL) must not print anything");
}

// ============== rows 6 & 7: parser exhaustion (rows / columns) =============

fn check_init_err(b: &Both, input: &str, w: c_int, h: c_int, expect_msg: &str) {
    let s = cs(input);
    let run = |api: &Api| {
        let (p, err) =
            capture_stderr(|| unsafe { (api.initialize_matrix_from_string)(s.as_ptr(), w, h) });
        if !p.is_null() {
            unsafe { (api.free_matrix)(p) };
        }
        (p.is_null(), err)
    };
    let (nc, ec) = run(&b.c);
    let (nr, er) = run(&b.rs);
    assert_eq!(nc, nr, "init({input:?},{w},{h}) NULL-ness mismatch");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er),
        "init({input:?},{w},{h}) stderr mismatch"
    );
    assert!(nc, "init({input:?},{w},{h}) should have returned NULL");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        expect_msg,
        "init({input:?},{w},{h}) unexpected diagnostic"
    );
}

/// ERRORS.md row 6 — fewer `\n`-separated rows than `height`.
#[test]
fn err_init_insufficient_rows() {
    let b = load_both();
    const MSG: &str = "Insufficient rows in input string.\n";
    check_init_err(&b, "", 1, 1, MSG);
    check_init_err(&b, "\n", 1, 1, MSG);
    check_init_err(&b, "\n\n\n", 2, 2, MSG);
    check_init_err(&b, "1 2\n", 2, 2, MSG);
    check_init_err(&b, "1 2\n3 4\n", 2, 3, MSG);
    check_init_err(&b, "", 0, 1, MSG);
    check_init_err(&b, "", 0, 5, MSG);
}

/// ERRORS.md row 7 — some row has fewer space-separated tokens than `width`;
/// the message carries the 1-based row index.
#[test]
fn err_init_insufficient_cols() {
    let b = load_both();
    check_init_err(&b, "1\n2\n", 2, 2, "Insufficient columns in row 1.\n");
    check_init_err(&b, "1 2\n3\n", 2, 2, "Insufficient columns in row 2.\n");
    check_init_err(&b, "1 2\n3 4\n5\n", 2, 3, "Insufficient columns in row 3.\n");
    check_init_err(&b, "1 2 3\n", 4, 1, "Insufficient columns in row 1.\n");
    // A row of only separators yields no column tokens at all.
    check_init_err(&b, "   \n", 1, 1, "Insufficient columns in row 1.\n");
}

// ==================== row 8: multiply dimension mismatch ==================

/// ERRORS.md row 8 — `mat_a->width != mat_b->height`.
#[test]
fn err_multiply_dim_mismatch() {
    let b = load_both();
    const MSG: &str = "Matrix dimensions do not allow multiplication.\n";
    for (wa, ha, wb, hb) in [
        (2, 2, 2, 3),
        (3, 1, 1, 2),
        (1, 1, 1, 0),
        (0, 1, 1, 1),
        (5, 2, 2, 4),
    ] {
        let run = |api: &Api| unsafe {
            let a = make_matrix(api, wa, ha, &vec![1; (wa * ha) as usize]);
            let bb = make_matrix(api, wb, hb, &vec![1; (wb * hb) as usize]);
            let (res, err) = capture_stderr(|| (api.multiply_matrices)(a, bb));
            if !res.is_null() {
                (api.free_matrix)(res);
            }
            (api.free_matrix)(a);
            (api.free_matrix)(bb);
            (res.is_null(), err)
        };
        let (nc, ec) = run(&b.c);
        let (nr, er) = run(&b.rs);
        assert_eq!(nc, nr, "multiply({wa}x{ha},{wb}x{hb}) NULL-ness mismatch");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "multiply({wa}x{ha},{wb}x{hb}) stderr mismatch"
        );
        assert!(nc);
        assert_eq!(String::from_utf8_lossy(&ec), MSG);
    }
}

// ===================== rows 9 & 10: matrix_to_string =====================

/// ERRORS.md row 9 — `matrix_to_string(NULL)`.
#[test]
fn err_matrix_to_string_null() {
    let b = load_both();
    let run = |api: &Api| {
        let (p, err) = capture_stderr(|| unsafe { (api.matrix_to_string)(std::ptr::null_mut()) });
        if !p.is_null() {
            unsafe { libc_free(p as *mut c_void) };
        }
        (p.is_null(), err)
    };
    let (nc, ec) = run(&b.c);
    let (nr, er) = run(&b.rs);
    assert_eq!(nc, nr);
    assert_eq!(String::from_utf8_lossy(&ec), String::from_utf8_lossy(&er));
    assert!(nc);
    assert_eq!(String::from_utf8_lossy(&ec), "Error: Matrix is NULL.\n");
}

/// ERRORS.md row 10 — `malloc(buffer_size)` fails because the `int` expression
/// `height*(width*10+width)+height+1` is negative and sign-extends.
#[test]
fn err_matrix_to_string_alloc_fail() {
    let b = load_both();
    for (w, bad_h) in [(1, -1), (3, -5), (2, -100), (1, -2)] {
        let run = |api: &Api| unsafe {
            // Build a real 1x1 matrix, then forge a negative height, exactly
            // the state a caller can hand the C function.
            let m = make_matrix(api, w, 1, &vec![7; w as usize]);
            let saved = (*m).height;
            (*m).height = bad_h;
            let (p, err) = capture_stderr(|| (api.matrix_to_string)(m));
            if !p.is_null() {
                libc_free(p as *mut c_void);
            }
            (*m).height = saved;
            (api.free_matrix)(m);
            (p.is_null(), err)
        };
        let (nc, ec) = run(&b.c);
        let (nr, er) = run(&b.rs);
        assert_eq!(nc, nr, "matrix_to_string(w={w},h={bad_h}) NULL-ness mismatch");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "matrix_to_string(w={w},h={bad_h}) stderr mismatch"
        );
        assert!(nc, "expected allocation failure for w={w}, h={bad_h}");
        assert!(
            ec.starts_with(b"Failed to allocate memory for matrix string"),
            "unexpected perror text: {}",
            String::from_utf8_lossy(&ec)
        );
    }
}

// ============================ write_to_file rows =========================

/// Compares `write_to_file` return code + stderr for an expected error code.
fn check_write_err(
    b: &Both,
    filename: Option<&str>,
    content: Option<&[u8]>,
    expect: c_int,
    prep: &dyn Fn(),
) {
    let fname: Option<CString> = filename.map(|f| CString::new(f).unwrap());
    let cont: Option<CString> = content.map(|c| CString::new(c.to_vec()).unwrap());
    let fp: *const c_char = fname.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    let cp: *const c_char = cont.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());

    let run = |api: &Api| {
        prep();
        capture_stderr(|| unsafe { (api.write_to_file)(fp, cp) })
    };
    let (rc_c, ec) = run(&b.c);
    let (rc_r, er) = run(&b.rs);
    assert_eq!(
        rc_c, rc_r,
        "write_to_file({filename:?}) return code mismatch: C={rc_c} Rust={rc_r}"
    );
    assert_eq!(
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er),
        "write_to_file({filename:?}) stderr mismatch"
    );
    assert_eq!(
        rc_c, expect,
        "write_to_file({filename:?}) expected {expect}, got {rc_c}"
    );
}

/// ERRORS.md row 11 — `content == NULL` -> `EINVAL` (22).
#[test]
fn err_write_null_content() {
    let b = load_both();
    let d = tmpdir("nullcontent");
    let p = d.join("out.txt").to_str().unwrap().to_string();
    check_write_err(&b, Some(&p), None, 22, &|| {});
    // Also with a NULL filename: the content check comes FIRST in the C, so
    // EINVAL still wins.
    check_write_err(&b, None, None, 22, &|| {});
    assert!(!d.join("out.txt").exists(), "no file must be created");
}

/// ERRORS.md row 12a — `fopen` fails, path inside a non-existent directory.
#[test]
fn err_write_fopen_enoent() {
    let b = load_both();
    let missing = std::env::temp_dir().join("difftest-no-such-dir-4f2a9/out.txt");
    check_write_err(&b, Some(missing.to_str().unwrap()), Some(b"x"), 2, &|| {});
}

/// ERRORS.md row 12b — `filename == ""`.
#[test]
fn err_write_fopen_empty_name() {
    let b = load_both();
    check_write_err(&b, Some(""), Some(b"x"), 2, &|| {});
}

/// ERRORS.md row 12c — `filename` names an existing directory -> EISDIR (21).
#[test]
fn err_write_fopen_eisdir() {
    let b = load_both();
    let d = tmpdir("eisdir");
    check_write_err(&b, Some(d.to_str().unwrap()), Some(b"x"), 21, &|| {});
}

/// ERRORS.md row 12d — `filename == NULL` -> glibc `fopen` yields EFAULT (14).
#[test]
fn err_write_fopen_null_name() {
    let b = load_both();
    check_write_err(&b, None, Some(b"x"), 14, &|| {});
}

/// ERRORS.md row 12e — target exists but is not writable -> EACCES (13).
#[test]
fn err_write_fopen_eacces() {
    if unsafe { libc_geteuid() } == 0 {
        eprintln!("skipping EACCES row: running as root");
        return;
    }
    let b = load_both();
    let d = tmpdir("eacces");
    let f = d.join("ro.txt");
    let fs = f.clone();
    let prep = move || {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::remove_file(&fs);
        std::fs::write(&fs, b"locked").unwrap();
        std::fs::set_permissions(&fs, std::fs::Permissions::from_mode(0o400)).unwrap();
    };
    check_write_err(&b, Some(f.to_str().unwrap()), Some(b"x"), 13, &prep);
    let _ = std::fs::remove_file(&f);
}

unsafe extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

/// ERRORS.md row 13 — the write error surfaces inside `fprintf` because the
/// payload exceeds `BUFSIZ` and is flushed immediately. `/dev/full` -> ENOSPC.
#[test]
fn err_write_fprintf_fails() {
    if !PathBuf::from("/dev/full").exists() {
        eprintln!("skipping: /dev/full unavailable");
        return;
    }
    let b = load_both();
    let big = vec![b'x'; 200_000];
    check_write_err(&b, Some("/dev/full"), Some(&big), 28, &|| {});
}

/// ERRORS.md row 14 — short payload stays buffered, so the error is only
/// reported by `fclose`. `/dev/full` -> ENOSPC.
#[test]
fn err_write_fclose_fails() {
    if !PathBuf::from("/dev/full").exists() {
        eprintln!("skipping: /dev/full unavailable");
        return;
    }
    let b = load_both();
    check_write_err(&b, Some("/dev/full"), Some(b"hello"), 28, &|| {});
    check_write_err(&b, Some("/dev/full"), Some(b""), 0, &|| {});
}

// ==================== generic FFI boundary rows (G1..G5) ==================

/// ERRORS.md G1 — zero dimensions are valid, not errors.
#[test]
fn err_zero_dims_not_an_error() {
    let b = load_both();
    check_alloc_err(&b, 0, 0, false);
    check_alloc_err(&b, 0, 3, false);
    check_alloc_err(&b, 3, 0, false);
}

/// ERRORS.md G2/G3/G4 — this API declares no enums, so the `int` axis is swept
/// with its extremes instead: `INT_MIN`, `-1`, `0`, `1`, `INT_MAX`.
#[test]
fn err_int_extremes_dims() {
    let b = load_both();
    // `height = INT_MAX` only reaches the failing allocation when the kernel
    // refuses a 17 GiB request; with overcommit_memory=1 it would instead spin
    // for 2^31 iterations, so that one case is guarded.
    let overcommit_always = std::fs::read_to_string("/proc/sys/vm/overcommit_memory")
        .map(|s| s.trim() == "1")
        .unwrap_or(false);

    let extremes = [i32::MIN, -1, 0, 1];
    for &w in &extremes {
        for &h in &extremes {
            let (nc, ec) = alloc_probe(&b.c, w, h);
            let (nr, er) = alloc_probe(&b.rs, w, h);
            assert_eq!(nc, nr, "allocate_matrix({w},{h}) NULL-ness mismatch");
            assert_eq!(
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er),
                "allocate_matrix({w},{h}) stderr mismatch"
            );
        }
    }
    // width = INT_MAX with a single row: one 8 GiB row allocation, refused.
    check_alloc_err(&b, i32::MAX, 1, true);
    check_alloc_err(&b, i32::MAX, 0, false);
    if !overcommit_always {
        check_alloc_err(&b, 1, i32::MAX, true);
        check_alloc_err(&b, 0, i32::MAX, true);
    }

    // Same extremes through initialize_matrix_from_string, where a NULL
    // `allocate_matrix` result is only safe to reach when no element is read.
    for &h in &[i32::MIN, -1] {
        let run = |api: &Api| {
            let s = cs("1 2\n3 4\n");
            let (p, err) =
                capture_stderr(|| unsafe { (api.initialize_matrix_from_string)(s.as_ptr(), 2, h) });
            if !p.is_null() {
                unsafe { (api.free_matrix)(p) };
            }
            (p.is_null(), err)
        };
        let (nc, ec) = run(&b.c);
        let (nr, er) = run(&b.rs);
        assert_eq!(nc, nr, "init(width=2,height={h}) NULL-ness mismatch");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "init(width=2,height={h}) stderr mismatch"
        );
    }

    // matrix_to_string over the same int extremes for `width`, with height 0 so
    // no element is dereferenced.
    for &w in &[i32::MIN, -1, 0, 1, i32::MAX] {
        let run = |api: &Api| unsafe {
            let m = make_matrix(api, 0, 0, &[]);
            (*m).width = w;
            let (p, err) = capture_stderr(|| (api.matrix_to_string)(m));
            let bytes = cstr_bytes(p);
            if !p.is_null() {
                libc_free(p as *mut c_void);
            }
            (*m).width = 0;
            (api.free_matrix)(m);
            (bytes, err)
        };
        let (bc, ec) = run(&b.c);
        let (br, er) = run(&b.rs);
        assert_eq!(bc, br, "matrix_to_string(width={w}, height=0) mismatch");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "matrix_to_string(width={w}) stderr mismatch"
        );
    }
}

/// ERRORS.md G5 — NULL pointers into every entry point that checks for them.
#[test]
fn err_null_pointer_sweep() {
    let b = load_both();
    // free_matrix(NULL), matrix_to_string(NULL) and write_to_file(_, NULL) are
    // covered above; here they are exercised back-to-back to confirm no
    // cross-call state divergence.
    let d = tmpdir("nullsweep");
    let p = cs(d.join("out.txt").to_str().unwrap());
    let run = |api: &Api| {
        capture_stderr(|| unsafe {
            (api.free_matrix)(std::ptr::null_mut());
            let s = (api.matrix_to_string)(std::ptr::null_mut());
            let a = (api.write_to_file)(p.as_ptr(), std::ptr::null());
            let c = (api.write_to_file)(std::ptr::null(), std::ptr::null());
            (s.is_null(), a, c)
        })
    };
    let (rc, ec) = run(&b.c);
    let (rr, er) = run(&b.rs);
    assert_eq!(rc, rr);
    assert_eq!(String::from_utf8_lossy(&ec), String::from_utf8_lossy(&er));
    assert_eq!(rc, (true, 22, 22));
}
