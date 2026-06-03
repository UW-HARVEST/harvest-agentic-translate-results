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
    // Replace `count` elements starting at `start` with the last `count`
    // elements, then truncate by `count`. Implemented via swaps so non-Copy
    // types are dropped correctly during truncate.
    let len = data.len();
    for i in 0..count {
        data.swap(start + i, len - count + i);
    }
    data.truncate(len - count);
}
pub fn vec_reserve_po2<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    let mut n2: usize = 1;
    while n2 < n {
        n2 <<= 1;
    }
    vec_reserve(data, n2)
}
pub fn vec_expand<T>(data: &mut Vec<T>) -> i32 {
    if data.len() + 1 > data.capacity() {
        let new_cap = if data.capacity() == 0 {
            1
        } else {
            data.capacity() * 2
        };
        let mut new_vec: Vec<T> = Vec::with_capacity(new_cap);
        new_vec.append(data);
        *data = new_vec;
    }
    0
}
pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n > data.capacity() {
        let mut new_vec: Vec<T> = Vec::with_capacity(n);
        new_vec.append(data);
        *data = new_vec;
    }
    0
}
pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    if data.is_empty() {
        *data = Vec::new();
    } else {
        let mut new_vec: Vec<T> = Vec::with_capacity(data.len());
        new_vec.append(data);
        *data = new_vec;
    }
    0
}
pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 == idx2 {
        return;
    }
    data.swap(idx1, idx2);
}
