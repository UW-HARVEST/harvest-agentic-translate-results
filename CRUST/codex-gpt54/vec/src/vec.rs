// Constants
pub const VEC_VERSION: &str = "0.2.1";

fn reallocate_with_capacity<T>(data: &mut Vec<T>, capacity: usize) {
    let old = std::mem::take(data);
    let mut replacement = Vec::with_capacity(capacity);
    replacement.extend(old);
    *data = replacement;
}

// Function Declarations
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value:T) -> i32 {
    let _ = vec_expand(data);
    data.insert(idx, value);
    0
}
pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    data.drain(start..start + count);
}
pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    let len = data.len();
    let new_len = len - count;
    let replaced = new_len.saturating_sub(start).min(count);

    let old: Vec<Option<T>> = std::mem::take(data).into_iter().map(Some).collect();
    let mut result = Vec::with_capacity(new_len);

    let mut old = old;
    for item in old.iter_mut().take(start) {
        result.push(item.take().unwrap());
    }
    for item in old.iter_mut().skip(len - count).take(replaced) {
        result.push(item.take().unwrap());
    }
    for item in old
        .iter_mut()
        .skip(start + count)
        .take(new_len.saturating_sub(start + count))
    {
        result.push(item.take().unwrap());
    }

    *data = result;
}
pub fn vec_reserve_po2<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }

    let mut capacity = 1usize;
    while capacity < n {
        capacity <<= 1;
    }

    vec_reserve(data, capacity)
}
pub fn vec_expand<T>(data: &mut Vec<T>) -> i32 {
    if data.len() + 1 > data.capacity() {
        let capacity = if data.capacity() == 0 {
            1
        } else {
            data.capacity() << 1
        };
        return vec_reserve(data, capacity);
    }

    0
}
pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n > data.capacity() {
        reallocate_with_capacity(data, n);
    }

    0
}
pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    if data.is_empty() {
        *data = Vec::new();
        return 0;
    }

    let boxed = std::mem::take(data).into_boxed_slice();
    *data = boxed.into_vec();
    0
}
pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 != idx2 {
        data.swap(idx1, idx2);
    }
}
