use crate::matrix::{
    add_matrices_impl, adjugate_matrix_impl, assert_matrix_impl, copy_matrix_impl,
    diagonal_product_impl, determinant_impl, element_cofactor_impl, element_minor_impl,
    fill_matrix_impl, flatten_matrix_impl, get_anti_diagonal_impl, get_col_vector_impl,
    get_main_diagonal_impl, get_matrix_element_impl, get_row_vector_impl,
    has_same_dimensions_impl, has_zero_col_impl, has_zero_row_impl, identity_matrix_impl,
    inverse_matrix_impl, is_diagonal_matrix_impl, is_identity_matrix_impl, is_invertible_impl,
    is_lo_tri_matrix_impl, is_matrix_equal_impl, is_matrix_orthogonal_impl,
    is_matrix_symmetric_impl, is_square_matrix_impl, is_triangular_matrix_impl,
    is_up_tri_matrix_impl, is_zero_matrix_impl, lu_decomposition_impl, matrix_cofactor_impl,
    matrix_minor_impl, matrix_size_bytes_impl, matrix_size_impl, multiply_matrices_impl,
    new_matrix_impl, null_matrix_impl, orth_proj_2d_impl, orth_proj_3d_impl, pivot_matrix_impl,
    pow_matrix_impl, print_matrix_impl, reflect_axis_2d_impl, reflect_axis_3d_impl,
    rotate_2d_impl, scale_matrix_impl, scale_n_space_impl, set_anti_diagonal_impl,
    set_col_vector_impl, set_main_diagonal_impl, set_matrix_element_impl, set_row_vector_impl,
    shear_2d_impl, sign_matrix_impl, sub_matrix_impl, trace_matrix_impl, transpose_matrix_impl,
    zero_matrix_impl,
};
use crate::vector::{
    add_vectors_impl, assert_vector_impl, copy_vector_impl, cross_product_impl, delete_vector_impl,
    dot_product_impl, fill_vector_impl, get_vector_element_impl, is_unit_vector_impl,
    is_vector_equal_impl, is_vector_orthogonal_impl, new_vector_impl, null_vector_impl,
    pow_vector_impl, print_vector_impl, scale_vector_impl, scalar_triple_product_impl,
    set_vector_element_impl, vector_distance_impl, vector_magnitude_impl, vector_size_bytes_impl,
    vector_size_impl, zero_vector_impl,
};

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
    assert_matrix_impl(m)
}

pub fn assert_vector(v: &Vector) -> bool {
    assert_vector_impl(v)
}

// --- Creation functions ---
pub fn new_matrix(d: &[f64], rows: usize, cols: usize) -> Matrix {
    new_matrix_impl(d, rows, cols)
}

pub fn new_vector(d: &[f64], cols: usize) -> Vector {
    new_vector_impl(d, cols)
}

pub fn null_matrix(rows: usize, cols: usize) -> Matrix {
    null_matrix_impl(rows, cols)
}

pub fn null_vector(cols: usize) -> Vector {
    null_vector_impl(cols)
}

pub fn zero_matrix(rows: usize, cols: usize) -> Matrix {
    zero_matrix_impl(rows, cols)
}

pub fn zero_vector(cols: usize) -> Vector {
    zero_vector_impl(cols)
}

// --- Fill functions ---
pub fn fill_matrix(m: &mut Matrix, n: f64) {
    fill_matrix_impl(m, n)
}

pub fn fill_vector(v: &mut Vector, n: f64) {
    fill_vector_impl(v, n)
}

/// Returns an identity matrix of size n.
pub fn identity_matrix(n: usize) -> Matrix {
    identity_matrix_impl(n)
}

/// “Releases” a matrix (stub; memory is managed automatically in Rust).
pub fn delete_matrix(m: Matrix) {
    crate::matrix::delete_matrix_impl(m)
}

/// “Releases” a vector.
pub fn delete_vector(v: Vector) {
    delete_vector_impl(v)
}

/// Returns a copy of the given matrix.
pub fn copy_matrix(m: &Matrix) -> Matrix {
    copy_matrix_impl(m)
}

/// Returns a copy of the given vector.
pub fn copy_vector(v: &Vector) -> Vector {
    copy_vector_impl(v)
}

/// Flattens the given matrix into a vector.
pub fn flatten_matrix(m: &Matrix) -> Vector {
    flatten_matrix_impl(m)
}

// --- Size functions ---
pub fn matrix_size(m: &Matrix) -> usize {
    matrix_size_impl(m)
}

pub fn vector_size(v: &Vector) -> usize {
    vector_size_impl(v)
}

pub fn matrix_size_bytes(m: &Matrix) -> usize {
    matrix_size_bytes_impl(m)
}

pub fn vector_size_bytes(v: &Vector) -> usize {
    vector_size_bytes_impl(v)
}

// --- Element accessor/mutator functions ---
pub fn set_matrix_element(m: &mut Matrix, i: usize, j: usize, s: f64) {
    set_matrix_element_impl(m, i, j, s)
}

pub fn get_matrix_element(m: &Matrix, i: usize, j: usize) -> f64 {
    get_matrix_element_impl(m, i, j)
}

pub fn set_vector_element(v: &mut Vector, i: usize, s: f64) {
    set_vector_element_impl(v, i, s)
}

pub fn get_vector_element(v: &Vector, i: usize) -> f64 {
    get_vector_element_impl(v, i)
}

// --- Row and column operations ---
pub fn set_row_vector(m: &mut Matrix, i: usize, v: &Vector) {
    set_row_vector_impl(m, i, v)
}

pub fn get_row_vector(m: &Matrix, i: usize) -> Vector {
    get_row_vector_impl(m, i)
}

pub fn set_col_vector(m: &mut Matrix, j: usize, v: &Vector) {
    set_col_vector_impl(m, j, v)
}

pub fn get_col_vector(m: &Matrix, j: usize) -> Vector {
    get_col_vector_impl(m, j)
}

// --- Diagonal operations ---
pub fn get_main_diagonal(m: &Matrix) -> Vector {
    get_main_diagonal_impl(m)
}

pub fn set_main_diagonal(m: &mut Matrix, v: &Vector) {
    set_main_diagonal_impl(m, v)
}

pub fn get_anti_diagonal(m: &Matrix) -> Vector {
    get_anti_diagonal_impl(m)
}

pub fn set_anti_diagonal(m: &mut Matrix, v: &Vector) {
    set_anti_diagonal_impl(m, v)
}

pub fn diagonal_product(m: &Matrix) -> f64 {
    diagonal_product_impl(m)
}

// --- “Pretty” print functions ---
pub fn print_matrix(m: &Matrix, include_indices: bool) {
    print_matrix_impl(m, include_indices)
}

pub fn print_vector(v: &Vector, include_indices: bool) {
    print_vector_impl(v, include_indices)
}

// --- Comparison functions ---
pub fn is_matrix_equal(m: &Matrix, n: &Matrix) -> bool {
    is_matrix_equal_impl(m, n)
}

pub fn is_vector_equal(v: &Vector, w: &Vector) -> bool {
    is_vector_equal_impl(v, w)
}

pub fn has_same_dimensions(m: &Matrix, n: &Matrix) -> bool {
    has_same_dimensions_impl(m, n)
}

// --- Property testing functions ---
pub fn is_zero_matrix(m: &Matrix) -> bool {
    is_zero_matrix_impl(m)
}

pub fn is_identity_matrix(m: &Matrix) -> bool {
    is_identity_matrix_impl(m)
}

pub fn is_square_matrix(m: &Matrix) -> bool {
    is_square_matrix_impl(m)
}

pub fn is_invertible(m: &Matrix) -> bool {
    is_invertible_impl(m)
}

pub fn is_diagonal_matrix(m: &Matrix) -> bool {
    is_diagonal_matrix_impl(m)
}

pub fn is_triangular_matrix(m: &Matrix) -> bool {
    is_triangular_matrix_impl(m)
}

pub fn is_up_tri_matrix(m: &Matrix) -> bool {
    is_up_tri_matrix_impl(m)
}

pub fn is_lo_tri_matrix(m: &Matrix) -> bool {
    is_lo_tri_matrix_impl(m)
}

pub fn is_matrix_symmetric(m: &Matrix) -> bool {
    is_matrix_symmetric_impl(m)
}

pub fn has_zero_row(m: &Matrix) -> bool {
    has_zero_row_impl(m)
}

pub fn has_zero_col(m: &Matrix) -> bool {
    has_zero_col_impl(m)
}

// --- Advanced operations ---
pub fn transpose_matrix(m: &Matrix) -> Matrix {
    transpose_matrix_impl(m)
}

pub fn trace_matrix(m: &Matrix) -> f64 {
    trace_matrix_impl(m)
}

pub fn add_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    add_matrices_impl(m1, m2)
}

pub fn add_vectors(v1: &Vector, v2: &Vector) -> Vector {
    add_vectors_impl(v1, v2)
}

pub fn pow_matrix(m: &Matrix, k: f64) -> Matrix {
    pow_matrix_impl(m, k)
}

pub fn pow_vector(v: &Vector, k: f64) -> Vector {
    pow_vector_impl(v, k)
}

pub fn multiply_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    multiply_matrices_impl(m1, m2)
}

pub fn scale_matrix(m: &Matrix, s: f64) -> Matrix {
    scale_matrix_impl(m, s)
}

pub fn dot_product(v: &Vector, w: &Vector) -> f64 {
    dot_product_impl(v, w)
}

pub fn cross_product(v: &Vector, w: &Vector) -> Vector {
    cross_product_impl(v, w)
}

pub fn vector_magnitude(v: &Vector) -> f64 {
    vector_magnitude_impl(v)
}

pub fn vector_distance(v: &Vector, w: &Vector) -> f64 {
    vector_distance_impl(v, w)
}

pub fn scale_vector(v: &Vector, s: f64) -> Vector {
    scale_vector_impl(v, s)
}

pub fn is_unit_vector(v: &Vector) -> bool {
    is_unit_vector_impl(v)
}

pub fn is_vector_orthogonal(v1: &Vector, v2: &Vector) -> bool {
    is_vector_orthogonal_impl(v1, v2)
}

pub fn is_matrix_orthogonal(m1: &Matrix, m2: &Matrix) -> bool {
    is_matrix_orthogonal_impl(m1, m2)
}

pub fn scalar_triple_product(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    scalar_triple_product_impl(v1, v2, v3)
}

// --- Geometric operations ---
pub fn reflect_axis_2d(m: &Matrix, axis: i32) -> Matrix {
    reflect_axis_2d_impl(m, axis)
}

pub fn reflect_axis_3d(m: &Matrix, axis: i32) -> Matrix {
    reflect_axis_3d_impl(m, axis)
}

pub fn orth_proj_2d(m: &Matrix, axis: i32) -> Matrix {
    orth_proj_2d_impl(m, axis)
}

pub fn orth_proj_3d(m: &Matrix, axis: i32) -> Matrix {
    orth_proj_3d_impl(m, axis)
}

pub fn rotate_2d(m: &Matrix, theta: f64) -> Matrix {
    rotate_2d_impl(m, theta)
}

pub fn scale_n_space(m: &Matrix, k: f64) -> Matrix {
    scale_n_space_impl(m, k)
}

pub fn shear_2d(m: &Matrix, k: f64, axis: i32) -> Matrix {
    shear_2d_impl(m, k, axis)
}

/// Returns the determinant of a matrix.
pub fn determinant(m: &Matrix) -> f64 {
    determinant_impl(m)
}

/// Performs LU decomposition. Returns (L, U, P, swaps).
pub fn lu_decomposition(m: &Matrix) -> (Matrix, Matrix, Matrix, i32) {
    lu_decomposition_impl(m)
}

/// Returns the submatrix of m excluding row i and column j.
pub fn sub_matrix(m: &Matrix, i: usize, j: usize) -> Matrix {
    sub_matrix_impl(m, i, j)
}

/// Returns the minor of m at (i,j).
pub fn element_minor(m: &Matrix, i: usize, j: usize) -> f64 {
    element_minor_impl(m, i, j)
}

/// Returns the matrix of minors of m.
pub fn matrix_minor(m: &Matrix) -> Matrix {
    matrix_minor_impl(m)
}

/// Returns the cofactor of element (i,j) in m.
pub fn element_cofactor(m: &Matrix, i: usize, j: usize) -> f64 {
    element_cofactor_impl(m, i, j)
}

/// Returns the matrix of cofactors of m.
pub fn matrix_cofactor(m: &Matrix) -> Matrix {
    matrix_cofactor_impl(m)
}

/// Returns a sign matrix of the given dimensions.
pub fn sign_matrix(rows: usize, cols: usize) -> Matrix {
    sign_matrix_impl(rows, cols)
}

/// Returns the adjugate matrix of m.
pub fn adjugate_matrix(m: &Matrix) -> Matrix {
    adjugate_matrix_impl(m)
}

/// Returns the inverse of m.
pub fn inverse_matrix(m: &Matrix) -> Matrix {
    inverse_matrix_impl(m)
}

/// Returns the pivot matrix of m along with the number of swaps.
pub fn pivot_matrix(m: &Matrix) -> (Matrix, i32) {
    pivot_matrix_impl(m)
}
