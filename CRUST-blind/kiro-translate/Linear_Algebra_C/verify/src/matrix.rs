use crate::linear_algebra::{self, Matrix, Vector};
pub fn assert_matrix_impl(m: &Matrix) -> bool { linear_algebra::assert_matrix(m) }
pub fn new_matrix_impl(d: &[f64], rows: usize, cols: usize) -> Matrix { linear_algebra::new_matrix(d, rows, cols) }
pub fn null_matrix_impl(rows: usize, cols: usize) -> Matrix { linear_algebra::null_matrix(rows, cols) }
pub fn zero_matrix_impl(rows: usize, cols: usize) -> Matrix { linear_algebra::zero_matrix(rows, cols) }
pub fn fill_matrix_impl(m: &mut Matrix, n: f64) { linear_algebra::fill_matrix(m, n) }
pub fn identity_matrix_impl(n: usize) -> Matrix { linear_algebra::identity_matrix(n) }
pub fn delete_matrix_impl(m: Matrix) { linear_algebra::delete_matrix(m) }
pub fn copy_matrix_impl(m: &Matrix) -> Matrix { linear_algebra::copy_matrix(m) }
pub fn flatten_matrix_impl(m: &Matrix) -> Vector { linear_algebra::flatten_matrix(m) }
pub fn matrix_size_impl(m: &Matrix) -> usize { linear_algebra::matrix_size(m) }
pub fn matrix_size_bytes_impl(m: &Matrix) -> usize { linear_algebra::matrix_size_bytes(m) }
pub fn set_matrix_element_impl(m: &mut Matrix, i: usize, j: usize, s: f64) { linear_algebra::set_matrix_element(m, i, j, s) }
pub fn get_matrix_element_impl(m: &Matrix, i: usize, j: usize) -> f64 { linear_algebra::get_matrix_element(m, i, j) }
pub fn set_row_vector_impl(m: &mut Matrix, i: usize, v: &Vector) { linear_algebra::set_row_vector(m, i, v) }
pub fn get_row_vector_impl(m: &Matrix, i: usize) -> Vector { linear_algebra::get_row_vector(m, i) }
pub fn set_col_vector_impl(m: &mut Matrix, j: usize, v: &Vector) { linear_algebra::set_col_vector(m, j, v) }
pub fn get_col_vector_impl(m: &Matrix, j: usize) -> Vector { linear_algebra::get_col_vector(m, j) }
pub fn get_main_diagonal_impl(m: &Matrix) -> Vector { linear_algebra::get_main_diagonal(m) }
pub fn set_main_diagonal_impl(m: &mut Matrix, v: &Vector) { linear_algebra::set_main_diagonal(m, v) }
pub fn get_anti_diagonal_impl(m: &Matrix) -> Vector { linear_algebra::get_anti_diagonal(m) }
pub fn set_anti_diagonal_impl(m: &mut Matrix, v: &Vector) { linear_algebra::set_anti_diagonal(m, v) }
pub fn diagonal_product_impl(m: &Matrix) -> f64 { linear_algebra::diagonal_product(m) }
pub fn is_matrix_equal_impl(m: &Matrix, n: &Matrix) -> bool { linear_algebra::is_matrix_equal(m, n) }
pub fn has_same_dimensions_impl(m: &Matrix, n: &Matrix) -> bool { linear_algebra::has_same_dimensions(m, n) }
pub fn is_zero_matrix_impl(m: &Matrix) -> bool { linear_algebra::is_zero_matrix(m) }
pub fn is_identity_matrix_impl(m: &Matrix) -> bool { linear_algebra::is_identity_matrix(m) }
pub fn is_square_matrix_impl(m: &Matrix) -> bool { linear_algebra::is_square_matrix(m) }
pub fn is_invertible_impl(m: &Matrix) -> bool { linear_algebra::is_invertible(m) }
pub fn is_diagonal_matrix_impl(m: &Matrix) -> bool { linear_algebra::is_diagonal_matrix(m) }
pub fn is_triangular_matrix_impl(m: &Matrix) -> bool { linear_algebra::is_triangular_matrix(m) }
pub fn is_up_tri_matrix_impl(m: &Matrix) -> bool { linear_algebra::is_up_tri_matrix(m) }
pub fn is_lo_tri_matrix_impl(m: &Matrix) -> bool { linear_algebra::is_lo_tri_matrix(m) }
pub fn is_matrix_symmetric_impl(m: &Matrix) -> bool { linear_algebra::is_matrix_symmetric(m) }
pub fn has_zero_row_impl(m: &Matrix) -> bool { linear_algebra::has_zero_row(m) }
pub fn has_zero_col_impl(m: &Matrix) -> bool { linear_algebra::has_zero_col(m) }
pub fn transpose_matrix_impl(m: &Matrix) -> Matrix { linear_algebra::transpose_matrix(m) }
pub fn trace_matrix_impl(m: &Matrix) -> f64 { linear_algebra::trace_matrix(m) }
pub fn add_matrices_impl(m1: &Matrix, m2: &Matrix) -> Matrix { linear_algebra::add_matrices(m1, m2) }
pub fn pow_matrix_impl(m: &Matrix, k: f64) -> Matrix { linear_algebra::pow_matrix(m, k) }
pub fn multiply_matrices_impl(m1: &Matrix, m2: &Matrix) -> Matrix { linear_algebra::multiply_matrices(m1, m2) }
pub fn scale_matrix_impl(m: &Matrix, s: f64) -> Matrix { linear_algebra::scale_matrix(m, s) }
pub fn sub_matrix_impl(m: &Matrix, i: usize, j: usize) -> Matrix { linear_algebra::sub_matrix(m, i, j) }
pub fn element_minor_impl(m: &Matrix, i: usize, j: usize) -> f64 { linear_algebra::element_minor(m, i, j) }
pub fn matrix_minor_impl(m: &Matrix) -> Matrix { linear_algebra::matrix_minor(m) }
pub fn element_cofactor_impl(m: &Matrix, i: usize, j: usize) -> f64 { linear_algebra::element_cofactor(m, i, j) }
pub fn matrix_cofactor_impl(m: &Matrix) -> Matrix { linear_algebra::matrix_cofactor(m) }
pub fn sign_matrix_impl(rows: usize, cols: usize) -> Matrix { linear_algebra::sign_matrix(rows, cols) }
pub fn adjugate_matrix_impl(m: &Matrix) -> Matrix { linear_algebra::adjugate_matrix(m) }
pub fn lu_decomposition_impl(m: &Matrix) -> (Matrix, Matrix, Matrix, i32) { linear_algebra::lu_decomposition(m) }
pub fn inverse_matrix_impl(m: &Matrix) -> Matrix { linear_algebra::inverse_matrix(m) }
pub fn pivot_matrix_impl(m: &Matrix) -> (Matrix, i32) { linear_algebra::pivot_matrix(m) }
pub fn scale_n_space_impl(m: &Matrix, k: f64) -> Matrix { linear_algebra::scale_n_space(m, k) }
pub fn reflect_axis_2d_impl(m: &Matrix, axis: i32) -> Matrix { linear_algebra::reflect_axis_2d(m, axis) }
pub fn reflect_axis_3d_impl(m: &Matrix, axis: i32) -> Matrix { linear_algebra::reflect_axis_3d(m, axis) }
pub fn orth_proj_2d_impl(m: &Matrix, axis: i32) -> Matrix { linear_algebra::orth_proj_2d(m, axis) }
pub fn orth_proj_3d_impl(m: &Matrix, axis: i32) -> Matrix { linear_algebra::orth_proj_3d(m, axis) }
pub fn shear_2d_impl(m: &Matrix, k: f64, axis: i32) -> Matrix { linear_algebra::shear_2d(m, k, axis) }
