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

/// Saved state for vector_save/restore operations.
#[derive(Debug, Default, Clone)]
pub struct VectorSave {
    pub pindex: i32,
    pub rindex: i32,
    pub mindex: i32,
    pub count: i32,
    pub flags: i32,
    pub data: Vec<u8>,
}

fn vector_in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn vector_in_bounds_for_pop(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.mindex
}

fn ensure_capacity(vector: &mut Vector, total_elements: i32) {
    let needed = (vector.rindex + total_elements + VECTOR_ELEMENT_INCREMENT as i32) as usize;
    let needed_bytes = needed * vector.esize;
    if vector.data.len() < needed_bytes {
        vector.data.resize(needed_bytes, 0);
    }
    if vector.rindex + total_elements >= vector.mindex {
        vector.mindex = vector.rindex + total_elements + VECTOR_ELEMENT_INCREMENT as i32;
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
    // Drop happens automatically.
}

/// Returns a reference to the element at the given index, if in range.
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
pub fn vector_peek_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    // In C this dereferences a pointer-to-pointer. In our Rust safe model
    // we just return the bytes that are stored at that index.
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
    let start = (vector.rindex as usize) * vector.esize;
    let end = start + vector.esize;
    let copy_len = elem.len().min(vector.esize);
    if end > vector.data.len() {
        vector.data.resize(end, 0);
    }
    vector.data[start..start + copy_len].copy_from_slice(&elem[..copy_len]);
    // Zero-pad if elem is smaller than esize.
    for i in copy_len..vector.esize {
        vector.data[start + i] = 0;
    }
    vector.rindex += 1;
    vector.count += 1;
    if vector.rindex >= vector.mindex {
        ensure_capacity(vector, 0);
    }
}

/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    vector_shift_right(vector, index, 1);
    if let Some(slot) = vector_at(vector, index) {
        let copy_len = ptr.len().min(slot.len());
        slot[..copy_len].copy_from_slice(&ptr[..copy_len]);
        for i in copy_len..slot.len() {
            slot[i] = 0;
        }
    }
}

fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    ensure_capacity(vector, amount);
    let esize = vector.esize;
    let total_to_move = (vector.count - index).max(0) as usize * esize;
    if total_to_move > 0 {
        let src = (index as usize) * esize;
        let dst = ((index + amount) as usize) * esize;
        let needed = dst + total_to_move;
        if vector.data.len() < needed {
            vector.data.resize(needed, 0);
        }
        // Shift right
        vector.data.copy_within(src..src + total_to_move, dst);
    }
    let zero_start = (index as usize) * esize;
    let zero_end = zero_start + (amount as usize) * esize;
    if zero_end > vector.data.len() {
        vector.data.resize(zero_end, 0);
    }
    for i in zero_start..zero_end {
        vector.data[i] = 0;
    }
}

fn vector_stretch(vector: &mut Vector, index: i32) {
    if index < vector.rindex {
        return;
    }
    let needed = (index as usize) * vector.esize;
    if vector.data.len() < needed {
        vector.data.resize(needed + VECTOR_ELEMENT_INCREMENT * vector.esize, 0);
    }
    vector.mindex = index;
    vector.count = index;
    vector.rindex = index;
}

fn vector_shift_right(vector: &mut Vector, index: i32, amount: i32) {
    if index < vector.rindex {
        vector_shift_right_in_bounds_no_increment(vector, index, amount);
        vector.rindex += amount;
        vector.count += amount;
        return;
    }
    vector_stretch(vector, index + amount);
    vector_shift_right_in_bounds_no_increment(vector, index, amount);
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
    if total == 0 {
        return Some("");
    }
    let slice = &vec.data[..total.min(vec.data.len())];
    // Find null terminator if present
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

/// Reads data into the vector from a file pointer (stub; not fully safe in typical Rust).
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
    let total = vector_src.count;
    let total_bytes = (total as usize) * vector_src.esize;
    vector_shift_right(vector_dst, dst_index, total);
    let dst_start = (dst_index as usize) * vector_dst.esize;
    let needed = dst_start + total_bytes;
    if vector_dst.data.len() < needed {
        vector_dst.data.resize(needed, 0);
    }
    vector_dst.data[dst_start..dst_start + total_bytes]
        .copy_from_slice(&vector_src.data[..total_bytes]);
    0
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(vector: &mut Vector, address: *const u8) -> i32 {
    let base = vector.data.as_ptr() as usize;
    let addr = address as usize;
    let offset = addr.saturating_sub(base);
    let index = (offset / vector.esize) as i32;
    vector_pop_at(vector, index);
    index
}

/// Removes the first element that matches the given value (compares the bytes).
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let old_pp = vector.pindex;
    vector.pindex = 0;
    let esize = vector.esize;
    let mut index: i32 = 0;
    let mut found: i32 = -1;
    while index < vector.count {
        let start = (index as usize) * esize;
        let end = start + esize;
        let cmp_len = val.len().min(esize);
        if vector.data[start..start + cmp_len] == val[..cmp_len] {
            found = index;
            break;
        }
        let _ = end;
        index += 1;
    }
    if found >= 0 {
        vector_pop_at(vector, found);
    }
    vector.pindex = old_pp;
    found
}

/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 || index >= vector.count {
        return;
    }
    let esize = vector.esize;
    let dst = (index as usize) * esize;
    let src = dst + esize;
    let end = (vector.count as usize) * esize;
    if src < end {
        let len = end - src;
        vector.data.copy_within(src..src + len, dst);
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
        data: vector.data.clone(),
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
        vector.data = save.data;
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
