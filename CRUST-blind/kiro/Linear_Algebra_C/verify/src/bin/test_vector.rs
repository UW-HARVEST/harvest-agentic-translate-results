use Linear_Algebra_C::vector;
use Linear_Algebra_C::linear_algebra::{Vector, new_vector};

fn v3(a: f64, b: f64, c: f64) -> Vector { new_vector(&[a, b, c], 3) }

#[test]
fn test_new_and_size() {
    let v = vector::new_vector_impl(&[1.0, 2.0, 3.0], 3);
    assert_eq!(vector::vector_size_impl(&v), 3);
    assert_eq!(vector::vector_size_bytes_impl(&v), 24);
    assert_eq!(vector::get_vector_element_impl(&v, 0), 1.0);
    assert_eq!(vector::get_vector_element_impl(&v, 1), 2.0);
    assert_eq!(vector::get_vector_element_impl(&v, 2), 3.0);
}

#[test]
fn test_zero_and_fill() {
    let mut v = vector::zero_vector_impl(4);
    assert_eq!(vector::get_vector_element_impl(&v, 0), 0.0);
    vector::fill_vector_impl(&mut v, 5.0);
    assert_eq!(vector::get_vector_element_impl(&v, 3), 5.0);
}

#[test]
fn test_copy_and_equal() {
    let mut v = vector::zero_vector_impl(4);
    vector::set_vector_element_impl(&mut v, 2, 2.0);
    let v2 = vector::copy_vector_impl(&v);
    assert!(vector::is_vector_equal_impl(&v, &v2));
    assert_eq!(vector::get_vector_element_impl(&v2, 2), 2.0);
}

#[test]
fn test_dot_product() {
    let v = v3(2.0, 4.0, 3.0);
    let w = v3(5.0, 10.0, 15.0);
    assert_eq!(vector::dot_product_impl(&v, &w), 95.0);
}

#[test]
fn test_cross_product() {
    let v = v3(2.0, 4.0, 3.0);
    let w = v3(5.0, 10.0, 15.0);
    let c = vector::cross_product_impl(&v, &w);
    assert_eq!(c.data[0], 30.0);
    assert_eq!(c.data[1], 15.0);
    assert_eq!(c.data[2], 0.0);
}

#[test]
fn test_vector_magnitude() {
    let v = v3(2.0, 4.0, 3.0);
    let mag = vector::vector_magnitude_impl(&v);
    assert_eq!(Linear_Algebra_C::utils::roundn(mag, 3), 5.385);
}

#[test]
fn test_vector_distance() {
    let v = v3(2.0, 4.0, 3.0);
    let w = v3(5.0, 10.0, 15.0);
    assert_eq!(Linear_Algebra_C::utils::roundn(vector::vector_distance_impl(&v, &w), 3), 13.748);
}

#[test]
fn test_is_unit_vector() {
    let v = v3(2.0, 4.0, 3.0);
    let e1 = v3(1.0, 0.0, 0.0);
    assert!(!vector::is_unit_vector_impl(&v));
    assert!(vector::is_unit_vector_impl(&e1));
}

#[test]
fn test_is_vector_orthogonal() {
    let e1 = v3(1.0, 0.0, 0.0);
    let e2 = v3(0.0, 1.0, 0.0);
    let v = v3(2.0, 4.0, 3.0);
    let w = v3(5.0, 10.0, 15.0);
    assert!(vector::is_vector_orthogonal_impl(&e1, &e2));
    assert!(!vector::is_vector_orthogonal_impl(&v, &w));
}

#[test]
fn test_add_vectors() {
    let v = v3(2.0, 4.0, 3.0);
    let w = v3(5.0, 10.0, 15.0);
    let s = vector::add_vectors_impl(&v, &w);
    assert_eq!(s.data, vec![7.0, 14.0, 18.0]);
}

#[test]
fn test_scale_vector() {
    let v = v3(2.0, 4.0, 3.0);
    let s = vector::scale_vector_impl(&v, 3.0);
    assert_eq!(s.data, vec![6.0, 12.0, 9.0]);
}

#[test]
fn test_pow_vector() {
    let v = v3(2.0, 4.0, 3.0);
    let p = vector::pow_vector_impl(&v, 2.0);
    assert_eq!(p.data, vec![4.0, 16.0, 9.0]);
}

#[test]
fn test_scalar_triple_product() {
    let e1 = v3(1.0, 0.0, 0.0);
    let e2 = v3(0.0, 1.0, 0.0);
    let e3 = v3(0.0, 0.0, 1.0);
    assert_eq!(vector::scalar_triple_product_impl(&e1, &e2, &e3), 1.0);
    let v = v3(2.0, 4.0, 3.0);
    let w = v3(5.0, 10.0, 15.0);
    assert_eq!(vector::scalar_triple_product_impl(&v, &w, &e1), -90.0);
}

fn main() {}
