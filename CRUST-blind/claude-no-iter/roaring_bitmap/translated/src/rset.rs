const MAX_CARDINALITY: usize = 1 << 16;
const LOW_CUTOFF: usize = 1 << 12;
const HIGH_CUTOFF: usize = MAX_CARDINALITY - LOW_CUTOFF;
const MAX_ITEM:u16 = 0xFFFF;
const MAX_SIZE:u16 = 1 << 16 -1;

// Internal constants (matching C's max_size = low_cutoff and default_size).
const INTERNAL_MAX_SIZE: usize = LOW_CUTOFF;
const DEFAULT_SIZE: usize = 8;
const GROWTH_FACTOR: usize = 2;

pub struct RSet {
    buffer: Vec<u16>,
    size: usize,
}
impl RSet {
    pub fn new() -> Self {
        // Equivalent to rset_import(NULL, default_size): allocate buffer of
        // (1 + default_size) u16s, then truncate to the empty-marker state.
        let size = DEFAULT_SIZE;
        let mut s = RSet {
            buffer: vec![0u16; 1 + size],
            size,
        };
        s.truncate();
        s
    }
    pub fn free(&mut self) {
        // In Rust the buffer is freed automatically when the value is dropped.
        // For API parity we clear the buffer here so the set behaves as
        // released. After calling free() the set should not be used again.
        self.buffer.clear();
        self.size = 0;
    }
    pub fn cardinality(&self) -> usize {
        if self.is_full() {
            return MAX_CARDINALITY;
        }
        if self.is_empty_marker() {
            return 0;
        }
        self.buffer[0] as usize
    }
    pub fn add(&mut self, item: u16) -> bool {
        if self.is_full() {
            return true;
        }
        if self.is_empty_marker() {
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
        if self.is_empty_marker() {
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
        let len_bytes = Self::length_for(cardinality);
        if len_bytes == 0 {
            return true;
        }
        let n = len_bytes / 2;
        // Compare item region: buffer[1..1+n].
        for i in 0..n {
            // Defensive bounds check; both buffers should have at least n+1 items.
            if 1 + i >= self.buffer.len() || 1 + i >= comparison.buffer.len() {
                return false;
            }
            if self.buffer[1 + i] != comparison.buffer[1 + i] {
                return false;
            }
        }
        true
    }
    pub fn invert(&self, result: &mut RSet) -> bool {
        if self.is_empty_marker() {
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
        // Update cardinality to its complement. Both source and result are
        // non-full and non-empty, so buffer[0] is in [1, 65535] and the
        // complement is also in [1, 65535] which fits in u16 without wrap.
        let new_card = MAX_CARDINALITY - result.buffer[0] as usize;
        result.buffer[0] = new_card as u16;
        if result.is_bitset() {
            result.invert_bitset();
        }
        true
    }
    pub fn intersection(&self, other: &RSet, result: &mut RSet) -> bool {
        if self.is_empty_marker() || other.is_empty_marker() {
            // A & 0 => 0
            return result.truncate();
        }
        if self.is_full() {
            // A & U => A (here A is `other`)
            return other.copy_to(result);
        }
        if other.is_full() {
            return self.copy_to(result);
        }
        if self.is_array() && other.is_array() {
            return Self::intersection_array(self, other, result);
        }

        // TODO: convert both operands to bitsets if necessary (mirrors C).
        if !result.grow_to(INTERNAL_MAX_SIZE) {
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
        // Empty-marker representation: cardinality field = 2, first item = MAX_ITEM.
        // Since the array mode is sorted ascending, no second item could be
        // greater than MAX_ITEM, so this is a state that never appears in a
        // legitimate non-empty array.
        self.buffer[0] = 2;
        if self.buffer.len() < 2 {
            // Defensive: ensure at least 2 slots exist (cardinality + marker).
            self.buffer.resize(2, 0);
            if self.size < 1 {
                self.size = 1;
            }
        }
        self.buffer[1] = MAX_ITEM;
        true
    }
    pub fn fill(&mut self) -> bool {
        // Full-set representation: cardinality field = 0. rset_cardinality
        // distinguishes this from the empty-marker state.
        self.buffer[0] = 0;
        true
    }
    pub fn export(&self) -> Vec<u8> {
        let len_bytes = self.length();
        let mut out = Vec::with_capacity(len_bytes);
        let n_full_u16 = len_bytes / 2;
        for i in 0..n_full_u16 {
            let bytes = self.buffer[i].to_le_bytes();
            out.push(bytes[0]);
            out.push(bytes[1]);
        }
        if len_bytes % 2 == 1 {
            let bytes = self.buffer[n_full_u16].to_le_bytes();
            out.push(bytes[0]);
        }
        out
    }
    pub fn length(&self) -> usize {
        // sizeof(uint16_t) for the cardinality field + items length.
        2 + Self::length_for(self.cardinality())
    }
    pub fn import(buffer: &[u8], length: usize) -> Self {
        // Mirror rset_import: `length` is interpreted as a byte count for
        // the memcpy AND as the desired number of u16 item slots; clamped
        // to INTERNAL_MAX_SIZE. The allocated buffer holds (1 + size) u16s.
        let mut size = if length == 0 { 1 } else { length };
        if size > INTERNAL_MAX_SIZE {
            size = INTERNAL_MAX_SIZE;
        }
        let mut s = RSet {
            buffer: vec![0u16; 1 + size],
            size,
        };
        if !buffer.is_empty() && length > 0 {
            let buf_byte_capacity = s.buffer.len() * 2;
            let bytes_to_copy = length.min(buffer.len()).min(buf_byte_capacity);
            let n_full_u16 = bytes_to_copy / 2;
            for i in 0..n_full_u16 {
                s.buffer[i] = u16::from_le_bytes([buffer[i * 2], buffer[i * 2 + 1]]);
            }
            if bytes_to_copy % 2 == 1 {
                // Odd trailing byte: place it as the low byte of the next u16.
                let i = n_full_u16;
                s.buffer[i] = u16::from_le_bytes([buffer[i * 2], 0]);
            }
        } else {
            s.truncate();
        }
        s
    }
    pub fn copy(&self) -> Self {
        // Equivalent to rset_import(rset_export(set), rset_length(set)).
        let bytes = self.export();
        let len = bytes.len();
        Self::import(&bytes, len)
    }

    // ======================
    // Internal helpers
    // ======================

    fn is_empty_marker(&self) -> bool {
        // Note: C calls this rset_is_empty. We rename it to avoid clashing
        // with Rust's conventional `is_empty` semantics; but the meaning is
        // "the set has cardinality 0".
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

    fn length_for(cardinality: usize) -> usize {
        // Items length in bytes (excluding the cardinality slot itself).
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
        if size > INTERNAL_MAX_SIZE {
            size = INTERNAL_MAX_SIZE;
        }
        if size <= self.size {
            // Already at max; can't grow further.
            return self.size >= INTERNAL_MAX_SIZE;
        }
        self.grow_to(size)
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

    // ----- Array operations -----

    fn add_array(&mut self, item: u16) -> bool {
        let cardinality = self.buffer[0] as usize;
        let mut i: usize;
        // Fast path: appending an item greater than the current maximum.
        if cardinality > 0 && self.buffer[cardinality] < item {
            i = cardinality + 1;
        } else {
            // Linear scan for the insertion point. If item already exists,
            // it's a no-op (the operation is successful).
            i = 1;
            while i <= cardinality {
                if self.buffer[i] < item {
                    i += 1;
                    continue;
                }
                if self.buffer[i] == item {
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
            // Shift buffer[i..=cardinality] right by one slot to make room.
            for k in (i..=cardinality).rev() {
                self.buffer[k + 1] = self.buffer[k];
            }
        }
        self.buffer[i] = item;
        self.buffer[0] = self.buffer[0].wrapping_add(1);
        true
    }

    fn add_bitset(&mut self, item: u16) -> bool {
        let item_usize = item as usize;
        let offset = (item_usize >> 4) + 1;
        let bit = 1u16 << (item_usize & 0xF);
        if (self.buffer[offset] & bit) == 0 {
            self.buffer[offset] |= bit;
            self.buffer[0] = self.buffer[0].wrapping_add(1);
        }
        true
    }

    fn add_inverted_array(&mut self, item: u16) -> bool {
        // The "inverted array" stores items NOT in the set. Adding `item` to
        // the set means removing it from the inverted array (or no-op if it
        // wasn't there, i.e. it was already in the set).
        let cardinality = MAX_CARDINALITY - self.buffer[0] as usize;
        // Fast path: the last entry of the inverted array equals the item.
        // Incrementing buffer[0] effectively shrinks the inverted array by 1.
        if cardinality > 0 && self.buffer[cardinality] == item {
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        // Linear scan.
        let mut i = 0;
        while i < cardinality {
            let v = self.buffer[1 + i];
            if v < item {
                i += 1;
                continue;
            }
            if v > item {
                break;
            }
            // v == item: remove from the inverted array at position i. Shift
            // the elements at positions i+1..cardinality one slot to the left.
            // After the shift, the element at position cardinality-1 holds the
            // old last element, but once we increment buffer[0] the effective
            // array length becomes cardinality-1, so that position is no
            // longer part of the array.
            //
            // C uses (cardinality - i) elements which reads one past the end
            // of the inverted array; we use (cardinality - i - 1) to stay
            // within the live region, which is safe because the would-be
            // overwritten slot is no longer referenced.
            if cardinality > i + 1 {
                for k in 0..(cardinality - i - 1) {
                    self.buffer[1 + i + k] = self.buffer[1 + i + 1 + k];
                }
            }
            self.buffer[0] = self.buffer[0].wrapping_add(1);
            return true;
        }
        true
    }

    fn contains_array(&self, item: u16) -> bool {
        // Binary search on a sorted array. For inverted arrays, the array
        // length is max_cardinality - buffer[0]; for plain arrays it equals
        // buffer[0].
        let mut cardinality = self.buffer[0] as usize;
        if cardinality > HIGH_CUTOFF {
            cardinality = MAX_CARDINALITY - cardinality;
        }
        if cardinality == 0 {
            return false;
        }
        let mut first: i64 = 0;
        let mut last: i64 = cardinality as i64 - 1;
        let mut middle = (first + last) / 2;
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
        let item_usize = item as usize;
        let offset = (item_usize >> 4) + 1;
        let bit = 1u16 << (item_usize & 0xF);
        (self.buffer[offset] & bit) != 0
    }

    // ----- Mode conversions -----

    fn convert_array_to_bitset(&mut self) -> bool {
        // Called when cardinality == LOW_CUTOFF (4096). The buffer must
        // already have capacity for the bitset (1 + LOW_CUTOFF u16s).
        if self.size < LOW_CUTOFF && !self.grow_to(LOW_CUTOFF) {
            return false;
        }
        let cardinality = self.buffer[0] as usize;
        let mut bitset = vec![0u16; LOW_CUTOFF];
        for i in 0..cardinality {
            let item = self.buffer[1 + i];
            let idx = (item >> 4) as usize;
            bitset[idx] |= 1u16 << (item & 0xF);
        }
        for i in 0..LOW_CUTOFF {
            self.buffer[1 + i] = bitset[i];
        }
        true
    }

    fn convert_bitset_to_inverted_array(&mut self) -> bool {
        // Called when cardinality == HIGH_CUTOFF (61440). The bitset has
        // exactly LOW_CUTOFF (4096) unset bits, which become the entries of
        // the inverted array.
        let mut array: Vec<u16> = Vec::with_capacity(LOW_CUTOFF);
        for i in 0..LOW_CUTOFF {
            let word = self.buffer[1 + i];
            for j in 0..16 {
                if (word & (1u16 << j)) == 0 {
                    array.push((i * 16 + j) as u16);
                }
            }
        }
        // Pad to LOW_CUTOFF (matches calloc'd backing in C). The trailing
        // entries are never accessed by valid code paths since the inverted
        // array length is MAX_CARDINALITY - cardinality.
        while array.len() < LOW_CUTOFF {
            array.push(0);
        }
        for i in 0..LOW_CUTOFF {
            self.buffer[1 + i] = array[i];
        }
        true
    }

    // ----- Bitset helpers -----

    fn invert_bitset(&mut self) {
        for i in 0..LOW_CUTOFF {
            self.buffer[1 + i] = !self.buffer[1 + i];
        }
    }

    // ----- Intersection helpers -----

    fn intersection_array(a: &RSet, b: &RSet, result: &mut RSet) -> bool {
        let a_card = a.buffer[0] as usize;
        let b_card = b.buffer[0] as usize;
        let result_size = a_card.max(b_card);
        if !result.grow_to(result_size) {
            return false;
        }
        let mut i_a = 0;
        let mut i_b = 0;
        let mut count = 0;
        while i_a < a_card && i_b < b_card {
            let va = a.buffer[1 + i_a];
            let vb = b.buffer[1 + i_b];
            if va < vb {
                i_a += 1;
            } else if vb < va {
                i_b += 1;
            } else {
                result.buffer[1 + count] = va;
                count += 1;
                i_a += 1;
                i_b += 1;
            }
        }
        result.buffer[0] = count as u16;
        if count == 0 {
            result.truncate();
        }
        true
    }

    fn intersection_bitset(a: &RSet, b: &RSet, result: &mut RSet) -> usize {
        let mut cardinality = 0usize;
        for i in 0..LOW_CUTOFF {
            let v = a.buffer[1 + i] & b.buffer[1 + i];
            result.buffer[1 + i] = v;
            cardinality += v.count_ones() as usize;
        }
        cardinality
    }
}

// Silence dead-code warnings for the publicly declared but unused constants.
#[allow(dead_code)]
const _UNUSED_MAX_ITEM: u16 = MAX_ITEM;
#[allow(dead_code)]
const _UNUSED_MAX_SIZE: u16 = MAX_SIZE;
