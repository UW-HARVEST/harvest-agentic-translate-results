// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;
// Structs
/// A safe, idiomatic representation of the original `struct vector`.
/// This struct is left opaque for now; details will be implemented later.
#[derive(Debug, Default, Clone)]
pub struct Vector {
    pub(crate) data: Vec<u8>,
    pub(crate) pindex: i32,
    pub(crate) rindex: i32,
    pub(crate) mindex: i32,
    pub(crate) count: i32,
    pub(crate) flags: i32,
    pub(crate) esize: usize,
    pub(crate) saves: Vec<Vector>,
}

fn vector_in_bounds_for_at(v: &Vector, index: i32) -> bool {
    index >= 0 && index < v.rindex
}

fn vector_in_bounds_for_pop(v: &Vector, index: i32) -> bool {
    index >= 0 && index < v.mindex
}

// Function Declarations
/// Creates a new vector with elements of size `esize`.
pub fn vector_create(_esize: usize) -> Vector {
    let mut v = Vector::default();
    v.esize = _esize;
    v.mindex = VECTOR_ELEMENT_INCREMENT as i32;
    v.rindex = 0;
    v.pindex = 0;
    v.count = 0;
    v.flags = 0;
    v.data = vec![0u8; _esize * VECTOR_ELEMENT_INCREMENT];
    v.saves = Vec::new();
    v
}
/// Frees the given vector (in Rust, typically done by dropping).
pub fn vector_free(_vector: Vector) {
    // Drop happens automatically.
    drop(_vector);
}
/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    if _index < 0 {
        return None;
    }
    let esize = _vector.esize;
    let start = (_index as usize) * esize;
    let end = start + esize;
    if end > _vector.data.len() {
        // grow the underlying buffer to fit
        _vector.data.resize(end + esize * VECTOR_ELEMENT_INCREMENT, 0);
    }
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the element at the given index for peek operations, if in range.
pub fn vector_peek_ptr_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    if _index < 0 || _index > _vector.count {
        return None;
    }
    let esize = _vector.esize;
    let start = (_index as usize) * esize;
    let end = start + esize;
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
    let esize = _vector.esize;
    let start = (_vector.pindex as usize) * esize;
    let end = start + esize;
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(_vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _vector.pindex) {
        return None;
    }
    let esize = _vector.esize;
    let start = (_vector.pindex as usize) * esize;
    let end = start + esize;
    if (_vector.flags & VECTOR_FLAG_PEEK_DECREMENT) != 0 {
        _vector.pindex -= 1;
    } else {
        _vector.pindex += 1;
    }
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the element at the given index without changing the peek pointer.
pub fn vector_peek_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _index) {
        return None;
    }
    let esize = _vector.esize;
    let start = (_index as usize) * esize;
    let end = start + esize;
    Some(&mut _vector.data[start..end])
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
    // In C, vector_peek_ptr does *(void**)vector_peek which returns the contained pointer.
    // In Rust, since elements are byte slices, we just return the same slice.
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
    let esize = _vector.esize;
    let start = (_vector.rindex as usize) * esize;
    let end = start + esize;
    if end > _vector.data.len() {
        _vector.data.resize(end + esize * VECTOR_ELEMENT_INCREMENT, 0);
    }
    let to_copy = esize.min(_elem.len());
    _vector.data[start..start + to_copy].copy_from_slice(&_elem[..to_copy]);
    if to_copy < esize {
        // Zero out remaining bytes
        for i in start + to_copy..end {
            _vector.data[i] = 0;
        }
    }
    _vector.rindex += 1;
    _vector.count += 1;
    if _vector.rindex >= _vector.mindex {
        // resize - ensure data has enough capacity
        let needed = (_vector.rindex as usize + VECTOR_ELEMENT_INCREMENT) * esize;
        if needed > _vector.data.len() {
            _vector.data.resize(needed, 0);
        }
        _vector.mindex = _vector.rindex + VECTOR_ELEMENT_INCREMENT as i32;
    }
}
/// Pushes a new element at a specific index.
pub fn vector_push_at(_vector: &mut Vector, _index: i32, _ptr: &[u8]) {
    // shift right by 1, then write at index
    let esize = _vector.esize;
    let idx = _index as usize;
    if _index < _vector.rindex {
        // in-bounds shift right
        let total_bytes = ((_vector.rindex as usize) - idx) * esize;
        // ensure enough room
        let needed = (_vector.rindex as usize + 1 + VECTOR_ELEMENT_INCREMENT) * esize;
        if needed > _vector.data.len() {
            _vector.data.resize(needed, 0);
        }
        // shift bytes
        let src_start = idx * esize;
        let dst_start = src_start + esize;
        // do shift in reverse to avoid overlap issues
        for i in (0..total_bytes).rev() {
            _vector.data[dst_start + i] = _vector.data[src_start + i];
        }
        _vector.rindex += 1;
        _vector.count += 1;
    } else {
        // out of bounds: stretch up to index+1
        let target = (_index as i32) + 1;
        let needed = (target as usize + VECTOR_ELEMENT_INCREMENT) * esize;
        if needed > _vector.data.len() {
            _vector.data.resize(needed, 0);
        }
        _vector.count = target;
        _vector.rindex = target;
        if _vector.rindex > _vector.mindex {
            _vector.mindex = _vector.rindex + VECTOR_ELEMENT_INCREMENT as i32;
        }
    }
    // copy element at index
    let start = idx * esize;
    let end = start + esize;
    if end > _vector.data.len() {
        _vector.data.resize(end, 0);
    }
    let to_copy = esize.min(_ptr.len());
    _vector.data[start..start + to_copy].copy_from_slice(&_ptr[..to_copy]);
    if to_copy < esize {
        for i in start + to_copy..end {
            _vector.data[i] = 0;
        }
    }
}
/// Removes the last element from the vector.
pub fn vector_pop(_vector: &mut Vector) {
    _vector.rindex -= 1;
    _vector.count -= 1;
    assert!(vector_in_bounds_for_pop(_vector, _vector.rindex));
}
/// Removes the peeked element from the vector.
pub fn vector_peek_pop(_vector: &mut Vector) {
    vector_pop_at(_vector, _vector.pindex);
}
/// Returns a reference to the last element in the vector (if any).
pub fn vector_back(_vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = _vector.rindex - 1;
    assert!(vector_in_bounds_for_pop(_vector, idx));
    let esize = _vector.esize;
    let start = (idx as usize) * esize;
    let end = start + esize;
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(_vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = _vector.rindex - 1;
    if !vector_in_bounds_for_at(_vector, idx) {
        return None;
    }
    let esize = _vector.esize;
    let start = (idx as usize) * esize;
    let end = start + esize;
    Some(&mut _vector.data[start..end])
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
    // Find null terminator or use full data
    let end = _vec.data.iter().position(|&b| b == 0).unwrap_or(_vec.data.len());
    std::str::from_utf8(&_vec.data[..end]).ok()
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
    use std::io::Read;
    let mut fp = _fp;
    let mut buf = [0u8; 1];
    while let Ok(n) = fp.read(&mut buf) {
        if n == 0 {
            break;
        }
        // push the byte read (matching C behaviour, which pushes its read amount)
        let bytes = (n as u64).to_le_bytes();
        let elem_bytes = &bytes[..(_vector.esize.min(8))];
        let mut elem = vec![0u8; _vector.esize];
        elem[..elem_bytes.len()].copy_from_slice(elem_bytes);
        vector_push(_vector, &elem);
    }
    0
}
/// Returns a reference to the underlying data of the vector.
pub fn vector_data_ptr(_vector: &Vector) -> &[u8] {
    let used = (_vector.count as usize) * _vector.esize;
    &_vector.data[..used.min(_vector.data.len())]
}
/// Inserts data from `vector_src` into `vector_dst` at `dst_index`.
pub fn vector_insert(_vector_dst: &mut Vector, _vector_src: &Vector, _dst_index: i32) -> i32 {
    if _vector_dst.esize != _vector_src.esize {
        return -1;
    }
    let esize = _vector_dst.esize;
    let total = _vector_src.count as usize;
    if total == 0 {
        return 0;
    }
    let idx = _dst_index as usize;
    // shift right by 'total'
    if (_dst_index as i32) < _vector_dst.rindex {
        let bytes_to_move = ((_vector_dst.rindex as usize) - idx) * esize;
        let needed = (_vector_dst.rindex as usize + total + VECTOR_ELEMENT_INCREMENT) * esize;
        if needed > _vector_dst.data.len() {
            _vector_dst.data.resize(needed, 0);
        }
        let src_start = idx * esize;
        let dst_start = src_start + total * esize;
        for i in (0..bytes_to_move).rev() {
            _vector_dst.data[dst_start + i] = _vector_dst.data[src_start + i];
        }
        _vector_dst.rindex += total as i32;
        _vector_dst.count += total as i32;
    } else {
        // stretch
        let target = (_dst_index as i32) + total as i32;
        let needed = (target as usize + VECTOR_ELEMENT_INCREMENT) * esize;
        if needed > _vector_dst.data.len() {
            _vector_dst.data.resize(needed, 0);
        }
        _vector_dst.count = target;
        _vector_dst.rindex = target;
        if _vector_dst.rindex > _vector_dst.mindex {
            _vector_dst.mindex = _vector_dst.rindex + VECTOR_ELEMENT_INCREMENT as i32;
        }
    }
    // copy src bytes into dst at idx
    let total_bytes = total * esize;
    let dst_start = idx * esize;
    if dst_start + total_bytes > _vector_dst.data.len() {
        _vector_dst.data.resize(dst_start + total_bytes, 0);
    }
    _vector_dst.data[dst_start..dst_start + total_bytes]
        .copy_from_slice(&_vector_src.data[..total_bytes]);
    0
}
/// Removes the element that matches the given data address from the vector.
pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    // Calculate index based on pointer offset from start of data buffer
    let base = _vector.data.as_ptr() as usize;
    let addr = _address as usize;
    let esize = _vector.esize;
    let index = ((addr - base) / esize) as i32;
    vector_pop_at(_vector, index);
    index
}
/// Removes the first element that matches the given value.
pub fn vector_pop_value(_vector: &mut Vector, _val: &[u8]) -> i32 {
    let esize = _vector.esize;
    let count = _vector.count;
    let mut found_index: i32 = -1;
    for i in 0..count {
        let start = (i as usize) * esize;
        let end = start + esize;
        if &_vector.data[start..end] == &_val[..esize.min(_val.len())] {
            found_index = i;
            break;
        }
    }
    if found_index >= 0 {
        vector_pop_at(_vector, found_index);
    }
    found_index
}
/// Removes the element at the given index.
pub fn vector_pop_at(_vector: &mut Vector, _index: i32) {
    if _index < 0 || _index >= _vector.rindex {
        return;
    }
    let esize = _vector.esize;
    let dst_start = (_index as usize) * esize;
    let end_pos = (_vector.rindex as usize) * esize;
    let next_pos = dst_start + esize;
    if next_pos < end_pos {
        // shift left
        let total = end_pos - next_pos;
        for i in 0..total {
            _vector.data[dst_start + i] = _vector.data[next_pos + i];
        }
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
    let mut saved = _vector.clone();
    saved.saves = Vec::new();
    _vector.saves.push(saved);
}
/// Restores a previously saved state of the vector.
pub fn vector_restore(_vector: &mut Vector) {
    if let Some(saved) = _vector.saves.pop() {
        let preserved_saves = std::mem::take(&mut _vector.saves);
        let preserved_data = std::mem::take(&mut _vector.data);
        _vector.pindex = saved.pindex;
        _vector.rindex = saved.rindex;
        _vector.mindex = saved.mindex;
        _vector.count = saved.count;
        _vector.flags = saved.flags;
        _vector.esize = saved.esize;
        _vector.data = preserved_data; // data is permanent, not restored
        _vector.saves = preserved_saves;
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
    let mut new_vec = _vector.clone();
    new_vec.saves = Vec::new();
    new_vec
}
