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
    saves: Option<Box<Vector>>,
}
// Function Declarations
/// Creates a new vector with elements of size `esize`.
pub fn vector_create(_esize: usize) -> Vector {
    let mut v = Vector {
        data: vec![0u8; _esize * VECTOR_ELEMENT_INCREMENT],
        pindex: 0,
        rindex: 0,
        mindex: VECTOR_ELEMENT_INCREMENT as i32,
        count: 0,
        flags: 0,
        esize: _esize,
        saves: None,
    };
    // saves is itself a vector of Vector-sized elements (no sub-saves)
    v.saves = Some(Box::new(Vector {
        data: vec![0u8; std::mem::size_of::<VectorSave>() * VECTOR_ELEMENT_INCREMENT],
        pindex: 0,
        rindex: 0,
        mindex: VECTOR_ELEMENT_INCREMENT as i32,
        count: 0,
        flags: 0,
        esize: std::mem::size_of::<VectorSave>(),
        saves: None,
    }));
    v
}

/// Internal save state (mirrors the scalar fields of Vector).
#[derive(Clone)]
struct VectorSave {
    pindex: i32,
    rindex: i32,
    mindex: i32,
    count: i32,
    flags: i32,
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

fn offset(vector: &Vector, index: i32) -> usize {
    index as usize * vector.esize
}

/// Frees the given vector (in Rust, typically done by dropping).
pub fn vector_free(_vector: Vector) {
    // drop
}
/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    let start = offset(_vector, _index);
    let end = start + _vector.esize;
    if end > _vector.data.len() {
        return None;
    }
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the element at the given index for peek operations, if in range.
pub fn vector_peek_ptr_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    if _index < 0 || _index > _vector.count {
        return None;
    }
    vector_at(_vector, _index)
}
/// Returns a reference to the next element to peek without incrementing the internal pointer.
pub fn vector_peek_no_increment(_vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _vector.pindex) {
        return None;
    }
    let start = offset(_vector, _vector.pindex);
    let end = start + _vector.esize;
    Some(&mut _vector.data[start..end])
}
/// Returns a reference to the next element to peek and increments the internal pointer.
pub fn vector_peek(_vector: &mut Vector) -> Option<&mut [u8]> {
    let pi = _vector.pindex;
    if !vector_in_bounds_for_at(_vector, pi) {
        return None;
    }
    if _vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        _vector.pindex -= 1;
    } else {
        _vector.pindex += 1;
    }
    let start = offset(_vector, pi);
    let end = start + _vector.esize;
    Some(&mut _vector.data[start..end])
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
    // In C this dereferences a pointer stored in the element. Here we just peek.
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
    let ri = _vector.rindex;
    let start = offset(_vector, ri);
    let end = start + _vector.esize;
    if end > _vector.data.len() {
        _vector.data.resize(end, 0);
    }
    let len = _vector.esize.min(_elem.len());
    _vector.data[start..start + len].copy_from_slice(&_elem[..len]);
    // zero-fill remainder if elem is shorter
    if len < _vector.esize {
        for b in &mut _vector.data[start + len..end] {
            *b = 0;
        }
    }
    _vector.rindex += 1;
    _vector.count += 1;
    if _vector.rindex >= _vector.mindex {
        vector_resize(_vector);
    }
}

fn vector_shift_right_in_bounds(_vector: &mut Vector, index: i32, amount: i32) {
    vector_resize_for_index(_vector, index, amount + (_vector.count - index));
    let src_start = offset(_vector, index);
    let dst_start = offset(_vector, index + amount);
    let bytes_to_move = (_vector.count - index) as usize * _vector.esize;
    // use copy within since src and dst may overlap
    _vector.data.resize(_vector.data.len().max(dst_start + bytes_to_move), 0);
    _vector.data.copy_within(src_start..src_start + bytes_to_move, dst_start);
    // zero the gap
    let gap_end = src_start + amount as usize * _vector.esize;
    for b in &mut _vector.data[src_start..gap_end] {
        *b = 0;
    }
    _vector.rindex += amount;
    _vector.count += amount;
}

/// Pushes a new element at a specific index.
pub fn vector_push_at(_vector: &mut Vector, _index: i32, _ptr: &[u8]) {
    vector_shift_right_in_bounds(_vector, _index, 1);
    let start = offset(_vector, _index);
    let len = _vector.esize.min(_ptr.len());
    _vector.data[start..start + len].copy_from_slice(&_ptr[..len]);
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
    let idx = _vector.rindex - 1;
    assert!(idx >= 0 && idx < _vector.mindex);
    vector_at(_vector, idx)
}
/// Returns a reference to the last element or `None`.
pub fn vector_back_or_null(_vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _vector.rindex - 1) {
        return None;
    }
    vector_at(_vector, _vector.rindex - 1)
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
    std::str::from_utf8(&_vec.data[..(_vec.count as usize * _vec.esize)]).ok()
}
/// Checks if the vector is empty.
pub fn vector_empty(_vector: &Vector) -> bool {
    _vector.count == 0
}
/// Clears the vector contents.
pub fn vector_clear(_vector: &mut Vector) {
    _vector.rindex = 0;
    _vector.count = 0;
    _vector.pindex = 0;
}
/// Returns the count of elements in the vector.
pub fn vector_count(_vector: &Vector) -> i32 {
    _vector.count
}
/// Reads data into the vector from a file pointer (stub; not fully safe in typical Rust).
pub fn vector_fread(_vector: &mut Vector, _amount: i32, mut _fp: std::fs::File) -> i32 {
    use std::io::Read;
    let mut byte = [0u8; 1];
    while _fp.read(&mut byte).unwrap_or(0) > 0 {
        vector_push(_vector, &byte);
    }
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
    // copy src data
    let src_bytes: Vec<u8> = _vector_src.data[..src_count as usize * _vector_src.esize].to_vec();
    vector_shift_right_in_bounds(_vector_dst, _dst_index, src_count);
    let start = offset(_vector_dst, _dst_index);
    _vector_dst.data[start..start + src_bytes.len()].copy_from_slice(&src_bytes);
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
    for i in 0.._vector.count {
        let start = offset(_vector, i);
        let end = start + _vector.esize;
        if end <= _vector.data.len() && _vector.data[start..end] == *_val {
            vector_pop_at(_vector, i);
            return i;
        }
    }
    -1
}
/// Removes the element at the given index.
pub fn vector_pop_at(_vector: &mut Vector, _index: i32) {
    let dst = offset(_vector, _index);
    let src = dst + _vector.esize;
    let end = offset(_vector, _vector.rindex);
    if src < end {
        _vector.data.copy_within(src..end, dst);
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
    let save = VectorSave {
        pindex: _vector.pindex,
        rindex: _vector.rindex,
        mindex: _vector.mindex,
        count: _vector.count,
        flags: _vector.flags,
    };
    let bytes = vector_save_to_bytes(&save);
    if let Some(ref mut saves) = _vector.saves {
        vector_push(saves, &bytes);
    }
}

fn vector_save_to_bytes(save: &VectorSave) -> Vec<u8> {
    let mut v = Vec::with_capacity(std::mem::size_of::<VectorSave>());
    v.extend_from_slice(&save.pindex.to_le_bytes());
    v.extend_from_slice(&save.rindex.to_le_bytes());
    v.extend_from_slice(&save.mindex.to_le_bytes());
    v.extend_from_slice(&save.count.to_le_bytes());
    v.extend_from_slice(&save.flags.to_le_bytes());
    v
}

fn vector_save_from_bytes(bytes: &[u8]) -> VectorSave {
    VectorSave {
        pindex: i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        rindex: i32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        mindex: i32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        count: i32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        flags: i32::from_le_bytes(bytes[16..20].try_into().unwrap()),
    }
}

/// Restores a previously saved state of the vector.
pub fn vector_restore(_vector: &mut Vector) {
    if let Some(ref mut saves) = _vector.saves {
        let back = vector_back(saves).expect("no save to restore");
        let save = vector_save_from_bytes(back);
        _vector.pindex = save.pindex;
        _vector.rindex = save.rindex;
        _vector.mindex = save.mindex;
        _vector.count = save.count;
        _vector.flags = save.flags;
        vector_pop(saves);
    }
}
/// Removes saved states from the vector.
pub fn vector_save_purge(_vector: &mut Vector) {
    if let Some(ref mut saves) = _vector.saves {
        vector_pop(saves);
    }
}
/// Returns the size of each element in the vector.
pub fn vector_element_size(_vector: &Vector) -> usize {
    _vector.esize
}
/// Clones the vector into a new one.
pub fn vector_clone(_vector: &Vector) -> Vector {
    let mut new_vec = _vector.clone();
    // saves are not cloned per the C code
    new_vec.saves = None;
    new_vec
}
