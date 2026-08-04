use Linear_Algebra_C::matrix as m_impl;
use Linear_Algebra_C::vector as v_impl;
use Linear_Algebra_C::linear_algebra::Matrix;

#[test]
fn test_assert_matrix_impl() {
    let m = m_impl::null_matrix_impl(2, 2);
    assert_eq!(m_impl::assert_matrix_impl(&m), true);
}

#[test]
fn test_new_matrix_impl() {
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let m = m_impl::new_matrix_impl(&data, 2, 2);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 2);
    assert_eq!(m.data, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_null_matrix_impl() {
    let m = m_impl::null_matrix_impl(3, 4);
    assert_eq!(m.rows, 3);
    assert_eq!(m.cols, 4);
    assert_eq!(m.data.len(), 12);
    for x in &m.data {
        assert_eq!(*x, 0.0);
    }
}

#[test]
fn test_zero_matrix_impl() {
    let m = m_impl::zero_matrix_impl(2, 3);
    assert_eq!(m.rows, 2);
    assert_eq!(m.cols, 3);
    assert_eq!(m.data, vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_fill_matrix_impl() {
    let mut m = m_impl::null_matrix_impl(2, 2);
    m_impl::fill_matrix_impl(&mut m, 3.5);
    assert_eq!(m.data, vec![3.5, 3.5, 3.5, 3.5]);
}

#[test]
fn test_identity_matrix_impl() {
    let id3 = m_impl::identity_matrix_impl(3);
    assert_eq!(id3.rows, 3);
    assert_eq!(id3.cols, 3);
    assert_eq!(id3.data, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_delete_matrix_impl_runs() {
    let m = Matrix { rows: 2, cols: 2, data: vec![1.0, 2.0, 3.0, 4.0] };
    m_impl::delete_matrix_impl(m);
}

#[test]
fn test_copy_matrix_impl() {
    let mut m = m_impl::zero_matrix_impl(2, 2);
    m_impl::set_matrix_element_impl(&mut m, 0, 1, 5.0);
    let n = m_impl::copy_matrix_impl(&m);
    assert_eq!(n.rows, 2);
    assert_eq!(n.cols, 2);
    assert_eq!(n.data, vec![0.0, 5.0, 0.0, 0.0]);
}

#[test]
fn test_flatten_matrix_impl() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let m = m_impl::new_matrix_impl(&data, 2, 3);
    let flat = m_impl::flatten_matrix_impl(&m);
    assert_eq!(flat.cols, 6);
    assert_eq!(flat.data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_matrix_size_impl() {
    let m = m_impl::null_matrix_impl(3, 5);
    assert_eq!(m_impl::matrix_size_impl(&m), 15);
}

#[test]
fn test_matrix_size_bytes_impl() {
    // 4 doubles = 32 bytes
    let m = m_impl::null_matrix_impl(2, 2);
    assert_eq!(m_impl::matrix_size_bytes_impl(&m), 32);
}

#[test]
fn test_set_get_matrix_element_impl() {
    let mut m = m_impl::zero_matrix_impl(2, 2);
    m_impl::set_matrix_element_impl(&mut m, 0, 0, 4.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 0, 0), 4.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 0, 1), 0.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 1, 0), 0.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 1, 1), 0.0);
}

#[test]
fn test_set_get_row_vector_impl() {
    let mut m = m_impl::zero_matrix_impl(3, 2);
    let v = v_impl::new_vector_impl(&[1.0, 4.0], 2);
    m_impl::set_row_vector_impl(&mut m, 1, &v);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 0, 0), 0.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 0, 1), 0.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 1, 0), 1.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 1, 1), 4.0);

    let row = m_impl::get_row_vector_impl(&m, 1);
    assert_eq!(row.cols, 2);
    assert_eq!(row.data, vec![1.0, 4.0]);
}

#[test]
fn test_get_row_vector_impl_2x3() {
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let r0 = m_impl::get_row_vector_impl(&m, 0);
    assert_eq!(r0.cols, 3);
    assert_eq!(r0.data, vec![1.0, 2.0, 3.0]);
    let r1 = m_impl::get_row_vector_impl(&m, 1);
    assert_eq!(r1.cols, 3);
    assert_eq!(r1.data, vec![4.0, 5.0, 6.0]);
}

#[test]
fn test_set_get_col_vector_impl() {
    let mut m = m_impl::zero_matrix_impl(3, 2);
    let v = v_impl::new_vector_impl(&[10.0, 3.0, 7.0], 3);
    m_impl::set_col_vector_impl(&mut m, 0, &v);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 0, 0), 10.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 1, 0), 3.0);
    assert_eq!(m_impl::get_matrix_element_impl(&m, 2, 0), 7.0);

    let m2 = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let c0 = m_impl::get_col_vector_impl(&m2, 0);
    assert_eq!(c0.cols, 3);
    assert_eq!(c0.data, vec![1.0, 3.0, 5.0]);
    let c1 = m_impl::get_col_vector_impl(&m2, 1);
    assert_eq!(c1.cols, 3);
    assert_eq!(c1.data, vec![2.0, 4.0, 6.0]);
}

#[test]
fn test_get_main_diagonal_impl() {
    let mut m = m_impl::zero_matrix_impl(2, 2);
    m_impl::set_matrix_element_impl(&mut m, 0, 0, 34.0);
    m_impl::set_matrix_element_impl(&mut m, 1, 1, 56.0);
    let d = m_impl::get_main_diagonal_impl(&m);
    assert_eq!(d.cols, 2);
    assert_eq!(d.data, vec![34.0, 56.0]);
}

#[test]
fn test_set_main_diagonal_impl() {
    let mut n = m_impl::zero_matrix_impl(2, 2);
    let w = v_impl::new_vector_impl(&[100.0, 4.0], 2);
    m_impl::set_main_diagonal_impl(&mut n, &w);
    assert_eq!(m_impl::get_matrix_element_impl(&n, 0, 0), 100.0);
    assert_eq!(m_impl::get_matrix_element_impl(&n, 0, 1), 0.0);
    assert_eq!(m_impl::get_matrix_element_impl(&n, 1, 0), 0.0);
    assert_eq!(m_impl::get_matrix_element_impl(&n, 1, 1), 4.0);
}

#[test]
fn test_get_anti_diagonal_impl() {
    let mut m = m_impl::zero_matrix_impl(2, 2);
    m_impl::set_matrix_element_impl(&mut m, 0, 1, 100.0);
    m_impl::set_matrix_element_impl(&mut m, 1, 0, 250.0);
    let d = m_impl::get_anti_diagonal_impl(&m);
    assert_eq!(d.cols, 2);
    // C ground truth: d[0]=250, d[1]=100
    assert_eq!(d.data, vec![250.0, 100.0]);

    // 3x3 anti-diagonal of [1..9]: [7, 5, 3]
    let m3 = m_impl::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    let d3 = m_impl::get_anti_diagonal_impl(&m3);
    assert_eq!(d3.cols, 3);
    assert_eq!(d3.data, vec![7.0, 5.0, 3.0]);
}

#[test]
fn test_set_anti_diagonal_impl() {
    let mut n = m_impl::zero_matrix_impl(2, 2);
    let e = v_impl::new_vector_impl(&[9.0, 8.0], 2);
    m_impl::set_anti_diagonal_impl(&mut n, &e);
    // C ground truth: n[1,0]=9, n[0,1]=8
    assert_eq!(m_impl::get_matrix_element_impl(&n, 1, 0), 9.0);
    assert_eq!(m_impl::get_matrix_element_impl(&n, 0, 1), 8.0);
    assert_eq!(m_impl::get_matrix_element_impl(&n, 0, 0), 0.0);
    assert_eq!(m_impl::get_matrix_element_impl(&n, 1, 1), 0.0);

    // 3x3 setAntiDiagonal with [10,20,30]
    let mut m3 = m_impl::zero_matrix_impl(3, 3);
    let v = v_impl::new_vector_impl(&[10.0, 20.0, 30.0], 3);
    m_impl::set_anti_diagonal_impl(&mut m3, &v);
    // C ground truth:
    // 0 0 30
    // 0 20 0
    // 10 0 0
    assert_eq!(m3.data, vec![0.0, 0.0, 30.0, 0.0, 20.0, 0.0, 10.0, 0.0, 0.0]);
}

#[test]
fn test_diagonal_product_impl() {
    // C ground truth: diagonalProduct([2,1,1,1,3,1,1,1,4]) = 24
    let m = m_impl::new_matrix_impl(&[2.0,1.0,1.0,1.0,3.0,1.0,1.0,1.0,4.0], 3, 3);
    assert_eq!(m_impl::diagonal_product_impl(&m), 24.0);
}

#[test]
fn test_is_matrix_equal_impl() {
    let m = m_impl::zero_matrix_impl(2, 2);
    let n = m_impl::zero_matrix_impl(2, 2);
    assert_eq!(m_impl::is_matrix_equal_impl(&m, &n), true);
    let mut m2 = m_impl::zero_matrix_impl(2, 2);
    m_impl::set_matrix_element_impl(&mut m2, 0, 0, 1.0);
    assert_eq!(m_impl::is_matrix_equal_impl(&m, &m2), false);
    let o = m_impl::zero_matrix_impl(3, 3);
    assert_eq!(m_impl::is_matrix_equal_impl(&m, &o), false);
}

#[test]
fn test_has_same_dimensions_impl() {
    let m = m_impl::null_matrix_impl(2, 3);
    let n = m_impl::null_matrix_impl(2, 3);
    let o = m_impl::null_matrix_impl(3, 2);
    assert_eq!(m_impl::has_same_dimensions_impl(&m, &n), true);
    assert_eq!(m_impl::has_same_dimensions_impl(&m, &o), false);
}

#[test]
fn test_is_zero_matrix_impl() {
    let m = m_impl::zero_matrix_impl(1, 3);
    let mut n = m_impl::zero_matrix_impl(2, 2);
    m_impl::set_matrix_element_impl(&mut n, 0, 0, 1.0);
    assert_eq!(m_impl::is_zero_matrix_impl(&m), true);
    assert_eq!(m_impl::is_zero_matrix_impl(&n), false);
}

#[test]
fn test_is_identity_matrix_impl() {
    let mut m = m_impl::zero_matrix_impl(2, 2);
    m_impl::set_matrix_element_impl(&mut m, 0, 0, 1.0);
    m_impl::set_matrix_element_impl(&mut m, 1, 1, 1.0);
    assert_eq!(m_impl::is_identity_matrix_impl(&m), true);

    let n = m_impl::zero_matrix_impl(1, 3);
    assert_eq!(m_impl::is_identity_matrix_impl(&n), false);

    let mut o = m_impl::zero_matrix_impl(3, 3);
    m_impl::set_matrix_element_impl(&mut o, 0, 0, 1.0);
    m_impl::set_matrix_element_impl(&mut o, 1, 1, 1.0);
    m_impl::set_matrix_element_impl(&mut o, 2, 2, 1.0);
    m_impl::set_matrix_element_impl(&mut o, 1, 2, -4.0);
    assert_eq!(m_impl::is_identity_matrix_impl(&o), false);

    let p = m_impl::identity_matrix_impl(2);
    assert_eq!(m_impl::is_identity_matrix_impl(&p), true);
}

#[test]
fn test_is_square_matrix_impl() {
    let m = m_impl::zero_matrix_impl(2, 2);
    let n = m_impl::zero_matrix_impl(1, 3);
    assert_eq!(m_impl::is_square_matrix_impl(&m), true);
    assert_eq!(m_impl::is_square_matrix_impl(&n), false);
}

#[test]
fn test_is_invertible_impl() {
    let m = m_impl::new_matrix_impl(&[1.0, 0.0, 0.0, 1.0], 2, 2);
    assert_eq!(m_impl::is_invertible_impl(&m), true);
    // Singular matrix
    let s = m_impl::new_matrix_impl(&[1.0, 2.0, 2.0, 4.0], 2, 2);
    assert_eq!(m_impl::is_invertible_impl(&s), false);
}

#[test]
fn test_is_diagonal_matrix_impl() {
    let m1 = m_impl::new_matrix_impl(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert_eq!(m_impl::is_diagonal_matrix_impl(&m1), false);
    let m2 = m_impl::new_matrix_impl(&[1.0,0.0,0.0,0.0,2.0,0.0,0.0,0.0,3.0], 3, 3);
    assert_eq!(m_impl::is_diagonal_matrix_impl(&m2), true);
    // 1x1 always diagonal
    let m3 = m_impl::new_matrix_impl(&[5.0], 1, 1);
    assert_eq!(m_impl::is_diagonal_matrix_impl(&m3), true);
}

#[test]
fn test_is_triangular_matrix_impl() {
    let m1 = m_impl::new_matrix_impl(&[1.0,0.0,0.0,4.0,5.0,0.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(m_impl::is_triangular_matrix_impl(&m1), true);
    // Diagonal: both upper and lower triangular -> XOR = false
    let m2 = m_impl::new_matrix_impl(&[1.0,0.0,0.0,0.0,2.0,0.0,0.0,0.0,3.0], 3, 3);
    assert_eq!(m_impl::is_triangular_matrix_impl(&m2), false);
}

#[test]
fn test_is_up_tri_matrix_impl() {
    let m1 = m_impl::new_matrix_impl(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert_eq!(m_impl::is_up_tri_matrix_impl(&m1), true);
    let m2 = m_impl::new_matrix_impl(&[1.0,0.0,0.0,4.0,5.0,0.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(m_impl::is_up_tri_matrix_impl(&m2), false);
}

#[test]
fn test_is_lo_tri_matrix_impl() {
    let m1 = m_impl::new_matrix_impl(&[1.0,0.0,0.0,4.0,5.0,0.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(m_impl::is_lo_tri_matrix_impl(&m1), true);
    let m2 = m_impl::new_matrix_impl(&[1.0,2.0,3.0,0.0,5.0,6.0,0.0,0.0,9.0], 3, 3);
    assert_eq!(m_impl::is_lo_tri_matrix_impl(&m2), false);
}

#[test]
fn test_is_matrix_symmetric_impl() {
    // Non-square never symmetric (transpose has different dims)
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    assert_eq!(m_impl::is_matrix_symmetric_impl(&m), false);
    // Zero matrix symmetric
    let n = m_impl::zero_matrix_impl(2, 2);
    assert_eq!(m_impl::is_matrix_symmetric_impl(&n), true);
    // Custom symmetric
    let s = m_impl::new_matrix_impl(&[1.0,2.0,3.0,2.0,4.0,5.0,3.0,5.0,6.0], 3, 3);
    assert_eq!(m_impl::is_matrix_symmetric_impl(&s), true);
}

#[test]
fn test_has_zero_row_impl() {
    // C ground truth: hasZeroRow [0,0,0,1,2,3] (2x3) = true
    let m1 = m_impl::new_matrix_impl(&[0.0,0.0,0.0,1.0,2.0,3.0], 2, 3);
    assert_eq!(m_impl::has_zero_row_impl(&m1), true);
    // hasZeroRow [1..6] = false
    let m2 = m_impl::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0], 2, 3);
    assert_eq!(m_impl::has_zero_row_impl(&m2), false);
    // 1x1 [0] -> true
    let m3 = m_impl::new_matrix_impl(&[0.0], 1, 1);
    assert_eq!(m_impl::has_zero_row_impl(&m3), true);
    // 1x1 [5] -> false
    let m4 = m_impl::new_matrix_impl(&[5.0], 1, 1);
    assert_eq!(m_impl::has_zero_row_impl(&m4), false);
}

#[test]
fn test_has_zero_col_impl() {
    // NOTE: C's hasZeroCol is buggy (it iterates j in 0..rows and i in 0..cols,
    // reading m->data[i * cols + j] which can read out of bounds for non-square
    // matrices and is UB). Rust mirrors the bug. We only test inputs where the
    // buggy index never goes out of bounds (square matrices and 1x1).

    // hasZeroCol [0,1,2,0,4,5,0,7,8] (3x3) = true (column 0 is all zero)
    let m3 = m_impl::new_matrix_impl(&[0.0,1.0,2.0,0.0,4.0,5.0,0.0,7.0,8.0], 3, 3);
    assert_eq!(m_impl::has_zero_col_impl(&m3), true);
    // 3x3 [1..9] - no zero col
    let m4 = m_impl::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    assert_eq!(m_impl::has_zero_col_impl(&m4), false);
    // 1x1 [0] -> true
    let m5 = m_impl::new_matrix_impl(&[0.0], 1, 1);
    assert_eq!(m_impl::has_zero_col_impl(&m5), true);
    // 1x1 [5] -> false
    let m6 = m_impl::new_matrix_impl(&[5.0], 1, 1);
    assert_eq!(m_impl::has_zero_col_impl(&m6), false);
}

#[test]
fn test_transpose_matrix_impl() {
    // C ground truth: transpose [1,2,3,4,5,6] (2x3) ->
    // [1,4]
    // [2,5]
    // [3,6]
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3);
    let t = m_impl::transpose_matrix_impl(&m);
    assert_eq!(t.rows, 3);
    assert_eq!(t.cols, 2);
    assert_eq!(m_impl::get_matrix_element_impl(&t, 0, 0), 1.0);
    assert_eq!(m_impl::get_matrix_element_impl(&t, 1, 0), 2.0);
    assert_eq!(m_impl::get_matrix_element_impl(&t, 2, 0), 3.0);
    assert_eq!(m_impl::get_matrix_element_impl(&t, 0, 1), 4.0);
    assert_eq!(m_impl::get_matrix_element_impl(&t, 1, 1), 5.0);
    assert_eq!(m_impl::get_matrix_element_impl(&t, 2, 1), 6.0);
}

#[test]
fn test_trace_matrix_impl() {
    // C ground truth: trace([1..9] 3x3) = 1+5+9 = 15
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(m_impl::trace_matrix_impl(&m), 15.0);
}

#[test]
fn test_add_matrices_impl() {
    let m1 = m_impl::new_matrix_impl(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    let m2 = m_impl::new_matrix_impl(&[6.0, 4.0, 7.0, 2.0], 2, 2);
    let sum = m_impl::add_matrices_impl(&m1, &m2);
    assert_eq!(sum.rows, 2);
    assert_eq!(sum.cols, 2);
    assert_eq!(sum.data, vec![10.0, 10.0, 10.0, 10.0]);
}

#[test]
fn test_pow_matrix_impl() {
    // C ground truth: powMatrix [1,2,3,4]^2 = [1,4,9,16]
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let p = m_impl::pow_matrix_impl(&m, 2.0);
    assert_eq!(p.rows, 2);
    assert_eq!(p.cols, 2);
    assert_eq!(p.data, vec![1.0, 4.0, 9.0, 16.0]);
}

#[test]
fn test_multiply_matrices_impl() {
    // C ground truth: [[1,2],[3,4]] * [[1,1],[1,1]] = [[3,3],[7,7]]
    let m1 = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let m2 = m_impl::new_matrix_impl(&[1.0, 1.0, 1.0, 1.0], 2, 2);
    let prod = m_impl::multiply_matrices_impl(&m1, &m2);
    assert_eq!(prod.rows, 2);
    assert_eq!(prod.cols, 2);
    assert_eq!(prod.data, vec![3.0, 3.0, 7.0, 7.0]);

    // C ground truth: [[1,2],[3,4]] * [[5,6],[7,8]] = [[19,22],[43,50]]
    let mb = m_impl::new_matrix_impl(&[5.0, 6.0, 7.0, 8.0], 2, 2);
    let p2 = m_impl::multiply_matrices_impl(&m1, &mb);
    assert_eq!(p2.data, vec![19.0, 22.0, 43.0, 50.0]);
}

#[test]
fn test_scale_matrix_impl() {
    // C ground truth: scale [1..6] (3x2) by 10 -> [10,20,30,40,50,60]
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2);
    let s = m_impl::scale_matrix_impl(&m, 10.0);
    assert_eq!(s.rows, 3);
    assert_eq!(s.cols, 2);
    assert_eq!(s.data, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]);
}

#[test]
fn test_sub_matrix_impl() {
    // C: subMatrix([1..9] 3x3, 2, 2) -> [1,2,4,5] (2x2)
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let s = m_impl::sub_matrix_impl(&m, 2, 2);
    assert_eq!(s.rows, 2);
    assert_eq!(s.cols, 2);
    assert_eq!(s.data, vec![1.0, 2.0, 4.0, 5.0]);

    // subMatrix at (0, 0)
    let s2 = m_impl::sub_matrix_impl(&m, 0, 0);
    assert_eq!(s2.data, vec![5.0, 6.0, 8.0, 9.0]);
}

#[test]
fn test_element_minor_impl() {
    // C: elementMinor([1..9] 3x3, 0, 0) = -3
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(m_impl::element_minor_impl(&m, 0, 0), -3.0);
    assert_eq!(m_impl::element_minor_impl(&m, 1, 1), -12.0);
}

#[test]
fn test_matrix_minor_impl() {
    // C ground truth for [1..9] 3x3:
    // -3 -6 -3
    // -6 -12 -6
    // -3 -6 -3
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let mm = m_impl::matrix_minor_impl(&m);
    assert_eq!(mm.rows, 3);
    assert_eq!(mm.cols, 3);
    assert_eq!(mm.data, vec![-3.0, -6.0, -3.0, -6.0, -12.0, -6.0, -3.0, -6.0, -3.0]);
}

#[test]
fn test_element_cofactor_impl() {
    // C: elementCofactor([1..9] 3x3, 0, 0) = -3, (0,1) = 6
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    assert_eq!(m_impl::element_cofactor_impl(&m, 0, 0), -3.0);
    assert_eq!(m_impl::element_cofactor_impl(&m, 0, 1), 6.0);
}

#[test]
fn test_matrix_cofactor_impl() {
    // C ground truth for [1..9] 3x3:
    // -3 6 -3
    // 6 -12 6
    // -3 6 -3
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3);
    let cof = m_impl::matrix_cofactor_impl(&m);
    assert_eq!(cof.rows, 3);
    assert_eq!(cof.cols, 3);
    assert_eq!(cof.data, vec![-3.0, 6.0, -3.0, 6.0, -12.0, 6.0, -3.0, 6.0, -3.0]);

    // 2x2 cofactor [4,6,3,8] -> [8,-3,-6,4]
    let m2 = m_impl::new_matrix_impl(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    let cof2 = m_impl::matrix_cofactor_impl(&m2);
    assert_eq!(cof2.data, vec![8.0, -3.0, -6.0, 4.0]);
}

#[test]
fn test_sign_matrix_impl() {
    // C ground truth signMatrix(3,3):
    // 1 -1 1
    // -1 1 -1
    // 1 -1 1
    let sm = m_impl::sign_matrix_impl(3, 3);
    assert_eq!(sm.rows, 3);
    assert_eq!(sm.cols, 3);
    assert_eq!(sm.data, vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]);

    // signMatrix(2,2):
    // 1 -1
    // 1 -1
    let sm2 = m_impl::sign_matrix_impl(2, 2);
    assert_eq!(sm2.rows, 2);
    assert_eq!(sm2.cols, 2);
    assert_eq!(sm2.data, vec![1.0, -1.0, 1.0, -1.0]);
}

#[test]
fn test_adjugate_matrix_impl() {
    // C ground truth: adjugate [3,0,2,2,0,-2,0,1,1]:
    // 2 -2 2
    // -2 2 -2
    // 10 -10 10
    let m = m_impl::new_matrix_impl(&[3.0, 0.0, 2.0, 2.0, 0.0, -2.0, 0.0, 1.0, 1.0], 3, 3);
    let adj = m_impl::adjugate_matrix_impl(&m);
    assert_eq!(adj.rows, 3);
    assert_eq!(adj.cols, 3);
    assert_eq!(adj.data, vec![2.0, -2.0, 2.0, -2.0, 2.0, -2.0, 10.0, -10.0, 10.0]);
}

#[test]
fn test_lu_decomposition_impl() {
    // Test on a simple 2x2 matrix
    // C output for 4x4 [11,9,24,2,1,5,2,6,3,17,18,1,2,5,7,1] -> swaps=0 (due to C bug)
    let m = m_impl::new_matrix_impl(&[11.0,9.0,24.0,2.0,1.0,5.0,2.0,6.0,3.0,17.0,18.0,1.0,2.0,5.0,7.0,1.0], 4, 4);
    let (l, u, p, swaps) = m_impl::lu_decomposition_impl(&m);
    assert_eq!(swaps, 0);
    assert_eq!(l.rows, 4);
    assert_eq!(l.cols, 4);
    assert_eq!(u.rows, 4);
    assert_eq!(u.cols, 4);
    assert_eq!(p.rows, 4);
    assert_eq!(p.cols, 4);
    // L diagonal = 1
    for i in 0..4 {
        assert_eq!(l.data[i * 4 + i], 1.0);
    }
    // U diag matches what we computed
    assert!((u.data[0] - 11.0).abs() < 1e-9);
    assert!((u.data[5] - 14.545454545454547).abs() < 1e-9);
    assert!((u.data[10] - (-3.4749999999999996)).abs() < 1e-9);
    assert!((u.data[15] - 0.510791366906476).abs() < 1e-6);
    // P matches C
    let expected_p = vec![1.0,0.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,0.0,1.0];
    assert_eq!(p.data, expected_p);
}

#[test]
fn test_inverse_matrix_impl_3x3() {
    // C ground truth: inverse([3,0,2,2,0,-2,0,1,1]) =
    // 0.2 -0.2 0.2
    // -0.2 0.2 -0.2
    // 1 -1 1
    let m = m_impl::new_matrix_impl(&[3.0,0.0,2.0,2.0,0.0,-2.0,0.0,1.0,1.0], 3, 3);
    let inv = m_impl::inverse_matrix_impl(&m);
    assert_eq!(inv.rows, 3);
    assert_eq!(inv.cols, 3);
    let exp = vec![0.2, -0.2, 0.2, -0.2, 0.2, -0.2, 1.0, -1.0, 1.0];
    for i in 0..9 {
        assert!((inv.data[i] - exp[i]).abs() < 1e-12, "i={}, got {}, expected {}", i, inv.data[i], exp[i]);
    }
}

#[test]
fn test_inverse_matrix_impl_2x2() {
    // C ground truth: inverse([4,6,3,8]) =
    // 0.7857142857 -0.7857142857
    // 0.7142857143 -0.7142857143
    let m = m_impl::new_matrix_impl(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    let inv = m_impl::inverse_matrix_impl(&m);
    assert_eq!(inv.rows, 2);
    assert_eq!(inv.cols, 2);
    assert!((inv.data[0] - 11.0/14.0).abs() < 1e-12);
    assert!((inv.data[1] - (-11.0/14.0)).abs() < 1e-12);
    assert!((inv.data[2] - 10.0/14.0).abs() < 1e-12);
    assert!((inv.data[3] - (-10.0/14.0)).abs() < 1e-12);
}

#[test]
fn test_pivot_matrix_impl() {
    // C ground truth: pivot of [11,9,24,2,1,5,2,6,3,17,18,1,2,5,7,1] (4x4)
    // swaps=0 (due to C pointer bug)
    // pivot:
    // 1 0 0 0
    // 0 0 1 0
    // 0 1 0 0
    // 0 0 0 1
    let m = m_impl::new_matrix_impl(&[11.0,9.0,24.0,2.0,1.0,5.0,2.0,6.0,3.0,17.0,18.0,1.0,2.0,5.0,7.0,1.0], 4, 4);
    let (p, swaps) = m_impl::pivot_matrix_impl(&m);
    assert_eq!(swaps, 0);
    assert_eq!(p.rows, 4);
    assert_eq!(p.cols, 4);
    let exp = vec![1.0,0.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,0.0,1.0];
    assert_eq!(p.data, exp);
}

#[test]
fn test_determinant_impl() {
    // C ground truth determinants:
    // det [10] = 10
    let m1 = m_impl::new_matrix_impl(&[10.0], 1, 1);
    assert_eq!(m_impl::determinant_impl(&m1), 10.0);

    // det [4,6,3,8] = 14
    let m2 = m_impl::new_matrix_impl(&[4.0, 6.0, 3.0, 8.0], 2, 2);
    assert_eq!(m_impl::determinant_impl(&m2), 14.0);

    // det [6,1,1,4,-2,5,2,8,7] = -306
    let m3 = m_impl::new_matrix_impl(&[6.0, 1.0, 1.0, 4.0, -2.0, 5.0, 2.0, 8.0, 7.0], 3, 3);
    assert_eq!(m_impl::determinant_impl(&m3), -306.0);

    // det [11,9,24,2,1,5,2,6,3,17,18,1,2,5,7,1] = 284 (rounded)
    let m4 = m_impl::new_matrix_impl(&[11.0,9.0,24.0,2.0,1.0,5.0,2.0,6.0,3.0,17.0,18.0,1.0,2.0,5.0,7.0,1.0], 4, 4);
    let det = m_impl::determinant_impl(&m4);
    assert!((det - 284.0).abs() < 1e-9, "got {}", det);

    // det [2,5,1,3] = 1
    let m5 = m_impl::new_matrix_impl(&[2.0, 5.0, 1.0, 3.0], 2, 2);
    assert_eq!(m_impl::determinant_impl(&m5), 1.0);
}

#[test]
fn test_determinant_impl_4x4_triangular() {
    // C ground truth: det 4x4 triangular [2,0,0,0,1,3,0,0,4,5,6,0,1,1,1,7] = 252
    let m = m_impl::new_matrix_impl(&[2.0,0.0,0.0,0.0, 1.0,3.0,0.0,0.0, 4.0,5.0,6.0,0.0, 1.0,1.0,1.0,7.0], 4, 4);
    let d = m_impl::determinant_impl(&m);
    assert_eq!(d, 252.0);
}

#[test]
fn test_reflect_axis_2d_impl() {
    // C ground truth:
    // axis=0 [1,2,3,4]: [-1,2,-3,4]
    // axis=1 [1,2,3,4]: [1,-2,3,-4]
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let r0 = m_impl::reflect_axis_2d_impl(&m, 0);
    assert_eq!(r0.rows, 2);
    assert_eq!(r0.cols, 2);
    assert_eq!(r0.data, vec![-1.0, 2.0, -3.0, 4.0]);

    let r1 = m_impl::reflect_axis_2d_impl(&m, 1);
    assert_eq!(r1.data, vec![1.0, -2.0, 3.0, -4.0]);
}

#[test]
fn test_orth_proj_2d_impl() {
    // C ground truth:
    // axis=0 [1,2,3,4]: [1,0,3,0]
    // axis=1 [1,2,3,4]: [0,2,0,4]
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let p0 = m_impl::orth_proj_2d_impl(&m, 0);
    assert_eq!(p0.data, vec![1.0, 0.0, 3.0, 0.0]);
    let p1 = m_impl::orth_proj_2d_impl(&m, 1);
    assert_eq!(p1.data, vec![0.0, 2.0, 0.0, 4.0]);
}

#[test]
fn test_orth_proj_3d_impl() {
    let m = m_impl::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    // axis=0 (XY): keep cols 0,1, zero col 2
    let p0 = m_impl::orth_proj_3d_impl(&m, 0);
    assert_eq!(p0.data, vec![1.0,2.0,0.0,4.0,5.0,0.0,7.0,8.0,0.0]);
    // axis=1 (XZ): keep cols 0,2, zero col 1
    let p1 = m_impl::orth_proj_3d_impl(&m, 1);
    assert_eq!(p1.data, vec![1.0,0.0,3.0,4.0,0.0,6.0,7.0,0.0,9.0]);
    // axis=2 (YZ): keep cols 1,2, zero col 0
    let p2 = m_impl::orth_proj_3d_impl(&m, 2);
    assert_eq!(p2.data, vec![0.0,2.0,3.0,0.0,5.0,6.0,0.0,8.0,9.0]);
}

#[test]
fn test_shear_2d_impl() {
    // C ground truth:
    // axis=0 k=5 [1,2,3,4]: [11,2,23,4]
    // axis=1 k=5 [1,2,3,4]: [1,7,3,19]
    let m = m_impl::new_matrix_impl(&[1.0, 2.0, 3.0, 4.0], 2, 2);
    let s0 = m_impl::shear_2d_impl(&m, 5.0, 0);
    assert_eq!(s0.data, vec![11.0, 2.0, 23.0, 4.0]);
    let s1 = m_impl::shear_2d_impl(&m, 5.0, 1);
    assert_eq!(s1.data, vec![1.0, 7.0, 3.0, 19.0]);
}

#[test]
fn test_scale_n_space_impl() {
    // C ground truth: scaleNSpace k=2 [1..9] (3x3) = each element * 2
    let m = m_impl::new_matrix_impl(&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0], 3, 3);
    let s = m_impl::scale_n_space_impl(&m, 2.0);
    assert_eq!(s.rows, 3);
    assert_eq!(s.cols, 3);
    assert_eq!(s.data, vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0]);
}

#[test]
fn test_reflect_axis_3d_impl() {
    // We test the apparent intent: reflection negates the appropriate axis.
    // axis=0 (XY): negate Z column -> diag = [1,1,-1]
    let m = m_impl::identity_matrix_impl(3);
    let r0 = m_impl::reflect_axis_3d_impl(&m, 0);
    // identity * diag([1,1,-1]) = diag([1,1,-1])
    assert_eq!(r0.data, vec![1.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,-1.0]);
    let r1 = m_impl::reflect_axis_3d_impl(&m, 1);
    assert_eq!(r1.data, vec![1.0,0.0,0.0, 0.0,-1.0,0.0, 0.0,0.0,1.0]);
    let r2 = m_impl::reflect_axis_3d_impl(&m, 2);
    assert_eq!(r2.data, vec![-1.0,0.0,0.0, 0.0,1.0,0.0, 0.0,0.0,1.0]);
}

fn main() {}
