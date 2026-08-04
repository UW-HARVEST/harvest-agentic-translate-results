// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

// Structs
/// A safe, idiomatic representation of the original `struct vector`.
/// Internally we store each element as a `Vec<u8>` of size `esize`.
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

/// Saved state for vector_save/vector_restore. Includes the data snapshot.
#[derive(Debug, Default, Clone)]
pub struct VectorState {
    pub data: Vec<u8>,
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
    // Drop happens automatically.
}

/// Returns a reference to the element at the given index.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 {
        return None;
    }
    let start = (index as usize) * vector.esize;
    let end = start + vector.esize;
    if end > vector.data.len() {
        return None;
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
    let idx = vector.pindex;
    if idx < 0 || idx >= vector.rindex {
        return None;
    }
    vector_at(vector, idx)
}

/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.pindex;
    if idx < 0 || idx >= vector.rindex {
        return None;
    }
    if (vector.flags & VECTOR_FLAG_PEEK_DECREMENT) != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    vector_at(vector, idx)
}

/// Returns a reference to the element at the given index without changing the peek pointer.
pub fn vector_peek_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 || index >= vector.rindex {
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
/// In the C code this would treat the element as a pointer to data; here we
/// just return the element bytes.
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

fn vector_resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_mindex = start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
    let new_byte_len = new_mindex as usize * vector.esize;
    vector.data.resize(new_byte_len, 0);
    vector.mindex = new_mindex;
}

fn vector_resize(vector: &mut Vector) {
    vector_resize_for_index(vector, vector.rindex, 0);
}

/// Pushes a new element (pointed to by `elem`) onto the vector.
pub fn vector_push(vector: &mut Vector, elem: &[u8]) {
    // Ensure capacity
    if (vector.rindex + 1) as usize * vector.esize > vector.data.len() {
        vector_resize(vector);
    }
    let start = (vector.rindex as usize) * vector.esize;
    let copy_len = elem.len().min(vector.esize);
    let end = start + copy_len;
    vector.data[start..end].copy_from_slice(&elem[..copy_len]);
    // Zero-pad the rest of the slot if elem is shorter than esize.
    if copy_len < vector.esize {
        for b in &mut vector.data[end..start + vector.esize] {
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
    let eindex = index + amount;
    let count = vector.count;
    let elements_until_end = count - index;
    let bytes_to_move = (elements_until_end as usize) * vector.esize;
    if bytes_to_move > 0 {
        let src = (index as usize) * vector.esize;
        let dst = (eindex as usize) * vector.esize;
        // memmove via copy_within
        vector.data.copy_within(src..src + bytes_to_move, dst);
    }
    let zero_start = (index as usize) * vector.esize;
    let zero_end = zero_start + (amount as usize) * vector.esize;
    for b in &mut vector.data[zero_start..zero_end] {
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
    let start = (index as usize) * vector.esize;
    let copy_len = ptr.len().min(vector.esize);
    vector.data[start..start + copy_len].copy_from_slice(&ptr[..copy_len]);
}

/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    vector.rindex -= 1;
    vector.count -= 1;
    assert!(vector.rindex >= 0 && vector.rindex < vector.mindex);
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    let idx = vector.pindex;
    vector_pop_at(vector, idx);
}

/// Returns a reference to the last element in the vector.
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    if vector.rindex - 1 < 0 {
        return None;
    }
    let idx = vector.rindex - 1;
    vector_at(vector, idx)
}

/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = vector.rindex - 1;
    if idx < 0 || idx >= vector.rindex {
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
    let bytes_len = (vec.count as usize) * vec.esize;
    let slice = &vec.data[..bytes_len.min(vec.data.len())];
    // find the first NUL terminator if any
    let nul_pos = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    std::str::from_utf8(&slice[..nul_pos]).ok()
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
    &vector.data
}

/// Inserts data from `vector_src` into `vector_dst` at `dst_index`.
pub fn vector_insert(vector_dst: &mut Vector, vector_src: &Vector, dst_index: i32) -> i32 {
    if vector_dst.esize != vector_src.esize {
        return -1;
    }
    let total = vector_src.count;
    let src_bytes_len = (total as usize) * vector_src.esize;
    let src_data: Vec<u8> = vector_src.data[..src_bytes_len].to_vec();
    vector_shift_right(vector_dst, dst_index, total);
    let dst_start = (dst_index as usize) * vector_dst.esize;
    vector_dst.data[dst_start..dst_start + src_bytes_len].copy_from_slice(&src_data);
    0
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    // Cannot reliably do pointer arithmetic in safe Rust over our owned Vec<u8>.
    // Stub: return 0.
    0
}

/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let cmp_len = val.len().min(vector.esize);
    let mut found = -1i32;
    for i in 0..vector.count {
        let start = (i as usize) * vector.esize;
        let end = start + cmp_len;
        if &vector.data[start..end] == &val[..cmp_len] {
            found = i;
            break;
        }
    }
    if found >= 0 {
        vector_pop_at(vector, found);
    }
    0
}

/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 || index >= vector.count {
        return;
    }
    let dst = (index as usize) * vector.esize;
    let next = dst + vector.esize;
    let end = (vector.rindex as usize) * vector.esize;
    if next < end {
        let bytes_to_move = end - next;
        vector.data.copy_within(next..next + bytes_to_move, dst);
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
    vector.saves.push(VectorState {
        data: vector.data.clone(),
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
    if let Some(s) = vector.saves.pop() {
        vector.data = s.data;
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
