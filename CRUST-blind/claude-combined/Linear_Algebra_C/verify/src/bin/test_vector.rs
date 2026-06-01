use Linear_Algebra_C::linear_algebra::Vector;
use Linear_Algebra_C::vector::*;

fn v(d: &[f64]) -> Vector {
    new_vector_impl(d, d.len())
}

#[test]
fn test_new_vector_impl() {
    let data = [1.0, 2.0, 3.0];
    let vec = new_vector_impl(&data, 3);
    assert_eq!(vec.cols, 3);
    assert_eq!(vec.data, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_null_vector_impl() {
    let n = null_vector_impl(5);
    assert_eq!(n.cols, 5);
    assert_eq!(n.data.len(), 5);
}

#[test]
fn test_zero_vector_impl() {
    let z = zero_vector_impl(4);
    assert_eq!(z.cols, 4);
    assert_eq!(z.data, vec![0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_fill_vector_impl() {
    let mut z = zero_vector_impl(3);
    fill_vector_impl(&mut z, 7.0);
    assert_eq!(z.data, vec![7.0, 7.0, 7.0]);
}

#[test]
fn test_assert_vector_impl() {
    let z = zero_vector_impl(2);
    assert_eq!(assert_vector_impl(&z), true);
}

#[test]
fn test_copy_vector_impl() {
    let orig = v(&[1.0, 2.0, 3.0]);
    let cp = copy_vector_impl(&orig);
    assert_eq!(cp.cols, 3);
    assert_eq!(cp.data, orig.data);
}

#[test]
fn test_vector_size_impl() {
    let z = v(&[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(vector_size_impl(&z), 4);
}

#[test]
fn test_vector_size_bytes_impl() {
    // C: sizeof(double) * cols = 8 * 8 = 64
    let z = v(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    assert_eq!(vector_size_bytes_impl(&z), 64);
}

#[test]
fn test_is_vector_equal_impl_true() {
    let a = v(&[1.0, 2.0, 3.0]);
    let b = v(&[1.0, 2.0, 3.0]);
    assert_eq!(is_vector_equal_impl(&a, &b), true);
}

#[test]
fn test_is_vector_equal_impl_false_different_size() {
    let a = v(&[1.0, 2.0, 3.0]);
    let b = v(&[1.0, 2.0]);
    assert_eq!(is_vector_equal_impl(&a, &b), false);
}

#[test]
fn test_is_vector_equal_impl_false_different_data() {
    let a = v(&[1.0, 2.0, 3.0]);
    let b = v(&[1.0, 2.0, 4.0]);
    assert_eq!(is_vector_equal_impl(&a, &b), false);
}

#[test]
fn test_set_get_vector_element_impl() {
    let mut z = zero_vector_impl(3);
    set_vector_element_impl(&mut z, 1, 42.5);
    assert_eq!(get_vector_element_impl(&z, 0), 0.0);
    assert_eq!(get_vector_element_impl(&z, 1), 42.5);
    assert_eq!(get_vector_element_impl(&z, 2), 0.0);
}

#[test]
fn test_vector_magnitude_impl() {
    // {2, 4, 3} -> sqrt(29)
    let a = v(&[2.0, 4.0, 3.0]);
    let mag = vector_magnitude_impl(&a);
    assert!((mag - 29f64.sqrt()).abs() < 1e-12);
}

#[test]
fn test_is_unit_vector_impl_true() {
    let a = v(&[1.0, 0.0, 0.0]);
    assert_eq!(is_unit_vector_impl(&a), true);
}

#[test]
fn test_is_unit_vector_impl_false() {
    let a = v(&[2.0, 4.0, 3.0]);
    assert_eq!(is_unit_vector_impl(&a), false);
}

#[test]
fn test_is_vector_orthogonal_impl_true() {
    let a = v(&[1.0, 0.0, 0.0]);
    let b = v(&[0.0, 1.0, 0.0]);
    assert_eq!(is_vector_orthogonal_impl(&a, &b), true);
}

#[test]
fn test_is_vector_orthogonal_impl_false() {
    let a = v(&[2.0, 4.0, 3.0]);
    let b = v(&[5.0, 10.0, 15.0]);
    assert_eq!(is_vector_orthogonal_impl(&a, &b), false);
}

#[test]
fn test_dot_product_impl() {
    let a = v(&[2.0, 4.0, 3.0]);
    let b = v(&[5.0, 10.0, 15.0]);
    // 10 + 40 + 45 = 95
    assert_eq!(dot_product_impl(&a, &b), 95.0);
}

#[test]
fn test_cross_product_impl() {
    let a = v(&[2.0, 4.0, 3.0]);
    let b = v(&[5.0, 10.0, 15.0]);
    let c = cross_product_impl(&a, &b);
    // C version: (b1*w2-b2*w1, a*c-c*a, ad*bd-bd*ad)
    // c0 = v1*w2 - v2*w1 = 4*15 - 3*10 = 60 - 30 = 30
    // c1 = v0*w2 - v2*w0 = 2*15 - 3*5  = 30 - 15 = 15
    // c2 = v0*w1 - v1*w0 = 2*10 - 4*5  = 20 - 20 = 0
    assert_eq!(c.cols, 3);
    assert_eq!(c.data[0], 30.0);
    assert_eq!(c.data[1], 15.0);
    assert_eq!(c.data[2], 0.0);
}

#[test]
fn test_vector_distance_impl() {
    let a = v(&[2.0, 4.0, 3.0]);
    let b = v(&[5.0, 10.0, 15.0]);
    let d = vector_distance_impl(&a, &b);
    // sqrt(9 + 36 + 144) = sqrt(189) ≈ 13.7477...
    let expected = 189f64.sqrt();
    assert!((d - expected).abs() < 1e-12);
}

#[test]
fn test_add_vectors_impl() {
    let a = v(&[1.0, 2.0, 3.0]);
    let b = v(&[4.0, 5.0, 6.0]);
    let s = add_vectors_impl(&a, &b);
    assert_eq!(s.cols, 3);
    assert_eq!(s.data, vec![5.0, 7.0, 9.0]);
}

#[test]
fn test_scale_vector_impl() {
    let a = v(&[1.0, 2.0, 3.0]);
    let s = scale_vector_impl(&a, 2.5);
    assert_eq!(s.cols, 3);
    assert_eq!(s.data, vec![2.5, 5.0, 7.5]);
}

#[test]
fn test_pow_vector_impl() {
    let a = v(&[2.0, 3.0, 4.0]);
    let p = pow_vector_impl(&a, 2.0);
    assert_eq!(p.cols, 3);
    assert_eq!(p.data, vec![4.0, 9.0, 16.0]);
}

#[test]
fn test_scalar_triple_product_impl() {
    let a = v(&[1.0, 2.0, 3.0]);
    let b = v(&[4.0, 5.0, 6.0]);
    let c = v(&[7.0, 8.0, 9.0]);
    // expected: -24 (computed via C)
    assert_eq!(scalar_triple_product_impl(&a, &b, &c), -24.0);
}

#[test]
fn test_delete_vector_impl_no_op() {
    let a = v(&[1.0, 2.0]);
    delete_vector_impl(a); // no-op; ensures it compiles & runs
}

fn main() {}
