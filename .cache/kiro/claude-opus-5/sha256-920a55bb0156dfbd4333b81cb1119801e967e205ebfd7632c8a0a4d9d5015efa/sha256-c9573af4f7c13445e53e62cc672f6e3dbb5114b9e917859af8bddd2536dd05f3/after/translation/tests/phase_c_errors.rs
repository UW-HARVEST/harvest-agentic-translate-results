//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input the C rejects, calls **both**
//! `.so`s, and asserts they return the same sentinel / errno **and** emit the
//! same diagnostic on `stderr`.
//!
//! `stderr` is captured by temporarily `dup2`-ing fd 2 onto a file, and the
//! `driver` rows need a private working directory: both are process-global, so
//! every test in this binary serialises on one mutex.

mod common;

use common::*;
use std::ffi::{CString, c_int};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, MutexGuard, OnceLock};

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn geteuid() -> u32;
}

fn serial() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with fd 2 redirected to a temporary file and returns what it wrote.
fn capture_stderr<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_stderr_{}_{n}", std::process::id()));
    let file = std::fs::File::create(&path).expect("create stderr capture file");
    let saved = unsafe { dup(2) };
    assert!(saved >= 0, "dup(2) failed");
    unsafe {
        dup2(file.as_raw_fd(), 2);
    }
    let out = f();
    // glibc's `stderr` is unbuffered, so everything is already written.
    unsafe {
        dup2(saved, 2);
        close(saved);
    }
    drop(file);
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (out, bytes)
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

// ---------------------------------------------------------------------------
// per-entry-point differential runners (result + stderr)
// ---------------------------------------------------------------------------

/// Returns `[(is_null, snapshot_or_none, stderr)]` for C then Rust.
fn diff_alloc_extreme(width: c_int, height: c_int) {
    let mut seen = Vec::new();
    for api in both() {
        let (isnull, err) = capture_stderr(|| unsafe {
            let m = (api.allocate_matrix)(width, height);
            let isnull = m.is_null();
            if !isnull {
                (api.free_matrix)(m);
            }
            isnull
        });
        seen.push((isnull, err));
    }
    assert_eq!(
        (seen[0].0, show(&seen[0].1)),
        (seen[1].0, show(&seen[1].1)),
        "allocate_matrix({width},{height}) diverged"
    );
}

fn diff_alloc_expect_null(width: c_int, height: c_int, expect_msg: &str) {
    diff_alloc_extreme(width, height);
    let (isnull, err) = capture_stderr(|| unsafe {
        let m = (c_api().allocate_matrix)(width, height);
        let n = m.is_null();
        if !n {
            (c_api().free_matrix)(m);
        }
        n
    });
    assert!(isnull, "C: allocate_matrix({width},{height}) unexpectedly succeeded");
    assert!(
        show(&err).starts_with(expect_msg),
        "C diagnostic {:?} does not start with {expect_msg:?}",
        show(&err)
    );
}

fn diff_init_err(input: &str, width: c_int, height: c_int) -> (bool, Vec<u8>) {
    let cs = cstring(input);
    let mut seen = Vec::new();
    for api in both() {
        let (res, err) = capture_stderr(|| unsafe {
            let m = (api.initialize_matrix_from_string)(cs.as_ptr(), width, height);
            if m.is_null() {
                None
            } else {
                let s = snapshot(m);
                (api.free_matrix)(m);
                Some(s)
            }
        });
        seen.push((res, err));
    }
    assert_eq!(
        (&seen[0].0, show(&seen[0].1)),
        (&seen[1].0, show(&seen[1].1)),
        "initialize_matrix_from_string({input:?},{width},{height}) diverged"
    );
    let is_null = seen[0].0.is_none();
    (is_null, seen.remove(0).1)
}

fn diff_write_err(filename: Option<&str>, content: Option<&str>) -> (c_int, Vec<u8>) {
    let f = filename.map(cstring);
    let c = content.map(cstring);
    let fp = f.as_ref().map_or(ptr::null(), |s| s.as_ptr());
    let cp = c.as_ref().map_or(ptr::null(), |s| s.as_ptr());
    let mut seen = Vec::new();
    for api in both() {
        let (rc, err) = capture_stderr(|| unsafe { (api.write_to_file)(fp, cp) });
        seen.push((rc, err));
    }
    assert_eq!(
        (seen[0].0, show(&seen[0].1)),
        (seen[1].0, show(&seen[1].1)),
        "write_to_file({filename:?},{content:?}) diverged"
    );
    let rc = seen[0].0;
    (rc, seen.remove(0).1)
}

fn diff_write_err_bytes(filename: &str, content: &[u8]) -> (c_int, Vec<u8>) {
    let f = cstring(filename);
    let c = CString::new(content).unwrap();
    let mut seen = Vec::new();
    for api in both() {
        let (rc, err) = capture_stderr(|| unsafe { (api.write_to_file)(f.as_ptr(), c.as_ptr()) });
        seen.push((rc, err));
    }
    assert_eq!(
        (seen[0].0, show(&seen[0].1)),
        (seen[1].0, show(&seen[1].1)),
        "write_to_file({filename:?}, <{} bytes>) diverged",
        content.len()
    );
    let rc = seen[0].0;
    (rc, seen.remove(0).1)
}

fn diff_driver_err(
    wa: c_int,
    ha: c_int,
    a: &str,
    wb: c_int,
    hb: c_int,
    b: &str,
) -> (c_int, Vec<u8>) {
    let ca = cstring(a);
    let cb = cstring(b);
    let mut seen = Vec::new();
    for api in both() {
        let _ = std::fs::remove_file("matrix.txt");
        let (rc, err) =
            capture_stderr(|| unsafe { (api.driver)(wa, ha, ca.as_ptr(), wb, hb, cb.as_ptr()) });
        let bytes = std::fs::read("matrix.txt").ok();
        seen.push((rc, err, bytes));
    }
    assert_eq!(
        (seen[0].0, show(&seen[0].1), &seen[0].2),
        (seen[1].0, show(&seen[1].1), &seen[1].2),
        "driver A({wa}x{ha})={a:?} B({wb}x{hb})={b:?} diverged"
    );
    let rc = seen[0].0;
    (rc, seen.remove(0).1)
}

struct Cwd {
    original: PathBuf,
}
impl Cwd {
    fn enter(dir: &Path) -> Cwd {
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        Cwd { original }
    }
}
impl Drop for Cwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

// ---------------------------------------------------------------------------
// E2 / E3 — allocate_matrix allocation failures
// ---------------------------------------------------------------------------

#[test]
fn err_e2_allocate_rows_alloc_fail() {
    let _g = serial();
    // height < 0 => malloc((size_t)(long)height * 8) is astronomically large
    for &h in &[-1, -2, -1000, i32::MIN, i32::MIN + 1] {
        for &w in &[0, 1, 5] {
            diff_alloc_expect_null(w, h, "Failed to allocate memory for matrix rows");
        }
    }
    // height == INT_MAX => 17 GiB row array; fails under this RLIMIT_DATA.
    diff_alloc_extreme(1, i32::MAX);
    diff_alloc_extreme(0, i32::MAX);
}

#[test]
fn err_e3_allocate_cols_alloc_fail() {
    let _g = serial();
    for &w in &[-1, -7, i32::MIN, i32::MIN + 1] {
        for &h in &[1, 2, 3] {
            diff_alloc_expect_null(w, h, "Failed to allocate memory for matrix columns");
        }
    }
    // width == INT_MAX => 8 GiB per row; fails under this RLIMIT_DATA.
    diff_alloc_extreme(i32::MAX, 1);
    diff_alloc_extreme(i32::MAX, 3);
}

// ---------------------------------------------------------------------------
// E4 — free_matrix(NULL)
// ---------------------------------------------------------------------------

#[test]
fn err_e4_free_matrix_null() {
    let _g = serial();
    let mut seen = Vec::new();
    for api in both() {
        let ((), err) = capture_stderr(|| unsafe {
            for _ in 0..100 {
                (api.free_matrix)(ptr::null_mut());
            }
        });
        seen.push(err);
    }
    assert_eq!(show(&seen[0]), show(&seen[1]));
    assert_eq!(seen[0], b"", "free_matrix(NULL) must be silent");
}

// ---------------------------------------------------------------------------
// E6 — insufficient rows
// ---------------------------------------------------------------------------

#[test]
fn err_e6_insufficient_rows() {
    let _g = serial();
    let cases: &[(&str, c_int, c_int)] = &[
        ("", 1, 1),
        ("", 3, 2),
        ("\n", 1, 1),
        ("\n\n\n", 2, 1),
        ("1 2\n", 2, 2),
        ("1 2\n3 4\n", 2, 3),
        ("1\n2\n3\n", 1, 4),
        ("1 2 3", 3, 2),
        ("\n\n1 2\n\n", 2, 2),
    ];
    for &(s, w, h) in cases {
        let (is_null, err) = diff_init_err(s, w, h);
        assert!(is_null, "expected NULL for {s:?} {w}x{h}");
        assert!(
            show(&err).contains("Insufficient rows in input string."),
            "unexpected diagnostic {:?}",
            show(&err)
        );
    }
    // randomized: always one row short
    let mut rng = Rng::new(SEED ^ 106);
    for _ in 0..200 {
        let h = rng.range(1, 5) as usize;
        let w = rng.range(1, 5) as usize;
        let rows: Vec<Vec<c_int>> = (0..h - 1)
            .map(|_| (0..w).map(|_| rng.i32_in(-99, 99)).collect())
            .collect();
        let (is_null, err) = diff_init_err(&canonical(&rows), w as c_int, h as c_int);
        assert!(is_null);
        assert!(show(&err).contains("Insufficient rows in input string."));
    }
}

// ---------------------------------------------------------------------------
// E7 — insufficient columns (message carries the 1-based row number)
// ---------------------------------------------------------------------------

#[test]
fn err_e7_insufficient_cols() {
    let _g = serial();
    let cases: &[(&str, c_int, c_int, i32)] = &[
        ("1\n", 2, 1, 1),
        ("1 2\n3\n", 2, 2, 2),
        ("1 2 3\n4 5 6\n7 8\n", 3, 3, 3),
        ("   \n1 2\n", 2, 2, 1), // a whitespace-only line yields no column token
        ("   \n", 1, 1, 1),       // spaces are not row delimiters, so row 1 exists
        ("1 2\n\n\n3\n", 2, 2, 2),
    ];
    for &(s, w, h, row) in cases {
        let (is_null, err) = diff_init_err(s, w, h);
        assert!(is_null, "expected NULL for {s:?}");
        assert!(
            show(&err).contains(&format!("Insufficient columns in row {row}.")),
            "unexpected diagnostic {:?} (wanted row {row})",
            show(&err)
        );
    }
    // randomized: one random row is short
    let mut rng = Rng::new(SEED ^ 107);
    for _ in 0..200 {
        let h = rng.range(1, 5) as usize;
        let w = rng.range(2, 5) as usize;
        let short = rng.range(0, h as i64 - 1) as usize;
        let mut s = String::new();
        for i in 0..h {
            let cols = if i == short { w - 1 } else { w };
            let cells: Vec<String> = (0..cols).map(|_| rng.i32_in(-99, 99).to_string()).collect();
            s.push_str(&cells.join(" "));
            s.push('\n');
        }
        let (is_null, err) = diff_init_err(&s, w as c_int, h as c_int);
        assert!(is_null);
        assert!(
            show(&err).contains(&format!("Insufficient columns in row {}.", short + 1)),
            "wanted row {} in {:?}",
            short + 1,
            show(&err)
        );
    }
}

// ---------------------------------------------------------------------------
// E8 — allocate_matrix failure propagates silently (the C never checks `mat`)
// ---------------------------------------------------------------------------

#[test]
fn err_e8_alloc_fail_propagates_silently() {
    let _g = serial();
    // height < 0: the parse loop never runs, so the NULL matrix is returned
    // with only the allocate diagnostic on stderr.
    for &h in &[-1, -5, i32::MIN] {
        for input in ["", "1 2 3\n4 5 6\n", "junk"] {
            for &w in &[0, 1, 3] {
                let (is_null, err) = diff_init_err(input, w, h);
                assert!(is_null, "expected NULL for h={h}");
                assert!(
                    !show(&err).contains("Insufficient"),
                    "must not report insufficient rows/cols: {:?}",
                    show(&err)
                );
                assert!(show(&err).contains("Failed to allocate memory for matrix rows"));
            }
        }
    }
    // width < 0 with enough rows present: the column loop never runs either.
    for &w in &[-1, -3, i32::MIN] {
        let (is_null, err) = diff_init_err("1 2\n3 4\n5 6\n", w, 2);
        assert!(is_null, "expected NULL for w={w}");
        assert!(
            !show(&err).contains("Insufficient"),
            "must be silent about tokens: {:?}",
            show(&err)
        );
        assert!(show(&err).contains("Failed to allocate memory for matrix columns"));
    }
    // width < 0 with too few rows: the row check fires first.
    let (is_null, err) = diff_init_err("", -1, 2);
    assert!(is_null);
    assert!(show(&err).contains("Insufficient rows in input string."));
}

// ---------------------------------------------------------------------------
// E9 — multiply_matrices dimension mismatch
// ---------------------------------------------------------------------------

#[test]
fn err_e9_dimension_mismatch() {
    let _g = serial();
    // Hand-built matrix_t: the C reads only `width`/`height` on this path.
    for a_w in -2..=4 {
        for b_h in -2..=4 {
            if a_w == b_h {
                continue;
            }
            let mut seen = Vec::new();
            for api in both() {
                let mut a = MatrixT {
                    matrix: ptr::null_mut(),
                    width: a_w,
                    height: 0,
                };
                let mut b = MatrixT {
                    matrix: ptr::null_mut(),
                    width: 0,
                    height: b_h,
                };
                let (isnull, err) = capture_stderr(|| unsafe {
                    (api.multiply_matrices)(&mut a as *mut _, &mut b as *mut _).is_null()
                });
                seen.push((isnull, err));
            }
            assert_eq!(
                (seen[0].0, show(&seen[0].1)),
                (seen[1].0, show(&seen[1].1)),
                "multiply mismatch a.width={a_w} b.height={b_h} diverged"
            );
            assert!(seen[0].0, "expected NULL for a.width={a_w} b.height={b_h}");
            assert!(
                show(&seen[0].1).contains("Matrix dimensions do not allow multiplication."),
                "unexpected diagnostic {:?}",
                show(&seen[0].1)
            );
        }
    }
    // Real matrices built through allocate_matrix, mismatched.
    let mut rng = Rng::new(SEED ^ 109);
    for _ in 0..100 {
        let m = rng.range(1, 4) as usize;
        let k1 = rng.range(1, 4) as usize;
        let mut k2 = rng.range(1, 4) as usize;
        if k2 == k1 {
            k2 = k1 + 1;
        }
        let n = rng.range(1, 4) as usize;
        let mut seen = Vec::new();
        for api in both() {
            let a: Vec<Vec<c_int>> = (0..m).map(|_| vec![1; k1]).collect();
            let b: Vec<Vec<c_int>> = (0..k2).map(|_| vec![1; n]).collect();
            let (isnull, err) = capture_stderr(|| unsafe {
                let ma = build_matrix(api, &a, k1 as c_int);
                let mb = build_matrix(api, &b, n as c_int);
                let r = (api.multiply_matrices)(ma, mb);
                let isnull = r.is_null();
                if !isnull {
                    (api.free_matrix)(r);
                }
                (api.free_matrix)(mb);
                (api.free_matrix)(ma);
                isnull
            });
            seen.push((isnull, err));
        }
        assert_eq!(
            (seen[0].0, show(&seen[0].1)),
            (seen[1].0, show(&seen[1].1))
        );
        assert!(seen[0].0);
    }
}

// ---------------------------------------------------------------------------
// E10 — multiply's own allocate_matrix failure, returned silently
// ---------------------------------------------------------------------------

#[test]
fn err_e10_result_alloc_fail_silent() {
    let _g = serial();
    // a.width == b.height (so the dimension check passes) but a.height < 0, so
    // allocate_matrix fails and the loops never dereference the NULL result.
    for &a_h in &[-1, -9, i32::MIN] {
        for &shared in &[0, 1, 4] {
            let mut seen = Vec::new();
            for api in both() {
                let mut a = MatrixT {
                    matrix: ptr::null_mut(),
                    width: shared,
                    height: a_h,
                };
                let mut b = MatrixT {
                    matrix: ptr::null_mut(),
                    width: 3,
                    height: shared,
                };
                let (isnull, err) = capture_stderr(|| unsafe {
                    (api.multiply_matrices)(&mut a as *mut _, &mut b as *mut _).is_null()
                });
                seen.push((isnull, err));
            }
            assert_eq!(
                (seen[0].0, show(&seen[0].1)),
                (seen[1].0, show(&seen[1].1)),
                "a.height={a_h} shared={shared} diverged"
            );
            assert!(seen[0].0, "expected NULL result");
            assert!(
                !show(&seen[0].1).contains("do not allow multiplication"),
                "must not be the dimension error: {:?}",
                show(&seen[0].1)
            );
            assert!(show(&seen[0].1).contains("Failed to allocate memory for matrix rows"));
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — matrix_to_string(NULL)
// ---------------------------------------------------------------------------

#[test]
fn err_e11_matrix_to_string_null() {
    let _g = serial();
    let mut seen = Vec::new();
    for api in both() {
        let (res, err) =
            capture_stderr(|| unsafe { take_c_string((api.matrix_to_string)(ptr::null_mut())) });
        seen.push((res, err));
    }
    assert_eq!(
        (&seen[0].0, show(&seen[0].1)),
        (&seen[1].0, show(&seen[1].1))
    );
    assert!(seen[0].0.is_none(), "expected NULL");
    assert_eq!(show(&seen[0].1), "Error: Matrix is NULL.\n");
}

// ---------------------------------------------------------------------------
// E12 — matrix_to_string buffer-size arithmetic wraps negative
// ---------------------------------------------------------------------------

#[test]
fn err_e12_matrix_to_string_alloc_fail() {
    let _g = serial();
    // (width, height, expect_null)
    //
    // Only combinations whose `malloc` genuinely fails are safe in-process: if
    // the wrapped `buffer_size` happens to come back *positive* the C proceeds
    // to dereference `mat->matrix` (NULL here) and crashes. That case — e.g.
    // width=200000000, height=2, where the product wraps twice and lands on
    // +105032704 — is verified for crash parity in `subprocess_parity.rs`.
    let cases: &[(c_int, c_int, bool)] = &[
        (200_000_000, 1, true),
        (300_000_000, 1, true),
        (1_000_000_000, 1, true),
        (195_225_787, 1, true),
        (1, i32::MIN, false), // wraps back to buffer_size == 1 -> ""
        (0, i32::MIN, true), // 0 + INT_MIN + 1 = -2147483647 -> malloc fails
    ];
    for &(width, height, expect_null) in cases {
        let mut seen = Vec::new();
        for api in both() {
            let mut m = MatrixT {
                matrix: ptr::null_mut(),
                width,
                height,
            };
            let (res, err) =
                capture_stderr(|| unsafe { take_c_string((api.matrix_to_string)(&mut m as *mut _)) });
            seen.push((res, err));
        }
        assert_eq!(
            (&seen[0].0, show(&seen[0].1)),
            (&seen[1].0, show(&seen[1].1)),
            "matrix_to_string(width={width}, height={height}) diverged"
        );
        assert_eq!(
            seen[0].0.is_none(),
            expect_null,
            "unexpected outcome for width={width} height={height}: {:?}",
            seen[0].0
        );
        if expect_null {
            assert!(
                show(&seen[0].1).contains("Failed to allocate memory for matrix string"),
                "unexpected diagnostic {:?}",
                show(&seen[0].1)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E13 — write_to_file with NULL content
// ---------------------------------------------------------------------------

#[test]
fn err_e13_write_null_content() {
    let _g = serial();
    let dir = scratch_dir("e13");
    let good = dir.join("out.txt");
    for filename in [Some(good.to_str().unwrap()), Some(""), None] {
        let (rc, err) = diff_write_err(filename, None);
        assert_eq!(rc, 22, "expected EINVAL for filename={filename:?}");
        assert_eq!(show(&err), "Error: Content is NULL.\n");
    }
    assert!(!good.exists(), "no file must be created when content is NULL");
}

// ---------------------------------------------------------------------------
// E14 — write_to_file fopen failures (one row per distinct errno)
// ---------------------------------------------------------------------------

#[test]
fn err_e14_fopen_failures() {
    let _g = serial();
    let dir = scratch_dir("e14");

    // E14a: non-existent directory component -> ENOENT
    let missing = dir.join("no_such_dir").join("f.txt");
    let (rc, err) = diff_write_err(Some(missing.to_str().unwrap()), Some("x"));
    assert_eq!(rc, 2, "expected ENOENT");
    assert!(show(&err).starts_with("Error opening file '"));

    // E14b: empty filename -> ENOENT
    let (rc, _err) = diff_write_err(Some(""), Some("x"));
    assert_eq!(rc, 2, "expected ENOENT for empty filename");

    // E14c: filename is an existing directory -> EISDIR
    let as_dir = dir.join("is_a_dir");
    std::fs::create_dir_all(&as_dir).unwrap();
    let (rc, _err) = diff_write_err(Some(as_dir.to_str().unwrap()), Some("x"));
    assert_eq!(rc, 21, "expected EISDIR");

    // E14d: unwritable directory -> EACCES (skipped when running as root)
    let ro = dir.join("readonly");
    std::fs::create_dir_all(&ro).unwrap();
    let target = ro.join("f.txt");
    std::fs::set_permissions(&ro, <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o555))
        .unwrap();
    let (rc, _err) = diff_write_err(Some(target.to_str().unwrap()), Some("x"));
    if unsafe { geteuid() } == 0 {
        assert_eq!(rc, 0, "root can write to a 0555 directory");
    } else {
        assert_eq!(rc, 13, "expected EACCES");
    }
    std::fs::set_permissions(&ro, <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755))
        .unwrap();

    // E14e: NULL filename with non-NULL content -> EFAULT, and "(null)" in the
    // diagnostic (glibc's %s rendering of a NULL pointer).
    let (rc, err) = diff_write_err(None, Some("x"));
    assert_eq!(rc, 14, "expected EFAULT");
    assert!(
        show(&err).contains("(null)"),
        "unexpected diagnostic {:?}",
        show(&err)
    );

    // trailing-slash on a regular file -> ENOTDIR (20)
    let reg = dir.join("regular");
    std::fs::write(&reg, b"data").unwrap();
    let bogus = format!("{}/sub", reg.to_str().unwrap());
    let (rc, _err) = diff_write_err(Some(&bogus), Some("x"));
    assert_eq!(rc, 20, "expected ENOTDIR");
}

// ---------------------------------------------------------------------------
// E15 — fprintf(file, ...) fails mid-write (/dev/full, payload > stdio buffer)
// ---------------------------------------------------------------------------

#[test]
fn err_e15_fprintf_write_failure() {
    let _g = serial();
    if !Path::new("/dev/full").exists() {
        eprintln!("skipping E15: /dev/full unavailable");
        return;
    }
    let payload = vec![b'z'; 1 << 18];
    let (rc, err) = diff_write_err_bytes("/dev/full", &payload);
    assert_eq!(rc, 28, "expected ENOSPC");
    assert!(
        show(&err).contains("Error writing to file '/dev/full'"),
        "unexpected diagnostic {:?}",
        show(&err)
    );
}

// ---------------------------------------------------------------------------
// E16 — fclose fails (/dev/full, payload small enough to stay buffered)
// ---------------------------------------------------------------------------

#[test]
fn err_e16_fclose_failure() {
    let _g = serial();
    if !Path::new("/dev/full").exists() {
        eprintln!("skipping E16: /dev/full unavailable");
        return;
    }
    for body in ["x", "hello", "1 2\n3 4\n"] {
        let (rc, err) = diff_write_err(Some("/dev/full"), Some(body));
        assert_eq!(rc, 28, "expected ENOSPC for {body:?}");
        assert!(
            show(&err).contains("Error closing file '/dev/full'"),
            "unexpected diagnostic {:?}",
            show(&err)
        );
    }
    // Empty content: nothing is buffered, so fclose succeeds and the call
    // returns 0 — the mirror case that proves the branch is data-dependent.
    let (rc, err) = diff_write_err(Some("/dev/full"), Some(""));
    assert_eq!(rc, 0);
    assert_eq!(show(&err), "");
}

// ---------------------------------------------------------------------------
// E17 / E18 / E19 / E21 — driver failure paths
// ---------------------------------------------------------------------------

#[test]
fn err_e17_driver_mat_a_fail() {
    let _g = serial();
    let dir = scratch_dir("e17");
    let _cwd = Cwd::enter(&dir);
    let cases: &[(c_int, c_int, &str)] = &[
        (2, 2, ""),
        (2, 2, "1 2\n"),
        (3, 1, "1 2\n"),
        (1, -1, "1\n"),
        (-1, 1, "1 2\n"),
        (2, i32::MIN, "1 2\n3 4\n"),
    ];
    for &(wa, ha, a) in cases {
        let (rc, _err) = diff_driver_err(wa, ha, a, 2, 2, "1 2\n3 4\n");
        assert_eq!(rc, 1, "expected EXIT_FAILURE for A({wa}x{ha})={a:?}");
    }
}

#[test]
fn err_e18_driver_mat_b_fail() {
    let _g = serial();
    let dir = scratch_dir("e18");
    let _cwd = Cwd::enter(&dir);
    let cases: &[(c_int, c_int, &str)] = &[
        (2, 2, ""),
        (2, 2, "1 2\n"),
        (2, 2, "1\n2\n"),
        (2, -1, "1 2\n3 4\n"),
        (-1, 2, "1 2\n3 4\n"),
    ];
    for &(wb, hb, b) in cases {
        let (rc, _err) = diff_driver_err(2, 2, "1 2\n3 4\n", wb, hb, b);
        assert_eq!(rc, 1, "expected EXIT_FAILURE for B({wb}x{hb})={b:?}");
    }
}

#[test]
fn err_e19_driver_dim_mismatch() {
    let _g = serial();
    let dir = scratch_dir("e19");
    let _cwd = Cwd::enter(&dir);
    // width_a != height_b, both parse fine
    let mut rng = Rng::new(SEED ^ 119);
    for _ in 0..60 {
        let m = rng.range(1, 4) as usize;
        let k1 = rng.range(1, 4) as usize;
        let mut k2 = rng.range(1, 4) as usize;
        if k2 == k1 {
            k2 = k1 + 1;
        }
        let n = rng.range(1, 4) as usize;
        let a: Vec<Vec<c_int>> = (0..m)
            .map(|_| (0..k1).map(|_| rng.i32_in(-9, 9)).collect())
            .collect();
        let b: Vec<Vec<c_int>> = (0..k2)
            .map(|_| (0..n).map(|_| rng.i32_in(-9, 9)).collect())
            .collect();
        let (rc, err) = diff_driver_err(
            k1 as c_int,
            m as c_int,
            &canonical(&a),
            n as c_int,
            k2 as c_int,
            &canonical(&b),
        );
        assert_eq!(rc, 1);
        assert!(show(&err).contains("Matrix dimensions do not allow multiplication."));
    }
}

#[test]
fn err_e21_driver_write_failure() {
    let _g = serial();
    let dir = scratch_dir("e21");
    let _cwd = Cwd::enter(&dir);
    // "matrix.txt" exists as a directory -> fopen fails with EISDIR
    std::fs::create_dir_all("matrix.txt").unwrap();
    let mut seen = Vec::new();
    let ca = cstring("1 2\n3 4\n");
    for api in both() {
        let (rc, err) =
            capture_stderr(|| unsafe { (api.driver)(2, 2, ca.as_ptr(), 2, 2, ca.as_ptr()) });
        seen.push((rc, err));
    }
    assert_eq!(
        (seen[0].0, show(&seen[0].1)),
        (seen[1].0, show(&seen[1].1)),
        "driver write-failure path diverged"
    );
    assert_eq!(seen[0].0, 1, "expected EXIT_FAILURE");
    assert!(
        show(&seen[0].1).contains("Error opening file 'matrix.txt'"),
        "unexpected diagnostic {:?}",
        show(&seen[0].1)
    );
    std::fs::remove_dir("matrix.txt").unwrap();

    // read-only working directory -> EACCES (skipped as root)
    if unsafe { geteuid() } != 0 {
        std::fs::set_permissions(
            ".",
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o555),
        )
        .unwrap();
        let mut seen = Vec::new();
        for api in both() {
            let (rc, err) =
                capture_stderr(|| unsafe { (api.driver)(2, 2, ca.as_ptr(), 2, 2, ca.as_ptr()) });
            seen.push((rc, err));
        }
        std::fs::set_permissions(
            ".",
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .unwrap();
        assert_eq!(
            (seen[0].0, show(&seen[0].1)),
            (seen[1].0, show(&seen[1].1)),
            "driver read-only-cwd path diverged"
        );
        assert_eq!(seen[0].0, 1);
    }
}

// ---------------------------------------------------------------------------
// E22 — zero lengths
// ---------------------------------------------------------------------------

#[test]
fn err_e22_zero_lengths() {
    let _g = serial();
    let dir = scratch_dir("e22");

    // allocate_matrix(0, 0)
    diff_alloc_extreme(0, 0);

    // initialize_matrix_from_string("", 0, 0) succeeds
    let (is_null, err) = diff_init_err("", 0, 0);
    assert!(!is_null, "0x0 parse must succeed");
    assert_eq!(show(&err), "");

    // matrix_to_string on a 0x0 matrix -> ""
    let mut seen = Vec::new();
    for api in both() {
        let (res, err) = capture_stderr(|| unsafe {
            let m = (api.allocate_matrix)(0, 0);
            let s = take_c_string((api.matrix_to_string)(m));
            (api.free_matrix)(m);
            s
        });
        seen.push((res, err));
    }
    assert_eq!(
        (&seen[0].0, show(&seen[0].1)),
        (&seen[1].0, show(&seen[1].1))
    );
    assert_eq!(seen[0].0.as_deref(), Some(&b""[..]));

    // write_to_file with empty content succeeds and creates a 0-byte file
    for api in both() {
        let p = dir.join(format!("empty.{}", api.name));
        let pc = cstring(p.to_str().unwrap());
        let empty = cstring("");
        let rc = unsafe { (api.write_to_file)(pc.as_ptr(), empty.as_ptr()) };
        assert_eq!(rc, 0);
        assert_eq!(std::fs::read(&p).unwrap().len(), 0);
    }
}

// ---------------------------------------------------------------------------
// E23 / E24 — oversized values, one-step-past boundaries, and dimension values
// with "no valid variant" for the data supplied
// ---------------------------------------------------------------------------

#[test]
fn err_e23_oversized_and_off_by_one() {
    let _g = serial();
    let extremes: &[c_int] = &[i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];

    // allocate_matrix over the full extreme cross-product, minus the two
    // combinations that would make the C loop ~2^31 times after a *successful*
    // 17 GiB allocation (impossible under this RLIMIT_DATA, but not worth
    // depending on): height in {INT_MAX-1, INT_MAX} with a positive width.
    for &w in extremes {
        for &h in extremes {
            if h >= i32::MAX - 1 && w > 0 {
                continue;
            }
            diff_alloc_extreme(w, h);
        }
    }

    // initialize_matrix_from_string with extreme dimensions against real data.
    //
    // `width >= INT_MAX-1` with `height >= 1` and data present is excluded here:
    // the row allocation fails, the C never checks it, and the column loop then
    // dereferences the NULL matrix. That crash is verified for parity in
    // `subprocess_parity.rs`.
    for &w in extremes {
        for &h in &[i32::MIN, -1, 0, 1, 2, 3] {
            if w >= i32::MAX - 1 && h >= 1 {
                diff_init_err("", w, h); // no rows: rejected before the deref
                continue;
            }
            diff_init_err("1 2 3\n4 5 6\n", w, h);
            diff_init_err("", w, h);
        }
    }

    // matrix_to_string: `j < width - 1` boundary, incl. width == INT_MIN where
    // `width - 1` wraps to INT_MAX
    for &w in extremes {
        for &h in &[i32::MIN, -1, 0] {
            let mut seen = Vec::new();
            for api in both() {
                let mut m = MatrixT {
                    matrix: ptr::null_mut(),
                    width: w,
                    height: h,
                };
                let (res, err) = capture_stderr(|| unsafe {
                    take_c_string((api.matrix_to_string)(&mut m as *mut _))
                });
                seen.push((res, err));
            }
            assert_eq!(
                (&seen[0].0, show(&seen[0].1)),
                (&seen[1].0, show(&seen[1].1)),
                "matrix_to_string(width={w}, height={h}) diverged"
            );
        }
    }
    // width == INT_MIN with a real 1-row matrix: `width - 1` wraps to INT_MAX so
    // the separator is emitted, and buffer_size wraps — compare whatever the C
    // does.
    for &h in &[0, -1] {
        let mut seen = Vec::new();
        for api in both() {
            let mut m = MatrixT {
                matrix: ptr::null_mut(),
                width: i32::MIN,
                height: h,
            };
            let (res, err) =
                capture_stderr(|| unsafe { take_c_string((api.matrix_to_string)(&mut m as *mut _)) });
            seen.push((res, err));
        }
        assert_eq!(
            (&seen[0].0, show(&seen[0].1)),
            (&seen[1].0, show(&seen[1].1))
        );
    }

    // multiply_matrices with extreme dimension fields (matrix pointers unused
    // on every path reached here)
    for &a_w in extremes {
        for &b_h in extremes {
            let mut seen = Vec::new();
            for api in both() {
                let mut a = MatrixT {
                    matrix: ptr::null_mut(),
                    width: a_w,
                    height: if a_w == b_h { -1 } else { 0 },
                };
                let mut b = MatrixT {
                    matrix: ptr::null_mut(),
                    width: 0,
                    height: b_h,
                };
                let (isnull, err) = capture_stderr(|| unsafe {
                    (api.multiply_matrices)(&mut a as *mut _, &mut b as *mut _).is_null()
                });
                seen.push((isnull, err));
            }
            assert_eq!(
                (seen[0].0, show(&seen[0].1)),
                (seen[1].0, show(&seen[1].1)),
                "multiply(a.width={a_w}, b.height={b_h}) diverged"
            );
            assert!(seen[0].0, "expected NULL for a.width={a_w} b.height={b_h}");
        }
    }
}

#[test]
fn err_e24_dimension_vs_data_disagreement() {
    let _g = serial();
    // Dimensions that disagree with the data shape: the FFI analogue of an
    // out-of-range enum value. Every combination is compared, whether it
    // succeeds (extra data ignored) or fails.
    let inputs = [
        "",
        "\n",
        "1",
        "1 2",
        "1 2\n",
        "1 2\n3 4\n",
        "1 2 3\n4 5 6\n7 8 9\n",
        "  1   2  \n\n 3 4 \n",
        "a b\nc d\n",
    ];
    for input in inputs {
        for w in -1..=4 {
            for h in -1..=4 {
                diff_init_err(input, w, h);
            }
        }
    }
    // And the same through the driver.
    let dir = scratch_dir("e24");
    let _cwd = Cwd::enter(&dir);
    let mut rng = Rng::new(SEED ^ 124);
    for _ in 0..120 {
        let a = *rng.pick(&inputs);
        let b = *rng.pick(&inputs);
        let wa = rng.i32_in(-1, 3);
        let ha = rng.i32_in(-1, 3);
        let wb = rng.i32_in(-1, 3);
        let hb = rng.i32_in(-1, 3);
        diff_driver_err(wa, ha, a, wb, hb, b);
    }
}

// ---------------------------------------------------------------------------
