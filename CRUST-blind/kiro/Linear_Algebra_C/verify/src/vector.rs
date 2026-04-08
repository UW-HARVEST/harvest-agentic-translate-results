use crate::linear_algebra::Vector;
pub fn assert_vector_impl(_v: &Vector) -> bool {
    true
}
pub fn new_vector_impl(d: &[f64], cols: usize) -> Vector {
    assert!(cols > 0);
    Vector { cols, data: d[..cols].to_vec() }
}
pub fn null_vector_impl(cols: usize) -> Vector {
    assert!(cols > 0);
    Vector { cols, data: vec![0.0; cols] }
}
pub fn zero_vector_impl(cols: usize) -> Vector {
    let mut v = null_vector_impl(cols);
    fill_vector_impl(&mut v, 0.0);
    v
}
pub fn fill_vector_impl(v: &mut Vector, n: f64) {
    for x in v.data.iter_mut() { *x = n; }
}
pub fn delete_vector_impl(_v: Vector) {
    // drop
}
pub fn copy_vector_impl(v: &Vector) -> Vector {
    Vector { cols: v.cols, data: v.data.clone() }
}
pub fn vector_size_impl(v: &Vector) -> usize {
    v.cols
}
pub fn vector_size_bytes_impl(v: &Vector) -> usize {
    std::mem::size_of::<f64>() * vector_size_impl(v)
}
pub fn is_vector_equal_impl(v: &Vector, w: &Vector) -> bool {
    if v.cols != w.cols { return false; }
    v.data == w.data
}
pub fn set_vector_element_impl(v: &mut Vector, i: usize, s: f64) {
    assert!(i < v.cols);
    v.data[i] = s;
}
pub fn get_vector_element_impl(v: &Vector, i: usize) -> f64 {
    assert!(i < v.cols);
    v.data[i]
}
pub fn print_vector_impl(v: &Vector, include_indices: bool) {
    for i in 0..v.cols {
        if include_indices { print!("[{}] -> ", i); }
        print!("{:16.8} ", v.data[i]);
    }
}
pub fn vector_magnitude_impl(v: &Vector) -> f64 {
    v.data.iter().map(|x| x * x).sum::<f64>().sqrt()
}
pub fn is_unit_vector_impl(v: &Vector) -> bool {
    vector_magnitude_impl(v) == 1.0
}
pub fn is_vector_orthogonal_impl(v1: &Vector, v2: &Vector) -> bool {
    dot_product_impl(v1, v2) == 0.0
}
pub fn dot_product_impl(v: &Vector, w: &Vector) -> f64 {
    assert!(v.cols == w.cols);
    v.data.iter().zip(w.data.iter()).map(|(a, b)| a * b).sum()
}
pub fn cross_product_impl(v: &Vector, w: &Vector) -> Vector {
    assert!(v.cols == 3 && w.cols == 3);
    // Match C bug: element[1] is NOT negated
    Vector {
        cols: 3,
        data: vec![
            v.data[1] * w.data[2] - v.data[2] * w.data[1],
            v.data[0] * w.data[2] - v.data[2] * w.data[0],
            v.data[0] * w.data[1] - v.data[1] * w.data[0],
        ],
    }
}
pub fn vector_distance_impl(v: &Vector, w: &Vector) -> f64 {
    assert!(v.cols == w.cols);
    v.data.iter().zip(w.data.iter()).map(|(a, b)| (b - a) * (b - a)).sum::<f64>().sqrt()
}
pub fn add_vectors_impl(v1: &Vector, v2: &Vector) -> Vector {
    assert!(v1.cols == v2.cols);
    Vector {
        cols: v1.cols,
        data: v1.data.iter().zip(v2.data.iter()).map(|(a, b)| a + b).collect(),
    }
}
pub fn scale_vector_impl(v: &Vector, s: f64) -> Vector {
    Vector { cols: v.cols, data: v.data.iter().map(|x| x * s).collect() }
}
pub fn pow_vector_impl(v: &Vector, k: f64) -> Vector {
    Vector { cols: v.cols, data: v.data.iter().map(|x| x.powf(k)).collect() }
}
pub fn scalar_triple_product_impl(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    assert!(v1.cols == 3 && v2.cols == 3 && v3.cols == 3);
    dot_product_impl(v1, &cross_product_impl(v2, v3))
}
