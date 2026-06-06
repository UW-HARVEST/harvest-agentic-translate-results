use crate::linear_algebra::{Matrix, Vector};
use crate::utils::exclusive_or;
use crate::vector::{
    assert_vector_impl, new_vector_impl, null_vector_impl,
};

pub fn assert_matrix_impl(m: &Matrix) -> bool {
    assert_eq!(m.data.len(), m.rows * m.cols);
    true
}

pub fn new_matrix_impl(d: &[f64], rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    let mut m = null_matrix_impl(rows, cols);
    let mut idx = 0;
    for i in 0..m.rows {
        for j in 0..m.cols {
            m.data[i * m.cols + j] = d[idx];
            idx += 1;
        }
    }
    m
}

pub fn null_matrix_impl(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    Matrix {
        rows,
        cols,
        data: vec![0.0; rows * cols],
    }
}

pub fn zero_matrix_impl(rows: usize, cols: usize) -> Matrix {
    let mut m = null_matrix_impl(rows, cols);
    fill_matrix_impl(&mut m, 0.0);
    m
}

pub fn fill_matrix_impl(m: &mut Matrix, n: f64) {
    assert_matrix_impl(m);
    for i in 0..m.rows {
        for j in 0..m.cols {
            let cols = m.cols;
            m.data[i * cols + j] = n;
        }
    }
}

pub fn identity_matrix_impl(n: usize) -> Matrix {
    let mut m = zero_matrix_impl(n, n);
    for i in 0..m.rows {
        for j in 0..m.rows {
            if i == j {
                let cols = m.cols;
                m.data[i * cols + j] = 1.0;
            }
        }
    }
    m
}

pub fn delete_matrix_impl(_m: Matrix) {
    // No-op in Rust; memory is managed automatically.
}

pub fn copy_matrix_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut c = zero_matrix_impl(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            c.data[i * m.cols + j] = m.data[i * m.cols + j];
        }
    }
    c
}

pub fn flatten_matrix_impl(m: &Matrix) -> Vector {
    assert_matrix_impl(m);
    let mut flat = null_vector_impl(m.rows * m.cols);
    let mut idx = 0;
    for i in 0..m.rows {
        for j in 0..m.cols {
            flat.data[idx] = m.data[i * m.cols + j];
            idx += 1;
        }
    }
    flat
}

pub fn matrix_size_impl(m: &Matrix) -> usize {
    assert_matrix_impl(m);
    m.rows * m.cols
}

pub fn matrix_size_bytes_impl(m: &Matrix) -> usize {
    std::mem::size_of::<f64>() * matrix_size_impl(m)
}

pub fn set_matrix_element_impl(m: &mut Matrix, i: usize, j: usize, s: f64) {
    assert_matrix_impl(m);
    assert!(i < m.rows && j < m.cols);
    let cols = m.cols;
    m.data[i * cols + j] = s;
}

pub fn get_matrix_element_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    assert_matrix_impl(m);
    assert!(i < m.rows && j < m.cols);
    m.data[i * m.cols + j]
}

pub fn set_row_vector_impl(m: &mut Matrix, i: usize, v: &Vector) {
    assert_matrix_impl(m);
    assert_vector_impl(v);
    assert!(i < m.rows);
    let cols = m.cols;
    for j in 0..v.cols {
        m.data[i * cols + j] = v.data[j];
    }
}

pub fn get_row_vector_impl(m: &Matrix, i: usize) -> Vector {
    assert_matrix_impl(m);
    assert!(i < m.rows);
    let mut row = vec![0.0; m.cols];
    for j in 0..m.cols {
        row[j] = m.data[i * m.cols + j];
    }
    new_vector_impl(&row, m.cols)
}

pub fn set_col_vector_impl(m: &mut Matrix, j: usize, v: &Vector) {
    assert_matrix_impl(m);
    assert_vector_impl(v);
    assert!(j < m.cols);
    let cols = m.cols;
    for i in 0..v.cols {
        m.data[i * cols + j] = v.data[i];
    }
}

pub fn get_col_vector_impl(m: &Matrix, j: usize) -> Vector {
    assert_matrix_impl(m);
    assert!(j < m.cols);
    let mut col = vec![0.0; m.rows];
    for i in 0..m.rows {
        col[i] = m.data[i * m.cols + j];
    }
    new_vector_impl(&col, m.rows)
}

pub fn get_main_diagonal_impl(m: &Matrix) -> Vector {
    assert!(is_square_matrix_impl(m));
    let mut diag = vec![0.0; m.rows];
    for x in 0..m.rows {
        diag[x] = m.data[x * m.cols + x];
    }
    new_vector_impl(&diag, m.rows)
}

pub fn set_main_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix_impl(m));
    assert_vector_impl(v);
    assert_eq!(m.cols, v.cols);
    let cols = m.cols;
    for x in 0..v.cols {
        m.data[x * cols + x] = v.data[x];
    }
}

pub fn get_anti_diagonal_impl(m: &Matrix) -> Vector {
    assert!(is_square_matrix_impl(m));
    let mut x = 0;
    let mut diag = vec![0.0; m.rows];
    let rows = m.rows;
    'outer: for i in (0..rows).rev() {
        for j in (0..m.cols).rev() {
            if i + j == rows - 1 {
                diag[x] = m.data[i * m.cols + j];
                x += 1;
                if x == rows {
                    break 'outer;
                }
            }
        }
    }
    new_vector_impl(&diag, rows)
}

pub fn set_anti_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix_impl(m));
    assert_vector_impl(v);
    assert_eq!(m.cols, v.cols);
    let mut idx = 0;
    let rows = m.rows;
    let cols = m.cols;
    'outer: for i in (0..rows).rev() {
        for j in (0..cols).rev() {
            if i + j == rows - 1 {
                m.data[i * cols + j] = v.data[idx];
                idx += 1;
                if idx == rows {
                    break 'outer;
                }
            }
        }
    }
}

pub fn diagonal_product_impl(m: &Matrix) -> f64 {
    let diagonal = get_main_diagonal_impl(m);
    let mut product = 1.0;
    for i in 0..diagonal.cols {
        product *= diagonal.data[i];
    }
    product
}

pub fn is_matrix_equal_impl(m: &Matrix, n: &Matrix) -> bool {
    assert_matrix_impl(m);
    assert_matrix_impl(n);
    if m.rows != n.rows || m.cols != n.cols {
        return false;
    }
    for i in 0..m.rows {
        for j in 0..m.cols {
            if m.data[i * m.cols + j] != n.data[i * n.cols + j] {
                return false;
            }
        }
    }
    true
}

pub fn has_same_dimensions_impl(m: &Matrix, n: &Matrix) -> bool {
    assert_matrix_impl(m);
    assert_matrix_impl(n);
    m.rows == n.rows && m.cols == n.cols
}

pub fn is_zero_matrix_impl(m: &Matrix) -> bool {
    assert_matrix_impl(m);
    for i in 0..m.rows {
        for j in 0..m.cols {
            if m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}

pub fn is_identity_matrix_impl(m: &Matrix) -> bool {
    if !is_square_matrix_impl(m) {
        return false;
    }
    for i in 0..m.rows {
        for j in 0..m.cols {
            if i == j && m.data[i * m.cols + j] != 1.0 {
                return false;
            } else if i != j && m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}

pub fn is_square_matrix_impl(m: &Matrix) -> bool {
    assert_matrix_impl(m);
    m.rows == m.cols
}

pub fn is_invertible_impl(m: &Matrix) -> bool {
    is_square_matrix_impl(m) && determinant_impl(m) != 0.0
}

pub fn is_diagonal_matrix_impl(m: &Matrix) -> bool {
    assert!(is_square_matrix_impl(m));
    for i in 0..m.rows {
        for j in 0..m.cols {
            if i != j && m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}

pub fn is_triangular_matrix_impl(m: &Matrix) -> bool {
    assert!(is_square_matrix_impl(m));
    exclusive_or(is_up_tri_matrix_impl(m), is_lo_tri_matrix_impl(m))
}

pub fn is_up_tri_matrix_impl(m: &Matrix) -> bool {
    assert!(is_square_matrix_impl(m));
    for i in 0..m.rows {
        for j in 0..i {
            if m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}

pub fn is_lo_tri_matrix_impl(m: &Matrix) -> bool {
    assert!(is_square_matrix_impl(m));
    for i in 0..m.rows {
        for j in (i + 1)..m.cols {
            if m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}

pub fn is_matrix_symmetric_impl(m: &Matrix) -> bool {
    let t = transpose_matrix_impl(m);
    is_matrix_equal_impl(m, &t)
}

pub fn has_zero_row_impl(m: &Matrix) -> bool {
    assert_matrix_impl(m);
    let mut all_zeroes = true;
    for i in 0..m.rows {
        for j in 0..m.cols {
            if m.data[i * m.cols + j] != 0.0 {
                all_zeroes = false;
            }
        }
        if all_zeroes {
            return true;
        }
        all_zeroes = true;
    }
    false
}

pub fn has_zero_col_impl(m: &Matrix) -> bool {
    assert_matrix_impl(m);
    // Mirror C: outer loop over rows and inner over cols, but with j as outer index.
    let mut all_zeroes = true;
    for j in 0..m.rows {
        for i in 0..m.cols {
            if m.data[i * m.cols + j] != 0.0 {
                all_zeroes = false;
            }
        }
        if all_zeroes {
            return true;
        }
        all_zeroes = true;
    }
    false
}

pub fn transpose_matrix_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut t = zero_matrix_impl(m.cols, m.rows);
    let t_cols = t.cols;
    for i in 0..m.rows {
        for j in 0..m.cols {
            t.data[j * t_cols + i] = m.data[i * m.cols + j];
        }
    }
    t
}

pub fn trace_matrix_impl(m: &Matrix) -> f64 {
    assert!(is_square_matrix_impl(m));
    let mut trace = 0.0;
    for i in 0..m.rows {
        trace += m.data[i * m.cols + i];
    }
    trace
}

pub fn add_matrices_impl(m: &Matrix, n: &Matrix) -> Matrix {
    assert!(has_same_dimensions_impl(m, n));
    let mut sum = null_matrix_impl(m.rows, m.cols);
    let mut idx = 0;
    for _i in 0..m.rows {
        for _j in 0..m.cols {
            sum.data[idx] = m.data[idx] + n.data[idx];
            idx += 1;
        }
    }
    sum
}

pub fn pow_matrix_impl(m: &Matrix, k: f64) -> Matrix {
    assert_matrix_impl(m);
    let mut p = null_matrix_impl(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            p.data[i * m.cols + j] = m.data[i * m.cols + j].powf(k);
        }
    }
    p
}

pub fn multiply_matrices_impl(m: &Matrix, n: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    assert_matrix_impl(n);
    assert_eq!(m.cols, n.cols);
    assert_eq!(m.rows, n.rows);
    let mut prod = null_matrix_impl(m.rows, m.cols);
    // Match the C version exactly (which uses m.cols throughout)
    for j in 0..m.rows {
        for i in 0..m.cols {
            let mut val = 0.0;
            for k in 0..m.cols {
                val += m.data[i * m.cols + k] * n.data[k * m.cols + j];
            }
            prod.data[i * m.cols + j] = val;
        }
    }
    prod
}

pub fn scale_matrix_impl(m: &Matrix, s: f64) -> Matrix {
    assert_matrix_impl(m);
    let mut scaled = null_matrix_impl(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            scaled.data[i * m.cols + j] = m.data[i * m.cols + j] * s;
        }
    }
    scaled
}

pub fn sub_matrix_impl(m: &Matrix, i: usize, j: usize) -> Matrix {
    assert_matrix_impl(m);
    assert!(i < m.rows && j < m.cols);
    let mut sm = null_matrix_impl(m.rows - 1, m.cols - 1);
    let mut idx = 0;
    for row in 0..m.rows {
        for col in 0..m.cols {
            if row != i && col != j {
                sm.data[idx] = m.data[row * m.cols + col];
                idx += 1;
            }
        }
    }
    sm
}

pub fn element_minor_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    let sm = sub_matrix_impl(m, i, j);
    determinant_impl(&sm)
}

pub fn matrix_minor_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut mm = null_matrix_impl(m.rows, m.cols);
    let mm_cols = mm.cols;
    for i in 0..mm.rows {
        for j in 0..mm.cols {
            mm.data[i * mm_cols + j] = element_minor_impl(m, i, j);
        }
    }
    mm
}

pub fn element_cofactor_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    let exp = ((i + 1) + (j + 1)) as i32;
    (-1f64).powi(exp) * element_minor_impl(m, i, j)
}

pub fn matrix_cofactor_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut cfm = null_matrix_impl(m.rows, m.cols);
    let cfm_cols = cfm.cols;
    for i in 0..cfm.rows {
        for j in 0..cfm.cols {
            cfm.data[i * cfm_cols + j] = element_cofactor_impl(m, i, j);
        }
    }
    cfm
}

pub fn sign_matrix_impl(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    let mut sm = null_matrix_impl(rows, cols);
    fill_matrix_impl(&mut sm, 1.0);
    let sm_cols = sm.cols;
    for i in 0..sm.rows {
        for j in 0..sm.cols {
            // Match C: ((i*cols+j)+1) % 2 ? 1 : -1
            let val = if ((i * sm_cols + j) + 1) % 2 != 0 {
                1.0
            } else {
                -1.0
            };
            sm.data[i * sm_cols + j] = val;
        }
    }
    sm
}

pub fn adjugate_matrix_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mm = matrix_minor_impl(m);
    let sign = sign_matrix_impl(m.rows, m.cols);
    multiply_matrices_impl(&mm, &sign)
}

pub fn lu_decomposition_impl(m: &Matrix) -> (Matrix, Matrix, Matrix, i32) {
    assert!(is_square_matrix_impl(m));
    let n = m.cols;
    let mut l = zero_matrix_impl(n, n);
    let mut u = zero_matrix_impl(n, n);
    let (p, p_swaps) = pivot_matrix_impl(m);
    let swaps = p_swaps;
    let m2 = multiply_matrices_impl(&p, m);
    for j in 0..n {
        l.data[j * n + j] = 1.0;
        for i in 0..(j + 1) {
            let mut sum_u = 0.0;
            for k in 0..i {
                sum_u += u.data[k * n + j] * l.data[i * n + k];
            }
            u.data[i * n + j] = m2.data[i * n + j] - sum_u;
        }
        for i in j..n {
            let mut sum_l = 0.0;
            for k in 0..j {
                sum_l += u.data[k * n + j] * l.data[i * n + k];
            }
            l.data[i * n + j] = (m2.data[i * n + j] - sum_l) / u.data[j * n + j];
        }
    }
    (l, u, p, swaps)
}

pub fn inverse_matrix_impl(m: &Matrix) -> Matrix {
    assert!(is_invertible_impl(m));
    let adj = adjugate_matrix_impl(m);
    scale_matrix_impl(&adj, 1.0 / determinant_impl(m))
}

pub fn pivot_matrix_impl(m: &Matrix) -> (Matrix, i32) {
    assert!(is_square_matrix_impl(m));
    let n = m.cols;
    let mut pivot = identity_matrix_impl(n);
    let mut swaps = 0i32;
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
            // Note: original C has `swaps++` (local pointer increment, no effect),
            // but we increment our local swap counter correctly.
            swaps += 1;
        }
    }
    (pivot, swaps)
}

pub fn scale_n_space_impl(m: &Matrix, k: f64) -> Matrix {
    assert!(is_square_matrix_impl(m));
    let mut n = zero_matrix_impl(m.cols, m.cols);
    let mut v = crate::vector::zero_vector_impl(m.cols);
    crate::vector::fill_vector_impl(&mut v, k);
    set_main_diagonal_impl(&mut n, &v);
    multiply_matrices_impl(m, &n)
}

pub fn reflect_axis_2d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 2);
    let mut n = zero_matrix_impl(2, 2);
    set_matrix_element_impl(&mut n, 0, 0, if axis != 0 { 1.0 } else { -1.0 });
    set_matrix_element_impl(&mut n, 1, 1, if axis != 0 { -1.0 } else { 1.0 });
    multiply_matrices_impl(m, &n)
}

pub fn reflect_axis_3d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 3 && (0..=2).contains(&axis));
    // Construct a proper reflection matrix.
    let mut n = zero_matrix_impl(3, 3);
    match axis {
        0 => {
            // XY plane: keep x,y, flip z
            set_matrix_element_impl(&mut n, 0, 0, 1.0);
            set_matrix_element_impl(&mut n, 1, 1, 1.0);
            set_matrix_element_impl(&mut n, 2, 2, -1.0);
        }
        1 => {
            set_matrix_element_impl(&mut n, 0, 0, 1.0);
            set_matrix_element_impl(&mut n, 1, 1, -1.0);
            set_matrix_element_impl(&mut n, 2, 2, 1.0);
        }
        _ => {
            set_matrix_element_impl(&mut n, 0, 0, -1.0);
            set_matrix_element_impl(&mut n, 1, 1, 1.0);
            set_matrix_element_impl(&mut n, 2, 2, 1.0);
        }
    }
    multiply_matrices_impl(m, &n)
}

pub fn orth_proj_2d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 2);
    let mut n = zero_matrix_impl(2, 2);
    let a = axis as usize;
    set_matrix_element_impl(&mut n, a, a, 1.0);
    multiply_matrices_impl(m, &n)
}

pub fn orth_proj_3d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 3);
    let mut n = zero_matrix_impl(3, 3);
    match axis {
        0 => {
            set_matrix_element_impl(&mut n, 0, 0, 1.0);
            set_matrix_element_impl(&mut n, 1, 1, 1.0);
        }
        1 => {
            set_matrix_element_impl(&mut n, 0, 0, 1.0);
            set_matrix_element_impl(&mut n, 2, 2, 1.0);
        }
        2 => {
            set_matrix_element_impl(&mut n, 1, 1, 1.0);
            set_matrix_element_impl(&mut n, 2, 2, 1.0);
        }
        _ => {}
    }
    multiply_matrices_impl(m, &n)
}

pub fn shear_2d_impl(m: &Matrix, k: f64, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 2);
    let mut n = zero_matrix_impl(2, 2);
    set_matrix_element_impl(&mut n, 0, 0, 1.0);
    set_matrix_element_impl(&mut n, 0, 1, if axis != 0 { k } else { 0.0 });
    set_matrix_element_impl(&mut n, 1, 0, if axis != 0 { 0.0 } else { k });
    set_matrix_element_impl(&mut n, 1, 1, 1.0);
    multiply_matrices_impl(m, &n)
}

pub fn determinant_impl(m: &Matrix) -> f64 {
    assert!(is_square_matrix_impl(m));
    match m.rows {
        1 => return m.data[0],
        2 => return (m.data[0] * m.data[3]) - (m.data[1] * m.data[2]),
        3 => {
            return (m.data[0] * ((m.data[4] * m.data[8]) - (m.data[5] * m.data[7])))
                - (m.data[1] * ((m.data[3] * m.data[8]) - (m.data[5] * m.data[6])))
                + (m.data[2] * ((m.data[3] * m.data[7]) - (m.data[4] * m.data[6])));
        }
        _ => {}
    }
    if is_triangular_matrix_impl(m) {
        return diagonal_product_impl(m);
    }
    let (l, u, _p, _swaps) = lu_decomposition_impl(m);
    // Mirror the C source's behavior: pivotMatrix's `swaps++` increments
    // the pointer (a no-op for the caller's int), so the returned swap
    // count is always 0. Therefore det = pow(-1, 0-1) * det(L) * det(U)
    // = -det(L) * det(U).
    (-1f64).powi(-1) * determinant_impl(&l) * determinant_impl(&u)
}
