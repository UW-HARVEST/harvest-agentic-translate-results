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
    pub saves: Vec<VectorSaveState>,
}

#[derive(Debug, Clone)]
pub struct VectorSaveState {
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
}

fn vector_in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn vector_in_bounds_for_pop(vector: &Vector, index: i32) -> bool {
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
    // dropped automatically
}

fn vector_resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_mindex = start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
    let new_size = (new_mindex as usize) * vector.esize;
    vector.data.resize(new_size, 0);
    vector.mindex = start_index + total_elements;
}

fn vector_resize_for(vector: &mut Vector, total_elements: i32) {
    vector_resize_for_index(vector, vector.rindex, total_elements);
}

fn vector_resize(vector: &mut Vector) {
    vector_resize_for(vector, 0);
}

/// Returns a reference to the element at the given index.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    let esize = vector.esize;
    let start = (index as usize) * esize;
    let end = start + esize;
    if end > vector.data.len() {
        return None;
    }
    Some(&mut vector.data[start..end])
}

/// Returns a reference to the element at the given index for peek operations.
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
    vector_pop_at(vector, vector.pindex - 1);
}

/// Returns a reference to the last pushed element for peek purposes (treat as ptr).
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
    let esize = vector.esize;
    let start = (vector.rindex as usize) * esize;
    if start + esize > vector.data.len() {
        let new_size = start + esize;
        vector.data.resize(new_size, 0);
    }
    let copy_len = esize.min(elem.len());
    vector.data[start..start + copy_len].copy_from_slice(&elem[..copy_len]);
    if copy_len < esize {
        for b in &mut vector.data[start + copy_len..start + esize] {
            *b = 0;
        }
    }
    vector.rindex += 1;
    vector.count += 1;
    if vector.rindex >= vector.mindex {
        vector_resize(vector);
    }
}

fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    vector_resize_for_index(vector, index, amount);
    let esize = vector.esize;
    let elements_to_move = (vector.count - index) as usize;
    let bytes_to_move = elements_to_move * esize;
    let src_start = (index as usize) * esize;
    let dst_start = ((index + amount) as usize) * esize;
    // Ensure capacity
    let needed = dst_start + bytes_to_move;
    if vector.data.len() < needed {
        vector.data.resize(needed, 0);
    }
    // Copy: source overlaps destination, use copy_within
    vector.data.copy_within(src_start..src_start + bytes_to_move, dst_start);
    // Zero the gap
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

/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    vector_shift_right(vector, index, 1);
    let esize = vector.esize;
    let start = (index as usize) * esize;
    let copy_len = esize.min(ptr.len());
    vector.data[start..start + copy_len].copy_from_slice(&ptr[..copy_len]);
}

/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    vector.rindex -= 1;
    vector.count -= 1;
    assert!(vector_in_bounds_for_pop(vector, vector.rindex));
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    vector_pop_at(vector, vector.pindex);
}

/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    if idx < 0 {
        return None;
    }
    vector_at(vector, idx)
}

/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(vector, vector.rindex - 1) {
        return None;
    }
    vector_at(vector, vector.rindex - 1)
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
    // Find length up to first null byte for safety
    let len = vec.data.iter().position(|&b| b == 0).unwrap_or(vec.data.len());
    std::str::from_utf8(&vec.data[..len]).ok()
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
    let esize = vector_src.esize;
    let bytes = (total as usize) * esize;
    vector_shift_right(vector_dst, dst_index, total);
    let start = (dst_index as usize) * esize;
    let needed = start + bytes;
    if vector_dst.data.len() < needed {
        vector_dst.data.resize(needed, 0);
    }
    vector_dst.data[start..start + bytes].copy_from_slice(&vector_src.data[..bytes]);
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

/// Removes the first element that matches the given value (by raw bytes).
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let esize = vector.esize;
    let count = vector.count;
    for i in 0..count {
        let start = (i as usize) * esize;
        if &vector.data[start..start + esize.min(val.len())] == &val[..esize.min(val.len())] {
            vector_pop_at(vector, i);
            return 0;
        }
    }
    0
}

/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    let esize = vector.esize;
    let dst = (index as usize) * esize;
    let next = dst + esize;
    let end = (vector.rindex as usize) * esize;
    if end > next {
        vector.data.copy_within(next..end, dst);
    }
    vector.count -= 1;
    vector.rindex -= 1;
}

/// Moves the peek pointer back by 1.
pub fn vector_peek_back(vector: &mut Vector) {
    vector.pindex -= 1;
}

/// Returns the current rindex of the vector.
pub fn vector_current_index(vector: &Vector) -> i32 {
    vector.rindex
}

/// Saves the current state of the vector for future restore.
pub fn vector_save(vector: &mut Vector) {
    vector.saves.push(VectorSaveState {
        pindex: vector.pindex,
        rindex: vector.rindex,
        mindex: vector.mindex,
        count: vector.count,
        flags: vector.flags,
    });
}

/// Restores a previously saved state of the vector.
pub fn vector_restore(vector: &mut Vector) {
    if let Some(s) = vector.saves.pop() {
        vector.pindex = s.pindex;
        vector.rindex = s.rindex;
        vector.mindex = s.mindex;
        vector.count = s.count;
        vector.flags = s.flags;
    }
}

/// Removes the last saved state.
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
