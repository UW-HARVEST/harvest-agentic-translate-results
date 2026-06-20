use crate::linear_algebra::{Matrix, Vector};
use crate::utils::exclusive_or;
use crate::vector::{
    assert_vector_impl, fill_vector_impl, new_vector_impl, zero_vector_impl,
};

fn index(m: &Matrix, i: usize, j: usize) -> usize {
    i * m.cols + j
}

pub fn assert_matrix_impl(m: &Matrix) -> bool {
    assert!(m.rows > 0);
    assert!(m.cols > 0);
    assert_eq!(m.data.len(), m.rows * m.cols);
    true
}

pub fn new_matrix_impl(d: &[f64], rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    assert!(d.len() >= rows * cols);
    Matrix {
        rows,
        cols,
        data: d[..rows * cols].to_vec(),
    }
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
    for value in &mut m.data {
        *value = n;
    }
}

pub fn identity_matrix_impl(n: usize) -> Matrix {
    let mut m = zero_matrix_impl(n, n);
    for i in 0..n {
        let idx = index(&m, i, i);
        m.data[idx] = 1.0;
    }
    m
}

pub fn delete_matrix_impl(m: Matrix) {
    drop(m);
}

pub fn copy_matrix_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    m.clone()
}

pub fn flatten_matrix_impl(m: &Matrix) -> Vector {
    assert_matrix_impl(m);
    Vector {
        cols: m.rows * m.cols,
        data: m.data.clone(),
    }
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
    let idx = index(m, i, j);
    m.data[idx] = s;
}

pub fn get_matrix_element_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    assert_matrix_impl(m);
    assert!(i < m.rows && j < m.cols);
    m.data[index(m, i, j)]
}

pub fn set_row_vector_impl(m: &mut Matrix, i: usize, v: &Vector) {
    assert_matrix_impl(m);
    assert_vector_impl(v);
    assert!(i < m.rows);
    assert!(v.cols <= m.cols);
    for j in 0..v.cols {
        let idx = index(m, i, j);
        m.data[idx] = v.data[j];
    }
}

pub fn get_row_vector_impl(m: &Matrix, i: usize) -> Vector {
    assert_matrix_impl(m);
    assert!(i < m.rows);
    let start = i * m.cols;
    let end = start + m.cols;
    new_vector_impl(&m.data[start..end], m.cols)
}

pub fn set_col_vector_impl(m: &mut Matrix, j: usize, v: &Vector) {
    assert_matrix_impl(m);
    assert_vector_impl(v);
    assert!(j < m.cols);
    assert!(v.cols <= m.rows);
    for i in 0..v.cols {
        let idx = index(m, i, j);
        m.data[idx] = v.data[i];
    }
}

pub fn get_col_vector_impl(m: &Matrix, j: usize) -> Vector {
    assert_matrix_impl(m);
    assert!(j < m.cols);
    let mut data = Vec::with_capacity(m.rows);
    for i in 0..m.rows {
        data.push(m.data[index(m, i, j)]);
    }
    Vector {
        cols: m.rows,
        data,
    }
}

pub fn get_main_diagonal_impl(m: &Matrix) -> Vector {
    assert!(is_square_matrix_impl(m));
    let mut data = Vec::with_capacity(m.rows);
    for x in 0..m.rows {
        data.push(m.data[index(m, x, x)]);
    }
    Vector {
        cols: m.rows,
        data,
    }
}

pub fn set_main_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix_impl(m));
    assert_vector_impl(v);
    assert_eq!(m.cols, v.cols);
    for x in 0..v.cols {
        let idx = index(m, x, x);
        m.data[idx] = v.data[x];
    }
}

pub fn get_anti_diagonal_impl(m: &Matrix) -> Vector {
    assert!(is_square_matrix_impl(m));
    let mut data = Vec::with_capacity(m.rows);
    for x in 0..m.rows {
        data.push(m.data[index(m, m.rows - 1 - x, x)]);
    }
    Vector {
        cols: m.rows,
        data,
    }
}

pub fn set_anti_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix_impl(m));
    assert_vector_impl(v);
    assert_eq!(m.cols, v.cols);
    for x in 0..v.cols {
        let idx = index(m, m.rows - 1 - x, x);
        m.data[idx] = v.data[x];
    }
}

pub fn diagonal_product_impl(m: &Matrix) -> f64 {
    let diagonal = get_main_diagonal_impl(m);
    diagonal.data.into_iter().product()
}

pub fn is_matrix_equal_impl(m: &Matrix, n: &Matrix) -> bool {
    assert_matrix_impl(m);
    assert_matrix_impl(n);
    if m.rows != n.rows || m.cols != n.cols {
        return false;
    }
    m.data.iter().zip(&n.data).all(|(a, b)| a == b)
}

pub fn has_same_dimensions_impl(m: &Matrix, n: &Matrix) -> bool {
    assert_matrix_impl(m);
    assert_matrix_impl(n);
    m.rows == n.rows && m.cols == n.cols
}

pub fn is_zero_matrix_impl(m: &Matrix) -> bool {
    assert_matrix_impl(m);
    m.data.iter().all(|value| *value == 0.0)
}

pub fn is_identity_matrix_impl(m: &Matrix) -> bool {
    if !is_square_matrix_impl(m) {
        return false;
    }
    for i in 0..m.rows {
        for j in 0..m.cols {
            let value = m.data[index(m, i, j)];
            if i == j && value != 1.0 {
                return false;
            } else if i != j && value != 0.0 {
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
            if i != j && m.data[index(m, i, j)] != 0.0 {
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
            if m.data[index(m, i, j)] != 0.0 {
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
            if m.data[index(m, i, j)] != 0.0 {
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
    for i in 0..m.rows {
        let mut all_zeroes = true;
        for j in 0..m.cols {
            if m.data[index(m, i, j)] != 0.0 {
                all_zeroes = false;
            }
        }
        if all_zeroes {
            return true;
        }
    }
    false
}

pub fn has_zero_col_impl(m: &Matrix) -> bool {
    assert_matrix_impl(m);
    for j in 0..m.cols {
        let mut all_zeroes = true;
        for i in 0..m.rows {
            if m.data[index(m, i, j)] != 0.0 {
                all_zeroes = false;
            }
        }
        if all_zeroes {
            return true;
        }
    }
    false
}

pub fn transpose_matrix_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut t = zero_matrix_impl(m.cols, m.rows);
    for i in 0..m.rows {
        for j in 0..m.cols {
            let dst = index(&t, j, i);
            t.data[dst] = m.data[index(m, i, j)];
        }
    }
    t
}

pub fn trace_matrix_impl(m: &Matrix) -> f64 {
    assert!(is_square_matrix_impl(m));
    let mut trace = 0.0;
    for i in 0..m.rows {
        trace += m.data[index(m, i, i)];
    }
    trace
}

pub fn add_matrices_impl(m1: &Matrix, m2: &Matrix) -> Matrix {
    assert!(has_same_dimensions_impl(m1, m2));
    Matrix {
        rows: m1.rows,
        cols: m1.cols,
        data: m1.data.iter().zip(&m2.data).map(|(a, b)| a + b).collect(),
    }
}

pub fn pow_matrix_impl(m: &Matrix, k: f64) -> Matrix {
    assert_matrix_impl(m);
    Matrix {
        rows: m.rows,
        cols: m.cols,
        data: m.data.iter().map(|value| value.powf(k)).collect(),
    }
}

pub fn multiply_matrices_impl(m1: &Matrix, m2: &Matrix) -> Matrix {
    assert_matrix_impl(m1);
    assert_matrix_impl(m2);
    assert_eq!(m1.cols, m2.cols);
    assert_eq!(m1.rows, m2.rows);
    let mut prod = null_matrix_impl(m1.rows, m1.cols);
    for j in 0..m1.rows {
        for i in 0..m1.cols {
            let mut val = 0.0;
            for k in 0..m1.cols {
                val += m1.data[index(m1, i, k)] * m2.data[index(m2, k, j)];
            }
            let dst = index(&prod, i, j);
            prod.data[dst] = val;
        }
    }
    prod
}

pub fn scale_matrix_impl(m: &Matrix, s: f64) -> Matrix {
    assert_matrix_impl(m);
    Matrix {
        rows: m.rows,
        cols: m.cols,
        data: m.data.iter().map(|value| value * s).collect(),
    }
}

pub fn sub_matrix_impl(m: &Matrix, i: usize, j: usize) -> Matrix {
    assert_matrix_impl(m);
    assert!(i < m.rows && j < m.cols);
    let mut data = Vec::with_capacity((m.rows - 1) * (m.cols - 1));
    for row in 0..m.rows {
        for col in 0..m.cols {
            if row != i && col != j {
                data.push(m.data[index(m, row, col)]);
            }
        }
    }
    Matrix {
        rows: m.rows - 1,
        cols: m.cols - 1,
        data,
    }
}

pub fn element_minor_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    let sm = sub_matrix_impl(m, i, j);
    determinant_impl(&sm)
}

pub fn matrix_minor_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut mm = null_matrix_impl(m.rows, m.cols);
    for i in 0..mm.rows {
        for j in 0..mm.cols {
            let dst = index(&mm, i, j);
            mm.data[dst] = element_minor_impl(m, i, j);
        }
    }
    mm
}

pub fn element_cofactor_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    (-1.0_f64).powi((i as i32 + 1) + (j as i32 + 1)) * element_minor_impl(m, i, j)
}

pub fn matrix_cofactor_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut cfm = null_matrix_impl(m.rows, m.cols);
    for i in 0..cfm.rows {
        for j in 0..cfm.cols {
            let dst = index(&cfm, i, j);
            cfm.data[dst] = element_cofactor_impl(m, i, j);
        }
    }
    cfm
}

pub fn sign_matrix_impl(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    let mut sm = null_matrix_impl(rows, cols);
    fill_matrix_impl(&mut sm, 1.0);
    for i in 0..sm.rows {
        for j in 0..sm.cols {
            let idx = index(&sm, i, j);
            sm.data[idx] = if ((i * sm.cols + j) + 1) % 2 == 1 {
                1.0
            } else {
                -1.0
            };
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
    let (p, _) = pivot_matrix_impl(m);
    let m2 = multiply_matrices_impl(&p, m);

    for j in 0..n {
        let diag_idx = index(&l, j, j);
        l.data[diag_idx] = 1.0;
        for i in 0..=j {
            let mut sum_u = 0.0;
            for k in 0..i {
                sum_u += u.data[index(&u, k, j)] * l.data[index(&l, i, k)];
            }
            let dst = index(&u, i, j);
            u.data[dst] = m2.data[index(&m2, i, j)] - sum_u;
        }
        for i in j..n {
            let mut sum_l = 0.0;
            for k in 0..j {
                sum_l += u.data[index(&u, k, j)] * l.data[index(&l, i, k)];
            }
            let dst = index(&l, i, j);
            l.data[dst] = (m2.data[index(&m2, i, j)] - sum_l) / u.data[index(&u, j, j)];
        }
    }

    (l, u, p, 0)
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
    for i in 0..n {
        let mut max = m.data[index(m, i, i)];
        let mut row = i;
        for j in i..n {
            if m.data[index(m, j, i)] > max {
                max = m.data[index(m, j, i)];
                row = j;
            }
        }
        if i != row {
            let v = get_row_vector_impl(&pivot, i);
            let w = get_row_vector_impl(&pivot, row);
            set_row_vector_impl(&mut pivot, i, &w);
            set_row_vector_impl(&mut pivot, row, &v);
        }
    }
    (pivot, 0)
}

pub fn scale_n_space_impl(m: &Matrix, k: f64) -> Matrix {
    assert!(is_square_matrix_impl(m));
    let mut n = zero_matrix_impl(m.cols, m.cols);
    let mut v = zero_vector_impl(m.cols);
    fill_vector_impl(&mut v, k);
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
    assert!(is_square_matrix_impl(m) && m.cols == 3);
    assert!((0..=2).contains(&axis));
    let diagonal = match axis {
        0 => [1.0, 1.0, -1.0],
        1 => [1.0, -1.0, 1.0],
        _ => [-1.0, 1.0, 1.0],
    };
    let mut n = zero_matrix_impl(3, 3);
    for (i, value) in diagonal.into_iter().enumerate() {
        set_matrix_element_impl(&mut n, i, i, value);
    }
    multiply_matrices_impl(m, &n)
}

pub fn orth_proj_2d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 2);
    assert!((0..=1).contains(&axis));
    let mut n = zero_matrix_impl(2, 2);
    set_matrix_element_impl(&mut n, axis as usize, axis as usize, 1.0);
    multiply_matrices_impl(m, &n)
}

pub fn orth_proj_3d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 3);
    assert!((0..=2).contains(&axis));
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
        _ => {
            set_matrix_element_impl(&mut n, 1, 1, 1.0);
            set_matrix_element_impl(&mut n, 2, 2, 1.0);
        }
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

    let (l, u, _p, swaps) = lu_decomposition_impl(m);
    (-1.0_f64).powi(swaps - 1) * determinant_impl(&l) * determinant_impl(&u)
}
