const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM: u16 = 0xFFFF;
const MAX_SIZE: u16 = 1 << 16 - 1;

const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;
// `max_size` in C is `low_cutoff` (4096) — used as upper bound for buffer size.
const MAX_BUFFER_SIZE: usize = LOW_CUTOFF;

pub struct RSet {
    pub buffer: Vec<u16>,
    size: usize,
}

impl RSet {
    pub fn new() -> Self {
        // Equivalent to rset_import(NULL, default_size).
        Self::import_internal(None, DEFAULT_SIZE)
    }

    pub fn free(&mut self) {
        // Rust handles deallocation automatically; clear to release memory.
        self.buffer.clear();
        self.buffer.shrink_to_fit();
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
        let cardinality = self.cardinality();
        if cardinality != comparison.cardinality() {
            return false;
        }
        let length_bytes = rset_length_for(cardinality);
        if length_bytes == 0 {
            return true;
        }
        let count_u16 = length_bytes / std::mem::size_of::<u16>();
        // Compare items starting at index 1 (skip the cardinality slot).
        for i in 0..count_u16 {
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
        let cur = result.buffer[0] as usize;
        result.buffer[0] = (MAX_CARDINALITY - cur) as u16;
        if result.is_bitset() {
            // Invert all bits in the bitset.
            for i in 0..MAX_BUFFER_SIZE {
                result.buffer[1 + i] = !result.buffer[1 + i];
            }
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

        // Otherwise treat as bitsets.
        if !result.grow_to(MAX_BUFFER_SIZE) {
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
        // Empty marker: cardinality=2, first item=MAX_ITEM (invalid state).
        self.buffer[0] = 2;
        self.buffer[1] = MAX_ITEM;
        true
    }

    pub fn fill(&mut self) -> bool {
        self.buffer[0] = 0;
        true
    }

    pub fn export(&self) -> Vec<u8> {
        // Convert the buffer (u16 items) into a byte vector matching the C
        // memory layout. Only the meaningful prefix (length()) is returned.
        let length_bytes = self.length();
        let mut out = Vec::with_capacity(length_bytes);
        let count_u16 = length_bytes / std::mem::size_of::<u16>();
        for i in 0..count_u16 {
            let v = self.buffer[i];
            out.push((v & 0xFF) as u8);
            out.push((v >> 8) as u8);
        }
        out
    }

    pub fn length(&self) -> usize {
        std::mem::size_of::<u16>() + rset_length_for(self.cardinality())
    }

    pub fn import(buffer: &[u8], length: usize) -> Self {
        // length here is the byte length (matches `unsigned length` in C
        // which counts bytes per memcpy(buffer, length)).
        Self::import_internal(Some(buffer), length)
    }

    pub fn copy(&self) -> Self {
        let exported = self.export();
        Self::import(&exported, self.length())
    }
}

// === Private helper functions ===

fn rset_length_for(cardinality: usize) -> usize {
    let mut c = cardinality;
    if c == 0 {
        c = 1;
    } else if c >= HIGH_CUTOFF {
        c = MAX_CARDINALITY - c;
    } else if c > LOW_CUTOFF {
        c = LOW_CUTOFF;
    }
    std::mem::size_of::<u16>() * c
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

    fn import_internal(src: Option<&[u8]>, length_or_size: usize) -> Self {
        // In C: `unsigned size = length ? length : 1; if (size > max_size) size = max_size;`
        // Here length_or_size is either the byte length when importing from a
        // buffer, or the requested logical size when constructing fresh.
        let mut size = if length_or_size == 0 { 1 } else { length_or_size };
        if size > MAX_BUFFER_SIZE {
            size = MAX_BUFFER_SIZE;
        }
        // Allocate (1 + size) u16 slots.
        let buffer = vec![0u16; 1 + size];

        let has_src = match src {
            Some(b) => !b.is_empty() && length_or_size != 0,
            None => false,
        };

        let mut set = RSet { buffer, size };

        if has_src {
            let bytes = src.unwrap();
            let copy_len = length_or_size.min(bytes.len());
            // Copy bytes into the u16 buffer using little-endian layout.
            let count_u16 = copy_len / 2;
            for i in 0..count_u16 {
                let lo = bytes[2 * i] as u16;
                let hi = bytes[2 * i + 1] as u16;
                set.buffer[i] = lo | (hi << 8);
            }
            // Handle odd trailing byte (matches memcpy behavior).
            if copy_len % 2 == 1 {
                let i = count_u16;
                let lo = bytes[2 * i] as u16;
                // Preserve high byte from the original zero buffer (which is 0).
                set.buffer[i] = lo;
            }
        } else {
            set.truncate();
        }
        set
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
        if size > MAX_BUFFER_SIZE {
            size = MAX_BUFFER_SIZE;
        }
        self.grow_to(size)
    }

    fn copy_to(&self, dest: &mut RSet) -> bool {
        if !dest.grow_to(self.size) {
            return false;
        }
        // Copy the meaningful bytes (length() bytes worth = length()/2 u16 slots).
        let length_bytes = self.length();
        let count_u16 = length_bytes / std::mem::size_of::<u16>();
        for i in 0..count_u16 {
            dest.buffer[i] = self.buffer[i];
        }
        true
    }

    fn convert_array_to_bitset(&mut self) -> bool {
        // Build a fresh bitset from the existing sorted array, then write
        // back into buffer[1..1+MAX_BUFFER_SIZE].
        let mut bitset = vec![0u16; MAX_BUFFER_SIZE];
        // The array contains exactly MAX_BUFFER_SIZE (=4096) items at this point.
        for i in 0..MAX_BUFFER_SIZE {
            let item = self.buffer[1 + i] as usize;
            bitset[item >> 4] |= 1u16 << (item & 0xF);
        }
        // Ensure the buffer is large enough.
        if !self.grow_to(MAX_BUFFER_SIZE) {
            return false;
        }
        for i in 0..MAX_BUFFER_SIZE {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        // Walk the bitset and emit the indices of clear bits, producing a
        // sorted array of items NOT in the set.
        let mut array = Vec::with_capacity(MAX_BUFFER_SIZE);
        let mut bit: u32 = 0;
        for i in 0..MAX_BUFFER_SIZE {
            let word = self.buffer[1 + i];
            for j in 0..16u32 {
                if (word & (1u16 << j)) == 0 {
                    array.push(bit as u16);
                }
                bit += 1;
            }
        }
        if !self.grow_to(MAX_BUFFER_SIZE) {
            return false;
        }
        for i in 0..MAX_BUFFER_SIZE {
            // The C code memcpy's max_size * sizeof(uint16_t) bytes, but the
            // array may have fewer elements than MAX_BUFFER_SIZE — leftover
            // memory is uninitialized (calloc init'd to 0). Mirror by filling
            // the rest with 0.
            self.buffer[1 + i] = if i < array.len() { array[i] } else { 0 };
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
            let mut found_or_break = false;
            while i <= cardinality {
                if self.buffer[i] < item {
                    i += 1;
                    continue;
                }
                if self.buffer[i] == item {
                    return true;
                }
                found_or_break = true;
                break;
            }
            // If we never broke, `i` ends as cardinality + 1, matching C.
            let _ = found_or_break;
        }
        if cardinality == self.size && !self.grow() {
            return false;
        }
        if cardinality + 1 > i {
            // memmove buffer[i..cardinality+1] -> buffer[i+1..cardinality+2]
            let count = cardinality + 1 - i;
            for k in (0..count).rev() {
                self.buffer[i + 1 + k] = self.buffer[i + k];
            }
        }
        self.buffer[i] = item;
        self.buffer[0] = self.buffer[0].wrapping_add(1);
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let offset = (item as usize >> 4) + 1;
        let bit = 1u16 << (item & 0xF);
        if (self.buffer[offset] & bit) == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        // The "inverted array" stores items NOT in the set. Adding an item
        // means removing it from this list. The cardinality stored in
        // buffer[0] is `max_cardinality - count_of_excluded_items`.
        let cardinality = MAX_CARDINALITY - self.buffer[0] as usize;
        if cardinality == 0 {
            return true;
        }
        // C special-cases: if buffer[cardinality] == item, just bump count.
        // This appears to be a (buggy but harmless) shortcut: if the last
        // excluded item equals `item`, treat it as removed without actually
        // shrinking the array. Mirror it for correctness with the C tests.
        if self.buffer[cardinality] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        // Linear scan in buffer[1..=cardinality] for the item.
        for i in 0..cardinality {
            let val = self.buffer[1 + i];
            if val < item {
                continue;
            }
            if val > item {
                break;
            }
            // val == item — remove it via memmove of (cardinality - i) u16's.
            // memmove(array + i, array + i + 1, (cardinality - i) * sizeof(u16))
            // Note: C copies cardinality-i items but only cardinality-1-i are
            // actually meaningful; we replicate exactly.
            let count = cardinality - i;
            for k in 0..count {
                // Source index relative to buffer: 1 + (i + 1 + k); dest: 1 + (i + k).
                let src_idx = 1 + i + 1 + k;
                let dst_idx = 1 + i + k;
                if src_idx < self.buffer.len() {
                    self.buffer[dst_idx] = self.buffer[src_idx];
                } else {
                    // Out of bounds in C would be UB; here, safely bail.
                    break;
                }
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
        // Binary search over buffer[1..=cardinality].
        let mut first: isize = 0;
        let mut last: isize = cardinality as isize - 1;
        let mut middle: isize = (first + last) / 2;
        while first <= last {
            let idx = 1 + middle as usize;
            let v = self.buffer[idx];
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
        let offset = (item as usize >> 4) + 1;
        (self.buffer[offset] & (1u16 << (item & 0xF))) != 0
    }

    fn intersection_array(a: &RSet, b: &RSet, result: &mut RSet) -> bool {
        let a_card = a.buffer[0] as usize;
        let b_card = b.buffer[0] as usize;
        let result_size = a_card.max(b_card);
        if !result.grow_to(result_size) {
            return false;
        }
        // Naive sorted intersection.
        let mut i = 0usize;
        let mut j = 0usize;
        let mut out = 0usize;
        while i < a_card && j < b_card {
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
        for i in 0..MAX_BUFFER_SIZE {
            let v = a.buffer[1 + i] & b.buffer[1 + i];
            result.buffer[1 + i] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
    }
}

// Suppress unused-constant warnings from constants kept to match the C source.
#[allow(dead_code)]
const _UNUSED_MAX_SIZE: u16 = MAX_SIZE;
