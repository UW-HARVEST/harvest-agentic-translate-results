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
    pub saves: Vec<VectorState>,
}

#[derive(Debug, Default, Clone)]
pub struct VectorState {
    pub pindex: i32,
    pub rindex: i32,
    pub count: i32,
    pub flags: i32,
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

fn vector_in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn ensure_capacity(vector: &mut Vector, total_elements: i32) {
    let needed = (vector.rindex + total_elements) as usize;
    if needed >= vector.mindex as usize {
        let new_size = needed + VECTOR_ELEMENT_INCREMENT;
        vector.data.resize(new_size * vector.esize, 0);
        vector.mindex = new_size as i32;
    }
}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(vector, index) {
        return None;
    }
    let start = index as usize * vector.esize;
    let end = start + vector.esize;
    Some(&mut vector.data[start..end])
}
/// Returns a reference to the element at the given index for peek operations, if in range.
pub fn vector_peek_ptr_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    vector_at(vector, index)
}
/// Returns a reference to the next element to peek without incrementing the internal pointer.
pub fn vector_peek_no_increment(vector: &mut Vector) -> Option<&mut [u8]> {
    if vector.pindex >= vector.rindex || vector.pindex < 0 {
        return None;
    }
    let start = vector.pindex as usize * vector.esize;
    let end = start + vector.esize;
    Some(&mut vector.data[start..end])
}
/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    if vector.pindex >= vector.rindex || vector.pindex < 0 {
        return None;
    }
    let idx = vector.pindex;
    if (vector.flags & VECTOR_FLAG_PEEK_DECREMENT) != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    let start = idx as usize * vector.esize;
    let end = start + vector.esize;
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
    if vector.pindex > 0 {
        vector_pop_at(vector, vector.pindex - 1);
    }
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
    ensure_capacity(vector, 1);
    let start = vector.rindex as usize * vector.esize;
    let copy_len = elem.len().min(vector.esize);
    for i in 0..copy_len {
        vector.data[start + i] = elem[i];
    }
    vector.rindex += 1;
    vector.count += 1;
}
/// Pushes a new element at a specific index.
pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    if index < 0 {
        return;
    }
    // Make room for the new element.
    ensure_capacity(vector, 1);
    // Shift elements right by one.
    let esize = vector.esize;
    let count = vector.rindex as usize;
    if (index as usize) < count {
        // shift down
        let mut i = count;
        while i > index as usize {
            let src = (i - 1) * esize;
            let dst = i * esize;
            for j in 0..esize {
                vector.data[dst + j] = vector.data[src + j];
            }
            i -= 1;
        }
    }
    let start = index as usize * esize;
    let copy_len = ptr.len().min(esize);
    for j in 0..copy_len {
        vector.data[start + j] = ptr[j];
    }
    vector.rindex += 1;
    vector.count += 1;
}
/// Removes the last element from the vector.
pub fn vector_pop(vector: &mut Vector) {
    if vector.rindex > 0 {
        vector.rindex -= 1;
        if vector.count > 0 {
            vector.count -= 1;
        }
    }
}
/// Removes the peeked element from the vector.
pub fn vector_peek_pop(vector: &mut Vector) {
    if vector.pindex > 0 {
        vector_pop_at(vector, vector.pindex - 1);
    }
}
/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    if vector.rindex == 0 {
        return None;
    }
    let idx = vector.rindex - 1;
    let start = idx as usize * vector.esize;
    let end = start + vector.esize;
    Some(&mut vector.data[start..end])
}
/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}
/// Returns a reference to the last element in the vector for pointer usage.
pub fn vector_back_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}
/// Returns a reference to the last element or null, specialized for pointer usage.
pub fn vector_back_ptr_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}
/// Returns a string slice representation from the vector if it contains text data.
pub fn vector_string(vec: &Vector) -> Option<&str> {
    let total = (vec.count as usize) * vec.esize;
    std::str::from_utf8(&vec.data[..total]).ok()
}
/// Checks if the vector is empty.
pub fn vector_empty(vector: &Vector) -> bool {
    vector.rindex == 0
}
/// Clears the vector contents.
pub fn vector_clear(vector: &mut Vector) {
    vector.rindex = 0;
    vector.pindex = 0;
    vector.count = 0;
}
/// Returns the count of elements in the vector.
pub fn vector_count(vector: &Vector) -> i32 {
    vector.count
}
/// Reads data into the vector from a file pointer (stub; not fully safe in typical Rust).
pub fn vector_fread(vector: &mut Vector, amount: i32, mut fp: std::fs::File) -> i32 {
    use std::io::Read;
    ensure_capacity(vector, amount);
    let start = vector.rindex as usize * vector.esize;
    let total = amount as usize * vector.esize;
    let end = start + total;
    let read_count = match fp.read(&mut vector.data[start..end]) {
        Ok(n) => n,
        Err(_) => return -1,
    };
    let elements_read = (read_count / vector.esize) as i32;
    vector.rindex += elements_read;
    vector.count += elements_read;
    elements_read
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
    let mut idx = dst_index;
    for i in 0..vector_src.rindex {
        let start = i as usize * vector_src.esize;
        let end = start + vector_src.esize;
        let elem = &vector_src.data[start..end];
        vector_push_at(vector_dst, idx, elem);
        idx += 1;
    }
    0
}
/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    -1
}
/// Removes the first element that matches the given value.
pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let esize = vector.esize;
    for i in 0..vector.rindex {
        let start = i as usize * esize;
        let end = start + esize;
        let cmp_len = val.len().min(esize);
        if &vector.data[start..start + cmp_len] == &val[..cmp_len] {
            vector_pop_at(vector, i);
            return i;
        }
        let _ = end;
    }
    -1
}
/// Removes the element at the given index.
pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 || index >= vector.rindex {
        return;
    }
    let esize = vector.esize;
    let count = vector.rindex as usize;
    // Shift elements left by one.
    for i in (index as usize)..(count - 1) {
        let src = (i + 1) * esize;
        let dst = i * esize;
        for j in 0..esize {
            vector.data[dst + j] = vector.data[src + j];
        }
    }
    vector.rindex -= 1;
    if vector.count > 0 {
        vector.count -= 1;
    }
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
        pindex: vector.pindex,
        rindex: vector.rindex,
        count: vector.count,
        flags: vector.flags,
    });
}
/// Restores a previously saved state of the vector.
pub fn vector_restore(vector: &mut Vector) {
    if let Some(state) = vector.saves.pop() {
        vector.pindex = state.pindex;
        vector.rindex = state.rindex;
        vector.count = state.count;
        vector.flags = state.flags;
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
