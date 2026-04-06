use crate::linear_algebra::{self, Vector};
pub fn assert_vector_impl(v: &Vector) -> bool {
    linear_algebra::assert_vector(v)
}
pub fn new_vector_impl(d: &[f64], cols: usize) -> Vector {
    linear_algebra::new_vector(d, cols)
}
pub fn null_vector_impl(cols: usize) -> Vector {
    linear_algebra::null_vector(cols)
}
pub fn zero_vector_impl(cols: usize) -> Vector {
    linear_algebra::zero_vector(cols)
}
pub fn fill_vector_impl(v: &mut Vector, n: f64) {
    linear_algebra::fill_vector(v, n)
}
pub fn delete_vector_impl(v: Vector) {
    linear_algebra::delete_vector(v)
}
pub fn copy_vector_impl(v: &Vector) -> Vector {
    linear_algebra::copy_vector(v)
}
pub fn vector_size_impl(v: &Vector) -> usize {
    linear_algebra::vector_size(v)
}
pub fn vector_size_bytes_impl(v: &Vector) -> usize {
    linear_algebra::vector_size_bytes(v)
}
pub fn is_vector_equal_impl(v: &Vector, w: &Vector) -> bool {
    linear_algebra::is_vector_equal(v, w)
}
pub fn set_vector_element_impl(v: &mut Vector, i: usize, s: f64) {
    linear_algebra::set_vector_element(v, i, s)
}
pub fn get_vector_element_impl(v: &Vector, i: usize) -> f64 {
    linear_algebra::get_vector_element(v, i)
}
pub fn print_vector_impl(v: &Vector, include_indices: bool) {
    linear_algebra::print_vector(v, include_indices)
}
pub fn vector_magnitude_impl(v: &Vector) -> f64 {
    linear_algebra::vector_magnitude(v)
}
pub fn is_unit_vector_impl(v: &Vector) -> bool {
    linear_algebra::is_unit_vector(v)
}
pub fn is_vector_orthogonal_impl(v1: &Vector, v2: &Vector) -> bool {
    linear_algebra::is_vector_orthogonal(v1, v2)
}
pub fn dot_product_impl(v: &Vector, w: &Vector) -> f64 {
    linear_algebra::dot_product(v, w)
}
pub fn cross_product_impl(v: &Vector, w: &Vector) -> Vector {
    linear_algebra::cross_product(v, w)
}
pub fn vector_distance_impl(v: &Vector, w: &Vector) -> f64 {
    linear_algebra::vector_distance(v, w)
}
pub fn add_vectors_impl(v1: &Vector, v2: &Vector) -> Vector {
    linear_algebra::add_vectors(v1, v2)
}
pub fn scale_vector_impl(v: &Vector, s: f64) -> Vector {
    linear_algebra::scale_vector(v, s)
}
pub fn pow_vector_impl(v: &Vector, k: f64) -> Vector {
    linear_algebra::pow_vector(v, k)
}
pub fn scalar_triple_product_impl(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    linear_algebra::scalar_triple_product(v1, v2, v3)
}
