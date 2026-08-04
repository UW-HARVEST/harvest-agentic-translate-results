const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = 1 << 16 -1;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
const BITSET_SIZE: usize = LOW_CUTOFF; // 4096 u16 words = 65536 bits

pub struct RSet {
    pub buffer: Vec<u16>,
    size: usize,
}

impl RSet {
    fn is_empty(&self) -> bool {
        self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
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
        std::mem::size_of::<u16>() * c
    }

    fn grow_to(&mut self, new_size: usize) -> bool {
        if self.size >= new_size {
            return true;
        }
        self.buffer.resize(1 + new_size, 0);
        self.size = new_size;
        true
    }

    fn grow(&mut self) -> bool {
        let mut new_size = self.size * GROWTH_FACTOR;
        if new_size > BITSET_SIZE {
            new_size = BITSET_SIZE;
        }
        self.grow_to(new_size)
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        let mut bitset = vec![0u16; BITSET_SIZE];
        for i in 0..BITSET_SIZE {
            let val = self.buffer[1 + i] as usize;
            bitset[val >> 4] |= 1 << (val & 0xF);
        }
        self.buffer.resize(1 + BITSET_SIZE, 0);
        self.buffer[1..1 + BITSET_SIZE].copy_from_slice(&bitset);
        self.size = BITSET_SIZE;
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array = Vec::with_capacity(BITSET_SIZE);
        let bitset = &self.buffer[1..1 + BITSET_SIZE];
        let mut bit: u32 = 0;
        for i in 0..BITSET_SIZE {
            for j in 0..16u16 {
                if bitset[i] & (1 << j) == 0 {
                    array.push(bit as u16);
                }
                bit += 1;
            }
        }
        // Pad to BITSET_SIZE
        array.resize(BITSET_SIZE, 0);
        self.buffer[1..1 + BITSET_SIZE].copy_from_slice(&array);
        true
    }

    fn contains_array(&self, item: u16) -> bool {
        let mut cardinality = self.buffer[0] as usize;
        if cardinality > HIGH_CUTOFF {
            cardinality = MAX_CARDINALITY - cardinality;
        }
        let array = &self.buffer[1..1 + cardinality];
        array.binary_search(&item).is_ok()
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let idx = (item >> 4) as usize + 1;
        self.buffer[idx] & (1 << (item & 0xF)) != 0
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        // Fast path: append if item > last element
        let i = if cardinality > 0 && self.buffer[cardinality] < item {
            cardinality + 1
        } else {
            // Binary search for insertion point
            match self.buffer[1..1 + cardinality].binary_search(&item) {
                Ok(_) => return true, // already present
                Err(pos) => pos + 1,
            }
        };
        if cardinality == self.size && !self.grow() {
            return false;
        }
        // Shift elements right
        if cardinality + 1 > i {
            // Ensure buffer is large enough
            if self.buffer.len() <= cardinality + 1 {
                self.buffer.resize(cardinality + 2, 0);
            }
            let src = i;
            let count = cardinality + 1 - i;
            // memmove equivalent
            for k in (0..count).rev() {
                self.buffer[src + 1 + k] = self.buffer[src + k];
            }
        } else if self.buffer.len() <= i {
            self.buffer.resize(i + 1, 0);
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
        // Check last element
        if self.buffer[cardinality] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        let array = &self.buffer[1..1 + cardinality];
        for i in 0..cardinality {
            if array[i] < item {
                continue;
            }
            if array[i] > item {
                break;
            }
            // Found: remove from inverted array
            for k in i..cardinality - 1 {
                self.buffer[1 + k] = self.buffer[1 + k + 1];
            }
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        true
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len = self.length();
        let words = (len + 1) / 2; // number of u16 words
        for i in 0..words {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 1..1 + BITSET_SIZE {
            self.buffer[i] = !self.buffer[i];
        }
    }

    fn intersection_array(a: &RSet, b: &RSet, result: &mut RSet) -> bool {
        let a_card = a.buffer[0] as usize;
        let b_card = b.buffer[0] as usize;
        let result_size = a_card.max(b_card);
        if !result.grow_to(result_size) {
            return false;
        }
        // Naive sorted intersection
        let mut ia = 0usize;
        let mut ib = 0usize;
        let mut count = 0usize;
        while ia < a_card && ib < b_card {
            let va = a.buffer[1 + ia];
            let vb = b.buffer[1 + ib];
            if va < vb {
                ia += 1;
            } else if vb < va {
                ib += 1;
            } else {
                result.buffer[1 + count] = va;
                count += 1;
                ia += 1;
                ib += 1;
            }
        }
        if count == 0 {
            return result.truncate();
        }
        result.buffer[0] = count as u16;
        true
    }

    fn intersection_bitset(a: &RSet, b: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..BITSET_SIZE {
            let v = a.buffer[1 + i] & b.buffer[1 + i];
            result.buffer[1 + i] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
    }

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
        let card = self.cardinality();
        if card != comparison.cardinality() {
            return false;
        }
        let len = Self::length_for(card);
        if len == 0 {
            return true;
        }
        let words = len / 2;
        self.buffer[1..1 + words] == comparison.buffer[1..1 + words]
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
        result.buffer[0] = (MAX_CARDINALITY as u32 - result.buffer[0] as u32) as u16;
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
        }
        if other.is_full() {
            return self.copy_to(result);
        }
        if self.is_array() && other.is_array() {
            return Self::intersection_array(self, other, result);
        }
        if !result.grow_to(BITSET_SIZE) {
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
        let len = self.length();
        let mut out = vec![0u8; len];
        let words = len / 2;
        for i in 0..words {
            let bytes = self.buffer[i].to_ne_bytes();
            out[i * 2] = bytes[0];
            out[i * 2 + 1] = bytes[1];
        }
        out
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + Self::length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let size = if length == 0 { 1 } else { length.min(BITSET_SIZE) };
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            // Copy bytes into u16 buffer
            let byte_len = buffer.len().min(length);
            let words = byte_len / 2;
            for i in 0..words {
                buf[i] = u16::from_ne_bytes([buffer[i * 2], buffer[i * 2 + 1]]);
            }
        } else {
            // truncate (empty set)
            buf[0] = 2;
            buf[1] = MAX_ITEM;
        }
        RSet { buffer: buf, size }
    }

    pub fn copy(&self) -> Self {
        let exported = self.export();
        Self::import(&exported, self.length())
    }
}
