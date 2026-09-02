//! Phase C (extension) — crafted `matrix_t` states and negative dimensions.
//!
//! These are the "one step past the valid range" inputs that the C reaches
//! WITHOUT undefined behaviour, because a negative loop bound makes the body
//! never execute, so no NULL/out-of-range dereference happens. They are the
//! easiest cases for a translation to get wrong (an `as usize` cast instead of
//! signed comparison would turn a skipped loop into an enormous one), so each
//! is compared against the C.

mod common;

use common::*;
use std::os::raw::{c_int, c_void};

/// Runs `initialize_matrix_from_string` on both and compares result + stderr.
fn cmp_init(b: &Both, input: &str, w: c_int, h: c_int) {
    let s = cs(input);
    let run = |api: &Api| {
        let (p, err) =
            capture_stderr(|| unsafe { (api.initialize_matrix_from_string)(s.as_ptr(), w, h) });
        let null = p.is_null();
        let snap = unsafe { snapshot(p) };
        if !p.is_null() {
            unsafe { (api.free_matrix)(p) };
        }
        (null, snap, err)
    };
    let (nc, sc, ec) = run(&b.c);
    let (nr, sr, er) = run(&b.rs);
    assert_eq!(nc, nr, "init({input:?},{w},{h}) NULL-ness mismatch");
    assert_eq!(sc, sr, "init({input:?},{w},{h}) snapshot mismatch");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        String::from_utf8_lossy(&er),
        "init({input:?},{w},{h}) stderr mismatch"
    );
}

/// `width < 0` with `height > 0`: `allocate_matrix` fails and returns NULL, but
/// the column loop `for (j = 0; j < width; j++)` never runs, so the C never
/// dereferences the NULL matrix. It tokenises every row and returns NULL with
/// only `allocate_matrix`'s `perror` on stderr — no "Insufficient …" message.
#[test]
fn crafted_init_negative_width_positive_height() {
    let b = load_both();
    for w in [-1, -2, -12345, i32::MIN] {
        cmp_init(&b, "1 2\n3 4\n", w, 2);
        cmp_init(&b, "1 2\n3 4\n", w, 1);
        // Fewer rows than `height`: the "Insufficient rows" branch is reached
        // with a NULL `mat`, and `free_matrix(NULL)` keeps it safe.
        cmp_init(&b, "1 2\n", w, 3);
        cmp_init(&b, "", w, 1);
    }
}

/// `height < 0`: the row loop never runs; `allocate_matrix` already returned
/// NULL, and the function returns it unchanged after freeing the copy.
#[test]
fn crafted_init_negative_height() {
    let b = load_both();
    for h in [-1, -2, -9999, i32::MIN] {
        for w in [0, 1, 3, -1, i32::MIN] {
            cmp_init(&b, "1 2 3\n4 5 6\n", w, h);
        }
    }
}

/// `matrix_to_string` over crafted negative `width`/`height`.
///
/// Note the interesting combination `width = -1, height = -1`, where
/// `buffer_size = -1*(-11) + -1 + 1 = 11` is POSITIVE, so `malloc` succeeds and
/// the function returns the empty string rather than NULL.
#[test]
fn crafted_to_string_negative_dims() {
    let b = load_both();
    let run = |api: &Api, w: c_int, h: c_int| unsafe {
        let m = make_matrix(api, 0, 0, &[]);
        (*m).width = w;
        (*m).height = h;
        let (p, err) = capture_stderr(|| (api.matrix_to_string)(m));
        let bytes = cstr_bytes(p);
        if !p.is_null() {
            libc_free(p as *mut c_void);
        }
        (*m).width = 0;
        (*m).height = 0;
        (api.free_matrix)(m);
        (bytes, err)
    };
    for w in [i32::MIN, -3, -2, -1, 0, 1, 2, 3, i32::MAX] {
        for h in [i32::MIN, -3, -2, -1, 0] {
            let (bc, ec) = run(&b.c, w, h);
            let (br, er) = run(&b.rs, w, h);
            assert_eq!(bc, br, "matrix_to_string(w={w},h={h}) result mismatch");
            assert_eq!(
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er),
                "matrix_to_string(w={w},h={h}) stderr mismatch"
            );
        }
    }
    // Pin the surprising case so a regression is visible, not silently equal.
    let (bc, _) = run(&b.c, -1, -1);
    assert_eq!(bc.as_deref(), Some(&b""[..]), "expected empty string for (-1,-1)");
}

/// `multiply_matrices` when `mat_a->height < 0`: the result allocation fails,
/// but the outer loop never runs, so the C safely returns the NULL it got.
#[test]
fn crafted_multiply_negative_height_a() {
    let b = load_both();
    let run = |api: &Api, inner: c_int, wb: c_int, bad_ha: c_int| unsafe {
        let a = make_matrix(api, inner, 1, &vec![2; inner.max(0) as usize]);
        let bb = make_matrix(api, wb, inner, &vec![3; (wb * inner).max(0) as usize]);
        let saved = (*a).height;
        (*a).height = bad_ha;
        let (res, err) = capture_stderr(|| (api.multiply_matrices)(a, bb));
        let snap = snapshot(res);
        let null = res.is_null();
        if !res.is_null() {
            (api.free_matrix)(res);
        }
        (*a).height = saved;
        (api.free_matrix)(a);
        (api.free_matrix)(bb);
        (null, snap, err)
    };
    for bad in [-1, -5, i32::MIN] {
        for (inner, wb) in [(1, 1), (2, 3), (3, 2)] {
            let (nc, sc, ec) = run(&b.c, inner, wb, bad);
            let (nr, sr, er) = run(&b.rs, inner, wb, bad);
            assert_eq!(nc, nr, "multiply(ha={bad}) NULL-ness mismatch");
            assert_eq!(sc, sr, "multiply(ha={bad}) snapshot mismatch");
            assert_eq!(
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er),
                "multiply(ha={bad}) stderr mismatch"
            );
        }
    }
}

/// `multiply_matrices` when `mat_b->width < 0` **and** `mat_a->height == 0`:
/// the result allocation fails, and with zero rows the C never touches it.
#[test]
fn crafted_multiply_negative_width_b_zero_height_a() {
    let b = load_both();
    let run = |api: &Api, inner: c_int, bad_wb: c_int| unsafe {
        let a = make_matrix(api, inner, 0, &[]);
        let bb = make_matrix(api, 1, inner, &vec![3; inner.max(0) as usize]);
        let saved = (*bb).width;
        (*bb).width = bad_wb;
        let (res, err) = capture_stderr(|| (api.multiply_matrices)(a, bb));
        let null = res.is_null();
        let snap = snapshot(res);
        if !res.is_null() {
            (api.free_matrix)(res);
        }
        (*bb).width = saved;
        (api.free_matrix)(a);
        (api.free_matrix)(bb);
        (null, snap, err)
    };
    for bad in [-1, -7, i32::MIN] {
        for inner in [0, 1, 3] {
            let (nc, sc, ec) = run(&b.c, inner, bad);
            let (nr, sr, er) = run(&b.rs, inner, bad);
            assert_eq!(nc, nr, "multiply(wb={bad},inner={inner}) NULL-ness mismatch");
            assert_eq!(sc, sr, "multiply(wb={bad},inner={inner}) snapshot mismatch");
            assert_eq!(
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er),
                "multiply(wb={bad},inner={inner}) stderr mismatch"
            );
        }
    }
}

/// `free_matrix` with a negative `height` must skip the row-free loop rather
/// than iterate a huge unsigned count. Verified indirectly: if either side
/// looped, it would free wild pointers and abort.
#[test]
fn crafted_free_matrix_negative_height() {
    let b = load_both();
    for api in [&b.c, &b.rs] {
        unsafe {
            let m = make_matrix(api, 2, 2, &[1, 2, 3, 4]);
            let rows = (*m).matrix;
            (*m).height = -4;
            let (_, err) = capture_stderr(|| (api.free_matrix)(m));
            assert!(err.is_empty(), "{} free_matrix printed: {:?}", api.name, err);
            // `mat` and `mat->matrix` were freed, the two rows leaked — exactly
            // what the C does. Free them to keep the test allocation-clean.
            let _ = rows; // rows array itself is already freed; nothing to do.
        }
    }
}

/// `driver` with negative dimensions: parsing returns NULL for A, so the C
/// bails out at the first check. Uses a private cwd since `driver` writes
/// `matrix.txt` relative to it.
#[test]
fn crafted_driver_negative_dims() {
    let d = std::env::temp_dir().join(format!("difftest-driver-crafted-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::env::set_current_dir(&d).unwrap();
    let b = load_both();

    let run = |api: &Api, wa: c_int, ha: c_int, wb: c_int, hb: c_int| {
        let sa = cs("1 2\n3 4\n");
        let sb = cs("5 6\n7 8\n");
        let _ = std::fs::remove_file("matrix.txt");
        let (rc, err) = capture_stderr(|| unsafe {
            (api.driver)(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr())
        });
        let out = std::fs::read("matrix.txt").ok();
        let _ = std::fs::remove_file("matrix.txt");
        (rc, out, err)
    };
    for (wa, ha, wb, hb) in [
        (-1, 2, 2, 2),
        (2, -1, 2, 2),
        (2, 2, -1, 2),
        (2, 2, 2, -1),
        (i32::MIN, 2, 2, 2),
        (2, i32::MIN, 2, 2),
        (2, 2, i32::MIN, 2),
        (2, 2, 2, i32::MIN),
        (-1, -1, -1, -1),
    ] {
        let (rc_c, out_c, ec) = run(&b.c, wa, ha, wb, hb);
        let (rc_r, out_r, er) = run(&b.rs, wa, ha, wb, hb);
        assert_eq!(rc_c, rc_r, "driver({wa},{ha},{wb},{hb}) rc mismatch");
        assert_eq!(out_c, out_r, "driver({wa},{ha},{wb},{hb}) output mismatch");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "driver({wa},{ha},{wb},{hb}) stderr mismatch"
        );
    }
}
