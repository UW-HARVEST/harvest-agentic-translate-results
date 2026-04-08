// Import necessary modules
use std::cell::UnsafeCell;
use std::mem;
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

fn as_bytes<T>(val: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(val as *const T as *const u8, mem::size_of::<T>()) }
}

// Function Definitions
pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}
pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    let p = mem_ptr as *mut u8;
    unsafe {
        let b = std::slice::from_raw_parts(p, 8);
        b[0] as u64
            | (b[1] as u64) << 8
            | (b[2] as u64) << 16
            | (b[3] as u64) << 24
            | (b[4] as u64) << 32
            | (b[5] as u64) << 40
            | (b[6] as u64) << 48
            | (b[7] as u64) << 56
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
    if xxh_is_little_endian() {
        xxh_read32(ptr)
    } else {
        xxh_swap32(&mut xxh_read32(ptr))
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
pub fn xxh64_finalize(mut h64: XXHU64, ptr: &mut XXHU8, len: usize) -> XXHU64 {
    xxh64_finalize_bytes(h64, ptr as *const u8, len)
}

fn xxh64_finalize_bytes(mut h64: u64, mut ptr: *const u8, len: usize) -> u64 {
    let mut remaining = len & 31;
    while remaining >= 8 {
        let k1 = xxh64_round(0, read_le64_from_ptr(ptr));
        ptr = unsafe { ptr.add(8) };
        h64 ^= k1;
        h64 = xxh_rotl64(h64, 27).wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4);
        remaining -= 8;
    }
    if remaining >= 4 {
        h64 ^= (read_le32_from_ptr(ptr) as u64).wrapping_mul(XXH_PRIME64_1);
        ptr = unsafe { ptr.add(4) };
        h64 = xxh_rotl64(h64, 23).wrapping_mul(XXH_PRIME64_2).wrapping_add(XXH_PRIME64_3);
        remaining -= 4;
    }
    while remaining > 0 {
        let b = unsafe { *ptr };
        ptr = unsafe { ptr.add(1) };
        h64 ^= (b as u64).wrapping_mul(XXH_PRIME64_5);
        h64 = xxh_rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        remaining -= 1;
    }
    xxh64_avalanche(h64)
}

fn read_le64_from_ptr(ptr: *const u8) -> u64 {
    let b = unsafe { std::slice::from_raw_parts(ptr, 8) };
    b[0] as u64
        | (b[1] as u64) << 8
        | (b[2] as u64) << 16
        | (b[3] as u64) << 24
        | (b[4] as u64) << 32
        | (b[5] as u64) << 40
        | (b[6] as u64) << 48
        | (b[7] as u64) << 56
}

fn read_le32_from_ptr(ptr: *const u8) -> u32 {
    let b = unsafe { std::slice::from_raw_parts(ptr, 4) };
    b[0] as u32 | (b[1] as u32) << 8 | (b[2] as u32) << 16 | (b[3] as u32) << 24
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    xxh64_endian_align_impl(input as *const u8, len, seed, false)
}
pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    xxh64_endian_align_impl(input as *const u8, len, seed, true)
}

fn xxh64_endian_align_impl(input: *const u8, len: usize, seed: u64, h_variant: bool) -> u64 {
    let mut ptr = input;
    let h64;

    if len >= 32 {
        let end = unsafe { input.add(len) };
        let limit = unsafe { end.sub(32) };
        let (mut v1, mut v2, mut v3, mut v4) = if h_variant {
            (
                seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2),
                seed.wrapping_sub(XXH_PRIME64_2),
                seed.wrapping_add(XXH_PRIME64_3),
                seed.wrapping_sub(XXH_PRIME64_1),
            )
        } else {
            (
                seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2),
                seed.wrapping_add(XXH_PRIME64_2),
                seed.wrapping_add(0),
                seed.wrapping_sub(XXH_PRIME64_1),
            )
        };

        loop {
            v1 = xxh64_round(v1, read_le64_from_ptr(ptr));
            ptr = unsafe { ptr.add(8) };
            v2 = xxh64_round(v2, read_le64_from_ptr(ptr));
            ptr = unsafe { ptr.add(8) };
            v3 = xxh64_round(v3, read_le64_from_ptr(ptr));
            ptr = unsafe { ptr.add(8) };
            v4 = xxh64_round(v4, read_le64_from_ptr(ptr));
            ptr = unsafe { ptr.add(8) };
            if ptr > limit {
                break;
            }
        }

        h64 = xxh_rotl64(v1, 1)
            .wrapping_add(xxh_rotl64(v2, 7))
            .wrapping_add(xxh_rotl64(v3, 12))
            .wrapping_add(xxh_rotl64(v4, 18));
        let mut h = h64;
        h = xxh64_merge_round(h, v1);
        h = xxh64_merge_round(h, v2);
        h = xxh64_merge_round(h, v3);
        h = xxh64_merge_round(h, v4);
        let h_final = h.wrapping_add(len as u64);
        return xxh64_finalize_bytes(h_final, ptr, len);
    } else {
        h64 = if h_variant {
            seed.wrapping_add(XXH_PRIME64_1)
        } else {
            seed.wrapping_add(XXH_PRIME64_5)
        };
    }

    let h_final = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h_final, ptr, len)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    xxh64_endian_align_impl(input, len, seed, false)
}
pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    xxh64_endian_align_impl(input, len, seed, true)
}
pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64(memptr as *const u8, size, CSET_DEFAULT_SEED)
}
pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64_h(memptr as *const u8, size, CSET_DEFAULT_SEED) | 1
}

pub struct CsetValue<T> {
    pi: i32,
    elem: T,
}

pub struct Cset<T> {
    buckets: UnsafeCell<Vec<CsetValue<T>>>,
    max_load_factor: f64,
    min_load_factor: f64,
    seed: u64,
    v: CsetValue<T>,
    bucket_size: usize,
    compare: Option<fn(&T, &T) -> bool>,
    temp_buckets: UnsafeCell<Vec<CsetValue<T>>>,
}

impl<T: Copy + Default + PartialEq> Cset<T> {
    fn buckets(&self) -> &Vec<CsetValue<T>> {
        unsafe { &*self.buckets.get() }
    }

    fn buckets_mut(&self) -> &mut Vec<CsetValue<T>> {
        unsafe { &mut *self.buckets.get() }
    }

    fn h1hash(&self, val: &T) -> u64 {
        let bytes = as_bytes(val);
        xxh64(bytes.as_ptr(), bytes.len(), self.seed)
    }

    fn h2hash(&self, val: &T) -> u64 {
        let bytes = as_bytes(val);
        xxh64_h(bytes.as_ptr(), bytes.len(), self.seed) | 1
    }

    fn matches(&self, a: &T, b: &T) -> bool {
        if let Some(cmp) = self.compare {
            cmp(a, b)
        } else {
            as_bytes(a) == as_bytes(b)
        }
    }

    fn double_hash_index(h1: u64, h2: u64, i: usize, cap: usize) -> usize {
        ((h1.wrapping_add((i as u64).wrapping_mul(h2))) % cap as u64) as usize
    }

    fn add_to_buckets(&mut self, val: &T, buckets: &mut Vec<CsetValue<T>>) {
        let h1 = self.h1hash(val);
        let h2 = self.h2hash(val);
        let cap = buckets.len();
        let mut iteration = 1usize;
        let mut index;
        let mut found = false;
        loop {
            index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            if buckets[index].pi == 0 || buckets[index].pi == -1 {
                break;
            }
            if self.matches(&buckets[index].elem, val) {
                found = true;
                break;
            }
        }
        if !found {
            buckets[index].elem = *val;
            buckets[index].pi = iteration as i32;
            self.bucket_size += 1;
        }
    }

    fn resize(&mut self, new_cap: usize) {
        let buckets = self.buckets_mut();
        let old_cap = buckets.len();
        let mut new_buckets: Vec<CsetValue<T>> = Vec::with_capacity(new_cap);
        for _ in 0..new_cap {
            new_buckets.push(CsetValue { pi: 0, elem: T::default() });
        }
        self.bucket_size = 0;
        for i in 0..old_cap {
            let buckets = self.buckets();
            let pi = buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let val = buckets[i].elem;
            self.add_to_buckets(&val, &mut new_buckets);
        }
        *self.buckets_mut() = new_buckets;
    }

    pub fn new() -> Cset<T> {
        let mut s = Cset {
            buckets: UnsafeCell::new(Vec::new()),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: CsetValue { pi: 0, elem: T::default() },
            bucket_size: 0,
            compare: None,
            temp_buckets: UnsafeCell::new(Vec::new()),
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
        let buckets = self.buckets_mut();
        *buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        for _ in 0..CSET_INITIAL_CAP {
            buckets.push(CsetValue { pi: 0, elem: T::default() });
        }
    }
    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }
    pub fn tombstone(&self) -> bool {
        self.v.pi == -1
    }
    pub fn index(&self, index: usize) -> T {
        self.buckets()[index].elem
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
        self.buckets()
    }
    pub fn get_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        self.buckets_mut()
    }
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe { &mut *self.temp_buckets.get() }
    }
    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }
    pub fn capacity(&self) -> i32 {
        self.buckets().len() as i32
    }
    pub fn add(&mut self, value: T) -> i32 {
        let load = self.bucket_size as f64 / self.buckets().len() as f64;
        if load >= self.max_load_factor {
            let new_cap = self.buckets().len() * 2;
            self.resize(new_cap);
        }
        self.v.elem = value;
        let val = self.v.elem;
        let h1 = self.h1hash(&val);
        let h2 = self.h2hash(&val);
        let cap = self.buckets().len();
        let mut iteration = 1usize;
        let mut index;
        let mut found = false;
        loop {
            index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let buckets = self.buckets();
            if buckets[index].pi == 0 || buckets[index].pi == -1 {
                break;
            }
            if self.matches(&buckets[index].elem, &val) {
                found = true;
                break;
            }
        }
        if !found {
            let buckets = self.buckets_mut();
            buckets[index].elem = val;
            buckets[index].pi = iteration as i32;
            self.bucket_size += 1;
        }
        0
    }
    pub fn remove(&mut self, value: T) -> i32 {
        self.v.elem = value;
        let val = self.v.elem;
        let h1 = self.h1hash(&val);
        let h2 = self.h2hash(&val);
        let cap = self.buckets().len();
        let mut iteration = 1usize;
        let mut found = false;
        let mut index = 0;
        loop {
            if (iteration - 1) >= cap {
                break;
            }
            index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let buckets = self.buckets();
            if buckets[index].pi == -1 {
                continue;
            }
            if buckets[index].pi == 0 {
                break;
            }
            if self.matches(&buckets[index].elem, &val) {
                found = true;
                break;
            }
        }
        if found {
            self.buckets_mut()[index].pi = -1;
            self.bucket_size -= 1;
        }
        0
    }
    pub fn contains(&mut self, value: &T) -> bool {
        self.v.elem = *value;
        let val = self.v.elem;
        self.contains_val(&val)
    }
    pub fn iter(&mut self) -> Vec<T> {
        let mut result = Vec::new();
        let mut count = 0;
        let mut i = 0;
        let buckets = self.buckets();
        while count < self.bucket_size {
            let pi = buckets[i].pi;
            if pi != 0 && pi != -1 {
                result.push(buckets[i].elem);
                count += 1;
            }
            i += 1;
        }
        result
    }
    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }
    pub fn clear(&mut self) {
        let buckets = self.buckets_mut();
        buckets.clear();
        self.bucket_size = 0;
        let buckets = self.buckets_mut();
        for _ in 0..CSET_INITIAL_CAP {
            buckets.push(CsetValue { pi: 0, elem: T::default() });
        }
    }
    pub fn intersect(&mut self, first: &Self, second: &Self) {
        let cap = first.buckets().len();
        for i in 0..cap {
            let pi = first.buckets()[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let val = first.buckets()[i].elem;
            if self.contains_in(second, &val) {
                self.add(val);
            }
        }
    }
    pub fn union(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets().len() {
            let pi = first.buckets()[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            self.add(first.buckets()[i].elem);
        }
        for i in 0..second.buckets().len() {
            let pi = second.buckets()[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            self.add(second.buckets()[i].elem);
        }
    }
    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        let cap = self.buckets().len();
        for i in 0..cap {
            let pi = self.buckets()[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let val = self.buckets()[i].elem;
            if self.contains_in(other, &val) {
                return false;
            }
        }
        true
    }
    pub fn difference(&mut self, first: &Self, second: &Self) {
        let cap = first.buckets().len();
        for i in 0..cap {
            let pi = first.buckets()[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let val = first.buckets()[i].elem;
            if !self.contains_in(second, &val) {
                self.add(val);
            }
        }
    }

    fn contains_val(&self, val: &T) -> bool {
        let h1 = self.h1hash(val);
        let h2 = self.h2hash(val);
        let cap = self.buckets().len();
        let mut iteration = 1usize;
        for _ in 0..cap {
            let index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let buckets = self.buckets();
            if buckets[index].pi == -1 {
                continue;
            }
            if buckets[index].pi == 0 {
                return false;
            }
            if self.matches(&buckets[index].elem, val) {
                return true;
            }
        }
        false
    }

    fn contains_in(&self, other: &Self, val: &T) -> bool {
        let h1 = self.h1hash(val);
        let h2 = self.h2hash(val);
        let cap = other.buckets().len();
        let mut iteration = 1usize;
        for _ in 0..cap {
            let index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let buckets = other.buckets();
            if buckets[index].pi == -1 {
                continue;
            }
            if buckets[index].pi == 0 {
                return false;
            }
            if self.matches(&buckets[index].elem, val) {
                return true;
            }
        }
        false
    }
}
