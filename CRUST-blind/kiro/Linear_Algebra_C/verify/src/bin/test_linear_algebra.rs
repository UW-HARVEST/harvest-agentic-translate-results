use Linear_Algebra_C::linear_algebra::*;

// --- Creation and basic operations ---

#[test]
fn test_new_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 2);
    assert_eq!(get_matrix_element(&m, 0, 0), 1.0);
    assert_eq!(get_matrix_element(&m, 0, 1), 2.0);
    assert_eq!(get_matrix_element(&m, 1, 0), 3.0);
    assert_eq!(get_matrix_element(&m, 1, 1), 4.0);
    assert_eq!(matrix_size(&m), 4);
    assert_eq!(matrix_size_bytes(&m), 32);
}

#[test]
fn test_new_vector() {
    let v = new_vector(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 8);
    assert_eq!(vector_size(&v), 8);
    assert_eq!(vector_size_bytes(&v), 64);
}

#[test]
fn test_zero_matrix_and_vector() {
    let m = zero_matrix(2, 3);
    assert!(is_zero_matrix(&m));
    let v = zero_vector(5);
    assert_eq!(v.cols, 5);
    assert_eq!(v.data, vec![0.0; 5]);
}

#[test]
fn test_identity_matrix() {
    let m = identity_matrix(3);
    assert!(is_identity_matrix(&m));
    assert!(is_square_matrix(&m));
}

#[test]
fn test_fill_matrix() {
    let mut m = zero_matrix(2, 2);
    fill_matrix(&mut m, 9.0);
    assert_eq!(m.data, vec![9.0, 9.0, 9.0, 9.0]);
}

#[test]
fn test_fill_vector() {
    let mut v = zero_vector(3);
    fill_vector(&mut v, 5.0);
    assert_eq!(v.data, vec![5.0, 5.0, 5.0]);
}

#[test]
fn test_copy_matrix() {
    let mut m = zero_matrix(2, 2);
    set_matrix_element(&mut m, 0, 1, 5.0);
    let n = copy_matrix(&m);
    assert!(is_matrix_equal(&m, &n));
}

#[test]
fn test_copy_vector() {
    let mut v = zero_vector(4);
    set_vector_element(&mut v, 2, 2.0);
    let v2 = copy_vector(&v);
    assert!(is_vector_equal(&v, &v2));
    assert_eq!(get_vector_element(&v2, 2), 2.0);
}

#[test]
fn test_flatten_matrix() {
    let m = new_matrix(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let flat = flatten_matrix(&m);
    assert_eq!(get_vector_element(&flat, 0), 1.0);
    assert_eq!(get_vector_element(&flat, 5), 6.0);
}

// --- Element access ---

#[test]
fn test_set_get_matrix_element() {
    let mut m = zero_matrix(2, 2);
    set_matrix_element(&mut m, 0, 0, 4.0);
    assert_eq!(get_matrix_element(&m, 0, 0), 4.0);
    assert_eq!(get_matrix_element(&m, 0, 1), 0.0);
    assert_eq!(get_matrix_element(&m, 1, 0), 0.0);
    assert_eq!(get_matrix_element(&m, 1, 1), 0.0);
}

#[test]
fn test_set_get_vector_element() {
    let mut v = zero_vector(2);
    set_vector_element(&mut v, 1, 10.0);
    assert_eq!(get_vector_element(&v, 1), 10.0);
}

// --- Row/Col operations ---

#[test]
fn test_row_vector() {
    let mut m = zero_matrix(3, 2);
    let v = new_vector(&[1.0, 4.0], 2);
    set_row_vector(&mut m, 1, &v);
    assert_eq!(get_matrix_element(&m, 0, 0), 0.0);
    assert_eq!(get_matrix_element(&m, 0, 1), 0.0);
    assert_eq!(get_matrix_element(&m, 1, 0), 1.0);
    assert_eq!(get_matrix_element(&m, 1, 1), 4.0);
    let row = get_row_vector(&m, 1);
    assert_eq!(row.data, vec![1.0, 4.0]);
}

#[test]
fn test_col_vector() {
    let mut m = zero_matrix(3, 2);
    let v = new_vector(&[10.0, 3.0, 7.0], 3);
    set_col_vector(&mut m, 0, &v);
    let col = get_col_vector(&m, 0);
    assert_eq!(col.data, vec![10.0, 3.0, 7.0]);
}

// --- Diagonal operations ---

#[test]
fn test_main_diagonal() {
    let mut m = zero_matrix(2, 2);
    set_matrix_element(&mut m, 0, 0, 34.0);
    set_matrix_element(&mut m, 1, 1, 56.0);
    let d = get_main_diagonal(&m);
    assert_eq!(d.data, vec![34.0, 56.0]);

    let mut n = zero_matrix(2, 2);
    let v = new_vector(&[100.0, 4.0], 2);
    set_main_diagonal(&mut n, &v);
    assert_eq!(get_matrix_element(&n, 0, 0), 100.0);
    assert_eq!(get_matrix_element(&n, 1, 1), 4.0);
}

#[test]
fn test_anti_diagonal() {
    let m = new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    let ad = get_anti_diagonal(&m);
    assert_eq!(ad.data, vec![7.0, 5.0, 3.0]);
}

#[test]
fn test_diagonal_product() {
    let m = new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(diagonal_product(&m), 45.0);
}

// --- Comparison and property tests ---

#[test]
fn test_is_matrix_equal() {
    let m = zero_matrix(2, 2);
    let n = zero_matrix(2, 2);
    let o = zero_matrix(3, 3);
    assert!(is_matrix_equal(&m, &n));
    assert!(!is_matrix_equal(&m, &o));
}

#[test]
fn test_is_vector_equal() {
    let v = zero_vector(3);
    let w = zero_vector(3);
    assert!(is_vector_equal(&v, &w));
}

#[test]
fn test_is_zero_matrix() {
    let m = zero_matrix(1, 3);
    assert!(is_zero_matrix(&m));
    let mut n = zero_matrix(2, 2);
    set_matrix_element(&mut n, 0, 0, 1.0);
    assert!(!is_zero_matrix(&n));
}

#[test]
fn test_is_identity_matrix() {
    let mut m = zero_matrix(2, 2);
    set_matrix_element(&mut m, 0, 0, 1.0);
    set_matrix_element(&mut m, 1, 1, 1.0);
    assert!(is_identity_matrix(&m));

    let n = zero_matrix(1, 3);
    assert!(!is_identity_matrix(&n));

    let mut o = zero_matrix(3, 3);
    set_matrix_element(&mut o, 0, 0, 1.0);
    set_matrix_element(&mut o, 1, 1, 1.0);
    set_matrix_element(&mut o, 2, 2, 1.0);
    set_matrix_element(&mut o, 1, 2, -4.0);
    assert!(!is_identity_matrix(&o));
}

#[test]
fn test_is_diagonal_matrix() {
    let m = new_matrix(&[1.0,0.0,0.0,0.0,2.0,0.0,0.0,0.0,3.0], 3, 3);
    assert!(is_diagonal_matrix(&m));
    let n = new_matrix(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert!(!is_diagonal_matrix(&n));
}

#[test]
fn test_triangular() {
    let up = new_matrix(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert!(is_up_tri_matrix(&up));
    let lo = new_matrix(&[1.0,0.0,0.0,4.0,5.0,0.0,7.0,8.0,9.0], 3, 3);
    assert!(is_lo_tri_matrix(&lo));
    assert!(is_triangular_matrix(&lo));
}

#[test]
fn test_is_symmetric() {
    let m = zero_matrix(2, 2);
    assert!(is_matrix_symmetric(&m));
}

#[test]
fn test_has_zero_row() {
    let m = new_matrix(&[1.0,2.0,0.0,0.0], 2, 2);
    assert!(has_zero_row(&m));
}

#[test]
fn test_has_zero_col() {
    let m = new_matrix(&[1.0,0.0,3.0,0.0], 2, 2);
    assert!(has_zero_col(&m));
}

// --- Advanced operations ---

#[test]
fn test_transpose() {
    let m = new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0], 2, 3);
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
fn test_trace() {
    let m = new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(trace_matrix(&m), 15.0);
}

#[test]
fn test_add_matrices() {
    let m1 = new_matrix(&[4.0,6.0,3.0,8.0], 2, 2);
    let m2 = new_matrix(&[6.0,4.0,7.0,2.0], 2, 2);
    let s = add_matrices(&m1, &m2);
    assert_eq!(s.data, vec![10.0, 10.0, 10.0, 10.0]);
}

#[test]
fn test_add_vectors() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let w = new_vector(&[5.0, 10.0, 15.0], 3);
    let s = add_vectors(&v, &w);
    assert_eq!(s.data, vec![7.0, 14.0, 18.0]);
}

#[test]
fn test_pow_matrix() {
    let m = new_matrix(&[2.0,3.0,4.0,5.0], 2, 2);
    let p = pow_matrix(&m, 2.0);
    assert_eq!(p.data, vec![4.0, 9.0, 16.0, 25.0]);
}

#[test]
fn test_pow_vector() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let p = pow_vector(&v, 2.0);
    assert_eq!(p.data, vec![4.0, 16.0, 9.0]);
}

#[test]
fn test_multiply_matrices() {
    let m1 = new_matrix(&[1.0,2.0,3.0,4.0], 2, 2);
    let m2 = new_matrix(&[1.0,1.0,1.0,1.0], 2, 2);
    let p = multiply_matrices(&m1, &m2);
    assert_eq!(get_matrix_element(&p, 0, 0), 3.0);
    assert_eq!(get_matrix_element(&p, 0, 1), 3.0);
    assert_eq!(get_matrix_element(&p, 1, 0), 7.0);
    assert_eq!(get_matrix_element(&p, 1, 1), 7.0);
}

#[test]
fn test_scale_matrix() {
    let m = new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0], 3, 2);
    let s = scale_matrix(&m, 10.0);
    assert_eq!(s.data, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
}

#[test]
fn test_dot_product() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let w = new_vector(&[5.0, 10.0, 15.0], 3);
    assert_eq!(dot_product(&v, &w), 95.0);
}

#[test]
fn test_cross_product() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let w = new_vector(&[5.0, 10.0, 15.0], 3);
    let c = cross_product(&v, &w);
    assert_eq!(c.data[0], 30.0);
    assert_eq!(c.data[1], 15.0);
    assert_eq!(c.data[2], 0.0);
}

#[test]
fn test_vector_magnitude() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    assert_eq!(Linear_Algebra_C::utils::roundn(vector_magnitude(&v), 3), 5.385);
}

#[test]
fn test_vector_distance() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let w = new_vector(&[5.0, 10.0, 15.0], 3);
    assert_eq!(Linear_Algebra_C::utils::roundn(vector_distance(&v, &w), 3), 13.748);
}

#[test]
fn test_scale_vector() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let s = scale_vector(&v, 3.0);
    assert_eq!(s.data, vec![6.0, 12.0, 9.0]);
}

#[test]
fn test_is_unit_vector() {
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let e1 = new_vector(&[1.0, 0.0, 0.0], 3);
    assert!(!is_unit_vector(&v));
    assert!(is_unit_vector(&e1));
}

#[test]
fn test_is_vector_orthogonal() {
    let e1 = new_vector(&[1.0, 0.0, 0.0], 3);
    let e2 = new_vector(&[0.0, 1.0, 0.0], 3);
    assert!(is_vector_orthogonal(&e1, &e2));
}

#[test]
fn test_scalar_triple_product() {
    let e1 = new_vector(&[1.0, 0.0, 0.0], 3);
    let e2 = new_vector(&[0.0, 1.0, 0.0], 3);
    let e3 = new_vector(&[0.0, 0.0, 1.0], 3);
    assert_eq!(scalar_triple_product(&e1, &e2, &e3), 1.0);
    let v = new_vector(&[2.0, 4.0, 3.0], 3);
    let w = new_vector(&[5.0, 10.0, 15.0], 3);
    assert_eq!(scalar_triple_product(&v, &w, &e1), -90.0);
}

// --- Determinant ---

#[test]
fn test_determinant() {
    assert_eq!(determinant(&new_matrix(&[10.0], 1, 1)), 10.0);
    assert_eq!(determinant(&new_matrix(&[4.0,6.0,3.0,8.0], 2, 2)), 14.0);
    assert_eq!(determinant(&new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3)), -306.0);
    let m4 = new_matrix(&[11.0,9.0,24.0,2.0,1.0,5.0,2.0,6.0,3.0,17.0,18.0,1.0,2.0,5.0,7.0,1.0], 4, 4);
    assert_eq!(Linear_Algebra_C::utils::roundn(determinant(&m4), 1), 284.0);
}

// --- Sub matrix, minors, cofactors ---

#[test]
fn test_sub_matrix() {
    let m = new_matrix(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    let s = sub_matrix(&m, 2, 2);
    assert_eq!(s.data, vec![1.0, 2.0, 4.0, 5.0]);
}

#[test]
fn test_element_minor() {
    let m = new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    assert_eq!(element_minor(&m, 0, 0), -54.0);
    assert_eq!(element_minor(&m, 0, 1), 18.0);
}

#[test]
fn test_matrix_minor() {
    let m = new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let mm = matrix_minor(&m);
    assert_eq!(mm.data, vec![-54.0, 18.0, 36.0, -1.0, 40.0, 46.0, 7.0, 26.0, -16.0]);
}

#[test]
fn test_element_cofactor() {
    let m = new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    assert_eq!(element_cofactor(&m, 0, 0), -54.0);
    assert_eq!(element_cofactor(&m, 0, 1), -18.0);
    assert_eq!(element_cofactor(&m, 1, 0), 1.0);
}

#[test]
fn test_matrix_cofactor() {
    let m = new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let cf = matrix_cofactor(&m);
    assert_eq!(cf.data, vec![-54.0, -18.0, 36.0, 1.0, 40.0, -46.0, 7.0, -26.0, -16.0]);
}

#[test]
fn test_sign_matrix() {
    let s = sign_matrix(3, 3);
    assert_eq!(s.data, vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]);
}

#[test]
fn test_adjugate_matrix() {
    let m = new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let adj = adjugate_matrix(&m);
    assert_eq!(adj.data, vec![-36.0, 36.0, -36.0, 5.0, -5.0, 5.0, -35.0, 35.0, -35.0]);
}

#[test]
fn test_inverse_matrix() {
    let m = new_matrix(&[3.0,0.0,2.0,2.0,0.0,-2.0,0.0,1.0,1.0], 3, 3);
    let inv = inverse_matrix(&m);
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

// --- LU and pivot ---

#[test]
fn test_pivot_matrix() {
    let m = new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let (piv, swaps) = pivot_matrix(&m);
    assert_eq!(swaps, 0);
    assert_eq!(piv.data, vec![1.0,0.0,0.0, 0.0,0.0,1.0, 0.0,1.0,0.0]);
}

#[test]
fn test_lu_decomposition() {
    let m = new_matrix(&[6.0,1.0,1.0,4.0,-2.0,5.0,2.0,8.0,7.0], 3, 3);
    let (l, u, _p, swaps) = lu_decomposition(&m);
    assert_eq!(swaps, 0);
    // L should have 1s on diagonal
    assert_eq!(l.data[0], 1.0);
    assert_eq!(l.data[4], 1.0);
    assert_eq!(l.data[8], 1.0);
    // U first row
    assert_eq!(u.data[0], 6.0);
    assert_eq!(u.data[1], 1.0);
    assert_eq!(u.data[2], 1.0);
}

// --- Geometric operations ---

#[test]
fn test_reflect_axis_2d() {
    let eye = identity_matrix(2);
    let r0 = reflect_axis_2d(&eye, 0);
    assert_eq!(r0.data, vec![-1.0, 0.0, 0.0, 1.0]);
    let r1 = reflect_axis_2d(&eye, 1);
    assert_eq!(r1.data, vec![1.0, 0.0, 0.0, -1.0]);
}

#[test]
fn test_orth_proj_2d() {
    let eye = identity_matrix(2);
    let p0 = orth_proj_2d(&eye, 0);
    assert_eq!(p0.data, vec![1.0, 0.0, 0.0, 0.0]);
    let p1 = orth_proj_2d(&eye, 1);
    assert_eq!(p1.data, vec![0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_orth_proj_3d() {
    let eye = identity_matrix(3);
    let p0 = orth_proj_3d(&eye, 0);
    assert_eq!(p0.data, vec![1.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,0.0]);
    let p1 = orth_proj_3d(&eye, 1);
    assert_eq!(p1.data, vec![1.0,0.0,0.0, 0.0,0.0,0.0, 0.0,0.0,1.0]);
    let p2 = orth_proj_3d(&eye, 2);
    assert_eq!(p2.data, vec![0.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,1.0]);
}

#[test]
fn test_shear_2d() {
    let eye = identity_matrix(2);
    let s0 = shear_2d(&eye, 2.0, 0);
    assert_eq!(s0.data, vec![1.0, 0.0, 2.0, 1.0]);
    let s1 = shear_2d(&eye, 2.0, 1);
    assert_eq!(s1.data, vec![1.0, 2.0, 0.0, 1.0]);
}

#[test]
fn test_scale_n_space() {
    let eye = identity_matrix(2);
    let s = scale_n_space(&eye, 3.0);
    assert_eq!(s.data, vec![3.0, 0.0, 0.0, 3.0]);
}

fn main() {}
