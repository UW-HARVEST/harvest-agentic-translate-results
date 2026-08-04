const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE: usize = LOW_CUTOFF;
const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;

pub struct RSet {
    buffer: Vec<u16>,
    size: usize,
}

impl RSet {
    pub fn new() -> Self {
        Self::import(&[], DEFAULT_SIZE)
    }

    pub fn free(&mut self) {
        self.buffer = Vec::new();
        self.size = 0;
    }

    pub fn cardinality(&self) -> usize {
        if self.is_full() {
            return MAX_CARDINALITY;
        }
        if self.is_empty_state() {
            return 0;
        }
        self.buffer[0] as usize
    }

    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }
        if self.is_empty_state() {
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
        if self.is_empty_state() {
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
        let length = Self::length_for(cardinality);
        if length == 0 {
            return true;
        }
        let n_u16 = length / 2;
        if 1 + n_u16 > self.buffer.len() || 1 + n_u16 > comparison.buffer.len() {
            return false;
        }
        self.buffer[1..1 + n_u16] == comparison.buffer[1..1 + n_u16]
    }

    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty_state() {
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
            result.invert_bitset();
        }
        true
    }

    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty_state() || other.is_empty_state() {
            return result.truncate();
        }
        if self.is_full() {
            return other.copy_to(result);
        }
        if other.is_full() {
            return self.copy_to(result);
        }
        if self.is_array() && other.is_array() {
            return self.intersection_array(other, result);
        }

        if !result.grow_to(MAX_SIZE) {
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
        let len = self.length();
        let bytes_ptr = self.buffer.as_ptr() as *const u8;
        unsafe { std::slice::from_raw_parts(bytes_ptr, len).to_vec() }
    }

    pub fn length(&self) -> usize {
        2 + Self::length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length > 0 { length } else { 1 };
        if size > MAX_SIZE {
            size = MAX_SIZE;
        }
        let vec: Vec<u16> = vec![0u16; 1 + size];
        let mut set = RSet { buffer: vec, size };
        if !buffer.is_empty() && length > 0 {
            let n = length.min(buffer.len()).min(set.buffer.len() * 2);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    buffer.as_ptr(),
                    set.buffer.as_mut_ptr() as *mut u8,
                    n,
                );
            }
        } else {
            set.truncate();
        }
        set
    }

    pub fn copy(&self) -> Self {
        let exported = self.export();
        Self::import(&exported, self.length())
    }
}

// Private helpers
impl RSet {
    fn is_empty_state(&self) -> bool {
        self.buffer[0] == 2 && self.buffer.len() > 1 && self.buffer[1] == MAX_ITEM
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
        let n = if cardinality == 0 {
            1
        } else if cardinality >= HIGH_CUTOFF {
            MAX_CARDINALITY - cardinality
        } else if cardinality > LOW_CUTOFF {
            LOW_CUTOFF
        } else {
            cardinality
        };
        2 * n
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
        let mut size = self.size * GROWTH_FACTOR;
        if size > MAX_SIZE {
            size = MAX_SIZE;
        }
        self.grow_to(size)
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        if !self.grow_to(MAX_SIZE) {
            return false;
        }
        let mut bitset = vec![0u16; MAX_SIZE];
        for i in 0..MAX_SIZE {
            let v = self.buffer[i + 1] as usize;
            bitset[v >> 4] |= 1u16 << (v & 0xF);
        }
        for i in 0..MAX_SIZE {
            self.buffer[i + 1] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array = vec![0u16; MAX_SIZE];
        let mut ptr = 0usize;
        let mut bit: u32 = 0;
        for i in 0..MAX_SIZE {
            let val = self.buffer[i + 1];
            for j in 0..16u32 {
                if val & (1u16 << j) == 0 {
                    if ptr < MAX_SIZE {
                        array[ptr] = bit as u16;
                        ptr += 1;
                    }
                }
                bit += 1;
            }
        }
        for i in 0..MAX_SIZE {
            self.buffer[i + 1] = array[i];
        }
        true
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let i: usize;
        if cardinality > 0 && self.buffer[cardinality] < item {
            i = cardinality + 1;
        } else {
            let mut idx = 1usize;
            while idx <= cardinality {
                if self.buffer[idx] < item {
                    idx += 1;
                    continue;
                }
                if self.buffer[idx] == item {
                    return true;
                }
                break;
            }
            i = idx;
        }
        if cardinality == self.size && !self.grow() {
            return false;
        }
        if cardinality + 1 > i {
            let count = cardinality + 1 - i;
            for k in (0..count).rev() {
                self.buffer[i + 1 + k] = self.buffer[i + k];
            }
        }
        self.buffer[i] = item;
        self.buffer[0] = self.buffer[0].wrapping_add(1);
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        if self.buffer[offset] & bit == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let cardinality = MAX_CARDINALITY - self.buffer[0] as usize;
        if cardinality > 0 && cardinality < self.buffer.len() && self.buffer[cardinality] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        for i in 0..cardinality {
            let val = self.buffer[i + 1];
            if val < item {
                continue;
            }
            if val > item {
                break;
            }
            // val == item: shift remaining inverted-array elements down
            let count = cardinality - i - 1;
            for k in 0..count {
                self.buffer[i + 1 + k] = self.buffer[i + 2 + k];
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
        let mut first: isize = 0;
        let mut last: isize = cardinality as isize - 1;
        while first <= last {
            let middle = ((first + last) / 2) as usize;
            let val = self.buffer[middle + 1];
            if val == item {
                return true;
            }
            if val < item {
                first = middle as isize + 1;
            } else {
                last = middle as isize - 1;
            }
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        self.buffer[offset] & bit != 0
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len = self.length();
        let n_u16 = len / 2;
        dest.buffer[..n_u16].copy_from_slice(&self.buffer[..n_u16]);
        true
    }

    fn invert_bitset(&mut self) {
        for i in 0..MAX_SIZE {
            self.buffer[i + 1] = !self.buffer[i + 1];
        }
    }

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let result_size = (self.buffer[0] as usize).max(other.buffer[0] as usize);
        if !result.grow_to(result_size) {
            return false;
        }
        let a_size = self.buffer[0] as usize;
        let b_size = other.buffer[0] as usize;
        let mut a_idx = 0usize;
        let mut b_idx = 0usize;
        let mut r_idx = 0usize;
        while a_idx < a_size && b_idx < b_size {
            let va = self.buffer[a_idx + 1];
            let vb = other.buffer[b_idx + 1];
            if va < vb {
                a_idx += 1;
            } else if vb < va {
                b_idx += 1;
            } else {
                result.buffer[r_idx + 1] = va;
                r_idx += 1;
                a_idx += 1;
                b_idx += 1;
            }
        }
        result.buffer[0] = r_idx as u16;
        if r_idx == 0 {
            result.truncate();
        }
        true
    }

    fn intersection_bitset(&self, other: &RSet, result: &mut RSet) -> usize {
        let mut cardinality: usize = 0;
        for i in 0..MAX_SIZE {
            let va = self.buffer.get(i + 1).copied().unwrap_or(0);
            let vb = other.buffer.get(i + 1).copied().unwrap_or(0);
            let val = va & vb;
            result.buffer[i + 1] = val;
            cardinality += val.count_ones() as usize;
        }
        cardinality
    }
}
