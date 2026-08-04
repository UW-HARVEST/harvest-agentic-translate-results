use Linear_Algebra_C::linear_algebra::{Matrix, Vector};
use Linear_Algebra_C::matrix::*;
use Linear_Algebra_C::vector::new_vector_impl;

fn nm(d: &[f64], rows: usize, cols: usize) -> Matrix {
    new_matrix_impl(d, rows, cols)
}

fn nv(d: &[f64]) -> Vector {
    new_vector_impl(d, d.len())
}

#[test]
fn test_assert_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(assert_matrix_impl(&m), true);
}

#[test]
fn test_new_matrix_impl_data_layout() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 2);
    assert_eq!(m.data, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_null_matrix_impl() {
    let m = null_matrix_impl(3, 2);
    assert_eq!(m.rows, 3);
    assert_eq!(m.cols, 2);
    assert_eq!(m.data.len(), 6);
}

#[test]
fn test_zero_matrix_impl() {
    let m = zero_matrix_impl(2, 3);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 3);
    assert_eq!(m.data, vec![0.0; 6]);
}

#[test]
fn test_fill_matrix_impl() {
    let mut m = zero_matrix_impl(2, 2);
    fill_matrix_impl(&mut m, 5.0);
    assert_eq!(m.data, vec![5.0, 5.0, 5.0, 5.0]);
}

#[test]
fn test_identity_matrix_impl() {
    let m = identity_matrix_impl(3);
    assert_eq!(m.rows, 3);
    assert_eq!(m.cols, 3);
    assert_eq!(m.data, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_copy_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let c = copy_matrix_impl(&m);
    assert_eq!(c.rows, 2);
    assert_eq!(c.cols, 3);
    assert_eq!(c.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_flatten_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let f = flatten_matrix_impl(&m);
    assert_eq!(f.cols, 6);
    assert_eq!(f.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_matrix_size_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    assert_eq!(matrix_size_impl(&m), 6);
}

#[test]
fn test_matrix_size_bytes_impl() {
    // 2*2 doubles = 32 bytes
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(matrix_size_bytes_impl(&m), 32);
}

#[test]
fn test_set_get_matrix_element_impl() {
    let mut m = zero_matrix_impl(2, 2);
    set_matrix_element_impl(&mut m, 0, 1, 5.0);
    assert_eq!(get_matrix_element_impl(&m, 0, 0), 0.0);
    assert_eq!(get_matrix_element_impl(&m, 0, 1), 5.0);
    assert_eq!(get_matrix_element_impl(&m, 1, 0), 0.0);
    assert_eq!(get_matrix_element_impl(&m, 1, 1), 0.0);
}

#[test]
fn test_set_row_vector_impl() {
    let mut m = zero_matrix_impl(3, 2);
    let v = nv(&[1.0, 4.0]);
    set_row_vector_impl(&mut m, 1, &v);
    assert_eq!(get_matrix_element_impl(&m, 0, 0), 0.0);
    assert_eq!(get_matrix_element_impl(&m, 0, 1), 0.0);
    assert_eq!(get_matrix_element_impl(&m, 1, 0), 1.0);
    assert_eq!(get_matrix_element_impl(&m, 1, 1), 4.0);
    assert_eq!(get_matrix_element_impl(&m, 2, 0), 0.0);
    assert_eq!(get_matrix_element_impl(&m, 2, 1), 0.0);
}

#[test]
fn test_get_row_vector_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let row = get_row_vector_impl(&m, 1);
    assert_eq!(row.cols, 3);
    assert_eq!(row.data, vec![4.0, 5.0, 6.0]);
}

#[test]
fn test_set_col_vector_impl() {
    let mut m = zero_matrix_impl(3, 2);
    let v = nv(&[10.0, 20.0, 30.0]);
    set_col_vector_impl(&mut m, 0, &v);
    assert_eq!(get_matrix_element_impl(&m, 0, 0), 10.0);
    assert_eq!(get_matrix_element_impl(&m, 1, 0), 20.0);
    assert_eq!(get_matrix_element_impl(&m, 2, 0), 30.0);
    assert_eq!(get_matrix_element_impl(&m, 0, 1), 0.0);
}

#[test]
fn test_get_col_vector_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let col = get_col_vector_impl(&m, 1);
    assert_eq!(col.cols, 2);
    assert_eq!(col.data, vec![2.0, 5.0]);
}

#[test]
fn test_get_main_diagonal_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let d = get_main_diagonal_impl(&m);
    assert_eq!(d.cols, 3);
    assert_eq!(d.data, vec![1.0, 5.0, 9.0]);
}

#[test]
fn test_set_main_diagonal_impl() {
    let mut m = zero_matrix_impl(3, 3);
    let v = nv(&[7.0, 8.0, 9.0]);
    set_main_diagonal_impl(&mut m, &v);
    assert_eq!(get_matrix_element_impl(&m, 0, 0), 7.0);
    assert_eq!(get_matrix_element_impl(&m, 1, 1), 8.0);
    assert_eq!(get_matrix_element_impl(&m, 2, 2), 9.0);
    assert_eq!(get_matrix_element_impl(&m, 0, 1), 0.0);
}

#[test]
fn test_get_anti_diagonal_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let d = get_anti_diagonal_impl(&m);
    assert_eq!(d.cols, 3);
    // bottom-left first: (2,0)=7, (1,1)=5, (0,2)=3
    assert_eq!(d.data, vec![7.0, 5.0, 3.0]);
}

#[test]
fn test_set_anti_diagonal_impl() {
    let mut m = zero_matrix_impl(3, 3);
    let v = nv(&[1.0, 2.0, 3.0]);
    set_anti_diagonal_impl(&mut m, &v);
    // (2,0)=1, (1,1)=2, (0,2)=3
    assert_eq!(get_matrix_element_impl(&m, 2, 0), 1.0);
    assert_eq!(get_matrix_element_impl(&m, 1, 1), 2.0);
    assert_eq!(get_matrix_element_impl(&m, 0, 2), 3.0);
    assert_eq!(get_matrix_element_impl(&m, 0, 0), 0.0);
}

#[test]
fn test_diagonal_product_impl() {
    let m = nm(&[2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0], 3, 3);
    assert_eq!(diagonal_product_impl(&m), 24.0);
}

#[test]
fn test_is_matrix_equal_impl() {
    let a = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let c = nm(&[1.0, 2.0, 3.0, 5.0], 2, 2);
    let d = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    assert_eq!(is_matrix_equal_impl(&a, &b), true);
    assert_eq!(is_matrix_equal_impl(&a, &c), false);
    assert_eq!(is_matrix_equal_impl(&a, &d), false);
}

#[test]
fn test_has_same_dimensions_impl() {
    let a = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = nm(&[5.0, 6.0, 7.0, 8.0], 2, 2);
    let c = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    assert_eq!(has_same_dimensions_impl(&a, &b), true);
    assert_eq!(has_same_dimensions_impl(&a, &c), false);
}

#[test]
fn test_is_zero_matrix_impl() {
    let z = zero_matrix_impl(2, 2);
    let nz = nm(&[1.0, 0.0, 0.0, 0.0], 2, 2);
    assert_eq!(is_zero_matrix_impl(&z), true);
    assert_eq!(is_zero_matrix_impl(&nz), false);
}

#[test]
fn test_is_identity_matrix_impl() {
    let id = identity_matrix_impl(3);
    let z = zero_matrix_impl(3, 3);
    let bad = nm(&[1.0, 0.0, 0.0, 0.0, 1.0, -4.0, 0.0, 0.0, 1.0], 3, 3);
    let nonsq = nm(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0], 2, 3);
    assert_eq!(is_identity_matrix_impl(&id), true);
    assert_eq!(is_identity_matrix_impl(&z), false);
    assert_eq!(is_identity_matrix_impl(&bad), false);
    assert_eq!(is_identity_matrix_impl(&nonsq), false);
}

#[test]
fn test_is_square_matrix_impl() {
    let sq = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let nsq = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    assert_eq!(is_square_matrix_impl(&sq), true);
    assert_eq!(is_square_matrix_impl(&nsq), false);
}

#[test]
fn test_is_invertible_impl() {
    let inv = nm(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let zero = zero_matrix_impl(2, 2);
    assert_eq!(is_invertible_impl(&inv), true);
    assert_eq!(is_invertible_impl(&zero), false);
}

#[test]
fn test_is_diagonal_matrix_impl() {
    let d = nm(&[1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0], 3, 3);
    let nd = nm(&[1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 0.0, 0.0, 9.0], 3, 3);
    assert_eq!(is_diagonal_matrix_impl(&d), true);
    assert_eq!(is_diagonal_matrix_impl(&nd), false);
}

#[test]
fn test_is_up_tri_matrix_impl() {
    let ut = nm(&[1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 0.0, 0.0, 9.0], 3, 3);
    let lt = nm(&[1.0, 0.0, 0.0, 4.0, 5.0, 0.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(is_up_tri_matrix_impl(&ut), true);
    assert_eq!(is_up_tri_matrix_impl(&lt), false);
}

#[test]
fn test_is_lo_tri_matrix_impl() {
    let ut = nm(&[1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 0.0, 0.0, 9.0], 3, 3);
    let lt = nm(&[1.0, 0.0, 0.0, 4.0, 5.0, 0.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(is_lo_tri_matrix_impl(&ut), false);
    assert_eq!(is_lo_tri_matrix_impl(&lt), true);
}

#[test]
fn test_is_triangular_matrix_impl() {
    // upper triangular non-diagonal => xor(true,false) = true
    let ut = nm(&[1.0, 2.0, 3.0, 0.0, 5.0, 6.0, 0.0, 0.0, 9.0], 3, 3);
    assert_eq!(is_triangular_matrix_impl(&ut), true);
    // identity is both upper and lower triangular: xor = false
    let id = identity_matrix_impl(3);
    assert_eq!(is_triangular_matrix_impl(&id), false);
    // not triangular
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(is_triangular_matrix_impl(&m), false);
}

#[test]
fn test_is_matrix_symmetric_impl() {
    let s = nm(&[1.0, 2.0, 2.0, 4.0], 2, 2);
    let z = zero_matrix_impl(2, 2);
    let ns = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(is_matrix_symmetric_impl(&s), true);
    assert_eq!(is_matrix_symmetric_impl(&z), true);
    assert_eq!(is_matrix_symmetric_impl(&ns), false);
}

#[test]
fn test_has_zero_row_impl() {
    let with_zero = nm(&[0.0, 0.0, 0.0, 1.0, 2.0, 3.0], 2, 3);
    let no_zero = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(has_zero_row_impl(&with_zero), true);
    assert_eq!(has_zero_row_impl(&no_zero), false);
}

#[test]
fn test_has_zero_col_impl() {
    // The C version's hasZeroCol has weird indexing using m->rows in outer
    // and m->cols in inner. We mirror it. Test on a 2x2 matrix where col 1 is zero.
    let with_zero = nm(&[1.0, 0.0, 2.0, 0.0], 2, 2);
    assert_eq!(has_zero_col_impl(&with_zero), true);
    let no_zero = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    assert_eq!(has_zero_col_impl(&no_zero), false);
}

#[test]
fn test_transpose_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let t = transpose_matrix_impl(&m);
    assert_eq!(t.rows, 3);
    assert_eq!(t.cols, 2);
    assert_eq!(t.data, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
}

#[test]
fn test_trace_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(trace_matrix_impl(&m), 15.0);
}

#[test]
fn test_add_matrices_impl() {
    let a = nm(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    let b = nm(&[6.0, 4.0, 7.0, 2.0], 2, 2);
    let s = add_matrices_impl(&a, &b);
    assert_eq!(s.rows, 2);
    assert_eq!(s.cols, 2);
    assert_eq!(s.data, vec![10.0, 10.0, 10.0, 10.0]);
}

#[test]
fn test_pow_matrix_impl() {
    let m = nm(&[2.0, 3.0, 4.0, 5.0], 2, 2);
    let p = pow_matrix_impl(&m, 2.0);
    assert_eq!(p.data, vec![4.0, 9.0, 16.0, 25.0]);
}

#[test]
fn test_multiply_matrices_impl_2x2() {
    // C semantics: m.cols == n.cols && m.rows == n.rows (square equal)
    let a = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = nm(&[1.0, 1.0, 1.0, 1.0], 2, 2);
    let p = multiply_matrices_impl(&a, &b);
    assert_eq!(p.rows, 2);
    assert_eq!(p.cols, 2);
    assert_eq!(p.data, vec![3.0, 3.0, 7.0, 7.0]);
}

#[test]
fn test_multiply_matrices_impl_2x2_general() {
    let a = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let b = nm(&[5.0, 6.0, 7.0, 8.0], 2, 2);
    let p = multiply_matrices_impl(&a, &b);
    assert_eq!(p.data, vec![19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn test_multiply_matrices_impl_3x3() {
    let a = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let b = nm(&[9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0], 3, 3);
    let p = multiply_matrices_impl(&a, &b);
    assert_eq!(p.data, vec![30.0, 24.0, 18.0, 84.0, 69.0, 54.0, 138.0, 114.0, 90.0]);
}

#[test]
fn test_scale_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let s = scale_matrix_impl(&m, 10.0);
    assert_eq!(s.data, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
}

#[test]
fn test_sub_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let s = sub_matrix_impl(&m, 2, 2);
    assert_eq!(s.rows, 2);
    assert_eq!(s.cols, 2);
    assert_eq!(s.data, vec![1.0, 2.0, 4.0, 5.0]);
}

#[test]
fn test_element_minor_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(element_minor_impl(&m, 0, 0), -3.0);
    assert_eq!(element_minor_impl(&m, 0, 1), -6.0);
}

#[test]
fn test_matrix_minor_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let mm = matrix_minor_impl(&m);
    assert_eq!(mm.rows, 3);
    assert_eq!(mm.cols, 3);
    assert_eq!(
        mm.data,
        vec![-3.0, -6.0, -3.0, -6.0, -12.0, -6.0, -3.0, -6.0, -3.0]
    );
}

#[test]
fn test_element_cofactor_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(element_cofactor_impl(&m, 0, 0), -3.0);
    assert_eq!(element_cofactor_impl(&m, 0, 1), 6.0);
}

#[test]
fn test_matrix_cofactor_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let cf = matrix_cofactor_impl(&m);
    assert_eq!(
        cf.data,
        vec![-3.0, 6.0, -3.0, 6.0, -12.0, 6.0, -3.0, 6.0, -3.0]
    );
}

#[test]
fn test_sign_matrix_impl_2x2() {
    let s = sign_matrix_impl(2, 2);
    assert_eq!(s.data, vec![1.0, -1.0, 1.0, -1.0]);
}

#[test]
fn test_sign_matrix_impl_3x3() {
    let s = sign_matrix_impl(3, 3);
    assert_eq!(s.data, vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]);
}

#[test]
fn test_adjugate_matrix_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let adj = adjugate_matrix_impl(&m);
    assert_eq!(adj.rows, 2);
    assert_eq!(adj.cols, 2);
    assert_eq!(adj.data, vec![7.0, -7.0, 3.0, -3.0]);
}

#[test]
fn test_inverse_matrix_impl() {
    let m = nm(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let inv = inverse_matrix_impl(&m);
    let expected = vec![0.2, -0.2, 0.2, -0.2, 0.2, -0.2, 1.0, -1.0, 1.0];
    for (a, b) in inv.data.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-9, "inv mismatch: {} vs {}", a, b);
    }
}

#[test]
fn test_pivot_matrix_impl_no_swap() {
    let m = nm(&[1.0, 3.0, 5.0, 2.0, 4.0, 7.0, 1.0, 1.0, 0.0], 3, 3);
    let (p, _swaps) = pivot_matrix_impl(&m);
    assert_eq!(p.rows, 3);
    assert_eq!(p.cols, 3);
    // From C: 0 1 0  1 0 0  0 0 1
    assert_eq!(p.data, vec![0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_pivot_matrix_impl_159() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let (p, _swaps) = pivot_matrix_impl(&m);
    // From C: 0 0 1  1 0 0  0 1 0
    assert_eq!(p.data, vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
}

#[test]
fn test_lu_decomposition_impl() {
    let m = nm(&[1.0, 3.0, 5.0, 2.0, 4.0, 7.0, 1.0, 1.0, 0.0], 3, 3);
    let (l, u, p, swaps) = lu_decomposition_impl(&m);
    // From C: swaps=0
    assert_eq!(swaps, 0);
    // L: 1 0 0  0.5 1 0  0.5 -1 1
    let expected_l = vec![1.0, 0.0, 0.0, 0.5, 1.0, 0.0, 0.5, -1.0, 1.0];
    for (a, b) in l.data.iter().zip(expected_l.iter()) {
        assert!((a - b).abs() < 1e-9);
    }
    // U: 2 4 7  0 1 1.5  0 0 -2
    let expected_u = vec![2.0, 4.0, 7.0, 0.0, 1.0, 1.5, 0.0, 0.0, -2.0];
    for (a, b) in u.data.iter().zip(expected_u.iter()) {
        assert!((a - b).abs() < 1e-9);
    }
    // P: 0 1 0  1 0 0  0 0 1
    assert_eq!(p.data, vec![0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_scale_n_space_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let s = scale_n_space_impl(&m, 3.0);
    assert_eq!(s.data, vec![3.0, 6.0, 9.0, 12.0]);
}

#[test]
fn test_reflect_axis_2d_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let r0 = reflect_axis_2d_impl(&m, 0);
    // axis=0 (false in C): 0,0=-1; 1,1=1
    assert_eq!(r0.data, vec![-1.0, 2.0, -3.0, 4.0]);
    let r1 = reflect_axis_2d_impl(&m, 1);
    // axis=1 (true in C): 0,0=1; 1,1=-1
    assert_eq!(r1.data, vec![1.0, -2.0, 3.0, -4.0]);
}

#[test]
fn test_reflect_axis_3d_impl() {
    let m = identity_matrix_impl(3);
    // For identity input, output should be the reflection matrix itself
    let xy = reflect_axis_3d_impl(&m, 0);
    assert_eq!(xy.data, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0]);
    let xz = reflect_axis_3d_impl(&m, 1);
    assert_eq!(xz.data, vec![1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]);
    let yz = reflect_axis_3d_impl(&m, 2);
    assert_eq!(yz.data, vec![-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_orth_proj_2d_impl() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let p0 = orth_proj_2d_impl(&m, 0);
    assert_eq!(p0.data, vec![1.0, 0.0, 3.0, 0.0]);
    let p1 = orth_proj_2d_impl(&m, 1);
    assert_eq!(p1.data, vec![0.0, 2.0, 0.0, 4.0]);
}

#[test]
fn test_orth_proj_3d_impl() {
    let m = nm(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], 3, 3);
    let p0 = orth_proj_3d_impl(&m, 0);
    assert_eq!(p0.data, vec![1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0]);
    let p1 = orth_proj_3d_impl(&m, 1);
    assert_eq!(p1.data, vec![1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]);
    let p2 = orth_proj_3d_impl(&m, 2);
    assert_eq!(p2.data, vec![0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0]);
}

#[test]
fn test_shear_2d_impl() {
    let m = identity_matrix_impl(2);
    let s0 = shear_2d_impl(&m, 3.0, 0);
    assert_eq!(s0.data, vec![1.0, 0.0, 3.0, 1.0]);
    let s1 = shear_2d_impl(&m, 3.0, 1);
    assert_eq!(s1.data, vec![1.0, 3.0, 0.0, 1.0]);
}

#[test]
fn test_delete_matrix_impl_no_op() {
    let m = nm(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    delete_matrix_impl(m);
}

fn main() {}
