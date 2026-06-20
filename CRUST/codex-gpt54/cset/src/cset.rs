use std::mem::{self, MaybeUninit};

pub type XXH64HashT = u64;
pub type XXHU8 = u8;
pub type XXHU64 = XXH64HashT;
pub type XXHU32 = u32;
pub type XXH32HashT = u32;

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

fn rotl64(x: u64, r: u32) -> u64 {
    x.rotate_left(r)
}

fn as_bytes<T>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((value as *const T).cast::<u8>(), mem::size_of::<T>()) }
}

fn bytes_from_ptr<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

fn read_le64(bytes: &[u8]) -> u64 {
    let mut array = [0_u8; 8];
    array.copy_from_slice(&bytes[..8]);
    u64::from_le_bytes(array)
}

fn read_le32(bytes: &[u8]) -> u32 {
    let mut array = [0_u8; 4];
    array.copy_from_slice(&bytes[..4]);
    u32::from_le_bytes(array)
}

fn xxh64_round_impl(mut acc: u64, input: u64) -> u64 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = rotl64(acc, 31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

fn xxh64_merge_round_impl(mut acc: u64, val: u64) -> u64 {
    let merged = xxh64_round_impl(0, val);
    acc ^= merged;
    acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}

fn xxh64_finalize_impl(mut h64: u64, mut bytes: &[u8]) -> u64 {
    let mut len = bytes.len() & 31;

    while len >= 8 {
        let k1 = xxh64_round_impl(0, read_le64(bytes));
        h64 ^= k1;
        h64 = rotl64(h64, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        bytes = &bytes[8..];
        len -= 8;
    }

    if len >= 4 {
        h64 ^= u64::from(read_le32(bytes)).wrapping_mul(XXH_PRIME64_1);
        h64 = rotl64(h64, 23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        bytes = &bytes[4..];
        len -= 4;
    }

    while len > 0 {
        h64 ^= u64::from(bytes[0]).wrapping_mul(XXH_PRIME64_5);
        h64 = rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        bytes = &bytes[1..];
        len -= 1;
    }

    xxh64_avalanche(h64)
}

fn xxh64_endian_align_impl(bytes: &[u8], seed: u64, alternate: bool) -> u64 {
    let len = bytes.len();
    let mut remaining = bytes;
    let mut h64;

    if len >= 32 {
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = if alternate {
            seed.wrapping_sub(XXH_PRIME64_2)
        } else {
            seed.wrapping_add(XXH_PRIME64_2)
        };
        let mut v3 = if alternate {
            seed.wrapping_add(XXH_PRIME64_3)
        } else {
            seed
        };
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        while remaining.len() >= 32 {
            v1 = xxh64_round_impl(v1, read_le64(&remaining[0..8]));
            v2 = xxh64_round_impl(v2, read_le64(&remaining[8..16]));
            v3 = xxh64_round_impl(v3, read_le64(&remaining[16..24]));
            v4 = xxh64_round_impl(v4, read_le64(&remaining[24..32]));
            remaining = &remaining[32..];
        }

        h64 = rotl64(v1, 1)
            .wrapping_add(rotl64(v2, 7))
            .wrapping_add(rotl64(v3, 12))
            .wrapping_add(rotl64(v4, 18));
        h64 = xxh64_merge_round_impl(h64, v1);
        h64 = xxh64_merge_round_impl(h64, v2);
        h64 = xxh64_merge_round_impl(h64, v3);
        h64 = xxh64_merge_round_impl(h64, v4);
    } else {
        h64 = seed.wrapping_add(if alternate { XXH_PRIME64_1 } else { XXH_PRIME64_5 });
    }

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_impl(h64, remaining)
}

fn default_matches<T>(left: &T, right: &T) -> bool {
    as_bytes(left) == as_bytes(right)
}

fn zeroed_value<T>() -> T {
    unsafe { MaybeUninit::<T>::zeroed().assume_init() }
}

pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64(mem_ptr)
}

pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    let bytes = bytes_from_ptr(mem_ptr as *mut u8 as *const u8, 8);
    read_le64(bytes)
}

pub fn xxh_is_little_endian() -> bool {
    cfg!(target_endian = "little")
}

pub fn xxh_read_le64_align(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64(mem_ptr)
}

pub fn xxh_swap32(x: &mut XXHU32) -> XXHU32 {
    x.swap_bytes()
}

pub fn xxh_read32(mem_ptr: &mut XXHU32) -> XXHU32 {
    *mem_ptr
}

pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    xxh64_round_impl(acc, input)
}

pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    xxh64_merge_round_impl(acc, val)
}

pub fn xxh_get_32bits(ptr: &mut XXHU32) -> XXHU32 {
    xxh_read_le32_align(ptr)
}

pub fn xxh_read_le32_align(ptr: &mut XXHU32) -> XXHU32 {
    if xxh_is_little_endian() {
        xxh_read32(ptr)
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
    let bytes = bytes_from_ptr(ptr as *mut u8 as *const u8, len);
    xxh64_finalize_impl(h64, bytes)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let bytes = bytes_from_ptr(input as *mut u8 as *const u8, len);
    xxh64_endian_align_impl(bytes, seed, false)
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let bytes = bytes_from_ptr(input as *mut u8 as *const u8, len);
    xxh64_endian_align_impl(bytes, seed, true)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    let bytes = bytes_from_ptr(input, len);
    xxh64_endian_align_impl(bytes, seed, false)
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    let bytes = bytes_from_ptr(input, len);
    xxh64_endian_align_impl(bytes, seed, true)
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64(memptr as *mut u8 as *const u8, size, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64_h(memptr as *mut u8 as *const u8, size, CSET_DEFAULT_SEED) | 1
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

impl<T> Cset<T> {
    fn ensure_capacity_for_insert(&mut self) {
        let capacity = self.buckets.capacity().max(CSET_INITIAL_CAP);
        let current_load_factor = if capacity == 0 {
            0.0
        } else {
            self.bucket_size as f64 / capacity as f64
        };

        if current_load_factor >= self.max_load_factor {
            let new_capacity = capacity * 2;
            let mut new_buckets = Vec::with_capacity(new_capacity);
            new_buckets.append(&mut self.buckets);
            self.buckets = new_buckets;
        }
    }

    fn matches(&self, current: &T, other: &T) -> bool {
        if let Some(compare) = self.compare {
            compare(current, other)
        } else {
            default_matches(current, other)
        }
    }

    fn find_position(&self, value: &T) -> Option<usize> {
        self.buckets.iter().position(|entry| self.matches(&entry.elem, value))
    }

    pub fn new() -> Cset<T> {
        let mut cset = Cset {
            buckets: Vec::with_capacity(CSET_INITIAL_CAP),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: CsetValue {
                pi: 0,
                elem: zeroed_value(),
            },
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
        self.buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        self.temp_buckets.clear();
    }

    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }

    pub fn tombstone(&self) -> bool {
        false
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
        self.bucket_size = new_size.min(self.buckets.len());
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
        Box::leak(Box::new(Vec::new()))
    }

    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        Box::leak(Box::new(Vec::new()))
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.capacity().max(CSET_INITIAL_CAP) as i32
    }

    pub fn add(&mut self, value: T) -> i32 {
        if self.find_position(&value).is_some() {
            return self.size();
        }

        self.ensure_capacity_for_insert();
        self.buckets.push(CsetValue { pi: 1, elem: value });
        self.bucket_size += 1;
        self.size()
    }

    pub fn remove(&mut self, value: T) -> i32 {
        if let Some(index) = self.find_position(&value) {
            self.buckets.remove(index);
            self.bucket_size -= 1;
        }
        self.size()
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.find_position(value).is_some()
    }

    pub fn iter(&mut self) -> Vec<T>
    where
        T: Clone,
    {
        self.buckets.iter().map(|entry| entry.elem.clone()).collect()
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        self.buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        self.temp_buckets.clear();
        self.bucket_size = 0;
    }

    pub fn intersect(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for entry in &first.buckets {
            if second
                .buckets
                .iter()
                .any(|other| second.matches(&other.elem, &entry.elem))
            {
                self.add(entry.elem.clone());
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for entry in &first.buckets {
            self.add(entry.elem.clone());
        }
        for entry in &second.buckets {
            self.add(entry.elem.clone());
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        !self
            .buckets
            .iter()
            .any(|entry| other.buckets.iter().any(|other_entry| other.matches(&other_entry.elem, &entry.elem)))
    }

    pub fn difference(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for entry in &first.buckets {
            let contains = second
                .buckets
                .iter()
                .any(|other| second.matches(&other.elem, &entry.elem));
            if !contains {
                self.add(entry.elem.clone());
            }
        }
    }
}
