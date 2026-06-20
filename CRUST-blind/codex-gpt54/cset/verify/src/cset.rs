use std::mem::{self, MaybeUninit};
use std::slice;

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

fn bytes_from_ptr<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        // The C implementation accepts raw pointers and reads len bytes.
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

fn read_u64_le(ptr: *const u8) -> u64 {
    let bytes = bytes_from_ptr(ptr, 8);
    let mut buf = [0u8; 8];
    buf.copy_from_slice(bytes);
    u64::from_le_bytes(buf)
}

fn read_u32_le(ptr: *const u8) -> u32 {
    let bytes = bytes_from_ptr(ptr, 4);
    let mut buf = [0u8; 4];
    buf.copy_from_slice(bytes);
    u32::from_le_bytes(buf)
}

fn xxh64_finalize_bytes(mut h64: XXHU64, mut ptr: *const u8, mut len: usize) -> XXHU64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round(0, read_u64_le(ptr));
        ptr = ptr.wrapping_add(8);
        h64 ^= k1;
        h64 = rotl64(h64, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_u32_le(ptr) as u64).wrapping_mul(XXH_PRIME64_1);
        ptr = ptr.wrapping_add(4);
        h64 = rotl64(h64, 23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= unsafe { *ptr } as u64 * XXH_PRIME64_5;
        h64 = rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        ptr = ptr.wrapping_add(1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

fn xxh64_endian_align_bytes(input: *const u8, len: usize, seed: XXHU64, alt: bool) -> XXHU64 {
    let mut ptr = input;
    let mut h64;

    if len >= 32 {
        let limit = input.wrapping_add(len - 32);
        let mut v1 = seed
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2);
        let mut v2 = if alt {
            seed.wrapping_sub(XXH_PRIME64_2)
        } else {
            seed.wrapping_add(XXH_PRIME64_2)
        };
        let mut v3 = if alt {
            seed.wrapping_add(XXH_PRIME64_3)
        } else {
            seed
        };
        let mut v4 = seed.wrapping_sub(XXH_PRIME64_1);

        loop {
            v1 = xxh64_round(v1, read_u64_le(ptr));
            ptr = ptr.wrapping_add(8);
            v2 = xxh64_round(v2, read_u64_le(ptr));
            ptr = ptr.wrapping_add(8);
            v3 = xxh64_round(v3, read_u64_le(ptr));
            ptr = ptr.wrapping_add(8);
            v4 = xxh64_round(v4, read_u64_le(ptr));
            ptr = ptr.wrapping_add(8);
            if ptr > limit {
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
        h64 = seed.wrapping_add(if alt { XXH_PRIME64_1 } else { XXH_PRIME64_5 });
    }

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_bytes(h64, ptr, len)
}

fn value_as_bytes<T>(value: &T) -> &[u8] {
    unsafe { slice::from_raw_parts((value as *const T).cast::<u8>(), mem::size_of::<T>()) }
}

fn bytes_equal<T>(left: &T, right: &T) -> bool {
    value_as_bytes(left) == value_as_bytes(right)
}

fn pod_copy<T>(value: &T) -> T {
    unsafe { value_as_bytes(value).as_ptr().cast::<T>().read() }
}

fn zeroed_value<T>() -> T {
    unsafe { MaybeUninit::<T>::zeroed().assume_init() }
}

pub fn xxh_get64bits(mem_ptr: &mut XXHU8) -> XXHU64 {
    xxh_read_le64_align(mem_ptr)
}

pub fn xxh_read_le64(mem_ptr: &mut XXHU8) -> XXHU64 {
    read_u64_le(mem_ptr as *mut XXHU8 as *const u8)
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
    let acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    let acc = rotl64(acc, 31);
    acc.wrapping_mul(XXH_PRIME64_1)
}

pub fn xxh64_merge_round(acc: XXHU64, val: XXHU64) -> XXHU64 {
    let val = xxh64_round(0, val);
    let acc = acc ^ val;
    acc.wrapping_mul(XXH_PRIME64_1)
        .wrapping_add(XXH_PRIME64_4)
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
    xxh64_finalize_bytes(h64, ptr as *mut XXHU8 as *const u8, len)
}

pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    xxh64_endian_align_bytes(input as *mut XXHU8 as *const u8, len, seed, false)
}

pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    xxh64_endian_align_bytes(input as *mut XXHU8 as *const u8, len, seed, true)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    xxh64_endian_align_bytes(input, len, seed, false)
}

pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    xxh64_endian_align_bytes(input, len, seed, true)
}

pub fn cset_hash1_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64(memptr as *mut XXHU8 as *const u8, size, CSET_DEFAULT_SEED)
}

pub fn cset_hash2_callback(memptr: &mut XXHU8, size: usize) -> XXHU64 {
    xxh64_h(memptr as *mut XXHU8 as *const u8, size, CSET_DEFAULT_SEED) | 1
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
    pub fn new() -> Cset<T> {
        Cset {
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
            temp_buckets: Vec::with_capacity(CSET_INITIAL_CAP),
        }
    }

    pub fn init(&mut self) {
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
        self.buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        self.temp_buckets = Vec::with_capacity(CSET_INITIAL_CAP);
        self.v.pi = 0;
    }

    pub fn empty(&self) -> bool {
        self.v.pi == 0
    }

    pub fn tombstone(&self) -> bool {
        self.v.pi == -1
    }

    pub fn index(&self, index: usize) -> T {
        pod_copy(&self.buckets[index].elem)
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
        unsafe { &mut *(&self.buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>) }
    }

    #[allow(invalid_reference_casting)]
    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        unsafe {
            &mut *(&self.temp_buckets as *const Vec<CsetValue<T>> as *mut Vec<CsetValue<T>>)
        }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.capacity() as i32
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.capacity().max(1);
        let current_load_factor = self.bucket_size as f64 / cap as f64;
        if current_load_factor >= self.max_load_factor {
            self.buckets.reserve_exact(cap);
        }

        self.v.pi = 0;
        self.v.elem = value;
        if self.contains_ref(&self.v.elem, self.compare) {
            return 0;
        }

        self.buckets.push(CsetValue {
            pi: (self.bucket_size + 1) as i32,
            elem: unsafe { (&self.v.elem as *const T).read() },
        });
        self.bucket_size += 1;
        0
    }

    pub fn remove(&mut self, value: T) -> i32 {
        self.v.pi = 0;
        self.v.elem = value;
        if let Some(index) = self
            .buckets
            .iter()
            .position(|entry| self.matches(&entry.elem, &self.v.elem))
        {
            self.buckets.remove(index);
            self.bucket_size -= 1;
        }
        0
    }

    pub fn contains(&mut self, value: &T) -> bool {
        self.buckets.iter().any(|entry| self.matches(&entry.elem, value))
    }

    pub fn iter(&mut self) -> Vec<T> {
        self.buckets.iter().map(|entry| pod_copy(&entry.elem)).collect()
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
        self.buckets.shrink_to(CSET_INITIAL_CAP);
        if self.buckets.capacity() < CSET_INITIAL_CAP {
            self.buckets.reserve_exact(CSET_INITIAL_CAP - self.buckets.capacity());
        }
        self.bucket_size = 0;
        self.v.pi = 0;
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        self.clear();
        self.compare = first.compare.or(second.compare);
        for entry in &first.buckets {
            if second.contains_ref(&entry.elem, self.compare) {
                self.buckets.push(CsetValue {
                    pi: (self.bucket_size + 1) as i32,
                    elem: pod_copy(&entry.elem),
                });
                self.bucket_size += 1;
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        self.clear();
        self.compare = first.compare.or(second.compare);
        for entry in &first.buckets {
            self.add_entry_copy(&entry.elem);
        }
        for entry in &second.buckets {
            self.add_entry_copy(&entry.elem);
        }
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        !self
            .buckets
            .iter()
            .any(|entry| other.contains_ref(&entry.elem, self.compare.or(other.compare)))
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        self.clear();
        self.compare = first.compare.or(second.compare);
        for entry in &first.buckets {
            if !second.contains_ref(&entry.elem, self.compare) {
                self.buckets.push(CsetValue {
                    pi: (self.bucket_size + 1) as i32,
                    elem: pod_copy(&entry.elem),
                });
                self.bucket_size += 1;
            }
        }
    }

    fn matches(&self, left: &T, right: &T) -> bool {
        match self.compare {
            Some(compare) => compare(left, right),
            None => bytes_equal(left, right),
        }
    }

    fn contains_ref(&self, value: &T, compare: Option<fn(&T, &T) -> bool>) -> bool {
        self.buckets.iter().any(|entry| match compare {
            Some(cmp) => cmp(&entry.elem, value),
            None => bytes_equal(&entry.elem, value),
        })
    }

    fn add_entry_copy(&mut self, value: &T) {
        if !self.contains_ref(value, self.compare) {
            let cap = self.buckets.capacity().max(1);
            let current_load_factor = self.bucket_size as f64 / cap as f64;
            if current_load_factor >= self.max_load_factor {
                self.buckets.reserve_exact(cap);
            }
            self.buckets.push(CsetValue {
                pi: (self.bucket_size + 1) as i32,
                elem: pod_copy(value),
            });
            self.bucket_size += 1;
        }
    }
}
