// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

// Structs
/// A safe, idiomatic representation of the original `struct vector`.
/// Stores raw bytes, indexed by element size. Mirrors the C vector layout.
#[derive(Debug, Default, Clone)]
pub struct Vector {
    pub data: Vec<u8>,
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
    pub esize: usize,
    pub saves: Vec<VectorSave>,
}

/// A snapshot of the vector's bookkeeping state used by save/restore.
#[derive(Debug, Default, Clone)]
pub struct VectorSave {
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
    pub esize: usize,
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
    // Drop happens automatically in Rust.
}

fn vector_in_bounds_for_at(v: &Vector, index: i32) -> bool {
    index >= 0 && index < v.rindex
}

fn vector_resize_for_index(v: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < v.mindex {
        return;
    }
    let new_mindex = start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
    let new_size = (new_mindex as usize) * v.esize;
    v.data.resize(new_size, 0);
    v.mindex = new_mindex;
}

fn vector_resize_for(v: &mut Vector, total_elements: i32) {
    vector_resize_for_index(v, v.rindex, total_elements);
}

fn vector_resize(v: &mut Vector) {
    vector_resize_for(v, 0);
}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(_vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 {
        return None;
    }
    let esize = _vector.esize;
    let start = (index as usize) * esize;
    let end = start + esize;
    if end > _vector.data.len() {
        return None;
    }
    Some(&mut _vector.data[start..end])
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
    vector_at(vector, vector.pindex)
}

/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(vector, vector.pindex) {
        return None;
    }
    let idx = vector.pindex;
    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    vector_at(vector, idx)
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
    vector_pop_at(vector, vector.pindex - 1);
}

/// Returns a reference to the last pushed element for peek purposes.
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
    // Make sure data has space at rindex
    let esize = vector.esize;
    let start = (vector.rindex as usize) * esize;
    let end = start + esize;
    if end > vector.data.len() {
        vector.data.resize(end + esize * VECTOR_ELEMENT_INCREMENT, 0);
    }
    let copy_len = elem.len().min(esize);
    for i in 0..copy_len {
        vector.data[start + i] = elem[i];
    }
    // Zero-pad if elem shorter than esize.
    for i in copy_len..esize {
        vector.data[start + i] = 0;
    }
    vector.rindex += 1;
    vector.count += 1;
    if vector.rindex >= vector.mindex {
        vector_resize(vector);
    }
}

/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    // Shift right by 1 at index, then write.
    let esize = vector.esize;
    let new_rindex = vector.rindex + 1;
    let new_mindex_needed = new_rindex + 1;
    if new_mindex_needed >= vector.mindex {
        vector_resize_for_index(vector, index, 1);
    }
    if vector.data.len() < (new_rindex as usize) * esize {
        vector.data.resize((new_rindex as usize) * esize + esize, 0);
    }
    // Move all bytes from index..rindex to index+1..rindex+1
    let start = (index as usize) * esize;
    let end = (vector.rindex as usize) * esize;
    if end > start {
        let bytes: Vec<u8> = vector.data[start..end].to_vec();
        let dest_start = start + esize;
        for (i, b) in bytes.iter().enumerate() {
            vector.data[dest_start + i] = *b;
        }
    }
    let copy_len = ptr.len().min(esize);
    for i in 0..copy_len {
        vector.data[start + i] = ptr[i];
    }
    for i in copy_len..esize {
        vector.data[start + i] = 0;
    }
    vector.rindex += 1;
    vector.count += 1;
}

/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    vector.rindex -= 1;
    vector.count -= 1;
    assert!(vector.rindex >= 0 && vector.rindex < vector.mindex);
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    vector_pop_at(vector, vector.pindex);
}

/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    if idx < 0 || idx >= vector.mindex {
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
    let len = (vec.count as usize) * vec.esize;
    let bytes = &vec.data[..len.min(vec.data.len())];
    // Trim trailing zero bytes
    let trimmed_end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..trimmed_end]).ok()
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
pub fn vector_fread(_vector: &mut Vector, _amount: i32, _fp: std::fs::File) -> i32 {
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
    let total_bytes = (vector_src.count as usize) * vector_src.esize;
    // Use vector_push_at logic: shift right by `count` at dst_index, then copy bytes.
    let count = vector_src.count;
    // Make space
    let esize = vector_dst.esize;
    vector_resize_for_index(vector_dst, dst_index, count);
    if vector_dst.data.len() < ((vector_dst.rindex + count) as usize) * esize {
        vector_dst
            .data
            .resize(((vector_dst.rindex + count) as usize) * esize + esize, 0);
    }
    let start = (dst_index as usize) * esize;
    let end = (vector_dst.rindex as usize) * esize;
    if end > start {
        let bytes: Vec<u8> = vector_dst.data[start..end].to_vec();
        let dest_start = start + (count as usize) * esize;
        for (i, b) in bytes.iter().enumerate() {
            vector_dst.data[dest_start + i] = *b;
        }
    }
    // Copy source data
    for i in 0..total_bytes {
        vector_dst.data[start + i] = vector_src.data[i];
    }
    vector_dst.rindex += count;
    vector_dst.count += count;
    0
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    // Not used in our implementation.
    0
}

/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let esize = vector.esize;
    let count = vector.count;
    for i in 0..count {
        let start = (i as usize) * esize;
        let end = start + esize;
        if &vector.data[start..end] == &val[..esize.min(val.len())] {
            vector_pop_at(vector, i);
            return i;
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
        let bytes: Vec<u8> = vector.data[next_start..end_pos].to_vec();
        for (i, b) in bytes.iter().enumerate() {
            vector.data[dst_start + i] = *b;
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
    if let Some(s) = vector.saves.pop() {
        vector.pindex = s.pindex;
        vector.rindex = s.rindex;
        vector.mindex = s.mindex;
        vector.count = s.count;
        vector.flags = s.flags;
        vector.esize = s.esize;
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
    vector.clone()
}
