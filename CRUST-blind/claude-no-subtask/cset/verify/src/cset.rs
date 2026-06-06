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

// ---- internal helpers ---------------------------------------------------

fn read_le64_ptr(ptr: *const u8) -> u64 {
    unsafe {
        let mut bytes = [0u8; 8];
        std::ptr::copy_nonoverlapping(ptr, bytes.as_mut_ptr(), 8);
        u64::from_le_bytes(bytes)
    }
}

fn read_le32_ptr(ptr: *const u8) -> u32 {
    unsafe {
        let mut bytes = [0u8; 4];
        std::ptr::copy_nonoverlapping(ptr, bytes.as_mut_ptr(), 4);
        u32::from_le_bytes(bytes)
    }
}

fn xxh64_finalize_internal(mut h64: u64, mut ptr: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round(0, read_le64_ptr(ptr));
        ptr = unsafe { ptr.add(8) };
        h64 ^= k1;
        h64 = h64
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_le32_ptr(ptr) as u64).wrapping_mul(XXH_PRIME64_1);
        ptr = unsafe { ptr.add(4) };
        h64 = h64
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        let byte = unsafe { *ptr };
        ptr = unsafe { ptr.add(1) };
        h64 ^= (byte as u64).wrapping_mul(XXH_PRIME64_5);
        h64 = h64.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

fn xxh64_endian_align_internal(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let b_end: *const u8 = if !input.is_null() {
        unsafe { input.add(len) }
    } else {
        ptr::null()
    };
    let mut h64: u64;

    if len >= 32 {
        let limit = unsafe { b_end.sub(32) };
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            v2 = xxh64_round(v2, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            v3 = xxh64_round(v3, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            v4 = xxh64_round(v4, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            if p > limit {
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

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_internal(h64, p, len)
}

fn xxh64_endian_align_h_internal(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let b_end: *const u8 = if !input.is_null() {
        unsafe { input.add(len) }
    } else {
        ptr::null()
    };
    let mut h64: u64;

    if len >= 32 {
        let limit = unsafe { b_end.sub(32) };
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            v2 = xxh64_round(v2, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            v3 = xxh64_round(v3, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            v4 = xxh64_round(v4, read_le64_ptr(p));
            p = unsafe { p.add(8) };
            if p > limit {
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

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_internal(h64, p, len)
}

fn h1hash_of<T>(value: &T, seed: u64) -> u64 {
    let p = value as *const T as *const u8;
    let len = mem::size_of::<T>();
    xxh64(p, len, seed)
}

fn h2hash_of<T>(value: &T, seed: u64) -> u64 {
    let p = value as *const T as *const u8;
    let len = mem::size_of::<T>();
    xxh64_h(p, len, seed) | 1
}

fn bytes_eq<T>(a: &T, b: &T) -> bool {
    let pa = a as *const T as *const u8;
    let pb = b as *const T as *const u8;
    let len = mem::size_of::<T>();
    unsafe {
        let sa = std::slice::from_raw_parts(pa, len);
        let sb = std::slice::from_raw_parts(pb, len);
        sa == sb
    }
}

fn matches_check<T>(compare: &Option<fn(&T, &T) -> bool>, a: &T, b: &T) -> bool {
    match compare {
        Some(cmp) => cmp(a, b),
        None => bytes_eq(a, b),
    }
}

fn make_empty_buckets<T: Default>(cap: usize) -> Vec<CsetValue<T>> {
    let mut buckets = Vec::with_capacity(cap);
    for _ in 0..cap {
        buckets.push(CsetValue {
            pi: 0,
            elem: T::default(),
        });
    }
    buckets
}

// Insert a value into a buckets array using double-hash probing.
// Returns true if the value was newly inserted, false if it was already present.
fn insert_into_buckets<T>(
    buckets: &mut Vec<CsetValue<T>>,
    bucket_size: &mut usize,
    value: T,
    seed: u64,
    compare: &Option<fn(&T, &T) -> bool>,
) -> bool {
    let cap = buckets.len();
    if cap == 0 {
        return false;
    }
    let h1 = h1hash_of(&value, seed);
    let h2 = h2hash_of(&value, seed);
    let mut iteration: u64 = 1;
    let mut index: usize;
    let mut found = false;

    loop {
        index = (h1.wrapping_add((iteration - 1).wrapping_mul(h2)) as usize) % cap;
        iteration = iteration.wrapping_add(1);
        let pi = buckets[index].pi;
        if pi == 0 || pi == -1 {
            break;
        }
        if matches_check(compare, &buckets[index].elem, &value) {
            found = true;
            break;
        }
    }

    if !found {
        buckets[index].elem = value;
        buckets[index].pi = iteration as i32;
        *bucket_size += 1;
        true
    } else {
        false
    }
}

// ---- public xxhash function definitions ---------------------------------

pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}

pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    let p = mem_ptr as *const u8;
    read_le64_ptr(p)
}

pub fn xxh_is_little_endian() -> bool {
    cfg!(target_endian = "little")
}

pub fn xxh_read_le64_align(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64(mem_ptr)
}

pub fn xxh_swap32(x: &mut XXHU32) -> XXHU32 {
    let val = *x;
    ((val << 24) & 0xff000000)
        | ((val << 8) & 0x00ff0000)
        | ((val >> 8) & 0x0000ff00)
        | ((val >> 24) & 0x000000ff)
}

pub fn xxh_read32(mem_ptr: &mut XXHU32) -> XXHU32 {
    *mem_ptr
}

pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    let acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    let acc = acc.rotate_left(31);
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
        let mut v = *ptr;
        xxh_swap32(&mut v)
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
    let p = ptr as *const u8;
    xxh64_finalize_internal(h64, p, len)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let p = input as *const u8;
    xxh64_endian_align_internal(p, len, seed)
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let p = input as *const u8;
    xxh64_endian_align_h_internal(p, len, seed)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    xxh64_endian_align_internal(input, len, seed)
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    xxh64_endian_align_h_internal(input, len, seed)
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let p = memptr as *const u8;
    xxh64(p, size, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let p = memptr as *const u8;
    xxh64_h(p, size, CSET_DEFAULT_SEED) | 1
}

// ---- Cset struct and impl ----------------------------------------------

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

impl<T: Clone + Default> Cset<T> {
    pub fn new() -> Cset<T> {
        Cset {
            buckets: make_empty_buckets::<T>(CSET_INITIAL_CAP),
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
        self.buckets = make_empty_buckets::<T>(CSET_INITIAL_CAP);
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

    #[allow(invalid_reference_casting)]
    pub fn get_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        // The desired signature returns a mutable reference from a shared
        // self reference. Use an unsafe cast to satisfy the contract.
        let ptr = &self.buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>;
        unsafe { &mut *ptr }
    }

    #[allow(invalid_reference_casting)]
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        let ptr = &self.temp_buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>;
        unsafe { &mut *ptr }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    fn resize(&mut self, new_cap: usize) {
        let current_cap = self.buckets.len();
        let mut new_buckets: Vec<CsetValue<T>> = make_empty_buckets::<T>(new_cap);
        let mut new_size: usize = 0;
        let seed = self.seed;
        let compare = self.compare;

        for i in 0..current_cap {
            let pi = self.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let value = self.buckets[i].elem.clone();
            insert_into_buckets(&mut new_buckets, &mut new_size, value, seed, &compare);
        }

        self.buckets = new_buckets;
        self.bucket_size = new_size;
        self.temp_buckets = Vec::new();
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        if cap == 0 {
            self.resize(CSET_INITIAL_CAP);
        } else {
            let lf = (self.bucket_size as f64) / (cap as f64);
            if lf >= self.max_load_factor {
                self.resize(cap * 2);
            }
        }

        let seed = self.seed;
        let compare = self.compare;
        let inserted =
            insert_into_buckets(&mut self.buckets, &mut self.bucket_size, value, seed, &compare);
        if inserted {
            1
        } else {
            0
        }
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        if cap == 0 {
            return 0;
        }
        let h1 = h1hash_of(&value, self.seed);
        let h2 = h2hash_of(&value, self.seed);
        let mut iteration: u64 = 1;
        let mut found = false;
        let mut found_index: usize = 0;

        loop {
            if (iteration - 1) as usize >= cap {
                break;
            }
            let index = (h1.wrapping_add((iteration - 1).wrapping_mul(h2)) as usize) % cap;
            iteration = iteration.wrapping_add(1);
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                break;
            }
            if matches_check(&self.compare, &self.buckets[index].elem, &value) {
                found = true;
                found_index = index;
                break;
            }
        }

        if found {
            self.buckets[found_index].pi = -1;
            if self.bucket_size > 0 {
                self.bucket_size -= 1;
            }
            1
        } else {
            0
        }
    }

    fn contains_internal(&self, value: &T) -> bool {
        let cap = self.buckets.len();
        if cap == 0 {
            return false;
        }
        let h1 = h1hash_of(value, self.seed);
        let h2 = h2hash_of(value, self.seed);
        let mut iteration: u64 = 1;

        loop {
            if (iteration - 1) as usize >= cap {
                return false;
            }
            let index = (h1.wrapping_add((iteration - 1).wrapping_mul(h2)) as usize) % cap;
            iteration = iteration.wrapping_add(1);
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                return false;
            }
            if matches_check(&self.compare, &self.buckets[index].elem, value) {
                return true;
            }
        }
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.contains_internal(value)
    }

    pub fn iter(&mut self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.bucket_size);
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
        self.buckets = make_empty_buckets::<T>(CSET_INITIAL_CAP);
        self.bucket_size = 0;
        self.temp_buckets = Vec::new();
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem = first.buckets[i].elem.clone();
            if second.contains_internal(&elem) {
                self.add(elem);
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem = first.buckets[i].elem.clone();
            self.add(elem);
        }
        for i in 0..second.buckets.len() {
            let pi = second.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem = second.buckets[i].elem.clone();
            self.add(elem);
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for i in 0..self.buckets.len() {
            let pi = self.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem = self.buckets[i].elem.clone();
            if other.contains_internal(&elem) {
                return false;
            }
        }
        true
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        for i in 0..first.buckets.len() {
            let pi = first.buckets[i].pi;
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem = first.buckets[i].elem.clone();
            if !second.contains_internal(&elem) {
                self.add(elem);
            }
        }
    }
}
