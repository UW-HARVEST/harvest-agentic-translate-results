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

// Internal pointer-based helpers used by the public xxhash functions.
#[inline]
unsafe fn read_le64_ptr(p: *const u8) -> u64 {
    let mut result: u64 = 0;
    let mut i = 0usize;
    while i < 8 {
        result |= (*p.add(i) as u64) << (i * 8);
        i += 1;
    }
    result
}

#[inline]
unsafe fn read_le32_ptr(p: *const u8) -> u32 {
    let val: u32 = ptr::read_unaligned(p as *const u32);
    if cfg!(target_endian = "little") {
        val
    } else {
        ((val << 24) & 0xff000000)
            | ((val << 8) & 0x00ff0000)
            | ((val >> 8) & 0x0000ff00)
            | ((val >> 24) & 0x000000ff)
    }
}

// Function Definitions
pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}
pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    unsafe { read_le64_ptr(mem_ptr as *const u8) }
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
    unsafe { ptr::read_unaligned(mem_ptr as *const u32) }
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
        xxh_read32(ptr)
    } else {
        let mut val = xxh_read32(ptr);
        xxh_swap32(&mut val)
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
    unsafe { xxh64_finalize_internal(h64, ptr as *const u8, len) }
}

unsafe fn xxh64_finalize_internal(mut h64: u64, mut p: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round(0, read_le64_ptr(p));
        p = p.add(8);
        h64 ^= k1;
        h64 = h64
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        let val = read_le32_ptr(p) as u64;
        h64 ^= val.wrapping_mul(XXH_PRIME64_1);
        p = p.add(4);
        h64 = h64
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (*p as u64).wrapping_mul(XXH_PRIME64_5);
        p = p.add(1);
        h64 = h64.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe { xxh64_endian_align_internal(input as *const u8, len, seed) }
}

unsafe fn xxh64_endian_align_internal(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let mut h64: u64;
    if len >= 32 {
        let limit = input.add(len).offset(-32);
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        loop {
            v1 = xxh64_round(v1, read_le64_ptr(p));
            p = p.add(8);
            v2 = xxh64_round(v2, read_le64_ptr(p));
            p = p.add(8);
            v3 = xxh64_round(v3, read_le64_ptr(p));
            p = p.add(8);
            v4 = xxh64_round(v4, read_le64_ptr(p));
            p = p.add(8);
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
    let consumed = p as usize - input as usize;
    let remaining = len - consumed;
    xxh64_finalize_internal(h64, p, remaining)
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe { xxh64_endian_align_h_internal(input as *const u8, len, seed) }
}

unsafe fn xxh64_endian_align_h_internal(input: *const u8, len: usize, seed: u64) -> u64 {
    let mut p = input;
    let mut h64: u64;
    if len >= 32 {
        let limit = input.add(len).offset(-32);
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        loop {
            v1 = xxh64_round(v1, read_le64_ptr(p));
            p = p.add(8);
            v2 = xxh64_round(v2, read_le64_ptr(p));
            p = p.add(8);
            v3 = xxh64_round(v3, read_le64_ptr(p));
            p = p.add(8);
            v4 = xxh64_round(v4, read_le64_ptr(p));
            p = p.add(8);
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
    let consumed = p as usize - input as usize;
    let remaining = len - consumed;
    xxh64_finalize_internal(h64, p, remaining)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe { xxh64_endian_align_internal(input, len, seed) }
}
pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe { xxh64_endian_align_h_internal(input, len, seed) }
}
pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64(memptr as *const u8, size, CSET_DEFAULT_SEED)
}
pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64_h(memptr as *const u8, size, CSET_DEFAULT_SEED) | 1
}
pub struct  CsetValue<T> {
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
    fn empty_value() -> CsetValue<T> {
        // The bucket's elem field is treated as scratch space until a real
        // value is written into it via `ptr::write`. We use zeroed memory to
        // mirror the C version's malloc'd storage with `pi = 0`.
        CsetValue {
            pi: 0,
            elem: unsafe { mem::MaybeUninit::<T>::zeroed().assume_init() },
        }
    }

    fn empty_buckets(cap: usize) -> Vec<CsetValue<T>> {
        let mut v: Vec<CsetValue<T>> = Vec::with_capacity(cap);
        for _ in 0..cap {
            v.push(Self::empty_value());
        }
        v
    }

    fn h1hash_for(value: &T, seed: u64) -> u64 {
        xxh64(value as *const T as *const u8, mem::size_of::<T>(), seed)
    }

    fn h2hash_for(value: &T, seed: u64) -> u64 {
        xxh64_h(value as *const T as *const u8, mem::size_of::<T>(), seed) | 1
    }

    fn bytes_compare(a: &T, b: &T) -> bool {
        let size = mem::size_of::<T>();
        let a_bytes =
            unsafe { std::slice::from_raw_parts(a as *const T as *const u8, size) };
        let b_bytes =
            unsafe { std::slice::from_raw_parts(b as *const T as *const u8, size) };
        a_bytes == b_bytes
    }

    fn matches_at(
        buckets: &[CsetValue<T>],
        compare: Option<fn(&T, &T) -> bool>,
        ref_val: &T,
        index: usize,
    ) -> bool {
        if let Some(cmp) = compare {
            cmp(&buckets[index].elem, ref_val)
        } else {
            Self::bytes_compare(&buckets[index].elem, ref_val)
        }
    }

    fn double_hash_index(h1: u64, h2: u64, i: usize, cap: usize) -> usize {
        (h1.wrapping_add((i as u64).wrapping_mul(h2)) % (cap as u64)) as usize
    }

    fn add_to_buckets(
        buckets: &mut Vec<CsetValue<T>>,
        compare: Option<fn(&T, &T) -> bool>,
        seed: u64,
        value: T,
    ) -> bool {
        let cap = buckets.len();
        let h1 = Self::h1hash_for(&value, seed);
        let h2 = Self::h2hash_for(&value, seed);
        let mut iteration: usize = 1;
        loop {
            let index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            let pi = buckets[index].pi;
            if pi == 0 || pi == -1 {
                // Place the new value into the slot without dropping the
                // (uninitialized/zeroed) old contents.
                unsafe {
                    ptr::write(&mut buckets[index].elem, value);
                }
                buckets[index].pi = iteration as i32;
                return true;
            }
            if Self::matches_at(buckets, compare, &value, index) {
                return false;
            }
            iteration += 1;
        }
    }

    fn contains_in(
        buckets: &[CsetValue<T>],
        compare: Option<fn(&T, &T) -> bool>,
        seed: u64,
        value: &T,
    ) -> bool {
        let cap = buckets.len();
        if cap == 0 {
            return false;
        }
        let h1 = Self::h1hash_for(value, seed);
        let h2 = Self::h2hash_for(value, seed);
        let mut iteration: usize = 1;
        loop {
            if iteration - 1 >= cap {
                return false;
            }
            let index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let pi = buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                return false;
            }
            if Self::matches_at(buckets, compare, value, index) {
                return true;
            }
        }
    }

    fn resize(&mut self, new_cap: usize) {
        let mut new_buckets = Self::empty_buckets(new_cap);
        let compare = self.compare;
        let seed = self.seed;
        let old_buckets = mem::take(&mut self.buckets);
        self.bucket_size = 0;
        for entry in old_buckets {
            if entry.pi == 0 || entry.pi == -1 {
                continue;
            }
            // Move the elem out of the old entry without running drop.
            let elem = unsafe { ptr::read(&entry.elem) };
            mem::forget(entry);
            if Self::add_to_buckets(&mut new_buckets, compare, seed, elem) {
                self.bucket_size += 1;
            }
        }
        self.buckets = new_buckets;
    }

    pub fn new() -> Cset<T> {
        let mut cset = Cset {
            buckets: Vec::new(),
            max_load_factor: 0.0,
            min_load_factor: 0.0,
            seed: 0,
            v: Self::empty_value(),
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
        self.buckets = Self::empty_buckets(CSET_INITIAL_CAP);
        self.temp_buckets = Vec::new();
    }
    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }
    pub fn tombstone(&self) -> bool {
        // Without a bucket index parameter, expose whether any tombstone
        // currently exists in the table.
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
        // The signature requires a mutable reference from a shared `&self`,
        // mirroring the C macro that exposes the storage to mutation. Cast
        // through a raw pointer to honor the requested signature.
        let p = &self.buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>;
        unsafe { &mut *p }
    }
    #[allow(invalid_reference_casting)]
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        let p =
            &self.temp_buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>;
        unsafe { &mut *p }
    }
    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }
    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }
    pub fn add(&mut self, value: T) -> i32 {
        if self.buckets.is_empty() {
            self.init();
        }
        let cap = self.buckets.len();
        let load = (self.bucket_size as f64) / (cap as f64);
        if load >= self.max_load_factor {
            self.resize(cap * 2);
        }
        if Self::add_to_buckets(&mut self.buckets, self.compare, self.seed, value) {
            self.bucket_size += 1;
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
        let h1 = Self::h1hash_for(&value, self.seed);
        let h2 = Self::h2hash_for(&value, self.seed);
        let mut iteration: usize = 1;
        loop {
            if iteration - 1 >= cap {
                return 0;
            }
            let index = Self::double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                return 0;
            }
            if Self::matches_at(&self.buckets, self.compare, &value, index) {
                self.buckets[index].pi = -1;
                if self.bucket_size > 0 {
                    self.bucket_size -= 1;
                }
                return 1;
            }
        }
    }
    pub fn contains(&mut self, value: &T) -> bool {
        Self::contains_in(&self.buckets, self.compare, self.seed, value)
    }
    pub fn iter(&mut self) -> Vec<T>
    where
        T: Clone,
    {
        let mut result = Vec::with_capacity(self.bucket_size);
        for entry in self.buckets.iter() {
            if entry.pi != 0 && entry.pi != -1 {
                result.push(entry.elem.clone());
            }
        }
        result
    }
    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }
    pub fn clear(&mut self) {
        self.buckets = Self::empty_buckets(CSET_INITIAL_CAP);
        self.bucket_size = 0;
    }
    pub fn intersect(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for entry in first.buckets.iter() {
            if entry.pi == 0 || entry.pi == -1 {
                continue;
            }
            if Self::contains_in(&second.buckets, second.compare, second.seed, &entry.elem)
            {
                self.add(entry.elem.clone());
            }
        }
    }
    pub fn union(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for entry in first.buckets.iter() {
            if entry.pi == 0 || entry.pi == -1 {
                continue;
            }
            self.add(entry.elem.clone());
        }
        for entry in second.buckets.iter() {
            if entry.pi == 0 || entry.pi == -1 {
                continue;
            }
            self.add(entry.elem.clone());
        }
    }
    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for entry in self.buckets.iter() {
            if entry.pi == 0 || entry.pi == -1 {
                continue;
            }
            if Self::contains_in(&other.buckets, other.compare, other.seed, &entry.elem) {
                return false;
            }
        }
        true
    }
    pub fn difference(&mut self, first: &Self, second: &Self)
    where
        T: Clone,
    {
        for entry in first.buckets.iter() {
            if entry.pi == 0 || entry.pi == -1 {
                continue;
            }
            if !Self::contains_in(&second.buckets, second.compare, second.seed, &entry.elem)
            {
                self.add(entry.elem.clone());
            }
        }
    }
}
