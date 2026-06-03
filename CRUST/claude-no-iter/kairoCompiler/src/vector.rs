// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

// Structs
/// A safe representation of the original `struct vector`.
/// Stores raw bytes; each element is `esize` bytes wide.
#[derive(Debug, Default, Clone)]
pub struct Vector {
    pub data: Vec<u8>,
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
    pub esize: usize,
    pub saves: Vec<VectorState>,
}

/// A snapshot of the vector's state used by save/restore.
#[derive(Debug, Default, Clone)]
pub struct VectorState {
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
    // Dropped automatically.
}

fn ensure_capacity(vector: &mut Vector, total_elements: i32) {
    let needed = (vector.rindex + total_elements) as usize;
    if needed >= vector.mindex as usize {
        let new_size = (vector.rindex as usize
            + total_elements as usize
            + VECTOR_ELEMENT_INCREMENT)
            * vector.esize;
        if vector.data.len() < new_size {
            vector.data.resize(new_size, 0);
        }
        vector.mindex = (vector.rindex + total_elements + VECTOR_ELEMENT_INCREMENT as i32) as i32;
    }
}

fn ensure_capacity_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_mindex = start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
    let new_size = (new_mindex as usize) * vector.esize;
    if vector.data.len() < new_size {
        vector.data.resize(new_size, 0);
    }
    vector.mindex = new_mindex;
}

fn elem_offset(vector: &Vector, index: i32) -> usize {
    (index as usize) * vector.esize
}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 {
        return None;
    }
    let off = elem_offset(vector, index);
    let end = off + vector.esize;
    if end > vector.data.len() {
        return None;
    }
    Some(&mut vector.data[off..end])
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
    let pindex = vector.pindex;
    vector_at(vector, pindex)
}

/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(vector, vector.pindex) {
        return None;
    }
    let pindex = vector.pindex;
    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    vector_at(vector, pindex)
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

/// Returns a copy of the bytes at the next peek position, advancing the pointer.
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
    ensure_capacity(vector, 1);
    let off = elem_offset(vector, vector.rindex);
    let esize = vector.esize;
    let copy_len = elem.len().min(esize);
    if vector.data.len() < off + esize {
        vector.data.resize(off + esize, 0);
    }
    vector.data[off..off + copy_len].copy_from_slice(&elem[..copy_len]);
    if copy_len < esize {
        for b in &mut vector.data[off + copy_len..off + esize] {
            *b = 0;
        }
    }
    vector.rindex += 1;
    vector.count += 1;
    if vector.rindex >= vector.mindex {
        ensure_capacity(vector, 0);
    }
}

/// Shifts elements right within bounds without incrementing rindex.
fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    ensure_capacity_for_index(vector, index, amount);
    let esize = vector.esize;
    let count = vector.count;
    let bytes_to_move = ((count - index) as usize) * esize;
    let src_off = (index as usize) * esize;
    let dst_off = ((index + amount) as usize) * esize;
    if vector.data.len() < dst_off + bytes_to_move {
        vector.data.resize(dst_off + bytes_to_move, 0);
    }
    if bytes_to_move > 0 {
        // Copy bytes manually to avoid overlap issues
        let mut i = bytes_to_move;
        while i > 0 {
            i -= 1;
            vector.data[dst_off + i] = vector.data[src_off + i];
        }
    }
    // Zero out the gap
    for b in &mut vector.data[src_off..src_off + (amount as usize) * esize] {
        *b = 0;
    }
}

fn vector_shift_right_in_bounds(vector: &mut Vector, index: i32, amount: i32) {
    vector_shift_right_in_bounds_no_increment(vector, index, amount);
    vector.rindex += amount;
    vector.count += amount;
}

fn vector_stretch(vector: &mut Vector, index: i32) {
    if index < vector.rindex {
        return;
    }
    ensure_capacity_for_index(vector, index, 0);
    vector.count = index;
    vector.rindex = index;
}

fn vector_shift_right(vector: &mut Vector, index: i32, amount: i32) {
    if index < vector.rindex {
        vector_shift_right_in_bounds(vector, index, amount);
        return;
    }
    vector_stretch(vector, index + amount);
    vector_shift_right_in_bounds_no_increment(vector, index, amount);
}

/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    vector_shift_right(vector, index, 1);
    let off = elem_offset(vector, index);
    let esize = vector.esize;
    let copy_len = ptr.len().min(esize);
    if vector.data.len() < off + esize {
        vector.data.resize(off + esize, 0);
    }
    vector.data[off..off + copy_len].copy_from_slice(&ptr[..copy_len]);
}

/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    vector.rindex -= 1;
    vector.count -= 1;
    assert!(vector_in_bounds_for_pop(vector, vector.rindex));
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    let idx = vector.pindex;
    vector_pop_at(vector, idx);
}

/// Returns a reference to the last element in the vector (asserts in-bounds).
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    if !vector_in_bounds_for_pop(vector, idx) {
        return None;
    }
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
pub fn vector_back_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}

/// Returns a reference to the last element or null, specialized for pointer usage.
pub fn vector_back_ptr_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back_or_null(vector)
}

/// Returns a string slice representation from the vector if it contains text data.
pub fn vector_string(vec: &Vector) -> Option<&str> {
    let total = (vec.count as usize) * vec.esize;
    let total = total.min(vec.data.len());
    let bytes = &vec.data[..total];
    // Stop at the first null terminator
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).ok()
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
    let mut buf = [0u8; 1];
    while let Ok(n) = fp.read(&mut buf) {
        if n == 0 {
            break;
        }
        vector_push(vector, &buf[..n]);
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
    let total = vector_count(vector_src);
    let total_bytes = (total as usize) * vector_src.esize;
    vector_shift_right(vector_dst, dst_index, total);
    let dst_off = elem_offset(vector_dst, dst_index);
    if vector_dst.data.len() < dst_off + total_bytes {
        vector_dst.data.resize(dst_off + total_bytes, 0);
    }
    vector_dst.data[dst_off..dst_off + total_bytes]
        .copy_from_slice(&vector_src.data[..total_bytes]);
    0
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    // Pointer-based access has no safe equivalent; return -1.
    -1
}

/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let esize = vector.esize;
    let count = vector.count;
    for i in 0..count {
        let off = (i as usize) * esize;
        if &vector.data[off..off + esize.min(val.len())] == &val[..esize.min(val.len())] {
            vector_pop_at(vector, i);
            return i;
        }
    }
    -1
}

/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if !vector_in_bounds_for_at(vector, index) {
        return;
    }
    let esize = vector.esize;
    let dst = (index as usize) * esize;
    let src = dst + esize;
    let end = (vector.rindex as usize) * esize;
    if src < end {
        let len = end - src;
        // Move bytes from [src..end] to [dst..dst+len]
        for i in 0..len {
            vector.data[dst + i] = vector.data[src + i];
        }
    }
    vector.count -= 1;
    vector.rindex -= 1;
}

/// Moves the peek pointer back by one.
pub fn vector_peek_back(vector: &mut Vector) {
    vector.pindex -= 1;
}

/// Returns the current rindex (matches the C semantics of vector_current_index).
pub fn vector_current_index(vector: &Vector) -> i32 {
    vector.rindex
}

/// Saves the current state of the vector for future restore.
pub fn vector_save(vector: &mut Vector) {
    vector.saves.push(VectorState {
        pindex: vector.pindex,
        rindex: vector.rindex,
        mindex: vector.mindex,
        count: vector.count,
        flags: vector.flags,
        esize: vector.esize,
    });
}

/// Restores a previously saved state of the vector.
pub fn vector_restore(vector: &mut Vector) {
    if let Some(state) = vector.saves.pop() {
        vector.pindex = state.pindex;
        vector.rindex = state.rindex;
        vector.mindex = state.mindex;
        vector.count = state.count;
        vector.flags = state.flags;
        vector.esize = state.esize;
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
