// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

// Structs
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

#[derive(Debug, Default, Clone)]
pub struct VectorSave {
    pub data: Vec<u8>,
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

fn vector_assert_bounds_for_pop(vector: &Vector, index: i32) {
    assert!(vector_in_bounds_for_pop(vector, index));
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
pub fn vector_free(_vector: Vector) {}

fn vector_resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_mindex = start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
    let new_size = (new_mindex as usize) * vector.esize;
    vector.data.resize(new_size, 0);
    vector.mindex = new_mindex;
}

fn vector_resize_for(vector: &mut Vector, total_elements: i32) {
    vector_resize_for_index(vector, vector.rindex, total_elements);
}

fn vector_resize(vector: &mut Vector) {
    vector_resize_for(vector, 0);
}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 {
        return None;
    }
    let esize = vector.esize;
    let start = (index as usize) * esize;
    if start + esize > vector.data.len() {
        return None;
    }
    Some(&mut vector.data[start..start + esize])
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

/// Returns a reference to the last pushed element for peek purposes.
pub fn vector_peek_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    // In the C version it returns the dereferenced pointer; here we just return the bytes
    // (the caller is responsible for interpreting them).
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
    let esize = vector.esize;
    // Ensure we have capacity for index rindex
    let needed = ((vector.rindex as usize) + 1) * esize;
    if needed > vector.data.len() {
        vector_resize(vector);
    }
    let start = (vector.rindex as usize) * esize;
    let copy_len = esize.min(elem.len());
    vector.data[start..start + copy_len].copy_from_slice(&elem[..copy_len]);
    vector.rindex += 1;
    vector.count += 1;
    if vector.rindex >= vector.mindex {
        vector_resize(vector);
    }
}

/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    vector_shift_right(vector, index, 1);
    let esize = vector.esize;
    let start = (index as usize) * esize;
    let copy_len = esize.min(ptr.len());
    vector.data[start..start + copy_len].copy_from_slice(&ptr[..copy_len]);
}

fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    vector_resize_for_index(vector, index, amount);
    let esize = vector.esize;
    let eindex = (index + amount) as usize;
    let bytes_to_move = ((vector.count - index) as usize) * esize;
    let src_start = (index as usize) * esize;
    let dst_start = eindex * esize;
    // Use copy_within to handle overlapping copy correctly
    vector
        .data
        .copy_within(src_start..src_start + bytes_to_move, dst_start);
    // Zero out the gap
    for b in &mut vector.data[src_start..src_start + (amount as usize) * esize] {
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
    vector_resize_for_index(vector, index, 0);
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

/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    vector.rindex -= 1;
    vector.count -= 1;
    vector_assert_bounds_for_pop(vector, vector.rindex);
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    let pi = vector.pindex;
    vector_pop_at(vector, pi);
}

/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_assert_bounds_for_pop(vector, vector.rindex - 1);
    let idx = vector.rindex - 1;
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
    // Find first null terminator (or use full data)
    let end = vec.data.iter().position(|&b| b == 0).unwrap_or(vec.data.len());
    std::str::from_utf8(&vec.data[..end]).ok()
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
    let mut byte = [0u8; 1];
    while let Ok(n) = fp.read(&mut byte) {
        if n == 0 {
            break;
        }
        vector_push(vector, &byte);
    }
    0
}

/// Returns a reference to the underlying data of the vector.
pub fn vector_data_ptr(vector: &Vector) -> &[u8] {
    &vector.data[..]
}

/// Inserts data from `vector_src` into `vector_dst` at `dst_index`.
pub fn vector_insert(vector_dst: &mut Vector, vector_src: &Vector, dst_index: i32) -> i32 {
    if vector_dst.esize != vector_src.esize {
        return -1;
    }
    let total = vector_src.count;
    let esize = vector_src.esize;
    vector_shift_right(vector_dst, dst_index, total);
    let dst_start = (dst_index as usize) * esize;
    let total_bytes = (total as usize) * esize;
    vector_dst.data[dst_start..dst_start + total_bytes]
        .copy_from_slice(&vector_src.data[0..total_bytes]);
    0
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(vector: &mut Vector, address: *const u8) -> i32 {
    let base = vector.data.as_ptr() as usize;
    let addr = address as usize;
    let index = ((addr - base) / vector.esize) as i32;
    vector_pop_at(vector, index);
    index
}

/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let old_pp = vector.pindex;
    vector_set_peek_pointer(vector, 0);
    let esize = vector.esize;
    let mut index = 0;
    while index < vector.count {
        let start = (index as usize) * esize;
        if &vector.data[start..start + esize] == &val[..esize.min(val.len())] {
            vector_pop_at(vector, index);
            break;
        }
        index += 1;
    }
    vector.pindex = old_pp;
    0
}

/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    let esize = vector.esize;
    let dst_pos = (index as usize) * esize;
    let next_element_pos = dst_pos + esize;
    let end_pos = (vector.rindex as usize) * esize;
    if next_element_pos < end_pos {
        let total = end_pos - next_element_pos;
        vector
            .data
            .copy_within(next_element_pos..next_element_pos + total, dst_pos);
    }
    vector.count -= 1;
    vector.rindex -= 1;
}

/// Moves the peek pointer to the back of the vector.
pub fn vector_peek_back(vector: &mut Vector) {
    vector.pindex -= 1;
}

/// Returns the current index that a vector_push would push to.
pub fn vector_current_index(vector: &Vector) -> i32 {
    vector.rindex
}

/// Saves the current state of the vector for future restore.
pub fn vector_save(vector: &mut Vector) {
    let save = VectorSave {
        data: vector.data.clone(),
        pindex: vector.pindex,
        rindex: vector.rindex,
        mindex: vector.mindex,
        count: vector.count,
        flags: vector.flags,
        esize: vector.esize,
    };
    vector.saves.push(save);
}

/// Restores a previously saved state of the vector.
pub fn vector_restore(vector: &mut Vector) {
    if let Some(save) = vector.saves.pop() {
        vector.data = save.data;
        vector.pindex = save.pindex;
        vector.rindex = save.rindex;
        vector.mindex = save.mindex;
        vector.count = save.count;
        vector.flags = save.flags;
        vector.esize = save.esize;
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
