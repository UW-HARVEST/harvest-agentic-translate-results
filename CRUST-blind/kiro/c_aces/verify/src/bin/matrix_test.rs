use c_aces::matrix::{Matrix2D, Matrix3D, matrix2d_eye, matrix2d_multiply};

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
    m.set(1, 2, 42).unwrap();
    assert_eq!(m.get(1, 2), Some(42));
    assert_eq!(m.get(0, 0), Some(0));
    assert_eq!(m.get(3, 0), None);
    assert_eq!(m.get(0, 3), None);
}

#[test]
fn test_matrix2d_row() {
    let mut m = Matrix2D::new(3);
    m.set(1, 0, 10).unwrap();
    m.set(1, 1, 20).unwrap();
    m.set(1, 2, 30).unwrap();
    assert_eq!(m.row(1), Some(&[10u64, 20, 30][..]));
    assert_eq!(m.row(3), None);
}

#[test]
fn test_matrix2d_eye() {
    let mut m = Matrix2D::new(3);
    m.data.fill(99);
    matrix2d_eye(&mut m).unwrap();
    assert_eq!(m.data, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]);
}

#[test]
fn test_matrix2d_multiply_identity() {
    let mut m1 = Matrix2D::new(3);
    m1.data = vec![2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut m2 = Matrix2D::new(3);
    matrix2d_eye(&mut m2).unwrap();
    let res = matrix2d_multiply(&m1, &m2, 97).unwrap();
    assert_eq!(res.data, vec![2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_matrix2d_multiply_general() {
    let mut m1 = Matrix2D::new(2);
    m1.data = vec![1, 2, 3, 4];
    let mut m2 = Matrix2D::new(2);
    m2.data = vec![5, 6, 7, 8];
    let res = matrix2d_multiply(&m1, &m2, 100).unwrap();
    assert_eq!(res.data, vec![19, 22, 43, 50]);
}

#[test]
fn test_matrix3d_new() {
    let m = Matrix3D::new(3, 2);
    assert_eq!(m.data.len(), 3);
    for mat in &m.data {
        assert_eq!(mat.dim, 2);
        assert_eq!(mat.data.len(), 4);
    }
}

#[test]
fn test_matrix3d_get_mut() {
    let mut m = Matrix3D::new(2, 3);
    let inner = m.get_mut(0).unwrap();
    inner.set(0, 0, 99).unwrap();
    assert_eq!(m.data[0].get(0, 0), Some(99));
    assert!(m.get_mut(5).is_none());
}

fn main() {}
