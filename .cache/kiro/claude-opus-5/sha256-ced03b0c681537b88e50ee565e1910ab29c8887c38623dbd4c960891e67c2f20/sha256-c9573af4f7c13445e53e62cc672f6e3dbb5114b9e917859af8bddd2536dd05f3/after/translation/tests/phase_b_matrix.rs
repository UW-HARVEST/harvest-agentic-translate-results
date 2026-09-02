//! Phase B — valid-path differential tests for the matrix entry points.
//!
//! Rows 1–33 of `CONFIGS.md`. Everything is driven through the exported
//! symbols of BOTH `.so`s; the lowest-level entry point (`allocate_matrix`)
//! comes first and the composed ones build on it.

mod common;

use common::*;
use std::os::raw::{c_int, c_void};

/// width, height, and which row pointers came back non-NULL. Row *contents*
/// are deliberately not compared here: `allocate_matrix` leaves them as
/// uninitialised `malloc` memory.
unsafe fn alloc_shape(api: &Api, w: c_int, h: c_int) -> (c_int, c_int, Vec<bool>) {
    unsafe {
        let mat = (api.allocate_matrix)(w, h);
        assert!(!mat.is_null(), "{} allocate_matrix({w},{h}) -> NULL", api.name);
        let mut rows = Vec::new();
        for i in 0..h.max(0) {
            rows.push(!(*(*mat).matrix.offset(i as isize)).is_null());
        }
        let out = ((*mat).width, (*mat).height, rows);
        (api.free_matrix)(mat);
        out
    }
}

fn check_alloc(b: &Both, w: c_int, h: c_int) {
    unsafe {
        let a = alloc_shape(&b.c, w, h);
        let r = alloc_shape(&b.rs, w, h);
        assert_eq!(a, r, "allocate_matrix({w},{h}) shape mismatch");
    }
}

// --- CONFIGS.md rows 1-4: allocate_matrix / free_matrix -------------------

#[test]
fn row01_alloc_zero_by_zero() {
    let b = load_both();
    check_alloc(&b, 0, 0);
}

#[test]
fn row02_alloc_zero_dim() {
    let b = load_both();
    for n in [1, 2, 5, 17] {
        check_alloc(&b, 0, n);
        check_alloc(&b, n, 0);
    }
}

#[test]
fn row03_alloc_one_by_one() {
    let b = load_both();
    check_alloc(&b, 1, 1);
}

#[test]
fn row04_alloc_randomized() {
    let b = load_both();
    let mut rng = Rng::new(0x4A11_0C47);
    for _ in 0..256 {
        let w = rng.range(0, 32) as c_int;
        let h = rng.range(0, 32) as c_int;
        check_alloc(&b, w, h);
    }
}

// --- initialize_matrix_from_string ----------------------------------------

fn check_init(b: &Both, input: &str, w: c_int, h: c_int) {
    unsafe {
        let s = cs(input);
        let (mc, ec) = capture_stderr(|| (b.c.initialize_matrix_from_string)(s.as_ptr(), w, h));
        let sc = snapshot(mc);
        if !mc.is_null() {
            (b.c.free_matrix)(mc);
        }

        let (mr, er) = capture_stderr(|| (b.rs.initialize_matrix_from_string)(s.as_ptr(), w, h));
        let sr = snapshot(mr);
        if !mr.is_null() {
            (b.rs.free_matrix)(mr);
        }

        assert_eq!(
            sc, sr,
            "initialize_matrix_from_string({input:?},{w},{h}) result mismatch"
        );
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "initialize_matrix_from_string({input:?},{w},{h}) stderr mismatch"
        );
    }
}

#[test]
fn row05_init_1x1() {
    let b = load_both();
    check_init(&b, "7\n", 1, 1);
    check_init(&b, "7", 1, 1);
    check_init(&b, "-7\n", 1, 1);
    check_init(&b, "0\n", 1, 1);
}

#[test]
fn row06_init_column_vector() {
    let b = load_both();
    for n in [1, 2, 3, 8] {
        let vals: Vec<c_int> = (0..n).map(|i| if i % 2 == 0 { i } else { -i * 13 }).collect();
        let text = render_matrix_text(1, n as usize, &vals);
        check_init(&b, &text, 1, n);
    }
}

#[test]
fn row07_init_row_vector() {
    let b = load_both();
    for n in [1, 2, 3, 9] {
        let vals: Vec<c_int> = (0..n).map(|i| if i % 3 == 0 { -i } else { i * 7 }).collect();
        let text = render_matrix_text(n as usize, 1, &vals);
        check_init(&b, &text, n, 1);
    }
}

#[test]
fn row08_init_square_random() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0008);
    for _ in 0..200 {
        let n = rng.range(1, 8) as usize;
        let vals: Vec<c_int> = (0..n * n).map(|_| rng.safe_value()).collect();
        let text = render_matrix_text(n, n, &vals);
        check_init(&b, &text, n as c_int, n as c_int);
    }
}

#[test]
fn row09_init_rect_random() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0009);
    for _ in 0..200 {
        let w = rng.range(1, 9) as usize;
        let mut h = rng.range(1, 9) as usize;
        if h == w {
            h = if h == 9 { 1 } else { h + 1 };
        }
        let vals: Vec<c_int> = (0..w * h).map(|_| rng.safe_value()).collect();
        let text = render_matrix_text(w, h, &vals);
        check_init(&b, &text, w as c_int, h as c_int);
    }
}

#[test]
fn row10_init_zero_width() {
    let b = load_both();
    // Inner loop never runs, but `strtok_r(row, " ")` is still invoked once per
    // row, mutating the row buffer in place.
    check_init(&b, "1 2 3\n4 5 6\n", 0, 2);
    check_init(&b, "\n\n\n", 0, 3);
    check_init(&b, "a b\n", 0, 1);
}

#[test]
fn row11_init_zero_height() {
    let b = load_both();
    for input in ["", "\n", "1 2 3\n4 5 6\n", "garbage"] {
        check_init(&b, input, 0, 0);
        check_init(&b, input, 5, 0);
    }
}

#[test]
fn row12_init_surplus_rows() {
    let b = load_both();
    check_init(&b, "1 2\n3 4\n5 6\n7 8\n", 2, 2);
    check_init(&b, "1\n2\n3\n4\n5\n", 1, 1);
    check_init(&b, "1 2 3\n4 5 6\n7 8 9\n", 3, 1);
}

#[test]
fn row13_init_surplus_cols() {
    let b = load_both();
    check_init(&b, "1 2 3 4\n5 6 7 8\n", 2, 2);
    check_init(&b, "1 2 3 4 5\n", 1, 1);
    check_init(&b, "9 8 7\n6 5 4\n3 2 1\n", 1, 3);
}

#[test]
fn row14_init_separator_runs() {
    let b = load_both();
    check_init(&b, "1   2\n3      4\n", 2, 2);
    check_init(&b, "   1 2\n   3 4\n", 2, 2);
    check_init(&b, "1 2   \n3 4   \n", 2, 2);
    check_init(&b, "  1   2  \n  3   4  \n", 2, 2);
    // Tabs are NOT separators for this parser: "1\t2" is a single token.
    check_init(&b, "1\t2 3\n", 2, 1);
    check_init(&b, "1\t2\n", 2, 1);
}

#[test]
fn row15_init_line_endings() {
    let b = load_both();
    check_init(&b, "1 2\n3 4\n", 2, 2);
    check_init(&b, "1 2\n3 4", 2, 2);
    // strtok_r collapses runs of '\n', so blank lines are skipped.
    check_init(&b, "1 2\n\n\n3 4\n", 2, 2);
    check_init(&b, "\n\n1 2\n3 4\n", 2, 2);
    check_init(&b, "1 2\n3 4\n\n\n", 2, 2);
}

#[test]
fn row16_init_nonnumeric_tokens() {
    let b = load_both();
    let toks = [
        "abc", "12abc", "0x10", "+5", "-0", "007", "--3", ".5", "1e3", " ", "-", "+", "2147483648",
        "0", "-99", "9 9",
    ];
    for t in toks {
        let text = format!("{t}\n");
        check_init(&b, &text, 1, 1);
    }
    check_init(&b, "abc 12abc\n0x10 +5\n", 2, 2);
    check_init(&b, "--3 .5 1e3\n", 3, 1);
}

#[test]
fn row17_init_atoi_extremes() {
    let b = load_both();
    for t in [
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "99999999999",
        "-99999999999",
        "9223372036854775807",
        "-9223372036854775808",
        "18446744073709551616",
    ] {
        check_init(&b, &format!("{t}\n"), 1, 1);
    }
}

#[test]
fn row18_init_text_form_fuzz() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0018);
    let seps = [" ", "  ", "   ", " \t ", "\t"];
    let forms: [&str; 6] = ["{}", " {}", "{} ", "+{}", "{}x", "0{}"];
    for _ in 0..300 {
        let w = rng.range(1, 5) as usize;
        let h = rng.range(1, 5) as usize;
        let mut text = String::new();
        for _ in 0..h {
            let extra_cols = rng.range(0, 2) as usize;
            for j in 0..(w + extra_cols) {
                if j > 0 {
                    text.push_str(rng.pick(&seps));
                }
                let v = rng.range(-99_999, 99_999);
                let form = *rng.pick(&forms);
                text.push_str(&form.replace("{}", &v.to_string()));
            }
            text.push('\n');
            if rng.bool() {
                text.push('\n');
            }
        }
        if !rng.bool() {
            while text.ends_with('\n') {
                text.pop();
            }
        }
        check_init(&b, &text, w as c_int, h as c_int);
    }
}

// --- multiply_matrices ----------------------------------------------------

fn check_multiply(b: &Both, wa: c_int, ha: c_int, va: &[c_int], wb: c_int, hb: c_int, vb: &[c_int]) {
    unsafe {
        let (sc, ec) = {
            let a = make_matrix(&b.c, wa, ha, va);
            let bb = make_matrix(&b.c, wb, hb, vb);
            let (res, err) = capture_stderr(|| (b.c.multiply_matrices)(a, bb));
            let snap = snapshot(res);
            if !res.is_null() {
                (b.c.free_matrix)(res);
            }
            (b.c.free_matrix)(a);
            (b.c.free_matrix)(bb);
            (snap, err)
        };
        let (sr, er) = {
            let a = make_matrix(&b.rs, wa, ha, va);
            let bb = make_matrix(&b.rs, wb, hb, vb);
            let (res, err) = capture_stderr(|| (b.rs.multiply_matrices)(a, bb));
            let snap = snapshot(res);
            if !res.is_null() {
                (b.rs.free_matrix)(res);
            }
            (b.rs.free_matrix)(a);
            (b.rs.free_matrix)(bb);
            (snap, err)
        };
        assert_eq!(
            sc, sr,
            "multiply_matrices A({wa}x{ha}) B({wb}x{hb}) result mismatch"
        );
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "multiply_matrices A({wa}x{ha}) B({wb}x{hb}) stderr mismatch"
        );
    }
}

#[test]
fn row19_multiply_1x1() {
    let b = load_both();
    check_multiply(&b, 1, 1, &[6], 1, 1, &[7]);
    check_multiply(&b, 1, 1, &[-6], 1, 1, &[7]);
    check_multiply(&b, 1, 1, &[0], 1, 1, &[0]);
}

#[test]
fn row20_multiply_inner_product() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0020);
    for _ in 0..200 {
        let n = rng.range(1, 12) as usize;
        // Bound operands so the accumulation stays inside i32.
        let a: Vec<c_int> = (0..n).map(|_| rng.range(-1000, 1000) as c_int).collect();
        let bv: Vec<c_int> = (0..n).map(|_| rng.range(-1000, 1000) as c_int).collect();
        check_multiply(&b, n as c_int, 1, &a, 1, n as c_int, &bv);
    }
}

#[test]
fn row21_multiply_outer_product() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0021);
    for _ in 0..200 {
        let n = rng.range(1, 8) as usize;
        let m = rng.range(1, 8) as usize;
        let a: Vec<c_int> = (0..n).map(|_| rng.range(-40000, 40000) as c_int).collect();
        let bv: Vec<c_int> = (0..m).map(|_| rng.range(-40000, 40000) as c_int).collect();
        check_multiply(&b, 1, n as c_int, &a, m as c_int, 1, &bv);
    }
}

#[test]
fn row22_multiply_square_random() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0022);
    for _ in 0..200 {
        let n = rng.range(1, 8) as usize;
        let a: Vec<c_int> = (0..n * n).map(|_| rng.range(-2000, 2000) as c_int).collect();
        let bv: Vec<c_int> = (0..n * n).map(|_| rng.range(-2000, 2000) as c_int).collect();
        check_multiply(&b, n as c_int, n as c_int, &a, n as c_int, n as c_int, &bv);
    }
}

#[test]
fn row23_multiply_general_random() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0023);
    for _ in 0..200 {
        let ha = rng.range(1, 7) as usize;
        let wa = rng.range(1, 7) as usize; // == hb
        let wb = rng.range(1, 7) as usize;
        let a: Vec<c_int> = (0..ha * wa).map(|_| rng.range(-3000, 3000) as c_int).collect();
        let bv: Vec<c_int> = (0..wa * wb).map(|_| rng.range(-3000, 3000) as c_int).collect();
        check_multiply(&b, wa as c_int, ha as c_int, &a, wb as c_int, wa as c_int, &bv);
    }
}

#[test]
fn row24_multiply_inner_dim_zero() {
    let b = load_both();
    // mat_a->width == 0 == mat_b->height: the k-loop never runs, every result
    // cell keeps its initial 0.
    for (ha, wb) in [(1, 1), (3, 2), (2, 3), (4, 4)] {
        check_multiply(&b, 0, ha, &[], wb, 0, &[]);
    }
}

#[test]
fn row25_multiply_zero_height_a() {
    let b = load_both();
    for (wa, wb) in [(1, 1), (3, 2)] {
        let vb: Vec<c_int> = (0..wa * wb).map(|x| x as c_int).collect();
        check_multiply(&b, wa as c_int, 0, &[], wb as c_int, wa as c_int, &vb);
    }
}

#[test]
fn row26_multiply_zero_width_b() {
    let b = load_both();
    for (ha, wa) in [(1, 1), (2, 3)] {
        let va: Vec<c_int> = (0..ha * wa).map(|x| x as c_int).collect();
        check_multiply(&b, wa as c_int, ha as c_int, &va, 0, wa as c_int, &[]);
    }
}

#[test]
fn row27_multiply_overflowing_accumulation() {
    let b = load_both();
    // Signed overflow wraps identically in both builds.
    check_multiply(&b, 1, 1, &[i32::MAX], 1, 1, &[2]);
    check_multiply(&b, 1, 1, &[i32::MIN], 1, 1, &[-1]);
    check_multiply(&b, 2, 1, &[i32::MAX, i32::MAX], 1, 2, &[3, 5]);
    let mut rng = Rng::new(0x5EED_0027);
    for _ in 0..120 {
        let n = rng.range(2, 6) as usize;
        let a: Vec<c_int> = (0..n).map(|_| rng.range(i32::MIN as i64, i32::MAX as i64) as c_int).collect();
        let bv: Vec<c_int> = (0..n).map(|_| rng.range(i32::MIN as i64, i32::MAX as i64) as c_int).collect();
        check_multiply(&b, n as c_int, 1, &a, 1, n as c_int, &bv);
    }
}

// --- matrix_to_string ----------------------------------------------------

fn check_to_string(b: &Both, w: c_int, h: c_int, vals: &[c_int]) {
    unsafe {
        let (bc, ec) = {
            let m = make_matrix(&b.c, w, h, vals);
            let (p, err) = capture_stderr(|| (b.c.matrix_to_string)(m));
            let bytes = cstr_bytes(p);
            if !p.is_null() {
                libc_free(p as *mut c_void);
            }
            (b.c.free_matrix)(m);
            (bytes, err)
        };
        let (br, er) = {
            let m = make_matrix(&b.rs, w, h, vals);
            let (p, err) = capture_stderr(|| (b.rs.matrix_to_string)(m));
            let bytes = cstr_bytes(p);
            if !p.is_null() {
                libc_free(p as *mut c_void);
            }
            (b.rs.free_matrix)(m);
            (bytes, err)
        };
        assert_eq!(bc, br, "matrix_to_string({w}x{h}, {vals:?}) mismatch");
        assert_eq!(
            String::from_utf8_lossy(&ec),
            String::from_utf8_lossy(&er),
            "matrix_to_string({w}x{h}) stderr mismatch"
        );
    }
}

#[test]
fn row28_to_string_width_one() {
    let b = load_both();
    for h in [1, 2, 5] {
        let vals: Vec<c_int> = (0..h).map(|i| (i as c_int) * -111).collect();
        check_to_string(&b, 1, h as c_int, &vals);
    }
}

#[test]
fn row29_to_string_width_many() {
    let b = load_both();
    check_to_string(&b, 2, 1, &[1, 2]);
    check_to_string(&b, 3, 2, &[1, -2, 3, -4, 5, -6]);
    check_to_string(&b, 5, 4, &(0..20).map(|i| i as c_int * 7 - 33).collect::<Vec<_>>());
}

#[test]
fn row30_to_string_zero_height() {
    let b = load_both();
    for w in [0, 1, 3] {
        check_to_string(&b, w, 0, &[]);
    }
}

#[test]
fn row31_to_string_zero_width() {
    let b = load_both();
    for h in [1, 2, 6] {
        check_to_string(&b, 0, h, &[]);
    }
}

#[test]
fn row32_to_string_value_forms() {
    let b = load_both();
    // INT_MIN needs 11 chars; the buffer formula only accommodates that at
    // width == 1, where it fits exactly.
    check_to_string(&b, 1, 1, &[i32::MIN]);
    check_to_string(&b, 1, 3, &[i32::MIN, i32::MAX, 0]);
    check_to_string(&b, 3, 1, &[0, -1, i32::MAX]);
    check_to_string(&b, 4, 2, &[0, -1, 999_999_999, -999_999_999, 1, 10, 100, 1000]);
}

#[test]
fn row33_to_string_randomized() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_0033);
    for _ in 0..300 {
        let w = rng.range(0, 12) as usize;
        let h = rng.range(0, 12) as usize;
        let vals: Vec<c_int> = (0..w * h).map(|_| rng.safe_value()).collect();
        check_to_string(&b, w as c_int, h as c_int, &vals);
    }
}

// --- composed low-level pipeline (parse -> multiply -> stringify) ---------

#[test]
fn composed_pipeline_randomized() {
    let b = load_both();
    let mut rng = Rng::new(0x5EED_C0DE);
    for _ in 0..150 {
        let ha = rng.range(1, 6) as usize;
        let wa = rng.range(1, 6) as usize;
        let wb = rng.range(1, 6) as usize;
        let va: Vec<c_int> = (0..ha * wa).map(|_| rng.range(-500, 500) as c_int).collect();
        let vb: Vec<c_int> = (0..wa * wb).map(|_| rng.range(-500, 500) as c_int).collect();
        let ta = cs(&render_matrix_text(wa, ha, &va));
        let tb = cs(&render_matrix_text(wb, wa, &vb));

        let run = |api: &Api| unsafe {
            let ma = (api.initialize_matrix_from_string)(ta.as_ptr(), wa as c_int, ha as c_int);
            let mb = (api.initialize_matrix_from_string)(tb.as_ptr(), wb as c_int, wa as c_int);
            assert!(!ma.is_null() && !mb.is_null());
            let res = (api.multiply_matrices)(ma, mb);
            assert!(!res.is_null());
            let s = (api.matrix_to_string)(res);
            let bytes = cstr_bytes(s);
            let snap = snapshot(res);
            if !s.is_null() {
                libc_free(s as *mut c_void);
            }
            (api.free_matrix)(res);
            (api.free_matrix)(ma);
            (api.free_matrix)(mb);
            (snap, bytes)
        };
        let (out_c, _e1) = capture_stderr(|| run(&b.c));
        let (out_r, _e2) = capture_stderr(|| run(&b.rs));
        assert_eq!(out_c, out_r, "composed pipeline mismatch {wa}x{ha} * {wb}x{wa}");
    }
}
