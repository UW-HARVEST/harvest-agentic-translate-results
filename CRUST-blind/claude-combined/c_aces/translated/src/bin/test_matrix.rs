use c_aces::matrix::{
    fill_random_invertible_pairs, matrix2d_eye, matrix2d_multiply, Matrix2D, Matrix3D,
};

#[test]
fn test_matrix2d_new_zero() {
    let m = Matrix2D::new(3);
    assert_eq!(m.dim, 3);
    assert_eq!(m.data, vec![0u64; 9]);
}

#[test]
fn test_matrix2d_set_get() {
    let mut m = Matrix2D::new(3);
    m.set(0, 0, 7).unwrap();
    m.set(2, 1, 42).unwrap();
    assert_eq!(m.get(0, 0), Some(7));
    assert_eq!(m.get(2, 1), Some(42));
    assert_eq!(m.get(1, 1), Some(0));
}

#[test]
fn test_matrix2d_set_out_of_bounds() {
    let mut m = Matrix2D::new(3);
    assert!(m.set(3, 0, 1).is_err());
    assert!(m.set(0, 3, 1).is_err());
    assert!(m.get(3, 0).is_none());
    assert!(m.get(0, 3).is_none());
}

#[test]
fn test_matrix2d_row() {
    let mut m = Matrix2D::new(3);
    m.set(1, 0, 10).unwrap();
    m.set(1, 1, 20).unwrap();
    m.set(1, 2, 30).unwrap();
    let row = m.row(1).unwrap();
    assert_eq!(row, &[10, 20, 30]);
    assert!(m.row(3).is_none());
}

#[test]
fn test_matrix2d_row_mut() {
    let mut m = Matrix2D::new(3);
    {
        let row = m.row_mut(0).unwrap();
        row[0] = 5;
        row[1] = 6;
        row[2] = 7;
    }
    assert_eq!(m.row(0).unwrap(), &[5, 6, 7]);
    assert!(m.row_mut(3).is_none());
}

#[test]
fn test_matrix2d_eye() {
    let mut m = Matrix2D::new(3);
    m.data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    matrix2d_eye(&mut m).unwrap();
    assert_eq!(m.data, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]);
}

#[test]
fn test_matrix2d_multiply_identity() {
    // Multiplying by identity should preserve the matrix.
    let mut m1 = Matrix2D::new(3);
    m1.data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut id = Matrix2D::new(3);
    matrix2d_eye(&mut id).unwrap();
    let result = matrix2d_multiply(&m1, &id, 97).unwrap();
    assert_eq!(result.dim, 3);
    assert_eq!(result.data, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_matrix2d_multiply_known() {
    // From C scratch: m1=[1..9], m2=[9..1], mod=100
    // result = m1 * m2 with mod=100: 30 24 18 84 69 54 38 14 90
    let mut m1 = Matrix2D::new(3);
    m1.data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut m2 = Matrix2D::new(3);
    m2.data = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];
    let result = matrix2d_multiply(&m1, &m2, 100).unwrap();
    assert_eq!(result.data, vec![30, 24, 18, 84, 69, 54, 38, 14, 90]);
}

#[test]
fn test_matrix2d_multiply_dim_mismatch() {
    let m1 = Matrix2D::new(2);
    let m2 = Matrix2D::new(3);
    assert!(matrix2d_multiply(&m1, &m2, 7).is_err());
}

#[test]
fn test_matrix3d_new() {
    let m = Matrix3D::new(4, 3);
    assert_eq!(m.data.len(), 4);
    for sub in m.data.iter() {
        assert_eq!(sub.dim, 3);
        assert_eq!(sub.data, vec![0u64; 9]);
    }
}

#[test]
fn test_matrix3d_get_mut() {
    let mut m = Matrix3D::new(2, 2);
    {
        let entry = m.get_mut(0).unwrap();
        entry.set(0, 0, 7).unwrap();
    }
    assert_eq!(m.data[0].get(0, 0), Some(7));
    assert!(m.get_mut(2).is_none());
}

#[test]
fn test_fill_random_invertible_pairs() {
    // After fill_random_invertible_pairs, m * invm == identity (mod modulus).
    let mut m = Matrix2D::new(3);
    let mut invm = Matrix2D::new(3);
    fill_random_invertible_pairs(&mut m, &mut invm, 97, 600).unwrap();
    let prod = matrix2d_multiply(&m, &invm, 97).unwrap();
    assert_eq!(prod.data, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]);
    assert_eq!(prod.dim, 3);
}

#[test]
fn test_fill_random_invertible_pairs_zero_iterations() {
    // With 0 iterations, both should remain identity.
    let mut m = Matrix2D::new(4);
    let mut invm = Matrix2D::new(4);
    fill_random_invertible_pairs(&mut m, &mut invm, 97, 0).unwrap();
    let mut id = Matrix2D::new(4);
    matrix2d_eye(&mut id).unwrap();
    assert_eq!(m.data, id.data);
    assert_eq!(invm.data, id.data);
}

fn main() {}
