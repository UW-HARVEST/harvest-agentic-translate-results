const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = 1 << 16 -1;

// Internal-only constants (kept private; original signatures/structs unchanged).
const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
// In the C implementation, max_size == low_cutoff (4096). We use the same
// notion of "max size" internally, deliberately distinct from the (broken)
// MAX_SIZE constant declared above.
const INTERNAL_MAX_SIZE: usize = LOW_CUTOFF;

#[allow(dead_code)]
fn _unused_max_size() -> u16 { MAX_SIZE }

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
        // Capture cardinality once, mirroring the C dispatch logic. The
        // conversions below do not modify buffer[0], so the captured value
        // remains correct for the subsequent dispatch.
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
        let cardinality = self.cardinality();
        if cardinality != comparison.cardinality() {
            return false;
        }
        let length_bytes = length_for(cardinality);
        if length_bytes == 0 {
            return true;
        }
        let words = length_bytes / 2; // length_for always returns an even count
        for i in 1..=words {
            if self.buffer[i] != comparison.buffer[i] {
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
        // new_card is always in [1, 65535] here, since we already handled
        // empty (=> 65536) and full (=> 0) above.
        result.buffer[0] = new_card as u16;
        if result.is_bitset() {
            result.invert_bitset();
        }
        true
    }
    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty_set() || other.is_empty_set() {
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
        // Fallback: bitset-style intersection (matches the C TODO behavior;
        // assumes both operands are bitsets).
        if !result.grow_to(INTERNAL_MAX_SIZE) {
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
        // Ensure buffer has at least 2 slots.
        if self.buffer.len() < 2 {
            self.buffer.resize(2, 0);
        }
        self.buffer[0] = 2;
        self.buffer[1] = MAX_ITEM;
        true
    }
    pub fn fill(&mut self) -> bool {
        if self.buffer.is_empty() {
            self.buffer.resize(1, 0);
        }
        self.buffer[0] = 0;
        true
    }
    pub fn export(&self) -> Vec<u8> {
        let len_bytes = self.length();
        let mut result = Vec::with_capacity(len_bytes);
        let words = (len_bytes + 1) / 2;
        for i in 0..words {
            if i < self.buffer.len() {
                result.extend_from_slice(&self.buffer[i].to_le_bytes());
            } else {
                result.extend_from_slice(&[0u8, 0u8]);
            }
        }
        result.truncate(len_bytes);
        result
    }
    pub fn length(&self) -> usize {
        2 + length_for(self.cardinality())
    }
    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length > 0 { length } else { 1 };
        if size > INTERNAL_MAX_SIZE {
            size = INTERNAL_MAX_SIZE;
        }
        let mut buf = vec![0u16; size + 1];
        if !buffer.is_empty() && length > 0 {
            let copy_len = length.min(buffer.len());
            let words_full = copy_len / 2;
            for i in 0..words_full {
                if i < buf.len() {
                    buf[i] = u16::from_le_bytes([buffer[2 * i], buffer[2 * i + 1]]);
                }
            }
            if copy_len % 2 == 1 && words_full < buf.len() {
                buf[words_full] = buffer[copy_len - 1] as u16;
            }
            RSet { buffer: buf, size }
        } else {
            buf[0] = 2;
            if buf.len() > 1 {
                buf[1] = MAX_ITEM;
            }
            RSet { buffer: buf, size }
        }
    }
    pub fn copy(&self) -> Self {
        let bytes = self.export();
        Self::import(&bytes, self.length())
    }

    // ---- Internal helpers below ----

    fn is_full(&self) -> bool {
        !self.buffer.is_empty() && self.buffer[0] == 0
    }

    fn is_empty_set(&self) -> bool {
        self.buffer.len() >= 2 && self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
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
        let mut size = self.size * GROWTH_FACTOR;
        if size > INTERNAL_MAX_SIZE {
            size = INTERNAL_MAX_SIZE;
        }
        self.grow_to(size)
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        if !self.grow_to(INTERNAL_MAX_SIZE) {
            return false;
        }
        let cardinality = self.buffer[0] as usize; // == LOW_CUTOFF
        let mut bitset = vec![0u16; INTERNAL_MAX_SIZE];
        for i in 0..cardinality {
            let item = self.buffer[1 + i];
            bitset[(item >> 4) as usize] |= 1u16 << (item & 0xF);
        }
        for i in 0..INTERNAL_MAX_SIZE {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array = vec![0u16; INTERNAL_MAX_SIZE];
        let mut ptr = 0usize;
        let mut bit: u32 = 0;
        for i in 0..INTERNAL_MAX_SIZE {
            let word = self.buffer[1 + i];
            for j in 0..16 {
                if (word & (1u16 << j)) == 0 {
                    if ptr < array.len() {
                        array[ptr] = bit as u16;
                    }
                    ptr += 1;
                }
                bit += 1;
            }
        }
        for i in 0..INTERNAL_MAX_SIZE {
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
        if cardinality == self.size {
            if !self.grow() {
                return false;
            }
        }
        if cardinality + 1 > i {
            // Shift buffer[i..=cardinality] right by one.
            for k in (i..=cardinality).rev() {
                self.buffer[k + 1] = self.buffer[k];
            }
        }
        self.buffer[i] = item;
        self.buffer[0] = self.buffer[0].wrapping_add(1);
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item >> 4) as usize + 1;
        let bit = 1u16 << (item & 0xF);
        if (self.buffer[offset] & bit) == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let n_missing = MAX_CARDINALITY - self.buffer[0] as usize;
        if n_missing == 0 {
            // Shouldn't happen: caller ensures the set isn't full here.
            return true;
        }
        if self.buffer[n_missing] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        for i in 0..n_missing {
            let v = self.buffer[1 + i];
            if v < item {
                continue;
            }
            if v > item {
                break;
            }
            // v == item: remove it from the missing list by shifting left.
            for k in i..(n_missing - 1) {
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
        let mut first: i64 = 0;
        let mut last: i64 = cardinality as i64 - 1;
        while first <= last {
            let middle = (first + last) / 2;
            let v = self.buffer[1 + middle as usize];
            if v == item {
                return true;
            }
            if v < item {
                first = middle + 1;
            } else {
                last = middle - 1;
            }
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let idx = (item >> 4) as usize + 1;
        (self.buffer[idx] & (1u16 << (item & 0xF))) != 0
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len_bytes = self.length();
        let words = (len_bytes + 1) / 2;
        for i in 0..words {
            if i < self.buffer.len() && i < dest.buffer.len() {
                dest.buffer[i] = self.buffer[i];
            }
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 1..=INTERNAL_MAX_SIZE {
            if i < self.buffer.len() {
                self.buffer[i] = !self.buffer[i];
            }
        }
    }

    fn intersection_array(&self, b: &RSet, result: &mut RSet) -> bool {
        let a_card = self.buffer[0] as usize;
        let b_card = b.buffer[0] as usize;
        let result_size = if a_card > b_card { a_card } else { b_card };
        if !result.grow_to(result_size) {
            return false;
        }
        let mut ai = 0usize;
        let mut bi = 0usize;
        let mut ri = 0usize;
        while ai < a_card && bi < b_card {
            let av = self.buffer[1 + ai];
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
        result.buffer[0] = ri as u16;
        if ri == 0 {
            result.truncate();
        }
        true
    }

    fn intersection_bitset(&self, b: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 1..=INTERNAL_MAX_SIZE {
            let v = self.buffer[i] & b.buffer[i];
            result.buffer[i] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
    }
}
