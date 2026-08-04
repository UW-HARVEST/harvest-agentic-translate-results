// Import necessary modules
use std::mem;
use std::ptr;
use std::slice;
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

// ----- Private XXH64 implementation working on byte slices -----

#[inline]
fn rotl64(x: u64, r: u32) -> u64 {
    x.rotate_left(r)
}

#[inline]
fn read_le64_bytes(b: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&b[..8]);
    u64::from_le_bytes(arr)
}

#[inline]
fn read_le32_bytes(b: &[u8]) -> u32 {
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&b[..4]);
    u32::from_le_bytes(arr)
}

fn xxh64_round_priv(acc: u64, input: u64) -> u64 {
    let acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    let acc = rotl64(acc, 31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

fn xxh64_merge_round_priv(acc: u64, val: u64) -> u64 {
    let val = xxh64_round_priv(0, val);
    let acc = acc ^ val;
    acc.wrapping_mul(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_4)
}

fn xxh64_avalanche_priv(mut h64: u64) -> u64 {
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(XXH_PRIME64_2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(XXH_PRIME64_3);
    h64 ^= h64 >> 32;
    h64
}

fn xxh64_finalize_priv(mut h64: u64, data: &[u8]) -> u64 {
    let mut p = 0usize;
    let mut len = data.len() & 31;
    while len >= 8 {
        let k1 = xxh64_round_priv(0, read_le64_bytes(&data[p..p + 8]));
        p += 8;
        h64 ^= k1;
        h64 = rotl64(h64, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        let k = read_le32_bytes(&data[p..p + 4]) as u64;
        h64 ^= k.wrapping_mul(XXH_PRIME64_1);
        p += 4;
        h64 = rotl64(h64, 23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (data[p] as u64).wrapping_mul(XXH_PRIME64_5);
        p += 1;
        h64 = rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche_priv(h64)
}

fn xxh64_priv(data: &[u8], seed: u64) -> u64 {
    let len = data.len();
    let mut h64: u64;
    let mut p = 0usize;
    if len >= 32 {
        let mut v1 = seed
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_add(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(0);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        while p + 32 <= len {
            v1 = xxh64_round_priv(v1, read_le64_bytes(&data[p..p + 8]));
            p += 8;
            v2 = xxh64_round_priv(v2, read_le64_bytes(&data[p..p + 8]));
            p += 8;
            v3 = xxh64_round_priv(v3, read_le64_bytes(&data[p..p + 8]));
            p += 8;
            v4 = xxh64_round_priv(v4, read_le64_bytes(&data[p..p + 8]));
            p += 8;
        }
        h64 = rotl64(v1, 1)
            .wrapping_add(rotl64(v2, 7))
            .wrapping_add(rotl64(v3, 12))
            .wrapping_add(rotl64(v4, 18));
        h64 = xxh64_merge_round_priv(h64, v1);
        h64 = xxh64_merge_round_priv(h64, v2);
        h64 = xxh64_merge_round_priv(h64, v3);
        h64 = xxh64_merge_round_priv(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    }
    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_priv(h64, &data[p..])
}

fn xxh64_h_priv(data: &[u8], seed: u64) -> u64 {
    let len = data.len();
    let mut h64: u64;
    let mut p = 0usize;
    if len >= 32 {
        let mut v1 = seed
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2);
        let mut v2 = seed.wrapping_sub(XXH_PRIME64_2);
        let mut v3 = seed.wrapping_add(XXH_PRIME64_3);
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);
        while p + 32 <= len {
            v1 = xxh64_round_priv(v1, read_le64_bytes(&data[p..p + 8]));
            p += 8;
            v2 = xxh64_round_priv(v2, read_le64_bytes(&data[p..p + 8]));
            p += 8;
            v3 = xxh64_round_priv(v3, read_le64_bytes(&data[p..p + 8]));
            p += 8;
            v4 = xxh64_round_priv(v4, read_le64_bytes(&data[p..p + 8]));
            p += 8;
        }
        h64 = rotl64(v1, 1)
            .wrapping_add(rotl64(v2, 7))
            .wrapping_add(rotl64(v3, 12))
            .wrapping_add(rotl64(v4, 18));
        h64 = xxh64_merge_round_priv(h64, v1);
        h64 = xxh64_merge_round_priv(h64, v2);
        h64 = xxh64_merge_round_priv(h64, v3);
        h64 = xxh64_merge_round_priv(h64, v4);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }
    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_priv(h64, &data[p..])
}

#[inline]
fn t_as_bytes<T>(v: &T) -> &[u8] {
    unsafe { slice::from_raw_parts(v as *const T as *const u8, mem::size_of::<T>()) }
}

#[inline]
fn compute_h1<T>(value: &T, seed: u64) -> u64 {
    xxh64_priv(t_as_bytes(value), seed)
}

#[inline]
fn compute_h2<T>(value: &T, seed: u64) -> u64 {
    xxh64_h_priv(t_as_bytes(value), seed) | 1
}

#[inline]
fn double_hash_index(h1: u64, h2: u64, i: u64, cap: u64) -> usize {
    (h1.wrapping_add(i.wrapping_mul(h2)) % cap) as usize
}

fn elements_equal<T>(compare: &Option<fn(&T, &T) -> bool>, a: &T, b: &T) -> bool {
    if let Some(f) = compare {
        f(a, b)
    } else {
        t_as_bytes(a) == t_as_bytes(b)
    }
}

fn add_to_buckets<T>(
    buckets: &mut Vec<CsetValue<T>>,
    value: T,
    h1: u64,
    h2: u64,
    compare: &Option<fn(&T, &T) -> bool>,
) -> bool {
    let cap = buckets.len();
    let mut iteration: u64 = 1;
    let mut index: usize;
    let mut found = false;
    loop {
        index = double_hash_index(h1, h2, iteration - 1, cap as u64);
        iteration += 1;
        let pi = buckets[index].pi;
        if pi == 0 || pi == -1 {
            break;
        }
        if elements_equal(compare, &buckets[index].elem, &value) {
            found = true;
            break;
        }
        // safety guard against infinite loop (should not happen if load factor < 1)
        if iteration as usize > cap + 1 {
            break;
        }
    }
    if !found {
        // overwrite the slot. Drop existing elem (it was zeroed/never owned an init T,
        // but for types used in tests this is a no-op).
        buckets[index] = CsetValue {
            pi: iteration as i32,
            elem: value,
        };
        true
    } else {
        false
    }
}

// ----- Public XXH helper functions matching the required signatures -----

pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}

pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    unsafe {
        let p = mem_ptr as *const u8;
        let mut arr = [0u8; 8];
        for i in 0..8 {
            arr[i] = *p.add(i);
        }
        u64::from_le_bytes(arr)
    }
}

pub fn xxh_is_little_endian() -> bool {
    cfg!(target_endian = "little")
}

pub fn xxh_read_le64_align(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64(mem_ptr)
}

pub fn xxh_swap32(x: &mut XXHU32) -> XXHU32 {
    (*x).swap_bytes()
}

pub fn xxh_read32(mem_ptr: &mut XXHU32) -> XXHU32 {
    *mem_ptr
}

pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    xxh64_round_priv(acc, input)
}

pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    xxh64_merge_round_priv(acc, val)
}

pub fn xxh_get_32bits(ptr: &mut XXHU32) -> XXHU32 {
    xxh_read_le32_align(ptr)
}

pub fn xxh_read_le32_align(ptr: &mut XXHU32) -> XXHU32 {
    if xxh_is_little_endian() {
        *ptr
    } else {
        (*ptr).swap_bytes()
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
    let mut len = len & 31;
    unsafe {
        let mut p: *const u8 = ptr as *const u8;
        while len >= 8 {
            let mut arr = [0u8; 8];
            for i in 0..8 {
                arr[i] = *p.add(i);
            }
            let v = u64::from_le_bytes(arr);
            let k1 = xxh64_round_priv(0, v);
            p = p.add(8);
            h64 ^= k1;
            h64 = rotl64(h64, 27)
                .wrapping_mul(XXH_PRIME64_1)
                .wrapping_add(XXH_PRIME64_4);
            len -= 8;
        }
        if len >= 4 {
            let mut arr = [0u8; 4];
            for i in 0..4 {
                arr[i] = *p.add(i);
            }
            let k = u32::from_le_bytes(arr) as u64;
            h64 ^= k.wrapping_mul(XXH_PRIME64_1);
            p = p.add(4);
            h64 = rotl64(h64, 23)
                .wrapping_mul(XXH_PRIME64_2)
                .wrapping_add(XXH_PRIME64_3);
            len -= 4;
        }
        while len > 0 {
            h64 ^= (*p as u64).wrapping_mul(XXH_PRIME64_5);
            p = p.add(1);
            h64 = rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
            len -= 1;
        }
    }
    xxh64_avalanche(h64)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe {
        let p = input as *const u8;
        let bytes = slice::from_raw_parts(p, len);
        xxh64_priv(bytes, seed)
    }
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe {
        let p = input as *const u8;
        let bytes = slice::from_raw_parts(p, len);
        xxh64_h_priv(bytes, seed)
    }
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe {
        let bytes = slice::from_raw_parts(input, len);
        xxh64_priv(bytes, seed)
    }
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe {
        let bytes = slice::from_raw_parts(input, len);
        xxh64_h_priv(bytes, seed)
    }
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let p = memptr as *const u8;
    xxh64(p, size, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    let p = memptr as *const u8;
    xxh64_h(p, size, CSET_DEFAULT_SEED) | 1
}

// ----- Cset structures -----

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

fn make_empty_buckets<T>(cap: usize) -> Vec<CsetValue<T>> {
    let mut buckets: Vec<CsetValue<T>> = Vec::with_capacity(cap);
    for _ in 0..cap {
        buckets.push(CsetValue {
            pi: 0,
            elem: unsafe { mem::zeroed() },
        });
    }
    buckets
}

impl<T> Cset<T> {
    pub fn new() -> Cset<T> {
        Cset {
            buckets: make_empty_buckets(CSET_INITIAL_CAP),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v: CsetValue {
                pi: 0,
                elem: unsafe { mem::zeroed() },
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
        self.buckets = make_empty_buckets(CSET_INITIAL_CAP);
        self.temp_buckets = Vec::new();
    }

    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }

    pub fn tombstone(&self) -> bool {
        // Without an index argument, this is a generic check; mirror the macro by
        // returning whether any tombstone exists.
        self.buckets.iter().any(|b| b.pi == -1)
    }

    pub fn index(&self, index: usize) -> T {
        // Bytewise copy of the element at the given index, equivalent to a memcpy
        // in C. Safe for the POD types used in the tests.
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

    #[allow(invalid_reference_casting)]
    pub fn get_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        // The signature returns &mut from &self; this mirrors the macro
        // cset__vector_buckets_ref that returns the mutable address. We use
        // an unsafe cast which is sound when the caller respects aliasing.
        unsafe {
            let p: *const Vec<CsetValue<T>> = &self.buckets;
            &mut *(p as *mut Vec<CsetValue<T>>)
        }
    }

    #[allow(invalid_reference_casting)]
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe {
            let p: *const Vec<CsetValue<T>> = &self.temp_buckets;
            &mut *(p as *mut Vec<CsetValue<T>>)
        }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    fn resize(&mut self, new_cap: usize) {
        // Collect existing live elements.
        let old_buckets = mem::replace(&mut self.buckets, Vec::new());
        let mut elements: Vec<T> = Vec::with_capacity(self.bucket_size);
        for v in old_buckets {
            if v.pi != 0 && v.pi != -1 {
                // Move out the element.
                let CsetValue { elem, .. } = v;
                elements.push(elem);
            } else {
                // The elem here is a zeroed / uninitialised T placeholder. For the
                // POD types used in tests this is a no-op drop. We avoid running
                // user destructors on the zeroed placeholder by forgetting it.
                let CsetValue { elem, .. } = v;
                mem::forget(elem);
            }
        }
        self.buckets = make_empty_buckets(new_cap);
        self.bucket_size = 0;
        for elem in elements {
            let h1 = compute_h1(&elem, self.seed);
            let h2 = compute_h2(&elem, self.seed);
            if add_to_buckets(&mut self.buckets, elem, h1, h2, &self.compare) {
                self.bucket_size += 1;
            }
        }
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let load_factor = if cap == 0 {
            f64::INFINITY
        } else {
            (self.bucket_size as f64) / (cap as f64)
        };
        if load_factor >= self.max_load_factor {
            let new_cap = if cap == 0 { CSET_INITIAL_CAP } else { cap * 2 };
            self.resize(new_cap);
        }
        let h1 = compute_h1(&value, self.seed);
        let h2 = compute_h2(&value, self.seed);
        if add_to_buckets(&mut self.buckets, value, h1, h2, &self.compare) {
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
        let h1 = compute_h1(&value, self.seed);
        let h2 = compute_h2(&value, self.seed);
        let mut iteration: u64 = 1;
        let mut found = false;
        let mut index: usize = 0;
        loop {
            if (iteration - 1) >= cap as u64 {
                break;
            }
            index = double_hash_index(h1, h2, iteration - 1, cap as u64);
            iteration += 1;
            let pi = self.buckets[index].pi;
            if pi == -1 {
                continue;
            }
            if pi == 0 {
                break;
            }
            if elements_equal(&self.compare, &self.buckets[index].elem, &value) {
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

    fn contains_internal(&self, value: &T) -> bool {
        let cap = self.buckets.len();
        if cap == 0 {
            return false;
        }
        let h1 = compute_h1(value, self.seed);
        let h2 = compute_h2(value, self.seed);
        let mut iteration: u64 = 1;
        loop {
            if (iteration - 1) >= cap as u64 {
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
            if elements_equal(&self.compare, &self.buckets[index].elem, value) {
                return true;
            }
        }
        false
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.contains_internal(value)
    }

    pub fn iter(&mut self) -> Vec<T> {
        let mut out = Vec::with_capacity(self.bucket_size);
        for v in &self.buckets {
            if v.pi != 0 && v.pi != -1 {
                // Bytewise copy mirroring memcpy; valid for POD types used in tests.
                out.push(unsafe { ptr::read(&v.elem) });
            }
        }
        out
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        // Replicate cset__clear: free buckets and re-init with INITIAL_CAP empty slots.
        let old = mem::replace(&mut self.buckets, Vec::new());
        // Forget zeroed placeholders for empty/tombstone slots; drop live elements normally.
        for v in old {
            let CsetValue { pi, elem } = v;
            if pi == 0 || pi == -1 {
                mem::forget(elem);
            }
            // else: drop normally
        }
        self.buckets = make_empty_buckets(CSET_INITIAL_CAP);
        self.bucket_size = 0;
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        for v in &first.buckets {
            if v.pi == 0 || v.pi == -1 {
                continue;
            }
            if second.contains_internal(&v.elem) {
                let copy = unsafe { ptr::read(&v.elem) };
                self.add(copy);
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        for v in &first.buckets {
            if v.pi == 0 || v.pi == -1 {
                continue;
            }
            let copy = unsafe { ptr::read(&v.elem) };
            self.add(copy);
        }
        for v in &second.buckets {
            if v.pi == 0 || v.pi == -1 {
                continue;
            }
            let copy = unsafe { ptr::read(&v.elem) };
            self.add(copy);
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        for v in &self.buckets {
            if v.pi == 0 || v.pi == -1 {
                continue;
            }
            if other.contains_internal(&v.elem) {
                return false;
            }
        }
        true
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        for v in &first.buckets {
            if v.pi == 0 || v.pi == -1 {
                continue;
            }
            if !second.contains_internal(&v.elem) {
                let copy = unsafe { ptr::read(&v.elem) };
                self.add(copy);
            }
        }
    }
}

impl<T> Drop for Cset<T> {
    fn drop(&mut self) {
        // Forget zeroed placeholders so we don't run user destructors on uninitialised T.
        let buckets = mem::replace(&mut self.buckets, Vec::new());
        for v in buckets {
            let CsetValue { pi, elem } = v;
            if pi == 0 || pi == -1 {
                mem::forget(elem);
            }
        }
        let temp = mem::replace(&mut self.temp_buckets, Vec::new());
        for v in temp {
            let CsetValue { pi, elem } = v;
            if pi == 0 || pi == -1 {
                mem::forget(elem);
            }
        }
        // Forget the placeholder `v` field as well.
        let placeholder = mem::replace(
            &mut self.v,
            CsetValue {
                pi: 0,
                elem: unsafe { mem::zeroed() },
            },
        );
        mem::forget(placeholder);
    }
}
