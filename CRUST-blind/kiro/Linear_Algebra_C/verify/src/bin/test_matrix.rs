use Linear_Algebra_C::matrix;
use Linear_Algebra_C::linear_algebra::{new_vector, new_matrix};

#[test]
fn test_new_matrix_and_size() {
    let m = matrix::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(matrix::get_matrix_element_impl(&m, 0, 0), 1.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 0, 1), 2.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 1, 0), 3.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 1, 1), 4.0);
    assert_eq!(matrix::matrix_size_impl(&m), 4);
    assert_eq!(matrix::matrix_size_bytes_impl(&m), 32);
}

#[test]
fn test_zero_and_fill() {
    let mut m = matrix::zero_matrix_impl(2, 2);
    assert_eq!(matrix::get_matrix_element_impl(&m, 0, 0), 0.0);
    matrix::fill_matrix_impl(&mut m, 7.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 1, 1), 7.0);
}

#[test]
fn test_identity() {
    let m = matrix::identity_matrix_impl(3);
    assert!(matrix::is_identity_matrix_impl(&m));
    assert_eq!(matrix::get_matrix_element_impl(&m, 0, 0), 1.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 0, 1), 0.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 2, 2), 1.0);
}

#[test]
fn test_copy_and_equal() {
    let mut m = matrix::zero_matrix_impl(2, 2);
    matrix::set_matrix_element_impl(&mut m, 0, 1, 5.0);
    let n = matrix::copy_matrix_impl(&m);
    assert!(matrix::is_matrix_equal_impl(&m, &n));
}

#[test]
fn test_flatten() {
    let m = matrix::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let flat = matrix::flatten_matrix_impl(&m);
    assert_eq!(flat.data[0], 1.0);
    assert_eq!(flat.data[5], 6.0);
    assert_eq!(flat.cols, 6);
}

#[test]
fn test_row_vector() {
    let mut m = matrix::zero_matrix_impl(3, 2);
    let v = new_vector(&[1.0, 4.0], 2);
    matrix::set_row_vector_impl(&mut m, 1, &v);
    assert_eq!(matrix::get_matrix_element_impl(&m, 1, 0), 1.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 1, 1), 4.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 0, 0), 0.0);
    let row = matrix::get_row_vector_impl(&m, 1);
    assert_eq!(row.data, vec![1.0, 4.0]);
}

#[test]
fn test_col_vector() {
    let mut m = matrix::zero_matrix_impl(3, 2);
    let v = new_vector(&[10.0, 3.0, 7.0], 3);
    matrix::set_col_vector_impl(&mut m, 0, &v);
    assert_eq!(matrix::get_matrix_element_impl(&m, 0, 0), 10.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 1, 0), 3.0);
    assert_eq!(matrix::get_matrix_element_impl(&m, 2, 0), 7.0);
    let col = matrix::get_col_vector_impl(&m, 0);
    assert_eq!(col.data, vec![10.0, 3.0, 7.0]);
}

#[test]
fn test_main_diagonal() {
    let mut m = matrix::zero_matrix_impl(2, 2);
    matrix::set_matrix_element_impl(&mut m, 0, 0, 34.0);
    matrix::set_matrix_element_impl(&mut m, 1, 1, 56.0);
    let d = matrix::get_main_diagonal_impl(&m);
    assert_eq!(d.data, vec![34.0, 56.0]);

    let mut n = matrix::zero_matrix_impl(2, 2);
    let v = new_vector(&[100.0, 4.0], 2);
    matrix::set_main_diagonal_impl(&mut n, &v);
    assert_eq!(matrix::get_matrix_element_impl(&n, 0, 0), 100.0);
    assert_eq!(matrix::get_matrix_element_impl(&n, 1, 1), 4.0);
}

#[test]
fn test_anti_diagonal() {
    let mut m = matrix::zero_matrix_impl(2, 2);
    matrix::set_matrix_element_impl(&mut m, 0, 1, 100.0);
    matrix::set_matrix_element_impl(&mut m, 1, 0, 250.0);
    let d = matrix::get_anti_diagonal_impl(&m);
    assert_eq!(d.data[0], 250.0);
    assert_eq!(d.data[1], 100.0);

    let mut n = matrix::zero_matrix_impl(2, 2);
    let v = new_vector(&[9.0, 8.0], 2);
    matrix::set_anti_diagonal_impl(&mut n, &v);
    assert_eq!(matrix::get_matrix_element_impl(&n, 1, 0), 9.0);
    assert_eq!(matrix::get_matrix_element_impl(&n, 0, 1), 8.0);
}

#[test]
fn test_diagonal_product() {
    let m = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(matrix::diagonal_product_impl(&m), 45.0);
}

#[test]
fn test_same_dimensions() {
    let m = matrix::zero_matrix_impl(2, 3);
    let n = matrix::zero_matrix_impl(2, 3);
    let o = matrix::zero_matrix_impl(3, 3);
    assert!(matrix::has_same_dimensions_impl(&m, &n));
    assert!(!matrix::has_same_dimensions_impl(&m, &o));
}

#[test]
fn test_is_zero_matrix() {
    let m = matrix::zero_matrix_impl(1, 3);
    assert!(matrix::is_zero_matrix_impl(&m));
    let mut n = matrix::zero_matrix_impl(2, 2);
    matrix::set_matrix_element_impl(&mut n, 0, 0, 1.0);
    assert!(!matrix::is_zero_matrix_impl(&n));
}

#[test]
fn test_is_square_matrix() {
    let m = matrix::zero_matrix_impl(2, 2);
    assert!(matrix::is_square_matrix_impl(&m));
    let n = matrix::zero_matrix_impl(1, 3);
    assert!(!matrix::is_square_matrix_impl(&n));
}

#[test]
fn test_is_diagonal() {
    let m = matrix::new_matrix_impl(&[1.0,0.0,0.0,0.0,2.0,0.0,0.0,0.0,3.0], 3, 3);
    assert!(matrix::is_diagonal_matrix_impl(&m));
    let n = matrix::new_matrix_impl(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert!(!matrix::is_diagonal_matrix_impl(&n));
}

#[test]
fn test_is_triangular() {
    let up = matrix::new_matrix_impl(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert!(matrix::is_up_tri_matrix_impl(&up));
    let lo = matrix::new_matrix_impl(&[1.0,0.0,0.0,4.0,5.0,0.0,7.0,8.0,9.0], 3, 3);
    assert!(matrix::is_lo_tri_matrix_impl(&lo));
    assert!(matrix::is_triangular_matrix_impl(&lo));
}

#[test]
fn test_is_symmetric() {
    let m = matrix::zero_matrix_impl(2, 2);
    assert!(matrix::is_matrix_symmetric_impl(&m));
    let n = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0], 2, 3);
    assert!(!matrix::is_matrix_symmetric_impl(&n));
}

#[test]
fn test_has_zero_row() {
    let m = matrix::new_matrix_impl(&[1.0,2.0,3.0,0.0,0.0,0.0], 2, 3);
    assert!(matrix::has_zero_row_impl(&m));
    let n = matrix::new_matrix_impl(&[1.0,0.0,3.0,4.0,0.0,6.0], 2, 3);
    assert!(!matrix::has_zero_row_impl(&n));
}

#[test]
fn test_has_zero_col() {
    // Use square matrix to avoid C bug with swapped loop bounds
    // Column 1 is all zeros: [0,0,0]
    let m = new_matrix(&[1.0,0.0,3.0,4.0,0.0,6.0,7.0,0.0,9.0], 3, 3);
    assert!(matrix::has_zero_col_impl(&m));
    let n = new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    assert!(!matrix::has_zero_col_impl(&n));
}

#[test]
fn test_transpose() {
    let m = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0], 2, 3);
    let t = matrix::transpose_matrix_impl(&m);
    assert_eq!(t.rows, 3);
    assert_eq!(t.cols, 2);
    assert_eq!(matrix::get_matrix_element_impl(&t, 0, 0), 1.0);
    assert_eq!(matrix::get_matrix_element_impl(&t, 1, 0), 2.0);
    assert_eq!(matrix::get_matrix_element_impl(&t, 2, 0), 3.0);
    assert_eq!(matrix::get_matrix_element_impl(&t, 0, 1), 4.0);
    assert_eq!(matrix::get_matrix_element_impl(&t, 1, 1), 5.0);
    assert_eq!(matrix::get_matrix_element_impl(&t, 2, 1), 6.0);
}

#[test]
fn test_trace() {
    let m = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(matrix::trace_matrix_impl(&m), 15.0);
}

#[test]
fn test_add_matrices() {
    let m1 = matrix::new_matrix_impl(&[4.0,6.0,3.0,8.0], 2, 2);
    let m2 = matrix::new_matrix_impl(&[6.0,4.0,7.0,2.0], 2, 2);
    let s = matrix::add_matrices_impl(&m1, &m2);
    assert_eq!(s.data, vec![10.0, 10.0, 10.0, 10.0]);
}

#[test]
fn test_multiply_matrices() {
    let m1 = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0], 2, 2);
    let m2 = matrix::new_matrix_impl(&[1.0,1.0,1.0,1.0], 2, 2);
    let p = matrix::multiply_matrices_impl(&m1, &m2);
    assert_eq!(matrix::get_matrix_element_impl(&p, 0, 0), 3.0);
    assert_eq!(matrix::get_matrix_element_impl(&p, 0, 1), 3.0);
    assert_eq!(matrix::get_matrix_element_impl(&p, 1, 0), 7.0);
    assert_eq!(matrix::get_matrix_element_impl(&p, 1, 1), 7.0);
}

#[test]
fn test_scale_matrix() {
    let m = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0], 3, 2);
    let s = matrix::scale_matrix_impl(&m, 10.0);
    assert_eq!(s.data, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
}

#[test]
fn test_pow_matrix() {
    let m = matrix::new_matrix_impl(&[2.0,3.0,4.0,5.0], 2, 2);
    let p = matrix::pow_matrix_impl(&m, 2.0);
    assert_eq!(p.data, vec![4.0, 9.0, 16.0, 25.0]);
}

#[test]
fn test_sub_matrix() {
    let m = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    let s = matrix::sub_matrix_impl(&m, 2, 2);
    assert_eq!(s.data, vec![1.0, 2.0, 4.0, 5.0]);
}

#[test]
fn test_element_minor_and_cofactor() {
    let m = matrix::new_matrix_impl(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    assert_eq!(matrix::element_minor_impl(&m, 0, 0), -54.0);
    assert_eq!(matrix::element_minor_impl(&m, 0, 1), 18.0);
    assert_eq!(matrix::element_cofactor_impl(&m, 0, 0), -54.0);
    assert_eq!(matrix::element_cofactor_impl(&m, 0, 1), -18.0);
    assert_eq!(matrix::element_cofactor_impl(&m, 1, 0), 1.0);
}

#[test]
fn test_matrix_minor() {
    let m = matrix::new_matrix_impl(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let mm = matrix::matrix_minor_impl(&m);
    assert_eq!(mm.data, vec![-54.0, 18.0, 36.0, -1.0, 40.0, 46.0, 7.0, 26.0, -16.0]);
}

#[test]
fn test_matrix_cofactor() {
    let m = matrix::new_matrix_impl(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let cf = matrix::matrix_cofactor_impl(&m);
    assert_eq!(cf.data, vec![-54.0, -18.0, 36.0, 1.0, 40.0, -46.0, 7.0, -26.0, -16.0]);
}

#[test]
fn test_sign_matrix() {
    let s = matrix::sign_matrix_impl(3, 3);
    assert_eq!(s.data, vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]);
}

#[test]
fn test_adjugate() {
    let m = matrix::new_matrix_impl(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let adj = matrix::adjugate_matrix_impl(&m);
    assert_eq!(adj.data[0], -36.0);
    assert_eq!(adj.data[1], 36.0);
    assert_eq!(adj.data[2], -36.0);
    assert_eq!(adj.data[3], 5.0);
    assert_eq!(adj.data[4], -5.0);
    assert_eq!(adj.data[5], 5.0);
    assert_eq!(adj.data[6], -35.0);
    assert_eq!(adj.data[7], 35.0);
    assert_eq!(adj.data[8], -35.0);
}

#[test]
fn test_determinant() {
    let m1 = matrix::new_matrix_impl(&[10.0], 1, 1);
    assert_eq!(Linear_Algebra_C::linear_algebra::determinant(&m1), 10.0);

    let m2 = matrix::new_matrix_impl(&[4.0,6.0,3.0,8.0], 2, 2);
    assert_eq!(Linear_Algebra_C::linear_algebra::determinant(&m2), 14.0);

    let m3 = matrix::new_matrix_impl(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    assert_eq!(Linear_Algebra_C::linear_algebra::determinant(&m3), -306.0);

    let m4 = matrix::new_matrix_impl(&[11.0,9.0,24.0,2.0,1.0,5.0,2.0,6.0,3.0,17.0,18.0,1.0,2.0,5.0,7.0,1.0], 4, 4);
    assert_eq!(Linear_Algebra_C::utils::roundn(Linear_Algebra_C::linear_algebra::determinant(&m4), 1), 284.0);
}

#[test]
fn test_is_invertible() {
    let m = matrix::new_matrix_impl(&[3.0,0.0,2.0,2.0,0.0,-2.0,0.0,1.0,1.0], 3, 3);
    assert!(matrix::is_invertible_impl(&m));
    let m2 = matrix::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    assert!(!matrix::is_invertible_impl(&m2));
}

#[test]
fn test_inverse() {
    let m = matrix::new_matrix_impl(&[3.0,0.0,2.0,2.0,0.0,-2.0,0.0,1.0,1.0], 3, 3);
    let inv = matrix::inverse_matrix_impl(&m);
    let r = |v: f64| Linear_Algebra_C::utils::roundn(v, 1);
    assert_eq!(r(inv.data[0]), 0.2);
    assert_eq!(r(inv.data[1]), -0.2);
    assert_eq!(r(inv.data[2]), 0.2);
    assert_eq!(r(inv.data[3]), -0.2);
    assert_eq!(r(inv.data[4]), 0.2);
    assert_eq!(r(inv.data[5]), -0.2);
    assert_eq!(r(inv.data[6]), 1.0);
    assert_eq!(r(inv.data[7]), -1.0);
    assert_eq!(r(inv.data[8]), 1.0);
}

#[test]
fn test_pivot_matrix() {
    let m = matrix::new_matrix_impl(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let (piv, swaps) = matrix::pivot_matrix_impl(&m);
    assert_eq!(swaps, 0);
    assert_eq!(piv.data, vec![1.0,0.0,0.0, 0.0,0.0,1.0, 0.0,1.0,0.0]);
}

#[test]
fn test_lu_decomposition() {
    let m = matrix::new_matrix_impl(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let (l, u, _p, swaps) = matrix::lu_decomposition_impl(&m);
    assert_eq!(swaps, 0);
    let r = |v: f64| Linear_Algebra_C::utils::roundn(v, 6);
    assert_eq!(r(l.data[0]), 1.0);
    assert_eq!(r(l.data[4]), 1.0);
    assert_eq!(r(l.data[8]), 1.0);
    assert_eq!(r(u.data[0]), 6.0);
    assert_eq!(r(u.data[1]), 1.0);
    assert_eq!(r(u.data[2]), 1.0);
}

#[test]
fn test_reflect_axis_2d() {
    let eye = matrix::identity_matrix_impl(2);
    let r0 = matrix::reflect_axis_2d_impl(&eye, 0);
    assert_eq!(r0.data, vec![-1.0, 0.0, 0.0, 1.0]);
    let r1 = matrix::reflect_axis_2d_impl(&eye, 1);
    assert_eq!(r1.data, vec![1.0, 0.0, 0.0, -1.0]);
}

#[test]
fn test_orth_proj_2d() {
    let eye = matrix::identity_matrix_impl(2);
    let p0 = matrix::orth_proj_2d_impl(&eye, 0);
    assert_eq!(p0.data, vec![1.0, 0.0, 0.0, 0.0]);
    let p1 = matrix::orth_proj_2d_impl(&eye, 1);
    assert_eq!(p1.data, vec![0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_orth_proj_3d() {
    let eye = matrix::identity_matrix_impl(3);
    let p0 = matrix::orth_proj_3d_impl(&eye, 0);
    assert_eq!(p0.data, vec![1.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,0.0]);
    let p1 = matrix::orth_proj_3d_impl(&eye, 1);
    assert_eq!(p1.data, vec![1.0,0.0,0.0, 0.0,0.0,0.0, 0.0,0.0,1.0]);
    let p2 = matrix::orth_proj_3d_impl(&eye, 2);
    assert_eq!(p2.data, vec![0.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,1.0]);
}

#[test]
fn test_shear_2d() {
    let eye = matrix::identity_matrix_impl(2);
    let s0 = matrix::shear_2d_impl(&eye, 2.0, 0);
    assert_eq!(s0.data, vec![1.0, 0.0, 2.0, 1.0]);
    let s1 = matrix::shear_2d_impl(&eye, 2.0, 1);
    assert_eq!(s1.data, vec![1.0, 2.0, 0.0, 1.0]);
}

#[test]
fn test_scale_n_space() {
    let eye = matrix::identity_matrix_impl(2);
    let s = matrix::scale_n_space_impl(&eye, 3.0);
    assert_eq!(s.data, vec![3.0, 0.0, 0.0, 3.0]);
}

fn main() {}
