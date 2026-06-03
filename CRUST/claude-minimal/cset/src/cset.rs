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
fn xxh_rotl64(x: u64, r: u32) -> u64 {
    x.rotate_left(r)
}

#[inline]
unsafe fn read_le64_raw(p: *const u8) -> u64 {
    let b0 = *p as u64;
    let b1 = *p.add(1) as u64;
    let b2 = *p.add(2) as u64;
    let b3 = *p.add(3) as u64;
    let b4 = *p.add(4) as u64;
    let b5 = *p.add(5) as u64;
    let b6 = *p.add(6) as u64;
    let b7 = *p.add(7) as u64;
    b0 | (b1 << 8)
        | (b2 << 16)
        | (b3 << 24)
        | (b4 << 32)
        | (b5 << 40)
        | (b6 << 48)
        | (b7 << 56)
}

#[inline]
unsafe fn read_le32_raw(p: *const u8) -> u32 {
    let b0 = *p as u32;
    let b1 = *p.add(1) as u32;
    let b2 = *p.add(2) as u32;
    let b3 = *p.add(3) as u32;
    b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
}

// Function Definitions
pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}
pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    unsafe { read_le64_raw(mem_ptr as *const u8) }
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
    unsafe { ptr::read_unaligned(mem_ptr as *const u32) }
}
pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = xxh_rotl64(acc, 31);
    acc = acc.wrapping_mul(XXH_PRIME64_1);
    acc
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
    let raw = unsafe { ptr::read_unaligned(ptr as *const u32) };
    if xxh_is_little_endian() {
        raw
    } else {
        let mut v = raw;
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

unsafe fn xxh64_finalize_raw(mut h64: u64, mut ptr: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round(0, read_le64_raw(ptr));
        ptr = ptr.add(8);
        h64 ^= k1;
        h64 = xxh_rotl64(h64, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_le32_raw(ptr) as u64).wrapping_mul(XXH_PRIME64_1);
        ptr = ptr.add(4);
        h64 = xxh_rotl64(h64, 23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (*ptr as u64).wrapping_mul(XXH_PRIME64_5);
        ptr = ptr.add(1);
        h64 = xxh_rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

pub fn xxh64_finalize(h64: XXHU64, ptr: &mut XXHU8, len: usize) -> XXHU64 {
    unsafe { xxh64_finalize_raw(h64, ptr as *const u8, len) }
}

unsafe fn xxh64_endian_align_raw(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let b_end = if !input.is_null() {
        input.add(len)
    } else {
        ptr::null()
    };
    let mut h64;

    if len >= 32 {
        let limit = b_end.sub(32);
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_le64_raw(p));
            p = p.add(8);
            v2 = xxh64_round(v2, read_le64_raw(p));
            p = p.add(8);
            v3 = xxh64_round(v3, read_le64_raw(p));
            p = p.add(8);
            v4 = xxh64_round(v4, read_le64_raw(p));
            p = p.add(8);
            if p > limit {
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
    let remaining = len & 31;
    xxh64_finalize_raw(h64, p, remaining)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe { xxh64_endian_align_raw(input as *const u8, len, seed) }
}

unsafe fn xxh64_endian_align_h_raw(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let b_end = if !input.is_null() {
        input.add(len)
    } else {
        ptr::null()
    };
    let mut h64;

    if len >= 32 {
        let limit = b_end.sub(32);
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_le64_raw(p));
            p = p.add(8);
            v2 = xxh64_round(v2, read_le64_raw(p));
            p = p.add(8);
            v3 = xxh64_round(v3, read_le64_raw(p));
            p = p.add(8);
            v4 = xxh64_round(v4, read_le64_raw(p));
            p = p.add(8);
            if p > limit {
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
    let remaining = len & 31;
    xxh64_finalize_raw(h64, p, remaining)
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe { xxh64_endian_align_h_raw(input as *const u8, len, seed) }
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe { xxh64_endian_align_raw(input, len, seed) }
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe { xxh64_endian_align_h_raw(input, len, seed) }
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64(memptr as *const u8, size, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64_h(memptr as *const u8, size, CSET_DEFAULT_SEED) | 1
}

pub struct CsetValue<T> {
    pi: i32,
    elem: Option<T>,
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
    for _ in 0..cap {
        v.push(CsetValue { pi: 0, elem: None });
    }
    v
}

#[inline]
fn double_hash_index(h1: u64, h2: u64, i: u64, cap: u64) -> usize {
    (h1.wrapping_add(i.wrapping_mul(h2)) % cap) as usize
}

impl<T> Cset<T> {
    pub fn new() -> Cset<T> {
        Cset {
            buckets: make_empty_buckets(CSET_INITIAL_CAP),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: CsetValue { pi: 0, elem: None },
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
        self.temp_buckets = Vec::new();
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
        self.buckets[index].elem.as_ref().unwrap().clone()
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
        // Match the (unusual) signature in the scaffolding.
        // SAFETY: We expose interior mutability for parity with the C macros.
        unsafe {
            let p = &self.buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>;
            &mut *p
        }
    }

    #[allow(invalid_reference_casting)]
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe {
            let p = &self.temp_buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>;
            &mut *p
        }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    fn hash1(&self, value: &T) -> u64 {
        let p = value as *const T as *const u8;
        let len = mem::size_of::<T>();
        xxh64(p, len, self.seed)
    }

    fn hash2(&self, value: &T) -> u64 {
        let p = value as *const T as *const u8;
        let len = mem::size_of::<T>();
        xxh64_h(p, len, self.seed) | 1
    }

    fn matches(&self, a: &T, b: &T) -> bool {
        if let Some(cmp) = self.compare {
            cmp(a, b)
        } else {
            let pa = a as *const T as *const u8;
            let pb = b as *const T as *const u8;
            let len = mem::size_of::<T>();
            unsafe {
                std::slice::from_raw_parts(pa, len) == std::slice::from_raw_parts(pb, len)
            }
        }
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let factor = if cap == 0 {
            1.0
        } else {
            self.bucket_size as f64 / cap as f64
        };
        if factor >= self.max_load_factor {
            let new_cap = if cap == 0 { CSET_INITIAL_CAP } else { cap * 2 };
            self.resize(new_cap);
        }
        self.add_internal(value);
        0
    }

    fn add_internal(&mut self, value: T) {
        let cap = self.buckets.len();
        let h1 = self.hash1(&value);
        let h2 = self.hash2(&value);
        let mut iteration: u64 = 1;
        let mut index: usize = 0;
        let mut found = false;
        loop {
            index = double_hash_index(h1, h2, iteration - 1, cap as u64);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == 0 || pi == -1 {
                break;
            }
            // occupied - check match
            let elem_matches = {
                let elem = self.buckets[index].elem.as_ref().unwrap();
                self.matches(elem, &value)
            };
            if elem_matches {
                found = true;
                break;
            }
            if (iteration - 1) as usize >= cap {
                // No empty/tombstone slot found; safety break.
                break;
            }
        }
        if !found {
            self.buckets[index].elem = Some(value);
            self.buckets[index].pi = iteration as i32;
            self.bucket_size += 1;
        }
    }

    fn resize(&mut self, new_cap: usize) {
        let old_buckets = mem::replace(&mut self.buckets, make_empty_buckets(new_cap));
        self.bucket_size = 0;
        for bucket in old_buckets.into_iter() {
            if bucket.pi > 0 {
                if let Some(elem) = bucket.elem {
                    self.add_internal(elem);
                }
            }
        }
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        if cap == 0 {
            return 0;
        }
        let h1 = self.hash1(&value);
        let h2 = self.hash2(&value);
        let mut iteration: u64 = 1;
        let mut found_index: Option<usize> = None;
        loop {
            if (iteration - 1) as usize >= cap {
                break;
            }
            let index = double_hash_index(h1, h2, iteration - 1, cap as u64);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                break;
            }
            let elem_matches = {
                let elem = self.buckets[index].elem.as_ref().unwrap();
                self.matches(elem, &value)
            };
            if elem_matches {
                found_index = Some(index);
                break;
            }
        }
        if let Some(index) = found_index {
            self.buckets[index].pi = -1;
            self.buckets[index].elem = None;
            self.bucket_size -= 1;
        }
        0
    }

    fn contains_inner(&self, value: &T) -> bool {
        let cap = self.buckets.len();
        if cap == 0 {
            return false;
        }
        let h1 = self.hash1(value);
        let h2 = self.hash2(value);
        let mut iteration: u64 = 1;
        loop {
            if (iteration - 1) as usize >= cap {
                return false;
            }
            let index = double_hash_index(h1, h2, iteration - 1, cap as u64);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                return false;
            }
            let elem = self.buckets[index].elem.as_ref().unwrap();
            if self.matches(elem, value) {
                return true;
            }
        }
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.contains_inner(value)
    }

    pub fn iter(&mut self) -> Vec<T>
    where
        T: Clone,
    {
        let mut result = Vec::with_capacity(self.bucket_size);
        for bucket in &self.buckets {
            if bucket.pi > 0 {
                if let Some(elem) = &bucket.elem {
                    result.push(elem.clone());
                }
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

    pub fn intersect(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for bucket in &first.buckets {
            if bucket.pi > 0 {
                if let Some(elem) = &bucket.elem {
                    if second.contains_inner(elem) {
                        self.add(elem.clone());
                    }
                }
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for bucket in &first.buckets {
            if bucket.pi > 0 {
                if let Some(elem) = &bucket.elem {
                    self.add(elem.clone());
                }
            }
        }
        for bucket in &second.buckets {
            if bucket.pi > 0 {
                if let Some(elem) = &bucket.elem {
                    self.add(elem.clone());
                }
            }
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for bucket in &self.buckets {
            if bucket.pi > 0 {
                if let Some(elem) = &bucket.elem {
                    if other.contains_inner(elem) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn difference(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for bucket in &first.buckets {
            if bucket.pi > 0 {
                if let Some(elem) = &bucket.elem {
                    if !second.contains_inner(elem) {
                        self.add(elem.clone());
                    }
                }
            }
        }
    }
}
