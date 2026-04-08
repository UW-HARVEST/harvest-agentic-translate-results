const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = LOW_CUTOFF as u16;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;

pub struct RSet {
    buffer: Vec<u16>,
    size: usize,
}

// Internal helpers
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
        if size > MAX_SIZE as usize {
            size = MAX_SIZE as usize;
        }
        self.grow_to(size)
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        let mut bitset = vec![0u16; MAX_SIZE as usize];
        let card = self.buffer[0] as usize;
        for i in 0..card {
            let item = self.buffer[1 + i] as usize;
            bitset[item >> 4] |= 1 << (item & 0xF);
        }
        // Ensure buffer is large enough
        self.grow_to(MAX_SIZE as usize);
        for i in 0..MAX_SIZE as usize {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array = vec![0u16; MAX_SIZE as usize];
        let mut ptr = 0usize;
        let mut bit = 0u32;
        for i in 0..MAX_SIZE as usize {
            for j in 0..16u16 {
                if self.buffer[1 + i] & (1 << j) == 0 {
                    array[ptr] = bit as u16;
                    ptr += 1;
                }
                bit += 1;
            }
        }
        for i in 0..MAX_SIZE as usize {
            self.buffer[1 + i] = array[i];
        }
        true
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let insert_pos;
        if cardinality > 0 && self.buffer[cardinality] < item {
            insert_pos = cardinality + 1;
        } else {
            let mut found = None;
            for i in 1..=cardinality {
                if self.buffer[i] < item {
                    continue;
                }
                if self.buffer[i] == item {
                    return true;
                }
                found = Some(i);
                break;
            }
            insert_pos = found.unwrap_or(cardinality + 1);
        }
        if cardinality == self.size && !self.grow() {
            return false;
        }
        // Shift elements right
        if cardinality + 1 > insert_pos {
            // memmove equivalent
            let src = insert_pos;
            let count = cardinality + 1 - insert_pos;
            // Need to copy from end to avoid overlap issues
            for k in (0..count).rev() {
                self.buffer[src + 1 + k] = self.buffer[src + k];
            }
        }
        self.buffer[insert_pos] = item;
        self.buffer[0] += 1;
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        if self.buffer[offset] & bit == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] += 1;
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let cardinality = MAX_CARDINALITY - self.buffer[0] as usize;
        if self.buffer[cardinality] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        for i in 0..cardinality {
            if self.buffer[1 + i] < item {
                continue;
            }
            if self.buffer[1 + i] > item {
                break;
            }
            // Found it - remove from inverted array
            for k in i..cardinality - 1 {
                self.buffer[1 + k] = self.buffer[1 + k + 1];
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
            let middle = (first + last) / 2;
            let val = self.buffer[1 + middle as usize];
            if val == item {
                return true;
            }
            if val < item {
                first = middle + 1;
            } else {
                last = middle - 1;
            }
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        self.buffer[(item as usize >> 4) + 1] & (1 << (item & 0xF)) != 0
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len = self.length();
        // length is in bytes, but buffer is u16, so copy len/2 elements
        let elems = len / 2;
        for i in 0..elems {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 0..MAX_SIZE as usize {
            self.buffer[1 + i] = !self.buffer[1 + i];
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
        std::mem::size_of::<u16>() * c
    }
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
        let cardinality = self.cardinality();
        if cardinality != comparison.cardinality() {
            return false;
        }
        let length = Self::length_for(cardinality);
        if length == 0 {
            return true;
        }
        let elems = length / 2;
        for i in 0..elems {
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

        // Both need to be bitset-compatible; grow result
        if !result.grow_to(MAX_SIZE as usize) {
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

    fn intersection_array(a: &RSet, b: &RSet, result: &mut RSet) -> bool {
        let a_card = a.buffer[0] as usize;
        let b_card = b.buffer[0] as usize;
        let result_size = std::cmp::max(a_card, b_card);
        if !result.grow_to(result_size) {
            return false;
        }
        let mut ri = 0usize;
        let mut ai = 0usize;
        let mut bi = 0usize;
        while ai < a_card && bi < b_card {
            let av = a.buffer[1 + ai];
            let bv = b.buffer[1 + bi];
            if av < bv {
                ai += 1;
            } else if bv < av {
                bi += 1;
            } else {
                result.buffer[1 + ri] = av;
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
            let v = a.buffer[1 + i] & b.buffer[1 + i];
            result.buffer[1 + i] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
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
        let elems = len / 2;
        let mut bytes = Vec::with_capacity(len);
        for i in 0..elems {
            bytes.extend_from_slice(&self.buffer[i].to_ne_bytes());
        }
        bytes
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + Self::length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length > 0 { length } else { 1 };
        if size > MAX_SIZE as usize {
            size = MAX_SIZE as usize;
        }
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            // Copy bytes into u16 buffer
            let byte_count = buffer.len().min(length);
            let u16_count = byte_count / 2;
            for i in 0..u16_count {
                buf[i] = u16::from_ne_bytes([buffer[2 * i], buffer[2 * i + 1]]);
            }
        } else {
            // truncate: empty set
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
