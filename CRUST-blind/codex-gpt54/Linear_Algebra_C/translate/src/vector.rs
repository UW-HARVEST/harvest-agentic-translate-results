use crate::linear_algebra::Vector;

pub fn assert_vector_impl(v: &Vector) -> bool {
    assert!(v.cols > 0);
    assert_eq!(v.data.len(), v.cols);
    true
}

pub fn new_vector_impl(d: &[f64], cols: usize) -> Vector {
    assert!(cols > 0);
    assert!(d.len() >= cols);
    Vector {
        cols,
        data: d[..cols].to_vec(),
    }
}

pub fn null_vector_impl(cols: usize) -> Vector {
    assert!(cols > 0);
    Vector {
        cols,
        data: vec![0.0; cols],
    }
}

pub fn zero_vector_impl(cols: usize) -> Vector {
    let mut v = null_vector_impl(cols);
    fill_vector_impl(&mut v, 0.0);
    v
}

pub fn fill_vector_impl(v: &mut Vector, n: f64) {
    assert_vector_impl(v);
    for value in &mut v.data {
        *value = n;
    }
}

pub fn delete_vector_impl(v: Vector) {
    drop(v);
}

pub fn copy_vector_impl(v: &Vector) -> Vector {
    assert_vector_impl(v);
    v.clone()
}

pub fn vector_size_impl(v: &Vector) -> usize {
    assert_vector_impl(v);
    v.cols
}

pub fn vector_size_bytes_impl(v: &Vector) -> usize {
    std::mem::size_of::<f64>() * vector_size_impl(v)
}

pub fn is_vector_equal_impl(v: &Vector, w: &Vector) -> bool {
    assert_vector_impl(v);
    assert_vector_impl(w);
    if v.cols != w.cols {
        return false;
    }
    v.data.iter().zip(&w.data).all(|(a, b)| a == b)
}

pub fn set_vector_element_impl(v: &mut Vector, i: usize, s: f64) {
    assert_vector_impl(v);
    assert!(i < v.cols);
    v.data[i] = s;
}

pub fn get_vector_element_impl(v: &Vector, i: usize) -> f64 {
    assert_vector_impl(v);
    assert!(i < v.cols);
    v.data[i]
}

pub fn print_vector_impl(v: &Vector, include_indices: bool) {
    assert_vector_impl(v);
    for (i, value) in v.data.iter().enumerate() {
        if include_indices {
            print!("[{}] -> ", i);
        }
        print!("{:16.8} ", value);
    }
}

pub fn vector_magnitude_impl(v: &Vector) -> f64 {
    assert_vector_impl(v);
    let sum: f64 = v.data.iter().map(|value| value * value).sum();
    sum.sqrt()
}

pub fn is_unit_vector_impl(v: &Vector) -> bool {
    vector_magnitude_impl(v) == 1.0
}

pub fn is_vector_orthogonal_impl(v1: &Vector, v2: &Vector) -> bool {
    assert_vector_impl(v1);
    assert_vector_impl(v2);
    dot_product_impl(v1, v2) == 0.0
}

pub fn dot_product_impl(v: &Vector, w: &Vector) -> f64 {
    assert_vector_impl(v);
    assert_vector_impl(w);
    assert_eq!(v.cols, w.cols);
    v.data.iter().zip(&w.data).map(|(a, b)| a * b).sum()
}

pub fn cross_product_impl(v: &Vector, w: &Vector) -> Vector {
    assert_vector_impl(v);
    assert_vector_impl(w);
    assert_eq!(v.cols, 3);
    assert_eq!(w.cols, 3);
    Vector {
        cols: 3,
        data: vec![
            (v.data[1] * w.data[2]) - (v.data[2] * w.data[1]),
            (v.data[0] * w.data[2]) - (v.data[2] * w.data[0]),
            (v.data[0] * w.data[1]) - (v.data[1] * w.data[0]),
        ],
    }
}

pub fn vector_distance_impl(v: &Vector, w: &Vector) -> f64 {
    assert_vector_impl(v);
    assert_vector_impl(w);
    assert_eq!(v.cols, w.cols);
    let sum: f64 = v
        .data
        .iter()
        .zip(&w.data)
        .map(|(a, b)| {
            let diff = b - a;
            diff * diff
        })
        .sum();
    sum.sqrt()
}

pub fn add_vectors_impl(v1: &Vector, v2: &Vector) -> Vector {
    assert_vector_impl(v1);
    assert_vector_impl(v2);
    assert_eq!(v1.cols, v2.cols);
    Vector {
        cols: v1.cols,
        data: v1.data.iter().zip(&v2.data).map(|(a, b)| a + b).collect(),
    }
}

pub fn scale_vector_impl(v: &Vector, s: f64) -> Vector {
    assert_vector_impl(v);
    Vector {
        cols: v.cols,
        data: v.data.iter().map(|value| value * s).collect(),
    }
}

pub fn pow_vector_impl(v: &Vector, k: f64) -> Vector {
    assert_vector_impl(v);
    Vector {
        cols: v.cols,
        data: v.data.iter().map(|value| value.powf(k)).collect(),
    }
}

pub fn scalar_triple_product_impl(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    assert_eq!(v1.cols, 3);
    assert_eq!(v2.cols, 3);
    assert_eq!(v3.cols, 3);
    let cross = cross_product_impl(v2, v3);
    dot_product_impl(v1, &cross)
}
