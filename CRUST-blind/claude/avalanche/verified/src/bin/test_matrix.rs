#![allow(dead_code, unused_imports)]
use avalanche::avalanche::Matrix;

#[test]
fn test_matrix_alloc_dims() {
    let m = Matrix::matrix_alloc(2, 3);
    assert_eq!(m.n_rows, 2);
    assert_eq!(m.n_cols, 3);
    assert_eq!(m.vals.len(), 6);
    for v in &m.vals {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_matrix_alloc_zero_dims() {
    let m = Matrix::matrix_alloc(0, 0);
    assert_eq!(m.n_rows, 0);
    assert_eq!(m.n_cols, 0);
    assert_eq!(m.vals.len(), 0);
}

#[test]
fn test_matrix_alloc_larger() {
    let m = Matrix::matrix_alloc(32, 32);
    assert_eq!(m.n_rows, 32);
    assert_eq!(m.n_cols, 32);
    assert_eq!(m.vals.len(), 1024);
    for v in &m.vals {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_matrix_get_set_hilbert() {
    // Mirrors the C test_macros test: 2x2 Hilbert-style matrix.
    let mut m = Matrix::matrix_alloc(2, 2);
    for r in 0..2 {
        for c in 0..2 {
            m.matrix_set(r, c, 1.0 / ((r + c + 1) as f64));
        }
    }
    for r in 0..2 {
        for c in 0..2 {
            let expected = 1.0 / ((r + c + 1) as f64);
            assert_eq!(m.matrix_get(r, c), expected);
        }
    }
    // Specific values
    assert_eq!(m.matrix_get(0, 0), 1.0);
    assert_eq!(m.matrix_get(0, 1), 0.5);
    assert_eq!(m.matrix_get(1, 0), 0.5);
    assert_eq!(m.matrix_get(1, 1), 1.0 / 3.0);
}

#[test]
fn test_matrix_row_major_layout() {
    // Verify the C row-major layout: vals[r * n_cols + c]
    let mut m = Matrix::matrix_alloc(3, 4);
    for r in 0..3 {
        for c in 0..4 {
            m.matrix_set(r, c, (r * 100 + c) as f64);
        }
    }
    // Direct vals access mirrors C's MATRIX_GET macro
    for r in 0..3 {
        for c in 0..4 {
            assert_eq!(m.vals[r * 4 + c], (r * 100 + c) as f64);
            assert_eq!(m.matrix_get(r, c), (r * 100 + c) as f64);
        }
    }
}

#[test]
fn test_matrix_fprintf_hilbert_8_4f() {
    // Mirrors print_hilbert in C matrix_test.c with format "%8.4f"
    // Expected output (from running C):
    //   "  1.0000  0.5000  0.3333\n"
    //   "  0.5000  0.3333  0.2500\n"
    //   "  0.3333  0.2500  0.2000\n"
    let mut m = Matrix::matrix_alloc(3, 3);
    for r in 0..3 {
        for c in 0..3 {
            m.matrix_set(r, c, 1.0 / ((r + c + 1) as f64));
        }
    }
    let mut buf: Vec<u8> = Vec::new();
    m.matrix_fprintf(&mut buf, "%8.4f");
    let s = String::from_utf8(buf).unwrap();
    let expected = "  1.0000  0.5000  0.3333\n  0.5000  0.3333  0.2500\n  0.3333  0.2500  0.2000\n";
    assert_eq!(s, expected);
}

#[test]
fn test_matrix_fprintf_simple_8_4f() {
    // Test a 2x3 matrix using %8.4f, mirrors C output.
    let mut m = Matrix::matrix_alloc(2, 3);
    m.matrix_set(0, 0, 1.5);
    m.matrix_set(0, 1, 2.5);
    m.matrix_set(0, 2, 3.5);
    m.matrix_set(1, 0, -1.5);
    m.matrix_set(1, 1, 0.0);
    m.matrix_set(1, 2, 0.123456);
    let mut buf: Vec<u8> = Vec::new();
    m.matrix_fprintf(&mut buf, "%8.4f");
    let s = String::from_utf8(buf).unwrap();
    // From C:
    //   "  1.5000  2.5000  3.5000\n"
    //   " -1.5000  0.0000  0.1235\n"
    let expected = "  1.5000  2.5000  3.5000\n -1.5000  0.0000  0.1235\n";
    assert_eq!(s, expected);
}

#[test]
fn test_matrix_fprintf_g_format() {
    // Test using "% 6.4g" format
    let mut m = Matrix::matrix_alloc(2, 3);
    m.matrix_set(0, 0, 1.5);
    m.matrix_set(0, 1, 2.5);
    m.matrix_set(0, 2, 3.5);
    m.matrix_set(1, 0, -1.5);
    m.matrix_set(1, 1, 0.0);
    m.matrix_set(1, 2, 0.123456);
    let mut buf: Vec<u8> = Vec::new();
    m.matrix_fprintf(&mut buf, "% 6.4g");
    let s = String::from_utf8(buf).unwrap();
    // From C:
    //   "   1.5   2.5   3.5\n"
    //   "  -1.5     0 0.1235\n"
    let expected = "   1.5   2.5   3.5\n  -1.5     0 0.1235\n";
    assert_eq!(s, expected);
}

#[test]
fn test_matrix_fprintf_simple_5_2f() {
    let mut m = Matrix::matrix_alloc(2, 3);
    m.matrix_set(0, 0, 1.5);
    m.matrix_set(0, 1, 2.5);
    m.matrix_set(0, 2, 3.5);
    m.matrix_set(1, 0, -1.5);
    m.matrix_set(1, 1, 0.0);
    m.matrix_set(1, 2, 0.123456);
    let mut buf: Vec<u8> = Vec::new();
    m.matrix_fprintf(&mut buf, " %5.2f");
    let s = String::from_utf8(buf).unwrap();
    // From C:
    //   "  1.50  2.50  3.50\n"
    //   " -1.50  0.00  0.12\n"
    let expected = "  1.50  2.50  3.50\n -1.50  0.00  0.12\n";
    assert_eq!(s, expected);
}

#[test]
fn test_matrix_fprintf_empty() {
    // Empty matrix should produce no output.
    let m = Matrix::matrix_alloc(0, 0);
    let mut buf: Vec<u8> = Vec::new();
    m.matrix_fprintf(&mut buf, "%f");
    let s = String::from_utf8(buf).unwrap();
    assert_eq!(s, "");
}

fn main() {}
