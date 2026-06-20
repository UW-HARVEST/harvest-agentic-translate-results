const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE: u16 = 1 << 16 - 1;
const DEFAULT_SIZE: usize = 8;

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
        self.size = 0;
    }

    pub fn cardinality(&self) -> usize {
        if self.is_full() {
            MAX_CARDINALITY
        } else if self.is_empty() {
            0
        } else {
            self.raw_cardinality()
        }
    }

    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }

        if self.is_empty() {
            self.buffer[0] = 0;
        }

        let cardinality = self.raw_cardinality();
        if cardinality == LOW_CUTOFF {
            if self.contains_array(item, false) {
                return true;
            }
            self.convert_array_to_bitset();
        } else if cardinality == HIGH_CUTOFF {
            if self.contains_bitset(item) {
                return true;
            }
            self.convert_bitset_to_inverted_array();
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
        if self.is_empty() {
            return false;
        }
        if self.is_array() {
            return self.contains_array(item, false);
        }
        if self.is_inverted_array() {
            return !self.contains_array(item, true);
        }
        self.contains_bitset(item)
    }

    pub fn equals(&self, comparison: &RSet) -> bool {
        let cardinality = self.cardinality();
        if cardinality != comparison.cardinality() {
            return false;
        }

        if self.is_empty() && comparison.is_empty() {
            return self.buffer.get(1) == comparison.buffer.get(1);
        }
        if self.is_full() && comparison.is_full() {
            return true;
        }
        if self.is_array() && comparison.is_array() {
            return self.array_slice(false) == comparison.array_slice(false);
        }
        if self.is_bitset() && comparison.is_bitset() {
            return self.bitset_slice() == comparison.bitset_slice();
        }
        if self.is_inverted_array() && comparison.is_inverted_array() {
            return self.array_slice(true) == comparison.array_slice(true);
        }

        self.to_bitset_words() == comparison.to_bitset_words()
    }

    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty() {
            return result.fill();
        }
        if self.is_full() {
            return result.truncate();
        }
        result.copy_from(self);
        result.buffer[0] = (MAX_CARDINALITY - result.raw_cardinality()) as u16;
        if result.is_bitset() {
            for word in &mut result.buffer[1..=LOW_CUTOFF] {
                *word = !*word;
            }
        }
        true
    }

    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty() || other.is_empty() {
            return result.truncate();
        }
        if self.is_full() {
            result.copy_from(other);
            return true;
        }
        if other.is_full() {
            result.copy_from(self);
            return true;
        }
        if self.is_array() && other.is_array() {
            let a = self.array_slice(false);
            let b = other.array_slice(false);
            let mut out = Vec::with_capacity(a.len().max(b.len()));
            let mut i = 0;
            let mut j = 0;
            while i < a.len() && j < b.len() {
                match a[i].cmp(&b[j]) {
                    std::cmp::Ordering::Less => i += 1,
                    std::cmp::Ordering::Greater => j += 1,
                    std::cmp::Ordering::Equal => {
                        out.push(a[i]);
                        i += 1;
                        j += 1;
                    }
                }
            }
            result.load_from_sorted_items(&out);
            return true;
        }

        let a_bits = self.to_bitset_words();
        let b_bits = other.to_bitset_words();
        let mut result_bits = vec![0u16; LOW_CUTOFF];
        let mut cardinality = 0usize;
        for i in 0..LOW_CUTOFF {
            let word = a_bits[i] & b_bits[i];
            result_bits[i] = word;
            cardinality += word.count_ones() as usize;
        }
        result.load_from_bitset_words(&result_bits, cardinality);
        true
    }

    pub fn truncate(&mut self) -> bool {
        self.ensure_size(1);
        self.buffer[0] = 2;
        self.buffer[1] = MAX_ITEM;
        true
    }

    pub fn fill(&mut self) -> bool {
        self.ensure_size(1);
        self.buffer[0] = 0;
        true
    }

    pub fn export(&self) -> Vec<u8> {
        let word_len = self.length() / std::mem::size_of::<u16>();
        let mut bytes = Vec::with_capacity(self.length());
        for word in &self.buffer[..word_len] {
            bytes.extend_from_slice(&word.to_ne_bytes());
        }
        bytes
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + rset_length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let mut size = if length != 0 { length } else { 1 };
        if size > LOW_CUTOFF {
            size = LOW_CUTOFF;
        }

        let mut set = Self {
            buffer: vec![0; 1 + size],
            size,
        };

        if !buffer.is_empty() && length != 0 {
            let copy_len = length.min(buffer.len()).min(set.buffer.len() * 2);
            let mut raw = vec![0u8; set.buffer.len() * 2];
            raw[..copy_len].copy_from_slice(&buffer[..copy_len]);
            for (slot, chunk) in set.buffer.iter_mut().zip(raw.chunks_exact(2)) {
                *slot = u16::from_ne_bytes([chunk[0], chunk[1]]);
            }
        } else {
            set.truncate();
        }

        set
    }

    pub fn copy(&self) -> Self {
        Self::import(&self.export(), self.length())
    }

    fn is_empty(&self) -> bool {
        self.buffer.len() >= 2 && self.buffer[0] == 2 && self.buffer[1] == MAX_ITEM
    }

    fn is_full(&self) -> bool {
        self.buffer.first().copied().unwrap_or_default() == 0 && !self.is_empty()
    }

    fn is_bitset(&self) -> bool {
        let cardinality = self.raw_cardinality();
        cardinality > LOW_CUTOFF && cardinality <= HIGH_CUTOFF
    }

    fn is_array(&self) -> bool {
        self.raw_cardinality() <= LOW_CUTOFF
    }

    fn is_inverted_array(&self) -> bool {
        self.raw_cardinality() > HIGH_CUTOFF
    }

    fn raw_cardinality(&self) -> usize {
        self.buffer.first().copied().unwrap_or_default() as usize
    }

    fn ensure_size(&mut self, size: usize) {
        if self.size < size {
            self.size = size;
        }
        let required = 1 + self.size;
        if self.buffer.len() < required {
            self.buffer.resize(required, 0);
        }
    }

    fn copy_from(&mut self, other: &RSet) {
        self.ensure_size(other.size);
        let word_len = other.length() / std::mem::size_of::<u16>();
        self.buffer[..word_len].copy_from_slice(&other.buffer[..word_len]);
    }

    fn contains_array(&self, item: u16, inverted: bool) -> bool {
        let len = if inverted {
            MAX_CARDINALITY - self.raw_cardinality()
        } else {
            self.raw_cardinality()
        };
        self.buffer[1..1 + len].binary_search(&item).is_ok()
    }

    fn contains_bitset(&self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        self.buffer[offset] & bit != 0
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.raw_cardinality();
        let pos = match self.buffer[1..1 + cardinality].binary_search(&item) {
            Ok(_) => return true,
            Err(pos) => pos + 1,
        };
        if cardinality == self.size {
            self.grow();
        }
        if pos <= cardinality {
            self.buffer.copy_within(pos..=cardinality, pos + 1);
        }
        self.buffer[pos] = item;
        self.buffer[0] = (cardinality + 1) as u16;
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        if self.buffer[offset] & bit == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let excluded = MAX_CARDINALITY - self.raw_cardinality();
        let array = &mut self.buffer[1..1 + excluded];
        match array.binary_search(&item) {
            Ok(pos) => {
                if pos + 1 < excluded {
                    self.buffer.copy_within((pos + 2)..(1 + excluded), pos + 1);
                }
                self.buffer[0] = self.buffer[0].wrapping_add(1);
            }
            Err(_) => {}
        }
        true
    }

    fn grow(&mut self) {
        let mut size = self.size.saturating_mul(2);
        if size > LOW_CUTOFF {
            size = LOW_CUTOFF;
        }
        self.ensure_size(size);
    }

    fn convert_array_to_bitset(&mut self) {
        let items = self.array_slice(false).to_vec();
        self.ensure_size(LOW_CUTOFF);
        self.buffer[1..=LOW_CUTOFF].fill(0);
        for item in items {
            let offset = (item as usize >> 4) + 1;
            let bit = 1u16 << (item & 0xF);
            self.buffer[offset] |= bit;
        }
    }

    fn convert_bitset_to_inverted_array(&mut self) {
        let mut excluded = Vec::with_capacity(LOW_CUTOFF);
        for (word_index, word) in self.bitset_slice().iter().copied().enumerate() {
            for bit in 0..16 {
                if word & (1u16 << bit) == 0 {
                    excluded.push((word_index * 16 + bit) as u16);
                }
            }
        }
        self.ensure_size(LOW_CUTOFF);
        self.buffer[1..=LOW_CUTOFF].copy_from_slice(&excluded);
    }

    fn array_slice(&self, inverted: bool) -> &[u16] {
        let len = if inverted {
            MAX_CARDINALITY - self.raw_cardinality()
        } else {
            self.raw_cardinality()
        };
        &self.buffer[1..1 + len]
    }

    fn bitset_slice(&self) -> &[u16] {
        &self.buffer[1..=LOW_CUTOFF]
    }

    fn to_bitset_words(&self) -> Vec<u16> {
        if self.is_empty() {
            return vec![0; LOW_CUTOFF];
        }
        if self.is_full() {
            return vec![u16::MAX; LOW_CUTOFF];
        }
        if self.is_bitset() {
            return self.bitset_slice().to_vec();
        }

        let mut words = if self.is_inverted_array() {
            vec![u16::MAX; LOW_CUTOFF]
        } else {
            vec![0; LOW_CUTOFF]
        };

        if self.is_inverted_array() {
            for &item in self.array_slice(true) {
                let offset = item as usize >> 4;
                let bit = 1u16 << (item & 0xF);
                words[offset] &= !bit;
            }
        } else {
            for &item in self.array_slice(false) {
                let offset = item as usize >> 4;
                let bit = 1u16 << (item & 0xF);
                words[offset] |= bit;
            }
        }

        words
    }

    fn load_from_sorted_items(&mut self, items: &[u16]) {
        if items.is_empty() {
            self.truncate();
            return;
        }
        let cardinality = items.len();
        if cardinality == MAX_CARDINALITY {
            self.fill();
            return;
        }
        if cardinality <= LOW_CUTOFF {
            self.ensure_size(cardinality.max(1));
            self.buffer[0] = cardinality as u16;
            self.buffer[1..1 + cardinality].copy_from_slice(items);
            return;
        }

        let mut words = vec![0u16; LOW_CUTOFF];
        for &item in items {
            let offset = item as usize >> 4;
            let bit = 1u16 << (item & 0xF);
            words[offset] |= bit;
        }
        self.load_from_bitset_words(&words, cardinality);
    }

    fn load_from_bitset_words(&mut self, words: &[u16], cardinality: usize) {
        if cardinality == 0 {
            self.truncate();
            return;
        }
        if cardinality == MAX_CARDINALITY {
            self.fill();
            return;
        }
        if cardinality <= LOW_CUTOFF {
            let mut items = Vec::with_capacity(cardinality);
            for (word_index, word) in words.iter().copied().enumerate() {
                for bit in 0..16 {
                    if word & (1u16 << bit) != 0 {
                        items.push((word_index * 16 + bit) as u16);
                    }
                }
            }
            self.load_from_sorted_items(&items);
            return;
        }
        if cardinality > HIGH_CUTOFF {
            let excluded_len = MAX_CARDINALITY - cardinality;
            self.ensure_size(excluded_len.max(1));
            self.buffer[0] = cardinality as u16;
            let mut pos = 1;
            for (word_index, word) in words.iter().copied().enumerate() {
                for bit in 0..16 {
                    if word & (1u16 << bit) == 0 {
                        self.buffer[pos] = (word_index * 16 + bit) as u16;
                        pos += 1;
                    }
                }
            }
            return;
        }

        self.ensure_size(LOW_CUTOFF);
        self.buffer[0] = cardinality as u16;
        self.buffer[1..=LOW_CUTOFF].copy_from_slice(words);
    }
}

fn rset_length_for(mut cardinality: usize) -> usize {
    if cardinality == 0 {
        cardinality = 1;
    } else if cardinality >= HIGH_CUTOFF {
        cardinality = MAX_CARDINALITY - cardinality;
    } else if cardinality > LOW_CUTOFF {
        cardinality = LOW_CUTOFF;
    }
    std::mem::size_of::<u16>() * cardinality
}

#[allow(dead_code)]
const _: u16 = MAX_SIZE;
