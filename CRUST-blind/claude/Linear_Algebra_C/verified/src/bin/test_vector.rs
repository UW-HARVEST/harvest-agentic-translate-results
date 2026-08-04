use Linear_Algebra_C::vector as v_impl;
use Linear_Algebra_C::linear_algebra::Vector;

#[test]
fn test_assert_vector_impl() {
    let v = v_impl::null_vector_impl(3);
    assert_eq!(v_impl::assert_vector_impl(&v), true);
}

#[test]
fn test_new_vector_impl() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let v = v_impl::new_vector_impl(&data, 8);
    assert_eq!(v.cols, 8);
    assert_eq!(v.data.len(), 8);
    for i in 0..8 {
        assert_eq!(v.data[i], data[i]);
    }
}

#[test]
fn test_null_vector_impl() {
    let v = v_impl::null_vector_impl(5);
    assert_eq!(v.cols, 5);
    assert_eq!(v.data.len(), 5);
    for x in &v.data {
        assert_eq!(*x, 0.0);
    }
}

#[test]
fn test_zero_vector_impl() {
    let v = v_impl::zero_vector_impl(4);
    assert_eq!(v.cols, 4);
    assert_eq!(v.data, vec![0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_fill_vector_impl() {
    let mut v = v_impl::null_vector_impl(3);
    v_impl::fill_vector_impl(&mut v, 7.5);
    assert_eq!(v.cols, 3);
    assert_eq!(v.data, vec![7.5, 7.5, 7.5]);
}

#[test]
fn test_delete_vector_impl_runs() {
    let v = Vector { cols: 1, data: vec![1.0] };
    v_impl::delete_vector_impl(v);
}

#[test]
fn test_copy_vector_impl() {
    let v = v_impl::new_vector_impl(&[1.0, 2.0, 3.0, 4.0], 4);
    let c = v_impl::copy_vector_impl(&v);
    assert_eq!(c.cols, 4);
    assert_eq!(c.data, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_vector_size_impl() {
    let v = v_impl::new_vector_impl(&[1.0, 2.0, 3.0, 4.0, 5.0], 5);
    assert_eq!(v_impl::vector_size_impl(&v), 5);
}

#[test]
fn test_vector_size_bytes_impl() {
    // C: 8 doubles = 64 bytes
    let v = v_impl::new_vector_impl(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 8);
    assert_eq!(v_impl::vector_size_bytes_impl(&v), 64);
}

#[test]
fn test_is_vector_equal_impl() {
    let v = v_impl::zero_vector_impl(3);
    let w = v_impl::zero_vector_impl(3);
    assert_eq!(v_impl::is_vector_equal_impl(&v, &w), true);
    let mut x = v_impl::zero_vector_impl(3);
    x.data[0] = 1.0;
    assert_eq!(v_impl::is_vector_equal_impl(&v, &x), false);
    let y = v_impl::zero_vector_impl(4);
    assert_eq!(v_impl::is_vector_equal_impl(&v, &y), false);
}

#[test]
fn test_set_get_vector_element_impl() {
    let mut v = v_impl::zero_vector_impl(2);
    v_impl::set_vector_element_impl(&mut v, 1, 10.0);
    assert_eq!(v_impl::get_vector_element_impl(&v, 1), 10.0);
    assert_eq!(v_impl::get_vector_element_impl(&v, 0), 0.0);
}

#[test]
fn test_print_vector_impl_runs() {
    let v = v_impl::new_vector_impl(&[1.0, 2.0], 2);
    v_impl::print_vector_impl(&v, true);
    v_impl::print_vector_impl(&v, false);
}

#[test]
fn test_vector_magnitude_impl() {
    // C ground truth: vectorMagnitude([2,4,3]) = 5.385164807134504
    let v = v_impl::new_vector_impl(&[2.0, 4.0, 3.0], 3);
    let m = v_impl::vector_magnitude_impl(&v);
    assert!((m - 5.385164807134504_f64).abs() < 1e-12);

    let z = v_impl::zero_vector_impl(3);
    assert_eq!(v_impl::vector_magnitude_impl(&z), 0.0);

    // Pythagorean triple
    let p = v_impl::new_vector_impl(&[3.0, 4.0], 2);
    assert_eq!(v_impl::vector_magnitude_impl(&p), 5.0);
}

#[test]
fn test_is_unit_vector_impl() {
    let u = v_impl::new_vector_impl(&[1.0, 0.0], 2);
    assert_eq!(v_impl::is_unit_vector_impl(&u), true);
    let v = v_impl::new_vector_impl(&[2.0, 4.0, 3.0], 3);
    assert_eq!(v_impl::is_unit_vector_impl(&v), false);
    let w = v_impl::new_vector_impl(&[0.0, 0.0, 1.0], 3);
    assert_eq!(v_impl::is_unit_vector_impl(&w), true);
}

#[test]
fn test_is_vector_orthogonal_impl() {
    let u1 = v_impl::new_vector_impl(&[1.0, 0.0, 0.0], 3);
    let u2 = v_impl::new_vector_impl(&[0.0, 1.0, 0.0], 3);
    assert_eq!(v_impl::is_vector_orthogonal_impl(&u1, &u2), true);

    let v = v_impl::new_vector_impl(&[2.0, 4.0, 3.0], 3);
    let w = v_impl::new_vector_impl(&[5.0, 10.0, 15.0], 3);
    assert_eq!(v_impl::is_vector_orthogonal_impl(&v, &w), false);
}

#[test]
fn test_dot_product_impl() {
    // C ground truth: dotProduct([2,4,3], [5,10,15]) = 95
    let v = v_impl::new_vector_impl(&[2.0, 4.0, 3.0], 3);
    let w = v_impl::new_vector_impl(&[5.0, 10.0, 15.0], 3);
    assert_eq!(v_impl::dot_product_impl(&v, &w), 95.0);

    let a = v_impl::new_vector_impl(&[1.0, 2.0], 2);
    let b = v_impl::new_vector_impl(&[3.0, 4.0], 2);
    assert_eq!(v_impl::dot_product_impl(&a, &b), 11.0);
}

#[test]
fn test_cross_product_impl() {
    // C ground truth: cross([2,4,3], [5,10,15]) = [30, 15, 0]
    let v = v_impl::new_vector_impl(&[2.0, 4.0, 3.0], 3);
    let w = v_impl::new_vector_impl(&[5.0, 10.0, 15.0], 3);
    let c = v_impl::cross_product_impl(&v, &w);
    assert_eq!(c.cols, 3);
    assert_eq!(c.data[0], 30.0);
    assert_eq!(c.data[1], 15.0);
    assert_eq!(c.data[2], 0.0);
}

#[test]
fn test_vector_distance_impl() {
    // C ground truth: roundn(vectorDistance([2,4,3],[5,10,15]),3) = 13.748
    let v = v_impl::new_vector_impl(&[2.0, 4.0, 3.0], 3);
    let w = v_impl::new_vector_impl(&[5.0, 10.0, 15.0], 3);
    let d = v_impl::vector_distance_impl(&v, &w);
    assert!((d - 13.747727084867520_f64).abs() < 1e-12);

    let a = v_impl::new_vector_impl(&[0.0, 0.0], 2);
    let b = v_impl::new_vector_impl(&[3.0, 4.0], 2);
    assert_eq!(v_impl::vector_distance_impl(&a, &b), 5.0);
}

#[test]
fn test_add_vectors_impl() {
    let v = v_impl::new_vector_impl(&[1.0, 2.0, 3.0], 3);
    let w = v_impl::new_vector_impl(&[4.0, 5.0, 6.0], 3);
    let s = v_impl::add_vectors_impl(&v, &w);
    assert_eq!(s.cols, 3);
    assert_eq!(s.data, vec![5.0, 7.0, 9.0]);
}

#[test]
fn test_scale_vector_impl() {
    let v = v_impl::new_vector_impl(&[1.0, 2.0, 3.0], 3);
    let s = v_impl::scale_vector_impl(&v, 5.0);
    assert_eq!(s.cols, 3);
    assert_eq!(s.data, vec![5.0, 10.0, 15.0]);
}

#[test]
fn test_pow_vector_impl() {
    // C ground truth: powVector([1,2,3,4], 3) = [1, 8, 27, 64]
    let v = v_impl::new_vector_impl(&[1.0, 2.0, 3.0, 4.0], 4);
    let p = v_impl::pow_vector_impl(&v, 3.0);
    assert_eq!(p.cols, 4);
    assert_eq!(p.data, vec![1.0, 8.0, 27.0, 64.0]);

    let p2 = v_impl::pow_vector_impl(&v, 2.0);
    assert_eq!(p2.data, vec![1.0, 4.0, 9.0, 16.0]);
}

#[test]
fn test_scalar_triple_product_impl() {
    // C ground truth: scalarTripleProduct([1,2,3],[4,5,6],[7,8,10]) = -11
    let v1 = v_impl::new_vector_impl(&[1.0, 2.0, 3.0], 3);
    let v2 = v_impl::new_vector_impl(&[4.0, 5.0, 6.0], 3);
    let v3 = v_impl::new_vector_impl(&[7.0, 8.0, 10.0], 3);
    let s = v_impl::scalar_triple_product_impl(&v1, &v2, &v3);
    assert_eq!(s, -11.0);
}

fn main() {}
