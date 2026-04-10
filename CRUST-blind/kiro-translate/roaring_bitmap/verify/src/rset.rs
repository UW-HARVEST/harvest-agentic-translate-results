const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = 1 << 12;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;

pub struct RSet {
    buffer: Vec<u16>,
    size: usize,
}

fn is_empty(buffer: &[u16]) -> bool {
    buffer[0] == 2 && buffer[1] == MAX_ITEM
}

fn is_full(buffer: &[u16]) -> bool {
    buffer[0] == 0
}

fn is_bitset(buffer: &[u16]) -> bool {
    let c = buffer[0] as usize;
    c > LOW_CUTOFF && c <= HIGH_CUTOFF
}

fn is_array(buffer: &[u16]) -> bool {
    let c = buffer[0] as usize;
    c <= LOW_CUTOFF
}

fn is_inverted_array(buffer: &[u16]) -> bool {
    let c = buffer[0] as usize;
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
    std::mem::size_of::<u16>() * c
}

fn contains_array(buffer: &[u16], item: u16) -> bool {
    let mut cardinality = buffer[0] as usize;
    if cardinality > HIGH_CUTOFF {
        cardinality = MAX_CARDINALITY - cardinality;
    }
    let array = &buffer[1..1 + cardinality];
    array.binary_search(&item).is_ok()
}

fn contains_bitset(buffer: &[u16], item: u16) -> bool {
    let offset = (item >> 4) as usize + 1;
    let bit = 1u16 << (item & 0xF);
    buffer[offset] & bit != 0
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
        if is_full(&self.buffer) {
            return MAX_CARDINALITY;
        }
        if is_empty(&self.buffer) {
            return 0;
        }
        self.buffer[0] as usize
    }

    pub fn add(&mut self, item: u16) -> bool {
        if is_full(&self.buffer) {
            return true;
        }
        if is_empty(&self.buffer) {
            self.buffer[0] = 0;
        }

        let cardinality = self.buffer[0] as usize;

        if cardinality == LOW_CUTOFF {
            if contains_array(&self.buffer, item) {
                return true;
            }
            if !self.convert_array_to_bitset() {
                return false;
            }
        } else if cardinality == HIGH_CUTOFF {
            if contains_bitset(&self.buffer, item) {
                return true;
            }
            if !self.convert_bitset_to_inverted_array() {
                return false;
            }
        }

        let cardinality = self.buffer[0] as usize;
        if cardinality < LOW_CUTOFF {
            self.add_array(item)
        } else if cardinality >= HIGH_CUTOFF {
            self.add_inverted_array(item)
        } else {
            self.add_bitset(item)
        }
    }

    pub fn contains(&self, item: u16) -> bool {
        if is_full(&self.buffer) {
            return true;
        }
        if is_empty(&self.buffer) {
            return false;
        }
        if is_array(&self.buffer) {
            return contains_array(&self.buffer, item);
        }
        if is_inverted_array(&self.buffer) {
            return !contains_array(&self.buffer, item);
        }
        contains_bitset(&self.buffer, item)
    }

    pub fn equals(&self, comparison: &RSet) -> bool {
        let cardinality = self.cardinality();
        if cardinality != comparison.cardinality() {
            return false;
        }
        let length = length_for(cardinality);
        if length == 0 {
            return true;
        }
        let bytes = length / std::mem::size_of::<u16>();
        self.buffer[1..1 + bytes] == comparison.buffer[1..1 + bytes]
    }

    pub fn invert(&self, result: &mut RSet) -> bool {
        if is_empty(&self.buffer) {
            return result.fill();
        }
        if is_full(&self.buffer) {
            return result.truncate();
        }
        if !self.copy_to(result) {
            return false;
        }
        let old_card = result.buffer[0] as usize;
        result.buffer[0] = (MAX_CARDINALITY - old_card) as u16;
        if is_bitset(&result.buffer) {
            Self::invert_bitset(&mut result.buffer);
        }
        true
    }

    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if is_empty(&self.buffer) || is_empty(&other.buffer) {
            return result.truncate();
        }
        if is_full(&self.buffer) {
            return other.copy_to(result);
        }
        if is_full(&other.buffer) {
            return self.copy_to(result);
        }
        if is_array(&self.buffer) && is_array(&other.buffer) {
            return Self::intersection_array(self, other, result);
        }

        result.grow_to(MAX_SIZE as usize);
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
        let u16_count = len_bytes / 2;
        let mut bytes = Vec::with_capacity(len_bytes);
        for i in 0..u16_count {
            bytes.extend_from_slice(&self.buffer[i].to_ne_bytes());
        }
        bytes
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let size = if length == 0 { 1 } else { length.min(MAX_SIZE as usize) };
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            let copy_bytes = buffer.len().min((1 + size) * 2);
            let u16_count = copy_bytes / 2;
            for i in 0..u16_count {
                buf[i] = u16::from_ne_bytes([buffer[i * 2], buffer[i * 2 + 1]]);
            }
        } else {
            buf[0] = 2;
            buf[1] = MAX_ITEM;
        }
        RSet { buffer: buf, size }
    }

    pub fn copy(&self) -> Self {
        let exported = self.export();
        Self::import(&exported, self.length())
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
        let size = (self.size * GROWTH_FACTOR).min(MAX_SIZE as usize);
        self.grow_to(size)
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        let card = self.buffer[0] as usize;
        let mut bitset = vec![0u16; MAX_SIZE as usize];
        for i in 0..card {
            let val = self.buffer[1 + i];
            bitset[(val >> 4) as usize] |= 1 << (val & 0xF);
        }
        self.grow_to(MAX_SIZE as usize);
        for i in 0..MAX_SIZE as usize {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array = Vec::new();
        for i in 0..MAX_SIZE as usize {
            let word = self.buffer[1 + i];
            for j in 0..16u16 {
                let bit = (i as u16) * 16 + j;
                if word & (1 << j) == 0 {
                    array.push(bit);
                }
            }
        }
        for i in 0..array.len().min(MAX_SIZE as usize) {
            self.buffer[1 + i] = array[i];
        }
        true
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let i;
        if cardinality > 0 && self.buffer[cardinality] < item {
            i = cardinality + 1;
        } else {
            let mut found = cardinality + 1;
            for idx in 1..=cardinality {
                if self.buffer[idx] < item {
                    continue;
                }
                if self.buffer[idx] == item {
                    return true;
                }
                found = idx;
                break;
            }
            i = found;
        }
        if cardinality == self.size && !self.grow() {
            return false;
        }
        if cardinality + 1 > i {
            let count = cardinality + 1 - i;
            if self.buffer.len() <= cardinality + 1 {
                self.buffer.push(0);
            }
            for k in (0..count).rev() {
                self.buffer[i + 1 + k] = self.buffer[i + k];
            }
        } else if self.buffer.len() <= cardinality + 1 {
            self.buffer.push(0);
        }
        self.buffer[i] = item;
        self.buffer[0] += 1;
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item >> 4) as usize + 1;
        let bit = 1u16 << (item & 0xF);
        if self.buffer[offset] & bit == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let cardinality = MAX_CARDINALITY - self.buffer[0] as usize;
        if cardinality > 0 && self.buffer[cardinality] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        for i in 0..cardinality {
            let val = self.buffer[1 + i];
            if val < item {
                continue;
            }
            if val > item {
                break;
            }
            for j in i..cardinality - 1 {
                self.buffer[1 + j] = self.buffer[1 + j + 1];
            }
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        true
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        dest.grow_to(self.size);
        let len_u16 = self.length() / 2;
        for i in 0..len_u16 {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn invert_bitset(buffer: &mut Vec<u16>) {
        for i in 0..MAX_SIZE as usize {
            buffer[1 + i] = !buffer[1 + i];
        }
    }

    fn intersection_array(a: &RSet, b: &RSet, result: &mut RSet) -> bool {
        let a_card = a.buffer[0] as usize;
        let b_card = b.buffer[0] as usize;
        let result_size = a_card.max(b_card);
        result.grow_to(result_size);
        if result.buffer.len() < 1 + result_size {
            result.buffer.resize(1 + result_size, 0);
        }

        let a_arr = &a.buffer[1..1 + a_card];
        let b_arr = &b.buffer[1..1 + b_card];
        let mut ri = 0usize;
        let mut ai = 0usize;
        let mut bi = 0usize;
        while ai < a_card && bi < b_card {
            if a_arr[ai] < b_arr[bi] {
                ai += 1;
            } else if b_arr[bi] < a_arr[ai] {
                bi += 1;
            } else {
                result.buffer[1 + ri] = a_arr[ai];
                ri += 1;
                ai += 1;
                bi += 1;
            }
        }
        if ri == 0 {
            return result.truncate();
        }
        result.buffer[0] = ri as u16;
        true
    }

    fn intersection_bitset(a: &RSet, b: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..MAX_SIZE as usize {
            let val = a.buffer[1 + i] & b.buffer[1 + i];
            result.buffer[1 + i] = val;
            cardinality += val.count_ones() as usize;
        }
        cardinality
    }
}
