const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE: u16 = 1 << 16 - 1;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
// The actual maximum buffer size matching C's `max_size = low_cutoff = 4096`.
const ACTUAL_MAX_SIZE: usize = LOW_CUTOFF;

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

pub struct RSet {
    pub buffer: Vec<u16>,
    pub size: usize,
}

impl RSet {
    pub fn new() -> Self {
        Self::import_internal(None, DEFAULT_SIZE)
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
        let length_bytes = length_for(card);
        if length_bytes == 0 {
            return true;
        }
        let u16_count = length_bytes / 2;
        for i in 0..u16_count {
            if self.buffer[i + 1] != comparison.buffer[i + 1] {
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
        let new_card = MAX_CARDINALITY - result.buffer[0] as usize;
        // new_card fits in u16 (1..=65535) since original is in (0, max_cardinality).
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
        }
        if other.is_full() {
            return self.copy_to(result);
        }
        if self.is_array() && other.is_array() {
            return self.intersection_array(other, result);
        }

        // Fall back to bitset intersection (matches C behavior, though it
        // assumes both sets are bitsets).
        if !result.grow_to(ACTUAL_MAX_SIZE) {
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
        let mut bytes = Vec::with_capacity(self.buffer.len() * 2);
        for &v in &self.buffer {
            bytes.push((v & 0xFF) as u8);
            bytes.push(((v >> 8) & 0xFF) as u8);
        }
        bytes
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        Self::import_internal(Some(buffer), length)
    }

    pub fn copy(&self) -> Self {
        let exported = self.export();
        Self::import(&exported, self.length())
    }
}

// Private helpers
impl RSet {
    fn import_internal(data: Option<&[u8]>, length: usize) -> Self {
        let mut size = if length == 0 { 1 } else { length };
        if size > ACTUAL_MAX_SIZE {
            size = ACTUAL_MAX_SIZE;
        }
        let mut set = RSet {
            buffer: vec![0u16; 1 + size],
            size,
        };

        match data {
            Some(bytes) if length > 0 && !bytes.is_empty() => {
                let bytes_to_copy = length.min(bytes.len());
                let buffer_byte_capacity = set.buffer.len() * 2;
                let actual_copy = bytes_to_copy.min(buffer_byte_capacity);
                let full_u16s = actual_copy / 2;
                for i in 0..full_u16s {
                    let lo = bytes[2 * i] as u16;
                    let hi = bytes[2 * i + 1] as u16;
                    set.buffer[i] = lo | (hi << 8);
                }
                if actual_copy % 2 != 0 {
                    set.buffer[full_u16s] = bytes[actual_copy - 1] as u16;
                }
            }
            _ => {
                set.truncate();
            }
        }
        set
    }

    fn is_empty(&self) -> bool {
        self.buffer.len() >= 2 && self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full(&self) -> bool {
        !self.buffer.is_empty() && self.buffer[0] == 0
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
        if size > ACTUAL_MAX_SIZE {
            size = ACTUAL_MAX_SIZE;
        }
        self.grow_to(size)
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let length_bytes = self.length();
        let u16_count = length_bytes / 2;
        for i in 0..u16_count {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let mut i;
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
        self.buffer[0] += 1;
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
        let cardinality = self.buffer[0] as usize;
        let inverted_count = MAX_CARDINALITY - cardinality;
        // Fast path: check the last element of the inverted array.
        if inverted_count > 0 && self.buffer[inverted_count] == item {
            // Increment cardinality (decreasing inverted_count by 1).
            // Use wrapping_add since cardinality might reach max_cardinality (= 0 mod 2^16).
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        // Scan the inverted array for `item`.
        for i in 0..inverted_count {
            let v = self.buffer[i + 1];
            if v < item {
                continue;
            }
            if v > item {
                break;
            }
            // Found at index i — remove it.
            for j in i..(inverted_count - 1) {
                self.buffer[j + 1] = self.buffer[j + 2];
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
            let v = self.buffer[middle + 1];
            if v == item {
                return true;
            }
            if v < item {
                first = middle as isize + 1;
            } else {
                last = middle as isize - 1;
            }
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let offset = (item >> 4) as usize + 1;
        let bit = 1u16 << (item & 0xF);
        (self.buffer[offset] & bit) != 0
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        let cardinality = self.buffer[0] as usize;
        let mut bitset = vec![0u16; LOW_CUTOFF];
        for i in 0..cardinality {
            let v = self.buffer[i + 1];
            bitset[(v >> 4) as usize] |= 1u16 << (v & 0xF);
        }
        if !self.grow_to(LOW_CUTOFF) {
            return false;
        }
        for i in 0..LOW_CUTOFF {
            self.buffer[i + 1] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array: Vec<u16> = Vec::with_capacity(LOW_CUTOFF);
        for i in 0..LOW_CUTOFF {
            let bits = self.buffer[i + 1];
            for j in 0..16 {
                let bit_pos = (i * 16 + j) as u16;
                if (bits & (1u16 << j)) == 0 {
                    array.push(bit_pos);
                }
            }
        }
        for i in 0..array.len() {
            self.buffer[i + 1] = array[i];
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 0..LOW_CUTOFF {
            self.buffer[i + 1] = !self.buffer[i + 1];
        }
    }

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let a_card = self.buffer[0] as usize;
        let b_card = other.buffer[0] as usize;
        let result_size = a_card.max(b_card);
        if !result.grow_to(result_size) {
            return false;
        }
        let mut a_idx = 0usize;
        let mut b_idx = 0usize;
        let mut r_idx = 0usize;
        while a_idx < a_card && b_idx < b_card {
            let av = self.buffer[a_idx + 1];
            let bv = other.buffer[b_idx + 1];
            if av < bv {
                a_idx += 1;
            } else if bv < av {
                b_idx += 1;
            } else {
                result.buffer[r_idx + 1] = av;
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
        let mut cardinality = 0usize;
        for i in 0..LOW_CUTOFF {
            let v = self.buffer[i + 1] & other.buffer[i + 1];
            result.buffer[i + 1] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
    }
}

// Suppress unused-constant warnings for constants required by the spec.
#[allow(dead_code)]
const _UNUSED_MAX_SIZE: u16 = MAX_SIZE;
