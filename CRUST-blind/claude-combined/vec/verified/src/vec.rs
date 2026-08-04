// Constants
pub const VEC_VERSION: &str = "0.2.1";

// Function Declarations

/// Insert `value` at position `idx`, shifting subsequent elements to the right.
/// Returns 0 on success, -1 on failure (e.g., idx out of bounds).
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value: T) -> i32 {
    if idx > data.len() {
        return -1;
    }
    data.insert(idx, value);
    0
}

/// Remove `count` elements starting at index `start`. Subsequent elements
/// are shifted left.
pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let len = data.len();
    if start >= len {
        return;
    }
    let end = match start.checked_add(count) {
        Some(s) => s.min(len),
        None => len,
    };
    data.drain(start..end);
}

/// Remove `count` elements starting at index `start`, replacing them with
/// the last `count` elements (which are themselves then removed). Equivalent
/// to a memmove of the trailing `count` elements to position `start`,
/// followed by truncating the vector by `count`.
pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    let len = data.len();
    if count == 0 {
        return;
    }
    if start.checked_add(count).map_or(true, |s| s > len) {
        return;
    }
    let new_len = len - count;

    // Take ownership of the old buffer; use Option<T> so we can extract
    // elements out of arbitrary positions without requiring T: Default.
    let cap = data.capacity();
    let old: Vec<T> = std::mem::replace(data, Vec::new());
    let mut old_opts: Vec<Option<T>> = old.into_iter().map(Some).collect();
    let mut result: Vec<T> = Vec::with_capacity(cap);

    for i in 0..new_len {
        let src = if i < start {
            i
        } else if i < start + count {
            len - count + (i - start)
        } else {
            i
        };
        result.push(old_opts[src].take().unwrap());
    }

    *data = result;
}

/// Reserve capacity equal to the next power of two that is >= n.
/// Returns 0 on success, -1 on failure.
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

/// Reallocate the underlying buffer so its capacity becomes exactly `n`.
/// `n` must be >= len. Used internally to mirror `realloc(data, n * memsz)`.
fn realloc_exact<T>(data: &mut Vec<T>, n: usize) {
    let len = data.len();
    debug_assert!(n >= len);
    if n == data.capacity() {
        return;
    }
    let mut new_vec: Vec<T> = Vec::with_capacity(n);
    let old: Vec<T> = std::mem::replace(data, Vec::new());
    for item in old.into_iter() {
        new_vec.push(item);
    }
    *data = new_vec;
}

/// If the vector is full, double its capacity (or grow from 0 to 1).
/// Returns 0 on success, -1 on failure.
pub fn vec_expand<T>(data: &mut Vec<T>) -> i32 {
    let len = data.len();
    let cap = data.capacity();
    if len + 1 > cap {
        let n = if cap == 0 { 1 } else { cap << 1 };
        realloc_exact(data, n);
    }
    0
}

/// Reserve capacity to be at least `n`. If `n > capacity`, capacity grows
/// to exactly `n`. Returns 0 on success, -1 on failure.
pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    let cap = data.capacity();
    if n > cap {
        realloc_exact(data, n);
    }
    0
}

/// Shrink the capacity of the vector to exactly its current length. If the
/// length is zero, the buffer is freed. Returns 0 on success, -1 on failure.
pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    if data.is_empty() {
        // Match C: free buffer, set capacity = 0
        *data = Vec::new();
        return 0;
    }
    let len = data.len();
    realloc_exact(data, len);
    0
}

/// Swap the elements at positions `idx1` and `idx2`. No-op if equal.
pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 == idx2 {
        return;
    }
    data.swap(idx1, idx2);
}
