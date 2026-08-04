const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = 1 << 16 -1;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
const SIZE_CAP: usize = LOW_CUTOFF;

pub struct RSet {
    buffer: Vec<u16>,
    size: usize,
}
impl RSet {
    pub fn new() -> Self {
        Self::import(&[], DEFAULT_SIZE)
    }
    pub fn free(&mut self) {
        self.buffer.clear();
        self.size = 0;
    }
    pub fn cardinality(&self) -> usize {
        if self.is_full() {
            return MAX_CARDINALITY;
        }
        if self.is_empty() {
            return 0;
        }
        self.buffer[0] as usize
    }
    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }
        if self.is_empty() {
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
            self.add_array(item)
        } else if cardinality >= HIGH_CUTOFF {
            self.add_inverted_array(item)
        } else {
            self.add_bitset(item)
        }
    }
    pub fn contains(&self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }
        if self.is_empty() {
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
        let cardinality = self.cardinality();
        if cardinality != comparison.cardinality() {
            return false;
        }
        let length_bytes = Self::length_for(cardinality);
        if length_bytes == 0 {
            return true;
        }
        let n_words = length_bytes / 2;
        for i in 0..n_words {
            if self.buffer[1 + i] != comparison.buffer[1 + i] {
                return false;
            }
        }
        true
    }
    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty() {
            return result.fill();
        }
        if self.is_full() {
            return result.truncate();
        }
        if !self.copy_to(result) {
            return false;
        }
        let original = result.buffer[0] as usize;
        let new_card = MAX_CARDINALITY - original;
        // Note: if new_card == MAX_CARDINALITY (i.e., original was 0), we'd
        // have already returned via is_full above.
        result.buffer[0] = new_card as u16;
        if result.is_bitset() {
            result.invert_bitset();
        }
        true
    }
    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty() || other.is_empty() {
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

        if !result.grow_to(SIZE_CAP) {
            return false;
        }
        let cardinality = Self::intersection_bitset(self, other, result);

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
        let mut bytes = Vec::with_capacity(len_bytes);
        let n_words = len_bytes / 2;
        for i in 0..n_words {
            let w = self.buffer[i];
            bytes.push((w & 0xFF) as u8);
            bytes.push((w >> 8) as u8);
        }
        bytes
    }
    pub fn length(&self) -> usize {
        2 + Self::length_for(self.cardinality())
    }
    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length == 0 { 1 } else { length };
        if size > SIZE_CAP {
            size = SIZE_CAP;
        }
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            // memcpy length bytes
            let n_words = length / 2;
            let max_words = buf.len();
            let limit = n_words.min(max_words);
            for i in 0..limit {
                if 2 * i + 1 < buffer.len() {
                    buf[i] = u16::from_le_bytes([buffer[2 * i], buffer[2 * i + 1]]);
                } else if 2 * i < buffer.len() {
                    buf[i] = buffer[2 * i] as u16;
                }
            }
        } else {
            // truncate
            buf[0] = 2;
            buf[1] = MAX_ITEM;
        }
        RSet { buffer: buf, size }
    }
    pub fn copy(&self) -> Self {
        Self::import(&self.export(), self.length())
    }

    // ======== private helpers ========

    fn is_empty(&self) -> bool {
        self.buffer[0] == 2 && self.buffer.len() >= 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full(&self) -> bool {
        self.buffer[0] == 0
    }

    fn is_bitset(&self) -> bool {
        let c = self.buffer[0] as usize;
        c > LOW_CUTOFF && c <= HIGH_CUTOFF
    }

    fn is_array(&self) -> bool {
        let c = self.buffer[0] as usize;
        c <= LOW_CUTOFF
    }

    fn is_inverted_array(&self) -> bool {
        let c = self.buffer[0] as usize;
        c > HIGH_CUTOFF
    }

    fn length_for(cardinality: usize) -> usize {
        let c = if cardinality == 0 {
            1
        } else if cardinality >= HIGH_CUTOFF {
            MAX_CARDINALITY - cardinality
        } else if cardinality > LOW_CUTOFF {
            LOW_CUTOFF
        } else {
            cardinality
        };
        2 * c
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
        if new_size > SIZE_CAP {
            new_size = SIZE_CAP;
        }
        self.grow_to(new_size)
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len_bytes = self.length();
        let n_words = len_bytes / 2;
        for i in 0..n_words {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        // Need buffer of 1 + LOW_CUTOFF u16s
        if !self.grow_to(SIZE_CAP) {
            return false;
        }
        // Snapshot the array (it occupies buffer[1..1+LOW_CUTOFF])
        let mut bitset = vec![0u16; SIZE_CAP];
        for i in 0..SIZE_CAP {
            let v = self.buffer[1 + i] as usize;
            bitset[v >> 4] |= 1u16 << ((v & 0xF) as u32);
        }
        for i in 0..SIZE_CAP {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array: Vec<u16> = Vec::with_capacity(SIZE_CAP);
        let mut bit: u32 = 0;
        for i in 0..SIZE_CAP {
            let word = self.buffer[1 + i];
            for j in 0..16u32 {
                if (word & (1u16 << j)) == 0 {
                    array.push(bit as u16);
                }
                bit += 1;
            }
        }
        // Copy array into buffer[1..]
        for i in 0..array.len() {
            self.buffer[1 + i] = array[i];
        }
        true
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let mut i: usize;
        if cardinality > 0 && self.buffer[cardinality] < item {
            i = cardinality + 1;
        } else {
            i = 1;
            while i <= cardinality {
                let v = self.buffer[i];
                if v < item {
                    i += 1;
                    continue;
                }
                if v == item {
                    return true;
                }
                break;
            }
        }
        if cardinality == self.size && !self.grow() {
            return false;
        }
        if cardinality + 1 > i {
            // shift right: buffer[i..cardinality+1] -> buffer[i+1..cardinality+2]
            self.buffer.copy_within(i..cardinality + 1, i + 1);
        }
        self.buffer[i] = item;
        self.buffer[0] = self.buffer[0].wrapping_add(1);
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << ((item & 0xF) as u32);
        if (self.buffer[offset] & bit) == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let inv_len = MAX_CARDINALITY - self.buffer[0] as usize;
        if inv_len > 0 && self.buffer[inv_len] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        // search and remove from inverted array
        for i in 0..inv_len {
            let val = self.buffer[i + 1];
            if val < item {
                continue;
            }
            if val > item {
                break;
            }
            // Logical shift left: remove element at position i.
            // buffer[i+2..inv_len+1] -> buffer[i+1..inv_len]
            // Number of elements: (inv_len - 1 - i)
            if inv_len > i + 1 {
                self.buffer.copy_within(i + 2..inv_len + 1, i + 1);
            }
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
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
            let val = self.buffer[1 + middle];
            if val == item {
                return true;
            }
            if val < item {
                first = middle as i64 + 1;
            } else {
                last = middle as i64 - 1;
            }
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << ((item & 0xF) as u32);
        (self.buffer[offset] & bit) != 0
    }

    fn invert_bitset(&mut self) {
        for i in 0..SIZE_CAP {
            self.buffer[1 + i] = !self.buffer[1 + i];
        }
    }

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let a_card = self.buffer[0] as usize;
        let b_card = other.buffer[0] as usize;
        let result_size = a_card.max(b_card);
        if !result.grow_to(result_size) {
            return false;
        }

        let mut i = 0;
        let mut j = 0;
        let mut k = 0;
        while i < a_card && j < b_card {
            let a_val = self.buffer[1 + i];
            let b_val = other.buffer[1 + j];
            if a_val < b_val {
                i += 1;
            } else if a_val > b_val {
                j += 1;
            } else {
                result.buffer[1 + k] = a_val;
                i += 1;
                j += 1;
                k += 1;
            }
        }

        if k == 0 {
            result.truncate();
        } else {
            result.buffer[0] = k as u16;
        }
        true
    }

    fn intersection_bitset(a: &RSet, b: &RSet, result: &mut RSet) -> usize {
        let mut cardinality: usize = 0;
        for i in 0..SIZE_CAP {
            let merged = a.buffer[1 + i] & b.buffer[1 + i];
            result.buffer[1 + i] = merged;
            cardinality += merged.count_ones() as usize;
        }
        cardinality
    }
}

#[allow(dead_code)]
const _UNUSED_MAX_SIZE: u16 = MAX_SIZE;
