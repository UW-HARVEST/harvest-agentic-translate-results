use c_aces::matrix::{matrix2d_eye, matrix2d_multiply, fill_random_invertible_pairs, Matrix2D};

#[test]
fn test_matrix2d_new() {
    let m = Matrix2D::new(3);
    assert_eq!(m.dim, 3);
    assert_eq!(m.data.len(), 9);
    assert!(m.data.iter().all(|&v| v == 0));
}

#[test]
fn test_matrix2d_get_set() {
    let mut m = Matrix2D::new(3);
    m.set(0, 0, 42).unwrap();
    m.set(1, 2, 99).unwrap();
    assert_eq!(m.get(0, 0), Some(42));
    assert_eq!(m.get(1, 2), Some(99));
    assert_eq!(m.get(0, 1), Some(0));
    assert_eq!(m.get(3, 0), None);
    assert!(m.set(3, 0, 1).is_err());
}

#[test]
fn test_matrix2d_eye() {
    let mut m = Matrix2D::new(3);
    m.set(0, 1, 5).unwrap();
    matrix2d_eye(&mut m).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            let expected = if i == j { 1 } else { 0 };
            assert_eq!(m.get(i, j), Some(expected));
        }
    }
}

#[test]
fn test_matrix2d_multiply_identity() {
    // M * I = M (mod 97)
    let mut m = Matrix2D::new(3);
    let vals: Vec<u64> = vec![10, 20, 30, 40, 50, 60, 70, 80, 90];
    for (i, &v) in vals.iter().enumerate() {
        m.data[i] = v;
    }
    let mut eye = Matrix2D::new(3);
    matrix2d_eye(&mut eye).unwrap();

    let result = matrix2d_multiply(&m, &eye, 97).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            assert_eq!(result.get(i, j).unwrap(), m.get(i, j).unwrap() % 97);
        }
    }
}

#[test]
fn test_matrix2d_multiply_mod() {
    // 2x2 multiply with modulus
    let mut a = Matrix2D::new(2);
    a.data = vec![2, 3, 4, 5];
    let mut b = Matrix2D::new(2);
    b.data = vec![1, 0, 0, 1]; // identity
    let r = matrix2d_multiply(&a, &b, 10).unwrap();
    assert_eq!(r.data, vec![2, 3, 4, 5]);
}

#[test]
fn test_fill_random_invertible_pairs() {
    // M * M_inv = I (mod 97)
    let mut m = Matrix2D::new(3);
    let mut invm = Matrix2D::new(3);
    fill_random_invertible_pairs(&mut m, &mut invm, 97, 600).unwrap();

    let result = matrix2d_multiply(&m, &invm, 97).unwrap();
    let mut expected = Matrix2D::new(3);
    matrix2d_eye(&mut expected).unwrap();
    assert_eq!(result.data, expected.data);
}

#[test]
fn test_matrix2d_row() {
    let mut m = Matrix2D::new(3);
    m.data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    assert_eq!(m.row(0), Some(&[1u64, 2, 3][..]));
    assert_eq!(m.row(1), Some(&[4u64, 5, 6][..]));
    assert_eq!(m.row(2), Some(&[7u64, 8, 9][..]));
    assert_eq!(m.row(3), None);
}

fn main() {}
