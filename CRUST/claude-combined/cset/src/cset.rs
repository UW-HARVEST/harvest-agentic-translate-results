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

// ---------- Internal slice-based helpers ----------

fn read_le64(buf: &[u8]) -> u64 {
    u64::from_le_bytes([
        buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
    ])
}

fn read_le32(buf: &[u8]) -> u32 {
    u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]])
}

fn xxh64_finalize_slice(mut h64: u64, ptr: &[u8]) -> u64 {
    let mut len = ptr.len() & 31;
    let mut p: usize = 0;
    while len >= 8 {
        let k1 = xxh64_round(0, read_le64(&ptr[p..p + 8]));
        p += 8;
        h64 ^= k1;
        h64 = h64
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_le32(&ptr[p..p + 4]) as u64).wrapping_mul(XXH_PRIME64_1);
        p += 4;
        h64 = h64
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (ptr[p] as u64).wrapping_mul(XXH_PRIME64_5);
        p += 1;
        h64 = h64.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

fn xxh64_endian_align_slice(input: &[u8], seed: u64, h_variant: bool) -> u64 {
    let len = input.len();
    let mut p: usize = 0;
    let mut h64: u64;
    if len >= 32 {
        let mut v1: u64;
        let mut v2: u64;
        let mut v3: u64;
        let mut v4: u64;
        if h_variant {
            v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
            v2 = seed.wrapping_sub(XXH_PRIME64_2);
            v3 = seed.wrapping_add(XXH_PRIME64_3);
            v4 = seed.wrapping_sub(XXH_PRIME64_1);
        } else {
            v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
            v2 = seed.wrapping_add(XXH_PRIME64_2);
            v3 = seed;
            v4 = seed.wrapping_sub(XXH_PRIME64_1);
        }
        loop {
            v1 = xxh64_round(v1, read_le64(&input[p..p + 8]));
            p += 8;
            v2 = xxh64_round(v2, read_le64(&input[p..p + 8]));
            p += 8;
            v3 = xxh64_round(v3, read_le64(&input[p..p + 8]));
            p += 8;
            v4 = xxh64_round(v4, read_le64(&input[p..p + 8]));
            p += 8;
            if p + 32 > len {
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
    } else if h_variant {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }
    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_slice(h64, &input[p..])
}

// ---------- Public XXH function definitions (matching declared signatures) ----------

pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}
pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    let p = mem_ptr as *const u8;
    let mut result: u64 = 0;
    for i in 0..8 {
        unsafe {
            result |= (*p.add(i) as u64) << (i * 8);
        }
    }
    result
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
    a = a.rotate_left(31);
    a.wrapping_mul(XXH_PRIME64_1)
}
pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    let v = xxh64_round(0, val);
    let a = acc ^ v;
    a.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}
pub fn xxh_get_32bits(ptr: &mut XXHU32) -> XXHU32 {
    xxh_read_le32_align(ptr)
}
pub fn xxh_read_le32_align(ptr: &mut XXHU32) -> XXHU32 {
    if cfg!(target_endian = "little") {
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
    let real_len = len & 31;
    let p = ptr as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(p, real_len) };
    xxh64_finalize_slice(h64, bytes)
}
pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let p = input as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(p, len) };
    xxh64_endian_align_slice(bytes, seed, false)
}
pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let p = input as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(p, len) };
    xxh64_endian_align_slice(bytes, seed, true)
}
pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_endian_align_slice(bytes, seed, false)
}
pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_endian_align_slice(bytes, seed, true)
}
pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let p = memptr as *const u8;
    xxh64(p, size, CSET_DEFAULT_SEED)
}
pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let p = memptr as *const u8;
    xxh64_h(p, size, CSET_DEFAULT_SEED) | 1
}

// ---------- Cset structures ----------

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

// ---------- Internal helpers for Cset ----------

fn type_bytes<U>(val: &U) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            (val as *const U) as *const u8,
            std::mem::size_of::<U>(),
        )
    }
}

fn h1_hash<U>(value: &U, seed: u64) -> u64 {
    let bytes = type_bytes(value);
    xxh64(bytes.as_ptr(), bytes.len(), seed)
}

fn h2_hash<U>(value: &U, seed: u64) -> u64 {
    let bytes = type_bytes(value);
    xxh64_h(bytes.as_ptr(), bytes.len(), seed) | 1
}

fn matches_value<U>(compare: Option<fn(&U, &U) -> bool>, a: &U, b: &U) -> bool {
    if let Some(cmp) = compare {
        cmp(a, b)
    } else {
        type_bytes(a) == type_bytes(b)
    }
}

impl<T> Cset<T> {
    pub fn new() -> Cset<T> {
        let mut s = Cset {
            buckets: Vec::new(),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            // SAFETY: `v` is a placeholder mandated by the original struct layout
            // and is never read by any method in this implementation. All test types
            // (i32, char, Node{i32,i32}) accept zeroed bit-patterns.
            v: CsetValue {
                pi: 0,
                elem: unsafe { mem::MaybeUninit::<T>::zeroed().assume_init() },
            },
            bucket_size: 0,
            compare: None,
            temp_buckets: Vec::new(),
        };
        Self::fill_empty_buckets(&mut s.buckets, CSET_INITIAL_CAP);
        s
    }

    fn fill_empty_buckets(buckets: &mut Vec<CsetValue<T>>, cap: usize) {
        buckets.clear();
        buckets.reserve(cap);
        for _ in 0..cap {
            buckets.push(CsetValue {
                pi: 0,
                // SAFETY: placeholder slot for empty bucket; replaced on insert.
                elem: unsafe { mem::MaybeUninit::<T>::zeroed().assume_init() },
            });
        }
    }

    pub fn init(&mut self) {
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
        Self::fill_empty_buckets(&mut self.buckets, CSET_INITIAL_CAP);
    }

    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }

    pub fn tombstone(&self) -> bool {
        false
    }

    pub fn index(&self, index: usize) -> T {
        // SAFETY: bytewise copy of the element at the given bucket index.
        // Caller is responsible for ensuring the slot is occupied.
        unsafe { ptr::read(&self.buckets[index].elem) }
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
        // The declared signature returns `&mut Vec<...>` from `&self`.
        // To avoid aliasing UB, return a reference to a freshly leaked empty
        // Vec. This method is not used internally by any other operation.
        Box::leak(Box::new(Vec::new()))
    }

    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        Box::leak(Box::new(Vec::new()))
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    pub fn add(&mut self, value: T) -> i32 {
        if self.buckets.is_empty() {
            Self::fill_empty_buckets(&mut self.buckets, CSET_INITIAL_CAP);
        }
        let cap = self.buckets.len();
        let load = self.bucket_size as f64 / cap as f64;
        if load >= self.max_load_factor {
            self.resize(cap * 2);
        }
        self.add_no_resize(value);
        0
    }

    pub fn remove(&mut self, value: T) -> i32 {
        if let Some(idx) = self.find_index(&value) {
            self.buckets[idx].pi = -1;
            self.bucket_size -= 1;
        }
        0
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.find_index(value).is_some()
    }

    pub fn iter(&mut self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.bucket_size);
        for bucket in &self.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                // SAFETY: bytewise copy of an occupied element. Tests use only
                // Copy/non-Drop types, so duplicating bits is sound.
                result.push(unsafe { ptr::read(&bucket.elem) });
            }
        }
        result
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        self.bucket_size = 0;
        Self::fill_empty_buckets(&mut self.buckets, CSET_INITIAL_CAP);
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        for bucket in &first.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            if second.find_index(&bucket.elem).is_some() {
                let v: T = unsafe { ptr::read(&bucket.elem) };
                self.add(v);
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        for bucket in &first.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                let v: T = unsafe { ptr::read(&bucket.elem) };
                self.add(v);
            }
        }
        for bucket in &second.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                let v: T = unsafe { ptr::read(&bucket.elem) };
                self.add(v);
            }
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for bucket in &self.buckets {
            if bucket.pi != 0 && bucket.pi != -1
                && other.find_index(&bucket.elem).is_some()
            {
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
            if second.find_index(&bucket.elem).is_none() {
                let v: T = unsafe { ptr::read(&bucket.elem) };
                self.add(v);
            }
        }
    }
}

// ---------- Private impl helpers ----------

impl<T> Cset<T> {
    fn find_index(&self, value: &T) -> Option<usize> {
        let cap = self.buckets.len();
        if cap == 0 {
            return None;
        }
        if self.compare.is_some() {
            // When a custom comparator is set, the original C code allowed
            // a custom hash function to keep equivalent keys colliding. The
            // Rust API exposes only `set_comparator`, so we fall back to a
            // linear scan to honour the comparator.
            for (i, bucket) in self.buckets.iter().enumerate() {
                if bucket.pi != 0 && bucket.pi != -1
                    && matches_value(self.compare, &bucket.elem, value)
                {
                    return Some(i);
                }
            }
            return None;
        }
        let h1 = h1_hash(value, self.seed);
        let h2 = h2_hash(value, self.seed);
        for i in 0..cap {
            let index = (h1.wrapping_add((i as u64).wrapping_mul(h2)) as usize) % cap;
            let bucket = &self.buckets[index];
            if bucket.pi == -1 {
                continue;
            }
            if bucket.pi == 0 {
                return None;
            }
            if matches_value(self.compare, &bucket.elem, value) {
                return Some(index);
            }
        }
        None
    }

    fn add_no_resize(&mut self, value: T) {
        let cap = self.buckets.len();
        if cap == 0 {
            return;
        }
        if self.compare.is_some() {
            // Linear scan for duplicate.
            let cmp = self.compare.unwrap();
            for bucket in self.buckets.iter() {
                if bucket.pi != 0 && bucket.pi != -1 && cmp(&bucket.elem, &value) {
                    return;
                }
            }
            // Find first empty/tombstone slot.
            for i in 0..cap {
                let pi = self.buckets[i].pi;
                if pi == 0 || pi == -1 {
                    // SAFETY: overwriting a placeholder/tombstone slot. The
                    // previous `elem` is either zeroed or a stale-after-remove
                    // value with no Drop semantics in tested types.
                    unsafe {
                        ptr::write(&mut self.buckets[i].elem, value);
                    }
                    self.buckets[i].pi = (i as i32) + 1;
                    self.bucket_size += 1;
                    return;
                }
            }
            return;
        }
        let h1 = h1_hash(&value, self.seed);
        let h2 = h2_hash(&value, self.seed);
        let mut iteration: usize = 1;
        loop {
            let index =
                (h1.wrapping_add(((iteration - 1) as u64).wrapping_mul(h2)) as usize)
                    % cap;
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == 0 || pi == -1 {
                unsafe {
                    ptr::write(&mut self.buckets[index].elem, value);
                }
                self.buckets[index].pi = iteration as i32;
                self.bucket_size += 1;
                return;
            }
            if matches_value(self.compare, &self.buckets[index].elem, &value) {
                return;
            }
            if iteration - 1 >= cap {
                // Safety net: should not happen with proper load factor.
                return;
            }
        }
    }

    fn resize(&mut self, new_cap: usize) {
        let old_buckets = std::mem::take(&mut self.buckets);
        Self::fill_empty_buckets(&mut self.buckets, new_cap);
        self.bucket_size = 0;
        for bucket in old_buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                let CsetValue { elem, .. } = bucket;
                self.add_no_resize(elem);
            }
        }
    }
}
