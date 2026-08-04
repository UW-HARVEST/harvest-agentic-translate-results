// Constants
pub const VEC_VERSION: &str = "0.2.1";

// Function Declarations

/// Inserts `value` at index `idx`, shifting later elements one position to the right.
/// Returns 0 on success.
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value: T) -> i32 {
    data.insert(idx, value);
    0
}

/// Removes `count` elements starting at index `start`.
pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let end = start + count;
    data.drain(start..end);
}

/// Replaces `count` elements starting at index `start` with the last `count` elements
/// of the vector, then truncates by `count`.
pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let len = data.len();
    let src_start = len - count;
    // Move the last `count` elements into positions [start, start + count).
    // Equivalent to memmove in the C implementation.
    for i in 0..count {
        data.swap(start + i, src_start + i);
    }
    data.truncate(len - count);
}

/// Reserves capacity rounded up to the next power of two greater than or equal to `n`.
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

/// Doubles the capacity of the vector when there is no remaining space for an additional element.
pub fn vec_expand<T>(data: &mut Vec<T>) -> i32 {
    if data.len() + 1 > data.capacity() {
        let new_cap = if data.capacity() == 0 { 1 } else { data.capacity() << 1 };
        let additional = new_cap - data.len();
        data.reserve_exact(additional);
    }
    0
}

/// Reserves at least enough capacity to store `n` elements total.
pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n > data.capacity() {
        let additional = n - data.len();
        data.reserve_exact(additional);
    }
    0
}

/// Shrinks the capacity of the vector to match its current length.
pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    data.shrink_to_fit();
    0
}

/// Swaps the elements at positions `idx1` and `idx2`.
pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 == idx2 {
        return;
    }
    data.swap(idx1, idx2);
}
