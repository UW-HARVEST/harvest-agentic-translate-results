use Linear_Algebra_C::linear_algebra::*;

#[test]
fn test_assert_matrix_and_vector() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let v = new_vector(&[1.0, 2.0], 2);
    assert_eq!(assert_matrix(&m), true);
    assert_eq!(assert_vector(&v), true);
}

#[test]
fn test_new_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 2);
    assert_eq!(get_matrix_element(&m, 0, 0), 1.0);
    assert_eq!(get_matrix_element(&m, 0, 1), 2.0);
    assert_eq!(get_matrix_element(&m, 1, 0), 3.0);
    assert_eq!(get_matrix_element(&m, 1, 1), 4.0);
}

#[test]
fn test_new_vector() {
    let v = new_vector(&[1.0, 2.0, 3.0], 3);
    assert_eq!(v.cols, 3);
    assert_eq!(v.data, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_null_matrix_and_vector() {
    let m = null_matrix(2, 3);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 3);
    assert_eq!(m.data.len(), 6);
    let v = null_vector(4);
    assert_eq!(v.cols, 4);
    assert_eq!(v.data.len(), 4);
}

#[test]
fn test_zero_matrix_and_vector() {
    let m = zero_matrix(2, 2);
    assert_eq!(m.data, vec![0.0, 0.0, 0.0, 0.0]);
    let v = zero_vector(3);
    assert_eq!(v.data, vec![0.0, 0.0, 0.0]);
}

#[test]
fn test_fill_matrix_and_vector() {
    let mut m = zero_matrix(2, 2);
    fill_matrix(&mut m, 3.0);
    assert_eq!(m.data, vec![3.0, 3.0, 3.0, 3.0]);
    let mut v = zero_vector(3);
    fill_vector(&mut v, 7.0);
    assert_eq!(v.data, vec![7.0, 7.0, 7.0]);
}

#[test]
fn test_identity_matrix() {
    let m = identity_matrix(3);
    assert_eq!(m.rows, 3);
    assert_eq!(m.cols, 3);
    assert_eq!(m.data, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_delete_matrix_and_vector_no_op() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let v = new_vector(&[1.0, 2.0], 2);
    delete_matrix(m);
    delete_vector(v);
}

#[test]
fn test_copy_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let c = copy_matrix(&m);
    assert_eq!(c.rows, 2);
    assert_eq!(c.cols, 3);
    assert_eq!(c.data, m.data);
}

#[test]
fn test_copy_vector() {
    let v = new_vector(&[3.0, 1.0, 4.0], 3);
    let c = copy_vector(&v);
    assert_eq!(c.cols, 3);
    assert_eq!(c.data, v.data);
}

#[test]
fn test_flatten_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let f = flatten_matrix(&m);
    assert_eq!(f.cols, 6);
    assert_eq!(f.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_matrix_size() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(matrix_size(&m), 4);
    assert_eq!(matrix_size_bytes(&m), 32);
}

#[test]
fn test_vector_size() {
    let v = new_vector(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 8);
    assert_eq!(vector_size(&v), 8);
    assert_eq!(vector_size_bytes(&v), 64);
}

#[test]
fn test_set_get_matrix_element() {
    let mut m = zero_matrix(2, 2);
    set_matrix_element(&mut m, 0, 1, 5.0);
    assert_eq!(get_matrix_element(&m, 0, 0), 0.0);
    assert_eq!(get_matrix_element(&m, 0, 1), 5.0);
    assert_eq!(get_matrix_element(&m, 1, 0), 0.0);
    assert_eq!(get_matrix_element(&m, 1, 1), 0.0);
}

#[test]
fn test_set_get_vector_element() {
    let mut v = zero_vector(3);
    set_vector_element(&mut v, 1, 9.5);
    assert_eq!(get_vector_element(&v, 0), 0.0);
    assert_eq!(get_vector_element(&v, 1), 9.5);
    assert_eq!(get_vector_element(&v, 2), 0.0);
}

#[test]
fn test_row_vector() {
    let mut m = zero_matrix(3, 2);
    let v = new_vector(&[1.0, 4.0], 2);
    set_row_vector(&mut m, 1, &v);
    assert_eq!(get_matrix_element(&m, 1, 0), 1.0);
    assert_eq!(get_matrix_element(&m, 1, 1), 4.0);
    let row = get_row_vector(&m, 1);
    assert_eq!(row.cols, 2);
    assert_eq!(row.data, vec![1.0, 4.0]);
}

#[test]
fn test_col_vector() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let col = get_col_vector(&m, 1);
    assert_eq!(col.cols, 2);
    assert_eq!(col.data, vec![2.0, 5.0]);

    let mut z = zero_matrix(3, 2);
    let v = new_vector(&[10.0, 20.0, 30.0], 3);
    set_col_vector(&mut z, 0, &v);
    assert_eq!(get_matrix_element(&z, 0, 0), 10.0);
    assert_eq!(get_matrix_element(&z, 1, 0), 20.0);
    assert_eq!(get_matrix_element(&z, 2, 0), 30.0);
}

#[test]
fn test_main_diagonal() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let d = get_main_diagonal(&m);
    assert_eq!(d.data, vec![1.0, 5.0, 9.0]);

    let mut n = zero_matrix(3, 3);
    let v = new_vector(&[10.0, 20.0, 30.0], 3);
    set_main_diagonal(&mut n, &v);
    assert_eq!(get_matrix_element(&n, 0, 0), 10.0);
    assert_eq!(get_matrix_element(&n, 1, 1), 20.0);
    assert_eq!(get_matrix_element(&n, 2, 2), 30.0);
}

#[test]
fn test_anti_diagonal() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let d = get_anti_diagonal(&m);
    assert_eq!(d.data, vec![7.0, 5.0, 3.0]);

    let mut n = zero_matrix(3, 3);
    let v = new_vector(&[1.0, 2.0, 3.0], 3);
    set_anti_diagonal(&mut n, &v);
    assert_eq!(get_matrix_element(&n, 2, 0), 1.0);
    assert_eq!(get_matrix_element(&n, 1, 1), 2.0);
    assert_eq!(get_matrix_element(&n, 0, 2), 3.0);
}

#[test]
fn test_diagonal_product() {
    let m = new_matrix(&[2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0], 3, 3);
    assert_eq!(diagonal_product(&m), 24.0);
}

#[test]
fn test_print_matrix_no_panic() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    print_matrix(&m, false);
    print_matrix(&m, true);
}

#[test]
fn test_print_vector_no_panic() {
    let v = new_vector(&[1.0, 2.0, 3.0], 3);
    print_vector(&v, false);
    print_vector(&v, true);
}

#[test]
fn test_is_matrix_equal() {
    let a = zero_matrix(2, 2);
    let b = zero_matrix(2, 2);
    let c = zero_matrix(3, 3);
    assert_eq!(is_matrix_equal(&a, &b), true);
    assert_eq!(is_matrix_equal(&a, &c), false);
}

#[test]
fn test_is_vector_equal() {
    let a = zero_vector(3);
    let b = zero_vector(3);
    assert_eq!(is_vector_equal(&a, &b), true);
}

#[test]
fn test_has_same_dimensions() {
    let a = zero_matrix(2, 2);
    let b = zero_matrix(2, 2);
    let c = zero_matrix(2, 3);
    assert_eq!(has_same_dimensions(&a, &b), true);
    assert_eq!(has_same_dimensions(&a, &c), false);
}

#[test]
fn test_is_zero_matrix() {
    let z = zero_matrix(1, 3);
    let mut nz = zero_matrix(2, 2);
    set_matrix_element(&mut nz, 0, 0, 1.0);
    assert_eq!(is_zero_matrix(&z), true);
    assert_eq!(is_zero_matrix(&nz), false);
}

#[test]
fn test_is_identity_matrix() {
    let id = identity_matrix(2);
    let z = zero_matrix(2, 2);
    let nonsq = zero_matrix(1, 3);
    assert_eq!(is_identity_matrix(&id), true);
    assert_eq!(is_identity_matrix(&z), false);
    assert_eq!(is_identity_matrix(&nonsq), false);
}

#[test]
fn test_is_square_matrix() {
    let sq = zero_matrix(2, 2);
    let nsq = zero_matrix(2, 3);
    assert_eq!(is_square_matrix(&sq), true);
    assert_eq!(is_square_matrix(&nsq), false);
}

#[test]
fn test_is_invertible() {
    let m = new_matrix(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let z = zero_matrix(2, 2);
    assert_eq!(is_invertible(&m), true);
    assert_eq!(is_invertible(&z), false);
}

#[test]
fn test_is_diagonal_matrix() {
    let d = new_matrix(&[1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0], 3, 3);
    let nd = new_matrix(&[1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 0.0, 0.0, 9.0], 3, 3);
    assert_eq!(is_diagonal_matrix(&d), true);
    assert_eq!(is_diagonal_matrix(&nd), false);
}

#[test]
fn test_is_triangular_matrices() {
    let ut = new_matrix(&[1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 0.0, 0.0, 9.0], 3, 3);
    let lt = new_matrix(&[1.0, 0.0, 0.0, 4.0, 5.0, 0.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(is_up_tri_matrix(&ut), true);
    assert_eq!(is_lo_tri_matrix(&lt), true);
    assert_eq!(is_triangular_matrix(&lt), true);
}

#[test]
fn test_is_matrix_symmetric() {
    let m = new_matrix(&[1.0, 2.0, 2.0, 4.0], 2, 2);
    let z = zero_matrix(2, 2);
    assert_eq!(is_matrix_symmetric(&m), true);
    assert_eq!(is_matrix_symmetric(&z), true);
}

#[test]
fn test_has_zero_row_and_col() {
    let with_zero = new_matrix(&[0.0, 0.0, 0.0, 1.0, 2.0, 3.0], 2, 3);
    let no_zero = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(has_zero_row(&with_zero), true);
    assert_eq!(has_zero_row(&no_zero), false);
    let with_col_zero = new_matrix(&[1.0, 0.0, 2.0, 0.0], 2, 2);
    assert_eq!(has_zero_col(&with_col_zero), true);
    assert_eq!(has_zero_col(&no_zero), false);
}

#[test]
fn test_transpose_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let t = transpose_matrix(&m);
    assert_eq!(t.rows, 3);
    assert_eq!(t.cols, 2);
    assert_eq!(get_matrix_element(&t, 0, 0), 1.0);
    assert_eq!(get_matrix_element(&t, 1, 0), 2.0);
    assert_eq!(get_matrix_element(&t, 2, 0), 3.0);
    assert_eq!(get_matrix_element(&t, 0, 1), 4.0);
    assert_eq!(get_matrix_element(&t, 1, 1), 5.0);
    assert_eq!(get_matrix_element(&t, 2, 1), 6.0);
}

#[test]
fn test_trace_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(trace_matrix(&m), 15.0);
}

#[test]
fn test_add_matrices() {
    let a = new_matrix(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    let b = new_matrix(&[6.0, 4.0, 7.0, 2.0], 2, 2);
    let s = add_matrices(&a, &b);
    assert_eq!(s.data, vec![10.0, 10.0, 10.0, 10.0]);
}

#[test]
fn test_add_vectors() {
    let a = new_vector(&[1.0, 2.0, 3.0], 3);
    let b = new_vector(&[4.0, 5.0, 6.0], 3);
    let s = add_vectors(&a, &b);
    assert_eq!(s.cols, 3);
    assert_eq!(s.data, vec![5.0, 7.0, 9.0]);
}

#[test]
fn test_pow_matrix() {
    let m = new_matrix(&[2.0, 3.0, 4.0, 5.0], 2, 2);
    let p = pow_matrix(&m, 2.0);
    assert_eq!(p.data, vec![4.0, 9.0, 16.0, 25.0]);
}

#[test]
fn test_pow_vector() {
    let v = new_vector(&[2.0, 3.0, 4.0], 3);
    let p = pow_vector(&v, 2.0);
    assert_eq!(p.data, vec![4.0, 9.0, 16.0]);
}

#[test]
fn test_multiply_matrices() {
    let a = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = new_matrix(&[1.0, 1.0, 1.0, 1.0], 2, 2);
    let p = multiply_matrices(&a, &b);
    assert_eq!(p.data, vec![3.0, 3.0, 7.0, 7.0]);
}

#[test]
fn test_scale_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let s = scale_matrix(&m, 10.0);
    assert_eq!(s.data, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
}

#[test]
fn test_dot_product() {
    let a = new_vector(&[2.0, 4.0, 3.0], 3);
    let b = new_vector(&[5.0, 10.0, 15.0], 3);
    assert_eq!(dot_product(&a, &b), 95.0);
}

#[test]
fn test_cross_product() {
    let a = new_vector(&[2.0, 4.0, 3.0], 3);
    let b = new_vector(&[5.0, 10.0, 15.0], 3);
    let c = cross_product(&a, &b);
    assert_eq!(c.cols, 3);
    assert_eq!(get_vector_element(&c, 0), 30.0);
    assert_eq!(get_vector_element(&c, 1), 15.0);
    assert_eq!(get_vector_element(&c, 2), 0.0);
}

#[test]
fn test_vector_magnitude_and_distance() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let mag = vector_magnitude(&v);
    assert!((mag - 29f64.sqrt()).abs() < 1e-12);
    let w = new_vector(&[5.0, 10.0, 15.0], 3);
    let d = vector_distance(&v, &w);
    assert!((d - 189f64.sqrt()).abs() < 1e-12);
}

#[test]
fn test_scale_vector() {
    let v = new_vector(&[1.0, 2.0, 3.0], 3);
    let s = scale_vector(&v, 2.5);
    assert_eq!(s.data, vec![2.5, 5.0, 7.5]);
}

#[test]
fn test_is_unit_vector() {
    let u = new_vector(&[1.0, 0.0, 0.0], 3);
    let nu = new_vector(&[2.0, 4.0, 3.0], 3);
    assert_eq!(is_unit_vector(&u), true);
    assert_eq!(is_unit_vector(&nu), false);
}

#[test]
fn test_is_vector_orthogonal() {
    let a = new_vector(&[1.0, 0.0, 0.0], 3);
    let b = new_vector(&[0.0, 1.0, 0.0], 3);
    let c = new_vector(&[1.0, 2.0, 3.0], 3);
    assert_eq!(is_vector_orthogonal(&a, &b), true);
    assert_eq!(is_vector_orthogonal(&a, &c), false);
}

#[test]
fn test_is_matrix_orthogonal() {
    // For our impl, is_matrix_orthogonal(m1, m2) == is_matrix_equal(inverse_matrix(m1), m2).
    // So computing inverse of m and passing it as m2 must return true.
    let m = new_matrix(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let inv = inverse_matrix(&m);
    assert_eq!(is_matrix_orthogonal(&m, &inv), true);
    // Conversely, the zero matrix is not invertible, so result must be false
    let zero = zero_matrix(2, 2);
    let any = identity_matrix(2);
    assert_eq!(is_matrix_orthogonal(&zero, &any), false);
}

#[test]
fn test_scalar_triple_product() {
    let a = new_vector(&[1.0, 2.0, 3.0], 3);
    let b = new_vector(&[4.0, 5.0, 6.0], 3);
    let c = new_vector(&[7.0, 8.0, 9.0], 3);
    assert_eq!(scalar_triple_product(&a, &b, &c), -24.0);
}

#[test]
fn test_reflect_axis_2d() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let r0 = reflect_axis_2d(&m, 0);
    assert_eq!(r0.data, vec![-1.0, 2.0, -3.0, 4.0]);
    let r1 = reflect_axis_2d(&m, 1);
    assert_eq!(r1.data, vec![1.0, -2.0, 3.0, -4.0]);
}

#[test]
fn test_reflect_axis_3d() {
    let m = identity_matrix(3);
    let r = reflect_axis_3d(&m, 0);
    assert_eq!(r.data, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0]);
}

#[test]
fn test_orth_proj_2d() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let p = orth_proj_2d(&m, 0);
    assert_eq!(p.data, vec![1.0, 0.0, 3.0, 0.0]);
    let p1 = orth_proj_2d(&m, 1);
    assert_eq!(p1.data, vec![0.0, 2.0, 0.0, 4.0]);
}

#[test]
fn test_orth_proj_3d() {
    let m = new_matrix(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], 3, 3);
    let p0 = orth_proj_3d(&m, 0);
    assert_eq!(p0.data, vec![1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0]);
}

#[test]
fn test_rotate_2d() {
    let m = identity_matrix(2);
    let r = rotate_2d(&m, std::f64::consts::PI / 2.0);
    // For an identity input, rotate result should be the rotation matrix itself
    assert!((r.data[0] - 0.0).abs() < 1e-10);
    assert!((r.data[1] - (-1.0)).abs() < 1e-10);
    assert!((r.data[2] - 1.0).abs() < 1e-10);
    assert!((r.data[3] - 0.0).abs() < 1e-10);
}

#[test]
fn test_scale_n_space() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let s = scale_n_space(&m, 3.0);
    assert_eq!(s.data, vec![3.0, 6.0, 9.0, 12.0]);
}

#[test]
fn test_shear_2d() {
    let m = identity_matrix(2);
    let s0 = shear_2d(&m, 3.0, 0);
    assert_eq!(s0.data, vec![1.0, 0.0, 3.0, 1.0]);
    let s1 = shear_2d(&m, 3.0, 1);
    assert_eq!(s1.data, vec![1.0, 3.0, 0.0, 1.0]);
}

#[test]
fn test_determinant_1x1() {
    let m = new_matrix(&[10.0], 1, 1);
    assert_eq!(determinant(&m), 10.0);
}

#[test]
fn test_determinant_2x2() {
    let m = new_matrix(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    assert_eq!(determinant(&m), 14.0);
}

#[test]
fn test_determinant_3x3() {
    let m = new_matrix(&[6.0, 1.0, 1.0, 4.0, -2.0, 5.0, 2.0, 8.0, 7.0], 3, 3);
    assert_eq!(determinant(&m), -306.0);
}

#[test]
fn test_determinant_4x4_lu() {
    let m = new_matrix(
        &[
            11.0, 9.0, 24.0, 2.0, 1.0, 5.0, 2.0, 6.0, 3.0, 17.0, 18.0, 1.0, 2.0, 5.0, 7.0, 1.0,
        ],
        4,
        4,
    );
    let det = determinant(&m);
    let rounded = (det * 10.0).round() / 10.0;
    assert_eq!(rounded, 284.0);
}

#[test]
fn test_determinant_4x4_triangular() {
    // 4x4 lower triangular - takes the triangular path
    let m = new_matrix(
        &[
            1.0, 0.0, 0.0, 0.0, 2.0, 3.0, 0.0, 0.0, 4.0, 5.0, 6.0, 0.0, 7.0, 8.0, 9.0, 10.0,
        ],
        4,
        4,
    );
    assert_eq!(determinant(&m), 180.0);
}

#[test]
fn test_lu_decomposition() {
    let m = new_matrix(&[1.0, 3.0, 5.0, 2.0, 4.0, 7.0, 1.0, 1.0, 0.0], 3, 3);
    let (l, u, p, swaps) = lu_decomposition(&m);
    assert_eq!(swaps, 0);
    assert_eq!(l.rows, 3);
    assert_eq!(l.cols, 3);
    assert_eq!(u.rows, 3);
    assert_eq!(u.cols, 3);
    assert_eq!(p.rows, 3);
    assert_eq!(p.cols, 3);
    let expected_p = vec![0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(p.data, expected_p);
}

#[test]
fn test_sub_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let s = sub_matrix(&m, 2, 2);
    assert_eq!(s.rows, 2);
    assert_eq!(s.cols, 2);
    assert_eq!(s.data, vec![1.0, 2.0, 4.0, 5.0]);
}

#[test]
fn test_element_minor_and_cofactor() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(element_minor(&m, 0, 0), -3.0);
    assert_eq!(element_cofactor(&m, 0, 0), -3.0);
    assert_eq!(element_minor(&m, 0, 1), -6.0);
    assert_eq!(element_cofactor(&m, 0, 1), 6.0);
}

#[test]
fn test_matrix_minor() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let mm = matrix_minor(&m);
    assert_eq!(
        mm.data,
        vec![-3.0, -6.0, -3.0, -6.0, -12.0, -6.0, -3.0, -6.0, -3.0]
    );
}

#[test]
fn test_matrix_cofactor() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let cf = matrix_cofactor(&m);
    assert_eq!(
        cf.data,
        vec![-3.0, 6.0, -3.0, 6.0, -12.0, 6.0, -3.0, 6.0, -3.0]
    );
}

#[test]
fn test_sign_matrix() {
    let s2 = sign_matrix(2, 2);
    assert_eq!(s2.data, vec![1.0, -1.0, 1.0, -1.0]);
    let s3 = sign_matrix(3, 3);
    assert_eq!(s3.data, vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]);
}

#[test]
fn test_adjugate_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let adj = adjugate_matrix(&m);
    assert_eq!(adj.rows, 2);
    assert_eq!(adj.cols, 2);
    assert_eq!(adj.data, vec![7.0, -7.0, 3.0, -3.0]);
}

#[test]
fn test_inverse_matrix() {
    let m = new_matrix(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let inv = inverse_matrix(&m);
    assert_eq!(inv.rows, 3);
    assert_eq!(inv.cols, 3);
    let expected = [0.2, -0.2, 0.2, -0.2, 0.2, -0.2, 1.0, -1.0, 1.0];
    for i in 0..9 {
        assert!((inv.data[i] - expected[i]).abs() < 1e-9);
    }
}

#[test]
fn test_pivot_matrix_no_swap() {
    let m = new_matrix(&[1.0, 3.0, 5.0, 2.0, 4.0, 7.0, 1.0, 1.0, 0.0], 3, 3);
    let (p, _swaps) = pivot_matrix(&m);
    assert_eq!(p.rows, 3);
    assert_eq!(p.cols, 3);
    assert_eq!(p.data, vec![0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_pivot_matrix_159() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let (p, _swaps) = pivot_matrix(&m);
    assert_eq!(p.data, vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
}

fn main() {}
