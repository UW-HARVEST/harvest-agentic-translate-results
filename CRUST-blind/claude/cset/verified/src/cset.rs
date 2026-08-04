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

#[inline]
fn rotl64(x: u64, r: u32) -> u64 {
    x.rotate_left(r)
}

#[inline]
fn read_u64_le(bytes: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    u64::from_le_bytes(arr)
}

#[inline]
fn read_u32_le(bytes: &[u8]) -> u32 {
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[..4]);
    u32::from_le_bytes(arr)
}

// Internal slice-based finalize
fn finalize_slice(mut h64: u64, mut bytes: &[u8]) -> u64 {
    while bytes.len() >= 8 {
        let k1 = xxh64_round(0, read_u64_le(&bytes[0..8]));
        h64 ^= k1;
        h64 = rotl64(h64, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        bytes = &bytes[8..];
    }
    if bytes.len() >= 4 {
        h64 ^= (read_u32_le(&bytes[0..4]) as u64).wrapping_mul(XXH_PRIME64_1);
        h64 = rotl64(h64, 23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        bytes = &bytes[4..];
    }
    while !bytes.is_empty() {
        h64 ^= (bytes[0] as u64).wrapping_mul(XXH_PRIME64_5);
        h64 = rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        bytes = &bytes[1..];
    }
    xxh64_avalanche(h64)
}

fn endian_align_slice(input: &[u8], seed: u64) -> u64 {
    let len = input.len();
    let mut h64;
    let mut bytes = input;

    if len >= 32 {
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_u64_le(&bytes[0..8]));
            v2 = xxh64_round(v2, read_u64_le(&bytes[8..16]));
            v3 = xxh64_round(v3, read_u64_le(&bytes[16..24]));
            v4 = xxh64_round(v4, read_u64_le(&bytes[24..32]));
            bytes = &bytes[32..];
            if bytes.len() < 32 {
                break;
            }
        }

        h64 = rotl64(v1, 1)
            .wrapping_add(rotl64(v2, 7))
            .wrapping_add(rotl64(v3, 12))
            .wrapping_add(rotl64(v4, 18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(len as u64);
    finalize_slice(h64, bytes)
}

fn endian_align_h_slice(input: &[u8], seed: u64) -> u64 {
    let len = input.len();
    let mut h64;
    let mut bytes = input;

    if len >= 32 {
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_u64_le(&bytes[0..8]));
            v2 = xxh64_round(v2, read_u64_le(&bytes[8..16]));
            v3 = xxh64_round(v3, read_u64_le(&bytes[16..24]));
            v4 = xxh64_round(v4, read_u64_le(&bytes[24..32]));
            bytes = &bytes[32..];
            if bytes.len() < 32 {
                break;
            }
        }

        h64 = rotl64(v1, 1)
            .wrapping_add(rotl64(v2, 7))
            .wrapping_add(rotl64(v3, 12))
            .wrapping_add(rotl64(v4, 18));
        h64 = xxh64_merge_round(h64, v1);
        h64 = xxh64_merge_round(h64, v2);
        h64 = xxh64_merge_round(h64, v3);
        h64 = xxh64_merge_round(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }

    h64 = h64.wrapping_add(len as u64);
    finalize_slice(h64, bytes)
}

// Get the byte representation of a value (used for default hashing/comparison).
fn value_as_bytes<T>(value: &T) -> &[u8] {
    let size = mem::size_of::<T>();
    if size == 0 {
        return &[];
    }
    // SAFETY: We are creating a read-only view of the bytes of a valid `T`. The
    // slice's lifetime is tied to the input reference, and we only read it for
    // hashing/comparison purposes.
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, size) }
}

fn h1_hash<T>(seed: u64, value: &T) -> u64 {
    endian_align_slice(value_as_bytes(value), seed)
}

fn h2_hash<T>(seed: u64, value: &T) -> u64 {
    endian_align_h_slice(value_as_bytes(value), seed) | 1
}

fn matches_value<T>(compare: Option<fn(&T, &T) -> bool>, a: &T, b: &T) -> bool {
    match compare {
        Some(f) => f(a, b),
        None => value_as_bytes(a) == value_as_bytes(b),
    }
}

#[inline]
fn double_hash_index(h1: u64, h2: u64, i: u64, cap: usize) -> usize {
    (h1.wrapping_add(i.wrapping_mul(h2)) as usize) % cap
}

// Function Definitions
pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    // SAFETY: This mirrors the C semantics where the caller passes a pointer to
    // at least 8 bytes of memory. Used internally; not invoked elsewhere in the
    // crate (we use slice-based helpers).
    unsafe {
        let p = mem_ptr as *const u8;
        let bytes = std::slice::from_raw_parts(p, 8);
        u64::from_le_bytes(bytes.try_into().unwrap())
    }
}

pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_get64bits(mem_ptr)
}

pub fn xxh_is_little_endian() -> bool {
    let one: u32 = 1;
    let bytes = one.to_ne_bytes();
    bytes[0] == 1
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
    acc = rotl64(acc, 31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    let val = xxh64_round(0, val);
    let acc = acc ^ val;
    acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}

pub fn xxh_get_32bits(ptr: &mut XXHU32) -> XXHU32 {
    *ptr
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

pub fn xxh64_finalize(h64: XXHU64, ptr: &mut XXHU8, len: usize) -> XXHU64 {
    let len_masked = len & 31;
    if len_masked == 0 {
        return finalize_slice(h64, &[]);
    }
    // SAFETY: The caller is expected to provide a valid byte buffer of at
    // least `len_masked` bytes (matching C's pointer + length contract).
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len_masked) };
    finalize_slice(h64, bytes)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    if len == 0 {
        return endian_align_slice(&[], seed);
    }
    // SAFETY: Caller's contract: `input` points to a buffer of at least `len`
    // valid bytes.
    let bytes = unsafe { std::slice::from_raw_parts(input as *const u8, len) };
    endian_align_slice(bytes, seed)
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    if len == 0 {
        return endian_align_h_slice(&[], seed);
    }
    // SAFETY: Caller's contract: `input` points to a buffer of at least `len`
    // valid bytes.
    let bytes = unsafe { std::slice::from_raw_parts(input as *const u8, len) };
    endian_align_h_slice(bytes, seed)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() || len == 0 {
        return endian_align_slice(&[], seed);
    }
    // SAFETY: Caller's contract: `input` points to a buffer of at least `len`
    // valid bytes.
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    endian_align_slice(bytes, seed)
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() || len == 0 {
        return endian_align_h_slice(&[], seed);
    }
    // SAFETY: Caller's contract: `input` points to a buffer of at least `len`
    // valid bytes.
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    endian_align_h_slice(bytes, seed)
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    if size == 0 {
        return endian_align_slice(&[], CSET_DEFAULT_SEED);
    }
    // SAFETY: Caller is expected to pass a valid pointer and length.
    let bytes = unsafe { std::slice::from_raw_parts(memptr as *const u8, size) };
    endian_align_slice(bytes, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    if size == 0 {
        return endian_align_h_slice(&[], CSET_DEFAULT_SEED) | 1;
    }
    // SAFETY: Caller is expected to pass a valid pointer and length.
    let bytes = unsafe { std::slice::from_raw_parts(memptr as *const u8, size) };
    endian_align_h_slice(bytes, CSET_DEFAULT_SEED) | 1
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

impl<T: Default + Clone> Cset<T> {
    pub fn new() -> Cset<T> {
        let mut buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        for _ in 0..CSET_INITIAL_CAP {
            buckets.push(CsetValue {
                pi: 0,
                elem: T::default(),
            });
        }
        Cset {
            buckets,
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: CsetValue {
                pi: 0,
                elem: T::default(),
            },
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
        let mut buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        for _ in 0..CSET_INITIAL_CAP {
            buckets.push(CsetValue {
                pi: 0,
                elem: T::default(),
            });
        }
        self.buckets = buckets;
        self.temp_buckets = Vec::new();
    }

    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }

    pub fn tombstone(&self) -> bool {
        self.buckets.iter().any(|b| b.pi == -1)
    }

    pub fn index(&self, index: usize) -> T {
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

    pub fn get_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        // The signature mandates returning a mutable reference from a shared
        // reference. We use `addr_of!` so we never form an `&T` that we then
        // cast to `&mut T` (which would be undefined behavior the compiler
        // refuses outright).
        // SAFETY: The caller agrees not to alias the resulting reference. This
        // mirrors the C macro `cset__vector_buckets_ref` which simply returns a
        // raw pointer to the embedded buckets vector.
        let p: *const Vec<CsetValue<T>> = ptr::addr_of!(self.buckets);
        unsafe { &mut *(p as *mut Vec<CsetValue<T>>) }
    }

    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        // SAFETY: See `get_buckets_ref`. We use `addr_of!` to avoid creating
        // an intermediate `&T` reference.
        let p: *const Vec<CsetValue<T>> = ptr::addr_of!(self.temp_buckets);
        unsafe { &mut *(p as *mut Vec<CsetValue<T>>) }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        if cap == 0 {
            // Re-initialize buckets if not initialized.
            for _ in 0..CSET_INITIAL_CAP {
                self.buckets.push(CsetValue {
                    pi: 0,
                    elem: T::default(),
                });
            }
        }
        let cap = self.buckets.len();
        let current_load_factor = self.bucket_size as f64 / cap as f64;
        if current_load_factor >= self.max_load_factor {
            self.resize(cap * 2);
        }
        self.add_internal(value)
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        if cap == 0 {
            return 0;
        }
        let h1 = h1_hash(self.seed, &value);
        let mut iteration: usize = 1;
        let mut index: usize = 0;
        let mut found = false;
        loop {
            if (iteration - 1) >= cap {
                break;
            }
            let h2 = h2_hash(self.seed, &value);
            index = double_hash_index(h1, h2, (iteration - 1) as u64, cap);
            iteration += 1;
            if self.buckets[index].pi == -1 {
                continue;
            }
            if self.buckets[index].pi == 0 {
                break;
            }
            if matches_value(self.compare, &self.buckets[index].elem, &value) {
                found = true;
                break;
            }
        }
        if found {
            self.buckets[index].pi = -1;
            self.bucket_size -= 1;
            1
        } else {
            0
        }
    }

    pub fn contains(&mut self, value: &T) -> bool {
        Self::contains_check(self, value)
    }

    pub fn iter(&mut self) -> Vec<T> {
        let mut result = Vec::new();
        for bucket in &self.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                result.push(bucket.elem.clone());
            }
        }
        result
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        let mut buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        for _ in 0..CSET_INITIAL_CAP {
            buckets.push(CsetValue {
                pi: 0,
                elem: T::default(),
            });
        }
        self.buckets = buckets;
        self.bucket_size = 0;
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        for bucket in &first.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            if Self::contains_check(second, &bucket.elem) {
                self.add(bucket.elem.clone());
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        for bucket in &first.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            self.add(bucket.elem.clone());
        }
        for bucket in &second.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            self.add(bucket.elem.clone());
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for bucket in &self.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            if Self::contains_check(other, &bucket.elem) {
                return false;
            }
        }
        true
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        for bucket in &first.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            if !Self::contains_check(second, &bucket.elem) {
                self.add(bucket.elem.clone());
            }
        }
    }
}

// Private helpers
impl<T: Default + Clone> Cset<T> {
    fn contains_check(set: &Self, value: &T) -> bool {
        let cap = set.buckets.len();
        if cap == 0 {
            return false;
        }
        let h1 = h1_hash(set.seed, value);
        let mut iteration: usize = 1;
        loop {
            if (iteration - 1) >= cap {
                return false;
            }
            let h2 = h2_hash(set.seed, value);
            let index = double_hash_index(h1, h2, (iteration - 1) as u64, cap);
            iteration += 1;
            if set.buckets[index].pi == -1 {
                continue;
            }
            if set.buckets[index].pi == 0 {
                return false;
            }
            if matches_value(set.compare, &set.buckets[index].elem, value) {
                return true;
            }
        }
    }

    fn add_internal(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let h1 = h1_hash(self.seed, &value);
        let mut iteration: usize = 1;
        let mut index: usize = 0;
        let mut found = false;
        // Bound the loop at 2*cap to guarantee termination even in pathological
        // cases (matches the cap-based termination of contains/remove for
        // safety; the C version omits this bound but relies on resize ahead of
        // time).
        let max_iter = cap.saturating_mul(2).max(1);
        for _ in 0..max_iter {
            let h2 = h2_hash(self.seed, &value);
            index = double_hash_index(h1, h2, (iteration - 1) as u64, cap);
            iteration += 1;
            if self.buckets[index].pi == 0 || self.buckets[index].pi == -1 {
                break;
            }
            if matches_value(self.compare, &self.buckets[index].elem, &value) {
                found = true;
                break;
            }
        }
        if !found {
            self.buckets[index].elem = value;
            self.buckets[index].pi = iteration as i32;
            self.bucket_size += 1;
            1
        } else {
            0
        }
    }

    fn resize(&mut self, new_cap: usize) {
        let new_cap = if new_cap == 0 { CSET_INITIAL_CAP } else { new_cap };
        let mut new_buckets: Vec<CsetValue<T>> = Vec::with_capacity(new_cap);
        for _ in 0..new_cap {
            new_buckets.push(CsetValue {
                pi: 0,
                elem: T::default(),
            });
        }
        let old_buckets = mem::replace(&mut self.buckets, new_buckets);
        self.bucket_size = 0;
        for bucket in old_buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            self.add_internal(bucket.elem);
        }
    }
}

// Suppress unused-import warnings for `ptr` (kept to mirror original imports).
#[allow(dead_code)]
fn _unused_ptr_marker() {
    let _ = ptr::null::<u8>();
}
