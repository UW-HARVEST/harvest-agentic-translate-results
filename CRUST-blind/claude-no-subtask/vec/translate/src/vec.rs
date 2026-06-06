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
    let end = start + count;
    if end > data.len() {
        return;
    }
    data.drain(start..end);
}
pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let len = data.len();
    if start + count > len {
        return;
    }
    // Move the last `count` elements into positions `start..start+count`
    // by swapping. The swapped-out elements will end up at the tail and
    // be dropped when we truncate.
    for i in 0..count {
        let src = len - count + i;
        let dst = start + i;
        if src != dst {
            data.swap(dst, src);
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
    if data.len() + 1 > data.capacity() {
        let new_cap = if data.capacity() == 0 {
            1
        } else {
            data.capacity().saturating_mul(2)
        };
        let additional = new_cap - data.len();
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
    if idx1 < data.len() && idx2 < data.len() {
        data.swap(idx1, idx2);
    }
}
