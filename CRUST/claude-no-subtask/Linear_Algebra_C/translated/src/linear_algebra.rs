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
// --- Helper assertion functions ---
pub fn assert_matrix(_m: &Matrix) -> bool {
    true
}
pub fn assert_vector(_v: &Vector) -> bool {
    true
}
// --- Creation functions ---
pub fn new_matrix(d: &[f64], rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    let mut m = null_matrix(rows, cols);
    let mut idx = 0;
    for i in 0..rows {
        for j in 0..cols {
            m.data[i * cols + j] = d[idx];
            idx += 1;
        }
    }
    m
}
pub fn new_vector(d: &[f64], cols: usize) -> Vector {
    assert!(cols > 0);
    let mut v = null_vector(cols);
    for i in 0..cols {
        v.data[i] = d[i];
    }
    v
}
pub fn null_matrix(rows: usize, cols: usize) -> Matrix {
    assert!(rows > 0 && cols > 0);
    Matrix {
        rows,
        cols,
        data: vec![0.0; rows * cols],
    }
}
pub fn null_vector(cols: usize) -> Vector {
    assert!(cols > 0);
    Vector {
        cols,
        data: vec![0.0; cols],
    }
}
pub fn zero_matrix(rows: usize, cols: usize) -> Matrix {
    let mut m = null_matrix(rows, cols);
    fill_matrix(&mut m, 0.0);
    m
}
pub fn zero_vector(cols: usize) -> Vector {
    let mut v = null_vector(cols);
    fill_vector(&mut v, 0.0);
    v
}
// --- Fill functions ---
pub fn fill_matrix(m: &mut Matrix, n: f64) {
    for i in 0..m.rows {
        for j in 0..m.cols {
            let cols = m.cols;
            m.data[i * cols + j] = n;
        }
    }
}
pub fn fill_vector(v: &mut Vector, n: f64) {
    for i in 0..v.cols {
        v.data[i] = n;
    }
}
/// Returns an identity matrix of size n.
pub fn identity_matrix(n: usize) -> Matrix {
    let mut m = zero_matrix(n, n);
    for i in 0..n {
        for j in 0..n {
            if i == j {
                m.data[i * n + j] = 1.0;
            }
        }
    }
    m
}
/// "Releases" a matrix (stub; memory is managed automatically in Rust).
pub fn delete_matrix(_m: Matrix) {
    // no-op in Rust; Vec is dropped automatically
}
/// "Releases" a vector.
pub fn delete_vector(_v: Vector) {
    // no-op in Rust; Vec is dropped automatically
}
/// Returns a copy of the given matrix.
pub fn copy_matrix(m: &Matrix) -> Matrix {
    let mut c = zero_matrix(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            c.data[i * m.cols + j] = m.data[i * m.cols + j];
        }
    }
    c
}
/// Returns a copy of the given vector.
pub fn copy_vector(v: &Vector) -> Vector {
    let mut c = zero_vector(v.cols);
    for i in 0..v.cols {
        c.data[i] = v.data[i];
    }
    c
}
/// Flattens the given matrix into a vector.
pub fn flatten_matrix(m: &Matrix) -> Vector {
    let mut flat = null_vector(m.rows * m.cols);
    let mut idx = 0;
    for i in 0..m.rows {
        for j in 0..m.cols {
            flat.data[idx] = m.data[i * m.cols + j];
            idx += 1;
        }
    }
    flat
}
// --- Size functions ---
pub fn matrix_size(m: &Matrix) -> usize {
    m.rows * m.cols
}
pub fn vector_size(v: &Vector) -> usize {
    v.cols
}
pub fn matrix_size_bytes(m: &Matrix) -> usize {
    std::mem::size_of::<f64>() * matrix_size(m)
}
pub fn vector_size_bytes(v: &Vector) -> usize {
    std::mem::size_of::<f64>() * vector_size(v)
}
// --- Element accessor/mutator functions ---
pub fn set_matrix_element(m: &mut Matrix, i: usize, j: usize, s: f64) {
    assert!(i < m.rows && j < m.cols);
    let cols = m.cols;
    m.data[i * cols + j] = s;
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
    let cols = m.cols;
    for j in 0..v.cols {
        m.data[i * cols + j] = v.data[j];
    }
}
pub fn get_row_vector(m: &Matrix, i: usize) -> Vector {
    assert!(i < m.rows);
    let mut row = vec![0.0; m.cols];
    for j in 0..m.cols {
        row[j] = m.data[i * m.cols + j];
    }
    new_vector(&row, m.cols)
}
pub fn set_col_vector(m: &mut Matrix, j: usize, v: &Vector) {
    assert!(j < m.cols);
    let cols = m.cols;
    for i in 0..v.cols {
        m.data[i * cols + j] = v.data[i];
    }
}
pub fn get_col_vector(m: &Matrix, j: usize) -> Vector {
    assert!(j < m.cols);
    let mut col = vec![0.0; m.rows];
    for i in 0..m.rows {
        col[i] = m.data[i * m.cols + j];
    }
    new_vector(&col, m.rows)
}
// --- Diagonal operations ---
pub fn get_main_diagonal(m: &Matrix) -> Vector {
    assert!(is_square_matrix(m));
    let mut diag = vec![0.0; m.rows];
    for x in 0..m.rows {
        diag[x] = m.data[x * m.cols + x];
    }
    new_vector(&diag, m.rows)
}
pub fn set_main_diagonal(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix(m) && m.rows == m.cols && m.cols == v.cols);
    let cols = m.cols;
    for x in 0..v.cols {
        m.data[x * cols + x] = v.data[x];
    }
}
pub fn get_anti_diagonal(m: &Matrix) -> Vector {
    assert!(is_square_matrix(m));
    let rows = m.rows;
    let cols = m.cols;
    let mut diag = vec![0.0; rows];
    let mut x = 0;
    'outer: for i in (0..rows).rev() {
        for j in (0..cols).rev() {
            if i + j == rows - 1 {
                diag[x] = m.data[i * cols + j];
                x += 1;
                if x == rows {
                    break 'outer;
                }
            }
        }
    }
    new_vector(&diag, rows)
}
pub fn set_anti_diagonal(m: &mut Matrix, v: &Vector) {
    assert!(is_square_matrix(m) && m.rows == m.cols && m.cols == v.cols);
    let rows = m.rows;
    let cols = m.cols;
    let mut idx = 0;
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
pub fn diagonal_product(m: &Matrix) -> f64 {
    let diagonal = get_main_diagonal(m);
    let mut product = 1.0;
    for i in 0..diagonal.cols {
        product *= diagonal.data[i];
    }
    product
}
// --- "Pretty" print functions ---
pub fn print_matrix(m: &Matrix, include_indices: bool) {
    for i in 0..m.rows {
        for j in 0..m.cols {
            if include_indices {
                print!("[{},{}] -> ", i, j);
            }
            print!("{:8.2} ", m.data[i * m.cols + j]);
        }
        if i < m.rows {
            println!();
        }
    }
}
pub fn print_vector(v: &Vector, include_indices: bool) {
    for i in 0..v.cols {
        if include_indices {
            print!("[{}] -> ", i);
        }
        print!("{:16.8} ", v.data[i]);
    }
}
// --- Comparison functions ---
pub fn is_matrix_equal(m: &Matrix, n: &Matrix) -> bool {
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
pub fn is_vector_equal(v: &Vector, w: &Vector) -> bool {
    if v.cols != w.cols {
        return false;
    }
    for i in 0..v.cols {
        if v.data[i] != w.data[i] {
            return false;
        }
    }
    true
}
pub fn has_same_dimensions(m: &Matrix, n: &Matrix) -> bool {
    m.rows == n.rows && m.cols == n.cols
}
// --- Property testing functions ---
pub fn is_zero_matrix(m: &Matrix) -> bool {
    for i in 0..m.rows {
        for j in 0..m.cols {
            if m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}
pub fn is_identity_matrix(m: &Matrix) -> bool {
    if !is_square_matrix(m) {
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
pub fn is_square_matrix(m: &Matrix) -> bool {
    m.rows == m.cols
}
pub fn is_invertible(m: &Matrix) -> bool {
    is_square_matrix(m) && determinant(m) != 0.0
}
pub fn is_diagonal_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    for i in 0..m.rows {
        for j in 0..m.cols {
            if i != j && m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}
pub fn is_triangular_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    crate::utils::exclusive_or(is_up_tri_matrix(m), is_lo_tri_matrix(m))
}
pub fn is_up_tri_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    for i in 0..m.rows {
        for j in 0..i {
            if m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}
pub fn is_lo_tri_matrix(m: &Matrix) -> bool {
    assert!(is_square_matrix(m));
    for i in 0..m.rows {
        for j in (i + 1)..m.cols {
            if m.data[i * m.cols + j] != 0.0 {
                return false;
            }
        }
    }
    true
}
pub fn is_matrix_symmetric(m: &Matrix) -> bool {
    if m.rows != m.cols {
        return false;
    }
    let t = transpose_matrix(m);
    is_matrix_equal(m, &t)
}
pub fn has_zero_row(m: &Matrix) -> bool {
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
pub fn has_zero_col(m: &Matrix) -> bool {
    // Note: matches the (somewhat odd) C indexing exactly.
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
    let mut trace = 0.0;
    for i in 0..m.rows {
        trace += m.data[i * m.cols + i];
    }
    trace
}
pub fn add_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    assert!(has_same_dimensions(m1, m2));
    let mut sum = null_matrix(m1.rows, m1.cols);
    let mut idx = 0;
    for _i in 0..m1.rows {
        for _j in 0..m1.cols {
            sum.data[idx] = m1.data[idx] + m2.data[idx];
            idx += 1;
        }
    }
    sum
}
pub fn add_vectors(v1: &Vector, v2: &Vector) -> Vector {
    assert!(v1.cols == v2.cols);
    let mut sum = null_vector(v1.cols);
    for i in 0..v1.cols {
        sum.data[i] = v1.data[i] + v2.data[i];
    }
    sum
}
pub fn pow_matrix(m: &Matrix, k: f64) -> Matrix {
    let mut p = null_matrix(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            p.data[i * m.cols + j] = m.data[i * m.cols + j].powf(k);
        }
    }
    p
}
pub fn pow_vector(v: &Vector, k: f64) -> Vector {
    let mut p = null_vector(v.cols);
    for i in 0..v.cols {
        p.data[i] = v.data[i].powf(k);
    }
    p
}
pub fn multiply_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    // Mirrors the (square-only) C implementation.
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
    let mut scaled = null_matrix(m.rows, m.cols);
    for i in 0..m.rows {
        for j in 0..m.cols {
            scaled.data[i * m.cols + j] = m.data[i * m.cols + j] * s;
        }
    }
    scaled
}
pub fn dot_product(v: &Vector, w: &Vector) -> f64 {
    assert!(v.cols == w.cols);
    let mut dp = 0.0;
    for i in 0..v.cols {
        dp += v.data[i] * w.data[i];
    }
    dp
}
pub fn cross_product(v: &Vector, w: &Vector) -> Vector {
    assert!(v.cols == 3 && w.cols == 3);
    let mut c = null_vector(3);
    c.data[0] = (v.data[1] * w.data[2]) - (v.data[2] * w.data[1]);
    c.data[1] = (v.data[0] * w.data[2]) - (v.data[2] * w.data[0]);
    c.data[2] = (v.data[0] * w.data[1]) - (v.data[1] * w.data[0]);
    c
}
pub fn vector_magnitude(v: &Vector) -> f64 {
    let mut sum = 0.0;
    for i in 0..v.cols {
        sum += v.data[i] * v.data[i];
    }
    sum.sqrt()
}
pub fn vector_distance(v: &Vector, w: &Vector) -> f64 {
    assert!(v.cols == w.cols);
    let mut d = 0.0;
    for i in 0..v.cols {
        d += (w.data[i] - v.data[i]) * (w.data[i] - v.data[i]);
    }
    d.sqrt()
}
pub fn scale_vector(v: &Vector, s: f64) -> Vector {
    let mut scaled = null_vector(v.cols);
    for i in 0..v.cols {
        scaled.data[i] = v.data[i] * s;
    }
    scaled
}
pub fn is_unit_vector(v: &Vector) -> bool {
    vector_magnitude(v) == 1.0
}
pub fn is_vector_orthogonal(v1: &Vector, v2: &Vector) -> bool {
    dot_product(v1, v2) == 0.0
}
pub fn is_matrix_orthogonal(m1: &Matrix, m2: &Matrix) -> bool {
    // Matrix m1 is orthogonal to m2 if inv(m1) == transpose(m2)
    let inv = inverse_matrix(m1);
    let t = transpose_matrix(m2);
    is_matrix_equal(&inv, &t)
}
pub fn scalar_triple_product(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    assert!(v1.cols == 3 && v2.cols == 3 && v3.cols == 3);
    let cp = cross_product(v2, v3);
    dot_product(v1, &cp)
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
    assert!(is_square_matrix(m) && m.cols == 3 && (0..=2).contains(&axis));
    // The C version constructs `n` but never sets its data (it sets the diagonal of `m`
    // erroneously), then multiplies m by uninitialised n. We replicate the *intent*:
    // build a reflection matrix and multiply.
    let mut n = zero_matrix(3, 3);
    let data: [f64; 3] = match axis {
        0 => [1.0, 1.0, -1.0],   // XY plane
        1 => [1.0, -1.0, 1.0],   // XZ plane
        _ => [-1.0, 1.0, 1.0],   // YZ plane
    };
    let v = new_vector(&data, 3);
    set_main_diagonal(&mut n, &v);
    multiply_matrices(m, &n)
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
        0 => {
            set_matrix_element(&mut n, 0, 0, 1.0);
            set_matrix_element(&mut n, 1, 1, 1.0);
        }
        1 => {
            set_matrix_element(&mut n, 0, 0, 1.0);
            set_matrix_element(&mut n, 2, 2, 1.0);
        }
        2 => {
            set_matrix_element(&mut n, 1, 1, 1.0);
            set_matrix_element(&mut n, 2, 2, 1.0);
        }
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
        1 => return m.data[0],
        2 => return (m.data[0] * m.data[3]) - (m.data[1] * m.data[2]),
        3 => {
            return m.data[0] * ((m.data[4] * m.data[8]) - (m.data[5] * m.data[7]))
                - m.data[1] * ((m.data[3] * m.data[8]) - (m.data[5] * m.data[6]))
                + m.data[2] * ((m.data[3] * m.data[7]) - (m.data[4] * m.data[6]));
        }
        _ => {}
    }
    if is_triangular_matrix(m) {
        return diagonal_product(m);
    }
    let (l, u, _p, swaps) = lu_decomposition(m);
    // det(P) = (-1)^swaps; mirror C's exponent of (swaps - 1).
    (-1.0_f64).powi(swaps - 1) * determinant(&l) * determinant(&u)
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
/// Returns the submatrix of m excluding row i and column j.
pub fn sub_matrix(m: &Matrix, i: usize, j: usize) -> Matrix {
    assert!(i < m.rows && j < m.cols);
    let mut sm = null_matrix(m.rows - 1, m.cols - 1);
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
/// Returns the minor of m at (i,j).
pub fn element_minor(m: &Matrix, i: usize, j: usize) -> f64 {
    let sm = sub_matrix(m, i, j);
    determinant(&sm)
}
/// Returns the matrix of minors of m.
pub fn matrix_minor(m: &Matrix) -> Matrix {
    let mut mm = null_matrix(m.rows, m.cols);
    for i in 0..mm.rows {
        for j in 0..mm.cols {
            let cols = mm.cols;
            mm.data[i * cols + j] = element_minor(m, i, j);
        }
    }
    mm
}
/// Returns the cofactor of element (i,j) in m.
pub fn element_cofactor(m: &Matrix, i: usize, j: usize) -> f64 {
    let sign_exp = ((i + 1) + (j + 1)) as i32;
    (-1.0_f64).powi(sign_exp) * element_minor(m, i, j)
}
/// Returns the matrix of cofactors of m.
pub fn matrix_cofactor(m: &Matrix) -> Matrix {
    let mut cfm = null_matrix(m.rows, m.cols);
    for i in 0..cfm.rows {
        for j in 0..cfm.cols {
            let cols = cfm.cols;
            cfm.data[i * cols + j] = element_cofactor(m, i, j);
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
            let idx = i * cols + j;
            sm.data[idx] = if (idx + 1) % 2 != 0 { 1.0 } else { -1.0 };
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
    let det = determinant(m);
    scale_matrix(&adj, 1.0 / det)
}
/// Returns the pivot matrix of m along with the number of swaps.
pub fn pivot_matrix(m: &Matrix) -> (Matrix, i32) {
    assert!(is_square_matrix(m));
    let n = m.cols;
    let mut pivot = identity_matrix(n);
    // Replicating the C bug: `swaps++` in C increments the local pointer rather
    // than the value, so the swap counter never updates. We mirror that by
    // returning 0 to keep the determinant calculation consistent with C.
    let swaps: i32 = 0;
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
        }
    }
    (pivot, swaps)
}
