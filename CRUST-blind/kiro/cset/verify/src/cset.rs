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

fn xxh_rotl64(x: u64, r: u32) -> u64 {
    (x << r) | (x >> (64 - r))
}

// Helper: read bytes as little-endian u64 from a byte slice
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

// Helper: read bytes as little-endian u32 from a byte slice
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
    *mem_ptr as u64
}
pub fn xxh_is_little_endian() -> bool {
    cfg!(target_endian = "little")
}
pub fn xxh_read_le64_align(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64(mem_ptr)
}
pub fn xxh_swap32(x: &mut XXHU32) -> XXHU32 {
    ((*x << 24) & 0xff000000)
        | ((*x << 8) & 0x00ff0000)
        | ((*x >> 8) & 0x0000ff00)
        | ((*x >> 24) & 0x000000ff)
}
pub fn xxh_read32(mem_ptr: &mut XXHU32) -> XXHU32 {
    *mem_ptr
}
pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    let mut a = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    a = xxh_rotl64(a, 31);
    a.wrapping_mul(XXH_PRIME64_1)
}
pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    let v = xxh64_round(0, val);
    (acc ^ v).wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}
pub fn xxh_get_32bits(ptr: &mut XXHU32) -> XXHU32 {
    xxh_read_le32_align(ptr)
}
pub fn xxh_read_le32_align(ptr: &mut XXHU32) -> XXHU32 {
    *ptr
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
    // This signature only gives us a single byte reference; real work is in the slice-based helper
    h64
}
pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    xxh64_slice(&[*input], len, seed)
}
pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    xxh64_h_slice(&[*input], len, seed)
}
pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() || len == 0 {
        return xxh64_slice(&[], 0, seed);
    }
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_slice(bytes, len, seed)
}
pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() || len == 0 {
        return xxh64_h_slice(&[], 0, seed);
    }
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_h_slice(bytes, len, seed)
}

// Slice-based XXH64 (matches C XXH64_endian_align)
fn xxh64_slice(input: &[u8], len: usize, seed: u64) -> u64 {
    let mut h64: u64;
    let mut pos = 0;

    if len >= 32 {
        let limit = len - 32;
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_le64(&input[pos..]));
            pos += 8;
            v2 = xxh64_round(v2, read_le64(&input[pos..]));
            pos += 8;
            v3 = xxh64_round(v3, read_le64(&input[pos..]));
            pos += 8;
            v4 = xxh64_round(v4, read_le64(&input[pos..]));
            pos += 8;
            if pos > limit {
                break;
            }
        }

        h64 = xxh_rotl64(v1, 1)
            .wrapping_add(xxh_rotl64(v2, 7))
            .wrapping_add(xxh_rotl64(v3, 12))
            .wrapping_add(xxh_rotl64(v4, 18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(len as u64);
    finalize_slice(h64, &input[pos..], len & 31)
}

// Slice-based XXH64_h (matches C XXH64_endian_align_h — different seed init)
fn xxh64_h_slice(input: &[u8], len: usize, seed: u64) -> u64 {
    let mut h64: u64;
    let mut pos = 0;

    if len >= 32 {
        let limit = len - 32;
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_le64(&input[pos..]));
            pos += 8;
            v2 = xxh64_round(v2, read_le64(&input[pos..]));
            pos += 8;
            v3 = xxh64_round(v3, read_le64(&input[pos..]));
            pos += 8;
            v4 = xxh64_round(v4, read_le64(&input[pos..]));
            pos += 8;
            if pos > limit {
                break;
            }
        }

        h64 = xxh_rotl64(v1, 1)
            .wrapping_add(xxh_rotl64(v2, 7))
            .wrapping_add(xxh_rotl64(v3, 12))
            .wrapping_add(xxh_rotl64(v4, 18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }

    h64 = h64.wrapping_add(len as u64);
    finalize_slice(h64, &input[pos..], len & 31)
}

fn finalize_slice(mut h64: u64, data: &[u8], mut len: usize) -> u64 {
    let mut pos = 0;
    while len >= 8 {
        let k1 = xxh64_round(0, read_le64(&data[pos..]));
        pos += 8;
        h64 ^= k1;
        h64 = xxh_rotl64(h64, 27).wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_le32(&data[pos..]) as u64).wrapping_mul(XXH_PRIME64_1);
        pos += 4;
        h64 = xxh_rotl64(h64, 23).wrapping_mul(XXH_PRIME64_2).wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (data[pos] as u64).wrapping_mul(XXH_PRIME64_5);
        pos += 1;
        h64 = xxh_rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(memptr as *const u8, size) };
    xxh64_slice(bytes, size, CSET_DEFAULT_SEED)
}
pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let bytes = unsafe { std::slice::from_raw_parts(memptr as *const u8, size) };
    xxh64_h_slice(bytes, size, CSET_DEFAULT_SEED) | 1
}

// Helper to get raw bytes of a value
fn as_bytes<T>(val: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(val as *const T as *const u8, mem::size_of::<T>()) }
}

fn hash1<T>(val: &T, seed: u64) -> u64 {
    let bytes = as_bytes(val);
    xxh64_slice(bytes, bytes.len(), seed)
}

fn hash2<T>(val: &T, seed: u64) -> u64 {
    let bytes = as_bytes(val);
    xxh64_h_slice(bytes, bytes.len(), seed) | 1
}

fn bytes_eq<T>(a: &T, b: &T) -> bool {
    as_bytes(a) == as_bytes(b)
}

fn double_hash_index(h1: u64, h2: u64, i: usize, cap: usize) -> usize {
    ((h1.wrapping_add((i as u64).wrapping_mul(h2))) % cap as u64) as usize
}

pub struct CsetValue<T> {
    pi: i32,
    elem: T,
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

impl<T: Copy + Default> Cset<T> {
    pub fn new() -> Cset<T> {
        let mut s = Cset {
            buckets: Vec::new(),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: CsetValue { pi: 0, elem: T::default() },
            bucket_size: 0,
            compare: None,
            temp_buckets: Vec::new(),
        };
        s.init();
        s
    }

    pub fn init(&mut self) {
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
        self.buckets.clear();
        for _ in 0..CSET_INITIAL_CAP {
            self.buckets.push(CsetValue { pi: 0, elem: T::default() });
        }
    }

    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }

    pub fn tombstone(&self) -> bool {
        self.v.pi == -1
    }

    pub fn index(&self, index: usize) -> T {
        self.buckets[index].elem
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

    pub fn get_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe { &mut *ptr::addr_of!(self.buckets).cast_mut() }
    }

    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe { &mut *ptr::addr_of!(self.temp_buckets).cast_mut() }
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

    fn matches_val(compare: Option<fn(&T, &T) -> bool>, a: &T, b: &T) -> bool {
        match compare {
            Some(cmp) => cmp(a, b),
            None => bytes_eq(a, b),
        }
    }

    fn add_to_vec(compare: Option<fn(&T, &T) -> bool>, seed: u64, value: T, buckets: &mut [CsetValue<T>], bucket_size: &mut usize) {
        let h1 = hash1(&value, seed);
        let h2 = hash2(&value, seed);
        let cap = buckets.len();
        let mut iteration = 1usize;
        loop {
            let idx = double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            if buckets[idx].pi == 0 || buckets[idx].pi == -1 {
                buckets[idx].elem = value;
                buckets[idx].pi = iteration as i32;
                *bucket_size += 1;
                return;
            }
            if Self::matches_val(compare, &buckets[idx].elem, &value) {
                return;
            }
        }
    }

    fn resize(&mut self, new_cap: usize) {
        let mut new_buckets: Vec<CsetValue<T>> = (0..new_cap)
            .map(|_| CsetValue { pi: 0, elem: T::default() })
            .collect();
        let mut new_size = 0usize;
        for i in 0..self.buckets.len() {
            let pi = self.buckets[i].pi;
            if pi == 0 || pi == -1 { continue; }
            let elem = self.buckets[i].elem;
            Self::add_to_vec(self.compare, self.seed, elem, &mut new_buckets, &mut new_size);
        }
        self.buckets = new_buckets;
        self.bucket_size = new_size;
    }

    pub fn add(&mut self, value: T) -> i32 {
        let load = self.bucket_size as f64 / self.cap() as f64;
        if load >= self.max_load_factor {
            let new_cap = self.cap() * 2;
            self.resize(new_cap);
        }
        Self::add_to_vec(self.compare, self.seed, value, &mut self.buckets, &mut self.bucket_size);
        0
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let h1 = hash1(&value, self.seed);
        let h2 = hash2(&value, self.seed);
        let cap = self.cap();
        let mut iteration = 1usize;
        loop {
            if iteration - 1 >= cap { break; }
            let idx = double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            if self.buckets[idx].pi == -1 { continue; }
            if self.buckets[idx].pi == 0 { break; }
            if Self::matches_val(self.compare, &self.buckets[idx].elem, &value) {
                self.buckets[idx].pi = -1;
                self.bucket_size -= 1;
                return 0;
            }
        }
        0
    }

    pub fn contains(&mut self, value: &T) -> bool {
        Self::contains_in(&self.buckets, self.compare, self.seed, value)
    }

    pub fn iter(&mut self) -> Vec<T> {
        self.buckets.iter()
            .filter(|b| b.pi != 0 && b.pi != -1)
            .map(|b| b.elem)
            .collect()
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
        self.bucket_size = 0;
        for _ in 0..CSET_INITIAL_CAP {
            self.buckets.push(CsetValue { pi: 0, elem: T::default() });
        }
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 { continue; }
            if Self::contains_in(&second.buckets, second.compare, second.seed, &first.buckets[i].elem) {
                self.add(first.buckets[i].elem);
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            if first.buckets[i].pi != 0 && first.buckets[i].pi != -1 {
                self.add(first.buckets[i].elem);
            }
        }
        for i in 0..second.buckets.len() {
            if second.buckets[i].pi != 0 && second.buckets[i].pi != -1 {
                self.add(second.buckets[i].elem);
            }
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for i in 0..self.buckets.len() {
            if self.buckets[i].pi == 0 || self.buckets[i].pi == -1 { continue; }
            if Self::contains_in(&other.buckets, other.compare, other.seed, &self.buckets[i].elem) {
                return false;
            }
        }
        true
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 { continue; }
            if !Self::contains_in(&second.buckets, second.compare, second.seed, &first.buckets[i].elem) {
                self.add(first.buckets[i].elem);
            }
        }
    }

    fn contains_in(buckets: &[CsetValue<T>], compare: Option<fn(&T, &T) -> bool>, seed: u64, value: &T) -> bool {
        let h1 = hash1(value, seed);
        let h2 = hash2(value, seed);
        let cap = buckets.len();
        let mut iteration = 1usize;
        loop {
            if iteration - 1 >= cap { return false; }
            let idx = double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            if buckets[idx].pi == -1 { continue; }
            if buckets[idx].pi == 0 { return false; }
            if Self::matches_val(compare, &buckets[idx].elem, value) {
                return true;
            }
        }
    }
}
