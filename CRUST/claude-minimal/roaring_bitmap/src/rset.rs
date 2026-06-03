const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE_BUF: usize = LOW_CUTOFF;
const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;

pub struct RSet {
    pub buffer: Vec<u16>,
    pub size: usize,
}

impl RSet {
    fn is_empty_set(&self) -> bool {
        self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full_set(&self) -> bool {
        self.buffer[0] == 0
    }

    fn is_bitset_state(&self) -> bool {
        let c = self.buffer[0] as usize;
        c > LOW_CUTOFF && c <= HIGH_CUTOFF
    }

    fn is_array_state(&self) -> bool {
        let c = self.buffer[0] as usize;
        c <= LOW_CUTOFF
    }

    fn is_inverted_array_state(&self) -> bool {
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

    fn grow_to(&mut self, size: usize) -> bool {
        if self.size >= size {
            return true;
        }
        let size = size.min(MAX_SIZE_BUF);
        if self.size >= size {
            return true;
        }
        self.buffer.resize(1 + size, 0);
        self.size = size;
        true
    }

    fn grow(&mut self) -> bool {
        let mut size = self.size * GROWTH_FACTOR;
        if size > MAX_SIZE_BUF {
            size = MAX_SIZE_BUF;
        }
        self.grow_to(size)
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let bytes = self.length();
        let u16s = bytes / 2;
        for i in 0..u16s {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        if !self.grow_to(MAX_SIZE_BUF) {
            return false;
        }
        let cardinality = self.buffer[0] as usize;
        let array: Vec<u16> = self.buffer[1..=cardinality].to_vec();
        for i in 1..=MAX_SIZE_BUF {
            self.buffer[i] = 0;
        }
        for &item in &array {
            self.buffer[(item >> 4) as usize + 1] |= 1u16 << (item & 0xF);
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array: Vec<u16> = Vec::with_capacity(LOW_CUTOFF);
        for i in 0..MAX_SIZE_BUF {
            let val = self.buffer[i + 1];
            for j in 0..16u32 {
                if (val & (1u16 << j)) == 0 {
                    array.push((i as u32 * 16 + j) as u16);
                }
            }
        }
        for (idx, &item) in array.iter().enumerate() {
            self.buffer[idx + 1] = item;
        }
        true
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let mut i: usize;
        if cardinality > 0 && self.buffer[cardinality] < item {
            i = cardinality + 1;
        } else {
            i = cardinality + 1;
            for j in 1..=cardinality {
                if self.buffer[j] < item {
                    continue;
                }
                if self.buffer[j] == item {
                    return true;
                }
                i = j;
                break;
            }
        }
        if cardinality == self.size && !self.grow() {
            return false;
        }
        if cardinality + 1 > i {
            // Shift buffer[i..=cardinality] to buffer[i+1..=cardinality+1]
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
            self.buffer[0] += 1;
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let array_size = MAX_CARDINALITY - self.buffer[0] as usize;
        if array_size > 0 && self.buffer[array_size] == item {
            // Use wrapping add: when cardinality goes from 65535 to 65536
            // (set becomes full), buffer[0] wraps to 0 which is the "full"
            // sentinel.
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        for i in 0..array_size {
            let v = self.buffer[i + 1];
            if v < item {
                continue;
            }
            if v > item {
                break;
            }
            // v == item, remove it from the missing array
            for k in i..(array_size - 1) {
                self.buffer[k + 1] = self.buffer[k + 2];
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
            let v = self.buffer[middle + 1];
            if v == item {
                return true;
            }
            if v < item {
                first = middle as i64 + 1;
            } else {
                last = middle as i64 - 1;
            }
        }
        false
    }

    fn contains_bitset(&self, item: u16) -> bool {
        (self.buffer[(item >> 4) as usize + 1] & (1u16 << (item & 0xF))) != 0
    }

    pub fn new() -> Self {
        Self::import(&[], DEFAULT_SIZE)
    }

    pub fn free(&mut self) {
        self.buffer.clear();
        self.buffer.shrink_to_fit();
        self.size = 0;
    }

    pub fn cardinality(&self) -> usize {
        if self.is_full_set() {
            return MAX_CARDINALITY;
        }
        if self.is_empty_set() {
            return 0;
        }
        self.buffer[0] as usize
    }

    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full_set() {
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
            self.add_array(item)
        } else if cardinality >= HIGH_CUTOFF {
            self.add_inverted_array(item)
        } else {
            self.add_bitset(item)
        }
    }

    pub fn contains(&self, item: u16) -> bool {
        if self.is_full_set() {
            return true;
        }
        if self.is_empty_set() {
            return false;
        }
        if self.is_array_state() {
            return self.contains_array(item);
        }
        if self.is_inverted_array_state() {
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
        let length_u16 = length_bytes / 2;
        for i in 0..length_u16 {
            if self.buffer[i + 1] != comparison.buffer[i + 1] {
                return false;
            }
        }
        true
    }

    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty_set() {
            return result.fill();
        }
        if self.is_full_set() {
            return result.truncate();
        }
        if !self.copy_to(result) {
            return false;
        }
        let new_card = MAX_CARDINALITY - result.buffer[0] as usize;
        // new_card cannot equal MAX_CARDINALITY here because the original set
        // wasn't empty (cardinality > 0). But it could theoretically equal 0
        // only if original was full, which we handled above.
        result.buffer[0] = new_card as u16;
        if result.is_bitset_state() {
            for i in 0..MAX_SIZE_BUF {
                result.buffer[i + 1] = !result.buffer[i + 1];
            }
        }
        true
    }

    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty_set() || other.is_empty_set() {
            return result.truncate();
        }
        if self.is_full_set() {
            return other.copy_to(result);
        }
        if other.is_full_set() {
            return self.copy_to(result);
        }
        if self.is_array_state() && other.is_array_state() {
            return self.intersection_array(other, result);
        }
        // Bitset intersection (assumes both are bitsets; matches the C version
        // which has TODOs for converting array/inverted-array operands)
        if !result.grow_to(MAX_SIZE_BUF) {
            return false;
        }
        let mut cardinality: usize = 0;
        for i in 0..MAX_SIZE_BUF {
            let v = self.buffer[i + 1] & other.buffer[i + 1];
            result.buffer[i + 1] = v;
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

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let result_size = (self.buffer[0]).max(other.buffer[0]) as usize;
        if !result.grow_to(result_size) {
            return false;
        }
        let card_a = self.buffer[0] as usize;
        let card_b = other.buffer[0] as usize;
        let mut i = 0;
        let mut j = 0;
        let mut k = 0;
        while i < card_a && j < card_b {
            let av = self.buffer[i + 1];
            let bv = other.buffer[j + 1];
            if av < bv {
                i += 1;
            } else if bv < av {
                j += 1;
            } else {
                result.buffer[k + 1] = av;
                k += 1;
                i += 1;
                j += 1;
            }
        }
        if k == 0 {
            result.truncate();
        } else {
            result.buffer[0] = k as u16;
        }
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
        let length_bytes = self.length();
        let mut result = Vec::with_capacity(length_bytes);
        let length_u16 = length_bytes / 2;
        for i in 0..length_u16 {
            let bytes = self.buffer[i].to_ne_bytes();
            result.push(bytes[0]);
            result.push(bytes[1]);
        }
        result
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + Self::length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        // Mirrors the C `rset_import`: `length` is reused both as a desired item
        // count for sizing the underlying buffer and as a byte count for copying
        // the input data. Over-allocation is fine.
        let mut size = if length > 0 { length } else { 1 };
        if size > MAX_SIZE_BUF {
            size = MAX_SIZE_BUF;
        }
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            let bytes_avail = buffer.len().min(length);
            let u16s = (bytes_avail / 2).min(buf.len());
            for i in 0..u16s {
                buf[i] = u16::from_ne_bytes([buffer[i * 2], buffer[i * 2 + 1]]);
            }
        } else {
            // Default to truncated state
            buf[0] = 2;
            buf[1] = MAX_ITEM;
        }
        RSet {
            buffer: buf,
            size,
        }
    }

    pub fn copy(&self) -> Self {
        let exported = self.export();
        Self::import(&exported, self.length())
    }
}
