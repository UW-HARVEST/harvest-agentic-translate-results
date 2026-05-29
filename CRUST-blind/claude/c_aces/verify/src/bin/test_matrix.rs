#![allow(unused_imports)]
use c_aces::matrix::{
    fill_random_invertible_pairs, linear_mix_transform, matrix2d_eye, matrix2d_multiply,
    scale_transform, swap_transform, Matrix2D, Matrix3D,
};

#[test]
fn test_matrix2d_new() {
    let m = Matrix2D::new(3);
    assert_eq!(m.dim, 3);
    assert_eq!(m.data.len(), 9);
    for v in m.data.iter() {
        assert_eq!(*v, 0);
    }
}

#[test]
fn test_matrix2d_eye() {
    let mut m = Matrix2D::new(3);
    matrix2d_eye(&mut m).unwrap();
    let expected: Vec<u64> = vec![1, 0, 0, 0, 1, 0, 0, 0, 1];
    assert_eq!(m.data, expected);
}

#[test]
fn test_matrix2d_eye_4x4() {
    let mut m = Matrix2D::new(4);
    matrix2d_eye(&mut m).unwrap();
    let expected: Vec<u64> = vec![
        1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1,
    ];
    assert_eq!(m.data, expected);
}

#[test]
fn test_matrix2d_get_set() {
    let mut m = Matrix2D::new(3);
    m.set(0, 0, 1).unwrap();
    m.set(0, 1, 2).unwrap();
    m.set(1, 2, 5).unwrap();
    assert_eq!(m.get(0, 0).unwrap(), 1);
    assert_eq!(m.get(0, 1).unwrap(), 2);
    assert_eq!(m.get(1, 2).unwrap(), 5);
    assert_eq!(m.get(2, 2).unwrap(), 0);
    // Out of bounds
    assert!(m.get(3, 0).is_none());
    assert!(m.get(0, 3).is_none());
    assert!(m.set(3, 0, 1).is_err());
}

#[test]
fn test_matrix2d_row() {
    let mut m = Matrix2D::new(3);
    for i in 0..3 {
        for j in 0..3 {
            m.set(i, j, (i * 3 + j) as u64).unwrap();
        }
    }
    let r0 = m.row(0).unwrap();
    assert_eq!(r0, &[0u64, 1, 2]);
    let r1 = m.row(1).unwrap();
    assert_eq!(r1, &[3u64, 4, 5]);
    let r2 = m.row(2).unwrap();
    assert_eq!(r2, &[6u64, 7, 8]);
    assert!(m.row(3).is_none());
}

#[test]
fn test_matrix2d_multiply_identity() {
    // C: M * I = M, where M = [[1,2,3],[4,5,6],[7,8,9]]
    let mut m1 = Matrix2D::new(3);
    m1.data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut m2 = Matrix2D::new(3);
    matrix2d_eye(&mut m2).unwrap();
    let r = matrix2d_multiply(&m1, &m2, 100).unwrap();
    assert_eq!(r.data, vec![1u64, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(r.dim, 3);
}

#[test]
fn test_matrix2d_multiply_standard() {
    // C: A*B mod 1000 -> [30, 24, 18, 84, 69, 54, 138, 114, 90]
    let mut a = Matrix2D::new(3);
    a.data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut b = Matrix2D::new(3);
    b.data = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];
    let r = matrix2d_multiply(&a, &b, 1000).unwrap();
    assert_eq!(r.data, vec![30u64, 24, 18, 84, 69, 54, 138, 114, 90]);
}

#[test]
fn test_matrix2d_multiply_with_modulus() {
    // C: A*B mod 50 -> [30, 24, 18, 34, 19, 4, 38, 14, 40]
    let mut a = Matrix2D::new(3);
    a.data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut b = Matrix2D::new(3);
    b.data = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];
    let r = matrix2d_multiply(&a, &b, 50).unwrap();
    assert_eq!(r.data, vec![30u64, 24, 18, 34, 19, 4, 38, 14, 40]);
}

#[test]
fn test_matrix2d_multiply_2x2() {
    // C: 2x2 mul: [19, 22, 43, 50]
    let mut a = Matrix2D::new(2);
    a.data = vec![1, 2, 3, 4];
    let mut b = Matrix2D::new(2);
    b.data = vec![5, 6, 7, 8];
    let r = matrix2d_multiply(&a, &b, 1000).unwrap();
    assert_eq!(r.data, vec![19u64, 22, 43, 50]);
}

#[test]
fn test_matrix3d_new() {
    let m3 = Matrix3D::new(4, 3);
    assert_eq!(m3.data.len(), 4);
    for m in m3.data.iter() {
        assert_eq!(m.dim, 3);
        assert_eq!(m.data.len(), 9);
    }
}

#[test]
fn test_matrix3d_get_mut() {
    let mut m3 = Matrix3D::new(4, 3);
    {
        let m = m3.get_mut(2).unwrap();
        m.set(0, 0, 42).unwrap();
    }
    assert_eq!(m3.data[2].get(0, 0).unwrap(), 42);
    assert!(m3.get_mut(4).is_none());
}

#[test]
fn test_fill_random_invertible_pairs_yields_inverses() {
    // Per C test: fill_random_invertible_pairs(m, invm, 97, 600) and m*invm = I
    for _ in 0..3 {
        let mut m = Matrix2D::new(3);
        let mut invm = Matrix2D::new(3);
        fill_random_invertible_pairs(&mut m, &mut invm, 97, 600).unwrap();
        let r = matrix2d_multiply(&m, &invm, 97).unwrap();
        let mut expected = Matrix2D::new(3);
        matrix2d_eye(&mut expected).unwrap();
        assert_eq!(r.data, expected.data);
    }
}

#[test]
fn test_fill_random_invertible_pairs_dim4() {
    let mut m = Matrix2D::new(4);
    let mut invm = Matrix2D::new(4);
    fill_random_invertible_pairs(&mut m, &mut invm, 101, 600).unwrap();
    let r = matrix2d_multiply(&m, &invm, 101).unwrap();
    let mut expected = Matrix2D::new(4);
    matrix2d_eye(&mut expected).unwrap();
    assert_eq!(r.data, expected.data);
}

#[test]
fn test_swap_transform_preserves_inverse() {
    // After swap_transform on (m, invm) starting from identity, m * invm should still
    // be identity.
    let mut m = Matrix2D::new(4);
    let mut invm = Matrix2D::new(4);
    matrix2d_eye(&mut m).unwrap();
    matrix2d_eye(&mut invm).unwrap();
    swap_transform(&mut m, &mut invm, 97).unwrap();
    let r = matrix2d_multiply(&m, &invm, 97).unwrap();
    let mut expected = Matrix2D::new(4);
    matrix2d_eye(&mut expected).unwrap();
    assert_eq!(r.data, expected.data);
}

#[test]
fn test_scale_transform_preserves_inverse() {
    let mut m = Matrix2D::new(4);
    let mut invm = Matrix2D::new(4);
    matrix2d_eye(&mut m).unwrap();
    matrix2d_eye(&mut invm).unwrap();
    scale_transform(&mut m, &mut invm, 97).unwrap();
    let r = matrix2d_multiply(&m, &invm, 97).unwrap();
    let mut expected = Matrix2D::new(4);
    matrix2d_eye(&mut expected).unwrap();
    assert_eq!(r.data, expected.data);
}

#[test]
fn test_linear_mix_transform_preserves_inverse() {
    let mut m = Matrix2D::new(4);
    let mut invm = Matrix2D::new(4);
    matrix2d_eye(&mut m).unwrap();
    matrix2d_eye(&mut invm).unwrap();
    linear_mix_transform(&mut m, &mut invm, 97).unwrap();
    let r = matrix2d_multiply(&m, &invm, 97).unwrap();
    let mut expected = Matrix2D::new(4);
    matrix2d_eye(&mut expected).unwrap();
    assert_eq!(r.data, expected.data);
}

#[test]
fn test_matrix2d_multiply_dimension_mismatch() {
    let m1 = Matrix2D::new(3);
    let m2 = Matrix2D::new(4);
    assert!(matrix2d_multiply(&m1, &m2, 7).is_err());
}

#[test]
fn test_matrix2d_multiply_zero_modulus() {
    let m1 = Matrix2D::new(3);
    let m2 = Matrix2D::new(3);
    assert!(matrix2d_multiply(&m1, &m2, 0).is_err());
}

fn main() {}
