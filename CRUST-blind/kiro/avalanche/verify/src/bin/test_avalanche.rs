use avalanche::avalanche::{avalanche, HashFn, Matrix};
use std::io::Cursor;

/// FNV-1a 32-bit hash, matching the C implementation.
/// C uses `char*` (signed), so bytes >= 128 are sign-extended when XORed.
fn fnv_1a(key: &[u8], hash_value: &mut [u32]) {
    let mut hash: u32 = 0x811c9dc5;
    for &b in key {
        hash ^= b as i8 as i32 as u32; // sign-extend like C's `char`
        hash = hash.wrapping_mul(0x01000193);
    }
    hash_value[0] = hash;
}

// ── Matrix basics ──────────────────────────────────────────────

#[test]
fn test_matrix_alloc_zeros() {
    let m = Matrix::matrix_alloc(3, 4);
    assert_eq!(m.n_rows, 3);
    assert_eq!(m.n_cols, 4);
    assert_eq!(m.vals.len(), 12);
    for v in &m.vals {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_matrix_get_set() {
    let mut m = Matrix::matrix_alloc(2, 2);
    for r in 0..2 {
        for c in 0..2 {
            m.matrix_set(r, c, 1.0 / (r + c + 1) as f64);
        }
    }
    for r in 0..2 {
        for c in 0..2 {
            assert_eq!(m.matrix_get(r, c), 1.0 / (r + c + 1) as f64);
        }
    }
}

#[test]
fn test_matrix_get_set_row_major() {
    let mut m = Matrix::matrix_alloc(2, 3);
    m.matrix_set(0, 0, 1.0);
    m.matrix_set(0, 2, 3.0);
    m.matrix_set(1, 1, 5.0);
    assert_eq!(m.vals[0], 1.0);   // (0,0) -> index 0
    assert_eq!(m.vals[2], 3.0);   // (0,2) -> index 2
    assert_eq!(m.vals[4], 5.0);   // (1,1) -> index 4
}

// ── matrix_fprintf ─────────────────────────────────────────────

#[test]
fn test_matrix_fprintf_f_format() {
    let mut m = Matrix::matrix_alloc(3, 3);
    for r in 0..3 {
        for c in 0..3 {
            m.matrix_set(r, c, 1.0 / (r + c + 1) as f64);
        }
    }
    let mut out = Vec::new();
    m.matrix_fprintf(&mut out, "%8.4f");
    let output = String::from_utf8(out).unwrap();
    let expected = "  1.0000  0.5000  0.3333\n  0.5000  0.3333  0.2500\n  0.3333  0.2500  0.2000\n";
    assert_eq!(output, expected);
}

#[test]
fn test_matrix_fprintf_g_format() {
    let mut m = Matrix::matrix_alloc(2, 2);
    m.matrix_set(0, 0, 1.0);
    m.matrix_set(0, 1, 0.5);
    m.matrix_set(1, 0, 0.333333);
    m.matrix_set(1, 1, 0.0);
    let mut out = Vec::new();
    m.matrix_fprintf(&mut out, "% 6.4g");
    let output = String::from_utf8(out).unwrap();
    // C ground truth:
    // "     1   0.5\n 0.3333     0\n"
    let expected = "     1   0.5\n 0.3333     0\n";
    assert_eq!(output, expected);
}

#[test]
fn test_matrix_fprintf_e_format() {
    let mut m = Matrix::matrix_alloc(2, 2);
    m.matrix_set(0, 0, 1.0);
    m.matrix_set(0, 1, 0.5);
    m.matrix_set(1, 0, 0.333333);
    m.matrix_set(1, 1, 0.0);
    let mut out = Vec::new();
    m.matrix_fprintf(&mut out, "%10.4e");
    let output = String::from_utf8(out).unwrap();
    // C ground truth:
    // "1.0000e+005.0000e-01\n3.3333e-010.0000e+00\n"
    let expected = "1.0000e+005.0000e-01\n3.3333e-010.0000e+00\n";
    assert_eq!(output, expected);
}

// ── printf format edge cases ───────────────────────────────────

fn fmt(format: &str, val: f64) -> String {
    let m = Matrix { n_rows: 1, n_cols: 1, vals: vec![val] };
    let mut out = Vec::new();
    m.matrix_fprintf(&mut out, format);
    // strip trailing newline
    let s = String::from_utf8(out).unwrap();
    s.trim_end_matches('\n').to_string()
}

#[test]
fn test_fmt_f_values() {
    assert_eq!(fmt("%8.4f", 3.14159), "  3.1416");
    assert_eq!(fmt("%8.4f", 0.0), "  0.0000");
    assert_eq!(fmt("%8.4f", -1.5), " -1.5000");
}

#[test]
fn test_fmt_g_values() {
    assert_eq!(fmt("% 6.4g", 0.5), "   0.5");
    assert_eq!(fmt("% 6.4g", 0.0), "     0");
    assert_eq!(fmt("% 6.4g", 1234.5), "  1234");
    assert_eq!(fmt("% 6.4g", 0.00012345), " 0.0001234");
    assert_eq!(fmt("% 6.4g", -0.5), "  -0.5");
}

#[test]
fn test_fmt_e_values() {
    assert_eq!(fmt("%10.4e", 3.14159), "3.1416e+00");
    assert_eq!(fmt("%10.4e", 0.0), "0.0000e+00");
    assert_eq!(fmt("%10.4e", -1.5), "-1.5000e+00");
}

#[test]
fn test_fmt_g_default_precision() {
    assert_eq!(fmt("%g", 100000.0), "100000");
    assert_eq!(fmt("%g", 99999.9), "99999.9");
    assert_eq!(fmt("%g", 1e-4), "0.0001");
    assert_eq!(fmt("%g", 9.99999e-5), "9.99999e-05");
}

// ── Avalanche function ─────────────────────────────────────────

#[test]
fn test_avalanche_empty_stream() {
    let mut results = Matrix::matrix_alloc(8, 32);
    let data: Vec<u8> = vec![];
    let mut cursor = Cursor::new(data);
    avalanche(fnv_1a as HashFn, &mut cursor, 100, &mut results);
    for v in &results.vals {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_avalanche_single_key() {
    let mut results = Matrix::matrix_alloc(8, 32);
    let data: Vec<u8> = vec![0x42];
    let mut cursor = Cursor::new(data);
    avalanche(fnv_1a as HashFn, &mut cursor, 1, &mut results);

    // C ground truth for single key 0x42 (all values are 0.0 or 1.0)
    #[rustfmt::skip]
    let expected: [[f64; 32]; 8] = [
        [0.,0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,1.,1.,0.,0.,0.,1.,0.,1.,0.,1.,0.,1.,0.,1.,0.,0.,0.,0.,0.,0.,0.],
        [1.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,0.,0.,1.,1.,0.,1.,1.,1.,1.,0.,0.,0.,0.,0.,0.],
        [0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,1.,0.,1.,0.,1.,1.,0.,0.,1.,1.,0.,0.,0.,0.,0.],
        [0.,0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,1.,1.,0.,1.,1.,1.,1.,0.,0.,1.,1.,0.,0.,0.,0.],
        [0.,0.,0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,1.,1.,1.,0.,0.,1.,1.,1.,0.,0.,1.,1.,0.,0.,0.],
        [0.,0.,0.,0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,1.,1.,0.,1.,0.,1.,1.,1.,1.,0.,0.],
        [0.,0.,0.,0.,0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,1.,1.,1.,0.,1.,1.,0.,1.,0.],
        [0.,0.,0.,0.,0.,0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,0.,0.,1.,1.,1.,0.,1.,1.,1.],
    ];
    for r in 0..8 {
        for c in 0..32 {
            assert_eq!(
                results.matrix_get(r, c), expected[r][c],
                "mismatch at ({}, {})", r, c
            );
        }
    }
}

#[test]
fn test_avalanche_two_keys() {
    let mut results = Matrix::matrix_alloc(8, 32);
    let data: Vec<u8> = vec![0xAB, 0xCD];
    let mut cursor = Cursor::new(data);
    avalanche(fnv_1a as HashFn, &mut cursor, 2, &mut results);

    // C ground truth for 2 keys [0xAB, 0xCD], values in {0, 0.5, 1}
    #[rustfmt::skip]
    let expected: [[f64; 32]; 8] = [
        [0.,0.,0.,0.,0.5,0.5,1.,1.,1.,1.,1.,1.,1.,0.5,0.5,0.5,0.5,0.5,1.,0.,1.,1.,1.,0.5,1.,0.,0.,0.,0.,0.,0.,0.],
        [1.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.5,1.,1.,0.5,0.5,1.,0.,1.,0.5,1.,0.,0.,0.,0.,0.,0.],
        [0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.5,0.5,0.5,1.,0.,0.5,1.,0.5,0.5,1.,1.,0.,0.,0.,0.,0.],
        [0.,0.,0.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.5,0.5,0.5,0.5,0.5,1.,1.,1.,0.5,0.5,1.,0.,1.,0.5,1.,0.,0.,0.,0.],
        [0.,0.5,0.5,0.5,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,0.,1.,0.,0.5,1.,0.,0.5,1.,1.,0.,0.,0.],
        [0.,0.,0.,0.,0.5,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.5,1.,0.5,1.,0.5,0.5,1.,1.,1.,0.5,1.,0.,0.],
        [0.,0.,0.,0.,0.5,1.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.5,1.,1.,0.,0.,1.,0.,0.5,1.,1.,0.],
        [0.,0.,0.,0.,0.5,0.5,1.,1.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,1.,1.,0.5,1.,0.,0.5,1.,0.,0.5,1.,1.],
    ];
    for r in 0..8 {
        for c in 0..32 {
            assert_eq!(
                results.matrix_get(r, c), expected[r][c],
                "mismatch at ({}, {})", r, c
            );
        }
    }
}

#[test]
fn test_avalanche_max_iter_limits() {
    // Provide 3 keys but set max_iter=1, should only process 1 key
    let mut results = Matrix::matrix_alloc(8, 32);
    let data: Vec<u8> = vec![0x42, 0xAB, 0xCD];
    let mut cursor = Cursor::new(data);
    avalanche(fnv_1a as HashFn, &mut cursor, 1, &mut results);

    // With max_iter=1, only key 0x42 is processed — same as single key test
    // All values should be 0.0 or 1.0
    for v in &results.vals {
        assert!(*v == 0.0 || *v == 1.0, "expected 0 or 1, got {}", v);
    }
    // Spot-check a few known values from single-key ground truth
    assert_eq!(results.matrix_get(0, 7), 1.0);
    assert_eq!(results.matrix_get(0, 0), 0.0);
    assert_eq!(results.matrix_get(1, 0), 1.0);
}

// ── Avalanche output formatting (end-to-end like fnv_1a.c) ────

#[test]
fn test_avalanche_formatted_output() {
    let mut results = Matrix::matrix_alloc(8, 32);
    let data: Vec<u8> = vec![0x42];
    let mut cursor = Cursor::new(data);
    avalanche(fnv_1a as HashFn, &mut cursor, 1, &mut results);

    let mut out = Vec::new();
    results.matrix_fprintf(&mut out, "% 6.4g");
    let output = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 8);
    // First row from C: "     0     0     0     0     0     0     0     1     1 ..."
    // Values are 0 or 1, formatted with "% 6.4g"
    assert!(lines[0].contains("     0"));
    assert!(lines[0].contains("     1"));
}

fn main() {}
