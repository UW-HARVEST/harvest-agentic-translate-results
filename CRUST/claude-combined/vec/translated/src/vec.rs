// Constants
pub const VEC_VERSION: &str = "0.2.1";

// Function Declarations
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value: T) -> i32 {
    data.insert(idx, value);
    0
}

pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    data.drain(start..start + count);
}

pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    let len = data.len();
    if count == 0 {
        return;
    }
    let new_len = len.saturating_sub(count);
    // Drain the last `count` elements out of the vector.
    let saved: Vec<T> = data.drain(new_len..).collect();
    // Determine how many of the saved elements actually end up visible
    // after the implicit length truncation that vec_swapsplice performs.
    let replace_count = count.min(new_len.saturating_sub(start));
    // Replace data[start..start + replace_count] with the first replace_count
    // saved elements. Any remaining saved elements are dropped (their slots
    // would have been past the new length in the C version).
    data.splice(
        start..start + replace_count,
        saved.into_iter().take(replace_count),
    );
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
        let n = if data.capacity() == 0 {
            1
        } else {
            data.capacity() * 2
        };
        let needed = n - data.len();
        data.reserve_exact(needed);
    }
    0
}

pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n > data.capacity() {
        // Use a fresh allocation to obtain a tighter capacity than
        // reserve_exact may give us.
        let mut new_vec: Vec<T> = Vec::with_capacity(n);
        new_vec.extend(data.drain(..));
        *data = new_vec;
    }
    0
}

pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    if data.is_empty() {
        *data = Vec::new();
        return 0;
    }
    let len = data.len();
    if data.capacity() != len {
        let mut new_vec: Vec<T> = Vec::with_capacity(len);
        new_vec.extend(data.drain(..));
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
