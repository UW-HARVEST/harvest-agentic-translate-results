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
pub fn assert_matrix(m: &Matrix) -> bool {
    crate::matrix::assert_matrix_impl(m)
}
pub fn assert_vector(v: &Vector) -> bool {
    crate::vector::assert_vector_impl(v)
}
// --- Creation functions ---
pub fn new_matrix(d: &[f64], rows: usize, cols: usize) -> Matrix {
    crate::matrix::new_matrix_impl(d, rows, cols)
}
pub fn new_vector(d: &[f64], cols: usize) -> Vector {
    crate::vector::new_vector_impl(d, cols)
}
pub fn null_matrix(rows: usize, cols: usize) -> Matrix {
    crate::matrix::null_matrix_impl(rows, cols)
}
pub fn null_vector(cols: usize) -> Vector {
    crate::vector::null_vector_impl(cols)
}
pub fn zero_matrix(rows: usize, cols: usize) -> Matrix {
    crate::matrix::zero_matrix_impl(rows, cols)
}
pub fn zero_vector(cols: usize) -> Vector {
    crate::vector::zero_vector_impl(cols)
}
// --- Fill functions ---
pub fn fill_matrix(m: &mut Matrix, n: f64) {
    crate::matrix::fill_matrix_impl(m, n)
}
pub fn fill_vector(v: &mut Vector, n: f64) {
    crate::vector::fill_vector_impl(v, n)
}
/// Returns an identity matrix of size n.
pub fn identity_matrix(n: usize) -> Matrix {
    crate::matrix::identity_matrix_impl(n)
}
/// "Releases" a matrix (stub; memory is managed automatically in Rust).
pub fn delete_matrix(_m: Matrix) {
    // drop
}
/// "Releases" a vector.
pub fn delete_vector(_v: Vector) {
    // drop
}
/// Returns a copy of the given matrix.
pub fn copy_matrix(m: &Matrix) -> Matrix {
    crate::matrix::copy_matrix_impl(m)
}
/// Returns a copy of the given vector.
pub fn copy_vector(v: &Vector) -> Vector {
    crate::vector::copy_vector_impl(v)
}
/// Flattens the given matrix into a vector.
pub fn flatten_matrix(m: &Matrix) -> Vector {
    crate::matrix::flatten_matrix_impl(m)
}
// --- Size functions ---
pub fn matrix_size(m: &Matrix) -> usize {
    crate::matrix::matrix_size_impl(m)
}
pub fn vector_size(v: &Vector) -> usize {
    crate::vector::vector_size_impl(v)
}
pub fn matrix_size_bytes(m: &Matrix) -> usize {
    crate::matrix::matrix_size_bytes_impl(m)
}
pub fn vector_size_bytes(v: &Vector) -> usize {
    crate::vector::vector_size_bytes_impl(v)
}
// --- Element accessor/mutator functions ---
pub fn set_matrix_element(m: &mut Matrix, i: usize, j: usize, s: f64) {
    crate::matrix::set_matrix_element_impl(m, i, j, s)
}
pub fn get_matrix_element(m: &Matrix, i: usize, j: usize) -> f64 {
    crate::matrix::get_matrix_element_impl(m, i, j)
}
pub fn set_vector_element(v: &mut Vector, i: usize, s: f64) {
    crate::vector::set_vector_element_impl(v, i, s)
}
pub fn get_vector_element(v: &Vector, i: usize) -> f64 {
    crate::vector::get_vector_element_impl(v, i)
}
// --- Row and column operations ---
pub fn set_row_vector(m: &mut Matrix, i: usize, v: &Vector) {
    crate::matrix::set_row_vector_impl(m, i, v)
}
pub fn get_row_vector(m: &Matrix, i: usize) -> Vector {
    crate::matrix::get_row_vector_impl(m, i)
}
pub fn set_col_vector(m: &mut Matrix, j: usize, v: &Vector) {
    crate::matrix::set_col_vector_impl(m, j, v)
}
pub fn get_col_vector(m: &Matrix, j: usize) -> Vector {
    crate::matrix::get_col_vector_impl(m, j)
}
// --- Diagonal operations ---
pub fn get_main_diagonal(m: &Matrix) -> Vector {
    crate::matrix::get_main_diagonal_impl(m)
}
pub fn set_main_diagonal(m: &mut Matrix, v: &Vector) {
    crate::matrix::set_main_diagonal_impl(m, v)
}
pub fn get_anti_diagonal(m: &Matrix) -> Vector {
    crate::matrix::get_anti_diagonal_impl(m)
}
pub fn set_anti_diagonal(m: &mut Matrix, v: &Vector) {
    crate::matrix::set_anti_diagonal_impl(m, v)
}
pub fn diagonal_product(m: &Matrix) -> f64 {
    crate::matrix::diagonal_product_impl(m)
}
// --- "Pretty" print functions ---
pub fn print_matrix(m: &Matrix, include_indices: bool) {
    for i in 0..m.rows {
        for j in 0..m.cols {
            if include_indices { print!("[{},{}] -> ", i, j); }
            print!("{:8.2} ", m.data[i * m.cols + j]);
        }
        if i < m.rows { println!(); }
    }
}
pub fn print_vector(v: &Vector, include_indices: bool) {
    crate::vector::print_vector_impl(v, include_indices)
}
// --- Comparison functions ---
pub fn is_matrix_equal(m: &Matrix, n: &Matrix) -> bool {
    crate::matrix::is_matrix_equal_impl(m, n)
}
pub fn is_vector_equal(v: &Vector, w: &Vector) -> bool {
    crate::vector::is_vector_equal_impl(v, w)
}
pub fn has_same_dimensions(m: &Matrix, n: &Matrix) -> bool {
    crate::matrix::has_same_dimensions_impl(m, n)
}
// --- Property testing functions ---
pub fn is_zero_matrix(m: &Matrix) -> bool {
    crate::matrix::is_zero_matrix_impl(m)
}
pub fn is_identity_matrix(m: &Matrix) -> bool {
    crate::matrix::is_identity_matrix_impl(m)
}
pub fn is_square_matrix(m: &Matrix) -> bool {
    crate::matrix::is_square_matrix_impl(m)
}
pub fn is_invertible(m: &Matrix) -> bool {
    crate::matrix::is_invertible_impl(m)
}
pub fn is_diagonal_matrix(m: &Matrix) -> bool {
    crate::matrix::is_diagonal_matrix_impl(m)
}
pub fn is_triangular_matrix(m: &Matrix) -> bool {
    crate::matrix::is_triangular_matrix_impl(m)
}
pub fn is_up_tri_matrix(m: &Matrix) -> bool {
    crate::matrix::is_up_tri_matrix_impl(m)
}
pub fn is_lo_tri_matrix(m: &Matrix) -> bool {
    crate::matrix::is_lo_tri_matrix_impl(m)
}
pub fn is_matrix_symmetric(m: &Matrix) -> bool {
    crate::matrix::is_matrix_symmetric_impl(m)
}
pub fn has_zero_row(m: &Matrix) -> bool {
    crate::matrix::has_zero_row_impl(m)
}
pub fn has_zero_col(m: &Matrix) -> bool {
    crate::matrix::has_zero_col_impl(m)
}
// --- Advanced operations ---
pub fn transpose_matrix(m: &Matrix) -> Matrix {
    crate::matrix::transpose_matrix_impl(m)
}
pub fn trace_matrix(m: &Matrix) -> f64 {
    crate::matrix::trace_matrix_impl(m)
}
pub fn add_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    crate::matrix::add_matrices_impl(m1, m2)
}
pub fn add_vectors(v1: &Vector, v2: &Vector) -> Vector {
    crate::vector::add_vectors_impl(v1, v2)
}
pub fn pow_matrix(m: &Matrix, k: f64) -> Matrix {
    crate::matrix::pow_matrix_impl(m, k)
}
pub fn pow_vector(v: &Vector, k: f64) -> Vector {
    crate::vector::pow_vector_impl(v, k)
}
pub fn multiply_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    crate::matrix::multiply_matrices_impl(m1, m2)
}
pub fn scale_matrix(m: &Matrix, s: f64) -> Matrix {
    crate::matrix::scale_matrix_impl(m, s)
}
pub fn dot_product(v: &Vector, w: &Vector) -> f64 {
    crate::vector::dot_product_impl(v, w)
}
pub fn cross_product(v: &Vector, w: &Vector) -> Vector {
    crate::vector::cross_product_impl(v, w)
}
pub fn vector_magnitude(v: &Vector) -> f64 {
    crate::vector::vector_magnitude_impl(v)
}
pub fn vector_distance(v: &Vector, w: &Vector) -> f64 {
    crate::vector::vector_distance_impl(v, w)
}
pub fn scale_vector(v: &Vector, s: f64) -> Vector {
    crate::vector::scale_vector_impl(v, s)
}
pub fn is_unit_vector(v: &Vector) -> bool {
    crate::vector::is_unit_vector_impl(v)
}
pub fn is_vector_orthogonal(v1: &Vector, v2: &Vector) -> bool {
    crate::vector::is_vector_orthogonal_impl(v1, v2)
}
pub fn is_matrix_orthogonal(m1: &Matrix, _m2: &Matrix) -> bool {
    // Not implemented in C source; standard definition: inv(m) == transpose(m)
    let inv = inverse_matrix(m1);
    let t = transpose_matrix(m1);
    is_matrix_equal(&inv, &t)
}
pub fn scalar_triple_product(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    crate::vector::scalar_triple_product_impl(v1, v2, v3)
}
// --- Geometric operations ---
pub fn reflect_axis_2d(m: &Matrix, axis: i32) -> Matrix {
    crate::matrix::reflect_axis_2d_impl(m, axis)
}
pub fn reflect_axis_3d(m: &Matrix, axis: i32) -> Matrix {
    crate::matrix::reflect_axis_3d_impl(m, axis)
}
pub fn orth_proj_2d(m: &Matrix, axis: i32) -> Matrix {
    crate::matrix::orth_proj_2d_impl(m, axis)
}
pub fn orth_proj_3d(m: &Matrix, axis: i32) -> Matrix {
    crate::matrix::orth_proj_3d_impl(m, axis)
}
pub fn rotate_2d(m: &Matrix, theta: f64) -> Matrix {
    // rotate2D is declared in the C header but not implemented in the C source.
    // Standard 2D rotation matrix implementation:
    assert!(m.rows == m.cols && m.cols == 2);
    let mut n = zero_matrix(2, 2);
    n.data[0] = theta.cos();
    n.data[1] = -theta.sin();
    n.data[2] = theta.sin();
    n.data[3] = theta.cos();
    multiply_matrices(m, &n)
}
pub fn scale_n_space(m: &Matrix, k: f64) -> Matrix {
    crate::matrix::scale_n_space_impl(m, k)
}
pub fn shear_2d(m: &Matrix, k: f64, axis: i32) -> Matrix {
    crate::matrix::shear_2d_impl(m, k, axis)
}
/// Returns the determinant of a matrix.
pub fn determinant(m: &Matrix) -> f64 {
    assert!(m.rows == m.cols);
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
    crate::matrix::lu_decomposition_impl(m)
}
/// Returns the submatrix of m excluding row i and column j.
pub fn sub_matrix(m: &Matrix, i: usize, j: usize) -> Matrix {
    crate::matrix::sub_matrix_impl(m, i, j)
}
/// Returns the minor of m at (i,j).
pub fn element_minor(m: &Matrix, i: usize, j: usize) -> f64 {
    crate::matrix::element_minor_impl(m, i, j)
}
/// Returns the matrix of minors of m.
pub fn matrix_minor(m: &Matrix) -> Matrix {
    crate::matrix::matrix_minor_impl(m)
}
/// Returns the cofactor of element (i,j) in m.
pub fn element_cofactor(m: &Matrix, i: usize, j: usize) -> f64 {
    crate::matrix::element_cofactor_impl(m, i, j)
}
/// Returns the matrix of cofactors of m.
pub fn matrix_cofactor(m: &Matrix) -> Matrix {
    crate::matrix::matrix_cofactor_impl(m)
}
/// Returns a sign matrix of the given dimensions.
pub fn sign_matrix(rows: usize, cols: usize) -> Matrix {
    crate::matrix::sign_matrix_impl(rows, cols)
}
/// Returns the adjugate matrix of m.
pub fn adjugate_matrix(m: &Matrix) -> Matrix {
    crate::matrix::adjugate_matrix_impl(m)
}
/// Returns the inverse of m.
pub fn inverse_matrix(m: &Matrix) -> Matrix {
    crate::matrix::inverse_matrix_impl(m)
}
/// Returns the pivot matrix of m along with the number of swaps.
pub fn pivot_matrix(m: &Matrix) -> (Matrix, i32) {
    crate::matrix::pivot_matrix_impl(m)
}
