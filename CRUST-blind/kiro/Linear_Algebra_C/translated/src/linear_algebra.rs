#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Vector {
    pub cols: usize,
    pub data: Vec<f64>,
}

use crate::utils::exclusive_or;

// --- Helper assertion functions ---
pub fn assert_matrix(m: &Matrix) -> bool {
    assert!(!m.data.is_empty());
    true
}
pub fn assert_vector(v: &Vector) -> bool {
    assert!(!v.data.is_empty());
    true
}
// --- Creation functions ---
pub fn new_matrix(d: &[f64], rows: usize, cols: usize) -> Matrix {
    assert!(!d.is_empty() && rows > 0 && cols > 0);
    Matrix { rows, cols, data: d[..rows * cols].to_vec() }
}
pub fn new_vector(d: &[f64], cols: usize) -> Vector {
    assert!(!d.is_empty() && cols > 0);
    Vector { cols, data: d[..cols].to_vec() }
}
pub fn null_matrix(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    Matrix { rows, cols, data: vec![0.0; rows * cols] }
}
pub fn null_vector(cols: usize) -> Vector {
    assert!(cols > 0);
    Vector { cols, data: vec![0.0; cols] }
}
pub fn zero_matrix(rows: usize, cols: usize) -> Matrix {
    null_matrix(rows, cols)
}
pub fn zero_vector(cols: usize) -> Vector {
    null_vector(cols)
}
// --- Fill functions ---
pub fn fill_matrix(m: &mut Matrix, n: f64) {
    for v in m.data.iter_mut() { *v = n; }
}
pub fn fill_vector(v: &mut Vector, n: f64) {
    for x in v.data.iter_mut() { *x = n; }
}
/// Returns an identity matrix of size n.
pub fn identity_matrix(n: usize) -> Matrix {
    let mut m = zero_matrix(n, n);
    for i in 0..n { m.data[i * n + i] = 1.0; }
    m
}
/// "Releases" a matrix (stub; memory is managed automatically in Rust).
pub fn delete_matrix(_m: Matrix) {}
/// "Releases" a vector.
pub fn delete_vector(_v: Vector) {}
/// Returns a copy of the given matrix.
pub fn copy_matrix(m: &Matrix) -> Matrix { m.clone() }
/// Returns a copy of the given vector.
pub fn copy_vector(v: &Vector) -> Vector { v.clone() }
/// Flattens the given matrix into a vector.
pub fn flatten_matrix(m: &Matrix) -> Vector {
    Vector { cols: m.rows * m.cols, data: m.data.clone() }
}
// --- Size functions ---
pub fn matrix_size(m: &Matrix) -> usize { m.rows * m.cols }
pub fn vector_size(v: &Vector) -> usize { v.cols }
pub fn matrix_size_bytes(m: &Matrix) -> usize {
    std::mem::size_of::<f64>() * matrix_size(m)
}
pub fn vector_size_bytes(v: &Vector) -> usize {
    std::mem::size_of::<f64>() * vector_size(v)
}
// --- Element accessor/mutator functions ---
pub fn set_matrix_element(m: &mut Matrix, i: usize, j: usize, s: f64) {
    assert!(i < m.rows && j < m.cols);
    m.data[i * m.cols + j] = s;
}
pub fn get_matrix_element(m: &Matrix, i: usize, j: usize) -> f64 {
    assert!(i < m.rows && j < m.cols);
    m.data[i * m.cols + j]
}
pub fn set_vector_element(v: &mut Vector, i: usize, s: f64) {
    assert!(i < v.cols);
    v.data[i] = s;
}
pub fn get_vector_element(v: &Vector, i: usize) -> f64 {
    assert!(i < v.cols);
    v.data[i]
}
// --- Row and column operations ---
pub fn set_row_vector(m: &mut Matrix, i: usize, v: &Vector) {
    assert!(i < m.rows);
    for j in 0..v.cols { m.data[i * m.cols + j] = v.data[j]; }
}
pub fn get_row_vector(m: &Matrix, i: usize) -> Vector {
    assert!(i < m.rows);
    let start = i * m.cols;
    Vector { cols: m.cols, data: m.data[start..start + m.cols].to_vec() }
}
pub fn set_col_vector(m: &mut Matrix, j: usize, v: &Vector) {
    assert!(j < m.cols);
    for i in 0..v.cols { m.data[i * m.cols + j] = v.data[i]; }
}
pub fn get_col_vector(m: &Matrix, j: usize) -> Vector {
    assert!(j < m.cols);
    Vector { cols: m.rows, data: (0..m.rows).map(|i| m.data[i * m.cols + j]).collect() }
}
// --- Diagonal operations ---
pub fn get_main_diagonal(m: &Matrix) -> Vector {
    assert!(is_square_matrix(m));
    Vector { cols: m.rows, data: (0..m.rows).map(|i| m.data[i * m.cols + i]).collect() }
}
pub fn set_main_diagonal(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix(m) && m.cols == v.cols);
    for i in 0..v.cols { m.data[i * m.cols + i] = v.data[i]; }
}
pub fn get_anti_diagonal(m: &Matrix) -> Vector {
    assert!(is_square_matrix(m));
    let n = m.rows;
    let mut data = Vec::with_capacity(n);
    for i in (0..n).rev() {
        for j in (0..n).rev() {
            if i + j == n - 1 {
                data.push(m.data[i * m.cols + j]);
                if data.len() == n { break; }
            }
        }
    }
    Vector { cols: n, data }
}
pub fn set_anti_diagonal(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix(m) && m.cols == v.cols);
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
pub fn diagonal_product(m: &Matrix) -> f64 {
    let diag = get_main_diagonal(m);
    diag.data.iter().product()
}
// --- "Pretty" print functions ---
pub fn print_matrix(m: &Matrix, include_indices: bool) {
    for i in 0..m.rows {
        for j in 0..m.cols {
            if include_indices { print!("[{},{}] -> ", i, j); }
            print!("{:8.2} ", m.data[i * m.cols + j]);
        }
        println!();
    }
}
pub fn print_vector(v: &Vector, include_indices: bool) {
    for i in 0..v.cols {
        if include_indices { print!("[{}] -> ", i); }
        print!("{:16.8} ", v.data[i]);
    }
}
// --- Comparison functions ---
pub fn is_matrix_equal(m: &Matrix, n: &Matrix) -> bool {
    m.rows == n.rows && m.cols == n.cols && m.data == n.data
}
pub fn is_vector_equal(v: &Vector, w: &Vector) -> bool {
    v.cols == w.cols && v.data == w.data
}
pub fn has_same_dimensions(m: &Matrix, n: &Matrix) -> bool {
    m.rows == n.rows && m.cols == n.cols
}
// --- Property testing functions ---
pub fn is_zero_matrix(m: &Matrix) -> bool {
    m.data.iter().all(|&x| x == 0.0)
}
pub fn is_identity_matrix(m: &Matrix) -> bool {
    if !is_square_matrix(m) { return false; }
    for i in 0..m.rows {
        for j in 0..m.cols {
            let expected = if i == j { 1.0 } else { 0.0 };
            if m.data[i * m.cols + j] != expected { return false; }
        }
    }
    true
}
pub fn is_square_matrix(m: &Matrix) -> bool { m.rows == m.cols }
pub fn is_invertible(m: &Matrix) -> bool {
    is_square_matrix(m) && determinant(m) != 0.0
}
pub fn is_diagonal_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    for i in 0..m.rows {
        for j in 0..m.cols {
            if i != j && m.data[i * m.cols + j] != 0.0 { return false; }
        }
    }
    true
}
pub fn is_triangular_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    exclusive_or(is_up_tri_matrix(m), is_lo_tri_matrix(m))
}
pub fn is_up_tri_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    for i in 0..m.rows {
        for j in 0..i {
            if m.data[i * m.cols + j] != 0.0 { return false; }
        }
    }
    true
}
pub fn is_lo_tri_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    for i in 0..m.rows {
        for j in (i + 1)..m.cols {
            if m.data[i * m.cols + j] != 0.0 { return false; }
        }
    }
    true
}
pub fn is_matrix_symmetric(m: &Matrix) -> bool {
    let t = transpose_matrix(m);
    is_matrix_equal(m, &t)
}
pub fn has_zero_row(m: &Matrix) -> bool {
    for i in 0..m.rows {
        if (0..m.cols).all(|j| m.data[i * m.cols + j] == 0.0) { return true; }
    }
    false
}
pub fn has_zero_col(m: &Matrix) -> bool {
    for j in 0..m.cols {
        if (0..m.rows).all(|i| m.data[i * m.cols + j] == 0.0) { return true; }
    }
    false
}
// --- Advanced operations ---
pub fn transpose_matrix(m: &Matrix) -> Matrix {
    let mut t = zero_matrix(m.cols, m.rows);
    for i in 0..m.rows {
        for j in 0..m.cols {
            t.data[j * t.cols + i] = m.data[i * m.cols + j];
        }
    }
    t
}
pub fn trace_matrix(m: &Matrix) -> f64 {
    assert!(is_square_matrix(m));
    (0..m.rows).map(|i| m.data[i * m.cols + i]).sum()
}
pub fn add_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    assert!(has_same_dimensions(m1, m2));
    Matrix {
        rows: m1.rows, cols: m1.cols,
        data: m1.data.iter().zip(&m2.data).map(|(a, b)| a + b).collect(),
    }
}
pub fn add_vectors(v1: &Vector, v2: &Vector) -> Vector {
    assert!(v1.cols == v2.cols);
    Vector { cols: v1.cols, data: v1.data.iter().zip(&v2.data).map(|(a, b)| a + b).collect() }
}
pub fn pow_matrix(m: &Matrix, k: f64) -> Matrix {
    Matrix {
        rows: m.rows, cols: m.cols,
        data: m.data.iter().map(|x| x.powf(k)).collect(),
    }
}
pub fn pow_vector(v: &Vector, k: f64) -> Vector {
    Vector { cols: v.cols, data: v.data.iter().map(|x| x.powf(k)).collect() }
}
pub fn multiply_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    assert!(m1.cols == m2.cols && m1.rows == m2.rows);
    let mut prod = null_matrix(m1.rows, m1.cols);
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
pub fn scale_matrix(m: &Matrix, s: f64) -> Matrix {
    Matrix { rows: m.rows, cols: m.cols, data: m.data.iter().map(|x| x * s).collect() }
}
pub fn dot_product(v: &Vector, w: &Vector) -> f64 {
    assert!(v.cols == w.cols);
    v.data.iter().zip(&w.data).map(|(a, b)| a * b).sum()
}
pub fn cross_product(v: &Vector, w: &Vector) -> Vector {
    assert!(v.cols == 3 && w.cols == 3);
    Vector {
        cols: 3,
        data: vec![
            v.data[1] * w.data[2] - v.data[2] * w.data[1],
            v.data[0] * w.data[2] - v.data[2] * w.data[0],
            v.data[0] * w.data[1] - v.data[1] * w.data[0],
        ],
    }
}
pub fn vector_magnitude(v: &Vector) -> f64 {
    v.data.iter().map(|x| x * x).sum::<f64>().sqrt()
}
pub fn vector_distance(v: &Vector, w: &Vector) -> f64 {
    assert!(v.cols == w.cols);
    v.data.iter().zip(&w.data).map(|(a, b)| (b - a) * (b - a)).sum::<f64>().sqrt()
}
pub fn scale_vector(v: &Vector, s: f64) -> Vector {
    Vector { cols: v.cols, data: v.data.iter().map(|x| x * s).collect() }
}
pub fn is_unit_vector(v: &Vector) -> bool { vector_magnitude(v) == 1.0 }
pub fn is_vector_orthogonal(v1: &Vector, v2: &Vector) -> bool { dot_product(v1, v2) == 0.0 }
pub fn is_matrix_orthogonal(m1: &Matrix, _m2: &Matrix) -> bool {
    let inv = inverse_matrix(m1);
    let t = transpose_matrix(m1);
    is_matrix_equal(&inv, &t)
}
pub fn scalar_triple_product(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    assert!(v1.cols == 3 && v2.cols == 3 && v3.cols == 3);
    dot_product(v1, &cross_product(v2, v3))
}
// --- Geometric operations ---
pub fn reflect_axis_2d(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix(m) && m.cols == 2);
    let mut n = zero_matrix(2, 2);
    set_matrix_element(&mut n, 0, 0, if axis != 0 { 1.0 } else { -1.0 });
    set_matrix_element(&mut n, 1, 1, if axis != 0 { -1.0 } else { 1.0 });
    multiply_matrices(m, &n)
}
pub fn reflect_axis_3d(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix(m) && m.cols == 3 && axis >= 0 && axis <= 2);
    let mut m2 = m.clone();
    let diag = match axis {
        0 => vec![1.0, 1.0, -1.0],  // XY
        1 => vec![1.0, -1.0, 1.0],  // XZ
        _ => vec![-1.0, 1.0, 1.0],  // YZ
    };
    let v = new_vector(&diag, 3);
    set_main_diagonal(&mut m2, &v);
    let n = null_matrix(3, 3);
    multiply_matrices(&m2, &n)
}
pub fn orth_proj_2d(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix(m) && m.cols == 2);
    let mut n = zero_matrix(2, 2);
    set_matrix_element(&mut n, axis as usize, axis as usize, 1.0);
    multiply_matrices(m, &n)
}
pub fn orth_proj_3d(m: &Matrix, axis: i32) -> Matrix {
    assert!(is_square_matrix(m) && m.cols == 3);
    let mut n = zero_matrix(3, 3);
    match axis {
        0 => { set_matrix_element(&mut n, 0, 0, 1.0); set_matrix_element(&mut n, 1, 1, 1.0); }
        1 => { set_matrix_element(&mut n, 0, 0, 1.0); set_matrix_element(&mut n, 2, 2, 1.0); }
        2 => { set_matrix_element(&mut n, 1, 1, 1.0); set_matrix_element(&mut n, 2, 2, 1.0); }
        _ => {}
    }
    multiply_matrices(m, &n)
}
pub fn rotate_2d(m: &Matrix, theta: f64) -> Matrix {
    assert!(is_square_matrix(m) && m.cols == 2);
    let mut n = zero_matrix(2, 2);
    set_matrix_element(&mut n, 0, 0, theta.cos());
    set_matrix_element(&mut n, 0, 1, -theta.sin());
    set_matrix_element(&mut n, 1, 0, theta.sin());
    set_matrix_element(&mut n, 1, 1, theta.cos());
    multiply_matrices(m, &n)
}
pub fn scale_n_space(m: &Matrix, k: f64) -> Matrix {
    assert!(is_square_matrix(m));
    let mut n = zero_matrix(m.cols, m.cols);
    let mut v = zero_vector(m.cols);
    fill_vector(&mut v, k);
    set_main_diagonal(&mut n, &v);
    multiply_matrices(m, &n)
}
pub fn shear_2d(m: &Matrix, k: f64, axis: i32) -> Matrix {
    assert!(is_square_matrix(m) && m.cols == 2);
    let mut n = zero_matrix(2, 2);
    set_matrix_element(&mut n, 0, 0, 1.0);
    set_matrix_element(&mut n, 0, 1, if axis != 0 { k } else { 0.0 });
    set_matrix_element(&mut n, 1, 0, if axis != 0 { 0.0 } else { k });
    set_matrix_element(&mut n, 1, 1, 1.0);
    multiply_matrices(m, &n)
}
/// Returns the determinant of a matrix.
pub fn determinant(m: &Matrix) -> f64 {
    assert!(is_square_matrix(m));
    match m.rows {
        1 => m.data[0],
        2 => m.data[0] * m.data[3] - m.data[1] * m.data[2],
        3 => {
            m.data[0] * (m.data[4] * m.data[8] - m.data[5] * m.data[7])
                - m.data[1] * (m.data[3] * m.data[8] - m.data[5] * m.data[6])
                + m.data[2] * (m.data[3] * m.data[7] - m.data[4] * m.data[6])
        }
        _ => {
            if is_triangular_matrix(m) {
                return diagonal_product(m);
            }
            let (l, u, _p, swaps) = lu_decomposition(m);
            (-1.0f64).powi(swaps - 1) * determinant(&l) * determinant(&u)
        }
    }
}
/// Performs LU decomposition. Returns (L, U, P, swaps).
pub fn lu_decomposition(m: &Matrix) -> (Matrix, Matrix, Matrix, i32) {
    assert!(is_square_matrix(m));
    let n = m.cols;
    let mut l = zero_matrix(n, n);
    let mut u = zero_matrix(n, n);
    let (p, swaps) = pivot_matrix(m);
    let m2 = multiply_matrices(&p, m);
    for j in 0..n {
        l.data[j * n + j] = 1.0;
        for i in 0..=j {
            let sum_u: f64 = (0..i).map(|k| u.data[k * n + j] * l.data[i * n + k]).sum();
            u.data[i * n + j] = m2.data[i * n + j] - sum_u;
        }
        for i in j..n {
            let sum_l: f64 = (0..j).map(|k| u.data[k * n + j] * l.data[i * n + k]).sum();
            l.data[i * n + j] = (m2.data[i * n + j] - sum_l) / u.data[j * n + j];
        }
    }
    (l, u, p, swaps)
}
/// Returns the submatrix of m excluding row i and column j.
pub fn sub_matrix(m: &Matrix, i: usize, j: usize) -> Matrix {
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
/// Returns the minor of m at (i,j).
pub fn element_minor(m: &Matrix, i: usize, j: usize) -> f64 {
    determinant(&sub_matrix(m, i, j))
}
/// Returns the matrix of minors of m.
pub fn matrix_minor(m: &Matrix) -> Matrix {
    let mut mm = null_matrix(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            mm.data[i * m.cols + j] = element_minor(m, i, j);
        }
    }
    mm
}
/// Returns the cofactor of element (i,j) in m.
pub fn element_cofactor(m: &Matrix, i: usize, j: usize) -> f64 {
    (-1.0f64).powi((i + 1 + j + 1) as i32) * element_minor(m, i, j)
}
/// Returns the matrix of cofactors of m.
pub fn matrix_cofactor(m: &Matrix) -> Matrix {
    let mut cfm = null_matrix(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            cfm.data[i * m.cols + j] = element_cofactor(m, i, j);
        }
    }
    cfm
}
/// Returns a sign matrix of the given dimensions.
pub fn sign_matrix(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    let mut sm = null_matrix(rows, cols);
    fill_matrix(&mut sm, 1.0);
    for i in 0..rows {
        for j in 0..cols {
            sm.data[i * cols + j] = if (i * cols + j + 1) % 2 != 0 { 1.0 } else { -1.0 };
        }
    }
    sm
}
/// Returns the adjugate matrix of m.
pub fn adjugate_matrix(m: &Matrix) -> Matrix {
    let mm = matrix_minor(m);
    let sign = sign_matrix(m.rows, m.cols);
    multiply_matrices(&mm, &sign)
}
/// Returns the inverse of m.
pub fn inverse_matrix(m: &Matrix) -> Matrix {
    assert!(is_invertible(m));
    let adj = adjugate_matrix(m);
    scale_matrix(&adj, 1.0 / determinant(m))
}
/// Returns the pivot matrix of m along with the number of swaps.
pub fn pivot_matrix(m: &Matrix) -> (Matrix, i32) {
    assert!(is_square_matrix(m));
    let n = m.cols;
    let mut pivot = identity_matrix(n);
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
            let v = get_row_vector(&pivot, i);
            let w = get_row_vector(&pivot, row);
            set_row_vector(&mut pivot, i, &w);
            set_row_vector(&mut pivot, row, &v);
            swaps += 1;
        }
    }
    (pivot, swaps)
}
