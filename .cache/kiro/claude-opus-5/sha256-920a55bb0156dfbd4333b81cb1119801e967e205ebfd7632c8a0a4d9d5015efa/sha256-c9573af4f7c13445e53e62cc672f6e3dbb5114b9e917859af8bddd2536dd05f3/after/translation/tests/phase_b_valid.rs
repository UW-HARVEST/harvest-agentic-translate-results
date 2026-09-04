//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row
//! (C1–C31, C38, C39). Driver-pipeline rows C32–C37 live in
//! `phase_b_driver.rs` because they mutate the process working directory.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// generic differential helpers
// ---------------------------------------------------------------------------

fn diff_allocate(width: c_int, height: c_int) {
    let mut seen = Vec::new();
    for api in both() {
        unsafe {
            let m = (api.allocate_matrix)(width, height);
            assert!(
                !m.is_null(),
                "{}: allocate_matrix({width},{height}) returned NULL",
                api.name
            );
            let dims = ((*m).width, (*m).height);
            let ptrs = row_ptrs(m);
            assert!(
                ptrs.iter().all(|p| !p.is_null()),
                "{}: NULL row pointer for ({width},{height})",
                api.name
            );
            // Rows must be distinct allocations.
            for a in 0..ptrs.len() {
                for b in (a + 1)..ptrs.len() {
                    assert_ne!(ptrs[a], ptrs[b], "{}: aliased rows", api.name);
                }
            }
            // Writing every cell must be in-bounds (checked by the allocator).
            for i in 0..height.max(0) {
                for j in 0..width.max(0) {
                    *(*(*m).matrix.offset(i as isize)).offset(j as isize) = (i * 31 + j) as c_int;
                }
            }
            seen.push((dims, ptrs.len(), snapshot(m)));
            (api.free_matrix)(m);
        }
    }
    assert_eq!(
        seen[0], seen[1],
        "allocate_matrix({width},{height}) diverged"
    );
}

fn diff_init(input: &str, width: c_int, height: c_int) {
    let cs = cstring(input);
    let mut seen = Vec::new();
    for api in both() {
        unsafe {
            let m = (api.initialize_matrix_from_string)(cs.as_ptr(), width, height);
            if m.is_null() {
                seen.push(None);
            } else {
                seen.push(Some(snapshot(m)));
                (api.free_matrix)(m);
            }
        }
    }
    assert_eq!(
        seen[0], seen[1],
        "initialize_matrix_from_string({input:?}, w={width}, h={height}) diverged"
    );
}

fn diff_multiply(a: &[Vec<c_int>], a_w: c_int, b: &[Vec<c_int>], b_w: c_int) {
    let mut seen = Vec::new();
    for api in both() {
        unsafe {
            let ma = build_matrix(api, a, a_w);
            let mb = build_matrix(api, b, b_w);
            let r = (api.multiply_matrices)(ma, mb);
            if r.is_null() {
                seen.push(None);
            } else {
                seen.push(Some(snapshot(r)));
                (api.free_matrix)(r);
            }
            (api.free_matrix)(ma);
            (api.free_matrix)(mb);
        }
    }
    assert_eq!(seen[0], seen[1], "multiply_matrices diverged");
}

fn diff_to_string(rows: &[Vec<c_int>], width: c_int) -> Option<Vec<u8>> {
    let mut seen = Vec::new();
    for api in both() {
        unsafe {
            let m = build_matrix(api, rows, width);
            let s = (api.matrix_to_string)(m);
            seen.push(take_c_string(s));
            (api.free_matrix)(m);
        }
    }
    assert_eq!(
        seen[0].as_ref().map(|b| String::from_utf8_lossy(b).into_owned()),
        seen[1].as_ref().map(|b| String::from_utf8_lossy(b).into_owned()),
        "matrix_to_string diverged for {rows:?} width={width}"
    );
    seen.into_iter().next().unwrap()
}

fn diff_write(dir: &std::path::Path, name: &str, content: &[u8]) {
    let content_c = std::ffi::CString::new(content).expect("no interior NUL");
    let mut seen = Vec::new();
    for api in both() {
        let path = dir.join(format!("{name}.{}", api.name));
        let path_c = cstring(path.to_str().unwrap());
        let rc = unsafe { (api.write_to_file)(path_c.as_ptr(), content_c.as_ptr()) };
        let bytes = std::fs::read(&path).ok();
        seen.push((rc, bytes));
    }
    assert_eq!(seen[0].0, seen[1].0, "write_to_file rc diverged for {name}");
    assert_eq!(
        seen[0].1, seen[1].1,
        "write_to_file file bytes diverged for {name}"
    );
    assert_eq!(seen[0].0, 0, "expected success for {name}");
    assert_eq!(
        seen[0].1.as_deref(),
        Some(content),
        "file contents wrong for {name}"
    );
}

fn rand_rows(rng: &mut Rng, h: usize, w: usize, lo: c_int, hi: c_int) -> Vec<Vec<c_int>> {
    (0..h)
        .map(|_| (0..w).map(|_| rng.i32_in(lo, hi)).collect())
        .collect()
}

// ---------------------------------------------------------------------------
// C1–C3 — allocate_matrix / free_matrix
// ---------------------------------------------------------------------------

#[test]
fn c1_allocate_and_free_cross_product() {
    for &w in &[0, 1, 2, 3, 7, 64] {
        for &h in &[0, 1, 2, 3, 7, 64] {
            diff_allocate(w, h);
        }
    }
    // randomized sweep
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..300 {
        diff_allocate(rng.i32_in(0, 40), rng.i32_in(0, 40));
    }
}

#[test]
fn c2_allocate_height_zero() {
    for &w in &[0, 1, 5] {
        diff_allocate(w, 0);
        for api in both() {
            unsafe {
                let m = (api.allocate_matrix)(w, 0);
                assert!(!m.is_null(), "{}: malloc(0) row array was NULL", api.name);
                assert!(
                    !(*m).matrix.is_null(),
                    "{}: matrix field NULL for height 0",
                    api.name
                );
                assert_eq!(((*m).width, (*m).height), (w, 0));
                (api.free_matrix)(m);
            }
        }
    }
}

#[test]
fn c3_allocate_width_zero() {
    for &h in &[1, 3, 16] {
        diff_allocate(0, h);
        for api in both() {
            unsafe {
                let m = (api.allocate_matrix)(0, h);
                assert!(!m.is_null());
                for p in row_ptrs(m) {
                    assert!(!p.is_null(), "{}: malloc(0) row was NULL", api.name);
                }
                (api.free_matrix)(m);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C4–C14 — initialize_matrix_from_string
// ---------------------------------------------------------------------------

#[test]
fn c4_init_canonical_with_trailing_newline() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..400 {
        let h = rng.range(1, 6) as usize;
        let w = rng.range(1, 6) as usize;
        let rows = rand_rows(&mut rng, h, w, -10_000, 10_000);
        diff_init(&canonical(&rows), w as c_int, h as c_int);
    }
}

#[test]
fn c5_init_no_trailing_newline() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..400 {
        let h = rng.range(1, 6) as usize;
        let w = rng.range(1, 6) as usize;
        let rows = rand_rows(&mut rng, h, w, -10_000, 10_000);
        let s = canonical(&rows);
        diff_init(s.trim_end_matches('\n'), w as c_int, h as c_int);
    }
}

#[test]
fn c6_init_repeated_space_separators() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..400 {
        let h = rng.range(1, 5) as usize;
        let w = rng.range(1, 5) as usize;
        let rows = rand_rows(&mut rng, h, w, -9_999, 9_999);
        let mut s = String::new();
        for r in &rows {
            for (idx, v) in r.iter().enumerate() {
                if idx > 0 {
                    for _ in 0..rng.range(2, 5) {
                        s.push(' ');
                    }
                }
                s.push_str(&v.to_string());
            }
            s.push('\n');
        }
        diff_init(&s, w as c_int, h as c_int);
    }
}

#[test]
fn c7_init_leading_and_trailing_spaces() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..400 {
        let h = rng.range(1, 5) as usize;
        let w = rng.range(1, 5) as usize;
        let rows = rand_rows(&mut rng, h, w, -9_999, 9_999);
        let mut s = String::new();
        for r in &rows {
            for _ in 0..rng.range(1, 3) {
                s.push(' ');
            }
            let cells: Vec<String> = r.iter().map(|v| v.to_string()).collect();
            s.push_str(&cells.join(" "));
            for _ in 0..rng.range(1, 3) {
                s.push(' ');
            }
            s.push('\n');
        }
        diff_init(&s, w as c_int, h as c_int);
    }
}

#[test]
fn c8_init_blank_lines() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..400 {
        let h = rng.range(1, 5) as usize;
        let w = rng.range(1, 4) as usize;
        let rows = rand_rows(&mut rng, h, w, -999, 999);
        let mut s = String::new();
        for _ in 0..rng.range(0, 3) {
            s.push('\n');
        }
        for r in &rows {
            let cells: Vec<String> = r.iter().map(|v| v.to_string()).collect();
            s.push_str(&cells.join(" "));
            for _ in 0..rng.range(1, 4) {
                s.push('\n');
            }
        }
        diff_init(&s, w as c_int, h as c_int);
    }
}

#[test]
fn c9_init_extra_rows_and_columns_ignored() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..400 {
        let h = rng.range(1, 4) as usize;
        let w = rng.range(1, 4) as usize;
        let extra_r = rng.range(0, 3) as usize;
        let extra_c = rng.range(0, 3) as usize;
        let rows = rand_rows(&mut rng, h + extra_r, w + extra_c, -999, 999);
        diff_init(&canonical(&rows), w as c_int, h as c_int);
    }
}

#[test]
fn c10_init_height_zero_ignores_input() {
    let inputs = [
        "", "\n", "\n\n\n", " ", "garbage", "1 2 3\n4 5 6\n", "   \n  \n", "%s %d", "\t\t",
    ];
    for s in inputs {
        for &w in &[0, 1, 3] {
            diff_init(s, w, 0);
        }
    }
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..100 {
        let n = rng.range(0, 20) as usize;
        let s: String = (0..n)
            .map(|_| *rng.pick(&['a', '1', ' ', '\n', '-', '9', '\t']))
            .collect();
        diff_init(&s, rng.i32_in(0, 5), 0);
    }
}

#[test]
fn c11_init_width_zero() {
    let mut rng = Rng::new(SEED ^ 11);
    for h in 1..=4 {
        for _ in 0..50 {
            let n = rng.range(0, 8) as usize;
            let s: String = (0..n)
                .map(|_| *rng.pick(&['1', '2', ' ', '\n', '-']))
                .collect();
            diff_init(&s, 0, h);
        }
        diff_init("1 2\n3 4\n5 6\n7 8\n", 0, h);
        diff_init("\n\n\n\n\n", 0, h);
    }
}

#[test]
fn c12_init_atoi_corner_tokens() {
    let tokens = [
        "0",
        "-0",
        "+7",
        "007",
        "\t9",
        "12abc",
        "abc",
        "0x10",
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "99999999999999999999",
        "-99999999999999999999",
        "1e3",
        "--5",
        "3.9",
        "-",
        "+",
        ".5",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "4294967296",
        "4294967295",
    ];
    // one token per 1x1 matrix
    for t in tokens {
        diff_init(&format!("{t}\n"), 1, 1);
    }
    // random mixes into wider matrices
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..300 {
        let h = rng.range(1, 4) as usize;
        let w = rng.range(1, 4) as usize;
        let mut s = String::new();
        for _ in 0..h {
            let cells: Vec<&str> = (0..w).map(|_| *rng.pick(&tokens)).collect();
            s.push_str(&cells.join(" "));
            s.push('\n');
        }
        diff_init(&s, w as c_int, h as c_int);
    }
}

#[test]
fn c13_init_degenerate_shapes() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..200 {
        let n = rng.range(1, 12) as usize;
        // 1 x N
        let rows = rand_rows(&mut rng, 1, n, -100_000, 100_000);
        diff_init(&canonical(&rows), n as c_int, 1);
        // N x 1
        let rows = rand_rows(&mut rng, n, 1, -100_000, 100_000);
        diff_init(&canonical(&rows), 1, n as c_int);
        // 1 x 1
        let rows = rand_rows(&mut rng, 1, 1, i32::MIN, i32::MAX);
        diff_init(&canonical(&rows), 1, 1);
    }
}

#[test]
fn c14_init_large_shape() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..10 {
        let rows = rand_rows(&mut rng, 40, 30, i32::MIN, i32::MAX);
        diff_init(&canonical(&rows), 30, 40);
    }
}

// ---------------------------------------------------------------------------
// C15–C20 — multiply_matrices
// ---------------------------------------------------------------------------

#[test]
fn c15_multiply_random_conformable() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..400 {
        let m = rng.range(1, 6) as usize;
        let k = rng.range(1, 6) as usize;
        let n = rng.range(1, 6) as usize;
        let a = rand_rows(&mut rng, m, k, -100, 100);
        let b = rand_rows(&mut rng, k, n, -100, 100);
        diff_multiply(&a, k as c_int, &b, n as c_int);
    }
}

#[test]
fn c16_multiply_shared_dimension_zero() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..50 {
        let m = rng.range(1, 5) as usize;
        let n = rng.range(1, 5) as usize;
        let a: Vec<Vec<c_int>> = (0..m).map(|_| Vec::new()).collect();
        let b: Vec<Vec<c_int>> = Vec::new();
        diff_multiply(&a, 0, &b, n as c_int);
    }
}

#[test]
fn c17_multiply_empty_outer_dimensions() {
    // mat_a->height == 0
    for n in 0..4 {
        let a: Vec<Vec<c_int>> = Vec::new();
        let b: Vec<Vec<c_int>> = (0..2).map(|_| vec![1; n]).collect();
        diff_multiply(&a, 2, &b, n as c_int);
    }
    // mat_b->width == 0
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..50 {
        let m = rng.range(1, 4) as usize;
        let k = rng.range(1, 4) as usize;
        let a = rand_rows(&mut rng, m, k, -50, 50);
        let b: Vec<Vec<c_int>> = (0..k).map(|_| Vec::new()).collect();
        diff_multiply(&a, k as c_int, &b, 0);
    }
}

#[test]
fn c18_multiply_shared_dimension_one_and_long() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..100 {
        let m = rng.range(1, 4) as usize;
        let n = rng.range(1, 4) as usize;
        let a = rand_rows(&mut rng, m, 1, -1000, 1000);
        let b = rand_rows(&mut rng, 1, n, -1000, 1000);
        diff_multiply(&a, 1, &b, n as c_int);
    }
    for _ in 0..10 {
        let a = rand_rows(&mut rng, 3, 64, -1000, 1000);
        let b = rand_rows(&mut rng, 64, 3, -1000, 1000);
        diff_multiply(&a, 64, &b, 3);
    }
}

#[test]
fn c19_multiply_wrapping_overflow() {
    let mut rng = Rng::new(SEED ^ 19);
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -65_536,
        -1,
        0,
        1,
        65_536,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..400 {
        let m = rng.range(1, 4) as usize;
        let k = rng.range(1, 5) as usize;
        let n = rng.range(1, 4) as usize;
        let a: Vec<Vec<c_int>> = (0..m)
            .map(|_| (0..k).map(|_| *rng.pick(&extremes)).collect())
            .collect();
        let b: Vec<Vec<c_int>> = (0..k)
            .map(|_| (0..n).map(|_| *rng.pick(&extremes)).collect())
            .collect();
        diff_multiply(&a, k as c_int, &b, n as c_int);
    }
    // fully random 32-bit values
    for _ in 0..200 {
        let k = rng.range(1, 6) as usize;
        let a: Vec<Vec<c_int>> = (0..3).map(|_| (0..k).map(|_| rng.any_i32()).collect()).collect();
        let b: Vec<Vec<c_int>> = (0..k).map(|_| (0..3).map(|_| rng.any_i32()).collect()).collect();
        diff_multiply(&a, k as c_int, &b, 3);
    }
}

#[test]
fn c20_multiply_chained() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..200 {
        let m = rng.range(1, 4) as usize;
        let k = rng.range(1, 4) as usize;
        let n = rng.range(1, 4) as usize;
        let p = rng.range(1, 4) as usize;
        let a = rand_rows(&mut rng, m, k, -200, 200);
        let b = rand_rows(&mut rng, k, n, -200, 200);
        let c = rand_rows(&mut rng, n, p, -200, 200);
        let mut seen = Vec::new();
        for api in both() {
            unsafe {
                let ma = build_matrix(api, &a, k as c_int);
                let mb = build_matrix(api, &b, n as c_int);
                let mc = build_matrix(api, &c, p as c_int);
                let ab = (api.multiply_matrices)(ma, mb);
                assert!(!ab.is_null());
                let abc = (api.multiply_matrices)(ab, mc);
                assert!(!abc.is_null());
                let s = take_c_string((api.matrix_to_string)(abc));
                seen.push((snapshot(abc), s));
                (api.free_matrix)(abc);
                (api.free_matrix)(ab);
                (api.free_matrix)(mc);
                (api.free_matrix)(mb);
                (api.free_matrix)(ma);
            }
        }
        assert_eq!(seen[0], seen[1], "chained (A*B)*C diverged");
    }
}

// ---------------------------------------------------------------------------
// C21–C26 — matrix_to_string
//
// Values are kept to at most 10 decimal characters: the C buffer formula
// budgets 11*width bytes per row, which is exactly enough for 10-character
// values plus separators.  11-character values (<= -1000000000) overflow the
// buffer in the C — that quirk is verified separately in `subprocess_parity.rs`.
// ---------------------------------------------------------------------------

const SAFE_LO: c_int = -999_999_999;
const SAFE_HI: c_int = i32::MAX;

#[test]
fn c21_to_string_random() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..400 {
        let h = rng.range(1, 6) as usize;
        let w = rng.range(1, 6) as usize;
        let rows = rand_rows(&mut rng, h, w, -10_000, 10_000);
        diff_to_string(&rows, w as c_int);
    }
    for _ in 0..400 {
        let h = rng.range(1, 6) as usize;
        let w = rng.range(1, 6) as usize;
        let rows = rand_rows(&mut rng, h, w, SAFE_LO, SAFE_HI);
        diff_to_string(&rows, w as c_int);
    }
}

#[test]
fn c22_to_string_width_one() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..200 {
        let h = rng.range(1, 8) as usize;
        let rows = rand_rows(&mut rng, h, 1, i32::MIN, i32::MAX);
        // width == 1: buffer is 11*height + height + 1, and an 11-char value
        // plus its newline fits exactly.
        diff_to_string(&rows, 1);
    }
    // the exact worst case that still fits
    let out = diff_to_string(&[vec![i32::MIN]], 1).unwrap();
    assert_eq!(out, b"-2147483648\n");
}

#[test]
fn c23_to_string_width_zero() {
    for h in 1..=5usize {
        let rows: Vec<Vec<c_int>> = (0..h).map(|_| Vec::new()).collect();
        let out = diff_to_string(&rows, 0).unwrap();
        assert_eq!(out, vec![b'\n'; h]);
    }
}

#[test]
fn c24_to_string_height_zero() {
    let out = diff_to_string(&[], 0).unwrap();
    assert_eq!(out, b"");
    let out = diff_to_string(&[], 5).unwrap();
    assert_eq!(out, b"");
}

#[test]
fn c25_to_string_value_widths() {
    // all zeros
    for (h, w) in [(1usize, 1usize), (3, 4), (5, 5)] {
        let rows: Vec<Vec<c_int>> = (0..h).map(|_| vec![0; w]).collect();
        diff_to_string(&rows, w as c_int);
    }
    // single digit
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..100 {
        let h = rng.range(1, 5) as usize;
        let w = rng.range(1, 5) as usize;
        let rows = rand_rows(&mut rng, h, w, -9, 9);
        diff_to_string(&rows, w as c_int);
    }
    // widest values that still fit the C's buffer arithmetic (10 characters)
    for v in [-999_999_999, 1_000_000_000, i32::MAX, -1] {
        for (h, w) in [(1usize, 1usize), (2, 3), (4, 4)] {
            let rows: Vec<Vec<c_int>> = (0..h).map(|_| vec![v; w]).collect();
            diff_to_string(&rows, w as c_int);
        }
    }
}

#[test]
fn c26_to_string_large_shape() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..10 {
        let rows = rand_rows(&mut rng, 40, 30, SAFE_LO, SAFE_HI);
        diff_to_string(&rows, 30);
    }
}

// ---------------------------------------------------------------------------
// C27–C31 — write_to_file
// ---------------------------------------------------------------------------

#[test]
fn c27_write_new_file_payloads() {
    let dir = scratch_dir("c27");
    diff_write(&dir, "empty", b"");
    diff_write(&dir, "one", b"x");
    diff_write(&dir, "short", b"hello world");
    diff_write(&dir, "lines", b"1 2\n3 4\n");
    diff_write(&dir, "trailing_nl", b"\n");
    diff_write(&dir, "binaryish", &(1u8..=255).collect::<Vec<u8>>());
    let mut rng = Rng::new(SEED ^ 27);
    for i in 0..100 {
        let n = rng.range(0, 200) as usize;
        let body: Vec<u8> = (0..n).map(|_| rng.i32_in(1, 255) as u8).collect();
        diff_write(&dir, &format!("rand{i}"), &body);
    }
}

#[test]
fn c28_write_truncates_existing_longer_file() {
    let dir = scratch_dir("c28");
    for api in both() {
        let path = dir.join(format!("trunc.{}", api.name));
        std::fs::write(&path, b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").unwrap();
    }
    diff_write(&dir, "trunc", b"short");
    // and again, empty
    for api in both() {
        let path = dir.join(format!("trunc2.{}", api.name));
        std::fs::write(&path, b"BBBBBBBBBBBBBBBB").unwrap();
    }
    diff_write(&dir, "trunc2", b"");
}

#[test]
fn c29_write_payload_larger_than_stdio_buffer() {
    let dir = scratch_dir("c29");
    let mut rng = Rng::new(SEED ^ 29);
    for (i, n) in [4095usize, 4096, 4097, 65_536, 200_000].iter().enumerate() {
        let body: Vec<u8> = (0..*n).map(|_| rng.i32_in(b'a' as i32, b'z' as i32) as u8).collect();
        diff_write(&dir, &format!("big{i}"), &body);
    }
}

#[test]
fn c30_write_filename_forms() {
    let dir = scratch_dir("c30");
    // absolute (the helper already uses absolute paths under temp_dir)
    diff_write(&dir, "abs", b"absolute");
    // names containing printf-ish characters
    for (i, name) in ["pct%s", "pct%d", "pct%n", "a%%b", "brace{}"].iter().enumerate() {
        diff_write(&dir, &format!("{name}_{i}"), b"payload");
    }
    // relative path form
    let cwd = std::env::current_dir().unwrap();
    let rel_dir = dir.strip_prefix(&cwd).ok().map(|p| p.to_path_buf());
    if let Some(rel) = rel_dir {
        diff_write(&rel, "rel", b"relative");
    } else {
        // temp dir is not under cwd; exercise a relative name in a subdirectory
        // of the scratch dir reached via "." components instead.
        let dotted = dir.join(".").join(".");
        diff_write(&dotted, "dotted", b"dotted");
    }
}

#[test]
fn c31_write_content_with_format_specifiers() {
    let dir = scratch_dir("c31");
    for (i, body) in [
        &b"%s"[..],
        &b"%d %d %d"[..],
        &b"%n"[..],
        &b"100%"[..],
        &b"%%s%%d"[..],
        &b"%1$s %2$s"[..],
        &b"a\tb\\c\"d"[..],
    ]
    .iter()
    .enumerate()
    {
        diff_write(&dir, &format!("fmt{i}"), body);
    }
}

// ---------------------------------------------------------------------------
// C38 — struct layout parity: one library's matrix_t consumed by the other
// ---------------------------------------------------------------------------

#[test]
fn c38_cross_library_struct_consumption() {
    let mut rng = Rng::new(SEED ^ 38);
    let [c, r] = both();
    for _ in 0..200 {
        let m = rng.range(1, 5) as usize;
        let k = rng.range(1, 5) as usize;
        let n = rng.range(1, 5) as usize;
        let a = rand_rows(&mut rng, m, k, -300, 300);
        let b = rand_rows(&mut rng, k, n, -300, 300);

        let mut seen = Vec::new();
        // producer/consumer in all four assignments
        for (producer, consumer) in [(c, c), (c, r), (r, c), (r, r)] {
            unsafe {
                let ma = build_matrix(producer, &a, k as c_int);
                let mb = build_matrix(producer, &b, n as c_int);
                let res = (consumer.multiply_matrices)(ma, mb);
                assert!(!res.is_null());
                let s = take_c_string((consumer.matrix_to_string)(res));
                seen.push((snapshot(res), s));
                // the result was allocated by `consumer`
                (consumer.free_matrix)(res);
                (producer.free_matrix)(mb);
                (producer.free_matrix)(ma);
            }
        }
        for w in seen.windows(2) {
            assert_eq!(w[0], w[1], "cross-library struct handling diverged");
        }
    }
}

// ---------------------------------------------------------------------------
// C39 — cross-library pipelines: every assignment of the 3 stages
// ---------------------------------------------------------------------------

#[test]
fn c39_cross_library_pipeline() {
    let mut rng = Rng::new(SEED ^ 39);
    let libs = both();
    for _ in 0..100 {
        let m = rng.range(1, 5) as usize;
        let k = rng.range(1, 5) as usize;
        let n = rng.range(1, 5) as usize;
        let a = rand_rows(&mut rng, m, k, -300, 300);
        let b = rand_rows(&mut rng, k, n, -300, 300);
        let sa = cstring(&canonical(&a));
        let sb = cstring(&canonical(&b));

        let mut seen = Vec::new();
        for init in 0..2 {
            for mul in 0..2 {
                for to_s in 0..2 {
                    unsafe {
                        let li = libs[init];
                        let lm = libs[mul];
                        let ls = libs[to_s];
                        let ma =
                            (li.initialize_matrix_from_string)(sa.as_ptr(), k as c_int, m as c_int);
                        let mb =
                            (li.initialize_matrix_from_string)(sb.as_ptr(), n as c_int, k as c_int);
                        assert!(!ma.is_null() && !mb.is_null());
                        let res = (lm.multiply_matrices)(ma, mb);
                        assert!(!res.is_null());
                        let s = take_c_string((ls.matrix_to_string)(res));
                        seen.push(s);
                        (lm.free_matrix)(res);
                        (li.free_matrix)(mb);
                        (li.free_matrix)(ma);
                    }
                }
            }
        }
        for w in seen.windows(2) {
            assert_eq!(w[0], w[1], "cross-library pipeline diverged");
        }
    }
}
