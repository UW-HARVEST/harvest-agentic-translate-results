use crate::linear_algebra::{Matrix, Vector};
use crate::utils::exclusive_or;
use crate::vector::{
    assert_vector_impl, fill_vector_impl, new_vector_impl, zero_vector_impl,
};

pub fn assert_matrix_impl(m: &Matrix) -> bool {
    assert!(m.rows > 0 && m.cols > 0 && m.data.len() == m.rows * m.cols);
    true
}

pub fn new_matrix_impl(d: &[f64], rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0 && d.len() >= rows * cols);
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
        m.data[i * n + i] = 1.0;
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
    m.data[i * m.cols + j] = s;
}

pub fn get_matrix_element_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    assert_matrix_impl(m);
    assert!(i < m.rows && j < m.cols);
    m.data[i * m.cols + j]
}

pub fn set_row_vector_impl(m: &mut Matrix, i: usize, v: &Vector) {
    assert_matrix_impl(m);
    assert_vector_impl(v);
    assert!(i < m.rows && v.cols <= m.cols);

    for j in 0..v.cols {
        m.data[i * m.cols + j] = v.data[j];
    }
}

pub fn get_row_vector_impl(m: &Matrix, i: usize) -> Vector {
    assert_matrix_impl(m);
    assert!(i < m.rows);

    Vector {
        cols: m.cols,
        data: m.data[i * m.cols..(i + 1) * m.cols].to_vec(),
    }
}

pub fn set_col_vector_impl(m: &mut Matrix, j: usize, v: &Vector) {
    assert_matrix_impl(m);
    assert_vector_impl(v);
    assert!(j < m.cols && v.cols <= m.rows);

    for i in 0..v.cols {
        m.data[i * m.cols + j] = v.data[i];
    }
}

pub fn get_col_vector_impl(m: &Matrix, j: usize) -> Vector {
    assert_matrix_impl(m);
    assert!(j < m.cols);

    Vector {
        cols: m.rows,
        data: (0..m.rows).map(|i| m.data[i * m.cols + j]).collect(),
    }
}

pub fn get_main_diagonal_impl(m: &Matrix) -> Vector {
    assert!(is_square_matrix_impl(m));
    Vector {
        cols: m.rows,
        data: (0..m.rows).map(|x| m.data[x * m.cols + x]).collect(),
    }
}

pub fn set_main_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix_impl(m));
    assert_vector_impl(v);
    assert!(m.cols == v.cols);

    for x in 0..v.cols {
        m.data[x * m.cols + x] = v.data[x];
    }
}

pub fn get_anti_diagonal_impl(m: &Matrix) -> Vector {
    assert!(is_square_matrix_impl(m));
    let n = m.rows;
    Vector {
        cols: n,
        data: (0..n).map(|idx| m.data[(n - 1 - idx) * m.cols + idx]).collect(),
    }
}

pub fn set_anti_diagonal_impl(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix_impl(m));
    assert_vector_impl(v);
    assert!(m.cols == v.cols);

    let n = m.rows;
    for idx in 0..n {
        m.data[(n - 1 - idx) * m.cols + idx] = v.data[idx];
    }
}

pub fn diagonal_product_impl(m: &Matrix) -> f64 {
    get_main_diagonal_impl(m)
        .data
        .into_iter()
        .fold(1.0, |product, value| product * value)
}

pub fn is_matrix_equal_impl(m: &Matrix, n: &Matrix) -> bool {
    assert_matrix_impl(m);
    assert_matrix_impl(n);
    m.rows == n.rows && m.cols == n.cols && m.data == n.data
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
            let value = m.data[i * m.cols + j];
            if (i == j && value != 1.0) || (i != j && value != 0.0) {
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
        for j in i + 1..m.cols {
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
    (0..m.rows).any(|i| (0..m.cols).all(|j| m.data[i * m.cols + j] == 0.0))
}

pub fn has_zero_col_impl(m: &Matrix) -> bool {
    assert_matrix_impl(m);
    (0..m.cols).any(|j| (0..m.rows).all(|i| m.data[i * m.cols + j] == 0.0))
}

pub fn transpose_matrix_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut t = zero_matrix_impl(m.cols, m.rows);
    for i in 0..m.rows {
        for j in 0..m.cols {
            t.data[j * t.cols + i] = m.data[i * m.cols + j];
        }
    }
    t
}

pub fn trace_matrix_impl(m: &Matrix) -> f64 {
    assert!(is_square_matrix_impl(m));
    (0..m.rows).map(|i| m.data[i * m.cols + i]).sum()
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
                data.push(m.data[row * m.cols + col]);
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
    for i in 0..m.rows {
        for j in 0..m.cols {
            mm.data[i * mm.cols + j] = element_minor_impl(m, i, j);
        }
    }
    mm
}

pub fn element_cofactor_impl(m: &Matrix, i: usize, j: usize) -> f64 {
    let sign = if (i + j) % 2 == 0 { 1.0 } else { -1.0 };
    sign * element_minor_impl(m, i, j)
}

pub fn matrix_cofactor_impl(m: &Matrix) -> Matrix {
    assert_matrix_impl(m);
    let mut cfm = null_matrix_impl(m.rows, m.cols);
    for i in 0..cfm.rows {
        for j in 0..cfm.cols {
            cfm.data[i * cfm.cols + j] = element_cofactor_impl(m, i, j);
        }
    }
    cfm
}

pub fn sign_matrix_impl(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    let mut sm = null_matrix_impl(rows, cols);
    for i in 0..sm.rows {
        for j in 0..sm.cols {
            sm.data[i * sm.cols + j] = if ((i * sm.cols + j) + 1) % 2 == 1 {
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
    let (p, swaps) = pivot_matrix_impl(m);
    let m2 = multiply_matrices_impl(&p, m);

    for j in 0..n {
        l.data[j * n + j] = 1.0;
        for i in 0..=j {
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
    let mut swaps = 0;

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
            swaps += 1;
        }
    }

    (pivot, swaps)
}

pub fn print_matrix_impl(m: &Matrix, include_indices: bool) {
    assert_matrix_impl(m);
    for i in 0..m.rows {
        for j in 0..m.cols {
            if include_indices {
                print!("[{i},{j}] -> ");
            }
            print!("{:8.2} ", m.data[i * m.cols + j]);
        }
        println!();
    }
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
    assert!(is_square_matrix_impl(m) && m.cols == 3 && (0..=2).contains(&axis));
    let diag = match axis {
        0 => [1.0, 1.0, -1.0],
        1 => [1.0, -1.0, 1.0],
        _ => [-1.0, 1.0, 1.0],
    };
    let mut n = zero_matrix_impl(3, 3);
    let v = new_vector_impl(&diag, 3);
    set_main_diagonal_impl(&mut n, &v);
    multiply_matrices_impl(m, &n)
}

pub fn orth_proj_2d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 2 && (0..=1).contains(&axis));
    let mut n = zero_matrix_impl(2, 2);
    set_matrix_element_impl(&mut n, axis as usize, axis as usize, 1.0);
    multiply_matrices_impl(m, &n)
}

pub fn orth_proj_3d_impl(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 3 && (0..=2).contains(&axis));
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

pub fn rotate_2d_impl(m: &Matrix, theta: f64) -> Matrix {
    assert!(is_square_matrix_impl(m) && m.cols == 2);
    let data = [theta.cos(), theta.sin(), -theta.sin(), theta.cos()];
    let n = new_matrix_impl(&data, 2, 2);
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
    (-1.0f64).powi(swaps) * determinant_impl(&l) * determinant_impl(&u)
}

pub fn is_matrix_orthogonal_impl(m1: &Matrix, m2: &Matrix) -> bool {
    let inv = inverse_matrix_impl(m1);
    let transposed = transpose_matrix_impl(m2);
    is_matrix_equal_impl(&inv, &transposed)
}
