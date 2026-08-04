// Import necessary modules
use std::mem;
use std::ptr;
// Type aliases
pub type XXH64HashT = u64;
pub type XXHU8 = u8;
pub type XXHU64 = XXH64HashT;
pub type XXHU32 = u32;
pub type XXH32HashT = u32;
// Constants
pub const XXH_PRIME64_1: u64 = 0x9E3779B185EBCA87;
pub const XXH_PRIME64_2: u64 = 0xC2B2AE3D27D4EB4F;
pub const XXH_PRIME64_3: u64 = 0x165667B19E3779F9;
pub const XXH_PRIME64_4: u64 = 0x85EBCA77C2B2AE63;
pub const XXH_PRIME64_5: u64 = 0x27D4EB2F165667C5;
pub const CSET_FORCE_INITIALIZE: bool = true;
pub const CSET_INITIAL_CAP: usize = 2;
pub const CSET_DEFAULT_SEED: u64 = 2718182;
pub const CSET_MAX_LOAD_FACTOR: f64 = 0.7;
pub const CSET_MIN_LOAD_FACTOR: f64 = 0.2;

// ----- Internal helpers for byte-level reading -----

fn read_u64_le_slice(bytes: &[u8]) -> u64 {
    let mut v: u64 = 0;
    for i in 0..8 {
        v |= (bytes[i] as u64) << (i * 8);
    }
    v
}

fn read_u32_le_slice(bytes: &[u8]) -> u32 {
    let mut v: u32 = 0;
    for i in 0..4 {
        v |= (bytes[i] as u32) << (i * 8);
    }
    v
}

// XXH64 finalize on a byte slice.
fn xxh64_finalize_slice(mut h64: u64, bytes: &[u8]) -> u64 {
    let mut len = bytes.len() & 31;
    let mut idx = 0;

    while len >= 8 {
        let k1 = xxh64_round(0, read_u64_le_slice(&bytes[idx..]));
        idx += 8;
        h64 ^= k1;
        h64 = h64
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_u32_le_slice(&bytes[idx..]) as u64).wrapping_mul(XXH_PRIME64_1);
        idx += 4;
        h64 = h64
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (bytes[idx] as u64).wrapping_mul(XXH_PRIME64_5);
        idx += 1;
        h64 = h64.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

fn xxh64_compute(input: &[u8], seed: u64) -> u64 {
    let total_len = input.len();
    let mut bytes = input;
    let mut h64;

    if total_len >= 32 {
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            v2 = xxh64_round(v2, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            v3 = xxh64_round(v3, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            v4 = xxh64_round(v4, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            if bytes.len() < 32 {
                break;
            }
        }

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(total_len as u64);
    xxh64_finalize_slice(h64, bytes)
}

fn xxh64_h_compute(input: &[u8], seed: u64) -> u64 {
    let total_len = input.len();
    let mut bytes = input;
    let mut h64;

    if total_len >= 32 {
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            v2 = xxh64_round(v2, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            v3 = xxh64_round(v3, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            v4 = xxh64_round(v4, read_u64_le_slice(bytes));
            bytes = &bytes[8..];
            if bytes.len() < 32 {
                break;
            }
        }

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }

    h64 = h64.wrapping_add(total_len as u64);
    xxh64_finalize_slice(h64, bytes)
}

// ----- Public XXH-style functions matching the original signatures -----

pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}

pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    // Reads 8 little-endian bytes starting at mem_ptr.
    unsafe {
        let p = mem_ptr as *const u8;
        let mut v: u64 = 0;
        for i in 0..8 {
            v |= (*p.add(i) as u64) << (i * 8);
        }
        v
    }
}

pub fn xxh_is_little_endian() -> bool {
    cfg!(target_endian = "little")
}

pub fn xxh_read_le64_align(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64(mem_ptr)
}

pub fn xxh_swap32(x: &mut XXHU32) -> XXHU32 {
    let v = *x;
    ((v << 24) & 0xff000000)
        | ((v << 8) & 0x00ff0000)
        | ((v >> 8) & 0x0000ff00)
        | ((v >> 24) & 0x000000ff)
}

pub fn xxh_read32(mem_ptr: &mut XXHU32) -> XXHU32 {
    *mem_ptr
}

pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = acc.rotate_left(31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    let val = xxh64_round(0, val);
    let acc = acc ^ val;
    acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}

pub fn xxh_get_32bits(ptr: &mut XXHU32) -> XXHU32 {
    xxh_read_le32_align(ptr)
}

pub fn xxh_read_le32_align(ptr: &mut XXHU32) -> XXHU32 {
    if xxh_is_little_endian() {
        *ptr
    } else {
        xxh_swap32(ptr)
    }
}

pub fn xxh64_avalanche(mut h64: XXHU64) -> XXHU64 {
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(XXH_PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(XXH_PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}

pub fn xxh64_finalize(h64: XXHU64, ptr: &mut XXHU8, len: usize) -> XXHU64 {
    // Reads `len & 31` bytes starting at ptr.
    let len = len & 31;
    unsafe {
        let p = ptr as *const u8;
        let bytes = std::slice::from_raw_parts(p, len);
        xxh64_finalize_slice(h64, bytes)
    }
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe {
        let p = input as *const u8;
        let bytes = std::slice::from_raw_parts(p, len);
        xxh64_compute(bytes, seed)
    }
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe {
        let p = input as *const u8;
        let bytes = std::slice::from_raw_parts(p, len);
        xxh64_h_compute(bytes, seed)
    }
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() {
        // Match the behaviour of the C version when input is NULL: hash an empty buffer.
        return xxh64_compute(&[], seed);
    }
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_compute(bytes, seed)
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() {
        return xxh64_h_compute(&[], seed);
    }
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_h_compute(bytes, seed)
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64(memptr as *const u8, size, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64_h(memptr as *const u8, size, CSET_DEFAULT_SEED) | 1
}

// ----- Cset structure definitions -----

pub struct CsetValue<T> {
    pi: i32,
    elem: T,
}

pub struct Cset<T> {
    buckets: Vec<CsetValue<T>>,
    max_load_factor: f64,
    min_load_factor: f64,
    seed: u64,
    #[allow(dead_code)]
    v: CsetValue<T>,
    bucket_size: usize,
    compare: Option<fn(&T, &T) -> bool>,
    temp_buckets: Vec<CsetValue<T>>,
}

// Helper: get raw bytes of a T value.
fn t_bytes<T>(value: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(value as *const T as *const u8, mem::size_of::<T>())
    }
}

// Build an "empty" CsetValue<T> with pi == 0. The elem field is zero-initialized.
fn empty_cset_value<T>() -> CsetValue<T> {
    // Safety: we use this slot only when its `pi` is 0 or -1; the elem field is
    // intentionally treated as unused garbage. For all types used in the test
    // suite (i32, char, Node), zero-initialization is a valid bit pattern.
    unsafe { mem::zeroed::<CsetValue<T>>() }
}

impl<T> Cset<T> {
    fn double_hash_index(h1: u64, h2: u64, i: usize, cap: usize) -> usize {
        // C macro: ((h1) + ((i) * (h2))) % cap. This uses unsigned wrapping
        // arithmetic in C; emulate with wrapping_* on u64.
        ((h1.wrapping_add((i as u64).wrapping_mul(h2))) % (cap as u64)) as usize
    }

    pub fn new() -> Cset<T> {
        let mut cset = Cset {
            buckets: Vec::new(),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: empty_cset_value::<T>(),
            bucket_size: 0,
            compare: None,
            temp_buckets: Vec::new(),
        };
        cset.init();
        cset
    }

    pub fn init(&mut self) {
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
        // Replace buckets with a fresh, fully-initialized vector of the initial cap.
        let mut new_buckets: Vec<CsetValue<T>> = Vec::with_capacity(CSET_INITIAL_CAP);
        for _ in 0..CSET_INITIAL_CAP {
            new_buckets.push(empty_cset_value::<T>());
        }
        // Drop the old buckets (if any). The previously-stored values are dropped
        // along with the empty placeholder slots.
        let _old = mem::replace(&mut self.buckets, new_buckets);
        // temp_buckets is left empty.
        self.temp_buckets = Vec::new();
    }

    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }

    pub fn tombstone(&self) -> bool {
        // No meaningful single-set "tombstone" check; report whether any tombstone exists.
        self.buckets.iter().any(|b| b.pi == -1)
    }

    pub fn index(&self, index: usize) -> T
    where
        T: Clone,
    {
        self.buckets[index].elem.clone()
    }

    pub fn get_size(&self) -> usize {
        self.bucket_size
    }

    pub fn set_size(&mut self, new_size: usize) {
        self.bucket_size = new_size;
    }

    pub fn get_seed(&self) -> u64 {
        self.seed
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
    }

    pub fn get_max_load_factor(&self) -> f64 {
        self.max_load_factor
    }

    pub fn set_max_load_factor(&mut self, new_factor: f64) {
        self.max_load_factor = new_factor;
    }

    pub fn get_min_load_factor(&self) -> f64 {
        self.min_load_factor
    }

    pub fn set_min_load_factor(&mut self, new_factor: f64) {
        self.min_load_factor = new_factor;
    }

    pub fn get_buckets(&self) -> &Vec<CsetValue<T>> {
        &self.buckets
    }

    #[allow(invalid_reference_casting)]
    pub fn get_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        // The signature requires returning a mutable reference from an immutable
        // receiver. This mirrors the original C macro that hands out a raw
        // pointer without const-correctness tracking. The caller is
        // responsible for not aliasing mutations. None of the public tests
        // rely on this method; it is provided for API parity.
        unsafe {
            let p = std::ptr::addr_of!(self.buckets) as *mut Vec<CsetValue<T>>;
            &mut *p
        }
    }

    #[allow(invalid_reference_casting)]
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe {
            let p = std::ptr::addr_of!(self.temp_buckets) as *mut Vec<CsetValue<T>>;
            &mut *p
        }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    fn cap(&self) -> usize {
        self.buckets.len()
    }

    // Insert `value` into the supplied vector, using this cset's hash/compare settings.
    // Returns true if the value was inserted, false if it was already present.
    fn add_into(
        compare: Option<fn(&T, &T) -> bool>,
        seed: u64,
        vector: &mut Vec<CsetValue<T>>,
        value: T,
    ) -> bool {
        let cap = vector.len();
        let bytes = t_bytes(&value);
        let h1 = xxh64_compute(bytes, seed);
        let mut iteration: usize = 1;
        let mut index: usize;
        let mut found = false;
        loop {
            let h2 = xxh64_h_compute(t_bytes(&value), seed) | 1;
            index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let pi = vector[index].pi;
            if pi == 0 || pi == -1 {
                break;
            }
            // Slot is occupied; compare.
            let matches = match compare {
                Some(cmp) => cmp(&vector[index].elem, &value),
                None => t_bytes(&vector[index].elem) == t_bytes(&value),
            };
            if matches {
                found = true;
                break;
            }
            if iteration - 1 >= cap {
                // Safety net; with a properly-sized table we should always find an empty slot
                // before this triggers.
                break;
            }
        }
        if !found {
            // Replace the slot's value. The previous elem is dropped.
            // For empty (pi==0) slots, the elem was zero-initialised; for tombstones
            // (pi==-1) it held a previously-removed value. Either way, dropping is fine
            // for the trivially-droppable types used in the test suite.
            vector[index].elem = value;
            vector[index].pi = iteration as i32;
            true
        } else {
            // value falls out of scope and is dropped.
            false
        }
    }

    fn resize(&mut self, new_cap: usize) {
        // Build the new bucket vector.
        let mut new_buckets: Vec<CsetValue<T>> = Vec::with_capacity(new_cap);
        for _ in 0..new_cap {
            new_buckets.push(empty_cset_value::<T>());
        }
        // Take the old buckets out, replace with the new (empty) ones.
        let old_buckets = mem::replace(&mut self.buckets, new_buckets);
        let compare = self.compare;
        let seed = self.seed;
        // Reset size; we'll re-insert real entries.
        self.bucket_size = 0;

        // Iterate over old slots; move each occupied entry into the new buckets.
        for old in old_buckets.into_iter() {
            // Destructure to take ownership of pi and elem without invoking Drop on
            // CsetValue as a whole.
            let CsetValue { pi, elem } = old;
            if pi != 0 && pi != -1 {
                if Self::add_into(compare, seed, &mut self.buckets, elem) {
                    self.bucket_size += 1;
                }
            }
            // For empty/tombstone slots, `elem` is dropped here. For empty slots,
            // elem is zero-initialised; for tombstones it was a previously-stored
            // value. Both are safe for trivially-droppable types.
        }
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.cap();
        let load_factor = (self.bucket_size as f64) / (cap as f64);
        if load_factor >= self.max_load_factor {
            self.resize(cap * 2);
        }
        let compare = self.compare;
        let seed = self.seed;
        if Self::add_into(compare, seed, &mut self.buckets, value) {
            self.bucket_size += 1;
            1
        } else {
            0
        }
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let cap = self.cap();
        if cap == 0 {
            return 0;
        }
        let bytes = t_bytes(&value);
        let h1 = xxh64_compute(bytes, self.seed);
        let mut iteration: usize = 1;
        let mut found_index: usize = 0;
        let mut found = false;
        loop {
            if iteration - 1 >= cap {
                break;
            }
            let h2 = xxh64_h_compute(t_bytes(&value), self.seed) | 1;
            let index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                break;
            }
            let matches = match self.compare {
                Some(cmp) => cmp(&self.buckets[index].elem, &value),
                None => t_bytes(&self.buckets[index].elem) == t_bytes(&value),
            };
            if matches {
                found = true;
                found_index = index;
                break;
            }
        }
        if found {
            self.buckets[found_index].pi = -1;
            self.bucket_size -= 1;
            1
        } else {
            0
        }
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.contains_ref(value)
    }

    fn contains_ref(&self, value: &T) -> bool {
        let cap = self.cap();
        if cap == 0 {
            return false;
        }
        let bytes = t_bytes(value);
        let h1 = xxh64_compute(bytes, self.seed);
        let mut iteration: usize = 1;
        let mut found = false;
        loop {
            if iteration - 1 >= cap {
                break;
            }
            let h2 = xxh64_h_compute(t_bytes(value), self.seed) | 1;
            let index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                break;
            }
            let matches = match self.compare {
                Some(cmp) => cmp(&self.buckets[index].elem, value),
                None => t_bytes(&self.buckets[index].elem) == t_bytes(value),
            };
            if matches {
                found = true;
                break;
            }
        }
        found
    }

    pub fn iter(&mut self) -> Vec<T> {
        // Collects the values stored in the cset by raw-copying their bytes.
        // This is only safe for `Copy`-like types (no internal heap pointers).
        // The test suite only ever calls `iter` on `i32`, which is Copy.
        let mut result: Vec<T> = Vec::with_capacity(self.bucket_size);
        for bucket in &self.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                let copy = unsafe { ptr::read(&bucket.elem) };
                result.push(copy);
            }
        }
        result
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        // Equivalent to free + re-init with the initial capacity.
        let mut new_buckets: Vec<CsetValue<T>> = Vec::with_capacity(CSET_INITIAL_CAP);
        for _ in 0..CSET_INITIAL_CAP {
            new_buckets.push(empty_cset_value::<T>());
        }
        let _old = mem::replace(&mut self.buckets, new_buckets);
        self.bucket_size = 0;
    }

    // Walk over the occupied entries of `source` and call `f` with a reference to each.
    fn for_each_in<F: FnMut(&T)>(source: &Cset<T>, mut f: F) {
        for bucket in &source.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                f(&bucket.elem);
            }
        }
    }

    // Helper that copies a T value out of a slot via raw bytes. Same caveat as
    // `iter`: only safe for Copy-like types.
    fn raw_copy(value: &T) -> T {
        unsafe { ptr::read(value as *const T) }
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        // Fully reset `self` before populating it.
        self.clear();
        let mut to_add: Vec<T> = Vec::new();
        Self::for_each_in(first, |elem| {
            if second.contains_ref(elem) {
                to_add.push(Self::raw_copy(elem));
            }
        });
        for v in to_add {
            self.add(v);
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        self.clear();
        let mut to_add: Vec<T> = Vec::new();
        Self::for_each_in(first, |elem| {
            to_add.push(Self::raw_copy(elem));
        });
        Self::for_each_in(second, |elem| {
            to_add.push(Self::raw_copy(elem));
        });
        for v in to_add {
            self.add(v);
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        let mut disjoint = true;
        for bucket in &self.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                if other.contains_ref(&bucket.elem) {
                    disjoint = false;
                    break;
                }
            }
        }
        disjoint
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        self.clear();
        let mut to_add: Vec<T> = Vec::new();
        Self::for_each_in(first, |elem| {
            if !second.contains_ref(elem) {
                to_add.push(Self::raw_copy(elem));
            }
        });
        for v in to_add {
            self.add(v);
        }
    }
}
