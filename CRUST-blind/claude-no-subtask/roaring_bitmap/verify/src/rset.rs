const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
#[allow(dead_code)]
const MAX_SIZE: u16 = 1 << 16 - 1;

// Internal constants matching the C implementation.
const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
// Maximum number of u16 slots in the buffer (excluding the cardinality slot at index 0).
const BUFFER_MAX_SIZE: usize = LOW_CUTOFF;

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
        let card = self.cardinality();
        if card != comparison.cardinality() {
            return false;
        }
        let n_u16s = length_for_u16s(card);
        if n_u16s == 0 {
            return true;
        }
        if 1 + n_u16s > self.buffer.len() || 1 + n_u16s > comparison.buffer.len() {
            return false;
        }
        self.buffer[1..1 + n_u16s] == comparison.buffer[1..1 + n_u16s]
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
        let new_card = MAX_CARDINALITY.wrapping_sub(result.buffer[0] as usize);
        // new_card fits in u16 since result.buffer[0] is in [1, 65535] (excluding 0=full).
        // For new_card == 65536 it would only happen if buffer[0] == 0 which is full
        // (handled above), so this is safe.
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

        // For bitset intersection both operands need to be bitsets and the result
        // buffer needs to be large enough.
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
        let mut result = Vec::with_capacity(len_bytes);
        let n_u16s = len_bytes / 2;
        for i in 0..n_u16s {
            let bytes = self.buffer[i].to_le_bytes();
            result.push(bytes[0]);
            result.push(bytes[1]);
        }
        // Handle odd byte (shouldn't happen in practice since length is always even)
        if len_bytes % 2 == 1 && n_u16s < self.buffer.len() {
            let bytes = self.buffer[n_u16s].to_le_bytes();
            result.push(bytes[0]);
        }
        result
    }

    pub fn length(&self) -> usize {
        2 + 2 * length_for_u16s(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length > 0 { length } else { 1 };
        if size > BUFFER_MAX_SIZE {
            size = BUFFER_MAX_SIZE;
        }
        let mut buf = vec![0u16; 1 + size];

        if !buffer.is_empty() && length > 0 {
            // Copy `length` bytes (capped at the buffer's byte capacity) into buf.
            let max_bytes = 2 * (1 + size);
            let copy_bytes = length.min(buffer.len()).min(max_bytes);
            let n_full_u16s = copy_bytes / 2;
            for i in 0..n_full_u16s {
                buf[i] = u16::from_le_bytes([buffer[2 * i], buffer[2 * i + 1]]);
            }
            if copy_bytes % 2 == 1 {
                buf[n_full_u16s] = u16::from_le_bytes([buffer[2 * n_full_u16s], 0]);
            }
            RSet { buffer: buf, size }
        } else {
            let mut s = RSet { buffer: buf, size };
            s.truncate();
            s
        }
    }

    pub fn copy(&self) -> Self {
        let exported = self.export();
        Self::import(&exported, self.length())
    }
}

// ===== Private helpers =====

impl RSet {
    fn is_empty_state(&self) -> bool {
        self.buffer.len() >= 2 && self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full(&self) -> bool {
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
        let mut size = self.size.saturating_mul(GROWTH_FACTOR);
        if size > BUFFER_MAX_SIZE {
            size = BUFFER_MAX_SIZE;
        }
        self.grow_to(size)
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len_bytes = self.length();
        let n_u16s = len_bytes / 2;
        for i in 0..n_u16s {
            dest.buffer[i] = self.buffer[i];
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
                let val = self.buffer[i];
                if val < item {
                    i += 1;
                    continue;
                }
                if val == item {
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
            // Shift right: positions [i..=cardinality] move to [i+1..=cardinality+1]
            for j in (i..=cardinality).rev() {
                self.buffer[j + 1] = self.buffer[j];
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
        let cardinality_inverted = MAX_CARDINALITY - self.buffer[0] as usize;
        if cardinality_inverted == 0 {
            // Set is full, nothing to add.
            return true;
        }
        if self.buffer[cardinality_inverted] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        // Search for `item` in the inverted array (buffer[1..=cardinality_inverted]).
        for i in 0..cardinality_inverted {
            let val = self.buffer[1 + i];
            if val < item {
                continue;
            }
            if val > item {
                break;
            }
            // val == item: remove it from the inverted array by shifting the
            // following elements left by one.
            for j in i..(cardinality_inverted - 1) {
                self.buffer[1 + j] = self.buffer[1 + j + 1];
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
        let offset = (item >> 4) as usize + 1;
        let bit = 1u16 << (item & 0xF);
        (self.buffer[offset] & bit) != 0
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        if !self.grow_to(BUFFER_MAX_SIZE) {
            return false;
        }
        let mut bitset = vec![0u16; BUFFER_MAX_SIZE];
        // The array currently has LOW_CUTOFF items at buffer[1..=LOW_CUTOFF].
        for i in 0..LOW_CUTOFF {
            let item = self.buffer[1 + i];
            bitset[(item >> 4) as usize] |= 1u16 << (item & 0xF);
        }
        for i in 0..BUFFER_MAX_SIZE {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        if !self.grow_to(BUFFER_MAX_SIZE) {
            return false;
        }
        let mut array = vec![0u16; BUFFER_MAX_SIZE];
        let mut bit: u32 = 0;
        let mut ptr_idx: usize = 0;
        for i in 0..BUFFER_MAX_SIZE {
            let val = self.buffer[1 + i];
            for j in 0..16u32 {
                if (val & (1u16 << j)) == 0 && ptr_idx < BUFFER_MAX_SIZE {
                    array[ptr_idx] = bit as u16;
                    ptr_idx += 1;
                }
                bit += 1;
            }
        }
        for i in 0..BUFFER_MAX_SIZE {
            self.buffer[1 + i] = array[i];
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 0..BUFFER_MAX_SIZE {
            self.buffer[1 + i] = !self.buffer[1 + i];
        }
    }

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let a_size = self.buffer[0] as usize;
        let b_size = other.buffer[0] as usize;
        let result_size = a_size.max(b_size);
        if !result.grow_to(result_size) {
            return false;
        }

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

    fn intersection_bitset(&self, other: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..BUFFER_MAX_SIZE {
            let val = self.buffer[1 + i] & other.buffer[1 + i];
            result.buffer[1 + i] = val;
            cardinality += val.count_ones() as usize;
        }
        cardinality
    }
}

fn length_for_u16s(cardinality: usize) -> usize {
    let mut c = cardinality;
    if c == 0 {
        c = 1;
    } else if c >= HIGH_CUTOFF {
        c = MAX_CARDINALITY - c;
    } else if c > LOW_CUTOFF {
        c = LOW_CUTOFF;
    }
    c
}
