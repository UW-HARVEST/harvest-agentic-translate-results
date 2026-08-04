// Constants
pub const VEC_VERSION: &str = "0.2.1";

// Function Declarations

/// Inserts `value` at position `idx` in `data`, shifting subsequent elements
/// to the right. Returns 0 on success.
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value: T) -> i32 {
    data.insert(idx, value);
    0
}

/// Removes `count` elements starting at index `start` from `data`.
pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let end = start + count;
    if end > data.len() {
        return;
    }
    data.drain(start..end);
}

/// Replaces `count` elements starting at index `start` with the last `count`
/// elements of the vector, then truncates the vector by `count`. Mirrors the
/// behavior of the original C `vec_swapsplice_`.
pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let len = data.len();
    if count > len || start + count > len {
        return;
    }
    let tail_start = len - count;
    // Copy last `count` items into positions starting at `start`.
    // We need to be careful when ranges overlap. Use Vec::swap for each
    // pair, then truncate. Since the elements at the tail will be removed
    // anyway, swapping is fine and preserves the values that need to be
    // moved into `start..start+count`.
    for i in 0..count {
        data.swap(start + i, tail_start + i);
    }
    data.truncate(tail_start);
}

/// Ensures the vector has capacity for at least the next power of two greater
/// than or equal to `n` elements. Returns 0 on success.
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

/// Ensures there is room for at least one more element. If the current
/// capacity is insufficient, the capacity is doubled (or set to 1 if zero).
/// Returns 0 on success.
pub fn vec_expand<T>(data: &mut Vec<T>) -> i32 {
    if data.len() + 1 > data.capacity() {
        let n = if data.capacity() == 0 {
            1
        } else {
            data.capacity() << 1
        };
        vec_reserve(data, n)
    } else {
        0
    }
}

/// Ensures the capacity of the vector is exactly `n` if `n` is greater than
/// the current capacity. Returns 0 on success.
pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n > data.capacity() {
        // Move existing elements into a freshly allocated vector with the
        // requested capacity to match the C semantics of setting capacity to
        // exactly `n`.
        let old = std::mem::replace(data, Vec::with_capacity(n));
        for item in old {
            data.push(item);
        }
    }
    0
}

/// Shrinks the capacity of the vector to match its current length. Returns 0
/// on success.
pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    if data.is_empty() {
        *data = Vec::new();
    } else {
        data.shrink_to_fit();
    }
    0
}

/// Swaps the elements at `idx1` and `idx2` in `data`.
pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 == idx2 {
        return;
    }
    data.swap(idx1, idx2);
}
