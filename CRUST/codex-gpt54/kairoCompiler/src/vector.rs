use std::fs::File;
use std::io::Read;

pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;

#[derive(Debug, Clone)]
struct VectorSave {
    pindex: i32,
    rindex: i32,
    mindex: i32,
    count: i32,
    flags: i32,
}

#[derive(Debug, Clone)]
pub struct Vector {
    data: Vec<u8>,
    pindex: i32,
    rindex: i32,
    mindex: i32,
    count: i32,
    flags: i32,
    esize: usize,
    saves: Vec<VectorSave>,
}

impl Default for Vector {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            pindex: 0,
            rindex: 0,
            mindex: 0,
            count: 0,
            flags: 0,
            esize: 1,
            saves: Vec::new(),
        }
    }
}

impl Vector {
    fn element_range(&self, index: i32) -> Option<std::ops::Range<usize>> {
        if index < 0 {
            return None;
        }

        let start = index as usize * self.esize;
        let end = start.checked_add(self.esize)?;
        if end <= self.data.len() {
            Some(start..end)
        } else {
            None
        }
    }

    fn in_bounds_for_at(&self, index: i32) -> bool {
        index >= 0 && index < self.rindex
    }

    fn in_bounds_for_pop(&self, index: i32) -> bool {
        index >= 0 && index < self.mindex
    }

    fn ensure_capacity_for_index(&mut self, start_index: i32, total_elements: i32) {
        if start_index + total_elements < self.mindex {
            return;
        }

        let new_mindex = (start_index + total_elements + VECTOR_ELEMENT_INCREMENT as i32).max(0);
        self.data.resize(new_mindex as usize * self.esize, 0);
        self.mindex = start_index + total_elements;
    }

    fn data_end(&self) -> usize {
        self.rindex.max(0) as usize * self.esize
    }

    fn write_element(&mut self, index: i32, elem: &[u8]) {
        self.ensure_capacity_for_index(index, 0);
        if let Some(range) = self.element_range(index) {
            self.data[range.clone()].fill(0);
            let copy_len = elem.len().min(self.esize);
            self.data[range.start..range.start + copy_len].copy_from_slice(&elem[..copy_len]);
        }
    }

    fn read_bytes(&self, index: i32) -> Option<&[u8]> {
        let range = self.element_range(index)?;
        Some(&self.data[range])
    }

    fn resize_for(&mut self, total_elements: i32) {
        self.ensure_capacity_for_index(self.rindex, total_elements);
    }

    fn shift_right_in_bounds_no_increment(&mut self, index: i32, amount: i32) {
        self.ensure_capacity_for_index(index, amount);
        let eindex = index + amount;
        let src = index.max(0) as usize * self.esize;
        let dst = eindex.max(0) as usize * self.esize;
        let end = self.data_end();
        if src < end {
            self.data.copy_within(src..end, dst);
        }
        let zero_end = dst.min(self.data.len());
        if src < zero_end {
            self.data[src..zero_end].fill(0);
        }
    }

    fn shift_right_in_bounds(&mut self, index: i32, amount: i32) {
        self.shift_right_in_bounds_no_increment(index, amount);
        self.rindex += amount;
        self.count += amount;
    }

    fn stretch(&mut self, index: i32) {
        if index < self.rindex {
            return;
        }

        self.ensure_capacity_for_index(index, 0);
        self.count = index;
        self.rindex = index;
    }

    fn shift_right(&mut self, index: i32, amount: i32) {
        if index < self.rindex {
            self.shift_right_in_bounds(index, amount);
            return;
        }

        self.stretch(index + amount);
        self.shift_right_in_bounds_no_increment(index, amount);
    }
}

pub fn vector_create(esize: usize) -> Vector {
    let esize = esize.max(1);
    Vector {
        data: vec![0; esize * VECTOR_ELEMENT_INCREMENT],
        pindex: 0,
        rindex: 0,
        mindex: VECTOR_ELEMENT_INCREMENT as i32,
        count: 0,
        flags: 0,
        esize,
        saves: Vec::new(),
    }
}

pub fn vector_free(_vector: Vector) {}

pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    let range = vector.element_range(index)?;
    Some(&mut vector.data[range])
}

pub fn vector_peek_ptr_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 || index > vector.count {
        return None;
    }

    vector_at(vector, index)
}

pub fn vector_peek_no_increment(vector: &mut Vector) -> Option<&mut [u8]> {
    if !vector.in_bounds_for_at(vector.pindex) {
        return None;
    }

    vector_at(vector, vector.pindex)
}

pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    let current = vector.pindex;
    if !vector.in_bounds_for_at(current) {
        return None;
    }
    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    vector_at(vector, current)
}

pub fn vector_peek_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if !vector.in_bounds_for_at(index) {
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
    vector.pindex = vector.rindex - 1;
}

pub fn vector_push(vector: &mut Vector, elem: &[u8]) {
    vector.write_element(vector.rindex, elem);
    vector.rindex += 1;
    vector.count += 1;
    if vector.rindex >= vector.mindex {
        vector.resize_for(0);
    }
}

pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    vector.shift_right(index, 1);
    vector.write_element(index, ptr);
}

pub fn vector_pop(vector: &mut Vector) {
    if vector.count <= 0 {
        vector.rindex = 0;
        vector.count = 0;
        return;
    }
    vector.rindex -= 1;
    vector.count -= 1;
    if !vector.in_bounds_for_pop(vector.rindex) {
        vector.rindex = 0;
        vector.count = 0;
    }
}

pub fn vector_peek_pop(vector: &mut Vector) {
    vector_pop_at(vector, vector.pindex);
}

pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    let index = vector.rindex - 1;
    if !vector.in_bounds_for_pop(index) {
        return None;
    }
    vector_at(vector, index)
}

pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    let index = vector.rindex - 1;
    if !vector.in_bounds_for_at(index) {
        return None;
    }
    vector_at(vector, index)
}

pub fn vector_back_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}

pub fn vector_back_ptr_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back_or_null(vector)
}

pub fn vector_string(vec: &Vector) -> Option<&str> {
    let end = vec
        .data
        .iter()
        .position(|b| *b == 0)
        .unwrap_or_else(|| vec.count.max(0) as usize * vec.esize);
    std::str::from_utf8(&vec.data[..end.min(vec.data.len())]).ok()
}

pub fn vector_empty(vector: &Vector) -> bool {
    vector.count == 0
}

pub fn vector_clear(vector: &mut Vector) {
    while vector.count > 0 {
        vector_pop(vector);
    }
}

pub fn vector_count(vector: &Vector) -> i32 {
    vector.count
}

pub fn vector_fread(vector: &mut Vector, amount: i32, mut fp: File) -> i32 {
    let mut bytes = Vec::new();
    if fp.read_to_end(&mut bytes).is_err() {
        return -1;
    }

    let limit = if amount <= 0 {
        bytes.len()
    } else {
        amount as usize
    };

    for byte in bytes.into_iter().take(limit) {
        vector_push(vector, &[byte]);
    }

    0
}

pub fn vector_data_ptr(vector: &Vector) -> &[u8] {
    &vector.data[..vector.data_end().min(vector.data.len())]
}

pub fn vector_insert(vector_dst: &mut Vector, vector_src: &Vector, dst_index: i32) -> i32 {
    if vector_dst.esize != vector_src.esize {
        return -1;
    }

    let total = vector_src.count.max(0) as usize;
    vector_dst.shift_right(dst_index, total as i32);
    for i in 0..total {
        if let Some(bytes) = vector_src.read_bytes(i as i32) {
            vector_dst.write_element(dst_index + i as i32, bytes);
        }
    }

    0
}

pub fn vector_pop_at_data_address(vector: &mut Vector, address: *const u8) -> i32 {
    let base = vector.data.as_ptr() as usize;
    let ptr = address as usize;
    if ptr < base || vector.esize == 0 {
        return -1;
    }

    let offset = ptr - base;
    let index = (offset / vector.esize) as i32;
    vector_pop_at(vector, index);
    index
}

pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    let old_pp = vector.pindex;
    vector.pindex = 0;
    let mut index = 0;

    while index < vector.count {
        let found = vector
            .read_bytes(index)
            .map(|bytes| bytes.starts_with(val))
            .unwrap_or(false);
        if found {
            vector_pop_at(vector, index);
            vector.pindex = old_pp;
            return index;
        }
        index += 1;
    }

    vector.pindex = old_pp;
    -1
}

pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 || index >= vector.count {
        return;
    }

    let dst_start = index as usize * vector.esize;
    let src_start = dst_start + vector.esize;
    let end = vector.data_end();
    if src_start < end {
        vector.data.copy_within(src_start..end, dst_start);
    }
    if end >= vector.esize {
        vector.data[end - vector.esize..end].fill(0);
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
    vector.saves.push(VectorSave {
        pindex: vector.pindex,
        rindex: vector.rindex,
        mindex: vector.mindex,
        count: vector.count,
        flags: vector.flags,
    });
}

pub fn vector_restore(vector: &mut Vector) {
    if let Some(save) = vector.saves.pop() {
        vector.pindex = save.pindex;
        vector.rindex = save.rindex;
        vector.mindex = save.mindex;
        vector.count = save.count;
        vector.flags = save.flags;
    }
}

pub fn vector_save_purge(vector: &mut Vector) {
    let _ = vector.saves.pop();
}

pub fn vector_element_size(vector: &Vector) -> usize {
    vector.esize
}

pub fn vector_clone(vector: &Vector) -> Vector {
    let mut cloned = vector.clone();
    cloned.saves.clear();
    cloned
}
