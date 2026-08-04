const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = 1 << 16 -1;

// Internal constants matching the C implementation.
const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
const BUFFER_MAX_SIZE: usize = LOW_CUTOFF;

pub struct RSet {
    buffer: Vec<u16>,
    size: usize,
}
impl RSet {
    pub fn new() -> Self {
        // Equivalent to rset_import(NULL, default_size) in C.
        Self::import(&[], DEFAULT_SIZE)
    }

    pub fn free(&mut self) {
        // In Rust the Vec drop will free the buffer. We mimic by clearing.
        self.buffer.clear();
        self.buffer.shrink_to_fit();
        self.size = 0;
    }

    pub fn cardinality(&self) -> usize {
        if self.is_full() {
            return MAX_CARDINALITY;
        }
        if self.is_empty_set() {
            return 0;
        }
        self.buffer[0] as usize
    }

    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }
        if self.is_empty_set() {
            self.buffer[0] = 0;
        }
        let cardinality = self.buffer[0] as usize;
        if cardinality == LOW_CUTOFF {
            if self.contains_array(item) {
                return true;
            }
            if !self.convert_array_to_bitset() {
                return false;
            }
        } else if cardinality == HIGH_CUTOFF {
            if self.contains_bitset(item) {
                return true;
            }
            if !self.convert_bitset_to_inverted_array() {
                return false;
            }
        }

        if cardinality < LOW_CUTOFF {
            if !self.add_array(item) {
                return false;
            }
        } else if cardinality >= HIGH_CUTOFF {
            if !self.add_inverted_array(item) {
                return false;
            }
        } else if !self.add_bitset(item) {
            return false;
        }
        true
    }

    pub fn contains(&self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }
        if self.is_empty_set() {
            return false;
        }
        if self.is_array() {
            return self.contains_array(item);
        }
        if self.is_inverted_array() {
            return !self.contains_array(item);
        }
        self.contains_bitset(item)
    }

    pub fn equals(&self, comparison: &RSet) -> bool {
        let card = self.cardinality();
        if card != comparison.cardinality() {
            return false;
        }
        let length = length_for(card);
        if length == 0 {
            return true;
        }
        let n = length / 2;
        for i in 0..n {
            if self.buffer[1 + i] != comparison.buffer[1 + i] {
                return false;
            }
        }
        true
    }

    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty_set() {
            return result.fill();
        }
        if self.is_full() {
            return result.truncate();
        }
        if !self.copy_to(result) {
            return false;
        }
        let new_card = MAX_CARDINALITY - result.buffer[0] as usize;
        result.buffer[0] = new_card as u16;
        if result.is_bitset() {
            for i in 0..LOW_CUTOFF {
                result.buffer[1 + i] = !result.buffer[1 + i];
            }
        }
        true
    }

    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty_set() || other.is_empty_set() {
            return result.truncate();
        }
        if self.is_full() {
            return other.copy_to(result);
        } else if other.is_full() {
            return self.copy_to(result);
        }
        if self.is_array() && other.is_array() {
            return self.intersection_array(other, result);
        }

        // Fall back to a bitset intersection (only correct if both operands are
        // bitsets, matching the C implementation which has the same TODO).
        if !result.grow_to(BUFFER_MAX_SIZE) {
            return false;
        }
        let cardinality = self.intersection_bitset(other, result);
        if cardinality == 0 {
            return result.truncate();
        }
        if cardinality == MAX_CARDINALITY {
            return result.fill();
        }
        result.buffer[0] = cardinality as u16;
        true
    }

    pub fn truncate(&mut self) -> bool {
        self.buffer[0] = 2;
        self.buffer[1] = MAX_ITEM;
        true
    }

    pub fn fill(&mut self) -> bool {
        self.buffer[0] = 0;
        true
    }

    pub fn export(&self) -> Vec<u8> {
        let len_bytes = self.length();
        let mut out = Vec::with_capacity(len_bytes);
        let n_u16 = len_bytes / 2;
        for i in 0..n_u16 {
            let bytes = self.buffer[i].to_le_bytes();
            out.push(bytes[0]);
            out.push(bytes[1]);
        }
        if len_bytes % 2 == 1 {
            // Should never happen since length is always even, but handle.
            out.push(0);
        }
        out
    }

    pub fn length(&self) -> usize {
        2 + length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length > 0 { length } else { 1 };
        if size > BUFFER_MAX_SIZE {
            size = BUFFER_MAX_SIZE;
        }
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            let buffer_capacity_bytes = (1 + size) * 2;
            let bytes_to_copy = length.min(buffer.len()).min(buffer_capacity_bytes);
            let full_pairs = bytes_to_copy / 2;
            for i in 0..full_pairs {
                buf[i] = u16::from_le_bytes([buffer[2 * i], buffer[2 * i + 1]]);
            }
            if bytes_to_copy % 2 == 1 {
                buf[full_pairs] = buffer[bytes_to_copy - 1] as u16;
            }
            RSet { buffer: buf, size }
        } else {
            buf[0] = 2;
            buf[1] = MAX_ITEM;
            RSet { buffer: buf, size }
        }
    }

    pub fn copy(&self) -> Self {
        let bytes = self.export();
        Self::import(&bytes, self.length())
    }
}

// Internal helpers.
impl RSet {
    fn is_full(&self) -> bool {
        self.buffer[0] == 0
    }

    fn is_empty_set(&self) -> bool {
        self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_array(&self) -> bool {
        (self.buffer[0] as usize) <= LOW_CUTOFF
    }

    fn is_bitset(&self) -> bool {
        let c = self.buffer[0] as usize;
        c > LOW_CUTOFF && c <= HIGH_CUTOFF
    }

    fn is_inverted_array(&self) -> bool {
        (self.buffer[0] as usize) > HIGH_CUTOFF
    }

    fn grow_to(&mut self, size: usize) -> bool {
        if self.size >= size {
            return true;
        }
        self.buffer.resize(1 + size, 0);
        self.size = size;
        true
    }

    fn grow(&mut self) -> bool {
        let mut new_size = self.size * GROWTH_FACTOR;
        if new_size > BUFFER_MAX_SIZE {
            new_size = BUFFER_MAX_SIZE;
        }
        self.grow_to(new_size)
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let n_u16 = self.length() / 2;
        for i in 0..n_u16 {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn contains_array(&self, item: u16) -> bool {
        let mut cardinality = self.buffer[0] as usize;
        if cardinality > HIGH_CUTOFF {
            cardinality = MAX_CARDINALITY - cardinality;
        }
        if cardinality == 0 {
            return false;
        }
        let mut first: i64 = 0;
        let mut last: i64 = cardinality as i64 - 1;
        while first <= last {
            let middle = ((first + last) / 2) as usize;
            let v = self.buffer[1 + middle];
            if v == item {
                return true;
            }
            if v < item {
                first = middle as i64 + 1;
            } else {
                last = middle as i64 - 1;
            }
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        (self.buffer[offset] & bit) != 0
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let mut i: usize;
        if cardinality > 0 && self.buffer[cardinality] < item {
            i = cardinality + 1;
        } else {
            i = 1;
            while i <= cardinality {
                if self.buffer[i] < item {
                    i += 1;
                    continue;
                }
                if self.buffer[i] == item {
                    return true;
                }
                break;
            }
        }
        if cardinality == self.size && !self.grow() {
            return false;
        }
        // Shift any existing elements at positions [i, cardinality] right by one.
        if cardinality + 1 > i {
            for j in (i + 1..=cardinality + 1).rev() {
                self.buffer[j] = self.buffer[j - 1];
            }
        }
        self.buffer[i] = item;
        self.buffer[0] = self.buffer[0].wrapping_add(1);
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        if (self.buffer[offset] & bit) == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let cardinality_inv = MAX_CARDINALITY - self.buffer[0] as usize;
        // The C code reads buffer[cardinality_inv]; this corresponds to the
        // last element of the inverted array (array[cardinality_inv - 1]).
        if cardinality_inv > 0 && self.buffer[cardinality_inv] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        for i in 0..cardinality_inv {
            let v = self.buffer[1 + i];
            if v < item {
                continue;
            }
            if v > item {
                break;
            }
            // v == item: shift elements [i+1, cardinality_inv-1] left into [i, cardinality_inv-2].
            if cardinality_inv >= 2 {
                for j in i..(cardinality_inv - 1) {
                    self.buffer[1 + j] = self.buffer[1 + j + 1];
                }
            }
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        true
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        // Cardinality is LOW_CUTOFF; size has grown to LOW_CUTOFF.
        if !self.grow_to(BUFFER_MAX_SIZE) {
            return false;
        }
        let mut bitset = vec![0u16; LOW_CUTOFF];
        for i in 0..LOW_CUTOFF {
            let item = self.buffer[1 + i];
            bitset[(item as usize) >> 4] |= 1u16 << (item & 0xF);
        }
        for i in 0..LOW_CUTOFF {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        // Bitset has HIGH_CUTOFF bits set, so LOW_CUTOFF bits unset.
        let mut array: Vec<u16> = Vec::with_capacity(LOW_CUTOFF);
        let mut bit: u32 = 0;
        for i in 0..LOW_CUTOFF {
            for j in 0..16u32 {
                if (self.buffer[1 + i] & (1u16 << j)) == 0 {
                    array.push(bit as u16);
                }
                bit = bit.wrapping_add(1);
            }
        }
        for (k, &v) in array.iter().enumerate() {
            self.buffer[1 + k] = v;
        }
        true
    }

    fn intersection_bitset(&self, other: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..LOW_CUTOFF {
            let v = self.buffer[1 + i] & other.buffer[1 + i];
            result.buffer[1 + i] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
    }

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let result_size = (self.buffer[0] as usize).max(other.buffer[0] as usize);
        if !result.grow_to(result_size) {
            return false;
        }
        let a_size = self.buffer[0] as usize;
        let b_size = other.buffer[0] as usize;
        let mut i = 0usize;
        let mut j = 0usize;
        let mut k = 0usize;
        while i < a_size && j < b_size {
            let av = self.buffer[1 + i];
            let bv = other.buffer[1 + j];
            if av < bv {
                i += 1;
            } else if bv < av {
                j += 1;
            } else {
                result.buffer[1 + k] = av;
                k += 1;
                i += 1;
                j += 1;
            }
        }
        result.buffer[0] = k as u16;
        if k == 0 {
            result.truncate();
        }
        true
    }
}

fn length_for(cardinality: usize) -> usize {
    let mut c = cardinality;
    if c == 0 {
        c = 1;
    } else if c >= HIGH_CUTOFF {
        c = MAX_CARDINALITY - c;
    } else if c > LOW_CUTOFF {
        c = LOW_CUTOFF;
    }
    2 * c
}

#[allow(dead_code)]
fn _unused_max_size() -> u16 {
    MAX_SIZE
}
