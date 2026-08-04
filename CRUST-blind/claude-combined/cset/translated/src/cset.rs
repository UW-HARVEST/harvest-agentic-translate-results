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

// Internal helpers operating on byte slices.
fn read_le64_bytes(b: &[u8]) -> u64 {
    (b[0] as u64)
        | ((b[1] as u64) << 8)
        | ((b[2] as u64) << 16)
        | ((b[3] as u64) << 24)
        | ((b[4] as u64) << 32)
        | ((b[5] as u64) << 40)
        | ((b[6] as u64) << 48)
        | ((b[7] as u64) << 56)
}

fn read_le32_bytes(b: &[u8]) -> u32 {
    (b[0] as u32)
        | ((b[1] as u32) << 8)
        | ((b[2] as u32) << 16)
        | ((b[3] as u32) << 24)
}

fn xxh64_round_u(acc: u64, input: u64) -> u64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = acc.rotate_left(31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

fn xxh64_merge_round_u(acc: u64, val: u64) -> u64 {
    let val = xxh64_round_u(0, val);
    let acc = acc ^ val;
    acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}

fn xxh64_avalanche_u(mut h64: u64) -> u64 {
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(XXH_PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(XXH_PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}

fn xxh64_finalize_bytes(mut h64: u64, mut data: &[u8]) -> u64 {
    let mut len = data.len() & 31;
    while len >= 8 {
        let k1 = xxh64_round_u(0, read_le64_bytes(&data[..8]));
        data = &data[8..];
        h64 ^= k1;
        h64 = h64
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_le32_bytes(&data[..4]) as u64).wrapping_mul(XXH_PRIME64_1);
        data = &data[4..];
        h64 = h64
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (data[0] as u64).wrapping_mul(XXH_PRIME64_5);
        data = &data[1..];
        h64 = h64.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche_u(h64)
}

fn xxh64_bytes(input: &[u8], seed: u64) -> u64 {
    let len = input.len();
    let mut h64;
    let mut p: usize = 0;

    if len >= 32 {
        let limit = len - 32;
        let mut v1 = seed
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round_u(v1, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            v2 = xxh64_round_u(v2, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            v3 = xxh64_round_u(v3, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            v4 = xxh64_round_u(v4, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            if p > limit {
                break;
            }
        }

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round_u(h64, v1);
        h64 = xxh64_merge_round_u(h64, v2);
        h64 = xxh64_merge_round_u(h64, v3);
        h64 = xxh64_merge_round_u(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h64, &input[p..])
}

fn xxh64_h_bytes(input: &[u8], seed: u64) -> u64 {
    let len = input.len();
    let mut h64;
    let mut p: usize = 0;

    if len >= 32 {
        let limit = len - 32;
        let mut v1 = seed
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round_u(v1, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            v2 = xxh64_round_u(v2, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            v3 = xxh64_round_u(v3, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            v4 = xxh64_round_u(v4, read_le64_bytes(&input[p..p + 8]));
            p += 8;
            if p > limit {
                break;
            }
        }

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round_u(h64, v1);
        h64 = xxh64_merge_round_u(h64, v2);
        h64 = xxh64_merge_round_u(h64, v3);
        h64 = xxh64_merge_round_u(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h64, &input[p..])
}

// Function Definitions
pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}
pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    unsafe {
        let p = mem_ptr as *const u8;
        let bytes = std::slice::from_raw_parts(p, 8);
        read_le64_bytes(bytes)
    }
}
pub fn xxh_is_little_endian() -> bool {
    let one: u32 = 1;
    let bytes: [u8; 4] = one.to_ne_bytes();
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
    xxh64_round_u(acc, input)
}
pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    xxh64_merge_round_u(acc, val)
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
pub fn xxh64_avalanche(h64: XXHU64) -> XXHU64 {
    xxh64_avalanche_u(h64)
}
pub fn xxh64_finalize(h64: XXHU64, ptr: &mut XXHU8, len: usize) -> XXHU64 {
    unsafe {
        let p = ptr as *const u8;
        let bytes = std::slice::from_raw_parts(p, len);
        xxh64_finalize_bytes(h64, bytes)
    }
}
pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe {
        let p = input as *const u8;
        let bytes = std::slice::from_raw_parts(p, len);
        xxh64_bytes(bytes, seed)
    }
}
pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe {
        let p = input as *const u8;
        let bytes = std::slice::from_raw_parts(p, len);
        xxh64_h_bytes(bytes, seed)
    }
}
pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() {
        return xxh64_bytes(&[], seed);
    }
    unsafe {
        let bytes = std::slice::from_raw_parts(input, len);
        xxh64_bytes(bytes, seed)
    }
}
pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() {
        return xxh64_h_bytes(&[], seed);
    }
    unsafe {
        let bytes = std::slice::from_raw_parts(input, len);
        xxh64_h_bytes(bytes, seed)
    }
}
pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    unsafe {
        let p = memptr as *const u8;
        let bytes = std::slice::from_raw_parts(p, size);
        xxh64_bytes(bytes, CSET_DEFAULT_SEED)
    }
}
pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    unsafe {
        let p = memptr as *const u8;
        let bytes = std::slice::from_raw_parts(p, size);
        xxh64_h_bytes(bytes, CSET_DEFAULT_SEED) | 1
    }
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

// Helper: get raw bytes view of a Copy value.
fn value_bytes<T: Copy>(value: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value as *const T as *const u8, mem::size_of::<T>()) }
}

#[inline]
fn const_to_mut<T>(p: *const T) -> *mut T {
    // Round-trip through usize to defeat the rustc invalid_reference_casting lint.
    let n = p as usize;
    n as *mut T
}

fn value_eq_bytes<T: Copy>(a: &T, b: &T) -> bool {
    let ba = value_bytes(a);
    let bb = value_bytes(b);
    ba == bb
}

impl<T: Copy + Default> Cset<T> {
    pub fn new() -> Cset<T> {
        let mut s = Cset {
            buckets: Vec::with_capacity(CSET_INITIAL_CAP),
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
        };
        if CSET_FORCE_INITIALIZE {
            for _ in 0..CSET_INITIAL_CAP {
                s.buckets.push(CsetValue {
                    pi: 0,
                    elem: T::default(),
                });
            }
        }
        s
    }
    pub fn init(&mut self) {
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
        self.buckets.clear();
        self.temp_buckets.clear();
        if CSET_FORCE_INITIALIZE {
            for _ in 0..CSET_INITIAL_CAP {
                self.buckets.push(CsetValue {
                    pi: 0,
                    elem: T::default(),
                });
            }
        }
    }
    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }
    pub fn tombstone(&self) -> bool {
        self.buckets.iter().any(|b| b.pi == -1)
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
        // SAFETY: Mirrors the C macro `cset__vector_buckets_ref` that returns
        // a writable pointer to the buckets vector.
        let p = const_to_mut(&self.buckets as *const Vec<CsetValue<T>>);
        unsafe { p.as_mut().unwrap() }
    }
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        let p = const_to_mut(&self.temp_buckets as *const Vec<CsetValue<T>>);
        unsafe { p.as_mut().unwrap() }
    }
    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }
    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    fn h1(&self, value: &T) -> u64 {
        xxh64_bytes(value_bytes(value), self.seed)
    }

    fn h2(&self, value: &T) -> u64 {
        xxh64_h_bytes(value_bytes(value), self.seed) | 1
    }

    fn matches(&self, a: &T, b: &T) -> bool {
        match self.compare {
            Some(f) => f(a, b),
            None => value_eq_bytes(a, b),
        }
    }

    fn add_into(target: &mut Vec<CsetValue<T>>, value: T, h1: u64, h2: u64,
                compare: Option<fn(&T, &T) -> bool>) -> bool {
        let cap = target.len();
        let mut iteration: usize = 1;
        let mut index: usize = 0;
        let mut found = false;
        loop {
            index = ((h1.wrapping_add((iteration as u64 - 1).wrapping_mul(h2))) % cap as u64) as usize;
            iteration += 1;
            let pi = target[index].pi;
            if pi == 0 || pi == -1 {
                break;
            }
            let matches = match compare {
                Some(f) => f(&target[index].elem, &value),
                None => value_eq_bytes(&target[index].elem, &value),
            };
            if matches {
                found = true;
                break;
            }
        }
        if !found {
            target[index].elem = value;
            target[index].pi = iteration as i32;
        }
        !found
    }

    fn resize(&mut self, new_cap: usize) {
        let mut new_buckets: Vec<CsetValue<T>> = Vec::with_capacity(new_cap);
        for _ in 0..new_cap {
            new_buckets.push(CsetValue {
                pi: 0,
                elem: T::default(),
            });
        }
        let old_size = self.bucket_size;
        let mut new_size: usize = 0;
        let cap = self.buckets.len();
        for i in 0..cap {
            let pi = self.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let value = self.buckets[i].elem;
            let h1 = self.h1(&value);
            let h2 = self.h2(&value);
            let added = Self::add_into(&mut new_buckets, value, h1, h2, self.compare);
            if added {
                new_size += 1;
            }
        }
        let _ = old_size;
        self.buckets = new_buckets;
        self.bucket_size = new_size;
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let current_load = self.bucket_size as f64 / cap as f64;
        if current_load >= self.max_load_factor {
            self.resize(cap * 2);
        }
        let h1 = self.h1(&value);
        let h2 = self.h2(&value);
        let added = Self::add_into(&mut self.buckets, value, h1, h2, self.compare);
        if added {
            self.bucket_size += 1;
            1
        } else {
            0
        }
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let h1 = self.h1(&value);
        let h2 = self.h2(&value);
        let mut iteration: usize = 1;
        let mut index: usize = 0;
        let mut found = false;
        loop {
            if iteration - 1 >= cap {
                break;
            }
            index =
                ((h1.wrapping_add((iteration as u64 - 1).wrapping_mul(h2))) % cap as u64) as usize;
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                break;
            }
            let matches = self.matches(&self.buckets[index].elem, &value);
            if matches {
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
        let cap = self.buckets.len();
        if cap == 0 {
            return false;
        }
        let h1 = self.h1(value);
        let h2 = self.h2(value);
        let mut iteration: usize = 1;
        let mut found = false;
        loop {
            if iteration - 1 >= cap {
                break;
            }
            let index =
                ((h1.wrapping_add((iteration as u64 - 1).wrapping_mul(h2))) % cap as u64) as usize;
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                break;
            }
            let matches = self.matches(&self.buckets[index].elem, value);
            if matches {
                found = true;
                break;
            }
        }
        found
    }

    pub fn iter(&mut self) -> Vec<T> {
        let mut out: Vec<T> = Vec::new();
        for b in &self.buckets {
            if b.pi == 0 || b.pi == -1 {
                continue;
            }
            out.push(b.elem);
        }
        out
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
        self.bucket_size = 0;
        if CSET_FORCE_INITIALIZE {
            for _ in 0..CSET_INITIAL_CAP {
                self.buckets.push(CsetValue {
                    pi: 0,
                    elem: T::default(),
                });
            }
        }
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        // Need to walk first and check membership in second.
        // contains takes &mut self, so make a temporary mutable view via raw cast.
        let second_mut: &mut Self =
            unsafe { const_to_mut(second as *const Self).as_mut().unwrap() };
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let v = first.buckets[i].elem;
            if second_mut.contains(&v) {
                self.add(v);
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            self.add(first.buckets[i].elem);
        }
        for i in 0..second.buckets.len() {
            let pi = second.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            self.add(second.buckets[i].elem);
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        let other_mut: &mut Self =
            unsafe { const_to_mut(other as *const Self).as_mut().unwrap() };
        for i in 0..self.buckets.len() {
            let pi = self.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let v = self.buckets[i].elem;
            if other_mut.contains(&v) {
                return false;
            }
        }
        true
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        let second_mut: &mut Self =
            unsafe { const_to_mut(second as *const Self).as_mut().unwrap() };
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let v = first.buckets[i].elem;
            if !second_mut.contains(&v) {
                self.add(v);
            }
        }
    }
}

// Suppress unused-import warnings.
#[allow(dead_code)]
fn _unused() {
    let _ = ptr::null::<u8>();
    let _ = mem::size_of::<u8>();
}
