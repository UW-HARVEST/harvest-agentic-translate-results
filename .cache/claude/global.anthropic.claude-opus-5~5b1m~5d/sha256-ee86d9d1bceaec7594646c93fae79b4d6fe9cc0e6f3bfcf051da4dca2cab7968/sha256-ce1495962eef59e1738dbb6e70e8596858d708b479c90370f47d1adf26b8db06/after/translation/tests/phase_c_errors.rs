//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH `.so`s
//! through their exported symbols, and asserts the same error code / sentinel
//! *and* the same `stderr` bytes.

mod harness;

use harness::*;
use std::os::raw::c_int;
use std::ptr;

// glibc errno values (verified against <errno.h> on this platform)
const EINVAL: c_int = 22;
const ENOENT: c_int = 2;
const EISDIR: c_int = 21;
const EFAULT: c_int = 14;
const ENOTDIR: c_int = 20;
const ENAMETOOLONG: c_int = 36;
const EACCES: c_int = 13;
const ENOSPC: c_int = 28;
const EXIT_FAILURE: c_int = 1;
const EXIT_SUCCESS: c_int = 0;

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

fn run_pair<T: PartialEq + std::fmt::Debug>(tag: &str, f: impl Fn(&Api) -> T) -> T {
    let _g = lock();
    let (rc, ec) = with_captured_stderr("c", || f(c_api()));
    let (rr, er) = with_captured_stderr("r", || f(rust_api()));
    assert_eq!(
        show(&ec),
        show(&er),
        "[{tag}] stderr differs\n  C   : {:?}\n  Rust: {:?}",
        show(&ec),
        show(&er)
    );
    assert_eq!(rc, rr, "[{tag}] result differs");
    rc
}

/// Differential comparison for paths the C leaves **undefined** (a NULL
/// dereference). The C compiles these to a raw fault; a Rust build with
/// `debug_assertions` instead panics + aborts with a message. Under the shipped
/// (release) artifact the two are byte-identical, which is what `ub_strict()`
/// asserts; for a debug artifact we only require that both terminate abnormally.
fn assert_same_ub(tag: &str, exit_c: &Exit, exit_r: &Exit, err_c: &[u8], err_r: &[u8]) {
    let abnormal = |e: &Exit| matches!(e, Exit::Signal(_));
    assert!(
        abnormal(exit_c) == abnormal(exit_r),
        "[{tag}] one side faulted and the other did not: C={exit_c:?} Rust={exit_r:?}"
    );
    if ub_strict() {
        assert_eq!(
            show(err_c),
            show(err_r),
            "[{tag}] stderr differs on the UB path"
        );
        assert_eq!(exit_c, exit_r, "[{tag}] termination differs on the UB path");
    } else {
        eprintln!(
            "[{tag}] debug artifact: relaxed UB check (C={exit_c:?} Rust={exit_r:?})"
        );
    }
}

/// Same, but each side runs in a forked child (optionally with a capped address
/// space) so that allocation failures and faults can be observed.
fn run_pair_child(
    tag: &str,
    address_space_slack: Option<u64>,
    body: impl Fn(&'static Api, i32),
) -> (Exit, Vec<u8>, Vec<u8>) {
    let _g = lock();
    // Resolve both libraries *before* forking so no dlopen happens in a child.
    let c = c_api();
    let r = rust_api();
    let (xc, errc, payc) = run_in_child(&format!("{tag}-c"), address_space_slack, |fd| body(c, fd));
    let (xr, errr, payr) = run_in_child(&format!("{tag}-r"), address_space_slack, |fd| body(r, fd));
    assert_eq!(
        show(&errc),
        show(&errr),
        "[{tag}] child stderr differs\n  C   : {:?}\n  Rust: {:?}",
        show(&errc),
        show(&errr)
    );
    assert_eq!(
        show(&payc),
        show(&payr),
        "[{tag}] child payload differs\n  C   : {:?}\n  Rust: {:?}",
        show(&payc),
        show(&payr)
    );
    assert_eq!(xc, xr, "[{tag}] child exit status differs");
    assert_ne!(xc, Exit::Timeout, "[{tag}] child timed out");
    (xc, errc, payc)
}

/// Child-based comparison for an undefined-behaviour path.
fn run_pair_child_ub(
    tag: &str,
    body: impl Fn(&'static Api, i32),
) -> (Exit, Exit) {
    let _g = lock();
    let c = c_api();
    let r = rust_api();
    let (xc, errc, payc) = run_in_child(&format!("{tag}-c"), None, |fd| body(c, fd));
    let (xr, errr, payr) = run_in_child(&format!("{tag}-r"), None, |fd| body(r, fd));
    assert_ne!(xc, Exit::Timeout, "[{tag}] C child timed out");
    assert_ne!(xr, Exit::Timeout, "[{tag}] Rust child timed out");
    // Whatever the child managed to report before dying must agree.
    assert_eq!(
        show(&payc),
        show(&payr),
        "[{tag}] child payload differs\n  C   : {:?}\n  Rust: {:?}",
        show(&payc),
        show(&payr)
    );
    assert_same_ub(tag, &xc, &xr, &errc, &errr);
    (xc, xr)
}

/// `initialize_matrix_from_string` → observe → free.
fn init_observe(api: &Api, text: &str, w: c_int, h: c_int) -> MatObs {
    let c = cstr(text);
    unsafe {
        let m = (api.initialize_matrix_from_string)(c.as_ptr(), w, h);
        let o = observe(m);
        (api.free_matrix)(m);
        o
    }
}

/// The exact `int` expression the C uses to size its buffer:
/// `mat->height * (mat->width * 10 + mat->width) + mat->height + 1`.
fn buffer_size(w: c_int, h: c_int) -> c_int {
    h.wrapping_mul(w.wrapping_mul(10).wrapping_add(w))
        .wrapping_add(h)
        .wrapping_add(1)
}

unsafe fn observe_shape(mat: *mut MatrixT) -> (bool, c_int, c_int, bool) {
    if mat.is_null() {
        return (true, 0, 0, false);
    }
    (false, (*mat).width, (*mat).height, !(*mat).matrix.is_null())
}

// ===========================================================================
// E1 — allocate_matrix: malloc(sizeof(matrix_t)) fails
// ===========================================================================
/// Compare a pair of freshly re-exec'd children (one per library).
fn compare_self_children(tag: &str, test_name: &str, slack: Option<u64>) -> (Vec<u8>, Vec<u8>) {
    let _g = lock();
    let (xc, errc, payc) = spawn_child(test_name, "c", slack);
    let (xr, errr, payr) = spawn_child(test_name, "rust", slack);
    assert_eq!(
        show(&errc),
        show(&errr),
        "[{tag}] child stderr differs\n  C   : {:?}\n  Rust: {:?}",
        show(&errc),
        show(&errr)
    );
    assert_eq!(
        show(&payc),
        show(&payr),
        "[{tag}] child payload differs\n  C   : {:?}\n  Rust: {:?}",
        show(&payc),
        show(&payr)
    );
    assert_eq!(xc, xr, "[{tag}] child exit status differs");
    assert_eq!(xc, Exit::Code(0), "[{tag}] children must exit cleanly");
    (errc, payc)
}

#[test]
fn e1_allocate_matrix_struct_malloc_fails() {
    if let Some(cfg) = child_cfg() {
        let api = cfg.api(); // dlopen before the heap is capped
        let fd = cfg.open_out();
        cfg.apply_limit();
        unsafe {
            let injected = exhaust_heap();
            let m = (api.allocate_matrix)(2, 2);
            report(
                fd,
                match (injected, m.is_null()) {
                    (true, true) => b"NULL",
                    (true, false) => b"PTR_",
                    (false, _) => b"CAP_",
                },
            );
        }
        cfg.exit_now(fd);
    }

    let (err, pay) = compare_self_children("e1", "e1_allocate_matrix_struct_malloc_fails", Some(1 << 20));
    assert_eq!(
        pay, b"NULL",
        "allocate_matrix must return NULL under OOM (payload {:?}, stderr {:?})",
        show(&pay),
        show(&err)
    );
    assert!(
        String::from_utf8_lossy(&err).contains("Failed to allocate memory for matrix struct"),
        "unexpected stderr: {:?}",
        show(&err)
    );
}

// ===========================================================================
// E2 — allocate_matrix: rows malloc fails (negative height)
// ===========================================================================
#[test]
fn e2_allocate_matrix_negative_height() {
    for h in [-1, -2, -7, -1000, i32::MIN, i32::MIN + 1] {
        for w in [0, 1, 2, 5] {
            let r = run_pair(&format!("e2-{w}x{h}"), |api| unsafe {
                let m = (api.allocate_matrix)(w, h);
                let s = observe_shape(m);
                (api.free_matrix)(m);
                s
            });
            assert_eq!(r.0, true, "allocate_matrix({w},{h}) must be NULL");
        }
    }
    // and the exact stderr text
    let _g = lock();
    let (_, err) = with_captured_stderr("e2msg", || unsafe {
        (c_api().allocate_matrix)(3, -1);
    });
    assert!(
        String::from_utf8_lossy(&err).starts_with("Failed to allocate memory for matrix rows"),
        "unexpected stderr: {:?}",
        show(&err)
    );
}

// ===========================================================================
// E3 — allocate_matrix: column malloc fails (negative width, height >= 1)
// ===========================================================================
#[test]
fn e3_allocate_matrix_negative_width() {
    for w in [-1, -2, -9, -100000, i32::MIN, i32::MIN + 1] {
        for h in [1, 2, 5] {
            let r = run_pair(&format!("e3-{w}x{h}"), |api| unsafe {
                let m = (api.allocate_matrix)(w, h);
                let s = observe_shape(m);
                (api.free_matrix)(m);
                s
            });
            assert_eq!(r.0, true, "allocate_matrix({w},{h}) must be NULL");
        }
    }
    let _g = lock();
    let (_, err) = with_captured_stderr("e3msg", || unsafe {
        (c_api().allocate_matrix)(-1, 2);
    });
    assert!(
        String::from_utf8_lossy(&err).starts_with("Failed to allocate memory for matrix columns"),
        "unexpected stderr: {:?}",
        show(&err)
    );
}

// ===========================================================================
// E4 — free_matrix(NULL) is a no-op
// ===========================================================================
#[test]
fn e4_free_matrix_null() {
    let out = run_pair("e4", |api| unsafe {
        (api.free_matrix)(ptr::null_mut());
        (api.free_matrix)(ptr::null_mut());
        (api.free_matrix)(ptr::null_mut());
        0u8
    });
    assert_eq!(out, 0);
}

// ===========================================================================
// E5 — initialize_matrix_from_string: strdup fails
// ===========================================================================
#[test]
fn e5_init_strdup_fails() {
    if let Some(cfg) = child_cfg() {
        let api = cfg.api();
        let text = cstr("1 2\n3 4");
        let fd = cfg.open_out();
        cfg.apply_limit();
        unsafe {
            let injected = exhaust_heap();
            // width == height == 0 so that the only thing that can fail is the
            // strdup (allocate_matrix's own struct malloc fails first and is
            // reported too, exactly as the C does).
            let m = (api.initialize_matrix_from_string)(text.as_ptr(), 0, 0);
            report(
                fd,
                match (injected, m.is_null()) {
                    (true, true) => b"NULL",
                    (true, false) => b"PTR_",
                    (false, _) => b"CAP_",
                },
            );
        }
        cfg.exit_now(fd);
    }

    let (err, pay) = compare_self_children("e5", "e5_init_strdup_fails", Some(1 << 20));
    assert_eq!(pay, b"NULL", "stderr was {:?}", show(&err));
    assert!(
        String::from_utf8_lossy(&err).contains("Failed to duplicate input string"),
        "unexpected stderr: {:?}",
        show(&err)
    );
}

// ===========================================================================
// E6 — insufficient rows
// ===========================================================================
#[test]
fn e6_init_insufficient_rows() {
    let cases: &[(&str, c_int, c_int)] = &[
        ("", 1, 1),
        ("", 3, 2),
        ("1 2", 2, 2),
        ("1 2\n3 4", 2, 3),
        ("1 2\n3 4\n5 6", 2, 4),
        ("\n", 1, 1),
        ("\n\n\n", 1, 1),
        // NOTE: ("   ", 0, 1) is *not* here — `strtok_r("   ", "\n")` yields the
        // whole blank row, so one row is available and the call succeeds.
        // Two rows, however, are not:
        ("   ", 0, 2),
        ("1", 1, 2),
        ("1\n2\n3", 1, 4),
        ("1 2", 0, 2),
        ("1 2", 0, 9),
        ("1 2", -1, 2),
        ("1 2", -5, 3),
    ];
    for (t, w, h) in cases {
        let r = run_pair(&format!("e6-{}-{w}x{h}", show(t.as_bytes())), |api| {
            init_observe(api, t, *w, *h)
        });
        assert_eq!(
            r,
            MatObs::Null,
            "init({:?},{w},{h}) must be NULL",
            show(t.as_bytes())
        );
    }
    let _g = lock();
    let (_, err) = with_captured_stderr("e6msg", || {
        let t = cstr("");
        unsafe {
            (c_api().initialize_matrix_from_string)(t.as_ptr(), 1, 1);
        }
    });
    assert_eq!(
        String::from_utf8_lossy(&err),
        "Insufficient rows in input string.\n"
    );
}

// ===========================================================================
// E7 — insufficient columns (message carries the 1-based row number)
// ===========================================================================
#[test]
fn e7_init_insufficient_columns() {
    let cases: &[(&str, c_int, c_int)] = &[
        ("1", 2, 1),
        ("1 2\n3", 2, 2),
        ("1 2 3\n4 5 6\n7 8", 3, 3),
        ("1 2 3\n4 5\n7 8 9", 3, 3),
        ("   1   ", 2, 1),
        ("1\n2\n3", 2, 3),
        ("a b\nc", 2, 2),
        ("1 2 3 4 5", 6, 1),
    ];
    for (t, w, h) in cases {
        let r = run_pair(&format!("e7-{}-{w}x{h}", show(t.as_bytes())), |api| {
            init_observe(api, t, *w, *h)
        });
        assert_eq!(r, MatObs::Null);
    }
    // exact message, including the row index, for a failure in row 2 and row 3
    for (t, w, h, expect) in [
        ("1 2\n3", 2, 2, "Insufficient columns in row 2.\n"),
        (
            "1 2 3\n4 5 6\n7 8",
            3,
            3,
            "Insufficient columns in row 3.\n",
        ),
        ("1", 2, 1, "Insufficient columns in row 1.\n"),
    ] {
        let _g = lock();
        let (_, ec) = with_captured_stderr("e7c", || {
            let s = cstr(t);
            unsafe {
                (c_api().initialize_matrix_from_string)(s.as_ptr(), w, h);
            }
        });
        let (_, er) = with_captured_stderr("e7r", || {
            let s = cstr(t);
            unsafe {
                (rust_api().initialize_matrix_from_string)(s.as_ptr(), w, h);
            }
        });
        assert_eq!(String::from_utf8_lossy(&ec), expect);
        assert_eq!(String::from_utf8_lossy(&er), expect);
    }
}

// ===========================================================================
// E8 — multiply_matrices: dimension mismatch
// ===========================================================================
#[test]
fn e8_multiply_dimension_mismatch() {
    let mut rng = Rng::new(108);
    for iter in 0..120u64 {
        let wa = rng.range(0, 8) as c_int;
        let ha = rng.range(0, 8) as c_int;
        let mut hb = rng.range(0, 8) as c_int;
        if hb == wa {
            hb = (hb + 1).min(8);
            if hb == wa {
                hb = wa.saturating_sub(1).max(0);
            }
        }
        if hb == wa {
            continue;
        }
        let wb = rng.range(0, 8) as c_int;
        let r = run_pair(&format!("e8-{iter}-{ha}x{wa}*{hb}x{wb}"), |api| unsafe {
            let a = (api.allocate_matrix)(wa, ha);
            let b = (api.allocate_matrix)(wb, hb);
            let res = (api.multiply_matrices)(a, b);
            let s = observe_shape(res);
            (api.free_matrix)(res);
            (api.free_matrix)(a);
            (api.free_matrix)(b);
            s
        });
        assert_eq!(r.0, true, "multiply must be NULL for mismatched dims");
    }
    let _g = lock();
    let (_, err) = with_captured_stderr("e8msg", || unsafe {
        let api = c_api();
        let a = (api.allocate_matrix)(2, 2);
        let b = (api.allocate_matrix)(3, 1);
        (api.multiply_matrices)(a, b);
        (api.free_matrix)(a);
        (api.free_matrix)(b);
    });
    assert_eq!(
        String::from_utf8_lossy(&err),
        "Matrix dimensions do not allow multiplication.\n"
    );
}

// ===========================================================================
// E9 — matrix_to_string(NULL)
// ===========================================================================
#[test]
fn e9_to_string_null() {
    let r = run_pair("e9", |api| unsafe {
        observe_and_free_cstring((api.matrix_to_string)(ptr::null_mut()))
    });
    assert_eq!(r, StrObs::Null);
    let _g = lock();
    let (_, err) = with_captured_stderr("e9msg", || unsafe {
        (c_api().matrix_to_string)(ptr::null_mut());
    });
    assert_eq!(String::from_utf8_lossy(&err), "Error: Matrix is NULL.\n");
}

// ===========================================================================
// E10 — matrix_to_string: buffer_size <= 0 so malloc fails
// ===========================================================================
#[test]
fn e10_to_string_negative_buffer_size() {
    // (width, height) pairs for which
    //   buffer_size = h*(w*10 + w) + h + 1
    // is <= 0 as a 32-bit int. The `matrix` pointer is never dereferenced on
    // this path, so a null row array is safe (and matches what C would do).
    let candidates: &[(c_int, c_int)] = &[
        (-1, 1),
        (-5, 3),
        (1, -1),
        (3, -5),
        (-1, -1),
        (i32::MIN, 1),
        (1, i32::MIN),
        (i32::MIN, i32::MIN),
        (i32::MAX, 2),
        (2, i32::MAX),
        (100_000, 100_000),
        (65536, 65536),
        (46_341, 46_341),
        (-100_000, 7),
        (7, -100_000),
        (-7, 100_000),
        (i32::MAX, i32::MAX),
        (i32::MIN + 1, 5),
        (5, i32::MIN + 1),
    ];
    let mut covered = 0usize;
    for (w, h) in candidates {
        // Only the pairs whose `int` buffer_size is <= 0 hit the malloc-failure
        // branch. For the others the malloc succeeds and the C would go on to
        // dereference the (NULL) row array — that is a different, undefined
        // path and is exercised by G2 instead.
        if buffer_size(*w, *h) > 0 {
            continue;
        }
        covered += 1;
        let r = run_pair(&format!("e10-{w}x{h}"), |api| unsafe {
            let mut m = MatrixT {
                matrix: ptr::null_mut(),
                width: *w,
                height: *h,
            };
            observe_and_free_cstring((api.matrix_to_string)(&mut m))
        });
        assert_eq!(
            r,
            StrObs::Null,
            "matrix_to_string with w={w} h={h} must be NULL (buffer_size={})",
            buffer_size(*w, *h)
        );
    }
    assert!(covered >= 10, "expected many covered pairs, got {covered}");
    let _g = lock();
    let (_, err) = with_captured_stderr("e10msg", || unsafe {
        let mut m = MatrixT {
            matrix: ptr::null_mut(),
            width: -5,
            height: 3,
        };
        (c_api().matrix_to_string)(&mut m);
    });
    assert!(
        String::from_utf8_lossy(&err).starts_with("Failed to allocate memory for matrix string"),
        "unexpected stderr: {:?}",
        show(&err)
    );
}

// ===========================================================================
// E11 — write_to_file: content == NULL -> EINVAL
// ===========================================================================
#[test]
fn e11_write_null_content() {
    for name in [
        "target/difftest/e11.txt",
        "",
        "/definitely/not/here",
        "/dev/null",
    ] {
        let r = run_pair(&format!("e11-{name}"), |api| {
            let p = cstr(name);
            unsafe { (api.write_to_file)(p.as_ptr(), ptr::null()) }
        });
        assert_eq!(r, EINVAL, "content==NULL must yield EINVAL");
    }
    // NULL filename *and* NULL content: the content check comes first
    let r = run_pair("e11-nullname", |api| unsafe {
        (api.write_to_file)(ptr::null(), ptr::null())
    });
    assert_eq!(r, EINVAL);

    let _g = lock();
    let (_, err) = with_captured_stderr("e11msg", || unsafe {
        let p = cstr("target/difftest/e11.txt");
        (c_api().write_to_file)(p.as_ptr(), ptr::null());
    });
    assert_eq!(String::from_utf8_lossy(&err), "Error: Content is NULL.\n");
}

// ===========================================================================
// E12 — write_to_file: fopen failures (a..g)
// ===========================================================================
#[test]
fn e12_write_fopen_failures() {
    let dir = scratch_dir();
    let notdir = dir.join("e12-regular");
    std::fs::write(&notdir, b"x").unwrap();
    let rodir = dir.join("e12-rodir");
    let _ = std::fs::create_dir(&rodir);
    let mut perm = std::fs::metadata(&rodir).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perm.set_mode(0o500);
    }
    std::fs::set_permissions(&rodir, perm).unwrap();

    let long = "z".repeat(5000);
    let notdir_child = format!("{}/child", notdir.display());
    let rodir_child = format!("{}/child", rodir.display());
    let am_root = unsafe { libc::geteuid() } == 0;

    // (tag, filename, expected errno)
    let mut cases: Vec<(&str, String, c_int)> = vec![
        ("e12a-empty", String::new(), ENOENT),
        (
            "e12b-missing-dir",
            "/nonexistent_dir_xyz_123/f".to_string(),
            ENOENT,
        ),
        ("e12c-isdir", dir.display().to_string(), EISDIR),
        ("e12c-isdir-dot", ".".to_string(), EISDIR),
        ("e12e-notdir", notdir_child, ENOTDIR),
        ("e12f-toolong", long, ENAMETOOLONG),
    ];
    if !am_root {
        cases.push(("e12g-eacces", rodir_child, EACCES));
    }

    for (tag, name, expect) in &cases {
        let r = run_pair(tag, |api| {
            let p = cstr(name);
            let c = cstr("payload");
            unsafe { (api.write_to_file)(p.as_ptr(), c.as_ptr()) }
        });
        assert_eq!(r, *expect, "[{tag}] wrong errno for {name:?}");
    }

    // E12d — NULL filename crosses the FFI boundary; glibc's open(NULL) yields
    // EFAULT and "%s" renders "(null)".
    let r = run_pair("e12d-nullname", |api| {
        let c = cstr("payload");
        unsafe { (api.write_to_file)(ptr::null(), c.as_ptr()) }
    });
    assert_eq!(r, EFAULT);
    let _g = lock();
    let (_, ec) = with_captured_stderr("e12dc", || unsafe {
        let c = cstr("payload");
        (c_api().write_to_file)(ptr::null(), c.as_ptr());
    });
    assert_eq!(
        String::from_utf8_lossy(&ec),
        "Error opening file '(null)': Bad address\n"
    );

    // cleanup
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&rodir).unwrap().permissions();
        p.set_mode(0o700);
        let _ = std::fs::set_permissions(&rodir, p);
    }
    let _ = std::fs::remove_dir_all(&rodir);
    let _ = std::fs::remove_file(&notdir);
}

// ===========================================================================
// E13 — write_to_file: fprintf fails mid-stream (/dev/full, > BUFSIZ)
// ===========================================================================
#[test]
fn e13_write_fprintf_fails_dev_full() {
    if !std::path::Path::new("/dev/full").exists() {
        eprintln!("skip e13: /dev/full unavailable");
        return;
    }
    for n in [200_000usize, 65_536, 8_192] {
        let content: String = std::iter::repeat('A').take(n).collect();
        let r = run_pair(&format!("e13-{n}"), |api| {
            let p = cstr("/dev/full");
            let c = cstr(&content);
            unsafe { (api.write_to_file)(p.as_ptr(), c.as_ptr()) }
        });
        assert_eq!(r, ENOSPC, "/dev/full with {n} bytes must give ENOSPC");
    }
    let _g = lock();
    let content: String = std::iter::repeat('A').take(200_000).collect();
    let (_, ec) = with_captured_stderr("e13c", || unsafe {
        let p = cstr("/dev/full");
        let c = cstr(&content);
        (c_api().write_to_file)(p.as_ptr(), c.as_ptr());
    });
    let (_, er) = with_captured_stderr("e13r", || unsafe {
        let p = cstr("/dev/full");
        let c = cstr(&content);
        (rust_api().write_to_file)(p.as_ptr(), c.as_ptr());
    });
    assert_eq!(
        String::from_utf8_lossy(&ec),
        "Error writing to file '/dev/full': No space left on device\n"
    );
    assert_eq!(show(&ec), show(&er));
}

// ===========================================================================
// E14 — write_to_file: fclose fails at flush (/dev/full, < BUFSIZ)
// ===========================================================================
#[test]
fn e14_write_fclose_fails_dev_full() {
    if !std::path::Path::new("/dev/full").exists() {
        eprintln!("skip e14: /dev/full unavailable");
        return;
    }
    for content in ["hi", "", "a", "x".repeat(100).as_str()] {
        let r = run_pair(&format!("e14-{}", content.len()), |api| {
            let p = cstr("/dev/full");
            let c = cstr(content);
            unsafe { (api.write_to_file)(p.as_ptr(), c.as_ptr()) }
        });
        // An empty content writes nothing, so the close still fails only if
        // something was buffered; assert C and Rust agree on whatever happens.
        if !content.is_empty() {
            assert_eq!(r, ENOSPC, "/dev/full small write must fail at fclose");
        }
    }
    let _g = lock();
    let (_, ec) = with_captured_stderr("e14c", || unsafe {
        let p = cstr("/dev/full");
        let c = cstr("hi");
        (c_api().write_to_file)(p.as_ptr(), c.as_ptr());
    });
    let (_, er) = with_captured_stderr("e14r", || unsafe {
        let p = cstr("/dev/full");
        let c = cstr("hi");
        (rust_api().write_to_file)(p.as_ptr(), c.as_ptr());
    });
    assert_eq!(
        String::from_utf8_lossy(&ec),
        "Error closing file '/dev/full': No space left on device\n"
    );
    assert_eq!(show(&ec), show(&er));
}

// ---------------------------------------------------------------------------
// driver helper
// ---------------------------------------------------------------------------
const OUT_FILE: &str = "matrix.txt";

fn driver_case(
    api: &Api,
    wa: c_int,
    ha: c_int,
    ta: &str,
    wb: c_int,
    hb: c_int,
    tb: &str,
) -> (c_int, Option<Vec<u8>>) {
    let _ = std::fs::remove_file(OUT_FILE);
    let ca = cstr(ta);
    let cb = cstr(tb);
    let rc = unsafe { (api.driver)(wa, ha, ca.as_ptr(), wb, hb, cb.as_ptr()) };
    let bytes = std::fs::read(OUT_FILE).ok();
    let _ = std::fs::remove_file(OUT_FILE);
    (rc, bytes)
}

// ===========================================================================
// E15 — driver: matrix_a fails to parse
// ===========================================================================
#[test]
fn e15_driver_mat_a_null() {
    let cases: [(c_int, c_int, &str, c_int, c_int, &str); 5] = [
        (2, 2, "1 2", 2, 2, "1 2\n3 4"),
        (2, 2, "1\n2", 2, 2, "1 2\n3 4"),
        (1, 1, "", 1, 1, "5"),
        (2, -1, "1 2", 2, 2, "1 2\n3 4"),
        (3, 2, "1 2\n3 4", 2, 3, "1 2\n3 4\n5 6"),
    ];
    for (i, (wa, ha, ta, wb, hb, tb)) in cases.iter().enumerate() {
        let r = run_pair(&format!("e15-{i}"), |api| {
            driver_case(api, *wa, *ha, ta, *wb, *hb, tb)
        });
        assert_eq!(r.0, EXIT_FAILURE, "[e15-{i}] driver must fail");
        assert_eq!(r.1, None, "[e15-{i}] nothing should be written");
    }
}

// ===========================================================================
// E16 — driver: matrix_b fails to parse (matrix_a is fine)
// ===========================================================================
#[test]
fn e16_driver_mat_b_null() {
    let cases: [(c_int, c_int, &str, c_int, c_int, &str); 5] = [
        (2, 2, "1 2\n3 4", 2, 2, "1 2"),
        (2, 2, "1 2\n3 4", 2, 2, "1\n2"),
        (1, 1, "5", 1, 1, ""),
        (2, 2, "1 2\n3 4", 2, -3, "1 2\n3 4"),
        (2, 2, "1 2\n3 4", 5, 2, "1 2 3 4 5\n6 7 8"),
    ];
    for (i, (wa, ha, ta, wb, hb, tb)) in cases.iter().enumerate() {
        let r = run_pair(&format!("e16-{i}"), |api| {
            driver_case(api, *wa, *ha, ta, *wb, *hb, tb)
        });
        assert_eq!(r.0, EXIT_FAILURE);
        assert_eq!(r.1, None);
    }
}

// ===========================================================================
// E17 — driver: multiply_matrices rejects the dimensions
// ===========================================================================
#[test]
fn e17_driver_dimension_mismatch() {
    let cases: [(c_int, c_int, &str, c_int, c_int, &str); 4] = [
        (2, 2, "1 2\n3 4", 3, 1, "1 2 3"),
        (3, 1, "1 2 3", 2, 2, "1 2\n3 4"),
        (1, 1, "5", 1, 2, "6\n7"),
        (2, 3, "1 2\n3 4\n5 6", 2, 3, "1 2\n3 4\n5 6"),
    ];
    for (i, (wa, ha, ta, wb, hb, tb)) in cases.iter().enumerate() {
        let r = run_pair(&format!("e17-{i}"), |api| {
            driver_case(api, *wa, *ha, ta, *wb, *hb, tb)
        });
        assert_eq!(r.0, EXIT_FAILURE);
        assert_eq!(r.1, None);
    }
    let _g = lock();
    let (_, ec) = with_captured_stderr("e17c", || {
        driver_case(c_api(), 2, 2, "1 2\n3 4", 3, 1, "1 2 3")
    });
    assert_eq!(
        String::from_utf8_lossy(&ec),
        "Matrix dimensions do not allow multiplication.\n"
    );
}

// ===========================================================================
// E18 — driver: matrix_to_string's malloc fails (res_str == NULL)
// ===========================================================================
#[test]
fn e18_driver_to_string_oom() {
    // mat_a: 1 wide x N high, mat_b: N wide x 1 high  =>  res is N x N.
    //
    // Allocation budget for one `driver` call with N = 250:
    //   strdup(matrix_a) + strdup(matrix_b)         ~=   1 KiB
    //   mat_a = allocate_matrix(1, 250)             ~=  10 KiB
    //   mat_b = allocate_matrix(250, 1)             ~=   1 KiB
    //   res   = allocate_matrix(250, 250)           ~= 254 KiB
    //   ---------------------------------------------------------
    //   subtotal before matrix_to_string            ~= 266 KiB
    //   matrix_to_string buffer:
    //       buffer_size = 250*(250*10 + 250) + 251  =  687_751 B  (~672 KiB)
    //
    // Giving the heap a budget of 512 KiB therefore lets everything up to and
    // including `res` succeed while making the 672 KiB buffer impossible — which
    // is exactly the `res_str == NULL` branch of driver.c:53.
    //
    // N is kept small on purpose: the C `matrix_to_string` appends with
    // `strcat`, rescanning the whole buffer each time, so it is quadratic in the
    // output size and a large N would make the *success* path take minutes.
    const N: usize = 250;
    const HEAP_BUDGET: usize = 512 << 10;

    if let Some(cfg) = child_cfg() {
        let api = cfg.api();
        let text_a = (0..N).map(|_| "3").collect::<Vec<_>>().join("\n");
        let text_b = (0..N).map(|_| "4").collect::<Vec<_>>().join(" ");
        let ca = cstr(&text_a);
        let cb = cstr(&text_b);
        let fd = cfg.open_out();
        cfg.apply_limit();
        unsafe {
            let injected = constrain_heap_to(HEAP_BUDGET);
            let rc = (api.driver)(1, N as c_int, ca.as_ptr(), N as c_int, 1, cb.as_ptr());
            report(
                fd,
                match (injected, rc == EXIT_FAILURE) {
                    (true, true) => b"FAIL",
                    (true, false) => b"OK__",
                    (false, _) => b"CAP_",
                },
            );
        }
        cfg.exit_now(fd);
    }

    // A 4 MiB address-space cap only bounds how long `exhaust_heap` runs; the
    // precise budget comes from `constrain_heap_to`.
    let (err, pay) = compare_self_children("e18", "e18_driver_to_string_oom", Some(4 << 20));
    assert_eq!(
        pay,
        b"FAIL",
        "driver must return EXIT_FAILURE when matrix_to_string OOMs; stderr={:?}",
        show(&err)
    );
    assert!(
        String::from_utf8_lossy(&err).contains("Failed to allocate memory for matrix string"),
        "unexpected stderr: {:?}",
        show(&err)
    );
    // The success path would have written ./matrix.txt; make sure it did not.
    let _ = std::fs::remove_file(OUT_FILE);
}

// ===========================================================================
// E19 — driver: write_to_file fails (matrix.txt is a directory)
// ===========================================================================
#[test]
fn e19_driver_write_fails() {
    let _g = lock();
    let _ = std::fs::remove_file(OUT_FILE);
    let _ = std::fs::remove_dir_all(OUT_FILE);
    std::fs::create_dir(OUT_FILE).expect("create matrix.txt as a directory");

    let run = |api: &Api| {
        let ca = cstr("1 2\n3 4");
        let cb = cstr("5 6\n7 8");
        unsafe { (api.driver)(2, 2, ca.as_ptr(), 2, 2, cb.as_ptr()) }
    };
    let (rc_c, err_c) = with_captured_stderr("e19c", || run(c_api()));
    let (rc_r, err_r) = with_captured_stderr("e19r", || run(rust_api()));

    let _ = std::fs::remove_dir_all(OUT_FILE);

    assert_eq!(show(&err_c), show(&err_r), "stderr differs");
    assert_eq!(rc_c, rc_r, "return value differs");
    assert_eq!(rc_c, EXIT_FAILURE);
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        "Error opening file 'matrix.txt': Is a directory\n"
    );
}

// ===========================================================================
// G1 — initialize_matrix_from_string(NULL, ..) faults identically
// ===========================================================================
#[test]
fn g1_init_null_input_faults_both() {
    for (w, h) in [(1, 1), (2, 2), (0, 0), (0, 1), (1, 0)] {
        let (exit, _) = run_pair_child_ub(&format!("g1-{w}x{h}"), move |api, fd| unsafe {
            let m = (api.initialize_matrix_from_string)(ptr::null(), w, h);
            report(fd, if m.is_null() { b"NULL" } else { b"PTR" });
        });
        // Both sides agree (checked inside). `strdup(NULL)` faults inside libc,
        // so this is a SIGSEGV for both the C and the Rust build.
        match exit {
            Exit::Signal(s) => assert!(
                s == libc::SIGSEGV || s == libc::SIGBUS,
                "unexpected signal {s}"
            ),
            Exit::Code(0) => {}
            other => panic!("unexpected child exit {other:?}"),
        }
    }
}

// ===========================================================================
// G2 — multiply_matrices with NULL operands faults identically
// ===========================================================================
#[test]
fn g2_multiply_null_faults_both() {
    // (a_null, b_null)
    for (an, bn) in [(true, false), (false, true), (true, true)] {
        let (exit, _) = run_pair_child_ub(&format!("g2-{an}-{bn}"), move |api, fd| unsafe {
            let a = if an {
                ptr::null_mut()
            } else {
                (api.allocate_matrix)(2, 2)
            };
            let b = if bn {
                ptr::null_mut()
            } else {
                (api.allocate_matrix)(2, 2)
            };
            let r = (api.multiply_matrices)(a, b);
            report(fd, if r.is_null() { b"NULL" } else { b"PTR" });
        });
        assert!(
            matches!(exit, Exit::Signal(_)),
            "multiply_matrices(NULL, ..) must fault, got {exit:?}"
        );
    }
    // matrix_to_string on a struct whose row array is NULL but whose dims are
    // positive: dereferences NULL in both builds.
    let (exit, _) = run_pair_child_ub("g2-tostring-nullrows", |api, fd| unsafe {
        let mut m = MatrixT {
            matrix: ptr::null_mut(),
            width: 2,
            height: 2,
        };
        let s = (api.matrix_to_string)(&mut m);
        report(fd, if s.is_null() { b"NULL" } else { b"PTR" });
    });
    assert!(
        matches!(exit, Exit::Signal(_)),
        "matrix_to_string with NULL rows must fault, got {exit:?}"
    );
    // driver() whose result matrix cannot be allocated: multiply_matrices then
    // writes through a NULL result. width_b < 0 with height_b == 0 parses fine.
    let (exit, _) = run_pair_child_ub("g2-driver-null-result", |api, fd| unsafe {
        let a = cstr("1");
        let b = cstr("");
        let rc = (api.driver)(0, 1, a.as_ptr(), -1, 0, b.as_ptr());
        let d = [b'0' + (rc as u8 & 0x0f)];
        report(fd, &d);
    });
    match exit {
        Exit::Signal(_) | Exit::Code(_) => {}
        other => panic!("unexpected child exit {other:?}"),
    }
}

// ===========================================================================
// G3 — zero-sized dimensions are valid in C
// ===========================================================================
#[test]
fn g3_zero_dimensions() {
    for (w, h) in [(0, 0), (0, 1), (1, 0), (0, 7), (7, 0)] {
        run_pair(&format!("g3-{w}x{h}"), |api| unsafe {
            let m = (api.allocate_matrix)(w, h);
            let shape = observe_shape(m);
            let s = observe_and_free_cstring((api.matrix_to_string)(m));
            (api.free_matrix)(m);
            (shape, s)
        });
    }
}

// ===========================================================================
// G4 — dimension boundary sweep (one step past the valid range)
// ===========================================================================
#[test]
fn g4_dimension_boundaries() {
    let small: [c_int; 7] = [-2, -1, 0, 1, 2, 3, 16];
    for &w in &small {
        for &h in &small {
            run_pair(&format!("g4-small-{w}x{h}"), |api| unsafe {
                let m = (api.allocate_matrix)(w, h);
                let shape = observe_shape(m);
                (api.free_matrix)(m);
                shape
            });
            // matrix_to_string never dereferences the row array when its malloc
            // fails, which is the case whenever buffer_size <= 0.
            if buffer_size(w, h) <= 0 {
                run_pair(&format!("g4-str-{w}x{h}"), |api| unsafe {
                    let mut m = MatrixT {
                        matrix: ptr::null_mut(),
                        width: w,
                        height: h,
                    };
                    observe_and_free_cstring((api.matrix_to_string)(&mut m))
                });
            }
        }
    }

    // Extreme magnitudes: run under a capped address space so the huge
    // allocations fail promptly and deterministically on both sides.
    let extremes: [(c_int, c_int); 12] = [
        (i32::MIN, 1),
        (i32::MIN + 1, 1),
        (1, i32::MIN),
        (1, i32::MIN + 1),
        (i32::MAX, 1),
        (i32::MAX - 1, 1),
        (1, i32::MAX),
        (1, i32::MAX - 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
    ];
    for (w, h) in extremes {
        let (exit, _e, _p) =
            run_pair_child(&format!("g4-ext-{w}x{h}"), Some(64 << 20), move |api, fd| unsafe {
                let m = (api.allocate_matrix)(w, h);
                report(fd, if m.is_null() { b"NULL" } else { b"PTR" });
                (api.free_matrix)(m);
            });
        assert_eq!(exit, Exit::Code(0), "child for ({w},{h}) did not exit cleanly");
    }
}

// ===========================================================================
// G5 — randomized dimension fuzz (unconstrained int inputs)
// ===========================================================================
#[test]
fn g5_random_dimension_fuzz() {
    let mut rng = Rng::new(105);
    // in-process: at least one dimension negative, or both small
    for iter in 0..400u64 {
        let (w, h) = if iter % 2 == 0 {
            (rng.i32_full() | i32::MIN, rng.range(0, 32) as c_int)
        } else {
            (rng.range(0, 32) as c_int, rng.i32_full() | i32::MIN)
        };
        run_pair(&format!("g5-{iter}-{w}x{h}"), |api| unsafe {
            let m = (api.allocate_matrix)(w, h);
            let shape = observe_shape(m);
            (api.free_matrix)(m);
            shape
        });
    }
    // in a capped child: fully random widths, bounded heights
    let mut dims: Vec<(c_int, c_int)> = Vec::new();
    for _ in 0..64 {
        let w = rng.i32_full();
        let h = (rng.i32_full() % 100_000) as c_int;
        dims.push((w, h));
    }
    let dims_ptr = dims.as_ptr() as usize;
    let n = dims.len();
    let (exit, _e, _p) = run_pair_child("g5-child", Some(64 << 20), move |api, fd| unsafe {
        let d = std::slice::from_raw_parts(dims_ptr as *const (c_int, c_int), n);
        let mut buf = [0u8; 64];
        let mut k = 0usize;
        for &(w, h) in d {
            let m = (api.allocate_matrix)(w, h);
            buf[k] = if m.is_null() { b'0' } else { b'1' };
            (api.free_matrix)(m);
            k += 1;
        }
        report(fd, &buf[..k]);
    });
    assert_eq!(exit, Exit::Code(0), "g5 child did not exit cleanly");
}

// ===========================================================================
// G6 — oversized content for write_to_file
// ===========================================================================
#[test]
fn g6_write_oversized_content() {
    let path = scratch("g6.txt").to_str().unwrap().to_string();
    for n in [1_048_576usize, 2_000_003] {
        let content: String = std::iter::repeat('Q').take(n).collect();
        let r = run_pair(&format!("g6-{n}"), |api| {
            let _ = std::fs::remove_file(&path);
            let p = cstr(&path);
            let c = cstr(&content);
            let rc = unsafe { (api.write_to_file)(p.as_ptr(), c.as_ptr()) };
            (rc, std::fs::metadata(&path).map(|m| m.len()).ok())
        });
        assert_eq!(r.0, 0);
        assert_eq!(r.1, Some(n as u64));
    }
    let _ = std::fs::remove_file(&path);
}

// ===========================================================================
// G7 — zero-length content
// ===========================================================================
#[test]
fn g7_write_empty_content() {
    let path = scratch("g7.txt").to_str().unwrap().to_string();
    let r = run_pair("g7", |api| {
        let _ = std::fs::remove_file(&path);
        let p = cstr(&path);
        let c = cstr("");
        let rc = unsafe { (api.write_to_file)(p.as_ptr(), c.as_ptr()) };
        (rc, std::fs::read(&path).ok())
    });
    assert_eq!(r, (EXIT_SUCCESS, Some(Vec::new())));
    let _ = std::fs::remove_file(&path);
}

// ===========================================================================
// G8 — atoi token forms
// ===========================================================================
#[test]
fn g8_atoi_token_forms() {
    let toks = [
        "abc",
        "12abc",
        "+7",
        "  9",
        "0x10",
        "99999999999999999999",
        "-2147483649",
        "2147483648",
        "-0",
        "007",
        "1e3",
        "-",
        "+-2",
        "0",
        "-2147483648",
        "2147483647",
        "18446744073709551616",
        "9223372036854775808",
        "-9223372036854775809",
    ];
    for t in toks {
        let r = run_pair(&format!("g8-{}", show(t.as_bytes())), |api| {
            init_observe(api, t, 1, 1)
        });
        // both sides must agree; the concrete value is whatever glibc atoi does
        match r {
            MatObs::Some { cells: Some(_), .. } => {}
            other => panic!("unexpected observation for {t:?}: {other:?}"),
        }
    }
}
