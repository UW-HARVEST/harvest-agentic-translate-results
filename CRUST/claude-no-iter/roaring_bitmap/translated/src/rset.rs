const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = 1 << 16 -1;

// Internal helpers
const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
// The actual storage cap (matches C's `max_size`, which equals low_cutoff).
const STORAGE_CAP: usize = LOW_CUTOFF;

pub struct RSet {
    pub buffer: Vec<u16>,
    size: usize,
}

impl RSet {
    pub fn new() -> Self {
        Self::import(&[], DEFAULT_SIZE)
    }

    pub fn free(&mut self) {
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
        let cardinality = self.cardinality();
        if cardinality != comparison.cardinality() {
            return false;
        }
        let len_u16 = Self::length_for_u16(cardinality);
        if len_u16 == 0 {
            return true;
        }
        for i in 0..len_u16 {
            if self.buffer[1 + i] != comparison.buffer[1 + i] {
                return false;
            }
        }
        true
    }

    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty_set() {
            // ~0 => U
            return result.fill();
        }
        if self.is_full() {
            // ~U => 0
            return result.truncate();
        }
        if !self.copy_to(result) {
            return false;
        }
        let new_card = (MAX_CARDINALITY - result.buffer[0] as usize) as u16;
        result.buffer[0] = new_card;
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
            return Self::intersection_array(self, other, result);
        }

        // Bitset intersection path: assumes both operands occupy a full
        // bitset-sized buffer (matches the C implementation).
        if !result.grow_to(STORAGE_CAP) {
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
        // The buffer must hold at least 2 slots to encode the empty marker.
        if self.buffer.len() < 2 {
            self.buffer.resize(2, 0);
            if self.size < 1 {
                self.size = 1;
            }
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
        let mut out = Vec::with_capacity(len_bytes);
        let n_u16 = len_bytes / 2;
        for i in 0..n_u16 {
            let v = self.buffer[i];
            out.push((v & 0xFF) as u8);
            out.push((v >> 8) as u8);
        }
        out
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + Self::length_for_bytes(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length == 0 { 1 } else { length };
        if size > STORAGE_CAP {
            size = STORAGE_CAP;
        }
        let buf = vec![0u16; 1 + size];
        let mut set = RSet { buffer: buf, size };
        if !buffer.is_empty() && length > 0 {
            // Copy `length` bytes (little-endian u16's) into set.buffer.
            let n_u16 = length / 2;
            let take = n_u16.min(set.buffer.len());
            for i in 0..take {
                let lo_idx = i * 2;
                let hi_idx = lo_idx + 1;
                let lo = if lo_idx < buffer.len() {
                    buffer[lo_idx] as u16
                } else {
                    0
                };
                let hi = if hi_idx < buffer.len() {
                    buffer[hi_idx] as u16
                } else {
                    0
                };
                set.buffer[i] = lo | (hi << 8);
            }
        } else {
            set.truncate();
        }
        set
    }

    pub fn copy(&self) -> Self {
        Self::import(&self.export(), self.length())
    }
}

// ===== Internal helpers =====
impl RSet {
    fn is_empty_set(&self) -> bool {
        self.buffer.len() >= 2 && self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full(&self) -> bool {
        !self.buffer.is_empty() && self.buffer[0] == 0
    }

    fn is_bitset(&self) -> bool {
        let cardinality = self.buffer[0] as usize;
        cardinality > LOW_CUTOFF && cardinality <= HIGH_CUTOFF
    }

    fn is_array(&self) -> bool {
        let cardinality = self.buffer[0] as usize;
        cardinality <= LOW_CUTOFF
    }

    fn is_inverted_array(&self) -> bool {
        let cardinality = self.buffer[0] as usize;
        cardinality > HIGH_CUTOFF
    }

    /// Number of u16 items needed to represent the data portion (not counting
    /// the cardinality slot) for a given cardinality.
    fn length_for_u16(cardinality: usize) -> usize {
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

    fn length_for_bytes(cardinality: usize) -> usize {
        std::mem::size_of::<u16>() * Self::length_for_u16(cardinality)
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
        if size > STORAGE_CAP {
            size = STORAGE_CAP;
        }
        self.grow_to(size)
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        // Allocate scratch for the bitset (max_size u16's).
        let mut bitset = vec![0u16; STORAGE_CAP];
        // Read existing array (LOW_CUTOFF items at indices 1..=LOW_CUTOFF).
        for i in 0..STORAGE_CAP {
            let v = self.buffer[1 + i];
            bitset[(v >> 4) as usize] |= 1u16 << (v & 0xF);
        }
        if !self.grow_to(STORAGE_CAP) {
            return false;
        }
        for i in 0..STORAGE_CAP {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array = vec![0u16; STORAGE_CAP];
        let mut ptr = 0usize;
        let mut bit: u32 = 0;
        for i in 0..STORAGE_CAP {
            let word = self.buffer[1 + i];
            for j in 0..16u32 {
                if (word & (1u16 << j)) == 0 {
                    if ptr < STORAGE_CAP {
                        array[ptr] = bit as u16;
                        ptr += 1;
                    }
                }
                bit += 1;
            }
        }
        for i in 0..STORAGE_CAP {
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
            let mut found = false;
            while i <= cardinality {
                if self.buffer[i] < item {
                    i += 1;
                    continue;
                }
                if self.buffer[i] == item {
                    return true;
                }
                found = true;
                break;
            }
            // If we never broke, i == cardinality + 1 at this point.
            let _ = found;
        }

        if cardinality == self.size {
            if !self.grow() {
                return false;
            }
        }

        if cardinality + 1 > i {
            // Shift buffer[i..=cardinality] right by 1.
            let count = cardinality + 1 - i;
            // Copy from end to beginning to avoid overlap issues.
            for k in (0..count).rev() {
                self.buffer[i + 1 + k] = self.buffer[i + k];
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
            self.buffer[0] += 1;
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let cardinality = MAX_CARDINALITY - self.buffer[0] as usize;
        // Special-case: the item is the tail of the inverted array.
        if cardinality > 0 && self.buffer[cardinality] == item {
            // Use wrapping_add to allow the transition from a 65535-cardinality
            // set to the "full" marker (0) when the last missing item is
            // added.
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        // Walk the array (which lives at indices 1..=cardinality).
        for i in 0..cardinality {
            let v = self.buffer[1 + i];
            if v < item {
                continue;
            }
            if v > item {
                break;
            }
            // v == item: remove it from the inverted array.  We shift the
            // tail (`cardinality - i - 1` elements) one slot to the left.
            let count = cardinality - i - 1;
            for k in 0..count {
                self.buffer[1 + i + k] = self.buffer[1 + i + k + 1];
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
            let m = middle as usize;
            let v = self.buffer[1 + m];
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
        (self.buffer[offset] & bit) != 0
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len_u16 = self.length() / 2;
        for i in 0..len_u16 {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 0..STORAGE_CAP {
            self.buffer[1 + i] = !self.buffer[1 + i];
        }
    }

    fn intersection_array(a: &RSet, b: &RSet, result: &mut RSet) -> bool {
        let card_a = a.buffer[0] as usize;
        let card_b = b.buffer[0] as usize;
        let result_size = card_a.max(card_b);
        if !result.grow_to(result_size) {
            return false;
        }
        // Sorted merge intersection.
        let mut i = 0usize;
        let mut j = 0usize;
        let mut out = 0usize;
        while i < card_a && j < card_b {
            let av = a.buffer[1 + i];
            let bv = b.buffer[1 + j];
            if av < bv {
                i += 1;
            } else if bv < av {
                j += 1;
            } else {
                result.buffer[1 + out] = av;
                out += 1;
                i += 1;
                j += 1;
            }
        }
        result.buffer[0] = out as u16;
        if out == 0 {
            result.truncate();
        }
        true
    }

    fn intersection_bitset(a: &RSet, b: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..STORAGE_CAP {
            let av = a.buffer[1 + i];
            let bv = b.buffer[1 + i];
            let r = av & bv;
            result.buffer[1 + i] = r;
            cardinality += r.count_ones() as usize;
        }
        cardinality
    }
}

// Silence unused-constant warning for the existing typo'd constant.
#[allow(dead_code)]
const _UNUSED_MAX_SIZE: u16 = MAX_SIZE;
