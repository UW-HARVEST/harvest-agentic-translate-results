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

fn read_le64(bytes: &[u8]) -> u64 {
    bytes[0] as u64
        | (bytes[1] as u64) << 8
        | (bytes[2] as u64) << 16
        | (bytes[3] as u64) << 24
        | (bytes[4] as u64) << 32
        | (bytes[5] as u64) << 40
        | (bytes[6] as u64) << 48
        | (bytes[7] as u64) << 56
}

fn read_le32(bytes: &[u8]) -> u32 {
    bytes[0] as u32
        | (bytes[1] as u32) << 8
        | (bytes[2] as u32) << 16
        | (bytes[3] as u32) << 24
}

// Function Definitions
pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}
pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(mem_ptr as *const u8, 8) };
    read_le64(bytes)
}
pub fn xxh_is_little_endian() -> bool {
    cfg!(target_endian = "little")
}
pub fn xxh_read_le64_align(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64(mem_ptr)
}
pub fn xxh_swap32(x: &mut XXHU32) -> XXHU32 {
    ((*x) << 24 & 0xff000000)
        | ((*x) << 8 & 0x00ff0000)
        | ((*x) >> 8 & 0x0000ff00)
        | ((*x) >> 24 & 0x000000ff)
}
pub fn xxh_read32(mem_ptr: &mut XXHU32) -> XXHU32 {
    *mem_ptr
}
pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    let a = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    a.rotate_left(31).wrapping_mul(XXH_PRIME64_1)
}
pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    let v = xxh64_round(0, val);
    (acc ^ v).wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}
pub fn xxh_get_32bits(ptr: &mut XXHU32) -> XXHU32 {
    xxh_read_le32_align(ptr)
}
pub fn xxh_read_le32_align(ptr: &mut XXHU32) -> XXHU32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const XXHU32 as *const u8, 4) };
    read_le32(bytes)
}
pub fn xxh64_avalanche(mut h64: XXHU64) -> XXHU64 {
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(XXH_PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(XXH_PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}
pub fn xxh64_finalize(mut h64: XXHU64, ptr: &mut XXHU8, len: usize) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    xxh64_finalize_bytes(h64, bytes)
}

fn xxh64_finalize_bytes(mut h64: u64, data: &[u8]) -> u64 {
    let mut remaining = data.len() & 31;
    let mut offset = data.len() - remaining;
    while remaining >= 8 {
        let k1 = xxh64_round(0, read_le64(&data[offset..]));
        offset += 8;
        h64 ^= k1;
        h64 = h64.rotate_left(27).wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4);
        remaining -= 8;
    }
    if remaining >= 4 {
        h64 ^= (read_le32(&data[offset..]) as u64).wrapping_mul(XXH_PRIME64_1);
        offset += 4;
        h64 = h64.rotate_left(23).wrapping_mul(XXH_PRIME64_2).wrapping_add(XXH_PRIME64_3);
        remaining -= 4;
    }
    while remaining > 0 {
        h64 ^= (data[offset] as u64).wrapping_mul(XXH_PRIME64_5);
        offset += 1;
        h64 = h64.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        remaining -= 1;
    }
    xxh64_avalanche(h64)
}

fn xxh64_compute(input: &[u8], seed: u64) -> u64 {
    let len = input.len();
    let mut h64: u64;
    if len >= 32 {
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        let mut pos = 0;
        let limit = len - 32;
        loop {
            v1 = xxh64_round(v1, read_le64(&input[pos..])); pos += 8;
            v2 = xxh64_round(v2, read_le64(&input[pos..])); pos += 8;
            v3 = xxh64_round(v3, read_le64(&input[pos..])); pos += 8;
            v4 = xxh64_round(v4, read_le64(&input[pos..])); pos += 8;
            if pos > limit { break; }
        }
        h64 = v1.rotate_left(1).wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12)).wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }
    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h64, input)
}

fn xxh64_h_compute(input: &[u8], seed: u64) -> u64 {
    let len = input.len();
    let mut h64: u64;
    if len >= 32 {
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        let mut pos = 0;
        let limit = len - 32;
        loop {
            v1 = xxh64_round(v1, read_le64(&input[pos..])); pos += 8;
            v2 = xxh64_round(v2, read_le64(&input[pos..])); pos += 8;
            v3 = xxh64_round(v3, read_le64(&input[pos..])); pos += 8;
            v4 = xxh64_round(v4, read_le64(&input[pos..])); pos += 8;
            if pos > limit { break; }
        }
        h64 = v1.rotate_left(1).wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12)).wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }
    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h64, input)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(input as *const u8, len) };
    xxh64_compute(bytes, seed)
}
pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(input as *const u8, len) };
    xxh64_h_compute(bytes, seed)
}
pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_compute(bytes, seed)
}
pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_h_compute(bytes, seed)
}
pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(memptr as *const u8, size) };
    xxh64_compute(bytes, CSET_DEFAULT_SEED)
}
pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(memptr as *const u8, size) };
    xxh64_h_compute(bytes, CSET_DEFAULT_SEED) | 1
}

fn as_bytes<T>(val: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(val as *const T as *const u8, mem::size_of::<T>()) }
}

fn h1hash<T>(val: &T, seed: u64) -> u64 {
    xxh64_compute(as_bytes(val), seed)
}

fn h2hash<T>(val: &T, seed: u64) -> u64 {
    xxh64_h_compute(as_bytes(val), seed) | 1
}

fn double_hash_index(h1: u64, h2: u64, i: usize, cap: usize) -> usize {
    (h1.wrapping_add((i as u64).wrapping_mul(h2)) % cap as u64) as usize
}

fn bytes_equal<T>(a: &T, b: &T) -> bool {
    as_bytes(a) == as_bytes(b)
}

/// Bitwise copy of a T value (like C memcpy). Works for non-Copy types.
unsafe fn bitwise_copy<T>(src: &T) -> T {
    ptr::read(src as *const T)
}

pub struct CsetValue<T> {
    pi: i32,
    elem: T,
}

impl<T> CsetValue<T> {
    fn new_empty() -> Self {
        unsafe { mem::zeroed() }
    }
}

pub struct Cset<T> {
    buckets: Vec<CsetValue<T>>,
    max_load_factor: f64,
    min_load_factor: f64,
    seed: u64,
    v: CsetValue<T>,
    bucket_size: usize,
    compare: Option<fn(&T, &T) -> bool>,
    temp_buckets: Vec<CsetValue<T>>,
}

fn make_empty_buckets<T>(cap: usize) -> Vec<CsetValue<T>> {
    let mut v = Vec::with_capacity(cap);
    unsafe {
        ptr::write_bytes(v.as_mut_ptr(), 0, cap);
        v.set_len(cap);
    }
    v
}

impl<T> Cset<T> {
    pub fn new() -> Cset<T> {
        Cset {
            buckets: make_empty_buckets(CSET_INITIAL_CAP),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: CsetValue::new_empty(),
            bucket_size: 0,
            compare: None,
            temp_buckets: Vec::new(),
        }
    }
    pub fn init(&mut self) {
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
        self.buckets = make_empty_buckets(CSET_INITIAL_CAP);
    }
    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }
    pub fn tombstone(&self) -> bool {
        self.v.pi == -1
    }
    pub fn index(&self, index: usize) -> T {
        unsafe { bitwise_copy(&self.buckets[index].elem) }
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
        unsafe { &mut *(&self.buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>) }
    }
    #[allow(invalid_reference_casting)]
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe { &mut *(&self.temp_buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>) }
    }
    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }
    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    fn matches(&self, a: &T, b: &T) -> bool {
        match self.compare {
            Some(cmp) => cmp(a, b),
            None => bytes_equal(a, b),
        }
    }

    fn cap(&self) -> usize {
        self.buckets.len()
    }

    fn contains_in_buckets(&self, value: &T) -> bool {
        let h1 = h1hash(value, self.seed);
        let h2 = h2hash(value, self.seed);
        let cap = self.cap();
        for i in 0..cap {
            let idx = double_hash_index(h1, h2, i, cap);
            let pi = self.buckets[idx].pi;
            if pi == -1 { continue; }
            if pi == 0 { return false; }
            if self.matches(&self.buckets[idx].elem, value) {
                return true;
            }
        }
        false
    }

    fn add_to_buckets(buckets: &mut Vec<CsetValue<T>>, value: &T, seed: u64, compare: Option<fn(&T, &T) -> bool>, bucket_size: &mut usize) {
        let h1 = h1hash(value, seed);
        let h2 = h2hash(value, seed);
        let cap = buckets.len();
        let mut iteration = 1usize;
        loop {
            let idx = double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let pi = buckets[idx].pi;
            if pi == 0 || pi == -1 {
                unsafe { ptr::copy_nonoverlapping(value as *const T, &mut buckets[idx].elem as *mut T, 1); }
                buckets[idx].pi = iteration as i32;
                *bucket_size += 1;
                return;
            }
            let m = match compare {
                Some(cmp) => cmp(&buckets[idx].elem, value),
                None => bytes_equal(&buckets[idx].elem, value),
            };
            if m { return; }
        }
    }

    fn resize(&mut self, new_cap: usize) {
        let mut new_buckets = make_empty_buckets(new_cap);
        self.bucket_size = 0;
        for i in 0..self.buckets.len() {
            if self.buckets[i].pi > 0 {
                Self::add_to_buckets(&mut new_buckets, &self.buckets[i].elem, self.seed, self.compare, &mut self.bucket_size);
            }
        }
        self.buckets = new_buckets;
    }

    pub fn add(&mut self, value: T) -> i32 {
        let load = self.bucket_size as f64 / self.cap() as f64;
        if load >= self.max_load_factor {
            let new_cap = self.cap() * 2;
            self.resize(new_cap);
        }
        Self::add_to_buckets(&mut self.buckets, &value, self.seed, self.compare, &mut self.bucket_size);
        0
    }
    pub fn remove(&mut self, value: T) -> i32 {
        let h1 = h1hash(&value, self.seed);
        let h2 = h2hash(&value, self.seed);
        let cap = self.cap();
        for i in 0..cap {
            let idx = double_hash_index(h1, h2, i, cap);
            let pi = self.buckets[idx].pi;
            if pi == -1 { continue; }
            if pi == 0 { break; }
            if self.matches(&self.buckets[idx].elem, &value) {
                self.buckets[idx].pi = -1;
                self.bucket_size -= 1;
                break;
            }
        }
        0
    }
    pub fn contains(&mut self, value: &T) -> bool {
        self.contains_in_buckets(value)
    }
    pub fn iter(&mut self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.bucket_size);
        for cv in &self.buckets {
            if cv.pi > 0 {
                result.push(unsafe { bitwise_copy(&cv.elem) });
            }
        }
        result
    }
    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }
    pub fn clear(&mut self) {
        self.buckets = make_empty_buckets(CSET_INITIAL_CAP);
        self.bucket_size = 0;
    }
    pub fn intersect(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            if first.buckets[i].pi > 0 && second.contains_in_buckets(&first.buckets[i].elem) {
                let val = unsafe { bitwise_copy(&first.buckets[i].elem) };
                self.add(val);
            }
        }
    }
    pub fn union(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            if first.buckets[i].pi > 0 {
                let val = unsafe { bitwise_copy(&first.buckets[i].elem) };
                self.add(val);
            }
        }
        for i in 0..second.buckets.len() {
            if second.buckets[i].pi > 0 {
                let val = unsafe { bitwise_copy(&second.buckets[i].elem) };
                self.add(val);
            }
        }
    }
    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for i in 0..self.buckets.len() {
            if self.buckets[i].pi > 0 && other.contains_in_buckets(&self.buckets[i].elem) {
                return false;
            }
        }
        true
    }
    pub fn difference(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            if first.buckets[i].pi > 0 && !second.contains_in_buckets(&first.buckets[i].elem) {
                let val = unsafe { bitwise_copy(&first.buckets[i].elem) };
                self.add(val);
            }
        }
    }
}
