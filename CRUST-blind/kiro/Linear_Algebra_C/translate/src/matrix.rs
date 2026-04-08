use crate::linear_algebra::{Matrix, Vector};
pub fn assert_matrix_impl(_m: &Matrix) -> bool {
    true
}
pub fn new_matrix_impl(d: &[f64], rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    Matrix { rows, cols, data: d[..rows * cols].to_vec() }
}
pub fn null_matrix_impl(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    Matrix { rows, cols, data: vec![0.0; rows * cols] }
}
pub fn zero_matrix_impl(rows: usize, cols: usize) -> Matrix {
    let mut m = null_matrix_impl(rows, cols);
    fill_matrix_impl(&mut m, 0.0);
    m
}
pub fn fill_matrix_impl(m: &mut Matrix, n: f64) {
    for x in m.data.iter_mut() { *x = n; }
}
pub fn identity_matrix_impl(n: usize) -> Matrix {
    let mut m = zero_matrix_impl(n, n);
    for i in 0..n { m.data[i * n + i] = 1.0; }
    m
}
pub fn delete_matrix_impl(_m: Matrix) {
    // drop
}
pub fn copy_matrix_impl(m: &Matrix) -> Matrix {
    Matrix { rows: m.rows, cols: m.cols, data: m.data.clone() }
}
pub fn flatten_matrix_impl(m: &Matrix) -> Vector {
    Vector { cols: m.rows * m.cols, data: m.data.clone() }
}
pub fn matrix_size_impl(m: &Matrix) -> usize {
    m.rows * m.cols
}
pub fn matrix_size_bytes_impl(m: &Matrix) -> usize {
    std::mem::size_of::<f64>() * matrix_size_impl(m)
}
pub fn set_matrix_element_impl(m: &mut Matrix, i: usize, j: usize, s: f64) {
    assert!(i < m.rows && j < m.cols);
    m.data[i * m.cols + j] = s;
}
pub fn get_matrix_element_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    assert!(i < m.rows && j < m.cols);
    m.data[i * m.cols + j]
}
pub fn set_row_vector_impl(m: &mut Matrix, i: usize, v: &Vector) {
    assert!(i < m.rows);
    for j in 0..v.cols { m.data[i * m.cols + j] = v.data[j]; }
}
pub fn get_row_vector_impl(m: &Matrix, i: usize) -> Vector {
    assert!(i < m.rows);
    Vector { cols: m.cols, data: (0..m.cols).map(|j| m.data[i * m.cols + j]).collect() }
}
pub fn set_col_vector_impl(m: &mut Matrix, j: usize, v: &Vector) {
    assert!(j < m.cols);
    for i in 0..v.cols { m.data[i * m.cols + j] = v.data[i]; }
}
pub fn get_col_vector_impl(m: &Matrix, j: usize) -> Vector {
    assert!(j < m.cols);
    Vector { cols: m.rows, data: (0..m.rows).map(|i| m.data[i * m.cols + j]).collect() }
}
pub fn get_main_diagonal_impl(m: &Matrix) -> Vector {
    assert!(m.rows == m.cols);
    Vector { cols: m.rows, data: (0..m.rows).map(|x| m.data[x * m.cols + x]).collect() }
}
pub fn set_main_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(m.rows == m.cols && m.cols == v.cols);
    for x in 0..v.cols { m.data[x * m.cols + x] = v.data[x]; }
}
pub fn get_anti_diagonal_impl(m: &Matrix) -> Vector {
    assert!(m.rows == m.cols);
    let n = m.rows;
    let mut diag = Vec::with_capacity(n);
    for i in (0..n).rev() {
        for j in (0..n).rev() {
            if i + j == n - 1 {
                diag.push(m.data[i * m.cols + j]);
                if diag.len() == n { break; }
            }
        }
    }
    Vector { cols: n, data: diag }
}
pub fn set_anti_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(m.rows == m.cols && m.cols == v.cols);
    let n = m.rows;
    let mut idx = 0;
    for i in (0..n).rev() {
        for j in (0..n).rev() {
            if i + j == n - 1 {
                m.data[i * m.cols + j] = v.data[idx];
                idx += 1;
                if idx == n { break; }
            }
        }
    }
}
pub fn diagonal_product_impl(m: &Matrix) -> f64 {
    let diag = get_main_diagonal_impl(m);
    diag.data.iter().product()
}
pub fn is_matrix_equal_impl(m: &Matrix, n: &Matrix) -> bool {
    if m.rows != n.rows || m.cols != n.cols { return false; }
    m.data == n.data
}
pub fn has_same_dimensions_impl(m: &Matrix, n: &Matrix) -> bool {
    m.rows == n.rows && m.cols == n.cols
}
pub fn is_zero_matrix_impl(m: &Matrix) -> bool {
    m.data.iter().all(|&x| x == 0.0)
}
pub fn is_identity_matrix_impl(m: &Matrix) -> bool {
    if m.rows != m.cols { return false; }
    for i in 0..m.rows {
        for j in 0..m.cols {
            let v = m.data[i * m.cols + j];
            if i == j { if v != 1.0 { return false; } }
            else if v != 0.0 { return false; }
        }
    }
    true
}
pub fn is_square_matrix_impl(m: &Matrix) -> bool {
    m.rows == m.cols
}
pub fn is_invertible_impl(m: &Matrix) -> bool {
    is_square_matrix_impl(m) && crate::linear_algebra::determinant(m) != 0.0
}
pub fn is_diagonal_matrix_impl(m: &Matrix) -> bool {
    assert!(m.rows == m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            if i != j && m.data[i * m.cols + j] != 0.0 { return false; }
        }
    }
    true
}
pub fn is_triangular_matrix_impl(m: &Matrix) -> bool {
    assert!(m.rows == m.cols);
    crate::utils::exclusive_or(is_up_tri_matrix_impl(m), is_lo_tri_matrix_impl(m))
}
pub fn is_up_tri_matrix_impl(m: &Matrix) -> bool {
    assert!(m.rows == m.cols);
    for i in 0..m.rows {
        for j in 0..i {
            if m.data[i * m.cols + j] != 0.0 { return false; }
        }
    }
    true
}
pub fn is_lo_tri_matrix_impl(m: &Matrix) -> bool {
    assert!(m.rows == m.cols);
    for i in 0..m.rows {
        for j in (i + 1)..m.cols {
            if m.data[i * m.cols + j] != 0.0 { return false; }
        }
    }
    true
}
pub fn is_matrix_symmetric_impl(m: &Matrix) -> bool {
    let t = transpose_matrix_impl(m);
    is_matrix_equal_impl(m, &t)
}
pub fn has_zero_row_impl(m: &Matrix) -> bool {
    for i in 0..m.rows {
        let mut all_zeroes = true;
        for j in 0..m.cols {
            if m.data[i * m.cols + j] != 0.0 { all_zeroes = false; }
        }
        if all_zeroes { return true; }
    }
    false
}
pub fn has_zero_col_impl(m: &Matrix) -> bool {
    // Match C bug: outer loop j over m.rows, inner loop i over m.cols
    for j in 0..m.rows {
        let mut all_zeroes = true;
        for i in 0..m.cols {
            if m.data[i * m.cols + j] != 0.0 { all_zeroes = false; }
        }
        if all_zeroes { return true; }
    }
    false
}
pub fn transpose_matrix_impl(m: &Matrix) -> Matrix {
    let mut t = zero_matrix_impl(m.cols, m.rows);
    for i in 0..m.rows {
        for j in 0..m.cols {
            t.data[j * t.cols + i] = m.data[i * m.cols + j];
        }
    }
    t
}
pub fn trace_matrix_impl(m: &Matrix) -> f64 {
    assert!(m.rows == m.cols);
    (0..m.rows).map(|i| m.data[i * m.cols + i]).sum()
}
pub fn add_matrices_impl(m1: &Matrix, m2: &Matrix) -> Matrix {
    assert!(m1.rows == m2.rows && m1.cols == m2.cols);
    Matrix {
        rows: m1.rows, cols: m1.cols,
        data: m1.data.iter().zip(m2.data.iter()).map(|(a, b)| a + b).collect(),
    }
}
pub fn pow_matrix_impl(m: &Matrix, k: f64) -> Matrix {
    Matrix {
        rows: m.rows, cols: m.cols,
        data: m.data.iter().map(|x| x.powf(k)).collect(),
    }
}
pub fn multiply_matrices_impl(m1: &Matrix, m2: &Matrix) -> Matrix {
    // Match C: requires same dimensions, loops j over rows, i over cols
    assert!(m1.cols == m2.cols && m1.rows == m2.rows);
    let mut prod = null_matrix_impl(m1.rows, m1.cols);
    for j in 0..m1.rows {
        for i in 0..m1.cols {
            let mut val = 0.0;
            for k in 0..m1.cols {
                val += m1.data[i * m1.cols + k] * m2.data[k * m1.cols + j];
            }
            prod.data[i * m1.cols + j] = val;
        }
    }
    prod
}
pub fn scale_matrix_impl(m: &Matrix, s: f64) -> Matrix {
    Matrix {
        rows: m.rows, cols: m.cols,
        data: m.data.iter().map(|x| x * s).collect(),
    }
}
pub fn sub_matrix_impl(m: &Matrix, i: usize, j: usize) -> Matrix {
    assert!(i < m.rows && j < m.cols);
    let mut data = Vec::with_capacity((m.rows - 1) * (m.cols - 1));
    for row in 0..m.rows {
        for col in 0..m.cols {
            if row != i && col != j {
                data.push(m.data[row * m.cols + col]);
            }
        }
    }
    Matrix { rows: m.rows - 1, cols: m.cols - 1, data }
}
pub fn element_minor_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    let sm = sub_matrix_impl(m, i, j);
    crate::linear_algebra::determinant(&sm)
}
pub fn matrix_minor_impl(m: &Matrix) -> Matrix {
    let mut mm = null_matrix_impl(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            mm.data[i * mm.cols + j] = element_minor_impl(m, i, j);
        }
    }
    mm
}
pub fn element_cofactor_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    (-1.0f64).powi((i as i32 + 1) + (j as i32 + 1)) * element_minor_impl(m, i, j)
}
pub fn matrix_cofactor_impl(m: &Matrix) -> Matrix {
    let mut cfm = null_matrix_impl(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            cfm.data[i * cfm.cols + j] = element_cofactor_impl(m, i, j);
        }
    }
    cfm
}
pub fn sign_matrix_impl(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    let mut sm = null_matrix_impl(rows, cols);
    fill_matrix_impl(&mut sm, 1.0);
    for i in 0..rows {
        for j in 0..cols {
            sm.data[i * cols + j] = if (i * cols + j + 1) % 2 != 0 { 1.0 } else { -1.0 };
        }
    }
    sm
}
pub fn adjugate_matrix_impl(m: &Matrix) -> Matrix {
    let mm = matrix_minor_impl(m);
    let sign = sign_matrix_impl(m.rows, m.cols);
    multiply_matrices_impl(&mm, &sign)
}
pub fn lu_decomposition_impl(m: &Matrix) -> (Matrix, Matrix, Matrix, i32) {
    assert!(m.rows == m.cols);
    let n = m.cols;
    let mut l = zero_matrix_impl(n, n);
    let mut u = zero_matrix_impl(n, n);
    let (p, swaps) = pivot_matrix_impl(m);
    let m2 = multiply_matrices_impl(&p, m);
    for j in 0..n {
        l.data[j * n + j] = 1.0;
        for i in 0..=j {
            let mut sum_u = 0.0;
            for k in 0..i { sum_u += u.data[k * n + j] * l.data[i * n + k]; }
            u.data[i * n + j] = m2.data[i * n + j] - sum_u;
        }
        for i in j..n {
            let mut sum_l = 0.0;
            for k in 0..j { sum_l += u.data[k * n + j] * l.data[i * n + k]; }
            l.data[i * n + j] = (m2.data[i * n + j] - sum_l) / u.data[j * n + j];
        }
    }
    (l, u, p, swaps)
}
pub fn inverse_matrix_impl(m: &Matrix) -> Matrix {
    assert!(is_invertible_impl(m));
    let adj = adjugate_matrix_impl(m);
    let det = crate::linear_algebra::determinant(m);
    scale_matrix_impl(&adj, 1.0 / det)
}
pub fn pivot_matrix_impl(m: &Matrix) -> (Matrix, i32) {
    assert!(m.rows == m.cols);
    let n = m.cols;
    let mut pivot = identity_matrix_impl(n);
    // Match C bug: swaps++ increments the pointer, not the value.
    // So swaps is always 0 effectively (the value pointed to never changes).
    let swaps = 0i32;
    for i in 0..n {
        let mut max = m.data[i * n + i];
        let mut row = i;
        for j in i..n {
            if m.data[j * n + i] > max {
                max = m.data[j * n + i];
                row = j;
            }
        }
        if i != row {
            let v = get_row_vector_impl(&pivot, i);
            let w = get_row_vector_impl(&pivot, row);
            set_row_vector_impl(&mut pivot, i, &w);
            set_row_vector_impl(&mut pivot, row, &v);
            // C bug: swaps++ increments the pointer, not *swaps.
            // The pointed-to value is never modified, so swaps stays 0.
        }
    }
    (pivot, swaps)
}
pub fn scale_n_space_impl(m: &Matrix, k: f64) -> Matrix {
    assert!(m.rows == m.cols);
    let mut n = zero_matrix_impl(m.cols, m.cols);
    let mut v = crate::vector::zero_vector_impl(m.cols);
    crate::vector::fill_vector_impl(&mut v, k);
    set_main_diagonal_impl(&mut n, &v);
    multiply_matrices_impl(m, &n)
}
pub fn reflect_axis_2d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(m.rows == m.cols && m.cols == 2);
    let mut n = zero_matrix_impl(2, 2);
    n.data[0] = if axis != 0 { 1.0 } else { -1.0 };
    n.data[3] = if axis != 0 { -1.0 } else { 1.0 };
    multiply_matrices_impl(m, &n)
}
pub fn reflect_axis_3d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(m.rows == m.cols && m.cols == 3 && axis >= 0 && axis <= 2);
    let n = null_matrix_impl(3, 3);
    // C code modifies m's diagonal (bug: uses setMainDiagonal on m, not n)
    // and then multiplies m * n (where n is uninitialized/null).
    // We replicate: set m's diagonal, then multiply m * n
    let mut m_copy = copy_matrix_impl(m);
    let diag = match axis {
        0 => vec![1.0, 1.0, -1.0],  // XY
        1 => vec![1.0, -1.0, 1.0],  // XZ
        _ => vec![-1.0, 1.0, 1.0],  // YZ
    };
    let v = Vector { cols: 3, data: diag };
    set_main_diagonal_impl(&mut m_copy, &v);
    multiply_matrices_impl(&m_copy, &n)
}
pub fn orth_proj_2d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(m.rows == m.cols && m.cols == 2);
    let mut n = zero_matrix_impl(2, 2);
    n.data[axis as usize * 2 + axis as usize] = 1.0;
    multiply_matrices_impl(m, &n)
}
pub fn orth_proj_3d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(m.rows == m.cols && m.cols == 3);
    let mut n = zero_matrix_impl(3, 3);
    match axis {
        0 => { n.data[0] = 1.0; n.data[4] = 1.0; }  // XY
        1 => { n.data[0] = 1.0; n.data[8] = 1.0; }  // XZ
        2 => { n.data[4] = 1.0; n.data[8] = 1.0; }  // YZ
        _ => {}
    }
    multiply_matrices_impl(m, &n)
}
pub fn shear_2d_impl(m: &Matrix, k: f64, axis: i32) -> Matrix {
    assert!(m.rows == m.cols && m.cols == 2);
    let mut n = zero_matrix_impl(2, 2);
    n.data[0] = 1.0;
    n.data[1] = if axis != 0 { k } else { 0.0 };
    n.data[2] = if axis != 0 { 0.0 } else { k };
    n.data[3] = 1.0;
    multiply_matrices_impl(m, &n)
}
