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

// ---------- Internal helpers (operate on byte slices) ----------

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

fn xxh64_round_internal(mut acc: u64, input: u64) -> u64 {
    acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = acc.rotate_left(31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

fn xxh64_merge_round_internal(mut acc: u64, val: u64) -> u64 {
    let v = xxh64_round_internal(0, val);
    acc ^= v;
    acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}

fn xxh64_avalanche_internal(mut h64: u64) -> u64 {
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(XXH_PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(XXH_PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}

fn xxh64_finalize_bytes(mut h64: u64, mut ptr: &[u8], mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round_internal(0, read_le64_bytes(&ptr[..8]));
        ptr = &ptr[8..];
        h64 ^= k1;
        h64 = h64
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_le32_bytes(&ptr[..4]) as u64).wrapping_mul(XXH_PRIME64_1);
        ptr = &ptr[4..];
        h64 = h64
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        let b = ptr[0] as u64;
        ptr = &ptr[1..];
        h64 ^= b.wrapping_mul(XXH_PRIME64_5);
        h64 = h64.rotate_left(11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche_internal(h64)
}

fn xxh64_endian_align_bytes(input: &[u8], len: usize, seed: u64) -> u64 {
    let mut idx: usize = 0;
    let mut h64: u64;

    if len >= 32 {
        let limit = len - 32;
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round_internal(v1, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            v2 = xxh64_round_internal(v2, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            v3 = xxh64_round_internal(v3, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            v4 = xxh64_round_internal(v4, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            if idx > limit {
                break;
            }
        }

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round_internal(h64, v1);
        h64 = xxh64_merge_round_internal(h64, v2);
        h64 = xxh64_merge_round_internal(h64, v3);
        h64 = xxh64_merge_round_internal(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h64, &input[idx..], len)
}

fn xxh64_endian_align_h_bytes(input: &[u8], len: usize, seed: u64) -> u64 {
    let mut idx: usize = 0;
    let mut h64: u64;

    if len >= 32 {
        let limit = len - 32;
        let mut v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round_internal(v1, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            v2 = xxh64_round_internal(v2, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            v3 = xxh64_round_internal(v3, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            v4 = xxh64_round_internal(v4, read_le64_bytes(&input[idx..idx + 8]));
            idx += 8;
            if idx > limit {
                break;
            }
        }

        h64 = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        h64 = xxh64_merge_round_internal(h64, v1);
        h64 = xxh64_merge_round_internal(h64, v2);
        h64 = xxh64_merge_round_internal(h64, v3);
        h64 = xxh64_merge_round_internal(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h64, &input[idx..], len)
}

// ---------- Public XXH64 wrapper functions ----------

// These functions take `&mut XXHU8` (a single byte reference).  In the
// underlying C library they accept a raw pointer to a buffer of at least 8
// bytes.  To preserve the C semantics we read the required number of bytes
// past the supplied byte using unsafe pointer arithmetic.
pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}

pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(mem_ptr as *const u8, 8) };
    read_le64_bytes(bytes)
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
    xxh64_round_internal(acc, input)
}

pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    xxh64_merge_round_internal(acc, val)
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
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(ptr as *const u8, std::cmp::max(len, 1))
    };
    let used = std::cmp::min(bytes.len(), len);
    xxh64_finalize_bytes(h64, &bytes[..used], len)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(input as *const u8, std::cmp::max(len, 1))
    };
    let used = std::cmp::min(bytes.len(), len);
    xxh64_endian_align_bytes(&bytes[..used], len, seed)
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(input as *const u8, std::cmp::max(len, 1))
    };
    let used = std::cmp::min(bytes.len(), len);
    xxh64_endian_align_h_bytes(&bytes[..used], len, seed)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() || len == 0 {
        return xxh64_endian_align_bytes(&[], len, seed);
    }
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_endian_align_bytes(bytes, len, seed)
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    if input.is_null() || len == 0 {
        return xxh64_endian_align_h_bytes(&[], len, seed);
    }
    let bytes = unsafe { std::slice::from_raw_parts(input, len) };
    xxh64_endian_align_h_bytes(bytes, len, seed)
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(memptr as *const u8, std::cmp::max(size, 1))
    };
    let used = std::cmp::min(bytes.len(), size);
    xxh64_endian_align_bytes(&bytes[..used], size, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(memptr as *const u8, std::cmp::max(size, 1))
    };
    let used = std::cmp::min(bytes.len(), size);
    xxh64_endian_align_h_bytes(&bytes[..used], size, CSET_DEFAULT_SEED) | 1
}

// ---------- Cset Data Structures ----------

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

// ---------- Internal helpers for Cset<T> ----------

fn value_bytes<T>(v: &T) -> &[u8] {
    // SAFETY: We treat the memory of `T` as raw bytes for hashing/comparison
    // purposes.  This mirrors the behaviour of the C implementation which
    // hashes/compares values via memcpy/memcmp on their raw memory.
    unsafe {
        std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>())
    }
}

fn hash1_value<T>(value: &T, seed: u64) -> u64 {
    let bytes = value_bytes(value);
    xxh64_endian_align_bytes(bytes, bytes.len(), seed)
}

fn hash2_value<T>(value: &T, seed: u64) -> u64 {
    let bytes = value_bytes(value);
    xxh64_endian_align_h_bytes(bytes, bytes.len(), seed) | 1
}

fn values_match<T>(a: &T, b: &T, comparator: Option<fn(&T, &T) -> bool>) -> bool {
    if let Some(c) = comparator {
        c(a, b)
    } else {
        value_bytes(a) == value_bytes(b)
    }
}

fn double_hash_index(h1: u64, h2: u64, i: usize, cap: usize) -> usize {
    let mixed = h1.wrapping_add((i as u64).wrapping_mul(h2));
    (mixed % (cap as u64)) as usize
}

fn make_empty_value<T>() -> CsetValue<T> {
    // SAFETY: We mirror the C implementation which leaves uninitialised memory
    // for empty buckets and only relies on `pi` (== 0) to indicate emptiness.
    // Using `mem::zeroed` is acceptable for the value types used in the
    // accompanying tests (integers, chars, plain-old-data structs).
    CsetValue {
        pi: 0,
        elem: unsafe { mem::zeroed() },
    }
}

fn fresh_buckets<T>(cap: usize) -> Vec<CsetValue<T>> {
    let mut v: Vec<CsetValue<T>> = Vec::with_capacity(cap);
    for _ in 0..cap {
        v.push(make_empty_value());
    }
    v
}

impl<T> Cset<T> {
    pub fn new() -> Cset<T> {
        let mut c = Cset {
            buckets: Vec::new(),
            max_load_factor: 0.0,
            min_load_factor: 0.0,
            seed: 0,
            v: make_empty_value(),
            bucket_size: 0,
            compare: None,
            temp_buckets: Vec::new(),
        };
        c.init();
        c
    }

    pub fn init(&mut self) {
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
        self.buckets = fresh_buckets(CSET_INITIAL_CAP);
        self.temp_buckets = Vec::new();
    }

    pub fn empty(&self) -> bool {
        self.v.pi == 0
    }

    pub fn tombstone(&self) -> bool {
        self.v.pi == -1
    }

    pub fn index(&self, index: usize) -> T {
        // Returns a bitwise copy of the element at the requested bucket.
        // SAFETY: This mirrors the C macro which returns a pointer to the
        // bucket's element.  For Copy types this is equivalent to a regular
        // copy; for non-Copy types the caller must ensure the original is not
        // dropped concurrently.
        unsafe { ptr::read(&self.buckets[index].elem as *const T) }
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
        // SAFETY: The original C macro exposes a mutable pointer to the
        // bucket vector even from a const context.  We replicate this by
        // taking the address of the field via `addr_of!` and casting to a
        // mutable pointer.  Callers must ensure no aliasing mutation occurs.
        let ptr = std::ptr::addr_of!(self.buckets) as *mut Vec<CsetValue<T>>;
        unsafe { &mut *ptr }
    }

    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        // SAFETY: See `get_buckets_ref`.
        let ptr = std::ptr::addr_of!(self.temp_buckets) as *mut Vec<CsetValue<T>>;
        unsafe { &mut *ptr }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let load = (self.bucket_size as f64) / (cap as f64);
        if load >= self.max_load_factor {
            let new_cap = if cap == 0 { CSET_INITIAL_CAP } else { cap * 2 };
            self.resize(new_cap);
        }
        self.add_into_self(value)
    }

    fn add_into_self(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let h1 = hash1_value(&value, self.seed);
        let h2 = hash2_value(&value, self.seed);

        let mut iteration: usize = 1;
        let mut index: usize;
        let mut found = false;
        loop {
            index = double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == 0 || pi == -1 {
                break;
            }
            if values_match(&self.buckets[index].elem, &value, self.compare) {
                found = true;
                break;
            }
            if iteration > cap + 1 {
                // Safety net to avoid infinite loops; should not happen if
                // load-factor is properly bounded.
                break;
            }
        }
        if !found {
            self.buckets[index].elem = value;
            self.buckets[index].pi = iteration as i32;
            self.bucket_size += 1;
        }
        0
    }

    fn resize(&mut self, new_cap: usize) {
        let new_cap = if new_cap == 0 { 1 } else { new_cap };
        let mut new_buckets: Vec<CsetValue<T>> = fresh_buckets(new_cap);
        let old_buckets = mem::replace(&mut self.buckets, Vec::new());
        // Swap: install fresh empty buckets, then re-add each old entry.
        std::mem::swap(&mut self.buckets, &mut new_buckets);
        // `new_buckets` now holds the old (zeroed) vector we just created;
        // discard.
        drop(new_buckets);
        self.bucket_size = 0;

        for old in old_buckets {
            if old.pi == 0 || old.pi == -1 {
                continue;
            }
            self.add_into_self(old.elem);
        }
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        if cap == 0 {
            return 0;
        }
        let h1 = hash1_value(&value, self.seed);
        let h2 = hash2_value(&value, self.seed);

        let mut iteration: usize = 1;
        let mut found_index: Option<usize> = None;
        loop {
            if iteration - 1 >= cap {
                break;
            }
            let index = double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            if self.buckets[index].pi == -1 {
                continue;
            }
            if self.buckets[index].pi == 0 {
                break;
            }
            if values_match(&self.buckets[index].elem, &value, self.compare) {
                found_index = Some(index);
                break;
            }
        }
        if let Some(idx) = found_index {
            self.buckets[idx].pi = -1;
            self.bucket_size -= 1;
        }
        0
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.contains_internal(value)
    }

    fn contains_internal(&self, value: &T) -> bool {
        let cap = self.buckets.len();
        if cap == 0 {
            return false;
        }
        let h1 = hash1_value(value, self.seed);
        let h2 = hash2_value(value, self.seed);

        let mut iteration: usize = 1;
        loop {
            if iteration - 1 >= cap {
                return false;
            }
            let index = double_hash_index(h1, h2, iteration - 1, cap);
            iteration += 1;
            if self.buckets[index].pi == -1 {
                continue;
            }
            if self.buckets[index].pi == 0 {
                return false;
            }
            if values_match(&self.buckets[index].elem, value, self.compare) {
                return true;
            }
        }
    }

    pub fn iter(&mut self) -> Vec<T> {
        let mut result: Vec<T> = Vec::new();
        for bucket in &self.buckets {
            if bucket.pi != 0 && bucket.pi != -1 {
                // SAFETY: bitwise copy of the stored element.  Mirrors the
                // C iterator which yields raw pointers/values to the
                // underlying buckets.
                result.push(unsafe { ptr::read(&bucket.elem as *const T) });
            }
        }
        result
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        self.buckets = fresh_buckets(CSET_INITIAL_CAP);
        self.bucket_size = 0;
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        for bucket in &first.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            if second.contains_internal(&bucket.elem) {
                // SAFETY: Bitwise copy of the element to insert into self.
                let copy: T = unsafe { ptr::read(&bucket.elem as *const T) };
                self.add(copy);
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        for bucket in &first.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            let copy: T = unsafe { ptr::read(&bucket.elem as *const T) };
            self.add(copy);
        }
        for bucket in &second.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            let copy: T = unsafe { ptr::read(&bucket.elem as *const T) };
            self.add(copy);
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for bucket in &self.buckets {
            if bucket.pi == 0 || bucket.pi == -1 {
                continue;
            }
            if other.contains_internal(&bucket.elem) {
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
            if !second.contains_internal(&bucket.elem) {
                let copy: T = unsafe { ptr::read(&bucket.elem as *const T) };
                self.add(copy);
            }
        }
    }
}
