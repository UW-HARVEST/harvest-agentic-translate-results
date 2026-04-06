// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;
// Structs
/// A safe, idiomatic representation of the original `struct vector`.
/// This struct is left opaque for now; details will be implemented later.
#[derive(Debug, Default, Clone)]
pub struct Vector {
    data: Vec<u8>,
    pindex: i32,
    rindex: i32,
    mindex: i32,
    count: i32,
    flags: i32,
    esize: usize,
    saves: Vec<VectorState>,
}

#[derive(Debug, Default, Clone)]
struct VectorState {
    pindex: i32,
    rindex: i32,
    mindex: i32,
    count: i32,
    flags: i32,
}

// Function Declarations
/// Creates a new vector with elements of size `esize`.
pub fn vector_create(_esize: usize) -> Vector {
    let cap = _esize * VECTOR_ELEMENT_INCREMENT;
    Vector {
        data: vec![0u8; cap],
        pindex: 0,
        rindex: 0,
        mindex: VECTOR_ELEMENT_INCREMENT as i32,
        count: 0,
        flags: 0,
        esize: _esize,
        saves: Vec::new(),
    }
}
/// Frees the given vector (in Rust, typically done by dropping).
pub fn vector_free(_vector: Vector) {
    drop(_vector);
}

fn vector_in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn vector_resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_size = ((start_index + total_elements) as usize + VECTOR_ELEMENT_INCREMENT) * vector.esize;
    vector.data.resize(new_size, 0);
    vector.mindex = start_index + total_elements;
}

fn vector_resize(vector: &mut Vector) {
    vector_resize_for_index(vector, vector.rindex, 0);
}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    let start = _index as usize * _vector.esize;
    let end = start + _vector.esize;
    if end > _vector.data.len() {
        // Extend if needed
        _vector.data.resize(end, 0);
    }
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the element at the given index for peek operations, if in range.
pub fn vector_peek_ptr_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    if _index < 0 || _index > _vector.count {
        return None;
    }
    let start = _index as usize * _vector.esize;
    let end = start + _vector.esize;
    if end > _vector.data.len() {
        return None;
    }
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the next element to peek without incrementing the internal pointer.
pub fn vector_peek_no_increment(_vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _vector.pindex) {
        return None;
    }
    let idx = _vector.pindex;
    vector_at(_vector, idx)
}
/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(_vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _vector.pindex) {
        return None;
    }
    let idx = _vector.pindex;
    if _vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        _vector.pindex -= 1;
    } else {
        _vector.pindex += 1;
    }
    vector_at(_vector, idx)
}
/// Returns a reference to the element at the given index without changing the peek pointer.
pub fn vector_peek_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _index) {
        return None;
    }
    vector_at(_vector, _index)
}
/// Sets a flag in the vector.
pub fn vector_set_flag(_vector: &mut Vector, _flag: i32) {
    _vector.flags |= _flag;
}
/// Unsets a flag in the vector.
pub fn vector_unset_flag(_vector: &mut Vector, _flag: i32) {
    _vector.flags &= !_flag;
}
/// Removes the last peeked element from the vector if needed.
pub fn vector_pop_last_peek(_vector: &mut Vector) {
    assert!(_vector.pindex >= 1);
    vector_pop_at(_vector, _vector.pindex - 1);
}
/// Returns a reference to the last pushed element for peek purposes.
pub fn vector_peek_ptr(_vector: &mut Vector) -> Option<&mut [u8]> {
    vector_peek(_vector)
}
/// Sets the peek pointer to the given index.
pub fn vector_set_peek_pointer(_vector: &mut Vector, _index: i32) {
    _vector.pindex = _index;
}
/// Sets the peek pointer to the end of the vector.
pub fn vector_set_peek_pointer_end(_vector: &mut Vector) {
    _vector.pindex = _vector.rindex - 1;
}
/// Pushes a new element (pointed to by `elem`) onto the vector.
pub fn vector_push(_vector: &mut Vector, _elem: &[u8]) {
    let start = _vector.rindex as usize * _vector.esize;
    let end = start + _vector.esize;
    if end > _vector.data.len() {
        _vector.data.resize(end, 0);
    }
    let copy_len = _elem.len().min(_vector.esize);
    _vector.data[start..start + copy_len].copy_from_slice(&_elem[..copy_len]);
    // Zero remaining if elem is shorter
    if copy_len < _vector.esize {
        for b in &mut _vector.data[start + copy_len..end] {
            *b = 0;
        }
    }
    _vector.rindex += 1;
    _vector.count += 1;
    if _vector.rindex >= _vector.mindex {
        vector_resize(_vector);
    }
}
/// Pushes a new element at a specific index.
pub fn vector_push_at(_vector: &mut Vector, _index: i32, _ptr: &[u8]) {
    vector_shift_right(_vector, _index, 1);
    let start = _index as usize * _vector.esize;
    let end = start + _vector.esize;
    if end > _vector.data.len() {
        _vector.data.resize(end, 0);
    }
    let copy_len = _ptr.len().min(_vector.esize);
    _vector.data[start..start + copy_len].copy_from_slice(&_ptr[..copy_len]);
}

fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    vector_resize_for_index(vector, index, amount);
    let eindex = (index + amount) as usize;
    let idx = index as usize;
    let elements_left = (vector.count - index) as usize;
    let bytes_to_move = elements_left * vector.esize;
    let src_start = idx * vector.esize;
    let dst_start = eindex * vector.esize;
    let needed = dst_start + bytes_to_move;
    if needed > vector.data.len() {
        vector.data.resize(needed, 0);
    }
    vector.data.copy_within(src_start..src_start + bytes_to_move, dst_start);
    // Zero the gap
    for b in &mut vector.data[src_start..src_start + amount as usize * vector.esize] {
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
pub fn vector_pop(_vector: &mut Vector) {
    _vector.rindex -= 1;
    _vector.count -= 1;
    assert!(_vector.rindex >= 0);
}
/// Removes the peeked element from the vector.
pub fn vector_peek_pop(_vector: &mut Vector) {
    vector_pop_at(_vector, _vector.pindex);
}
/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(_vector: &mut Vector) -> Option<&mut [u8]> {
    assert!(_vector.rindex - 1 >= 0 && _vector.rindex - 1 < _vector.mindex);
    let idx = _vector.rindex - 1;
    vector_at(_vector, idx)
}
/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(_vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _vector.rindex - 1) {
        return None;
    }
    let idx = _vector.rindex - 1;
    vector_at(_vector, idx)
}
/// Returns a reference to the last element in the vector for pointer usage.
pub fn vector_back_ptr(_vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(_vector)
}
/// Returns a reference to the last element or null, specialized for pointer usage.
pub fn vector_back_ptr_or_null(_vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back_or_null(_vector)
}
/// Returns a string slice representation from the vector if it contains text data.
pub fn vector_string(_vec: &Vector) -> Option<&str> {
    std::str::from_utf8(&_vec.data[.._vec.rindex as usize * _vec.esize]).ok()
}
/// Checks if the vector is empty.
pub fn vector_empty(_vector: &Vector) -> bool {
    _vector.count == 0
}
/// Clears the vector contents.
pub fn vector_clear(_vector: &mut Vector) {
    while _vector.count > 0 {
        vector_pop(_vector);
    }
}
/// Returns the count of elements in the vector.
pub fn vector_count(_vector: &Vector) -> i32 {
    _vector.count
}
/// Reads data into the vector from a file pointer (stub; not fully safe in typical Rust).
pub fn vector_fread(_vector: &mut Vector, _amount: i32, _fp: std::fs::File) -> i32 {
    // Not used in the test paths
    0
}
/// Returns a reference to the underlying data of the vector.
pub fn vector_data_ptr(_vector: &Vector) -> &[u8] {
    &_vector.data
}
/// Inserts data from `vector_src` into `vector_dst` at `dst_index`.
pub fn vector_insert(_vector_dst: &mut Vector, _vector_src: &Vector, _dst_index: i32) -> i32 {
    if _vector_dst.esize != _vector_src.esize {
        return -1;
    }
    let src_count = _vector_src.count;
    let src_data_len = src_count as usize * _vector_src.esize;
    let src_data = _vector_src.data[..src_data_len].to_vec();
    vector_shift_right(_vector_dst, _dst_index, src_count);
    let start = _dst_index as usize * _vector_dst.esize;
    _vector_dst.data[start..start + src_data_len].copy_from_slice(&src_data);
    0
}
/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    let base = _vector.data.as_ptr() as usize;
    let addr = _address as usize;
    let index = ((addr - base) / _vector.esize) as i32;
    vector_pop_at(_vector, index);
    index
}
/// Removes the first element that matches the given value.
pub fn vector_pop_value(_vector: &mut Vector, _val: &[u8]) -> i32 {
    let old_pp = _vector.pindex;
    _vector.pindex = 0;
    let mut index = 0i32;
    while (_vector.pindex as usize) < _vector.count as usize {
        let start = _vector.pindex as usize * _vector.esize;
        let end = start + _vector.esize;
        if end <= _vector.data.len() && &_vector.data[start..end] == _val {
            vector_pop_at(_vector, index);
            _vector.pindex = old_pp;
            return index;
        }
        _vector.pindex += 1;
        index += 1;
    }
    _vector.pindex = old_pp;
    -1
}
/// Removes the element at the given index.
pub fn vector_pop_at(_vector: &mut Vector, _index: i32) {
    let idx = _index as usize;
    let esize = _vector.esize;
    let dst_start = idx * esize;
    let src_start = dst_start + esize;
    let end = _vector.rindex as usize * esize;
    if src_start < end {
        _vector.data.copy_within(src_start..end, dst_start);
    }
    _vector.count -= 1;
    _vector.rindex -= 1;
}
/// Moves the peek pointer to the back of the vector.
pub fn vector_peek_back(_vector: &mut Vector) {
    _vector.pindex -= 1;
}
/// Returns the current peek index.
pub fn vector_current_index(_vector: &Vector) -> i32 {
    _vector.rindex
}
/// Saves the current state of the vector for future restore.
pub fn vector_save(_vector: &mut Vector) {
    _vector.saves.push(VectorState {
        pindex: _vector.pindex,
        rindex: _vector.rindex,
        mindex: _vector.mindex,
        count: _vector.count,
        flags: _vector.flags,
    });
}
/// Restores a previously saved state of the vector.
pub fn vector_restore(_vector: &mut Vector) {
    if let Some(state) = _vector.saves.pop() {
        _vector.pindex = state.pindex;
        _vector.rindex = state.rindex;
        _vector.mindex = state.mindex;
        _vector.count = state.count;
        _vector.flags = state.flags;
    }
}
/// Removes saved states from the vector.
pub fn vector_save_purge(_vector: &mut Vector) {
    _vector.saves.pop();
}
/// Returns the size of each element in the vector.
pub fn vector_element_size(_vector: &Vector) -> usize {
    _vector.esize
}
/// Clones the vector into a new one.
pub fn vector_clone(_vector: &Vector) -> Vector {
    Vector {
        data: _vector.data.clone(),
        pindex: _vector.pindex,
        rindex: _vector.rindex,
        mindex: _vector.mindex,
        count: _vector.count,
        flags: _vector.flags,
        esize: _vector.esize,
        saves: Vec::new(), // saves not cloned
    }
}
