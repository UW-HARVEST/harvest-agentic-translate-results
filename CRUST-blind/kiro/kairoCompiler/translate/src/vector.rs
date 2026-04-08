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
    pub saves: Option<Box<Vector>>,
}

fn vector_in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn vector_in_bounds_for_pop(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.mindex
}

fn vector_resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }
    let new_size = (start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32) as usize * vector.esize;
    vector.data.resize(new_size, 0);
    vector.mindex = start_index + total_elements;
}

fn vector_resize_for(vector: &mut Vector, total_elements: i32) {
    let ri = vector.rindex;
    vector_resize_for_index(vector, ri, total_elements);
}

fn vector_resize(vector: &mut Vector) {
    vector_resize_for(vector, 0);
}

fn vector_create_no_saves(esize: usize) -> Vector {
    let mut v = Vector::default();
    v.data = vec![0u8; esize * VECTOR_ELEMENT_INCREMENT];
    v.mindex = VECTOR_ELEMENT_INCREMENT as i32;
    v.esize = esize;
    v
}

/// Creates a new vector with elements of size `esize`.
pub fn vector_create(_esize: usize) -> Vector {
    let mut v = vector_create_no_saves(_esize);
    v.saves = Some(Box::new(vector_create_no_saves(std::mem::size_of::<VectorSave>())));
    v
}

// A flattened save of vector state (without saves pointer)
#[derive(Clone)]
struct VectorSave {
    pindex: i32,
    rindex: i32,
    mindex: i32,
    count: i32,
    flags: i32,
    esize: usize,
}

/// Frees the given vector.
pub fn vector_free(_vector: Vector) {
    // drop
}

fn at_offset(vector: &Vector, index: i32) -> usize {
    index as usize * vector.esize
}

/// Returns a reference to the element at the given index.
pub fn vector_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    let start = at_offset(_vector, _index);
    let end = start + _vector.esize;
    if end <= _vector.data.len() {
        Some(&mut _vector.data[start..end])
    } else {
        None
    }
}

fn vector_at_ref(vector: &Vector, index: i32) -> Option<&[u8]> {
    let start = index as usize * vector.esize;
    let end = start + vector.esize;
    if end <= vector.data.len() {
        Some(&vector.data[start..end])
    } else {
        None
    }
}

/// Returns a reference to the element at the given index for peek operations.
pub fn vector_peek_ptr_at(_vector: &mut Vector, _index: i32) -> Option<&mut [u8]> {
    if _index < 0 || _index > _vector.count {
        return None;
    }
    vector_at(_vector, _index)
}

/// Returns a reference to the next element to peek without incrementing.
pub fn vector_peek_no_increment(_vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector_in_bounds_for_at(_vector, _vector.pindex) {
        return None;
    }
    let pi = _vector.pindex;
    vector_at(_vector, pi)
}

/// Returns a reference to the next element to peek and increments.
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
    vector_at(_vector, pi)
}

/// Returns a reference to the element at the given index without changing peek pointer.
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

/// Removes the last peeked element.
pub fn vector_pop_last_peek(_vector: &mut Vector) {
    assert!(_vector.pindex >= 1);
    let idx = _vector.pindex - 1;
    vector_pop_at(_vector, idx);
}

/// Returns a reference to the last pushed element for peek purposes (pointer dereference in C).
pub fn vector_peek_ptr(_vector: &mut Vector) -> Option<&mut [u8]> {
    // In C this dereferences a pointer stored in the vector element.
    // In our Rust version, we just return the raw bytes.
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

/// Pushes a new element onto the vector.
pub fn vector_push(_vector: &mut Vector, _elem: &[u8]) {
    let ri = _vector.rindex;
    let start = at_offset(_vector, ri);
    let end = start + _vector.esize;
    if end > _vector.data.len() {
        _vector.data.resize(end + _vector.esize * VECTOR_ELEMENT_INCREMENT, 0);
    }
    let copy_len = _elem.len().min(_vector.esize);
    _vector.data[start..start + copy_len].copy_from_slice(&_elem[..copy_len]);
    // Zero remaining bytes if elem is smaller than esize
    for i in start + copy_len..end {
        _vector.data[i] = 0;
    }
    _vector.rindex += 1;
    _vector.count += 1;
    if _vector.rindex >= _vector.mindex {
        vector_resize(_vector);
    }
}

fn vector_shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    vector_resize_for_index(vector, index, amount);
    let eindex = index + amount;
    let bytes_to_move = (vector.count - index) as usize * vector.esize;
    let src_start = index as usize * vector.esize;
    let dst_start = eindex as usize * vector.esize;
    if bytes_to_move > 0 && dst_start + bytes_to_move <= vector.data.len() && src_start + bytes_to_move <= vector.data.len() {
        vector.data.copy_within(src_start..src_start + bytes_to_move, dst_start);
    }
    let zero_end = (src_start + amount as usize * vector.esize).min(vector.data.len());
    for i in src_start..zero_end {
        vector.data[i] = 0;
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
pub fn vector_push_at(_vector: &mut Vector, _index: i32, _ptr: &[u8]) {
    vector_shift_right(_vector, _index, 1);
    let start = _index as usize * _vector.esize;
    let copy_len = _ptr.len().min(_vector.esize);
    _vector.data[start..start + copy_len].copy_from_slice(&_ptr[..copy_len]);
}

/// Removes the last element from the vector.
pub fn vector_pop(_vector: &mut Vector) {
    _vector.rindex -= 1;
    _vector.count -= 1;
    assert!(vector_in_bounds_for_pop(_vector, _vector.rindex));
}

/// Removes the peeked element from the vector.
pub fn vector_peek_pop(_vector: &mut Vector) {
    let pi = _vector.pindex;
    vector_pop_at(_vector, pi);
}

/// Returns a reference to the last element in the vector.
pub fn vector_back(_vector: &mut Vector) -> Option<&mut [u8]> {
    let idx = _vector.rindex - 1;
    assert!(vector_in_bounds_for_pop(_vector, idx));
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

/// Returns a reference to the last element (pointer dereference in C).
pub fn vector_back_ptr(_vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(_vector)
}

/// Returns a reference to the last element or null (pointer dereference in C).
pub fn vector_back_ptr_or_null(_vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back_or_null(_vector)
}

/// Returns a string slice representation from the vector.
pub fn vector_string(_vec: &Vector) -> Option<&str> {
    let end = _vec.rindex as usize * _vec.esize;
    let slice = if end <= _vec.data.len() { &_vec.data[..end] } else { &_vec.data };
    // Find first null byte or use whole slice
    let len = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    std::str::from_utf8(&slice[..len]).ok()
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

/// Reads data into the vector from a file.
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
    // Get source data
    let total_bytes = src_count as usize * _vector_src.esize;
    let src_data: Vec<u8> = _vector_src.data[..total_bytes].to_vec();
    
    vector_shift_right(_vector_dst, _dst_index, src_count);
    let start = _dst_index as usize * _vector_dst.esize;
    _vector_dst.data[start..start + total_bytes].copy_from_slice(&src_data);
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
    vector_set_peek_pointer(_vector, 0);
    let mut index = 0i32;
    loop {
        let pi = _vector.pindex;
        if !vector_in_bounds_for_at(_vector, pi) {
            break;
        }
        // Compare element bytes with val
        let start = pi as usize * _vector.esize;
        let end = start + _vector.esize;
        let matches = if end <= _vector.data.len() && _val.len() >= _vector.esize {
            _vector.data[start..end] == _val[.._vector.esize]
        } else {
            false
        };
        _vector.pindex += 1;
        if matches {
            vector_pop_at(_vector, index);
            break;
        }
        index += 1;
    }
    vector_set_peek_pointer(_vector, old_pp);
    index
}

/// Removes the element at the given index.
pub fn vector_pop_at(_vector: &mut Vector, _index: i32) {
    let dst_start = _index as usize * _vector.esize;
    let next_start = dst_start + _vector.esize;
    let end = _vector.rindex as usize * _vector.esize;
    if next_start < end {
        _vector.data.copy_within(next_start..end, dst_start);
    }
    _vector.count -= 1;
    _vector.rindex -= 1;
}

/// Moves the peek pointer back.
pub fn vector_peek_back(_vector: &mut Vector) {
    _vector.pindex -= 1;
}

/// Returns the current index.
pub fn vector_current_index(_vector: &Vector) -> i32 {
    _vector.rindex
}

/// Saves the current state of the vector.
pub fn vector_save(_vector: &mut Vector) {
    let save = VectorSave {
        pindex: _vector.pindex,
        rindex: _vector.rindex,
        mindex: _vector.mindex,
        count: _vector.count,
        flags: _vector.flags,
        esize: _vector.esize,
    };
    let save_bytes = unsafe {
        std::slice::from_raw_parts(
            &save as *const VectorSave as *const u8,
            std::mem::size_of::<VectorSave>(),
        )
    };
    if let Some(ref mut saves) = _vector.saves {
        vector_push(saves, save_bytes);
    }
}

/// Restores a previously saved state of the vector.
pub fn vector_restore(_vector: &mut Vector) {
    let save_size = std::mem::size_of::<VectorSave>();
    let save: VectorSave = {
        let saves = _vector.saves.as_mut().expect("no saves");
        let idx = saves.rindex - 1;
        let start = idx as usize * saves.esize;
        let bytes = &saves.data[start..start + save_size];
        unsafe { std::ptr::read(bytes.as_ptr() as *const VectorSave) }
    };
    // Pop from saves
    if let Some(ref mut saves) = _vector.saves {
        vector_pop(saves);
    }
    // Restore state but keep saves and data
    _vector.pindex = save.pindex;
    _vector.rindex = save.rindex;
    _vector.mindex = save.mindex;
    _vector.count = save.count;
    _vector.flags = save.flags;
    // esize shouldn't change, but match C behavior
    _vector.esize = save.esize;
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
    let total = _vector.count as usize * _vector.esize;
    let new_cap = (_vector.count as usize + VECTOR_ELEMENT_INCREMENT) * _vector.esize;
    let mut new_data = vec![0u8; new_cap];
    if total > 0 && total <= _vector.data.len() {
        new_data[..total].copy_from_slice(&_vector.data[..total]);
    }
    Vector {
        data: new_data,
        pindex: _vector.pindex,
        rindex: _vector.rindex,
        mindex: _vector.mindex,
        count: _vector.count,
        flags: _vector.flags,
        esize: _vector.esize,
        saves: None,
    }
}
