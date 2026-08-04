// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;
// Structs
/// A safe, idiomatic representation of the original `struct vector`.
/// Stores raw bytes (count * esize). Saves are stored in a parallel vector.
#[derive(Debug, Default, Clone)]
pub struct Vector {
    pub data: Vec<u8>,
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
    pub esize: usize,
    /// Saves are stored as snapshots of (pindex, rindex, mindex, count, flags, esize).
    /// We store one snapshot per save in this Vec. Saves do not preserve the data array.
    pub saves: Vec<VectorSave>,
}

#[derive(Debug, Default, Clone)]
pub struct VectorSave {
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
    pub esize: usize,
}

fn vector_in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn vector_in_bounds_for_pop(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.mindex
}

fn vector_resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_mindex = start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
    vector.mindex = start_index + total_elements;
    let bytes = (new_mindex as usize) * vector.esize;
    if vector.data.len() < bytes {
        vector.data.resize(bytes, 0);
    }
}

fn vector_resize_for(vector: &mut Vector, total_elements: i32) {
    let r = vector.rindex;
    vector_resize_for_index(vector, r, total_elements);
}

fn vector_resize(vector: &mut Vector) {
    vector_resize_for(vector, 0);
}

fn ensure_data_capacity(vector: &mut Vector) {
    let needed = (vector.mindex as usize) * vector.esize;
    if vector.data.len() < needed {
        vector.data.resize(needed, 0);
    }
}

// Function Declarations
/// Creates a new vector with elements of size `esize`.
pub fn vector_create(esize: usize) -> Vector {
    Vector {
        data: vec![0u8; esize * VECTOR_ELEMENT_INCREMENT],
        pindex: 0,
        rindex: 0,
        mindex: VECTOR_ELEMENT_INCREMENT as i32,
        count: 0,
        flags: 0,
        esize,
        saves: Vec::new(),
    }
}
/// Frees the given vector (in Rust, typically done by dropping).
pub fn vector_free(_vector: Vector) {
    // No-op; dropped automatically.
}
/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 {
        return None;
    }
    let start = (index as usize) * vector.esize;
    let end = start + vector.esize;
    if end > vector.data.len() {
        // Stretch capacity as needed; matches C behaviour where vector_at
        // returns a pointer into raw memory regardless of bounds.
        vector.data.resize(end, 0);
    }
    Some(&mut vector.data[start..end])
}
/// Returns a reference to the element at the given index for peek operations, if in range.
pub fn vector_peek_ptr_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 || index > vector.count {
        return None;
    }
    vector_at(vector, index)
}
/// Returns a reference to the next element to peek without incrementing the internal pointer.
pub fn vector_peek_no_increment(vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(vector, vector.pindex) {
        return None;
    }
    let pi = vector.pindex;
    vector_at(vector, pi)
}
/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(vector, vector.pindex) {
        return None;
    }
    let pi = vector.pindex;
    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    vector_at(vector, pi)
}
/// Returns a reference to the element at the given index without changing the peek pointer.
pub fn vector_peek_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(vector, index) {
        return None;
    }
    vector_at(vector, index)
}
/// Sets a flag in the vector.
pub fn vector_set_flag(vector: &mut Vector, flag: i32) {
    vector.flags |= flag;
}
/// Unsets a flag in the vector.
pub fn vector_unset_flag(vector: &mut Vector, flag: i32) {
    vector.flags &= !flag;
}
/// Removes the last peeked element from the vector if needed.
pub fn vector_pop_last_peek(vector: &mut Vector) {
    assert!(vector.pindex >= 1);
    let idx = vector.pindex - 1;
    vector_pop_at(vector, idx);
}
/// Returns a reference to the last pushed element for peek purposes.
/// Mirrors `vector_peek_ptr` (operates on a vector of pointers / encoded indices).
pub fn vector_peek_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_peek(vector)
}
/// Sets the peek pointer to the given index.
pub fn vector_set_peek_pointer(vector: &mut Vector, index: i32) {
    vector.pindex = index;
}
/// Sets the peek pointer to the end of the vector.
pub fn vector_set_peek_pointer_end(vector: &mut Vector) {
    vector.pindex = vector.rindex - 1;
}
/// Pushes a new element (pointed to by `elem`) onto the vector.
pub fn vector_push(vector: &mut Vector, elem: &[u8]) {
    let idx = vector.rindex;
    let start = (idx as usize) * vector.esize;
    let end = start + vector.esize;
    if end > vector.data.len() {
        vector.data.resize(end, 0);
    }
    let copy_len = elem.len().min(vector.esize);
    vector.data[start..start + copy_len].copy_from_slice(&elem[..copy_len]);
    if copy_len < vector.esize {
        for i in start + copy_len..end {
            vector.data[i] = 0;
        }
    }
    vector.rindex += 1;
    vector.count += 1;

    if vector.rindex >= vector.mindex {
        vector_resize(vector);
        ensure_data_capacity(vector);
    }
}
/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    vector_shift_right(vector, index, 1);
    let start = (index as usize) * vector.esize;
    let end = start + vector.esize;
    if end > vector.data.len() {
        vector.data.resize(end, 0);
    }
    let copy_len = ptr.len().min(vector.esize);
    vector.data[start..start + copy_len].copy_from_slice(&ptr[..copy_len]);
    if copy_len < vector.esize {
        for i in start + copy_len..end {
            vector.data[i] = 0;
        }
    }
}
/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    vector.rindex -= 1;
    vector.count -= 1;
    assert!(vector_in_bounds_for_pop(vector, vector.rindex));
}
/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    let pi = vector.pindex;
    vector_pop_at(vector, pi);
}
/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    assert!(vector_in_bounds_for_pop(vector, idx));
    vector_at(vector, idx)
}
/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    if !vector_in_bounds_for_at(vector, idx) {
        return None;
    }
    vector_at(vector, idx)
}
/// Returns a reference to the last element in the vector for pointer usage.
/// In the C variant this dereferences a `void**` — we return the last slot, as our vector
/// already stores the encoded pointer/value as element bytes.
pub fn vector_back_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}
/// Returns a reference to the last element or null, specialized for pointer usage.
pub fn vector_back_ptr_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back_or_null(vector)
}
/// Returns a string slice representation from the vector if it contains text data.
pub fn vector_string(vec: &Vector) -> Option<&str> {
    let bytes = &vec.data;
    // Find first NUL.
    let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..nul]).ok()
}
/// Checks if the vector is empty.
pub fn vector_empty(vector: &Vector) -> bool {
    vector.count == 0
}
/// Clears the vector contents.
pub fn vector_clear(vector: &mut Vector) {
    while vector.count > 0 {
        vector_pop(vector);
    }
}
/// Returns the count of elements in the vector.
pub fn vector_count(vector: &Vector) -> i32 {
    vector.count
}
/// Reads data into the vector from a file pointer (stub; not fully safe in typical Rust).
pub fn vector_fread(vector: &mut Vector, _amount: i32, mut fp: std::fs::File) -> i32 {
    use std::io::Read;
    let mut buf = vec![0u8; vector.esize];
    while let Ok(n) = fp.read(&mut buf) {
        if n == 0 {
            break;
        }
        let read_amount: usize = n;
        let bytes = read_amount.to_le_bytes();
        let push_slice: Vec<u8> = bytes.iter().copied().chain(std::iter::repeat(0)).take(vector.esize).collect();
        vector_push(vector, &push_slice);
    }
    0
}
/// Returns a reference to the underlying data of the vector.
pub fn vector_data_ptr(vector: &Vector) -> &[u8] {
    &vector.data
}
/// Inserts data from `vector_src` into `vector_dst` at `dst_index`.
pub fn vector_insert(vector_dst: &mut Vector, vector_src: &Vector, dst_index: i32) -> i32 {
    if vector_dst.esize != vector_src.esize {
        return -1;
    }
    let count = vector_count(vector_src);
    let total_bytes = (count as usize) * vector_src.esize;
    let src_data: Vec<u8> = vector_src.data[..total_bytes].to_vec();
    vector_push_multiple_at(vector_dst, dst_index, &src_data, count);
    0
}

fn vector_push_multiple_at(vector: &mut Vector, dst_index: i32, ptr: &[u8], total: i32) {
    vector_shift_right(vector, dst_index, total);
    let start = (dst_index as usize) * vector.esize;
    let total_bytes = (total as usize) * vector.esize;
    if start + total_bytes > vector.data.len() {
        vector.data.resize(start + total_bytes, 0);
    }
    vector.data[start..start + total_bytes].copy_from_slice(&ptr[..total_bytes]);
}

fn vector_shift_right(vector: &mut Vector, index: i32, amount: i32) {
    if index < vector.rindex {
        vector_shift_right_in_bounds(vector, index, amount);
        return;
    }
    vector_stretch(vector, index + amount);
    vector_shift_right_in_bounds_no_increment(vector, index, amount);
}

fn vector_shift_right_in_bounds(vector: &mut Vector, index: i32, amount: i32) {
    vector_shift_right_in_bounds_no_increment(vector, index, amount);
    vector.rindex += amount;
    vector.count += amount;
}

fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    vector_resize_for_index(vector, index, amount);
    let eindex = index + amount;
    let elements_to_move = vector.count - index;
    let bytes_to_move = (elements_to_move.max(0) as usize) * vector.esize;
    let src_start = (index as usize) * vector.esize;
    let dst_start = (eindex as usize) * vector.esize;
    let needed = dst_start + bytes_to_move;
    if vector.data.len() < needed {
        vector.data.resize(needed, 0);
    }
    if bytes_to_move > 0 {
        // Safe overlap-aware copy: copy_within handles overlap correctly.
        vector.data.copy_within(src_start..src_start + bytes_to_move, dst_start);
    }
    let zero_start = src_start;
    let zero_end = src_start + (amount as usize) * vector.esize;
    let zero_end = zero_end.min(vector.data.len());
    for i in zero_start..zero_end {
        vector.data[i] = 0;
    }
}

fn vector_stretch(vector: &mut Vector, index: i32) {
    if index < vector.rindex {
        return;
    }
    vector_resize_for_index(vector, index, 0);
    vector.count = index;
    vector.rindex = index;
    ensure_data_capacity(vector);
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    // Not directly applicable in our safe model; return -1 as "not found".
    -1
}
/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let n = vector.count;
    let esize = vector.esize;
    for i in 0..n {
        let start = (i as usize) * esize;
        let end = start + esize;
        if end <= vector.data.len() && &vector.data[start..end] == &val[..esize.min(val.len())] {
            // Compare the truncated value bytes with the element.
            let cmp_len = esize.min(val.len());
            if vector.data[start..start + cmp_len] == val[..cmp_len] {
                vector_pop_at(vector, i);
                return i;
            }
        }
    }
    -1
}
/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    let esize = vector.esize;
    let dst_start = (index as usize) * esize;
    let next_start = dst_start + esize;
    let end_pos = (vector.rindex as usize) * esize;
    if next_start < end_pos {
        let total = end_pos - next_start;
        if vector.data.len() >= end_pos {
            vector.data.copy_within(next_start..next_start + total, dst_start);
        }
    }
    vector.count -= 1;
    vector.rindex -= 1;
}
/// Moves the peek pointer to the back of the vector.
pub fn vector_peek_back(vector: &mut Vector) {
    vector.pindex -= 1;
}
/// Returns the current peek index.
pub fn vector_current_index(vector: &Vector) -> i32 {
    vector.rindex
}
/// Saves the current state of the vector for future restore.
pub fn vector_save(vector: &mut Vector) {
    let snap = VectorSave {
        pindex: vector.pindex,
        rindex: vector.rindex,
        mindex: vector.mindex,
        count: vector.count,
        flags: vector.flags,
        esize: vector.esize,
    };
    vector.saves.push(snap);
}
/// Restores a previously saved state of the vector.
pub fn vector_restore(vector: &mut Vector) {
    if let Some(snap) = vector.saves.pop() {
        vector.pindex = snap.pindex;
        vector.rindex = snap.rindex;
        vector.mindex = snap.mindex;
        vector.count = snap.count;
        vector.flags = snap.flags;
        vector.esize = snap.esize;
    }
}
/// Removes saved states from the vector.
pub fn vector_save_purge(vector: &mut Vector) {
    vector.saves.pop();
}
/// Returns the size of each element in the vector.
pub fn vector_element_size(vector: &Vector) -> usize {
    vector.esize
}
/// Clones the vector into a new one.
pub fn vector_clone(vector: &Vector) -> Vector {
    Vector {
        data: vector.data.clone(),
        pindex: vector.pindex,
        rindex: vector.rindex,
        mindex: vector.mindex,
        count: vector.count,
        flags: vector.flags,
        esize: vector.esize,
        saves: Vec::new(),
    }
}
