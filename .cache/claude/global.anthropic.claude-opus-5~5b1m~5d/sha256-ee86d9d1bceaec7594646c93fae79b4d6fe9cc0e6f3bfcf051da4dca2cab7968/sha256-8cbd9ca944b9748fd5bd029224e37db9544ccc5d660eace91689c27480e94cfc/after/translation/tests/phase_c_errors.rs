//! Phase C — error-path differential tests, one test per row of `ERRORS.md`
//! plus the generic FFI-boundary cases G1..G5.
//!
//! Every test asserts the two implementations agree on the *specific* rejection
//! (identical return value / sentinel **and** identical `stderr` bytes), not
//! merely that "both failed".

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::ptr;

// ---------------------------------------------------------------------------
// errno constants (Linux / glibc)
// ---------------------------------------------------------------------------

const ENOENT: c_int = 2;
const EBADF: c_int = 9;
const EACCES: c_int = 13;
const EFAULT: c_int = 14;
const EISDIR: c_int = 21;
const EINVAL: c_int = 22;
const ENOSPC: c_int = 28;
const ENAMETOOLONG: c_int = 36;
const EXIT_FAILURE: c_int = 1;
const EXIT_SUCCESS: c_int = 0;

// ---------------------------------------------------------------------------
// generic differential drivers
// ---------------------------------------------------------------------------

/// `allocate_matrix` on both libs: compare NULL-ness, dims and stderr.
fn diff_allocate(tag: &str, w: c_int, h: c_int) -> bool {
    let _g = global_lock();
    let (c, r) = both();
    let run = |api: &Api, t: &str| {
        capture_stderr(t, || unsafe {
            let m = (api.allocate_matrix)(w, h);
            let d = if m.is_null() {
                None
            } else {
                Some(((*m).width, (*m).height))
            };
            (api.free_matrix)(m);
            d
        })
    };
    let (cd, ce) = run(c, &format!("{tag}_c"));
    let (rd, re) = run(r, &format!("{tag}_r"));
    assert_eq!(cd, rd, "{tag}: allocate_matrix({w},{h}) NULL-ness/dims");
    assert_eq_stderr(&format!("{tag}: allocate_matrix({w},{h})"), &ce, &re);
    cd.is_none()
}

/// `initialize_matrix_from_string` on both libs; returns whether C returned NULL.
fn diff_init(tag: &str, input: &str, w: c_int, h: c_int) -> bool {
    let _g = global_lock();
    let (c, r) = both();
    let s = cs(input);
    let run = |api: &Api, t: &str| {
        capture_stderr(t, || unsafe {
            let m = (api.initialize_matrix_from_string)(s.as_ptr(), w, h);
            let d = if m.is_null() {
                None
            } else {
                Some(((*m).width, (*m).height))
            };
            (api.free_matrix)(m);
            d
        })
    };
    let (cd, ce) = run(c, &format!("{tag}_c"));
    let (rd, re) = run(r, &format!("{tag}_r"));
    let ctx = format!("{tag}: init({input:?},{w},{h})");
    assert_eq!(cd, rd, "{ctx}: NULL-ness/dims");
    assert_eq_stderr(&ctx, &ce, &re);
    cd.is_none()
}

/// `write_to_file` on both libs; returns the (identical) return code.
fn diff_write(tag: &str, filename: Option<&[u8]>, content: Option<&[u8]>) -> c_int {
    let _g = global_lock();
    let (c, r) = both();
    let fname = filename.map(cs_bytes);
    let cont = content.map(cs_bytes);
    let fp = fname.as_ref().map_or(ptr::null(), |s| s.as_ptr());
    let cp = cont.as_ref().map_or(ptr::null(), |s| s.as_ptr());

    let (crc, ce) = capture_stderr(&format!("{tag}_c"), || unsafe {
        (c.write_to_file)(fp, cp)
    });
    let (rrc, re) = capture_stderr(&format!("{tag}_r"), || unsafe {
        (r.write_to_file)(fp, cp)
    });
    assert_eq!(crc, rrc, "{tag}: write_to_file return code");
    assert_eq_stderr(&format!("{tag}: write_to_file"), &ce, &re);
    crc
}

/// `driver` on both libs inside a fresh scratch CWD; returns the return code.
#[allow(clippy::too_many_arguments)]
fn diff_driver(
    tag: &str,
    wa: c_int,
    ha: c_int,
    ma: &str,
    wb: c_int,
    hb: c_int,
    mb: &str,
    prepare: impl Fn(&std::path::Path),
) -> c_int {
    let _g = global_lock();
    let (c, r) = both();
    let dir = scratch_dir(tag);
    prepare(&dir);
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();

    let sa = cs(ma);
    let sb = cs(mb);
    let (crc, ce) = capture_stderr(&format!("{tag}_c"), || unsafe {
        (c.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr())
    });
    let cout = std::fs::read("matrix.txt").ok();
    let _ = std::fs::remove_file("matrix.txt");
    prepare(&dir);
    let (rrc, re) = capture_stderr(&format!("{tag}_r"), || unsafe {
        (r.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr())
    });
    let rout = std::fs::read("matrix.txt").ok();

    std::env::set_current_dir(prev).unwrap();

    assert_eq!(crc, rrc, "{tag}: driver return code");
    assert_eq_stderr(&format!("{tag}: driver"), &ce, &re);
    assert!(cout == rout, "{tag}: matrix.txt differs");
    crc
}

// ---------------------------------------------------------------------------
// out-of-process payloads (for the branches the C reaches by crashing, and for
// the one that needs ~850 MB so the leak is reclaimed by process exit)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stderr: Vec<u8>,
}

fn run_payload(which: &str, name: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "oop_payload", "--nocapture", "--test-threads=1"])
        .env("DIFFTEST_PAYLOAD", format!("{which}:{name}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn payload");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stderr: out.stderr,
    }
}

fn diff_payload(name: &str) -> Outcome {
    let _g = global_lock();
    let c = run_payload("c", name);
    let r = run_payload("rust", name);
    assert_eq!(
        c.code, r.code,
        "payload {name}: exit code differs (C {:?} vs Rust {:?}); stderr C={} Rust={}",
        c.code,
        r.code,
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.signal, r.signal,
        "payload {name}: termination signal differs (C {:?} vs Rust {:?})",
        c.signal, r.signal
    );
    assert_eq_stderr(&format!("payload {name}"), &c.stderr, &r.stderr);
    c
}

/// The child-process entry point. A no-op unless `DIFFTEST_PAYLOAD` is set.
#[test]
fn oop_payload() {
    let Ok(spec) = std::env::var("DIFFTEST_PAYLOAD") else {
        return;
    };
    let (which, name) = spec.split_once(':').expect("payload spec");
    // Same address-space cap as the parent, so the huge `malloc`s fail
    // deterministically instead of relying on overcommit.
    limit_address_space();
    let api = if which == "c" { c_api() } else { rust_api() };

    unsafe {
        match name {
            // ERRORS.md #5 — `input == NULL` is never checked; strdup(NULL) faults.
            "init_null_input" => {
                let m = (api.initialize_matrix_from_string)(ptr::null(), 1, 1);
                eprint!("UNREACHABLE m_null={}\n", m.is_null());
            }
            // ERRORS.md #10 — multiply_matrices dereferences without a NULL check.
            "multiply_null_a" => {
                let b = (api.allocate_matrix)(1, 1);
                let res = (api.multiply_matrices)(ptr::null_mut(), b);
                eprint!("UNREACHABLE res_null={}\n", res.is_null());
            }
            "multiply_null_b" => {
                let a = (api.allocate_matrix)(1, 1);
                let res = (api.multiply_matrices)(a, ptr::null_mut());
                eprint!("UNREACHABLE res_null={}\n", res.is_null());
            }
            "multiply_null_both" => {
                let res = (api.multiply_matrices)(ptr::null_mut(), ptr::null_mut());
                eprint!("UNREACHABLE res_null={}\n", res.is_null());
            }
            // ERRORS.md #21 — driver's `matrix_to_string` returns NULL because the
            // int `buffer_size` computation wraps negative. Needs ~850 MB, so it
            // is run in a child process.
            "driver_to_string_fail" => {
                let a = cs("x");
                let b = cs("x");
                let rc = (api.driver)(0, 1, a.as_ptr(), 214_748_365, 0, b.as_ptr());
                eprint!("RESULT={rc}\n");
            }
            // ERRORS.md #8 (deref variant) — `initialize_matrix_from_string`
            // does not NULL-check `allocate_matrix`, so when a huge dimension
            // makes the allocation fail the very next cell store faults.
            "init_huge_width" => {
                let s = cs("1 2\n3 4");
                let m = (api.initialize_matrix_from_string)(s.as_ptr(), i32::MAX, 2);
                eprint!("UNREACHABLE m_null={}\n", m.is_null());
            }
            "init_huge_height" => {
                let s = cs("1 2\n3 4");
                let m = (api.initialize_matrix_from_string)(s.as_ptr(), 2, i32::MAX);
                eprint!("UNREACHABLE m_null={}\n", m.is_null());
            }
            "init_alloc_fail_deref" => {
                let s = cs("1");
                let m = (api.initialize_matrix_from_string)(s.as_ptr(), 1, i32::MAX / 4);
                eprint!("UNREACHABLE m_null={}\n", m.is_null());
            }
            // ERRORS.md #24 — free_matrix never checks `mat->matrix`.
            "free_null_rows_h1" | "free_null_rows_h5" => {
                let h = if name.ends_with("h1") { 1 } else { 5 };
                let m = libc_malloc(std::mem::size_of::<MatrixT>()) as *mut MatrixT;
                (*m).matrix = ptr::null_mut();
                (*m).width = 3;
                (*m).height = h;
                (api.free_matrix)(m);
                eprint!("UNREACHABLE freed\n");
            }
            other => panic!("unknown payload {other}"),
        }
    }

    use std::io::Write;
    std::io::stderr().flush().ok();
    std::process::exit(0);
}

// ===========================================================================
// ERRORS.md #2 — allocate_matrix: malloc(height * sizeof(int*)) fails
// ===========================================================================

#[test]
fn err_02_allocate_negative_height() {
    for (w, h) in [
        (1, -1),
        (0, -1),
        (5, -5),
        (1, i32::MIN),
        (i32::MAX, -1),
        (-1, -1),
        (1, -(1 << 20)),
    ] {
        assert!(
            diff_allocate("err02", w, h),
            "err02: allocate_matrix({w},{h}) should fail on the row array"
        );
    }
    // huge positive height: height*8 exceeds the address space
    for (w, h) in [(1, i32::MAX), (1, i32::MAX - 1), (1, 1 << 30)] {
        assert!(
            diff_allocate("err02b", w, h),
            "err02: allocate_matrix({w},{h}) should fail on the row array"
        );
    }
}

// ===========================================================================
// ERRORS.md #3 — allocate_matrix: malloc(width * sizeof(int)) fails per row
// ===========================================================================

#[test]
fn err_03_allocate_negative_width() {
    for (w, h) in [
        (-1, 1),
        (-1, 4),
        (i32::MIN, 1),
        (i32::MIN, 3),
        (-(1 << 20), 2),
        (i32::MAX, 2),
        (1 << 30, 4),
    ] {
        assert!(
            diff_allocate("err03", w, h),
            "err03: allocate_matrix({w},{h}) should fail on a row"
        );
    }
    // `height == 0` short-circuits the row loop, so a negative width SUCCEEDS
    // (no row is ever allocated). Both must agree on that too.
    for w in [-1, i32::MIN, -(1 << 20)] {
        assert!(
            !diff_allocate("err03b", w, 0),
            "err03: allocate_matrix({w},0) must succeed (row loop never runs)"
        );
    }
}

// ===========================================================================
// ERRORS.md #4 — free_matrix(NULL) is a silent no-op
// ===========================================================================

#[test]
fn err_04_free_matrix_null() {
    let _g = global_lock();
    let (c, r) = both();
    let ((), ce) = capture_stderr("err04_c", || unsafe {
        (c.free_matrix)(ptr::null_mut());
        (c.free_matrix)(ptr::null_mut());
    });
    let ((), re) = capture_stderr("err04_r", || unsafe {
        (r.free_matrix)(ptr::null_mut());
        (r.free_matrix)(ptr::null_mut());
    });
    assert!(ce.is_empty(), "err04: C printed {}", show(&ce));
    assert_eq_stderr("err04: free_matrix(NULL)", &ce, &re);
}

// ===========================================================================
// ERRORS.md #24 — free_matrix on a heap `matrix_t` whose `matrix` field is NULL
//
// `free_matrix` only NULL-checks `mat`, never `mat->matrix`. With
// `height <= 0` the row loop never runs, so `free(NULL)` + `free(mat)` is a
// silent no-op; with `height > 0` the C dereferences NULL. Both cases are a
// legitimate FFI input (the caller owns the struct) and must match.
// ===========================================================================

/// Allocate a `matrix_t` with libc `malloc` so `free_matrix` may legally
/// `free()` it, with the given field values.
unsafe fn heap_matrix(rows: *mut *mut c_int, width: c_int, height: c_int) -> *mut MatrixT {
    let m = libc_malloc(std::mem::size_of::<MatrixT>()) as *mut MatrixT;
    (*m).matrix = rows;
    (*m).width = width;
    (*m).height = height;
    m
}

#[test]
fn err_24_free_matrix_null_rows_nonpositive_height() {
    let _g = global_lock();
    let (c, r) = both();
    for (w, h) in [(0, 0), (5, 0), (-1, 0), (5, -1), (0, i32::MIN), (7, -9)] {
        let ((), ce) = capture_stderr("err24_c", || unsafe {
            (c.free_matrix)(heap_matrix(ptr::null_mut(), w, h));
        });
        let ((), re) = capture_stderr("err24_r", || unsafe {
            (r.free_matrix)(heap_matrix(ptr::null_mut(), w, h));
        });
        let ctx = format!("err24: free_matrix({{matrix:NULL,width:{w},height:{h}}})");
        assert!(ce.is_empty(), "{ctx}: C printed {}", show(&ce));
        assert_eq_stderr(&ctx, &ce, &re);
    }
}

#[test]
fn err_24b_free_matrix_null_rows_positive_height() {
    // height > 0 -> `free(mat->matrix[0])` dereferences NULL in both builds.
    for name in ["free_null_rows_h1", "free_null_rows_h5"] {
        let o = diff_payload(name);
        assert!(
            o.signal.is_some(),
            "err24b/{name}: expected a fatal signal, got {o:?}"
        );
    }
}

// ===========================================================================
// ERRORS.md #5 — initialize_matrix_from_string(NULL, ..) faults in both builds
// ===========================================================================

#[test]
fn err_05_null_input_crashes_both() {
    let o = diff_payload("init_null_input");
    assert!(
        o.signal.is_some(),
        "err05: expected a fatal signal, got {o:?}"
    );
    assert!(
        !String::from_utf8_lossy(&o.stderr).contains("UNREACHABLE"),
        "err05: the call unexpectedly returned"
    );
}

// ===========================================================================
// ERRORS.md #6 — insufficient rows
// ===========================================================================

#[test]
fn err_06_insufficient_rows() {
    for (input, w, h) in [
        ("", 1, 1),
        ("\n", 1, 1),
        ("\n\n\n", 1, 2),
        ("1 2", 2, 2),
        ("1 2\n3 4", 2, 3),
        ("1 2\n3 4\n", 2, 3),
        ("", 0, 1),
        ("   ", 0, 2),
        ("a\nb", 0, 3),
        ("1", 1, 1_000),
        ("1", 1, 1_000_000),
    ] {
        assert!(
            diff_init("err06", input, w, h),
            "err06: init({input:?},{w},{h}) should report insufficient rows"
        );
    }
    // and the message text itself is compared by diff_init; assert C emits it
    let _g = global_lock();
    let (c, _r) = both();
    let s = cs("");
    let (_m, ce) = capture_stderr("err06_msg", || unsafe {
        let m = (c.initialize_matrix_from_string)(s.as_ptr(), 1, 1);
        (c.free_matrix)(m);
    });
    assert_eq!(ce, b"Insufficient rows in input string.\n");
}

// ===========================================================================
// ERRORS.md #7 — insufficient columns, 1-based row number in the message
// ===========================================================================

#[test]
fn err_07_insufficient_columns() {
    for (input, w, h) in [
        ("1", 2, 1),
        ("1 2\n3", 2, 2),
        ("1 2\n3 4\n5", 2, 3),
        (" ", 1, 1),
        ("   \n   ", 1, 2),
        ("1 2 3\n4 5", 3, 2),
        ("1", 1_000, 1),
    ] {
        assert!(
            diff_init("err07", input, w, h),
            "err07: init({input:?},{w},{h}) should report insufficient columns"
        );
    }
    // the "%d" is `i + 1`, i.e. 1-based; check the exact bytes for row 3
    let _g = global_lock();
    let (c, _r) = both();
    let s = cs("1 2\n3 4\n5");
    let (_m, ce) = capture_stderr("err07_msg", || unsafe {
        let m = (c.initialize_matrix_from_string)(s.as_ptr(), 2, 3);
        (c.free_matrix)(m);
    });
    assert_eq!(ce, b"Insufficient columns in row 3.\n");
}

// ===========================================================================
// ERRORS.md #8 — the unchecked `allocate_matrix` failure leaks out as a NULL
// "success" return
// ===========================================================================

#[test]
fn err_08_init_returns_null_from_failed_alloc() {
    // negative height: allocate_matrix fails, the row loop never runs, and the
    // NULL `mat` is returned without any "insufficient rows" diagnostic.
    for (input, w, h) in [("1 2\n3 4", 2, -1), ("", 1, -1), ("x", 0, i32::MIN)] {
        assert!(
            diff_init("err08", input, w, h),
            "err08: init({input:?},{w},{h}) must return NULL"
        );
    }
    // negative width with enough rows: allocate_matrix fails, the column loop
    // never runs, and NULL is returned after consuming `height` row tokens.
    for (input, w, h) in [("a\nb", -1, 2), ("a", -1, 1), ("a\nb\nc", i32::MIN, 3)] {
        assert!(
            diff_init("err08b", input, w, h),
            "err08: init({input:?},{w},{h}) must return NULL"
        );
    }
    // Exact byte check: only the allocate_matrix perror, no other message.
    let _g = global_lock();
    let (c, _r) = both();
    let s = cs("1 2\n3 4");
    let (_m, ce) = capture_stderr("err08_msg", || unsafe {
        let m = (c.initialize_matrix_from_string)(s.as_ptr(), 2, -1);
        (c.free_matrix)(m);
    });
    assert_eq!(
        ce,
        b"Failed to allocate memory for matrix rows: Cannot allocate memory\n"
    );
}

// ===========================================================================
// ERRORS.md #9 — multiply_matrices dimension mismatch
// ===========================================================================

#[test]
fn err_09_dimension_mismatch() {
    let _g = global_lock();
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 9);

    let mut cases: Vec<(c_int, c_int, c_int, c_int)> =
        vec![(1, 1, 2, 2), (2, 3, 4, 5), (1, 1, 1, 2), (0, 1, 1, 1), (3, 1, 1, 2)];
    for _ in 0..80 {
        let wa = rng.range_i32(0, 8);
        let ha = rng.range_i32(0, 8);
        let wb = rng.range_i32(0, 8);
        let mut hb = rng.range_i32(0, 8);
        if hb == wa {
            hb = (hb + 1) % 9;
        }
        cases.push((wa, ha, wb, hb));
    }

    for (wa, ha, wb, hb) in cases {
        assert_ne!(wa, hb, "case must be incompatible");
        let run = |api: &Api, tag: &str| {
            capture_stderr(tag, || unsafe {
                let a = (api.allocate_matrix)(wa, ha);
                let b = (api.allocate_matrix)(wb, hb);
                let res = (api.multiply_matrices)(a, b);
                let null = res.is_null();
                (api.free_matrix)(res);
                (api.free_matrix)(b);
                (api.free_matrix)(a);
                null
            })
        };
        let (cn, ce) = run(c, "err09_c");
        let (rn, re) = run(r, "err09_r");
        let ctx = format!("err09: mul(({wa}x{ha}),({wb}x{hb}))");
        assert_eq!(cn, rn, "{ctx}: NULL-ness");
        assert!(cn, "{ctx}: C should have returned NULL");
        assert_eq_stderr(&ctx, &ce, &re);
        assert_eq!(
            ce, b"Matrix dimensions do not allow multiplication.\n",
            "{ctx}: unexpected C message {}",
            show(&ce)
        );
    }
}

// ===========================================================================
// ERRORS.md #10 — multiply_matrices with NULL operands faults in both builds
// ===========================================================================

#[test]
fn err_10_multiply_null_crashes_both() {
    for name in ["multiply_null_a", "multiply_null_both"] {
        let o = diff_payload(name);
        assert!(o.signal.is_some(), "err10/{name}: expected a signal, got {o:?}");
    }
    // `mat_b == NULL` is only dereferenced *after* `mat_a->width`, so it too
    // faults — but assert only that C and Rust agree, whatever that is.
    let o = diff_payload("multiply_null_b");
    assert!(
        o.signal.is_some() || o.code == Some(0),
        "err10/multiply_null_b: unexpected outcome {o:?}"
    );
}

// ===========================================================================
// ERRORS.md #11 — matrix_to_string(NULL)
// ===========================================================================

#[test]
fn err_11_to_string_null() {
    let _g = global_lock();
    let (c, r) = both();
    let (cs_, ce) = capture_stderr("err11_c", || unsafe {
        take_cstring((c.matrix_to_string)(ptr::null_mut()))
    });
    let (rs_, re) = capture_stderr("err11_r", || unsafe {
        take_cstring((r.matrix_to_string)(ptr::null_mut()))
    });
    assert_eq!(cs_, None, "err11: C should return NULL");
    assert_eq_bytes("err11: matrix_to_string(NULL)", &cs_, &rs_);
    assert_eq_stderr("err11: matrix_to_string(NULL)", &ce, &re);
    assert_eq!(ce, b"Error: Matrix is NULL.\n");
}

// ===========================================================================
// ERRORS.md #12 — matrix_to_string: the int `buffer_size` wraps negative, so
// malloc gets a colossal size_t and fails
// ===========================================================================

/// Reproduce the C expression `height * (width * 10 + width) + height + 1`
/// with `int` wrap-around.
fn c_buffer_size(width: i32, height: i32) -> i32 {
    height
        .wrapping_mul(width.wrapping_mul(10).wrapping_add(width))
        .wrapping_add(height)
        .wrapping_add(1)
}

#[test]
fn err_12_to_string_buffer_size_overflow() {
    let _g = global_lock();
    let (c, r) = both();

    // Candidate dimensions whose buffer_size is negative. `matrix_to_string`
    // returns before touching any cell, so a 1x1 backing store is enough — this
    // is a perfectly legal `matrix_t` as far as the C ABI is concerned.
    let candidates: [(i32, i32); 7] = [
        (214_748_365, 1),
        (214_748_366, 1),
        (300_000_000, 1),
        (100_000_000, 3),
        (20_000, 10_000),
        (1 << 28, 1),
        (999_999_999, 1),
    ];

    for (w, h) in candidates {
        let bs = c_buffer_size(w, h);
        assert!(
            bs < 0,
            "err12: dims ({w},{h}) do not wrap buffer_size negative (got {bs})"
        );

        unsafe {
            let cell = libc_malloc(4) as *mut c_int;
            *cell = 0;
            let rowv = libc_malloc(std::mem::size_of::<*mut c_int>()) as *mut *mut c_int;
            *rowv = cell;
            let mut m = MatrixT {
                matrix: rowv,
                width: w,
                height: h,
            };
            let mp: *mut MatrixT = &mut m;

            let (cres, ce) = capture_stderr("err12_c", || take_cstring((c.matrix_to_string)(mp)));
            let (rres, re) = capture_stderr("err12_r", || take_cstring((r.matrix_to_string)(mp)));

            libc_free(rowv as *mut c_void);
            libc_free(cell as *mut c_void);

            let ctx = format!("err12: matrix_to_string(width={w},height={h}) bs={bs}");
            assert_eq!(cres, None, "{ctx}: C should return NULL");
            assert_eq_bytes(&ctx, &cres, &rres);
            assert_eq_stderr(&ctx, &ce, &re);
            assert_eq!(
                ce, b"Failed to allocate memory for matrix string: Cannot allocate memory\n",
                "{ctx}: unexpected message {}",
                show(&ce)
            );
        }
    }
}

// ===========================================================================
// ERRORS.md #13 — write_to_file with NULL content returns EINVAL
// ===========================================================================

#[test]
fn err_13_write_null_content() {
    let dir = scratch_dir("err13");
    let name = dir.join("never-created");
    let rc = diff_write("err13", Some(name.to_str().unwrap().as_bytes()), None);
    assert_eq!(rc, EINVAL, "err13: expected EINVAL");
    assert!(!name.exists(), "err13: no file should be created");

    // NULL content wins over a bad filename (the check comes first)
    assert_eq!(diff_write("err13b", Some(b"/no/such/dir/x"), None), EINVAL);
    // ...and over a NULL filename
    assert_eq!(diff_write("err13c", None, None), EINVAL);
}

// ===========================================================================
// ERRORS.md #14 — fopen failures
// ===========================================================================

#[test]
fn err_14_write_fopen_failures() {
    let dir = scratch_dir("err14");

    // nonexistent directory component -> ENOENT
    let p = dir.join("nope/x");
    assert_eq!(
        diff_write("err14_enoent", Some(p.to_str().unwrap().as_bytes()), Some(b"x")),
        ENOENT
    );
    // empty filename -> ENOENT
    assert_eq!(diff_write("err14_empty", Some(b""), Some(b"x")), ENOENT);
    // absolute nonexistent root path -> ENOENT
    assert_eq!(
        diff_write("err14_root", Some(b"/definitely-not-here-42/x"), Some(b"x")),
        ENOENT
    );

    // the filename IS a directory -> EISDIR
    let d = dir.join("adir");
    std::fs::create_dir_all(&d).unwrap();
    assert_eq!(
        diff_write("err14_eisdir", Some(d.to_str().unwrap().as_bytes()), Some(b"x")),
        EISDIR
    );
    assert_eq!(diff_write("err14_eisdir2", Some(b"/tmp"), Some(b"x")), EISDIR);

    // over-long component -> ENAMETOOLONG
    let long = dir.join("a".repeat(5000));
    assert_eq!(
        diff_write(
            "err14_toolong",
            Some(long.to_str().unwrap().as_bytes()),
            Some(b"x")
        ),
        ENAMETOOLONG
    );

    // unwritable directory -> EACCES (skipped when running as root, where the
    // permission bits are not enforced)
    let ro = dir.join("ro");
    std::fs::create_dir_all(&ro).unwrap();
    let mut perm = std::fs::metadata(&ro).unwrap().permissions();
    {
        use std::os::unix::fs::PermissionsExt;
        perm.set_mode(0o500);
    }
    std::fs::set_permissions(&ro, perm).unwrap();
    let target = ro.join("x");
    let rc = diff_write("err14_eacces", Some(target.to_str().unwrap().as_bytes()), Some(b"x"));
    if unsafe { geteuid() } != 0 {
        assert_eq!(rc, EACCES, "err14: expected EACCES in a 0500 directory");
    }
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p2 = std::fs::metadata(&ro).unwrap().permissions();
        p2.set_mode(0o700);
        std::fs::set_permissions(&ro, p2).unwrap();
    }
}

extern "C" {
    fn geteuid() -> u32;
}

// ===========================================================================
// ERRORS.md #15 — fprintf to the stream fails (ENOSPC via /dev/full)
// ===========================================================================

#[test]
fn err_15_write_fprintf_failure() {
    if !std::path::Path::new("/dev/full").exists() {
        eprintln!("err15: /dev/full unavailable — skipping");
        return;
    }
    // > BUFSIZ so that fprintf itself has to flush and therefore fails
    let big = vec![b'A'; 100_000];
    let rc = diff_write("err15", Some(b"/dev/full"), Some(&big));
    assert_eq!(rc, ENOSPC, "err15: expected ENOSPC from the failed write");
}

// ===========================================================================
// ERRORS.md #16 — fclose fails while flushing (ENOSPC via /dev/full)
// ===========================================================================

#[test]
fn err_16_write_fclose_failure() {
    if !std::path::Path::new("/dev/full").exists() {
        eprintln!("err16: /dev/full unavailable — skipping");
        return;
    }
    // small enough to sit in the stdio buffer, so only fclose fails
    for content in [&b"hello"[..], b"x", b"a\nb\n"] {
        let rc = diff_write("err16", Some(b"/dev/full"), Some(content));
        assert_eq!(rc, ENOSPC, "err16: expected ENOSPC from the failed fclose");
    }
    let _ = EBADF; // documented in ERRORS.md as the other reachable fclose errno
}

// ===========================================================================
// ERRORS.md #17 — filename == NULL is not checked; fopen(NULL) -> EFAULT
// ===========================================================================

#[test]
fn err_17_write_null_filename() {
    let rc = diff_write("err17", None, Some(b"payload"));
    assert_eq!(rc, EFAULT, "err17: expected EFAULT from fopen(NULL)");
    // and the diagnostic renders the NULL as glibc's "(null)"
    let _g = global_lock();
    let (c, _r) = both();
    let content = cs("payload");
    let (_rc, ce) = capture_stderr("err17_msg", || unsafe {
        (c.write_to_file)(ptr::null(), content.as_ptr())
    });
    assert_eq!(ce, b"Error opening file '(null)': Bad address\n");
}

// ===========================================================================
// ERRORS.md #18 — driver: matrix A fails to initialise
// ===========================================================================

#[test]
fn err_18_driver_mat_a_fails() {
    // insufficient rows for A
    assert_eq!(
        diff_driver("err18a", 2, 3, "1 2\n3 4", 2, 2, "1 2\n3 4", |_| {}),
        EXIT_FAILURE
    );
    // insufficient columns for A
    assert_eq!(
        diff_driver("err18b", 3, 2, "1 2\n3 4", 2, 3, "1 2\n3 4\n5 6", |_| {}),
        EXIT_FAILURE
    );
    // A's allocation fails (negative height)
    assert_eq!(
        diff_driver("err18c", 2, -1, "1 2\n3 4", 2, 2, "1 2\n3 4", |_| {}),
        EXIT_FAILURE
    );
    // A's allocation fails (negative width, enough rows)
    assert_eq!(
        diff_driver("err18d", -1, 2, "a\nb", 2, 2, "1 2\n3 4", |_| {}),
        EXIT_FAILURE
    );
}

// ===========================================================================
// ERRORS.md #19 — driver: matrix B fails to initialise
// ===========================================================================

#[test]
fn err_19_driver_mat_b_fails() {
    assert_eq!(
        diff_driver("err19a", 2, 2, "1 2\n3 4", 2, 3, "1 2\n3 4", |_| {}),
        EXIT_FAILURE
    );
    assert_eq!(
        diff_driver("err19b", 2, 2, "1 2\n3 4", 3, 2, "1 2\n3 4", |_| {}),
        EXIT_FAILURE
    );
    assert_eq!(
        diff_driver("err19c", 2, 2, "1 2\n3 4", 2, -1, "1 2\n3 4", |_| {}),
        EXIT_FAILURE
    );
    assert_eq!(
        diff_driver("err19d", 2, 2, "1 2\n3 4", -1, 2, "a\nb", |_| {}),
        EXIT_FAILURE
    );
}

// ===========================================================================
// ERRORS.md #20 — driver: multiply_matrices returns NULL
// ===========================================================================

#[test]
fn err_20_driver_dim_mismatch() {
    assert_eq!(
        diff_driver("err20a", 2, 2, "1 2\n3 4", 3, 3, "1 2 3\n4 5 6\n7 8 9", |_| {}),
        EXIT_FAILURE
    );
    assert_eq!(
        diff_driver("err20b", 1, 1, "5", 1, 2, "1\n2", |_| {}),
        EXIT_FAILURE
    );
    assert_eq!(
        diff_driver("err20c", 3, 1, "1 2 3", 1, 1, "9", |_| {}),
        EXIT_FAILURE
    );
    // width_b negative and height_b == 0: `multiply_matrices` succeeds in
    // reaching allocate_matrix, which fails, and the loops then never run, so
    // res is NULL without any dereference.
    assert_eq!(
        diff_driver("err20d", 0, 3, "a\nb\nc", -1, 0, "", |_| {}),
        EXIT_FAILURE
    );
}

// ===========================================================================
// ERRORS.md #21 — driver: matrix_to_string returns NULL (buffer_size wraps)
// ===========================================================================

#[test]
fn err_21_driver_to_string_fails() {
    let o = diff_payload("driver_to_string_fail");
    let s = String::from_utf8_lossy(&o.stderr);
    assert!(
        s.contains("Failed to allocate memory for matrix string"),
        "err21: expected the malloc diagnostic, got {s:?}"
    );
    assert!(
        s.contains(&format!("RESULT={EXIT_FAILURE}")),
        "err21: expected RESULT={EXIT_FAILURE}, got {s:?}"
    );
}

// ===========================================================================
// ERRORS.md #22 — driver: write_to_file fails
// ===========================================================================

#[test]
fn err_22_driver_write_fails() {
    // Make "matrix.txt" a directory so fopen(...,"w") fails with EISDIR.
    let rc = diff_driver("err22", 2, 2, "1 2\n3 4", 2, 2, "5 6\n7 8", |dir| {
        let p = dir.join("matrix.txt");
        if !p.is_dir() {
            let _ = std::fs::remove_file(&p);
            std::fs::create_dir_all(&p).unwrap();
        }
    });
    assert_eq!(rc, EXIT_FAILURE, "err22: expected EXIT_FAILURE");
}

// ===========================================================================
// ERRORS.md #23 — driver success returns EXIT_SUCCESS
// ===========================================================================

#[test]
fn err_23_driver_success_code() {
    assert_eq!(
        diff_driver("err23", 2, 2, "1 2\n3 4", 2, 2, "5 6\n7 8", |_| {}),
        EXIT_SUCCESS
    );
}

// ===========================================================================
// G1 — NULL pointers into every pointer parameter
// ===========================================================================

#[test]
fn g1_null_pointers() {
    // free_matrix(NULL), matrix_to_string(NULL), write_to_file(NULL, ...) and
    // write_to_file(..., NULL) are all covered above and are non-crashing.
    let _g = global_lock();
    let (c, r) = both();
    let ((), ce) = capture_stderr("g1_c", || unsafe {
        (c.free_matrix)(ptr::null_mut());
        let s = (c.matrix_to_string)(ptr::null_mut());
        assert!(s.is_null());
        assert_eq!((c.write_to_file)(ptr::null(), ptr::null()), EINVAL);
    });
    let ((), re) = capture_stderr("g1_r", || unsafe {
        (r.free_matrix)(ptr::null_mut());
        let s = (r.matrix_to_string)(ptr::null_mut());
        assert!(s.is_null());
        assert_eq!((r.write_to_file)(ptr::null(), ptr::null()), EINVAL);
    });
    assert_eq_stderr("g1: NULL pointers", &ce, &re);
    // The two that legitimately fault are compared out-of-process in
    // err_05_null_input_crashes_both / err_10_multiply_null_crashes_both.

    // `matrix_to_string` on a `matrix_t` whose `matrix` field is NULL but whose
    // height is 0: the cell loops never run, so it renders the empty string.
    let (c, r) = both();
    for (w, h) in [(0, 0), (5, 0), (-1, 0)] {
        let (cres, ce) = capture_stderr("g1b_c", || unsafe {
            let m = heap_matrix(ptr::null_mut(), w, h);
            let s = take_cstring((c.matrix_to_string)(m));
            libc_free(m as *mut c_void);
            s
        });
        let (rres, re) = capture_stderr("g1b_r", || unsafe {
            let m = heap_matrix(ptr::null_mut(), w, h);
            let s = take_cstring((r.matrix_to_string)(m));
            libc_free(m as *mut c_void);
            s
        });
        let ctx = format!("g1: matrix_to_string({{matrix:NULL,{w},{h}}})");
        assert_eq_bytes(&ctx, &cres, &rres);
        assert_eq_stderr(&ctx, &ce, &re);
    }
}

// ===========================================================================
// G2 — zero lengths
// ===========================================================================

#[test]
fn g2_zero_lengths() {
    for (w, h) in [(0, 0), (0, 1), (1, 0), (0, 64), (64, 0)] {
        diff_allocate("g2", w, h);
    }
    diff_init("g2b", "", 0, 0);
    diff_init("g2c", "1 2 3", 0, 0);
    let dir = scratch_dir("g2");
    assert_eq!(
        diff_write(
            "g2d",
            Some(dir.join("z").to_str().unwrap().as_bytes()),
            Some(b"")
        ),
        0
    );
}

// ===========================================================================
// G3 — oversized lengths
// ===========================================================================

#[test]
fn g3_oversized_lengths() {
    for (w, h) in [
        (i32::MAX, i32::MAX),
        (i32::MAX - 1, 2),
        (1 << 20, 1 << 20),
        (1 << 30, 1),
        (1, 1 << 30),
        (i32::MAX, 1),
        (1, i32::MAX),
    ] {
        diff_allocate("g3", w, h);
    }
    // `initialize_matrix_from_string` never NULL-checks `allocate_matrix`, so
    // when a huge dimension makes the allocation fail the C dereferences NULL.
    // That is real C behaviour, compared out-of-process.
    for name in [
        "init_huge_width",
        "init_huge_height",
        "init_alloc_fail_deref",
    ] {
        let o = diff_payload(name);
        assert!(o.signal.is_some(), "g3/{name}: expected a fatal signal, got {o:?}");
    }
}

// ===========================================================================
// G4 — one step past the valid range
// ===========================================================================

#[test]
fn g4_one_past_range() {
    for (w, h) in [
        (-1, 1),
        (1, -1),
        (-1, -1),
        (i32::MIN, 1),
        (1, i32::MIN),
        (i32::MIN, i32::MIN),
        (i32::MIN, 0),
        (0, i32::MIN),
    ] {
        diff_allocate("g4", w, h);
    }
    // atoi one past INT_MAX / INT_MIN (width 1 keeps rendering exact)
    for tok in [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775809",
    ] {
        let _g = global_lock();
        let (c, r) = both();
        let s = cs(tok);
        let run = |api: &Api, t: &str| {
            capture_stderr(t, || unsafe {
                let m = (api.initialize_matrix_from_string)(s.as_ptr(), 1, 1);
                let snap = snapshot(m);
                let rendered = take_cstring((api.matrix_to_string)(m));
                (api.free_matrix)(m);
                (snap, rendered)
            })
        };
        let ((csn, cst), ce) = run(c, "g4_c");
        let ((rsn, rst), re) = run(r, "g4_r");
        assert_eq_snap(&format!("g4: atoi({tok:?})"), &csn, &rsn);
        assert_eq_bytes(&format!("g4: atoi({tok:?}) rendered"), &cst, &rst);
        assert_eq_stderr(&format!("g4: atoi({tok:?})"), &ce, &re);
    }
}

// ===========================================================================
// G5 — unconstrained integer parameters across the FFI boundary
//
// The C API declares no enum types, so the analogue of "an enum value with no
// valid variant" is an arbitrary `int` in `width`/`height`. Fuzz the full i32
// range (biased towards the boundaries) and require identical outcomes.
// ===========================================================================

#[test]
fn g5_arbitrary_int_parameters() {
    let mut rng = Rng::new(SEED ^ 5);
    let boundaries = [
        i32::MIN,
        i32::MIN + 1,
        -(1 << 30),
        -(1 << 20),
        -2,
        -1,
        0,
        1,
        2,
        1 << 20,
        1 << 30,
        i32::MAX - 1,
        i32::MAX,
    ];

    // allocate_matrix over the boundary cross-product
    for &w in &boundaries {
        for &h in &boundaries {
            diff_allocate("g5_alloc", w, h);
        }
    }

    // initialize_matrix_from_string with arbitrary dims and a fixed input
    let inputs = ["", "1", "1 2\n3 4", "\n\n", "  ", "a b c\nd e f"];
    for &w in &boundaries {
        for &h in &boundaries {
            // Skip the combinations that would legitimately allocate gigabytes
            // (they are covered separately by g3); keep everything that fails
            // fast or is tiny.
            if w > 0 && h > 0 && (w as i64) * (h as i64) > 1_000_000 {
                continue;
            }
            for input in inputs {
                diff_init("g5_init", input, w, h);
            }
        }
    }

    // random i32 pairs
    for _ in 0..200 {
        let w = rng.next_u64() as i32;
        let h = rng.next_u64() as i32;
        if w > 0 && h > 0 && (w as i64) * (h as i64) > 1_000_000 {
            // both must still agree, but avoid multi-GB successful allocations
            continue;
        }
        diff_allocate("g5_rand", w, h);
        diff_init("g5_rand_init", "1 2\n3 4", w, h);
    }
}
