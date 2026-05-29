const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE: u16 = 1 << 16 - 1;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
// In the C source, max_size == low_cutoff (the maximum buffer size in u16
// units). The MAX_SIZE constant above is a u16 with a value of 32768 due
// to the precedence of the shift expression and is unused here.
const RT_MAX_SIZE: usize = LOW_CUTOFF;

pub struct RSet {
    pub buffer: Vec<u16>,
    pub size: usize,
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

impl RSet {
    pub fn new() -> Self {
        Self::import(&[], DEFAULT_SIZE)
    }

    pub fn free(&mut self) {
        self.buffer.clear();
        self.size = 0;
    }

    fn is_empty_state(&self) -> bool {
        self.buffer.len() >= 2 && self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full_state(&self) -> bool {
        !self.buffer.is_empty() && self.buffer[0] == 0
    }

    fn is_array_state(&self) -> bool {
        (self.buffer[0] as usize) <= LOW_CUTOFF
    }

    fn is_bitset_state(&self) -> bool {
        let c = self.buffer[0] as usize;
        c > LOW_CUTOFF && c <= HIGH_CUTOFF
    }

    fn is_inverted_array_state(&self) -> bool {
        (self.buffer[0] as usize) > HIGH_CUTOFF
    }

    pub fn cardinality(&self) -> usize {
        if self.is_full_state() {
            return MAX_CARDINALITY;
        }
        if self.is_empty_state() {
            return 0;
        }
        self.buffer[0] as usize
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
        let mut out = Vec::with_capacity(len_bytes);
        let n_u16 = len_bytes / 2;
        for i in 0..n_u16 {
            out.extend_from_slice(&self.buffer[i].to_le_bytes());
        }
        out
    }

    pub fn length(&self) -> usize {
        2 + length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length > 0 { length } else { 1 };
        if size > RT_MAX_SIZE {
            size = RT_MAX_SIZE;
        }
        let mut buf = vec![0u16; 1 + size];
        if !buffer.is_empty() && length > 0 {
            let copy_bytes = length.min(buffer.len()).min(buf.len() * 2);
            let n_u16 = copy_bytes / 2;
            for i in 0..n_u16 {
                buf[i] = u16::from_le_bytes([buffer[2 * i], buffer[2 * i + 1]]);
            }
        } else {
            buf[0] = 2;
            buf[1] = MAX_ITEM;
        }
        Self { buffer: buf, size }
    }

    pub fn copy(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            size: self.size,
        }
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
        let mut new_size = self.size * GROWTH_FACTOR;
        if new_size > RT_MAX_SIZE {
            new_size = RT_MAX_SIZE;
        }
        self.grow_to(new_size)
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        let mut bitset = vec![0u16; RT_MAX_SIZE];
        for i in 0..RT_MAX_SIZE {
            let item = self.buffer[1 + i];
            bitset[(item >> 4) as usize] |= 1u16 << (item & 0xF);
        }
        for i in 0..RT_MAX_SIZE {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let mut array: Vec<u16> = Vec::with_capacity(RT_MAX_SIZE);
        let mut bit: u32 = 0;
        for i in 0..RT_MAX_SIZE {
            let word = self.buffer[1 + i];
            for j in 0..16u32 {
                if (word & (1u16 << j)) == 0 {
                    array.push(bit as u16);
                }
                bit += 1;
            }
        }
        for (k, &v) in array.iter().enumerate() {
            self.buffer[1 + k] = v;
        }
        true
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let i: usize;
        if cardinality > 0 && self.buffer[cardinality] < item {
            i = cardinality + 1;
        } else {
            let mut idx = 1;
            loop {
                if idx > cardinality {
                    break;
                }
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
        if cardinality == self.size {
            if !self.grow() {
                return false;
            }
        }
        if cardinality + 1 > i {
            self.buffer.copy_within(i..(cardinality + 1), i + 1);
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
        let inv_card = MAX_CARDINALITY - self.buffer[0] as usize;
        if inv_card > 0 && self.buffer[inv_card] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        for i in 0..inv_card {
            let v = self.buffer[1 + i];
            if v < item {
                continue;
            }
            if v > item {
                break;
            }
            // found - remove from inverted array by shifting left
            if i + 2 <= inv_card + 1 {
                self.buffer.copy_within((i + 2)..(inv_card + 1), i + 1);
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
            let v = self.buffer[1 + middle];
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

    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full_state() {
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
        if self.is_full_state() {
            return true;
        }
        if self.is_empty_state() {
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
        let length = length_for(cardinality);
        if length == 0 {
            return true;
        }
        let n_u16 = length / 2;
        for i in 0..n_u16 {
            if self.buffer[1 + i] != comparison.buffer[1 + i] {
                return false;
            }
        }
        true
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        let len_bytes = self.length();
        let n_u16 = len_bytes / 2;
        for i in 0..n_u16 {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn invert_bitset(&mut self) {
        for i in 0..RT_MAX_SIZE {
            self.buffer[1 + i] = !self.buffer[1 + i];
        }
    }

    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty_state() {
            return result.fill();
        }
        if self.is_full_state() {
            return result.truncate();
        }
        if !self.copy_to(result) {
            return false;
        }
        let new_card = MAX_CARDINALITY - result.buffer[0] as usize;
        result.buffer[0] = (new_card as u16).wrapping_add(0);
        // For values in (0, 65535], `new_card as u16` is fine.
        // Note: when self.cardinality is `max_cardinality` (full), we already
        // returned early via the is_full_state path.
        if result.is_bitset_state() {
            result.invert_bitset();
        }
        true
    }

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let result_size = (self.buffer[0] as usize).max(other.buffer[0] as usize);
        if !result.grow_to(result_size) {
            return false;
        }
        let a_card = self.buffer[0] as usize;
        let b_card = other.buffer[0] as usize;
        let mut count: usize = 0;
        let mut i = 0usize;
        let mut j = 0usize;
        while i < a_card && j < b_card {
            let av = self.buffer[1 + i];
            let bv = other.buffer[1 + j];
            if av < bv {
                i += 1;
            } else if bv < av {
                j += 1;
            } else {
                result.buffer[1 + count] = av;
                count += 1;
                i += 1;
                j += 1;
            }
        }
        result.buffer[0] = count as u16;
        if count == 0 {
            result.truncate();
        }
        true
    }

    fn intersection_bitset(&self, other: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..RT_MAX_SIZE {
            let v = self.buffer[1 + i] & other.buffer[1 + i];
            result.buffer[1 + i] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
    }

    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty_state() || other.is_empty_state() {
            return result.truncate();
        }
        if self.is_full_state() {
            return other.copy_to(result);
        }
        if other.is_full_state() {
            return self.copy_to(result);
        }
        if self.is_array_state() && other.is_array_state() {
            return self.intersection_array(other, result);
        }
        if !result.grow_to(RT_MAX_SIZE) {
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
}
