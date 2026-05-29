// Constants
pub const VEC_VERSION: &str = "0.2.1";

// Function Declarations
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value: T) -> i32 {
    if idx > data.len() {
        return -1;
    }
    data.insert(idx, value);
    0
}

pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    if start >= data.len() {
        return;
    }
    let end = (start + count).min(data.len());
    data.drain(start..end);
}

pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let len = data.len();
    if start >= len || count > len {
        return;
    }
    // Mimic C's memmove: copy the last `count` elements to position `start`,
    // then shrink length by `count`.
    let src_start = len - count;
    if start != src_start {
        for i in 0..count {
            // After this swap loop, data[start..start+count] holds the values
            // that were in data[src_start..len]. The back portion gets the old
            // front values, which are discarded by the subsequent truncate.
            data.swap(start + i, src_start + i);
        }
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
    let len = data.len();
    let cap = data.capacity();
    if len + 1 > cap {
        let new_cap = if cap == 0 { 1 } else { cap << 1 };
        let additional = new_cap - len;
        data.reserve_exact(additional);
    }
    0
}

pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n > data.capacity() {
        let additional = n - data.len();
        data.reserve_exact(additional);
    }
    0
}

pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    data.shrink_to_fit();
    0
}

pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 == idx2 {
        return;
    }
    data.swap(idx1, idx2);
}
