const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE: u16 = 1 << 16 - 1;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
// The C max_size is low_cutoff = 4096 (the Rust MAX_SIZE constant
// above evaluates differently due to Rust operator precedence; see C source).
const MAX_BUFFER_SIZE: usize = LOW_CUTOFF;

pub struct RSet {
    pub buffer: Vec<u16>,
    pub size: usize,
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
        if self.is_full_internal() {
            return MAX_CARDINALITY;
        }
        if self.is_empty_internal() {
            return 0;
        }
        self.buffer[0] as usize
    }

    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full_internal() {
            return true;
        }
        if self.is_empty_internal() {
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
        if self.is_full_internal() {
            return true;
        }
        if self.is_empty_internal() {
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
        let length = length_for(cardinality);
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
        if self.is_empty_internal() {
            return result.fill();
        }
        if self.is_full_internal() {
            return result.truncate();
        }
        if !self.copy_to(result) {
            return false;
        }
        let cardinality = result.buffer[0] as usize;
        let new_card = MAX_CARDINALITY - cardinality;
        // new_card fits in u16 since cardinality >= 1 here
        result.buffer[0] = (new_card & 0xFFFF) as u16;
        if result.is_bitset() {
            result.invert_bitset();
        }
        true
    }

    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty_internal() || other.is_empty_internal() {
            return result.truncate();
        }
        if self.is_full_internal() {
            return other.copy_to(result);
        }
        if other.is_full_internal() {
            return self.copy_to(result);
        }
        if self.is_array() && other.is_array() {
            return Self::intersection_array(self, other, result);
        }
        // bitset intersection (assumes both are bitsets, matching the C TODO)
        if !result.grow_to(MAX_BUFFER_SIZE) {
            return false;
        }
        let mut cardinality: usize = 0;
        for i in 0..MAX_BUFFER_SIZE {
            let v = self.buffer[1 + i] & other.buffer[1 + i];
            result.buffer[1 + i] = v;
            cardinality += v.count_ones() as usize;
        }
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
        let mut bytes = Vec::with_capacity(len);
        let n = len / 2;
        for i in 0..n {
            let v = self.buffer[i];
            bytes.push((v & 0xFF) as u8);
            bytes.push(((v >> 8) & 0xFF) as u8);
        }
        bytes
    }

    pub fn length(&self) -> usize {
        2 + length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length > 0 { length } else { 1 };
        if size > MAX_BUFFER_SIZE {
            size = MAX_BUFFER_SIZE;
        }
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            // Copy `length` bytes from buffer into buf as little-endian u16s
            let cap_bytes = buf.len() * 2;
            let to_copy = length.min(cap_bytes).min(buffer.len());
            let n_full = to_copy / 2;
            for i in 0..n_full {
                let lo = buffer[i * 2] as u16;
                let hi = buffer[i * 2 + 1] as u16;
                buf[i] = lo | (hi << 8);
            }
            if to_copy % 2 == 1 && n_full < buf.len() {
                buf[n_full] = buffer[to_copy - 1] as u16;
            }
        } else {
            buf[0] = 2;
            buf[1] = MAX_ITEM;
        }
        Self { buffer: buf, size }
    }

    pub fn copy(&self) -> Self {
        Self::import(&self.export(), self.length())
    }

    // ----- Helpers -----

    fn is_empty_internal(&self) -> bool {
        self.buffer.len() >= 2 && self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full_internal(&self) -> bool {
        !self.buffer.is_empty() && self.buffer[0] == 0
    }

    fn is_bitset(&self) -> bool {
        let card = self.buffer[0] as usize;
        card > LOW_CUTOFF && card <= HIGH_CUTOFF
    }

    fn is_array(&self) -> bool {
        let card = self.buffer[0] as usize;
        card <= LOW_CUTOFF
    }

    fn is_inverted_array(&self) -> bool {
        let card = self.buffer[0] as usize;
        card > HIGH_CUTOFF
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
        if size > MAX_BUFFER_SIZE {
            size = MAX_BUFFER_SIZE;
        }
        self.grow_to(size)
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
        if cardinality == self.size {
            if !self.grow() {
                return false;
            }
        }
        if cardinality + 1 > i {
            // shift buffer[i..cardinality+1] -> buffer[i+1..cardinality+2]
            self.buffer.copy_within(i..(cardinality + 1), i + 1);
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
            self.buffer[0] += 1;
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
            let v = self.buffer[1 + i];
            if v < item {
                continue;
            }
            if v > item {
                break;
            }
            // v == item, remove it from inverted array
            if i + 1 < cardinality {
                self.buffer.copy_within((i + 2)..(cardinality + 1), i + 1);
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
        let mut middle: i64 = (first + last) / 2;
        while first <= last {
            let v = self.buffer[1 + middle as usize];
            if v == item {
                return true;
            }
            if v < item {
                first = middle + 1;
            } else {
                last = middle - 1;
            }
            middle = (first + last) / 2;
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let offset = (item >> 4) as usize + 1;
        let bit = 1u16 << (item & 0xF);
        self.buffer[offset] & bit != 0
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        if !self.grow_to(MAX_BUFFER_SIZE) {
            return false;
        }
        let mut bitset = vec![0u16; MAX_BUFFER_SIZE];
        // The current array has LOW_CUTOFF items at buffer[1..1+LOW_CUTOFF]
        for i in 0..MAX_BUFFER_SIZE {
            let item = self.buffer[1 + i];
            bitset[(item >> 4) as usize] |= 1u16 << (item & 0xF);
        }
        for i in 0..MAX_BUFFER_SIZE {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array: Vec<u16> = Vec::with_capacity(MAX_BUFFER_SIZE);
        for i in 0..MAX_BUFFER_SIZE {
            let bits = self.buffer[1 + i];
            for j in 0..16 {
                let bit_val = (i * 16 + j) as u16;
                if bits & (1u16 << j) == 0 {
                    array.push(bit_val);
                }
            }
        }
        // Copy at least MAX_BUFFER_SIZE values; pad with 0 if fewer
        // (in practice array.len() == MAX_BUFFER_SIZE at conversion time)
        for i in 0..MAX_BUFFER_SIZE {
            self.buffer[1 + i] = if i < array.len() { array[i] } else { 0 };
        }
        true
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len = self.length();
        let n = len / 2;
        for i in 0..n {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 0..MAX_BUFFER_SIZE {
            self.buffer[1 + i] = !self.buffer[1 + i];
        }
    }

    fn intersection_array(a: &RSet, b: &RSet, result: &mut RSet) -> bool {
        let a_card = a.buffer[0] as usize;
        let b_card = b.buffer[0] as usize;
        let result_size = a_card.max(b_card);
        if !result.grow_to(result_size) {
            return false;
        }
        let mut i = 0usize;
        let mut j = 0usize;
        let mut k: usize = 0;
        while i < a_card && j < b_card {
            let av = a.buffer[1 + i];
            let bv = b.buffer[1 + j];
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

#[allow(dead_code)]
fn _unused_max_size() -> u16 {
    MAX_SIZE
}
