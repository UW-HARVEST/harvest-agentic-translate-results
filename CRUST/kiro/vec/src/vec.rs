// Constants
pub const VEC_VERSION: &str = "0.2.1";
// Function Declarations
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value: T) -> i32 {
    data.insert(idx, value);
    0
}
pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    data.drain(start..start + count);
}
pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    let len = data.len();
    for i in 0..count {
        data.swap(start + i, len - count + i);
    }
    data.truncate(len - count);
}
pub fn vec_reserve_po2<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n == 0 { return 0; }
    let mut n2 = 1;
    while n2 < n { n2 <<= 1; }
    vec_reserve(data, n2)
}
pub fn vec_expand<T>(data: &mut Vec<T>) -> i32 {
    data.reserve(1);
    0
}
pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n > data.capacity() {
        let additional = n - data.len();
        data.reserve_exact(additional);
        // Rust's reserve_exact may allocate slightly more; force exact capacity
        if data.capacity() > n {
            data.shrink_to(n);
        }
    }
    0
}
pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    data.shrink_to_fit();
    0
}
pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 != idx2 {
        data.swap(idx1, idx2);
    }
}
