use Linear_Algebra_C::linear_algebra as la;

#[test]
fn test_assert_matrix_and_vector() {
    let m = la::null_matrix(2, 2);
    let v = la::null_vector(3);
    assert_eq!(la::assert_matrix(&m), true);
    assert_eq!(la::assert_vector(&v), true);
}

#[test]
fn test_new_matrix() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 2);
    assert_eq!(m.data, vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(la::matrix_size(&m), 4);
    assert_eq!(la::matrix_size_bytes(&m), 32);
}

#[test]
fn test_new_vector() {
    let v = la::new_vector(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 8);
    assert_eq!(v.cols, 8);
    assert_eq!(la::vector_size(&v), 8);
    assert_eq!(la::vector_size_bytes(&v), 64);
}

#[test]
fn test_null_zero_matrix() {
    let n = la::null_matrix(2, 3);
    assert_eq!(n.rows, 2);
    assert_eq!(n.cols, 3);
    let z = la::zero_matrix(2, 3);
    assert_eq!(z.rows, 2);
    assert_eq!(z.cols, 3);
    assert!(la::is_zero_matrix(&z));
}

#[test]
fn test_null_zero_vector() {
    let n = la::null_vector(4);
    assert_eq!(n.cols, 4);
    let z = la::zero_vector(4);
    assert_eq!(z.cols, 4);
    assert_eq!(z.data, vec![0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_fill_matrix_and_vector() {
    let mut m = la::null_matrix(2, 2);
    la::fill_matrix(&mut m, 9.0);
    assert_eq!(m.data, vec![9.0, 9.0, 9.0, 9.0]);

    let mut v = la::null_vector(3);
    la::fill_vector(&mut v, -1.5);
    assert_eq!(v.data, vec![-1.5, -1.5, -1.5]);
}

#[test]
fn test_identity_matrix() {
    let id = la::identity_matrix(3);
    assert_eq!(id.rows, 3);
    assert_eq!(id.cols, 3);
    assert_eq!(id.data, vec![1.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,1.0]);
}

#[test]
fn test_delete_matrix_and_vector_run() {
    la::delete_matrix(la::zero_matrix(1, 1));
    la::delete_vector(la::zero_vector(1));
}

#[test]
fn test_copy_matrix_and_vector() {
    let mut m = la::zero_matrix(2, 2);
    la::set_matrix_element(&mut m, 0, 1, 5.0);
    let c = la::copy_matrix(&m);
    assert_eq!(c.data, m.data);
    assert_eq!(la::get_matrix_element(&c, 0, 0), 0.0);
    assert_eq!(la::get_matrix_element(&c, 0, 1), 5.0);

    let mut v = la::zero_vector(4);
    la::set_vector_element(&mut v, 2, 2.0);
    let cv = la::copy_vector(&v);
    assert_eq!(cv.data, v.data);
    assert_eq!(la::get_vector_element(&cv, 2), 2.0);
}

#[test]
fn test_flatten_matrix() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let f = la::flatten_matrix(&m);
    assert_eq!(f.cols, 6);
    assert_eq!(f.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(la::get_vector_element(&f, 0), 1.0);
    assert_eq!(la::get_vector_element(&f, 5), 6.0);
}

#[test]
fn test_set_get_matrix_element() {
    let mut m = la::zero_matrix(2, 2);
    la::set_matrix_element(&mut m, 0, 0, 4.0);
    assert_eq!(la::get_matrix_element(&m, 0, 0), 4.0);
    assert_eq!(la::get_matrix_element(&m, 0, 1), 0.0);
}

#[test]
fn test_set_get_vector_element() {
    let mut v = la::zero_vector(2);
    la::set_vector_element(&mut v, 1, 10.0);
    assert_eq!(la::get_vector_element(&v, 1), 10.0);
}

#[test]
fn test_row_vector_ops() {
    let mut m = la::zero_matrix(3, 2);
    let v = la::new_vector(&[1.0, 4.0], 2);
    la::set_row_vector(&mut m, 1, &v);
    assert_eq!(la::get_matrix_element(&m, 0, 0), 0.0);
    assert_eq!(la::get_matrix_element(&m, 0, 1), 0.0);
    assert_eq!(la::get_matrix_element(&m, 1, 0), 1.0);
    assert_eq!(la::get_matrix_element(&m, 1, 1), 4.0);
    let r = la::get_row_vector(&m, 1);
    assert_eq!(r.data, vec![1.0, 4.0]);
}

#[test]
fn test_col_vector_ops() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let c0 = la::get_col_vector(&m, 0);
    assert_eq!(c0.data, vec![1.0, 3.0, 5.0]);
    let c1 = la::get_col_vector(&m, 1);
    assert_eq!(c1.data, vec![2.0, 4.0, 6.0]);

    let mut z = la::zero_matrix(3, 2);
    let v = la::new_vector(&[7.0, 8.0, 9.0], 3);
    la::set_col_vector(&mut z, 1, &v);
    assert_eq!(la::get_matrix_element(&z, 0, 1), 7.0);
    assert_eq!(la::get_matrix_element(&z, 1, 1), 8.0);
    assert_eq!(la::get_matrix_element(&z, 2, 1), 9.0);
}

#[test]
fn test_main_diagonal_ops() {
    let mut m = la::zero_matrix(2, 2);
    la::set_matrix_element(&mut m, 0, 0, 34.0);
    la::set_matrix_element(&mut m, 1, 1, 56.0);
    let v = la::get_main_diagonal(&m);
    assert_eq!(v.data, vec![34.0, 56.0]);

    let mut n = la::zero_matrix(2, 2);
    let w = la::new_vector(&[100.0, 4.0], 2);
    la::set_main_diagonal(&mut n, &w);
    assert_eq!(la::get_matrix_element(&n, 0, 0), 100.0);
    assert_eq!(la::get_matrix_element(&n, 1, 1), 4.0);
}

#[test]
fn test_anti_diagonal_ops() {
    let mut m = la::zero_matrix(2, 2);
    la::set_matrix_element(&mut m, 0, 1, 100.0);
    la::set_matrix_element(&mut m, 1, 0, 250.0);
    let v = la::get_anti_diagonal(&m);
    assert_eq!(v.data, vec![250.0, 100.0]);

    let mut n = la::zero_matrix(2, 2);
    let e = la::new_vector(&[9.0, 8.0], 2);
    la::set_anti_diagonal(&mut n, &e);
    assert_eq!(la::get_matrix_element(&n, 1, 0), 9.0);
    assert_eq!(la::get_matrix_element(&n, 0, 1), 8.0);
}

#[test]
fn test_diagonal_product() {
    let m = la::new_matrix(&[2.0, 1.0, 1.0, 1.0, 3.0, 1.0, 1.0, 1.0, 4.0], 3, 3);
    assert_eq!(la::diagonal_product(&m), 24.0);
}

#[test]
fn test_print_runs() {
    let m = la::identity_matrix(2);
    la::print_matrix(&m, true);
    la::print_matrix(&m, false);
    let v = la::new_vector(&[1.0, 2.0], 2);
    la::print_vector(&v, true);
    la::print_vector(&v, false);
}

#[test]
fn test_is_matrix_and_vector_equal() {
    let m = la::zero_matrix(2, 2);
    let n = la::zero_matrix(2, 2);
    let o = la::zero_matrix(3, 3);
    assert_eq!(la::is_matrix_equal(&m, &o), false);
    assert_eq!(la::is_matrix_equal(&m, &n), true);
    let mut p = la::zero_matrix(2, 2);
    la::set_matrix_element(&mut p, 0, 0, 1.0);
    assert_eq!(la::is_matrix_equal(&m, &p), false);

    let v = la::zero_vector(3);
    let w = la::zero_vector(3);
    assert_eq!(la::is_vector_equal(&v, &w), true);
}

#[test]
fn test_has_same_dimensions() {
    let m = la::null_matrix(2, 3);
    let n = la::null_matrix(2, 3);
    let o = la::null_matrix(3, 2);
    assert_eq!(la::has_same_dimensions(&m, &n), true);
    assert_eq!(la::has_same_dimensions(&m, &o), false);
}

#[test]
fn test_property_predicates() {
    let m = la::zero_matrix(1, 3);
    assert_eq!(la::is_zero_matrix(&m), true);

    let id = la::identity_matrix(2);
    assert_eq!(la::is_identity_matrix(&id), true);
    assert_eq!(la::is_square_matrix(&id), true);
    assert_eq!(la::is_invertible(&id), true);
    assert_eq!(la::is_diagonal_matrix(&id), true);

    let n = la::zero_matrix(1, 3);
    assert_eq!(la::is_identity_matrix(&n), false);
    assert_eq!(la::is_square_matrix(&n), false);

    let lo = la::new_matrix(&[1.0,0.0,0.0,4.0,5.0,0.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(la::is_lo_tri_matrix(&lo), true);
    assert_eq!(la::is_up_tri_matrix(&lo), false);
    assert_eq!(la::is_triangular_matrix(&lo), true);

    let up = la::new_matrix(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert_eq!(la::is_up_tri_matrix(&up), true);
    assert_eq!(la::is_lo_tri_matrix(&up), false);

    // Symmetric tests
    let sym_zero = la::zero_matrix(2, 2);
    assert_eq!(la::is_matrix_symmetric(&sym_zero), true);
    let asym = la::new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0], 2, 3);
    assert_eq!(la::is_matrix_symmetric(&asym), false);

    // Has zero row
    let zr = la::new_matrix(&[0.0,0.0,0.0,1.0,2.0,3.0], 2, 3);
    assert_eq!(la::has_zero_row(&zr), true);
    let zc = la::new_matrix(&[0.0,1.0,2.0,0.0,4.0,5.0,0.0,7.0,8.0], 3, 3);
    assert_eq!(la::has_zero_col(&zc), true);
}

#[test]
fn test_transpose() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let t = la::transpose_matrix(&m);
    assert_eq!(t.rows, 3);
    assert_eq!(t.cols, 2);
    assert_eq!(la::get_matrix_element(&t, 0, 0), 1.0);
    assert_eq!(la::get_matrix_element(&t, 1, 0), 2.0);
    assert_eq!(la::get_matrix_element(&t, 2, 0), 3.0);
    assert_eq!(la::get_matrix_element(&t, 0, 1), 4.0);
    assert_eq!(la::get_matrix_element(&t, 1, 1), 5.0);
    assert_eq!(la::get_matrix_element(&t, 2, 1), 6.0);
}

#[test]
fn test_trace() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(la::trace_matrix(&m), 15.0);
}

#[test]
fn test_add_matrices() {
    let m1 = la::new_matrix(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    let m2 = la::new_matrix(&[6.0, 4.0, 7.0, 2.0], 2, 2);
    let sum = la::add_matrices(&m1, &m2);
    assert_eq!(sum.data, vec![10.0, 10.0, 10.0, 10.0]);
    assert_eq!(sum.rows, 2);
    assert_eq!(sum.cols, 2);
}

#[test]
fn test_add_vectors() {
    let v = la::new_vector(&[1.0, 2.0, 3.0], 3);
    let w = la::new_vector(&[4.0, 5.0, 6.0], 3);
    let s = la::add_vectors(&v, &w);
    assert_eq!(s.cols, 3);
    assert_eq!(s.data, vec![5.0, 7.0, 9.0]);
}

#[test]
fn test_pow_matrix_and_vector() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let p = la::pow_matrix(&m, 2.0);
    assert_eq!(p.data, vec![1.0, 4.0, 9.0, 16.0]);

    let v = la::new_vector(&[1.0, 2.0, 3.0, 4.0], 4);
    let pv = la::pow_vector(&v, 3.0);
    assert_eq!(pv.data, vec![1.0, 8.0, 27.0, 64.0]);
}

#[test]
fn test_multiply_matrices() {
    let m1 = la::new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let m2 = la::new_matrix(&[1.0, 1.0, 1.0, 1.0], 2, 2);
    let prod = la::multiply_matrices(&m1, &m2);
    assert_eq!(prod.data, vec![3.0, 3.0, 7.0, 7.0]);
}

#[test]
fn test_scale_matrix() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let s = la::scale_matrix(&m, 10.0);
    assert_eq!(s.data, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
}

#[test]
fn test_dot_product() {
    let v = la::new_vector(&[2.0, 4.0, 3.0], 3);
    let w = la::new_vector(&[5.0, 10.0, 15.0], 3);
    assert_eq!(la::dot_product(&v, &w), 95.0);
}

#[test]
fn test_cross_product() {
    let v = la::new_vector(&[2.0, 4.0, 3.0], 3);
    let w = la::new_vector(&[5.0, 10.0, 15.0], 3);
    let c = la::cross_product(&v, &w);
    assert_eq!(c.data, vec![30.0, 15.0, 0.0]);
}

#[test]
fn test_vector_magnitude_and_distance() {
    let v = la::new_vector(&[2.0, 4.0, 3.0], 3);
    let mag = la::vector_magnitude(&v);
    assert!((mag - 5.385164807134504).abs() < 1e-12);

    let w = la::new_vector(&[5.0, 10.0, 15.0], 3);
    let d = la::vector_distance(&v, &w);
    assert!((d - 13.747727084867520).abs() < 1e-12);
}

#[test]
fn test_scale_vector() {
    let v = la::new_vector(&[1.0, 2.0, 3.0], 3);
    let s = la::scale_vector(&v, 5.0);
    assert_eq!(s.data, vec![5.0, 10.0, 15.0]);
}

#[test]
fn test_unit_orthogonal() {
    let u1 = la::new_vector(&[1.0, 0.0, 0.0], 3);
    let u2 = la::new_vector(&[0.0, 1.0, 0.0], 3);
    assert_eq!(la::is_unit_vector(&u1), true);
    assert_eq!(la::is_vector_orthogonal(&u1, &u2), true);

    let v = la::new_vector(&[2.0, 4.0, 3.0], 3);
    assert_eq!(la::is_unit_vector(&v), false);
}

#[test]
fn test_is_matrix_orthogonal() {
    // The C is_matrix_orthogonal is only declared, never implemented.
    // The Rust impl computes inv(m) == transpose(m). Because the underlying
    // multiplyMatrices uses the buggy C signature (only square equal-dim),
    // and adjugateMatrix uses signMatrix (which is also buggy), the
    // inverse_matrix used here does not match what one might expect.
    // For identity 3x3, inverse() returns a buggy result that is not equal
    // to the transpose (identity), so the function returns false.
    let id = la::identity_matrix(3);
    assert_eq!(la::is_matrix_orthogonal(&id, &id), false);

    // Non-invertible is not orthogonal
    let s = la::new_matrix(&[1.0, 2.0, 2.0, 4.0], 2, 2);
    assert_eq!(la::is_matrix_orthogonal(&s, &s), false);
}

#[test]
fn test_scalar_triple_product() {
    let v1 = la::new_vector(&[1.0, 2.0, 3.0], 3);
    let v2 = la::new_vector(&[4.0, 5.0, 6.0], 3);
    let v3 = la::new_vector(&[7.0, 8.0, 10.0], 3);
    assert_eq!(la::scalar_triple_product(&v1, &v2, &v3), -11.0);
}

#[test]
fn test_geometric_ops() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let r0 = la::reflect_axis_2d(&m, 0);
    assert_eq!(r0.data, vec![-1.0, 2.0, -3.0, 4.0]);
    let r1 = la::reflect_axis_2d(&m, 1);
    assert_eq!(r1.data, vec![1.0, -2.0, 3.0, -4.0]);

    let p0 = la::orth_proj_2d(&m, 0);
    assert_eq!(p0.data, vec![1.0, 0.0, 3.0, 0.0]);
    let p1 = la::orth_proj_2d(&m, 1);
    assert_eq!(p1.data, vec![0.0, 2.0, 0.0, 4.0]);

    let s0 = la::shear_2d(&m, 5.0, 0);
    assert_eq!(s0.data, vec![11.0, 2.0, 23.0, 4.0]);
    let s1 = la::shear_2d(&m, 5.0, 1);
    assert_eq!(s1.data, vec![1.0, 7.0, 3.0, 19.0]);

    // 3D ops
    let m3 = la::new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    let s = la::scale_n_space(&m3, 2.0);
    assert_eq!(s.data, vec![2.0,4.0,6.0,8.0,10.0,12.0,14.0,16.0,18.0]);

    let p3 = la::orth_proj_3d(&m3, 0);
    assert_eq!(p3.data, vec![1.0,2.0,0.0,4.0,5.0,0.0,7.0,8.0,0.0]);

    let id3 = la::identity_matrix(3);
    let r3 = la::reflect_axis_3d(&id3, 0);
    assert_eq!(r3.data, vec![1.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,-1.0]);
}

#[test]
fn test_rotate_2d() {
    // Rotate identity by pi/2: [[0,-1],[1,0]]
    let m = la::identity_matrix(2);
    let theta = std::f64::consts::FRAC_PI_2;
    let r = la::rotate_2d(&m, theta);
    // Row-major from public Rust impl: m * [[cos, -sin],[sin, cos]]
    // For identity, result = [[cos, -sin],[sin, cos]]
    assert!((r.data[0] - theta.cos()).abs() < 1e-12);
    assert!((r.data[1] - (-theta.sin())).abs() < 1e-12);
    assert!((r.data[2] - theta.sin()).abs() < 1e-12);
    assert!((r.data[3] - theta.cos()).abs() < 1e-12);
}

#[test]
fn test_determinant() {
    let m1 = la::new_matrix(&[10.0], 1, 1);
    assert_eq!(la::determinant(&m1), 10.0);
    let m2 = la::new_matrix(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    assert_eq!(la::determinant(&m2), 14.0);
    let m3 = la::new_matrix(&[6.0, 1.0, 1.0, 4.0, -2.0, 5.0, 2.0, 8.0, 7.0], 3, 3);
    assert_eq!(la::determinant(&m3), -306.0);
}

#[test]
fn test_lu_decomposition() {
    // C swaps=0 (due to pointer bug)
    let m = la::new_matrix(&[11.0,9.0,24.0,2.0,1.0,5.0,2.0,6.0,3.0,17.0,18.0,1.0,2.0,5.0,7.0,1.0], 4, 4);
    let (l, u, p, swaps) = la::lu_decomposition(&m);
    assert_eq!(swaps, 0);
    assert_eq!(l.rows, 4);
    assert_eq!(u.rows, 4);
    assert_eq!(p.rows, 4);
    for i in 0..4 {
        assert_eq!(l.data[i * 4 + i], 1.0);
    }
    let exp_p = vec![1.0,0.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,0.0,1.0];
    assert_eq!(p.data, exp_p);
}

#[test]
fn test_sub_matrix() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let s = la::sub_matrix(&m, 2, 2);
    assert_eq!(s.rows, 2);
    assert_eq!(s.cols, 2);
    assert_eq!(s.data, vec![1.0, 2.0, 4.0, 5.0]);
}

#[test]
fn test_element_minor_and_cofactor() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(la::element_minor(&m, 0, 0), -3.0);
    assert_eq!(la::element_cofactor(&m, 0, 0), -3.0);
    assert_eq!(la::element_cofactor(&m, 0, 1), 6.0);
}

#[test]
fn test_matrix_minor_and_cofactor() {
    let m = la::new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let mm = la::matrix_minor(&m);
    assert_eq!(mm.data, vec![-3.0, -6.0, -3.0, -6.0, -12.0, -6.0, -3.0, -6.0, -3.0]);
    let cof = la::matrix_cofactor(&m);
    assert_eq!(cof.data, vec![-3.0, 6.0, -3.0, 6.0, -12.0, 6.0, -3.0, 6.0, -3.0]);
}

#[test]
fn test_sign_matrix() {
    let sm = la::sign_matrix(3, 3);
    assert_eq!(sm.data, vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]);
    let sm2 = la::sign_matrix(2, 2);
    assert_eq!(sm2.data, vec![1.0, -1.0, 1.0, -1.0]);
}

#[test]
fn test_adjugate_matrix() {
    let m = la::new_matrix(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let adj = la::adjugate_matrix(&m);
    assert_eq!(adj.rows, 3);
    assert_eq!(adj.cols, 3);
    assert_eq!(adj.data, vec![2.0, -2.0, 2.0, -2.0, 2.0, -2.0, 10.0, -10.0, 10.0]);
}

#[test]
fn test_inverse_matrix() {
    let m = la::new_matrix(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let inv = la::inverse_matrix(&m);
    let exp = vec![0.2, -0.2, 0.2, -0.2, 0.2, -0.2, 1.0, -1.0, 1.0];
    for i in 0..9 {
        assert!((inv.data[i] - exp[i]).abs() < 1e-12, "i={} got {} expected {}", i, inv.data[i], exp[i]);
    }
}

#[test]
fn test_pivot_matrix() {
    let m = la::new_matrix(&[11.0,9.0,24.0,2.0,1.0,5.0,2.0,6.0,3.0,17.0,18.0,1.0,2.0,5.0,7.0,1.0], 4, 4);
    let (p, swaps) = la::pivot_matrix(&m);
    assert_eq!(swaps, 0);
    let exp = vec![1.0,0.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,0.0,1.0];
    assert_eq!(p.data, exp);
}

fn main() {}
