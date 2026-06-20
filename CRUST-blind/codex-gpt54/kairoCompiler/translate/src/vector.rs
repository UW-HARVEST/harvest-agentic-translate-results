use std::fs::File;
use std::io::Read;

pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

#[derive(Debug, Default, Clone)]
struct VectorState {
    pindex: i32,
    rindex: i32,
    mindex: i32,
    count: i32,
    flags: i32,
}

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

fn bytes_for(vector: &Vector, elements: usize) -> usize {
    elements.saturating_mul(vector.esize)
}

fn in_bounds_for_at(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.rindex
}

fn in_bounds_for_pop(vector: &Vector, index: i32) -> bool {
    index >= 0 && index < vector.mindex
}

fn ensure_len(vector: &mut Vector, elements: usize) {
    let needed = bytes_for(vector, elements);
    if vector.data.len() < needed {
        vector.data.resize(needed, 0);
    }
}

fn resize_for_index(vector: &mut Vector, start_index: i32, total_elements: i32) {
    if start_index + total_elements < vector.mindex {
        return;
    }

    let alloc_elements = (start_index + total_elements).max(0) as usize + VECTOR_ELEMENT_INCREMENT;
    ensure_len(vector, alloc_elements);
    vector.mindex = start_index + total_elements;
}

fn resize_for(vector: &mut Vector, total_elements: i32) {
    resize_for_index(vector, vector.rindex, total_elements);
}

fn resize(vector: &mut Vector) {
    resize_for(vector, 0);
}

fn range_for(vector: &Vector, index: i32) -> Option<std::ops::Range<usize>> {
    if index < 0 || vector.esize == 0 {
        return None;
    }

    let start = index as usize * vector.esize;
    let end = start.checked_add(vector.esize)?;
    if end > vector.data.len() {
        return None;
    }

    Some(start..end)
}

fn data_end(vector: &Vector) -> usize {
    bytes_for(vector, vector.rindex.max(0) as usize)
}

fn elements_until_end(vector: &Vector, index: i32) -> i32 {
    vector.count - index
}

fn shift_right_in_bounds_no_increment(vector: &mut Vector, index: i32, amount: i32) {
    resize_for_index(vector, index, amount);
    let eindex = index + amount;
    let bytes_to_move = (elements_until_end(vector, index).max(0) as usize).saturating_mul(vector.esize);
    if bytes_to_move == 0 {
        return;
    }

    let src_start = index.max(0) as usize * vector.esize;
    let dst_start = eindex.max(0) as usize * vector.esize;
    let src_end = src_start.saturating_add(bytes_to_move).min(vector.data.len());
    let move_len = src_end.saturating_sub(src_start);
    if move_len > 0 {
        vector.data.copy_within(src_start..src_start + move_len, dst_start);
    }

    let zero_start = src_start;
    let zero_end = zero_start
        .saturating_add(amount.max(0) as usize * vector.esize)
        .min(vector.data.len());
    vector.data[zero_start..zero_end].fill(0);
}

fn shift_right_in_bounds(vector: &mut Vector, index: i32, amount: i32) {
    shift_right_in_bounds_no_increment(vector, index, amount);
    vector.rindex += amount;
    vector.count += amount;
}

fn stretch(vector: &mut Vector, index: i32) {
    if index < vector.rindex {
        return;
    }

    resize_for_index(vector, index, 0);
    vector.count = index;
    vector.rindex = index;
}

fn shift_right(vector: &mut Vector, index: i32, amount: i32) {
    if index < vector.rindex {
        shift_right_in_bounds(vector, index, amount);
        return;
    }

    stretch(vector, index + amount);
    shift_right_in_bounds_no_increment(vector, index, amount);
}

pub fn vector_create(esize: usize) -> Vector {
    let mut vector = Vector {
        data: vec![0; esize.saturating_mul(VECTOR_ELEMENT_INCREMENT)],
        pindex: 0,
        rindex: 0,
        mindex: VECTOR_ELEMENT_INCREMENT as i32,
        count: 0,
        flags: 0,
        esize,
        saves: Vec::new(),
    };
    ensure_len(&mut vector, VECTOR_ELEMENT_INCREMENT);
    vector
}

pub fn vector_free(_vector: Vector) {}

pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    let range = range_for(vector, index)?;
    Some(&mut vector.data[range])
}

pub fn vector_peek_ptr_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 || index > vector.count {
        return None;
    }
    vector_at(vector, index)
}

pub fn vector_peek_no_increment(vector: &mut Vector) -> Option<&mut [u8]> {
    if !in_bounds_for_at(vector, vector.pindex) {
        return None;
    }
    vector_at(vector, vector.pindex)
}

pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    let index = vector.pindex;
    if !in_bounds_for_at(vector, index) {
        return None;
    }

    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }

    vector_at(vector, index)
}

pub fn vector_peek_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if !in_bounds_for_at(vector, index) {
        return None;
    }
    vector_at(vector, index)
}

pub fn vector_set_flag(vector: &mut Vector, flag: i32) {
    vector.flags |= flag;
}

pub fn vector_unset_flag(vector: &mut Vector, flag: i32) {
    vector.flags &= !flag;
}

pub fn vector_pop_last_peek(vector: &mut Vector) {
    if vector.pindex >= 1 {
        vector_pop_at(vector, vector.pindex - 1);
    }
}

pub fn vector_peek_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_peek(vector)
}

pub fn vector_set_peek_pointer(vector: &mut Vector, index: i32) {
    vector.pindex = index;
}

pub fn vector_set_peek_pointer_end(vector: &mut Vector) {
    vector_set_peek_pointer(vector, vector.rindex - 1);
}

pub fn vector_push(vector: &mut Vector, elem: &[u8]) {
    let index = vector.rindex;
    let esize = vector.esize;
    resize_for_index(vector, index, 0);
    if let Some(dst) = vector_at(vector, index) {
        dst.fill(0);
        let count = elem.len().min(esize);
        dst[..count].copy_from_slice(&elem[..count]);
    }

    vector.rindex += 1;
    vector.count += 1;

    if vector.rindex >= vector.mindex {
        resize(vector);
    }
}

pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    let esize = vector.esize;
    shift_right(vector, index, 1);
    if let Some(dst) = vector_at(vector, index) {
        dst.fill(0);
        let count = ptr.len().min(esize);
        dst[..count].copy_from_slice(&ptr[..count]);
    }
}

pub fn vector_pop(vector: &mut Vector) {
    vector.rindex -= 1;
    vector.count -= 1;
    if !in_bounds_for_pop(vector, vector.rindex) {
        vector.rindex = vector.rindex.max(0);
        vector.count = vector.count.max(0);
    }
}

pub fn vector_peek_pop(vector: &mut Vector) {
    vector_pop_at(vector, vector.pindex);
}

pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    if !in_bounds_for_pop(vector, vector.rindex - 1) {
        return None;
    }
    vector_at(vector, vector.rindex - 1)
}

pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    if !in_bounds_for_at(vector, vector.rindex - 1) {
        return None;
    }
    vector_at(vector, vector.rindex - 1)
}

pub fn vector_back_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}

pub fn vector_back_ptr_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back_or_null(vector)
}

pub fn vector_string(vec: &Vector) -> Option<&str> {
    let nul = vec.data.iter().position(|b| *b == 0).unwrap_or(vec.data.len());
    std::str::from_utf8(&vec.data[..nul]).ok()
}

pub fn vector_empty(vector: &Vector) -> bool {
    vector_count(vector) == 0
}

pub fn vector_clear(vector: &mut Vector) {
    while vector_count(vector) > 0 {
        vector_pop(vector);
    }
}

pub fn vector_count(vector: &Vector) -> i32 {
    vector.count
}

pub fn vector_fread(vector: &mut Vector, _amount: i32, mut fp: File) -> i32 {
    let mut buf = [0u8; 1];
    while let Ok(read) = fp.read(&mut buf) {
        if read == 0 {
            break;
        }
        vector_push(vector, &buf[..read]);
    }
    0
}

pub fn vector_data_ptr(vector: &Vector) -> &[u8] {
    let end = data_end(vector).min(vector.data.len());
    &vector.data[..end]
}

pub fn vector_insert(vector_dst: &mut Vector, vector_src: &Vector, dst_index: i32) -> i32 {
    if vector_dst.esize != vector_src.esize {
        return -1;
    }

    let total = vector_src.count.max(0) as usize;
    if total == 0 {
        return 0;
    }

    shift_right(vector_dst, dst_index, total as i32);
    let dst_start = dst_index.max(0) as usize * vector_dst.esize;
    let src_bytes = &vector_src.data[..data_end(vector_src).min(vector_src.data.len())];
    let dst_end = dst_start.saturating_add(src_bytes.len()).min(vector_dst.data.len());
    let copy_len = dst_end.saturating_sub(dst_start);
    if copy_len > 0 {
        vector_dst.data[dst_start..dst_start + copy_len].copy_from_slice(&src_bytes[..copy_len]);
    }
    0
}

pub fn vector_pop_at_data_address(vector: &mut Vector, address: *const u8) -> i32 {
    let base = vector.data.as_ptr() as usize;
    let addr = address as usize;
    if addr < base || vector.esize == 0 {
        return -1;
    }

    let offset = addr - base;
    let index = (offset / vector.esize) as i32;
    vector_pop_at(vector, index);
    index
}

pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let old_pp = vector.pindex;
    vector_set_peek_pointer(vector, 0);
    let mut index = 0;
    let mut found = -1;

    loop {
        let matches = match vector_peek_ptr(vector) {
            Some(ptr) => ptr == val,
            None => false,
        };

        if matches {
            vector_pop_at(vector, index);
            found = index;
            break;
        }

        if vector.pindex >= vector.count {
            break;
        }
        index += 1;
    }

    vector_set_peek_pointer(vector, old_pp);
    found
}

pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 || index >= vector.count || vector.esize == 0 {
        return;
    }

    let dst_start = index as usize * vector.esize;
    let next_start = dst_start + vector.esize;
    let end = data_end(vector).min(vector.data.len());
    if next_start < end {
        vector.data.copy_within(next_start..end, dst_start);
    }
    let tail_start = end.saturating_sub(vector.esize);
    let data_len = vector.data.len();
    let fill_len = vector.esize.min(data_len.saturating_sub(tail_start));
    if tail_start < data_len && fill_len > 0 {
        vector.data[tail_start..tail_start + fill_len].fill(0);
    }
    vector.count -= 1;
    vector.rindex -= 1;
}

pub fn vector_peek_back(vector: &mut Vector) {
    vector.pindex -= 1;
}

pub fn vector_current_index(vector: &Vector) -> i32 {
    vector.rindex
}

pub fn vector_save(vector: &mut Vector) {
    vector.saves.push(VectorState {
        pindex: vector.pindex,
        rindex: vector.rindex,
        mindex: vector.mindex,
        count: vector.count,
        flags: vector.flags,
    });
}

pub fn vector_restore(vector: &mut Vector) {
    if let Some(save_vec) = vector.saves.pop() {
        vector.pindex = save_vec.pindex;
        vector.rindex = save_vec.rindex;
        vector.mindex = save_vec.mindex;
        vector.count = save_vec.count;
        vector.flags = save_vec.flags;
    }
}

pub fn vector_save_purge(vector: &mut Vector) {
    let _ = vector.saves.pop();
}

pub fn vector_element_size(vector: &Vector) -> usize {
    vector.esize
}

pub fn vector_clone(vector: &Vector) -> Vector {
    vector.clone()
}
