use crate::linear_algebra::Vector;

pub fn assert_vector_impl(v: &Vector) -> bool {
    assert!(v.data.len() == v.cols);
    true
}

pub fn new_vector_impl(d: &[f64], cols: usize) -> Vector {
    assert!(cols > 0);
    let mut v = null_vector_impl(cols);
    for i in 0..cols {
        v.data[i] = d[i];
    }
    v
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
    for i in 0..v.cols {
        v.data[i] = n;
    }
}

pub fn delete_vector_impl(_v: Vector) {
    // memory is freed automatically in Rust
}

pub fn copy_vector_impl(v: &Vector) -> Vector {
    assert_vector_impl(v);
    let mut c = zero_vector_impl(v.cols);
    for i in 0..v.cols {
        c.data[i] = v.data[i];
    }
    c
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
    for i in 0..v.cols {
        if v.data[i] != w.data[i] {
            return false;
        }
    }
    true
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
    for i in 0..v.cols {
        if include_indices {
            print!("[{}] -> ", i);
        }
        print!("{:16.8} ", v.data[i]);
    }
}

pub fn vector_magnitude_impl(v: &Vector) -> f64 {
    assert_vector_impl(v);
    let mut sum = 0.0;
    for i in 0..v.cols {
        sum += v.data[i] * v.data[i];
    }
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
    assert!(v.cols == w.cols);
    let mut dp = 0.0;
    for i in 0..v.cols {
        dp += v.data[i] * w.data[i];
    }
    dp
}

pub fn cross_product_impl(v: &Vector, w: &Vector) -> Vector {
    assert_vector_impl(v);
    assert_vector_impl(w);
    assert!(v.cols == 3 && w.cols == 3);
    let mut c = null_vector_impl(3);
    c.data[0] = v.data[1] * w.data[2] - v.data[2] * w.data[1];
    c.data[1] = v.data[0] * w.data[2] - v.data[2] * w.data[0];
    c.data[2] = v.data[0] * w.data[1] - v.data[1] * w.data[0];
    c
}

pub fn vector_distance_impl(v: &Vector, w: &Vector) -> f64 {
    assert_vector_impl(v);
    assert_vector_impl(w);
    assert!(v.cols == w.cols);
    let mut d = 0.0;
    for i in 0..v.cols {
        d += (w.data[i] - v.data[i]) * (w.data[i] - v.data[i]);
    }
    d.sqrt()
}

pub fn add_vectors_impl(v: &Vector, w: &Vector) -> Vector {
    assert_vector_impl(v);
    assert_vector_impl(w);
    assert!(v.cols == w.cols);
    let mut sum = null_vector_impl(v.cols);
    for i in 0..v.cols {
        sum.data[i] = v.data[i] + w.data[i];
    }
    sum
}

pub fn scale_vector_impl(v: &Vector, s: f64) -> Vector {
    assert_vector_impl(v);
    let mut scaled = null_vector_impl(v.cols);
    for i in 0..v.cols {
        scaled.data[i] = v.data[i] * s;
    }
    scaled
}

pub fn pow_vector_impl(v: &Vector, k: f64) -> Vector {
    assert_vector_impl(v);
    let mut p = null_vector_impl(v.cols);
    for i in 0..v.cols {
        p.data[i] = v.data[i].powf(k);
    }
    p
}

pub fn scalar_triple_product_impl(v1: &Vector, v2: &Vector, v3: &Vector) -> f64 {
    assert!(v1.cols == 3 && v2.cols == 3 && v3.cols == 3);
    let cross = cross_product_impl(v2, v3);
    dot_product_impl(v1, &cross)
}
