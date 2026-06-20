// Constants
pub const VEC_VERSION: &str = "0.2.1";

fn try_reserve_exact_to<T>(data: &mut Vec<T>, target_capacity: usize) -> i32 {
    if target_capacity <= data.capacity() {
        return 0;
    }

    match data.try_reserve_exact(target_capacity - data.capacity()) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

// Function Declarations
pub fn vec_insert<T>(data: &mut Vec<T>, idx: usize, value: T) -> i32 {
    if idx > data.len() {
        return -1;
    }
    if vec_expand(data) != 0 {
        return -1;
    }
    data.insert(idx, value);
    0
}
pub fn vec_splice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 || start >= data.len() {
        return;
    }
    let end = start.saturating_add(count).min(data.len());
    data.drain(start..end);
}
pub fn vec_swapsplice<T>(data: &mut Vec<T>, start: usize, count: usize) {
    if count == 0 || start >= data.len() {
        return;
    }

    let remove_count = count.min(data.len() - start);
    let tail_start = data.len() - remove_count;

    if start >= tail_start {
        data.truncate(tail_start);
        return;
    }

    let mut tail = data.split_off(tail_start);
    let replace_count = remove_count.min(tail_start - start);
    for (dst, src) in data[start..start + replace_count]
        .iter_mut()
        .zip(tail.drain(..replace_count))
    {
        *dst = src;
    }
}
pub fn vec_reserve_po2<T>(data: &mut Vec<T>, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    match n.checked_next_power_of_two() {
        Some(target) => vec_reserve(data, target),
        None => -1,
    }
}
pub fn vec_expand<T>(data: &mut Vec<T>) -> i32 {
    if data.len() < data.capacity() {
        return 0;
    }

    let target_capacity = if data.capacity() == 0 {
        1
    } else {
        match data.capacity().checked_mul(2) {
            Some(capacity) => capacity,
            None => return -1,
        }
    };

    try_reserve_exact_to(data, target_capacity)
}
pub fn vec_reserve<T>(data: &mut Vec<T>, n: usize) -> i32 {
    try_reserve_exact_to(data, n)
}
pub fn vec_compact<T>(data: &mut Vec<T>) -> i32 {
    if data.is_empty() {
        *data = Vec::new();
        return 0;
    }

    let compacted = std::mem::take(data).into_boxed_slice().into_vec();
    *data = compacted;
    0
}
pub fn vec_swap<T>(data: &mut Vec<T>, idx1: usize, idx2: usize) {
    if idx1 == idx2 || idx1 >= data.len() || idx2 >= data.len() {
        return;
    }
    data.swap(idx1, idx2);
}
