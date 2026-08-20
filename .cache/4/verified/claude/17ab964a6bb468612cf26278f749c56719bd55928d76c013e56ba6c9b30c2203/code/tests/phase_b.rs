//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row
//! (rows C1–C34; the `driver` rows C35–C39 live in `tests/driver.rs` because
//! they need control over the process' working directory).
//!
//! Every test loads BOTH shared libraries through `libloading` and compares
//! return values, matrix contents, produced strings, written files and the
//! bytes emitted on `stderr` byte-for-byte.

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::fs;

// ---------------------------------------------------------------------------
// small helpers: run the same call on both libraries and compare
// ---------------------------------------------------------------------------

struct Pair {
    c: *mut MatrixT,
    r: *mut MatrixT,
}

impl Pair {
    fn free(self) {
        let (c, r) = both();
        unsafe {
            (c.free_matrix)(self.c);
            (r.free_matrix)(self.r);
        }
    }
    fn is_null(&self) -> bool {
        self.c.is_null()
    }
}

/// `allocate_matrix` on both, comparing the observable struct state.
fn alloc_pair(width: c_int, height: c_int, ctx: &str) -> Pair {
    let (c, r) = both();
    let (cp, c_err) = capture_stderr(|| unsafe { (c.allocate_matrix)(width, height) });
    let (rp, r_err) = capture_stderr(|| unsafe { (r.allocate_matrix)(width, height) });
    let cs = unsafe { snap_matrix(cp, false) };
    let rs = unsafe { snap_matrix(rp, false) };
    assert_eq!(cs, rs, "allocate_matrix({width}, {height}) mismatch [{ctx}]");
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("allocate_matrix({width}, {height}) stderr mismatch [{ctx}]"),
    );
    Pair { c: cp, r: rp }
}

/// `initialize_matrix_from_string` on both, comparing contents + stderr.
fn init_pair(text: &CBuf, width: c_int, height: c_int, ctx: &str) -> Pair {
    let (cp, rp) = unsafe { init_both(text, width, height, ctx) };
    Pair { c: cp, r: rp }
}

fn init_pair_str(text: &str, width: c_int, height: c_int, ctx: &str) -> Pair {
    init_pair(&CBuf::new(text), width, height, ctx)
}

/// `multiply_matrices` on both, comparing contents + stderr.
fn mul_pair(a: &Pair, b: &Pair, ctx: &str) -> Pair {
    let (c, r) = both();
    let (cp, c_err) = capture_stderr(|| unsafe { (c.multiply_matrices)(a.c, b.c) });
    let (rp, r_err) = capture_stderr(|| unsafe { (r.multiply_matrices)(a.r, b.r) });
    let cs = unsafe { snap_matrix(cp, true) };
    let rs = unsafe { snap_matrix(rp, true) };
    assert_eq!(cs, rs, "multiply_matrices mismatch [{ctx}]");
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("multiply_matrices stderr mismatch [{ctx}]"),
    );
    Pair { c: cp, r: rp }
}

/// `matrix_to_string` on both, comparing the produced bytes + stderr.
fn to_string_pair(p: &Pair, ctx: &str) -> Option<Vec<u8>> {
    let (c, r) = both();
    let (cs_ptr, c_err) = capture_stderr(|| unsafe { (c.matrix_to_string)(p.c) });
    let (rs_ptr, r_err) = capture_stderr(|| unsafe { (r.matrix_to_string)(p.r) });
    let cs = unsafe { take_c_string(cs_ptr) };
    let rs = unsafe { take_c_string(rs_ptr) };
    assert_opt_bytes_eq(&cs, &rs, &format!("matrix_to_string mismatch [{ctx}]"));
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("matrix_to_string stderr mismatch [{ctx}]"),
    );
    cs
}

/// Writes every cell of both matrices and reads the values back, proving the
/// row allocations really have `width` usable ints in both libraries.
fn fill_and_verify(p: &Pair, rng: &mut Rng, ctx: &str) {
    if p.is_null() {
        return;
    }
    let (w, h) = unsafe { ((*p.c).width, (*p.c).height) };
    let values: Vec<Vec<i32>> = (0..h)
        .map(|_| (0..w).map(|_| rng.i32_in(-100_000, 100_000)).collect())
        .collect();
    for m in [p.c, p.r] {
        for i in 0..h {
            let row = unsafe { *(*m).matrix.offset(i as isize) };
            for j in 0..w {
                unsafe { *row.offset(j as isize) = values[i as usize][j as usize] };
            }
        }
    }
    let cs = unsafe { snap_matrix(p.c, true) };
    let rs = unsafe { snap_matrix(p.r, true) };
    assert_eq!(cs, rs, "allocated matrix read-back mismatch [{ctx}]");
    let flat: Vec<i32> = values.iter().flatten().copied().collect();
    assert_eq!(cs.cells, flat, "allocated matrix lost values [{ctx}]");
}

// ---------------------------------------------------------------------------
// C1–C6 — allocate_matrix / free_matrix
// ---------------------------------------------------------------------------

#[test]
fn c1_allocate_0x0() {
    let p = alloc_pair(0, 0, "C1");
    assert!(!p.is_null());
    unsafe {
        assert_eq!((*p.c).width, 0);
        assert_eq!((*p.c).height, 0);
        assert!(!(*p.c).matrix.is_null());
        assert!(!(*p.r).matrix.is_null());
    }
    p.free();
}

#[test]
fn c2_allocate_zero_width() {
    let mut rng = Rng::new(0xC2);
    for _ in 0..64 {
        let h = rng.i32_in(1, 32);
        let p = alloc_pair(0, h, "C2");
        assert!(!p.is_null());
        unsafe {
            for i in 0..h {
                assert!(!(*p.c).matrix.offset(i as isize).read().is_null());
                assert!(!(*p.r).matrix.offset(i as isize).read().is_null());
            }
        }
        p.free();
    }
}

#[test]
fn c3_allocate_zero_height() {
    let mut rng = Rng::new(0xC3);
    for _ in 0..64 {
        let w = rng.i32_in(0, 64);
        let p = alloc_pair(w, 0, "C3");
        assert!(!p.is_null());
        p.free();
    }
}

#[test]
fn c4_allocate_random_shapes() {
    let mut rng = Rng::new(0xC4);
    // 1x1, 1xN, Nx1, square and rectangular shapes.
    let mut shapes = vec![(1, 1)];
    for _ in 0..24 {
        let n = rng.i32_in(1, 64);
        shapes.push((1, n));
        shapes.push((n, 1));
        shapes.push((n, n));
        shapes.push((rng.i32_in(1, 64), rng.i32_in(1, 64)));
    }
    for (w, h) in shapes {
        let ctx = format!("C4 {w}x{h}");
        let p = alloc_pair(w, h, &ctx);
        assert!(!p.is_null());
        unsafe {
            assert_eq!((*p.c).width, w);
            assert_eq!((*p.c).height, h);
            assert_eq!((*p.r).width, w);
            assert_eq!((*p.r).height, h);
        }
        fill_and_verify(&p, &mut rng, &ctx);
        p.free();
    }
}

#[test]
fn c5_allocate_large_shape() {
    let mut rng = Rng::new(0xC5);
    for (w, h) in [(64, 64), (4096, 1), (1, 4096), (128, 32)] {
        let ctx = format!("C5 {w}x{h}");
        let p = alloc_pair(w, h, &ctx);
        assert!(!p.is_null());
        fill_and_verify(&p, &mut rng, &ctx);
        p.free();
    }
}

#[test]
fn c6_allocate_boundary_dims() {
    // {0, 1} x {0, 1} plus INT_MAX width (a single ~8 GiB request: whatever
    // malloc() decides, both libraries must decide the same).
    for (w, h) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let p = alloc_pair(w, h, "C6");
        assert!(!p.is_null());
        p.free();
    }

    let (c, r) = both();
    let (cp, c_err) = capture_stderr(|| unsafe { (c.allocate_matrix)(i32::MAX, 1) });
    let c_null = cp.is_null();
    unsafe { (c.free_matrix)(cp) };
    let (rp, r_err) = capture_stderr(|| unsafe { (r.allocate_matrix)(i32::MAX, 1) });
    let r_null = rp.is_null();
    unsafe { (r.free_matrix)(rp) };
    assert_eq!(c_null, r_null, "allocate_matrix(INT_MAX, 1) NULL-ness mismatch");
    assert_bytes_eq(&c_err, &r_err, "allocate_matrix(INT_MAX, 1) stderr mismatch");
}

// ---------------------------------------------------------------------------
// C7–C18 — initialize_matrix_from_string
// ---------------------------------------------------------------------------

#[test]
fn c7_init_1x1_random() {
    let mut rng = Rng::new(0xC7);
    for _ in 0..300 {
        let v = rng.i32_in(-999_999_999, 999_999_999);
        let text = format!("{v}\n");
        let p = init_pair_str(&text, 1, 1, &format!("C7 {v}"));
        assert!(!p.is_null());
        assert_eq!(unsafe { snap_matrix(p.c, true) }.cells, vec![v]);
        let s = to_string_pair(&p, "C7").unwrap();
        assert_eq!(s, format!("{v}\n").into_bytes());
        p.free();
    }
}

#[test]
fn c8_init_random_shapes() {
    let mut rng = Rng::new(0xC8);
    for _ in 0..200 {
        let w = rng.i32_in(1, 12);
        let h = rng.i32_in(1, 12);
        let values = random_values(&mut rng, w, h, -999_999_999, 999_999_999);
        let text = matrix_text(&values);
        let ctx = format!("C8 {w}x{h}");
        let p = init_pair_str(&text, w, h, &ctx);
        assert!(!p.is_null());
        let flat: Vec<i32> = values.iter().flatten().copied().collect();
        assert_eq!(unsafe { snap_matrix(p.c, true) }.cells, flat, "{ctx}");
        let s = to_string_pair(&p, &ctx).unwrap();
        assert_eq!(s, text.as_bytes(), "{ctx}");
        p.free();
    }
}

#[test]
fn c9_init_single_column() {
    let mut rng = Rng::new(0xC9);
    for _ in 0..100 {
        let h = rng.i32_in(1, 20);
        let values = random_values(&mut rng, 1, h, -50_000, 50_000);
        let text = matrix_text(&values);
        let p = init_pair_str(&text, 1, h, "C9");
        assert!(!p.is_null());
        let s = to_string_pair(&p, "C9").unwrap();
        assert_eq!(s, text.as_bytes());
        assert!(!s.contains(&b' '), "single column must not contain spaces");
        p.free();
    }
}

#[test]
fn c10_init_single_row() {
    let mut rng = Rng::new(0xCA);
    for _ in 0..100 {
        let w = rng.i32_in(1, 20);
        let values = random_values(&mut rng, w, 1, -50_000, 50_000);
        let text = matrix_text(&values);
        let p = init_pair_str(&text, w, 1, "C10");
        assert!(!p.is_null());
        let s = to_string_pair(&p, "C10").unwrap();
        assert_eq!(s, text.as_bytes());
        p.free();
    }
}

#[test]
fn c11_init_zero_height() {
    // height == 0 ⇒ the input is never tokenised at all.
    for text in ["", "\n", "1 2 3\n", "garbage", " ", "\n\n\n"] {
        for w in [0, 1, 7] {
            let ctx = format!("C11 w={w} text={text:?}");
            let p = init_pair_str(text, w, 0, &ctx);
            assert!(!p.is_null(), "{ctx}");
            let s = to_string_pair(&p, &ctx).unwrap();
            assert_eq!(s, b"", "{ctx}");
            p.free();
        }
    }
}

#[test]
fn c12_init_zero_width() {
    // width == 0, height > 0 ⇒ rows are tokenised, no cells are stored.
    for (text, h) in [("a\nb\nc\n", 3), ("1\n", 1), ("x\ny\n", 2)] {
        let ctx = format!("C12 {text:?} h={h}");
        let p = init_pair_str(text, 0, h, &ctx);
        assert!(!p.is_null(), "{ctx}");
        let s = to_string_pair(&p, &ctx).unwrap();
        assert_eq!(s, "\n".repeat(h as usize).as_bytes(), "{ctx}");
        p.free();
    }
}

#[test]
fn c13_init_extra_rows_and_columns() {
    let mut rng = Rng::new(0xCD);
    for _ in 0..150 {
        let w = rng.i32_in(1, 6);
        let h = rng.i32_in(1, 6);
        let extra_rows = rng.i32_in(0, 4);
        let extra_cols = rng.i32_in(0, 4);
        let values = random_values(&mut rng, w + extra_cols, h + extra_rows, -1000, 1000);
        let text = matrix_text(&values);
        let ctx = format!("C13 {w}x{h} +{extra_cols}c +{extra_rows}r");
        let p = init_pair_str(&text, w, h, &ctx);
        assert!(!p.is_null(), "{ctx}");
        let expect: Vec<i32> = values
            .iter()
            .take(h as usize)
            .flat_map(|row| row.iter().take(w as usize).copied())
            .collect();
        assert_eq!(unsafe { snap_matrix(p.c, true) }.cells, expect, "{ctx}");
        to_string_pair(&p, &ctx).unwrap();
        p.free();
    }
}

#[test]
fn c14_init_whitespace_runs() {
    let mut rng = Rng::new(0xCE);
    for _ in 0..150 {
        let w = rng.i32_in(1, 5);
        let h = rng.i32_in(1, 5);
        let values = random_values(&mut rng, w, h, -1000, 1000);
        let mut text = String::new();
        for row in &values {
            if rng.bool() {
                text.push_str(&" ".repeat(rng.range(1, 3) as usize)); // leading spaces
            }
            for (i, v) in row.iter().enumerate() {
                if i > 0 {
                    text.push_str(&" ".repeat(rng.range(1, 4) as usize));
                }
                text.push_str(&v.to_string());
            }
            if rng.bool() {
                text.push_str(&" ".repeat(rng.range(1, 3) as usize)); // trailing spaces
            }
            text.push('\n');
        }
        let ctx = format!("C14 {w}x{h} {text:?}");
        let p = init_pair_str(&text, w, h, &ctx);
        assert!(!p.is_null(), "{ctx}");
        let flat: Vec<i32> = values.iter().flatten().copied().collect();
        assert_eq!(unsafe { snap_matrix(p.c, true) }.cells, flat, "{ctx}");
        to_string_pair(&p, &ctx).unwrap();
        p.free();
    }
}

#[test]
fn c15_init_blank_lines() {
    let mut rng = Rng::new(0xCF);
    for _ in 0..150 {
        let w = rng.i32_in(1, 4);
        let h = rng.i32_in(1, 4);
        let values = random_values(&mut rng, w, h, -1000, 1000);
        let mut text = String::new();
        if rng.bool() {
            text.push_str(&"\n".repeat(rng.range(1, 3) as usize)); // leading newlines
        }
        for row in &values {
            let cols: Vec<String> = row.iter().map(|v| v.to_string()).collect();
            text.push_str(&cols.join(" "));
            text.push_str(&"\n".repeat(rng.range(1, 3) as usize)); // blank lines
        }
        let ctx = format!("C15 {w}x{h} {text:?}");
        let p = init_pair_str(&text, w, h, &ctx);
        assert!(!p.is_null(), "{ctx}");
        let flat: Vec<i32> = values.iter().flatten().copied().collect();
        assert_eq!(unsafe { snap_matrix(p.c, true) }.cells, flat, "{ctx}");
        to_string_pair(&p, &ctx).unwrap();
        p.free();
    }
}

#[test]
fn c16_init_crlf_and_tabs() {
    // (text, width, height, expected cells or None when C rejects the input)
    let cases: &[(&str, i32, i32, Option<&[i32]>)] = &[
        // '\r' stays inside the last token of a line; atoi() stops at it
        ("1 2\r\n3 4\r\n", 2, 2, Some(&[1, 2, 3, 4])),
        // atoi() skips leading whitespace, including '\t'
        ("\t5 \t6\n\t7 8\n", 2, 2, Some(&[5, 6, 7, 8])),
        // "1\t2" is ONE space-delimited token ⇒ only 2 of 3 columns exist
        ("1\t2 3\n", 3, 1, None),
        // no '\n' at all ⇒ a single row token; "22\r33" parses as 22
        ("11 22\r33 44\r", 2, 1, Some(&[11, 22])),
        ("-1\r\n", 1, 1, Some(&[-1])),
        ("\t\t9\n", 1, 1, Some(&[9])),
    ];
    for (text, w, h, expected) in cases {
        let ctx = format!("C16 {text:?} {w}x{h}");
        let p = init_pair_str(text, *w, *h, &ctx);
        match expected {
            Some(cells) => {
                assert!(!p.is_null(), "{ctx}: expected success");
                assert_eq!(
                    unsafe { snap_matrix(p.c, true) }.cells,
                    cells.to_vec(),
                    "{ctx}"
                );
                to_string_pair(&p, &ctx).unwrap();
            }
            None => assert!(p.is_null(), "{ctx}: expected NULL"),
        }
        p.free();
    }
}

#[test]
fn c17_init_non_numeric_tokens() {
    // fixed cases with the exact `atoi()` results the C library produces
    let fixed: &[(&str, i32)] = &[
        ("abc", 0),
        ("-", 0),
        ("+", 0),
        (".", 0),
        ("12abc", 12),
        ("3.7", 3),
        ("0x1f", 0),
        ("--5", 0),
        ("1e3", 1),
        ("#", 0),
        ("z9", 0),
        ("-12x", -12),
    ];
    for (tok, expect) in fixed {
        let ctx = format!("C17 fixed {tok:?}");
        let p = init_pair_str(&format!("{tok}\n"), 1, 1, &ctx);
        assert!(!p.is_null(), "{ctx}");
        assert_eq!(
            unsafe { snap_matrix(p.c, true) }.cells,
            vec![*expect],
            "{ctx}"
        );
        assert_eq!(
            to_string_pair(&p, &ctx).unwrap(),
            format!("{expect}\n").into_bytes(),
            "{ctx}"
        );
        p.free();
    }

    let tokens = [
        "abc", "-", "+", ".", "12abc", "3.7", "0x1f", "--5", "1e3", "#", "z9", "",
    ];
    let mut rng = Rng::new(0xD1);
    for _ in 0..200 {
        let w = rng.i32_in(1, 4);
        let h = rng.i32_in(1, 4);
        let mut text = String::new();
        for _ in 0..h {
            let mut cols: Vec<String> = Vec::new();
            for _ in 0..w {
                if rng.bool() {
                    cols.push(rng.pick(&tokens).to_string());
                } else {
                    cols.push(rng.i32_in(-1000, 1000).to_string());
                }
            }
            text.push_str(&cols.join(" "));
            text.push('\n');
        }
        let ctx = format!("C17 {w}x{h} {text:?}");
        let p = init_pair_str(&text, w, h, &ctx);
        if !p.is_null() {
            to_string_pair(&p, &ctx).unwrap();
        }
        p.free();
    }
}

#[test]
fn c18_init_atoi_range_tokens() {
    let tokens = [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "99999999999999999999",
        "-99999999999999999999",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "000123",
        "  +42",
        "+0",
        "-0",
        "0",
    ];
    for t in tokens {
        let ctx = format!("C18 {t:?}");
        let text = format!("{t}\n");
        let p = init_pair_str(&text, 1, 1, &ctx);
        assert!(!p.is_null(), "{ctx}");
        to_string_pair(&p, &ctx).unwrap();
        p.free();
    }
    // ... and the same tokens mixed into wider rows.
    let mut rng = Rng::new(0xD2);
    for _ in 0..100 {
        let w = rng.i32_in(1, 3);
        let h = rng.i32_in(1, 3);
        let mut text = String::new();
        for _ in 0..h {
            let cols: Vec<String> = (0..w).map(|_| rng.pick(&tokens).trim().to_string()).collect();
            text.push_str(&cols.join(" "));
            text.push('\n');
        }
        let ctx = format!("C18 mixed {text:?}");
        let p = init_pair_str(&text, w, h, &ctx);
        assert!(!p.is_null(), "{ctx}");
        p.free();
    }
}

// ---------------------------------------------------------------------------
// C19–C23 — multiply_matrices
// ---------------------------------------------------------------------------

#[test]
fn c19_multiply_1x1() {
    let mut rng = Rng::new(0xD3);
    for _ in 0..300 {
        let a = rng.i32_in(-40_000, 40_000);
        let b = rng.i32_in(-40_000, 40_000);
        let pa = init_pair_str(&format!("{a}\n"), 1, 1, "C19");
        let pb = init_pair_str(&format!("{b}\n"), 1, 1, "C19");
        let res = mul_pair(&pa, &pb, "C19");
        assert!(!res.is_null());
        assert_eq!(
            unsafe { snap_matrix(res.c, true) }.cells,
            vec![a.wrapping_mul(b)]
        );
        to_string_pair(&res, "C19").unwrap();
        res.free();
        pa.free();
        pb.free();
    }
}

#[test]
fn c20_multiply_random_shapes() {
    let mut rng = Rng::new(0xD4);
    for _ in 0..200 {
        let m = rng.i32_in(1, 10);
        let k = rng.i32_in(1, 10);
        let n = rng.i32_in(1, 10);
        let av = random_values(&mut rng, k, m, -1000, 1000);
        let bv = random_values(&mut rng, n, k, -1000, 1000);
        let ctx = format!("C20 {m}x{k} * {k}x{n}");
        let pa = init_pair_str(&matrix_text(&av), k, m, &ctx);
        let pb = init_pair_str(&matrix_text(&bv), n, k, &ctx);
        let res = mul_pair(&pa, &pb, &ctx);
        assert!(!res.is_null(), "{ctx}");
        // independent reference product
        let mut expect = Vec::new();
        for i in 0..m as usize {
            for j in 0..n as usize {
                let mut acc: i32 = 0;
                for kk in 0..k as usize {
                    acc = acc.wrapping_add(av[i][kk].wrapping_mul(bv[kk][j]));
                }
                expect.push(acc);
            }
        }
        assert_eq!(unsafe { snap_matrix(res.c, true) }.cells, expect, "{ctx}");
        to_string_pair(&res, &ctx).unwrap();
        res.free();
        pa.free();
        pb.free();
    }
}

#[test]
fn c21_multiply_inner_dim_zero() {
    let mut rng = Rng::new(0xD5);
    for _ in 0..40 {
        let m = rng.i32_in(1, 6);
        let n = rng.i32_in(1, 6);
        let ctx = format!("C21 {m}x0 * 0x{n}");
        // A: width 0, height m (needs m row tokens); B: height 0, width n.
        let pa = init_pair_str(&"x\n".repeat(m as usize), 0, m, &ctx);
        let pb = init_pair_str("", n, 0, &ctx);
        let res = mul_pair(&pa, &pb, &ctx);
        assert!(!res.is_null(), "{ctx}");
        let snap = unsafe { snap_matrix(res.c, true) };
        assert_eq!(snap.width, n);
        assert_eq!(snap.height, m);
        assert!(snap.cells.iter().all(|&v| v == 0), "{ctx}: {snap:?}");
        to_string_pair(&res, &ctx).unwrap();
        res.free();
        pa.free();
        pb.free();
    }
}

#[test]
fn c22_multiply_empty_outer_dims() {
    let mut rng = Rng::new(0xD6);
    for _ in 0..40 {
        let k = rng.i32_in(1, 5);
        let n = rng.i32_in(1, 5);
        // m == 0: A has no rows.
        let ctx = format!("C22 0x{k} * {k}x{n}");
        let pa = init_pair_str("", k, 0, &ctx);
        let bv = random_values(&mut rng, n, k, -100, 100);
        let pb = init_pair_str(&matrix_text(&bv), n, k, &ctx);
        let res = mul_pair(&pa, &pb, &ctx);
        assert!(!res.is_null(), "{ctx}");
        assert_eq!(to_string_pair(&res, &ctx).unwrap(), b"", "{ctx}");
        res.free();
        pa.free();
        pb.free();

        // n == 0: B has no columns.
        let m = rng.i32_in(1, 5);
        let ctx = format!("C22 {m}x{k} * {k}x0");
        let av = random_values(&mut rng, k, m, -100, 100);
        let pa = init_pair_str(&matrix_text(&av), k, m, &ctx);
        let pb = init_pair_str(&"y\n".repeat(k as usize), 0, k, &ctx);
        let res = mul_pair(&pa, &pb, &ctx);
        assert!(!res.is_null(), "{ctx}");
        assert_eq!(
            to_string_pair(&res, &ctx).unwrap(),
            "\n".repeat(m as usize).as_bytes(),
            "{ctx}"
        );
        res.free();
        pa.free();
        pb.free();
    }
}

#[test]
fn c23_multiply_wrapping_products() {
    let mut rng = Rng::new(0xD7);
    let big = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        1_000_000_000,
        -1_000_000_000,
        65_536,
        -65_537,
        2_147_483_647,
    ];
    for _ in 0..150 {
        let m = rng.i32_in(1, 4);
        let k = rng.i32_in(1, 4);
        let n = rng.i32_in(1, 4);
        let av: Vec<Vec<i32>> = (0..m)
            .map(|_| (0..k).map(|_| *rng.pick(&big)).collect())
            .collect();
        let bv: Vec<Vec<i32>> = (0..k)
            .map(|_| (0..n).map(|_| *rng.pick(&big)).collect())
            .collect();
        let ctx = format!("C23 {m}x{k} * {k}x{n}");
        let pa = init_pair_str(&matrix_text(&av), k, m, &ctx);
        let pb = init_pair_str(&matrix_text(&bv), n, k, &ctx);
        let res = mul_pair(&pa, &pb, &ctx);
        assert!(!res.is_null(), "{ctx}");
        let mut expect = Vec::new();
        for i in 0..m as usize {
            for j in 0..n as usize {
                let mut acc: i32 = 0;
                for kk in 0..k as usize {
                    acc = acc.wrapping_add(av[i][kk].wrapping_mul(bv[kk][j]));
                }
                expect.push(acc);
            }
        }
        assert_eq!(unsafe { snap_matrix(res.c, true) }.cells, expect, "{ctx}");
        res.free();
        pa.free();
        pb.free();
    }
}

// ---------------------------------------------------------------------------
// C24–C26 — matrix_to_string
// ---------------------------------------------------------------------------

#[test]
fn c24_to_string_degenerate_shapes() {
    // height == 0 ⇒ ""
    for w in [0, 1, 5] {
        let p = init_pair_str("", w, 0, "C24 h=0");
        assert_eq!(to_string_pair(&p, "C24 h=0").unwrap(), b"");
        p.free();
    }
    // width == 0, height > 0 ⇒ h bare newlines
    for h in [1, 2, 5] {
        let p = init_pair_str(&"r\n".repeat(h), 0, h as c_int, "C24 w=0");
        assert_eq!(
            to_string_pair(&p, "C24 w=0").unwrap(),
            "\n".repeat(h).as_bytes()
        );
        p.free();
    }
    // 1x1, 1xN, Nx1
    let p = init_pair_str("42\n", 1, 1, "C24 1x1");
    assert_eq!(to_string_pair(&p, "C24 1x1").unwrap(), b"42\n");
    p.free();
    let p = init_pair_str("1\n2\n3\n", 1, 3, "C24 1x3");
    assert_eq!(to_string_pair(&p, "C24 1x3").unwrap(), b"1\n2\n3\n");
    p.free();
    let p = init_pair_str("1 2 3\n", 3, 1, "C24 3x1");
    assert_eq!(to_string_pair(&p, "C24 3x1").unwrap(), b"1 2 3\n");
    p.free();
}

#[test]
fn c25_to_string_mixed_widths() {
    let mut rng = Rng::new(0xD8);
    let magnitudes = [1, 9, 10, 99, 100, 12_345, 999_999, 999_999_999];
    for _ in 0..200 {
        let w = rng.i32_in(1, 8);
        let h = rng.i32_in(1, 8);
        let values: Vec<Vec<i32>> = (0..h)
            .map(|_| {
                (0..w)
                    .map(|_| {
                        let m = *rng.pick(&magnitudes);
                        let v = rng.i32_in(0, m);
                        if rng.bool() {
                            -v
                        } else {
                            v
                        }
                    })
                    .collect()
            })
            .collect();
        let text = matrix_text(&values);
        let ctx = format!("C25 {w}x{h}");
        let p = init_pair_str(&text, w, h, &ctx);
        assert!(!p.is_null(), "{ctx}");
        assert_eq!(to_string_pair(&p, &ctx).unwrap(), text.as_bytes(), "{ctx}");
        p.free();
    }
}

#[test]
fn c26_to_string_11_char_values() {
    // 11-character renderings exactly fill the C buffer when width == 1.
    for v in [i32::MIN, -1_000_000_000, -2_000_000_000, i32::MAX] {
        let ctx = format!("C26 {v}");
        let p = init_pair_str(&format!("{v}\n"), 1, 1, &ctx);
        assert!(!p.is_null(), "{ctx}");
        assert_eq!(
            to_string_pair(&p, &ctx).unwrap(),
            format!("{v}\n").into_bytes(),
            "{ctx}"
        );
        p.free();
    }
    // multi-row single-column variant
    let text = format!("{}\n{}\n{}\n", i32::MIN, i32::MAX, -1_000_000_000);
    let p = init_pair_str(&text, 1, 3, "C26 3 rows");
    assert_eq!(to_string_pair(&p, "C26 3 rows").unwrap(), text.as_bytes());
    p.free();
}

// ---------------------------------------------------------------------------
// C27–C33 — write_to_file
// ---------------------------------------------------------------------------

/// Runs `write_to_file` on both libraries (into two distinct files) and
/// compares return codes, stderr and the resulting file bytes.
fn write_both(content: &CBuf, ctx: &str) -> c_int {
    let (c, r) = both();
    let dir = unique_dir("write");
    let cf = dir.join("c.out");
    let rf = dir.join("r.out");
    let cpath = path_cbuf(&cf);
    let rpath = path_cbuf(&rf);
    let (crc, c_err) = capture_stderr(|| unsafe { (c.write_to_file)(cpath.as_ptr(), content.as_ptr()) });
    let (rrc, r_err) = capture_stderr(|| unsafe { (r.write_to_file)(rpath.as_ptr(), content.as_ptr()) });
    assert_eq!(crc, rrc, "write_to_file return mismatch [{ctx}]");
    let cb = fs::read(&cf).ok();
    let rb = fs::read(&rf).ok();
    assert_eq!(cb, rb, "write_to_file file content mismatch [{ctx}]");
    // stderr messages embed the (different) file names, so only compare when
    // the call succeeded (no message at all) — error-path messages are checked
    // in Phase C with identical file names.
    if crc == 0 {
        assert_eq!(c_err.len(), 0, "unexpected stderr from C [{ctx}]");
        assert_eq!(r_err.len(), 0, "unexpected stderr from Rust [{ctx}]");
    }
    crc
}

#[test]
fn c27_write_simple() {
    assert_eq!(write_both(&CBuf::new("hello world\n"), "C27"), 0);
    let mut rng = Rng::new(0xD9);
    for _ in 0..50 {
        let n = rng.range(1, 200) as usize;
        let body: String = (0..n)
            .map(|_| (b'a' + (rng.next_u64() % 26) as u8) as char)
            .collect();
        assert_eq!(write_both(&CBuf::new(body), "C27 random"), 0);
    }
}

#[test]
fn c28_write_empty_content() {
    assert_eq!(write_both(&CBuf::new(""), "C28"), 0);
}

#[test]
fn c29_write_special_bytes() {
    let cases: Vec<Vec<u8>> = vec![
        b"line1\nline2\nline3\n".to_vec(),
        b"100% sure: %s %d %% %n\n".to_vec(),
        b"tab\there\r\nvertical\x0bform\x0c".to_vec(),
        (1u8..=255).collect(),
        vec![0xff, 0xfe, 0x80, 0x0a],
        "unicode: \u{263A} \u{1F600}".as_bytes().to_vec(),
    ];
    for (i, case) in cases.iter().enumerate() {
        assert_eq!(write_both(&CBuf::new(case.clone()), &format!("C29 #{i}")), 0);
    }
}

#[test]
fn c30_write_one_mib() {
    let content: Vec<u8> = (0..1024 * 1024).map(|i| b'a' + (i % 26) as u8).collect();
    assert_eq!(write_both(&CBuf::new(content), "C30"), 0);
}

#[test]
fn c31_write_truncates_existing() {
    let (c, r) = both();
    let dir = unique_dir("trunc");
    let cf = dir.join("c.out");
    let rf = dir.join("r.out");
    fs::write(&cf, "X".repeat(4096)).unwrap();
    fs::write(&rf, "X".repeat(4096)).unwrap();
    let content = CBuf::new("short\n");
    let cpath = path_cbuf(&cf);
    let rpath = path_cbuf(&rf);
    let crc = unsafe { (c.write_to_file)(cpath.as_ptr(), content.as_ptr()) };
    let rrc = unsafe { (r.write_to_file)(rpath.as_ptr(), content.as_ptr()) };
    assert_eq!(crc, 0);
    assert_eq!(rrc, 0);
    assert_eq!(fs::read(&cf).unwrap(), b"short\n");
    assert_eq!(fs::read(&cf).unwrap(), fs::read(&rf).unwrap());
}

#[test]
fn c32_write_dev_null() {
    let (c, r) = both();
    let path = CBuf::new("/dev/null");
    let content = CBuf::new("anything\n");
    let (crc, c_err) = capture_stderr(|| unsafe { (c.write_to_file)(path.as_ptr(), content.as_ptr()) });
    let (rrc, r_err) = capture_stderr(|| unsafe { (r.write_to_file)(path.as_ptr(), content.as_ptr()) });
    assert_eq!(crc, rrc, "/dev/null return mismatch");
    assert_eq!(crc, 0);
    assert_bytes_eq(&c_err, &r_err, "/dev/null stderr mismatch");
}

#[test]
fn c33_write_nested_random_paths() {
    let (c, r) = both();
    let mut rng = Rng::new(0xDA);
    for i in 0..30 {
        let dir = unique_dir("nested").join(format!("a{i}/b/c"));
        fs::create_dir_all(&dir).unwrap();
        let name: String = (0..rng.range(1, 12))
            .map(|_| (b'A' + (rng.next_u64() % 26) as u8) as char)
            .collect();
        let cf = dir.join(format!("c-{name}.txt"));
        let rf = dir.join(format!("r-{name}.txt"));
        let body: String = (0..rng.range(0, 500))
            .map(|_| (b' ' + (rng.next_u64() % 90) as u8) as char)
            .collect();
        let content = CBuf::new(body);
        let cpath = path_cbuf(&cf);
        let rpath = path_cbuf(&rf);
        let crc = unsafe { (c.write_to_file)(cpath.as_ptr(), content.as_ptr()) };
        let rrc = unsafe { (r.write_to_file)(rpath.as_ptr(), content.as_ptr()) };
        assert_eq!(crc, rrc);
        assert_eq!(crc, 0);
        assert_eq!(fs::read(&cf).unwrap(), fs::read(&rf).unwrap());
    }
}

// ---------------------------------------------------------------------------
// C34 — the whole pipeline, driven from the low-level exports by hand
// ---------------------------------------------------------------------------

#[test]
fn c34_manual_pipeline() {
    let (c, r) = both();
    let mut rng = Rng::new(0xDB);
    for iter in 0..60 {
        let m = rng.i32_in(1, 8);
        let k = rng.i32_in(1, 8);
        let n = rng.i32_in(1, 8);
        let av = random_values(&mut rng, k, m, -5000, 5000);
        let bv = random_values(&mut rng, n, k, -5000, 5000);
        let ctx = format!("C34 #{iter} {m}x{k}*{k}x{n}");

        let pa = init_pair_str(&matrix_text(&av), k, m, &ctx);
        let pb = init_pair_str(&matrix_text(&bv), n, k, &ctx);
        let res = mul_pair(&pa, &pb, &ctx);
        assert!(!res.is_null(), "{ctx}");

        // matrix_to_string on each side separately (pointers are owned by the
        // library that produced them).
        let c_str = unsafe { (c.matrix_to_string)(res.c) };
        let r_str = unsafe { (r.matrix_to_string)(res.r) };
        assert!(!c_str.is_null() && !r_str.is_null(), "{ctx}");

        let dir = unique_dir("pipeline");
        let cf = dir.join("c.txt");
        let rf = dir.join("r.txt");
        let cpath = path_cbuf(&cf);
        let rpath = path_cbuf(&rf);
        let crc = unsafe { (c.write_to_file)(cpath.as_ptr(), c_str) };
        let rrc = unsafe { (r.write_to_file)(rpath.as_ptr(), r_str) };
        assert_eq!(crc, 0, "{ctx}");
        assert_eq!(rrc, 0, "{ctx}");

        let cb = fs::read(&cf).unwrap();
        let rb = fs::read(&rf).unwrap();
        assert_eq!(cb, rb, "{ctx}: written bytes differ");

        // and an independent reference rendering
        let mut expect = String::new();
        for i in 0..m as usize {
            let mut cols = Vec::new();
            for j in 0..n as usize {
                let mut acc: i32 = 0;
                for kk in 0..k as usize {
                    acc = acc.wrapping_add(av[i][kk].wrapping_mul(bv[kk][j]));
                }
                cols.push(acc.to_string());
            }
            expect.push_str(&cols.join(" "));
            expect.push('\n');
        }
        assert_eq!(String::from_utf8_lossy(&cb), expect, "{ctx}");

        unsafe {
            free(c_str as *mut c_void);
            free(r_str as *mut c_void);
        }
        res.free();
        pa.free();
        pb.free();
    }
}

// ---------------------------------------------------------------------------
// Additional valid-path coverage: larger shapes, huge tokens, symlink targets
// and a Rust-first ordering (guards against results that only match because
// the C library ran first and left global state behind).
// ---------------------------------------------------------------------------

#[test]
fn c8b_init_large_shapes() {
    let mut rng = Rng::new(0x8B);
    for (w, h) in [(40, 40), (100, 7), (7, 100), (1, 500), (500, 1)] {
        let values = random_values(&mut rng, w, h, -999_999_999, 999_999_999);
        let text = matrix_text(&values);
        let ctx = format!("C8b {w}x{h}");
        let p = init_pair_str(&text, w, h, &ctx);
        assert!(!p.is_null(), "{ctx}");
        let flat: Vec<i32> = values.iter().flatten().copied().collect();
        assert_eq!(unsafe { snap_matrix(p.c, true) }.cells, flat, "{ctx}");
        assert_eq!(to_string_pair(&p, &ctx).unwrap(), text.as_bytes(), "{ctx}");
        p.free();
    }
}

#[test]
fn c20b_multiply_large_shapes() {
    let mut rng = Rng::new(0x20B);
    for (m, k, n) in [(32, 32, 32), (1, 64, 1), (64, 1, 64), (17, 5, 23)] {
        let av = random_values(&mut rng, k, m, -1000, 1000);
        let bv = random_values(&mut rng, n, k, -1000, 1000);
        let ctx = format!("C20b {m}x{k}*{k}x{n}");
        let pa = init_pair_str(&matrix_text(&av), k, m, &ctx);
        let pb = init_pair_str(&matrix_text(&bv), n, k, &ctx);
        let res = mul_pair(&pa, &pb, &ctx);
        assert!(!res.is_null(), "{ctx}");
        let mut expect = Vec::new();
        for i in 0..m as usize {
            for j in 0..n as usize {
                let mut acc: i32 = 0;
                for kk in 0..k as usize {
                    acc = acc.wrapping_add(av[i][kk].wrapping_mul(bv[kk][j]));
                }
                expect.push(acc);
            }
        }
        assert_eq!(unsafe { snap_matrix(res.c, true) }.cells, expect, "{ctx}");
        to_string_pair(&res, &ctx).unwrap();
        res.free();
        pa.free();
        pb.free();
    }
}

#[test]
fn c18b_init_huge_tokens() {
    // Tokens far longer than any int: glibc atoi() = (int)strtol() saturation.
    let cases = [
        "9".repeat(5000),
        format!("-{}", "9".repeat(5000)),
        format!("{}1", "0".repeat(100)),
        format!("+{}", "1".repeat(30)),
        format!("{}{}", " ".repeat(0), "12345678901234567890"),
    ];
    for t in &cases {
        let ctx = format!("C18b len={}", t.len());
        let p = init_pair_str(&format!("{t}\n"), 1, 1, &ctx);
        assert!(!p.is_null(), "{ctx}");
        to_string_pair(&p, &ctx).unwrap();
        p.free();
    }
}

#[test]
fn c31b_write_through_symlink() {
    let (c, r) = both();
    let dir = unique_dir("symlink");
    let c_target = dir.join("c-target.txt");
    let r_target = dir.join("r-target.txt");
    fs::write(&c_target, "old").unwrap();
    fs::write(&r_target, "old").unwrap();
    let c_link = dir.join("c-link");
    let r_link = dir.join("r-link");
    std::os::unix::fs::symlink(&c_target, &c_link).unwrap();
    std::os::unix::fs::symlink(&r_target, &r_link).unwrap();
    let content = CBuf::new("through the link\n");
    let cl = path_cbuf(&c_link);
    let rl = path_cbuf(&r_link);
    let crc = unsafe { (c.write_to_file)(cl.as_ptr(), content.as_ptr()) };
    let rrc = unsafe { (r.write_to_file)(rl.as_ptr(), content.as_ptr()) };
    assert_eq!(crc, rrc);
    assert_eq!(crc, 0);
    assert_eq!(fs::read(&c_target).unwrap(), b"through the link\n");
    assert_eq!(fs::read(&c_target).unwrap(), fs::read(&r_target).unwrap());
    // dangling symlink ⇒ the target is created
    let c_dangling = dir.join("c-dangling");
    let r_dangling = dir.join("r-dangling");
    std::os::unix::fs::symlink(dir.join("c-created.txt"), &c_dangling).unwrap();
    std::os::unix::fs::symlink(dir.join("r-created.txt"), &r_dangling).unwrap();
    let cd = path_cbuf(&c_dangling);
    let rd = path_cbuf(&r_dangling);
    let crc = unsafe { (c.write_to_file)(cd.as_ptr(), content.as_ptr()) };
    let rrc = unsafe { (r.write_to_file)(rd.as_ptr(), content.as_ptr()) };
    assert_eq!(crc, rrc);
    assert_eq!(
        fs::read(dir.join("c-created.txt")).unwrap(),
        fs::read(dir.join("r-created.txt")).unwrap()
    );
}

#[test]
fn c34b_pipeline_rust_first() {
    // Same as C34, but the RUST library is driven first in every step.
    let (c, r) = both();
    let mut rng = Rng::new(0x34B);
    for iter in 0..40 {
        let m = rng.i32_in(1, 8);
        let k = rng.i32_in(1, 8);
        let n = rng.i32_in(1, 8);
        let av = random_values(&mut rng, k, m, -5000, 5000);
        let bv = random_values(&mut rng, n, k, -5000, 5000);
        let a_text = CBuf::new(matrix_text(&av));
        let b_text = CBuf::new(matrix_text(&bv));
        let ctx = format!("C34b #{iter} {m}x{k}*{k}x{n}");

        let ra = unsafe { (r.initialize_matrix_from_string)(a_text.as_ptr(), k, m) };
        let rb = unsafe { (r.initialize_matrix_from_string)(b_text.as_ptr(), n, k) };
        let rres = unsafe { (r.multiply_matrices)(ra, rb) };
        let rstr = unsafe { (r.matrix_to_string)(rres) };

        let ca = unsafe { (c.initialize_matrix_from_string)(a_text.as_ptr(), k, m) };
        let cb = unsafe { (c.initialize_matrix_from_string)(b_text.as_ptr(), n, k) };
        let cres = unsafe { (c.multiply_matrices)(ca, cb) };
        let cstr = unsafe { (c.matrix_to_string)(cres) };

        assert!(!rstr.is_null() && !cstr.is_null(), "{ctx}");
        let rb_bytes = unsafe { take_c_string(rstr) }.unwrap();
        let cb_bytes = unsafe { take_c_string(cstr) }.unwrap();
        assert_bytes_eq(&cb_bytes, &rb_bytes, &ctx);
        let expect = {
            let mut s = String::new();
            for i in 0..m as usize {
                let mut cols = Vec::new();
                for j in 0..n as usize {
                    let mut acc: i32 = 0;
                    for kk in 0..k as usize {
                        acc = acc.wrapping_add(av[i][kk].wrapping_mul(bv[kk][j]));
                    }
                    cols.push(acc.to_string());
                }
                s.push_str(&cols.join(" "));
                s.push('\n');
            }
            s
        };
        assert_bytes_eq(&cb_bytes, expect.as_bytes(), &ctx);

        unsafe {
            (r.free_matrix)(ra);
            (r.free_matrix)(rb);
            (r.free_matrix)(rres);
            (c.free_matrix)(ca);
            (c.free_matrix)(cb);
            (c.free_matrix)(cres);
        }
    }
}

#[test]
fn c4b_allocate_rust_first() {
    let (c, r) = both();
    let mut rng = Rng::new(0x4B);
    for _ in 0..50 {
        let w = rng.i32_in(0, 32);
        let h = rng.i32_in(0, 32);
        let (rp, r_err) = capture_stderr(|| unsafe { (r.allocate_matrix)(w, h) });
        let (cp, c_err) = capture_stderr(|| unsafe { (c.allocate_matrix)(w, h) });
        let rs = unsafe { snap_matrix(rp, false) };
        let cs = unsafe { snap_matrix(cp, false) };
        assert_eq!(cs, rs, "allocate_matrix({w}, {h}) mismatch (rust first)");
        assert_bytes_eq(&c_err, &r_err, "allocate stderr mismatch (rust first)");
        unsafe {
            (r.free_matrix)(rp);
            (c.free_matrix)(cp);
        }
    }
}
