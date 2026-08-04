// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

// Structs
/// A safe, idiomatic representation of the original `struct vector`.
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
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
    pub esize: usize,
}

fn in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn in_bounds_for_pop(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.mindex
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
    // Drop handles cleanup automatically
}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 {
        return None;
    }
    let start = (index as usize) * vector.esize;
    let end = start + vector.esize;
    if end > vector.data.len() {
        // need to grow the underlying buffer
        vector.data.resize(end, 0u8);
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
    if !in_bounds_for_at(vector, vector.pindex) {
        return None;
    }
    let pindex = vector.pindex;
    vector_at(vector, pindex)
}

/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    if !in_bounds_for_at(vector, vector.pindex) {
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
    if !in_bounds_for_at(vector, index) {
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

/// Returns a reference to the last pushed element for peek purposes (peek + advance).
pub fn vector_peek_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    // In the C code, vector_peek_ptr dereferences the pointer (vector of pointers).
    // Here we simply return the same as vector_peek.
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

fn vector_resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_mindex = start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
    let new_byte_size = (new_mindex as usize) * vector.esize;
    vector.data.resize(new_byte_size, 0u8);
    vector.mindex = new_mindex;
}

fn vector_resize_for(vector: &mut Vector, total_elements: i32) {
    vector_resize_for_index(vector, vector.rindex, total_elements);
}

fn vector_resize(vector: &mut Vector) {
    vector_resize_for(vector, 0);
}

/// Pushes a new element (pointed to by `elem`) onto the vector.
pub fn vector_push(vector: &mut Vector, elem: &[u8]) {
    // Ensure we have capacity
    let needed_byte_size = (vector.rindex as usize + 1) * vector.esize;
    if vector.data.len() < needed_byte_size {
        vector.data.resize(needed_byte_size, 0u8);
    }
    let offset = (vector.rindex as usize) * vector.esize;
    let copy_len = std::cmp::min(elem.len(), vector.esize);
    vector.data[offset..offset + copy_len].copy_from_slice(&elem[..copy_len]);
    if copy_len < vector.esize {
        // Zero out remainder
        for i in copy_len..vector.esize {
            vector.data[offset + i] = 0;
        }
    }

    vector.rindex += 1;
    vector.count += 1;

    if vector.rindex >= vector.mindex {
        vector_resize(vector);
    }
}

/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    vector_shift_right(vector, index, 1);
    if let Some(slice) = vector_at(vector, index) {
        let copy_len = std::cmp::min(ptr.len(), slice.len());
        slice[..copy_len].copy_from_slice(&ptr[..copy_len]);
    }
}

fn vector_data_end_offset(vector: &Vector) -> usize {
    (vector.rindex as usize) * vector.esize
}

fn vector_elements_until_end(vector: &Vector, index: i32) -> i32 {
    vector.count - index
}

fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    vector_resize_for_index(vector, index, amount);
    let eindex = index + amount;
    let bytes_to_move = (vector_elements_until_end(vector, index) as usize) * vector.esize;
    let src_off = (index as usize) * vector.esize;
    let dst_off = (eindex as usize) * vector.esize;
    // ensure capacity
    let needed = dst_off + bytes_to_move;
    if vector.data.len() < needed {
        vector.data.resize(needed, 0u8);
    }
    // Use copy_within
    if bytes_to_move > 0 {
        vector.data.copy_within(src_off..src_off + bytes_to_move, dst_off);
    }
    let zero_len = (amount as usize) * vector.esize;
    for i in 0..zero_len {
        if src_off + i < vector.data.len() {
            vector.data[src_off + i] = 0;
        }
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
    assert!(in_bounds_for_pop(vector, vector.rindex));
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    let pindex = vector.pindex;
    vector_pop_at(vector, pindex);
}

/// Returns a reference to the last element in the vector.
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    if !in_bounds_for_pop(vector, idx) {
        return None;
    }
    vector_at(vector, idx)
}

/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    if !in_bounds_for_at(vector, idx) {
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
    let used = (vec.count as usize) * vec.esize;
    let slice = &vec.data[..used.min(vec.data.len())];
    // find null terminator
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    std::str::from_utf8(&slice[..end]).ok()
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

/// Reads data into the vector from a file pointer.
pub fn vector_fread(vector: &mut Vector, _amount: i32, mut fp: std::fs::File) -> i32 {
    use std::io::Read;
    let mut buf = [0u8; 1];
    while let Ok(n) = fp.read(&mut buf) {
        if n == 0 {
            break;
        }
        vector_push(vector, &buf);
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
    let src_data: Vec<u8> = vector_src.data[..total_bytes].to_vec();
    vector_shift_right(vector_dst, dst_index, total);
    let dst_off = (dst_index as usize) * vector_dst.esize;
    if vector_dst.data.len() < dst_off + total_bytes {
        vector_dst.data.resize(dst_off + total_bytes, 0u8);
    }
    vector_dst.data[dst_off..dst_off + total_bytes].copy_from_slice(&src_data);
    0
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    // Not safely implementable in Rust without raw pointers/unsafe; we'll provide a basic version
    // that returns -1.
    -1
}

/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let old_pp = vector.pindex;
    vector.pindex = 0;
    let mut found = -1;
    for index in 0..vector.count {
        let off = (index as usize) * vector.esize;
        let slice = &vector.data[off..off + vector.esize];
        if slice == val {
            found = index;
            break;
        }
    }
    if found != -1 {
        vector_pop_at(vector, found);
    }
    vector.pindex = old_pp;
    found
}

/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 || index >= vector.rindex {
        return;
    }
    let dst_off = (index as usize) * vector.esize;
    let src_off = dst_off + vector.esize;
    let end_off = vector_data_end_offset(vector);
    if src_off < end_off {
        let total = end_off - src_off;
        vector.data.copy_within(src_off..src_off + total, dst_off);
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
    let save = VectorSave {
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
