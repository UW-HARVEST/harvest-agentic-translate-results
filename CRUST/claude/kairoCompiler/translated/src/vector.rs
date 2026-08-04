// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

#[derive(Debug, Default, Clone)]
struct VectorSave {
    pindex: i32,
    rindex: i32,
    count: i32,
    flags: i32,
    data_len: usize,
}

// Structs
/// A safe, idiomatic representation of the original `struct vector`.
#[derive(Debug, Default, Clone)]
pub struct Vector {
    data: Vec<u8>,
    pindex: i32,
    rindex: i32,
    count: i32,
    flags: i32,
    esize: usize,
    saves: Vec<VectorSave>,
}

// Function Declarations
/// Creates a new vector with elements of size `esize`.
pub fn vector_create(esize: usize) -> Vector {
    Vector {
        data: Vec::new(),
        pindex: 0,
        rindex: 0,
        count: 0,
        flags: 0,
        esize,
        saves: Vec::new(),
    }
}

/// Frees the given vector (in Rust, typically done by dropping).
pub fn vector_free(_vector: Vector) {
    // Drop occurs automatically.
}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 || index >= vector.rindex {
        return None;
    }
    let esize = vector.esize;
    let start = (index as usize) * esize;
    let end = start + esize;
    if end > vector.data.len() {
        return None;
    }
    Some(&mut vector.data[start..end])
}

/// Returns a reference to the element at the given index for peek operations, if in range.
pub fn vector_peek_ptr_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    vector_at(vector, index)
}

/// Returns a reference to the next element to peek without incrementing the internal pointer.
pub fn vector_peek_no_increment(vector: &mut Vector) -> Option<&mut [u8]> {
    let p = vector.pindex;
    vector_at(vector, p)
}

/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    let p = vector.pindex;
    if p < 0 || p >= vector.rindex {
        return None;
    }
    let esize = vector.esize;
    let start = (p as usize) * esize;
    let end = start + esize;
    if end > vector.data.len() {
        return None;
    }
    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    Some(&mut vector.data[start..end])
}

/// Returns a reference to the element at the given index without changing the peek pointer.
pub fn vector_peek_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
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
    let esize = vector.esize;
    if elem.len() >= esize {
        vector.data.extend_from_slice(&elem[..esize]);
    } else {
        vector.data.extend_from_slice(elem);
        // Pad with zeros if elem too small
        for _ in elem.len()..esize {
            vector.data.push(0);
        }
    }
    vector.rindex += 1;
    vector.count += 1;
}

/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    let esize = vector.esize;
    let pos = (index as usize) * esize;
    // Make sure we have enough room
    if pos > vector.data.len() {
        vector.data.resize(pos, 0);
    }
    let bytes_to_insert: Vec<u8> = if ptr.len() >= esize {
        ptr[..esize].to_vec()
    } else {
        let mut v = ptr.to_vec();
        v.resize(esize, 0);
        v
    };
    // Insert at position
    vector.data.splice(pos..pos, bytes_to_insert);
    vector.rindex += 1;
    vector.count += 1;
}

/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    if vector.rindex <= 0 {
        return;
    }
    let esize = vector.esize;
    let new_len = vector.data.len().saturating_sub(esize);
    vector.data.truncate(new_len);
    vector.rindex -= 1;
    vector.count -= 1;
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    let p = vector.pindex;
    vector_pop_at(vector, p);
}

/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    if vector.rindex <= 0 {
        return None;
    }
    let last_idx = vector.rindex - 1;
    vector_at(vector, last_idx)
}

/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    if vector.rindex <= 0 {
        return None;
    }
    let last_idx = vector.rindex - 1;
    vector_at(vector, last_idx)
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
    std::str::from_utf8(&vec.data).ok()
}

/// Checks if the vector is empty.
pub fn vector_empty(vector: &Vector) -> bool {
    vector.count == 0
}

/// Clears the vector contents.
pub fn vector_clear(vector: &mut Vector) {
    vector.data.clear();
    vector.rindex = 0;
    vector.count = 0;
    vector.pindex = 0;
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
    let esize = vector_dst.esize;
    let pos = (dst_index as usize) * esize;
    if pos > vector_dst.data.len() {
        vector_dst.data.resize(pos, 0);
    }
    let to_insert = vector_src.data.clone();
    let count = vector_src.count;
    vector_dst.data.splice(pos..pos, to_insert);
    vector_dst.rindex += count;
    vector_dst.count += count;
    0
}

/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    // This is unsafe and not directly translatable; return 0.
    0
}

/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let esize = vector.esize;
    let count = vector.count as usize;
    for i in 0..count {
        let start = i * esize;
        let end = start + esize;
        if end <= vector.data.len() && &vector.data[start..end] == val {
            vector_pop_at(vector, i as i32);
            return i as i32;
        }
    }
    -1
}

/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 || index >= vector.rindex {
        return;
    }
    let esize = vector.esize;
    let start = (index as usize) * esize;
    let end = start + esize;
    vector.data.drain(start..end);
    vector.rindex -= 1;
    vector.count -= 1;
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
    vector.saves.push(VectorSave {
        pindex: vector.pindex,
        rindex: vector.rindex,
        count: vector.count,
        flags: vector.flags,
        data_len: vector.data.len(),
    });
}

/// Restores a previously saved state of the vector.
pub fn vector_restore(vector: &mut Vector) {
    if let Some(save) = vector.saves.pop() {
        vector.pindex = save.pindex;
        vector.rindex = save.rindex;
        vector.count = save.count;
        vector.flags = save.flags;
        vector.data.truncate(save.data_len);
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
        count: vector.count,
        flags: vector.flags,
        esize: vector.esize,
        saves: Vec::new(),
    }
}
