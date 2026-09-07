//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are reached only through `dlopen`+`dlsym` on their
//! `.so`s, starting at the lowest-level entry point (`allocate_matrix`) and
//! working up to the fully composed `driver`.

mod common;

use common::*;
use std::ffi::c_int;

// ===========================================================================
// A. allocate_matrix — lowest level
// ===========================================================================

/// Compare `allocate_matrix` structurally. Element bytes are uninitialised
/// `malloc` memory, so only shape / allocation success / usability is compared.
fn diff_allocate(label: &str, w: c_int, h: c_int) {
    let p = pair();
    let (c_res, c_err) = capture_stderr(|| unsafe {
        let m = p.c.allocate_matrix(w, h);
        let s = snapshot(m, false);
        let probe = probe_writable(m);
        p.c.free_matrix(m);
        (s, probe)
    });
    let (r_res, r_err) = capture_stderr(|| unsafe {
        let m = p.rs.allocate_matrix(w, h);
        let s = snapshot(m, false);
        let probe = probe_writable(m);
        p.rs.free_matrix(m);
        (s, probe)
    });
    assert_eq!(
        c_res.0, r_res.0,
        "[{label}] allocate_matrix({w},{h}) shape diverged\n C: {:?}\nRS: {:?}",
        c_res.0, r_res.0
    );
    assert_eq!(
        c_res.1, r_res.1,
        "[{label}] allocate_matrix({w},{h}) row usability diverged"
    );
    assert_eq!(
        show(&c_err),
        show(&r_err),
        "[{label}] allocate_matrix({w},{h}) stderr diverged"
    );
}

#[test]
fn cfg_a1_allocate_1x1() {
    let _g = lock();
    diff_allocate("A1", 1, 1);
}

#[test]
fn cfg_a2_allocate_square() {
    let _g = lock();
    for n in 2..=8 {
        diff_allocate("A2", n, n);
    }
}

#[test]
fn cfg_a3_allocate_zero_width() {
    let _g = lock();
    for h in 1..=4 {
        diff_allocate("A3", 0, h);
    }
}

#[test]
fn cfg_a4_allocate_zero_height() {
    let _g = lock();
    for w in 1..=4 {
        diff_allocate("A4", w, 0);
    }
}

#[test]
fn cfg_a5_allocate_zero_zero() {
    let _g = lock();
    diff_allocate("A5", 0, 0);
}

#[test]
fn cfg_a6_allocate_wide_and_tall() {
    let _g = lock();
    diff_allocate("A6-wide", 7, 2);
    diff_allocate("A6-tall", 2, 7);
}

#[test]
fn cfg_a7_allocate_fuzz() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xA7);
    for i in 0..200 {
        let w = rng.range(0, 32) as c_int;
        let h = rng.range(0, 32) as c_int;
        diff_allocate(&format!("A7#{i}"), w, h);
    }
}

// ===========================================================================
// B. initialize_matrix_from_string
// ===========================================================================

#[test]
fn cfg_b1_init_1x1() {
    let _g = lock();
    diff_init("B1", "42\n", 1, 1);
    diff_init("B1-no-nl", "42", 1, 1);
    diff_init("B1-neg", "-7", 1, 1);
}

#[test]
fn cfg_b2_init_square_exact() {
    let _g = lock();
    diff_init("B2", "1 2 3\n4 5 6\n7 8 9", 3, 3);
}

#[test]
fn cfg_b3_init_trailing_newline() {
    let _g = lock();
    diff_init("B3", "1 2 3\n4 5 6\n7 8 9\n", 3, 3);
    diff_init("B3-many-nl", "1 2\n3 4\n\n\n", 2, 2);
}

#[test]
fn cfg_b4_init_extra_rows_ignored() {
    let _g = lock();
    diff_init("B4", "1 2\n3 4\n5 6\n7 8\n", 2, 2);
}

#[test]
fn cfg_b5_init_extra_cols_ignored() {
    let _g = lock();
    diff_init("B5", "1 2 3 4 5\n6 7 8 9 10\n", 2, 2);
}

#[test]
fn cfg_b6_init_delimiter_collapsing() {
    let _g = lock();
    diff_init("B6-multi-space", "1    2\n3\t4 5", 2, 2);
    diff_init("B6-leading", "   1 2\n   3 4", 2, 2);
    diff_init("B6-trailing", "1 2   \n3 4   ", 2, 2);
    diff_init("B6-blank-lines", "1 2\n\n\n3 4", 2, 2);
    diff_init("B6-leading-nl", "\n\n1 2\n3 4", 2, 2);
}

#[test]
fn cfg_b7_init_atoi_semantics() {
    let _g = lock();
    diff_init("B7-nonnum", "abc def\nghi jkl", 2, 2);
    diff_init("B7-partial", "12abc 34xyz\n5-6 7+8", 2, 2);
    diff_init("B7-plus", "+5 +0\n-0 +12345", 2, 2);
    diff_init("B7-overflow", "99999999999 -99999999999\n2147483648 -2147483649", 2, 2);
    diff_init("B7-intmax", "2147483647 -2147483648\n0 1", 2, 2);
    diff_init("B7-hexish", "0x10 010\n1e3 .5", 2, 2);
    diff_init("B7-empty-tok", "- +\n. ,", 2, 2);
}

#[test]
fn cfg_b8_init_zero_width() {
    let _g = lock();
    diff_init("B8", "1 2\n3 4\n5 6", 0, 3);
    diff_init("B8-short", "a\nb\nc", 0, 3);
}

#[test]
fn cfg_b9_init_zero_height() {
    let _g = lock();
    diff_init("B9", "1 2 3", 3, 0);
    diff_init("B9-empty", "", 3, 0);
    diff_init("B9-0x0", "", 0, 0);
}

#[test]
fn cfg_b10_init_wide_and_tall() {
    let _g = lock();
    diff_init("B10-wide", "1 2 3 4\n5 6 7 8", 4, 2);
    diff_init("B10-tall", "1 2\n3 4\n5 6\n7 8", 2, 4);
}

#[test]
fn cfg_b11_init_fuzz() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xB1);
    for i in 0..300 {
        let w = rng.range(0, 10) as i32;
        let h = rng.range(0, 10) as i32;
        let vals: Vec<Vec<i32>> = (0..h.max(0))
            .map(|_| (0..w.max(0)).map(|_| rng.i32_bounded(999_999_999)).collect())
            .collect();
        let s = render(&vals);
        diff_init(&format!("B11#{i}"), &s, w, h);
    }
}

#[test]
fn cfg_b12_init_fuzz_messy_whitespace() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xB2);
    let toks = [
        "0", "1", "-1", "7", "abc", "12abc", "+5", "-0", "2147483647", "-2147483648",
        "99999999999", "000123", "  9",
    ];
    for i in 0..300 {
        let w = rng.range(0, 6) as i32;
        let h = rng.range(0, 6) as i32;
        // Supply between `n` and `n+2` rows/cols so extras are exercised.
        let extra_rows = rng.range(0, 2) as i32;
        let extra_cols = rng.range(0, 2) as i32;
        let mut s = String::new();
        for _ in 0..(h.max(0) + extra_rows) {
            for c in 0..(w.max(0) + extra_cols) {
                if c > 0 {
                    for _ in 0..rng.range(1, 3) {
                        s.push(' ');
                    }
                }
                s.push_str(rng.pick(&toks));
            }
            for _ in 0..rng.range(1, 2) {
                s.push('\n');
            }
        }
        diff_init(&format!("B12#{i}"), &s, w, h);
    }
}

// ===========================================================================
// C. multiply_matrices
// ===========================================================================

#[test]
fn cfg_c1_multiply_1x1() {
    let _g = lock();
    diff_multiply("C1", "6", 1, 1, "7", 1, 1);
}

#[test]
fn cfg_c2_multiply_inner_dim_zero() {
    let _g = lock();
    // A is 0 wide / 2 high, B is 3 wide / 0 high → inner dim 0 → 2x3 of zeros.
    diff_multiply("C2", "x\ny", 0, 2, "", 3, 0);
}

#[test]
fn cfg_c3_multiply_result_height_zero() {
    let _g = lock();
    diff_multiply("C3", "", 2, 0, "1 2\n3 4", 2, 2);
}

#[test]
fn cfg_c4_multiply_result_width_zero() {
    let _g = lock();
    diff_multiply("C4", "1 2\n3 4", 2, 2, "x\ny", 0, 2);
}

#[test]
fn cfg_c5_multiply_square() {
    let _g = lock();
    diff_multiply(
        "C5",
        "1 2 3\n4 5 6\n7 8 9",
        3,
        3,
        "9 8 7\n6 5 4\n3 2 1",
        3,
        3,
    );
}

#[test]
fn cfg_c6_multiply_non_square_chain() {
    let _g = lock();
    // A: width 4, height 2 ; B: width 3, height 4 → 2x3
    diff_multiply(
        "C6",
        "1 2 3 4\n5 6 7 8",
        4,
        2,
        "1 2 3\n4 5 6\n7 8 9\n10 11 12",
        3,
        4,
    );
}

#[test]
fn cfg_c7_multiply_vectors() {
    let _g = lock();
    // row (1x4) × col (4x1) -> 1x1
    diff_multiply("C7-dot", "1 2 3 4", 4, 1, "5\n6\n7\n8", 1, 4);
    // col (4x1) × row (1x4) -> 4x4 outer product
    diff_multiply("C7-outer", "1\n2\n3\n4", 1, 4, "5 6 7 8", 4, 1);
}

#[test]
fn cfg_c8_multiply_int_overflow_wraps() {
    let _g = lock();
    diff_multiply(
        "C8",
        "100000 100000\n2147483647 -2147483648",
        2,
        2,
        "100000 -100000\n2147483647 3",
        2,
        2,
    );
    diff_multiply(
        "C8-accum",
        "2000000000 2000000000 2000000000",
        3,
        1,
        "2000000000\n2000000000\n2000000000",
        1,
        3,
    );
}

#[test]
fn cfg_c9_multiply_fuzz() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xC9);
    for i in 0..300 {
        let ha = rng.range(0, 8) as i32;
        let inner = rng.range(0, 8) as i32;
        let wb = rng.range(0, 8) as i32;
        // Mix small and overflow-prone magnitudes.
        let bound: i32 = *rng.pick(&[9, 1000, 100_000, 2_000_000_000]);
        let a: Vec<Vec<i32>> = (0..ha)
            .map(|_| (0..inner).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        let b: Vec<Vec<i32>> = (0..inner)
            .map(|_| (0..wb).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        diff_multiply(
            &format!("C9#{i}"),
            &render(&a),
            inner,
            ha,
            &render(&b),
            wb,
            inner,
        );
    }
}

// ===========================================================================
// D. matrix_to_string / write_to_file / driver
// ===========================================================================

#[test]
fn cfg_d1_to_string_single_column() {
    let _g = lock();
    diff_to_string("D1", "42", 1, 1);
    diff_to_string("D1-col", "1\n2\n3", 1, 3);
}

#[test]
fn cfg_d2_to_string_separator_branch() {
    let _g = lock();
    diff_to_string("D2", "1 2 3\n4 5 6", 3, 2);
}

#[test]
fn cfg_d3_to_string_zero_width() {
    let _g = lock();
    diff_to_string("D3", "a\nb\nc", 0, 3);
}

#[test]
fn cfg_d4_to_string_zero_height() {
    let _g = lock();
    diff_to_string("D4", "", 3, 0);
    diff_to_string("D4-0x0", "", 0, 0);
}

#[test]
fn cfg_d5_to_string_digit_widths() {
    let _g = lock();
    // 1..10 significant characters incl. sign — the largest length that stays
    // inside the C's own buffer arithmetic (see ERRORS.md UB note).
    diff_to_string(
        "D5",
        "0 -1 22 -333\n4444 -55555 666666 -7777777\n88888888 -999999999 123456789 -12345678",
        4,
        3,
    );
}

#[test]
fn cfg_d6_to_string_fuzz() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xD6);
    for i in 0..300 {
        let w = rng.range(0, 8) as i32;
        let h = rng.range(0, 8) as i32;
        let bound: i32 = *rng.pick(&[9, 99, 99_999, 999_999_999]);
        let vals: Vec<Vec<i32>> = (0..h)
            .map(|_| (0..w).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        diff_to_string(&format!("D6#{i}"), &render(&vals), w, h);
    }
}

#[test]
fn cfg_d7_write_new_file() {
    let _g = lock();
    diff_write("D7", "new", b"hello world\n");
}

#[test]
fn cfg_d8_write_truncates_existing() {
    let _g = lock();
    let p = pair();
    let path = unique_tmp_path("trunc");
    std::fs::write(&path, b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap();
    let name = cs(path.to_str().unwrap());
    let short = cs("short");
    let (c_rc, _) = capture_stderr(|| unsafe { p.c.write_to_file(name.as_ptr(), short.as_ptr()) });
    let c_bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap();
    let (r_rc, _) = capture_stderr(|| unsafe { p.rs.write_to_file(name.as_ptr(), short.as_ptr()) });
    let r_bytes = std::fs::read(&path).unwrap();
    assert_eq!(c_rc, r_rc, "D8 rc diverged");
    assert_eq!(c_bytes, r_bytes, "D8 truncation diverged");
    assert_eq!(c_bytes, b"short", "D8 mode \"w\" must truncate");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn cfg_d9_write_empty_content() {
    let _g = lock();
    diff_write("D9", "empty", b"");
}

#[test]
fn cfg_d10_write_large_content() {
    let _g = lock();
    let big: Vec<u8> = (0..256 * 1024).map(|i| b'a' + (i % 26) as u8).collect();
    diff_write("D10", "big", &big);
}

#[test]
fn cfg_d11_write_format_specifiers_not_expanded() {
    let _g = lock();
    diff_write("D11", "fmt", b"%s %d %n %% %p %1000000d\n");
}

#[test]
fn cfg_d12_write_newlines_and_utf8() {
    let _g = lock();
    diff_write("D12", "utf8", "a\nb\r\nc\t\u{00e9}\u{4e2d}\u{1F600}\n".as_bytes());
}

#[test]
fn cfg_d13_write_fuzz() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xD3);
    for i in 0..120 {
        let len = rng.range(0, 4096) as usize;
        let content: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        diff_write(&format!("D13#{i}"), &format!("f{i}"), &content);
    }
}

#[test]
fn cfg_d14_driver_square() {
    let _g = lock();
    diff_driver("D14", 2, 2, "1 2\n3 4\n", 2, 2, "5 6\n7 8\n");
}

#[test]
fn cfg_d15_driver_non_square() {
    let _g = lock();
    // A: width 3, height 2 ; B: width 2, height 3
    diff_driver("D15a", 3, 2, "1 2 3\n4 5 6\n", 2, 3, "7 8\n9 10\n11 12\n");
    // A: width 1, height 3 ; B: width 3, height 1
    diff_driver("D15b", 1, 3, "1\n2\n3\n", 3, 1, "4 5 6\n");
}

#[test]
fn cfg_d16_driver_zero_dims() {
    let _g = lock();
    diff_driver("D16-h0", 2, 0, "", 3, 2, "1 2 3\n4 5 6\n");
    diff_driver("D16-w0", 0, 2, "x\ny", 3, 0, "");
    diff_driver("D16-wb0", 2, 2, "1 2\n3 4", 0, 2, "x\ny");
}

#[test]
fn cfg_d17_driver_messy_input() {
    let _g = lock();
    diff_driver(
        "D17",
        2,
        2,
        "  1    2   \n\n3 4\n99 99\n",
        2,
        2,
        "\n5 6 77\n7 8 88\n",
    );
}

#[test]
fn cfg_d18_driver_fuzz() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 0xD8);
    let mut skipped = 0usize;
    for i in 0..120 {
        let ha = rng.range(0, 6) as i32;
        let inner = rng.range(0, 6) as i32;
        let wb = rng.range(0, 6) as i32;
        let bound: i32 = *rng.pick(&[9, 999, 99_999]);
        let a: Vec<Vec<i32>> = (0..ha)
            .map(|_| (0..inner).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        let b: Vec<Vec<i32>> = (0..inner)
            .map(|_| (0..wb).map(|_| rng.i32_bounded(bound)).collect())
            .collect();
        // Guarded: a wrapped product below -999_999_999 renders as 11 chars and
        // overflows the C's own buffer (see the UB note in ERRORS.md).
        if !diff_driver_if_safe(
            &format!("D18#{i}"),
            inner,
            ha,
            &render(&a),
            wb,
            inner,
            &render(&b),
        ) {
            skipped += 1;
        }
    }
    assert!(skipped < 120, "D18: every case was skipped by the C-UB guard");
}

// ===========================================================================
// E. Cross-cutting
// ===========================================================================

#[test]
fn cfg_e1_allocate_free_round_trips() {
    let _g = lock();
    let shapes = [
        (0, 0),
        (0, 3),
        (3, 0),
        (1, 1),
        (5, 5),
        (9, 2),
        (2, 9),
        (16, 16),
    ];
    for _ in 0..25 {
        for (w, h) in shapes {
            diff_allocate("E1", w, h);
        }
    }
}

#[test]
fn cfg_e2_hand_built_matrix_t_layout() {
    let _g = lock();
    let p = pair();
    assert_eq!(
        std::mem::size_of::<MatrixT>(),
        16,
        "matrix_t must be 16 bytes on LP64"
    );

    // Build matrix_t values entirely on the test side (Rust `Vec` backing
    // store), i.e. NOT produced by either library, and feed them to both.
    let build = |vals: &Vec<Vec<c_int>>,
                 rows: &mut Vec<Vec<c_int>>,
                 ptrs: &mut Vec<*mut c_int>|
     -> MatrixT {
        rows.clear();
        for r in vals {
            rows.push(r.clone());
        }
        ptrs.clear();
        for r in rows.iter_mut() {
            ptrs.push(r.as_mut_ptr());
        }
        MatrixT {
            matrix: ptrs.as_mut_ptr(),
            width: vals.first().map(|r| r.len()).unwrap_or(0) as c_int,
            height: vals.len() as c_int,
        }
    };

    let a_vals: Vec<Vec<c_int>> = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let b_vals: Vec<Vec<c_int>> = vec![vec![7, 8], vec![9, 10], vec![11, 12]];

    let (mut ar, mut ap) = (Vec::new(), Vec::new());
    let (mut br, mut bp) = (Vec::new(), Vec::new());
    let mut a = build(&a_vals, &mut ar, &mut ap);
    let mut b = build(&b_vals, &mut br, &mut bp);

    let ap_a: *mut MatrixT = &raw mut a;
    let ap_b: *mut MatrixT = &raw mut b;
    let run = |imp: &Impl| unsafe {
        let res = imp.multiply_matrices(ap_a, ap_b);
        let snap = snapshot(res, true);
        let s = imp.matrix_to_string(res);
        let text = take_c_string(s);
        imp.free_matrix(res);
        (snap, text)
    };
    let (c_res, c_err) = capture_stderr(|| run(&p.c));
    let (r_res, r_err) = capture_stderr(|| run(&p.rs));
    assert_eq!(c_res.0, r_res.0, "E2 multiply on hand-built matrix_t diverged");
    assert_eq!(c_res.1, r_res.1, "E2 to_string on hand-built matrix_t diverged");
    assert_eq!(show(&c_err), show(&r_err), "E2 stderr diverged");
    // Sanity: the expected 2x2 product, proving the layout really was honoured.
    assert_eq!(
        show(c_res.1.as_ref().unwrap()),
        "58 64\n139 154\n",
        "E2 product wrong — struct layout mismatch would show up here"
    );

    // Also exercise matrix_to_string on a hand-built matrix directly.
    let mut only = build(&a_vals, &mut ar, &mut ap);
    let (c_txt, _) = capture_stderr(|| unsafe { take_c_string(p.c.matrix_to_string(&mut only)) });
    let (r_txt, _) = capture_stderr(|| unsafe { take_c_string(p.rs.matrix_to_string(&mut only)) });
    assert_eq!(c_txt, r_txt, "E2 direct to_string diverged");
    assert_eq!(show(c_txt.as_ref().unwrap()), "1 2 3\n4 5 6\n");
}

#[test]
fn cfg_e3_ownership_and_allocator_interop() {
    let _g = lock();
    let p = pair();
    let input = cs("1 2\n3 4\n");

    // A pointer returned by the Rust `.so` must be freeable by libc `free`
    // (i.e. it must come from libc `malloc`, like the C's). `take_c_string`
    // already does this; do a few round trips so a wrong allocator trips the
    // allocator's own consistency checks.
    for _ in 0..50 {
        for imp in [&p.c, &p.rs] {
            unsafe {
                let m = imp.initialize_matrix_from_string(input.as_ptr(), 2, 2);
                assert!(!m.is_null());
                let s = imp.matrix_to_string(m);
                let text = take_c_string(s).expect("non-null string");
                assert_eq!(show(&text), "1 2\n3 4\n");
                imp.free_matrix(m);
            }
        }
    }
}
