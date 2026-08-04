const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE: u16 = 1 << 16 - 1;

pub struct RSet {
    buffer: Vec<u16>,
    size: usize,
}

impl RSet {
    fn default_size() -> usize {
        8
    }

    fn max_size() -> usize {
        LOW_CUTOFF
    }

    fn is_empty(&self) -> bool {
        self.buffer[0] == 2 && self.buffer.get(1).copied().unwrap_or_default() == MAX_ITEM
    }

    fn is_full(&self) -> bool {
        self.buffer[0] == 0
    }

    fn is_bitset(&self) -> bool {
        let cardinality = self.buffer[0] as usize;
        cardinality > LOW_CUTOFF && cardinality <= HIGH_CUTOFF
    }

    fn is_array(&self) -> bool {
        (self.buffer[0] as usize) <= LOW_CUTOFF
    }

    fn is_inverted_array(&self) -> bool {
        (self.buffer[0] as usize) > HIGH_CUTOFF
    }

    fn logical_header(&self) -> usize {
        self.buffer[0] as usize
    }

    fn set_header(&mut self, value: usize) {
        self.buffer[0] = (value & 0xFFFF) as u16;
    }

    fn length_for(cardinality: usize) -> usize {
        let words = if cardinality == 0 {
            1
        } else if cardinality >= HIGH_CUTOFF {
            MAX_CARDINALITY - cardinality
        } else if cardinality > LOW_CUTOFF {
            LOW_CUTOFF
        } else {
            cardinality
        };
        std::mem::size_of::<u16>() * words
    }

    fn ensure_size(&mut self, size: usize) -> bool {
        if self.size >= size {
            return true;
        }
        self.buffer.resize(1 + size, 0);
        self.size = size;
        true
    }

    fn grow(&mut self) -> bool {
        let mut size = self.size.saturating_mul(2);
        if size > Self::max_size() {
            size = Self::max_size();
        }
        self.ensure_size(size)
    }

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.logical_header();
        let mut insert_at = cardinality + 1;

        if cardinality != 0 && self.buffer[cardinality] < item {
            insert_at = cardinality + 1;
        } else {
            for i in 1..=cardinality {
                if self.buffer[i] < item {
                    continue;
                }
                if self.buffer[i] == item {
                    return true;
                }
                insert_at = i;
                break;
            }
        }

        if cardinality == self.size && !self.grow() {
            return false;
        }

        if cardinality + 1 > insert_at {
            self.buffer.copy_within(insert_at..=cardinality, insert_at + 1);
        }
        self.buffer[insert_at] = item;
        self.set_header(cardinality + 1);
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = ((item as usize) >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        if self.buffer[offset] & bit == 0 {
            self.buffer[offset] |= bit;
            self.set_header(self.logical_header() + 1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        let cardinality = MAX_CARDINALITY - self.logical_header();
        if self.buffer[cardinality] == item {
            self.set_header(self.logical_header() + 1);
            return true;
        }

        for i in 0..cardinality {
            let value = self.buffer[i + 1];
            if value < item {
                continue;
            }
            if value > item {
                break;
            }
            self.buffer.copy_within((i + 2)..=(cardinality), i + 1);
            self.set_header(self.logical_header() + 1);
            return true;
        }

        true
    }

    fn contains_array(&self, item: u16) -> bool {
        let mut cardinality = self.logical_header();
        if cardinality > HIGH_CUTOFF {
            cardinality = MAX_CARDINALITY - cardinality;
        }
        self.buffer[1..=cardinality].binary_search(&item).is_ok()
    }

    fn contains_bitset(&self, item: u16) -> bool {
        self.buffer[((item as usize) >> 4) + 1] & (1u16 << (item & 0xF)) != 0
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        let array = self.buffer[1..=Self::max_size()].to_vec();
        let mut bitset = vec![0u16; Self::max_size()];
        for value in array {
            let idx = (value as usize) >> 4;
            bitset[idx] |= 1u16 << (value & 0xF);
        }
        self.buffer[1..=Self::max_size()].copy_from_slice(&bitset);
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        let bitset = self.buffer[1..=Self::max_size()].to_vec();
        let mut array = vec![0u16; Self::max_size()];
        let mut ptr = 0usize;
        let mut bit = 0u16;

        for word in bitset {
            for j in 0..16u16 {
                if word & (1u16 << j) == 0 {
                    array[ptr] = bit;
                    ptr += 1;
                }
                bit = bit.wrapping_add(1);
            }
        }

        self.buffer[1..=Self::max_size()].copy_from_slice(&array);
        true
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.ensure_size(self.size) {
            return false;
        }
        let words = self.length() / std::mem::size_of::<u16>();
        dest.buffer[..words].copy_from_slice(&self.buffer[..words]);
        true
    }

    fn invert_bitset(&mut self) {
        for word in &mut self.buffer[1..=Self::max_size()] {
            *word = !*word;
        }
    }

    fn intersection_array(&self, other: &RSet, result: &mut RSet) -> bool {
        let result_size = self.logical_header().max(other.logical_header());
        if !result.ensure_size(result_size) {
            return false;
        }

        let a = &self.buffer[1..=self.logical_header()];
        let b = &other.buffer[1..=other.logical_header()];
        let mut i = 0usize;
        let mut j = 0usize;
        let mut count = 0usize;

        while i < a.len() && j < b.len() {
            if a[i] < b[j] {
                i += 1;
            } else if b[j] < a[i] {
                j += 1;
            } else {
                result.buffer[count + 1] = a[i];
                count += 1;
                i += 1;
                j += 1;
            }
        }

        result.set_header(count);
        if count == 0 {
            result.truncate();
        }
        true
    }

    fn intersection_bitset(&self, other: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..Self::max_size() {
            let a = self.buffer.get(i + 1).copied().unwrap_or(0);
            let b = other.buffer.get(i + 1).copied().unwrap_or(0);
            let value = a & b;
            result.buffer[i + 1] = value;
            cardinality += value.count_ones() as usize;
        }
        cardinality
    }

    pub fn new() -> Self {
        let mut set = Self {
            buffer: vec![0; 1 + Self::default_size()],
            size: Self::default_size(),
        };
        set.truncate();
        set
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
        self.logical_header()
    }

    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }

        if self.is_empty() {
            self.set_header(0);
        }

        let cardinality = self.logical_header();
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

        let words = Self::length_for(cardinality) / std::mem::size_of::<u16>();
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
        result.set_header(MAX_CARDINALITY - result.logical_header());
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
        } else if other.is_full() {
            return self.copy_to(result);
        }
        if self.is_array() && other.is_array() {
            return self.intersection_array(other, result);
        }

        if !result.ensure_size(Self::max_size()) {
            return false;
        }

        let cardinality = self.intersection_bitset(other, result);
        if cardinality == 0 {
            return result.truncate();
        }
        if cardinality == MAX_CARDINALITY {
            return result.fill();
        }

        result.set_header(cardinality);
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
        let words = self.length() / std::mem::size_of::<u16>();
        self.buffer[..words]
            .iter()
            .flat_map(|word| word.to_ne_bytes())
            .collect()
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + Self::length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        let size = if length == 0 { 1 } else { length.min(Self::max_size()) };
        let mut raw = vec![0u8; (1 + size) * std::mem::size_of::<u16>()];
        let copy_len = length.min(buffer.len()).min(raw.len());
        raw[..copy_len].copy_from_slice(&buffer[..copy_len]);

        let mut words = Vec::with_capacity(1 + size);
        for chunk in raw.chunks_exact(2) {
            words.push(u16::from_ne_bytes([chunk[0], chunk[1]]));
        }

        let mut set = Self { buffer: words, size };
        if buffer.is_empty() || length == 0 {
            set.truncate();
        }
        set
    }

    pub fn copy(&self) -> Self {
        Self::import(&self.export(), self.length())
    }
}

#[allow(dead_code)]
const _: u16 = MAX_SIZE;
