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

unsafe fn read_le64_raw(p: *const u8) -> u64 {
    let mut result: u64 = 0;
    for i in 0..8usize {
        result |= (*p.add(i) as u64) << (i * 8);
    }
    result
}

unsafe fn read_le32_raw(p: *const u8) -> u32 {
    let mut result: u32 = 0;
    for i in 0..4usize {
        result |= (*p.add(i) as u32) << (i * 8);
    }
    result
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
    *mem_ptr
}
pub fn xxh64_round(acc: XXHU64, input: XXHU64) -> XXHU64 {
    let mut acc = acc.wrapping_add(input.wrapping_mul(XXH_PRIME64_2));
    acc = xxh_rotl64(acc, 31);
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
pub fn xxh64_finalize(mut h64: XXHU64, ptr: &mut XXHU8, len: usize) -> XXHU64 {
    let mut len = len & 31;
    let mut p = ptr as *const u8;
    unsafe {
        while len >= 8 {
            let k1 = xxh64_round(0, read_le64_raw(p));
            p = p.add(8);
            h64 ^= k1;
            h64 = xxh_rotl64(h64, 27)
                .wrapping_mul(XXH_PRIME64_1)
                .wrapping_add(XXH_PRIME64_4);
            len -= 8;
        }
        if len >= 4 {
            h64 ^= (read_le32_raw(p) as u64).wrapping_mul(XXH_PRIME64_1);
            p = p.add(4);
            h64 = xxh_rotl64(h64, 23)
                .wrapping_mul(XXH_PRIME64_2)
                .wrapping_add(XXH_PRIME64_3);
            len -= 4;
        }
        while len > 0 {
            h64 ^= (*p as u64).wrapping_mul(XXH_PRIME64_5);
            p = p.add(1);
            h64 = xxh_rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
            len -= 1;
        }
    }
    xxh64_avalanche(h64)
}
pub fn xxh64_endian_align(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe { xxh64_endian_align_raw(input as *const u8, len, seed, false) }
}
pub fn xxh64_endian_align_h(input: &mut XXHU8, len: usize, seed: XXHU64) -> XXHU64 {
    unsafe { xxh64_endian_align_raw(input as *const u8, len, seed, true) }
}

unsafe fn xxh64_endian_align_raw(
    mut input: *const u8,
    len: usize,
    seed: u64,
    h_variant: bool,
) -> u64 {
    let mut h64: u64;

    if len >= 32 {
        let b_end = input.add(len);
        let limit = b_end.sub(32);
        let (mut v1, mut v2, mut v3, mut v4): (u64, u64, u64, u64);
        if !h_variant {
            v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
            v2 = seed.wrapping_add(XXH_PRIME64_2);
            v3 = seed;
            v4 = seed.wrapping_sub(XXH_PRIME64_1);
        } else {
            v1 = seed.wrapping_add(XXH_PRIME64_1).wrapping_add(XXH_PRIME64_2);
            v2 = seed.wrapping_sub(XXH_PRIME64_2);
            v3 = seed.wrapping_add(XXH_PRIME64_3);
            v4 = seed.wrapping_sub(XXH_PRIME64_1);
        }

        loop {
            v1 = xxh64_round(v1, read_le64_raw(input));
            input = input.add(8);
            v2 = xxh64_round(v2, read_le64_raw(input));
            input = input.add(8);
            v3 = xxh64_round(v3, read_le64_raw(input));
            input = input.add(8);
            v4 = xxh64_round(v4, read_le64_raw(input));
            input = input.add(8);
            if input > limit {
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
    } else if !h_variant {
        h64 = seed.wrapping_add(XXH_PRIME64_5);
    } else {
        h64 = seed.wrapping_add(XXH_PRIME64_1);
    }

    h64 = h64.wrapping_add(len as u64);
    xxh64_finalize_raw(h64, input, len)
}

unsafe fn xxh64_finalize_raw(mut h64: u64, mut p: *const u8, mut len: usize) -> u64 {
    len &= 31;
    while len >= 8 {
        let k1 = xxh64_round(0, read_le64_raw(p));
        p = p.add(8);
        h64 ^= k1;
        h64 = xxh_rotl64(h64, 27)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4);
        len -= 8;
    }
    if len >= 4 {
        h64 ^= (read_le32_raw(p) as u64).wrapping_mul(XXH_PRIME64_1);
        p = p.add(4);
        h64 = xxh_rotl64(h64, 23)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3);
        len -= 4;
    }
    while len > 0 {
        h64 ^= (*p as u64).wrapping_mul(XXH_PRIME64_5);
        p = p.add(1);
        h64 = xxh_rotl64(h64, 11).wrapping_mul(XXH_PRIME64_1);
        len -= 1;
    }
    xxh64_avalanche(h64)
}

pub fn xxh64(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe { xxh64_endian_align_raw(input, len, seed, false) }
}
pub fn xxh64_h(input: *const u8, len: usize, seed: XXH64HashT) -> XXH64HashT {
    unsafe { xxh64_endian_align_raw(input, len, seed, true) }
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
    buckets: Vec<CsetValue<T>>,
    max_load_factor: f64,
    min_load_factor: f64,
    seed: u64,
    v: CsetValue<T>,
    bucket_size: usize,
    compare: Option<fn(&T, &T) -> bool>,
    temp_buckets: Vec<CsetValue<T>>,
}

fn alloc_buckets<T>(cap: usize) -> Vec<CsetValue<T>> {
    let mut v: Vec<CsetValue<T>> = Vec::with_capacity(cap);
    unsafe {
        v.set_len(cap);
        for i in 0..cap {
            ptr::addr_of_mut!((*v.as_mut_ptr().add(i)).pi).write(0);
        }
    }
    v
}

fn free_buckets<T>(buckets: &mut Vec<CsetValue<T>>) {
    let len = buckets.len();
    let p = buckets.as_mut_ptr();
    unsafe {
        for i in 0..len {
            let elem_ptr = p.add(i);
            let pi = (*elem_ptr).pi;
            if pi != 0 && pi != -1 {
                ptr::drop_in_place(ptr::addr_of_mut!((*elem_ptr).elem));
            }
        }
        buckets.set_len(0);
    }
}

fn bytes_of<T>(val: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(val as *const T as *const u8, mem::size_of::<T>())
    }
}

fn hash1_of<T>(value: &T, seed: u64) -> u64 {
    let bytes = bytes_of(value);
    xxh64(bytes.as_ptr(), bytes.len(), seed)
}

fn hash2_of<T>(value: &T, seed: u64) -> u64 {
    let bytes = bytes_of(value);
    xxh64_h(bytes.as_ptr(), bytes.len(), seed) | 1
}

impl<T> Drop for Cset<T> {
    fn drop(&mut self) {
        free_buckets(&mut self.buckets);
        free_buckets(&mut self.temp_buckets);
        // self.v: pi=0, elem holds zeroed bytes (set up in `new`).
        // Rust will drop self.v.elem after this method returns.
        // For our supported types (i32, char, structs of integers), zeroed
        // bytes form a valid value with a trivial drop, which is safe.
    }
}

impl<T> Cset<T> {
    pub fn new() -> Cset<T> {
        let v = unsafe {
            let mut val: mem::MaybeUninit<CsetValue<T>> = mem::MaybeUninit::zeroed();
            ptr::addr_of_mut!((*val.as_mut_ptr()).pi).write(0);
            val.assume_init()
        };
        Cset {
            buckets: alloc_buckets::<T>(CSET_INITIAL_CAP),
            max_load_factor: CSET_MAX_LOAD_FACTOR,
            min_load_factor: CSET_MIN_LOAD_FACTOR,
            seed: CSET_DEFAULT_SEED,
            v,
            bucket_size: 0,
            compare: None,
            temp_buckets: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        free_buckets(&mut self.buckets);
        self.buckets = alloc_buckets::<T>(CSET_INITIAL_CAP);
        self.max_load_factor = CSET_MAX_LOAD_FACTOR;
        self.min_load_factor = CSET_MIN_LOAD_FACTOR;
        self.seed = CSET_DEFAULT_SEED;
        self.bucket_size = 0;
        self.compare = None;
    }

    pub fn empty(&self) -> bool {
        self.bucket_size == 0
    }

    pub fn tombstone(&self) -> bool {
        false
    }

    pub fn index(&self, index: usize) -> T {
        unsafe { ptr::read(ptr::addr_of!((*self.buckets.as_ptr().add(index)).elem)) }
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
        let p: *const Vec<CsetValue<T>> = &self.buckets;
        unsafe { mem::transmute::<*const Vec<CsetValue<T>>, &mut Vec<CsetValue<T>>>(p) }
    }

    pub fn get_temp_buckets_ref(&self) -> &mut Vec<CsetValue<T>> {
        let p: *const Vec<CsetValue<T>> = &self.temp_buckets;
        unsafe { mem::transmute::<*const Vec<CsetValue<T>>, &mut Vec<CsetValue<T>>>(p) }
    }

    pub fn size(&self) -> i32 {
        self.bucket_size as i32
    }

    pub fn capacity(&self) -> i32 {
        self.buckets.len() as i32
    }

    pub fn add(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        let load = if cap == 0 {
            f64::INFINITY
        } else {
            self.bucket_size as f64 / cap as f64
        };
        if load >= self.max_load_factor {
            let new_cap = if cap == 0 { CSET_INITIAL_CAP } else { cap * 2 };
            self.resize_internal(new_cap);
        }
        let seed = self.seed;
        let compare = self.compare;
        Self::add_to_buckets(
            &mut self.buckets,
            &mut self.bucket_size,
            seed,
            compare,
            value,
        );
        0
    }

    fn add_to_buckets(
        buckets: &mut Vec<CsetValue<T>>,
        bucket_size: &mut usize,
        seed: u64,
        compare: Option<fn(&T, &T) -> bool>,
        value: T,
    ) {
        let cap = buckets.len();
        let h1 = hash1_of(&value, seed);
        let h2 = hash2_of(&value, seed);
        let mut iteration: usize = 1;
        let mut found = false;
        let mut index: usize = 0;
        loop {
            index = (h1.wrapping_add(((iteration - 1) as u64).wrapping_mul(h2))
                % (cap as u64)) as usize;
            iteration += 1;
            let cur_pi = unsafe { (*buckets.as_ptr().add(index)).pi };
            if cur_pi == 0 || cur_pi == -1 {
                break;
            }
            let elem_ref = unsafe { &(*buckets.as_ptr().add(index)).elem };
            let matches = match compare {
                Some(f) => f(elem_ref, &value),
                None => bytes_of(elem_ref) == bytes_of(&value),
            };
            if matches {
                found = true;
                break;
            }
        }
        if !found {
            unsafe {
                let p = buckets.as_mut_ptr().add(index);
                ptr::write(ptr::addr_of_mut!((*p).elem), value);
                ptr::addr_of_mut!((*p).pi).write(iteration as i32);
            }
            *bucket_size += 1;
        }
        // If found, `value` is dropped at end of scope.
    }

    fn resize_internal(&mut self, new_cap: usize) {
        // Free any leftover temp_buckets first
        free_buckets(&mut self.temp_buckets);
        self.temp_buckets = alloc_buckets::<T>(new_cap);
        self.bucket_size = 0;
        let old_cap = self.buckets.len();
        let seed = self.seed;
        let compare = self.compare;

        for i in 0..old_cap {
            let pi = unsafe { (*self.buckets.as_ptr().add(i)).pi };
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem = unsafe {
                ptr::read(ptr::addr_of!((*self.buckets.as_ptr().add(i)).elem))
            };
            // Mark the old slot as empty so free_buckets won't drop it again.
            unsafe {
                ptr::addr_of_mut!((*self.buckets.as_mut_ptr().add(i)).pi).write(0);
            }
            Self::add_to_buckets(
                &mut self.temp_buckets,
                &mut self.bucket_size,
                seed,
                compare,
                elem,
            );
        }

        // All slots in old buckets are now empty; safely drop the Vec.
        unsafe {
            self.buckets.set_len(0);
        }
        self.buckets = mem::replace(&mut self.temp_buckets, Vec::new());
    }

    pub fn remove(&mut self, value: T) -> i32 {
        let cap = self.buckets.len();
        if cap == 0 {
            return 0;
        }
        let h1 = hash1_of(&value, self.seed);
        let h2 = hash2_of(&value, self.seed);
        let mut iteration: usize = 1;
        let mut found = false;
        let mut index: usize = 0;
        loop {
            if iteration - 1 >= cap {
                break;
            }
            index = (h1.wrapping_add(((iteration - 1) as u64).wrapping_mul(h2))
                % (cap as u64)) as usize;
            iteration += 1;
            let cur_pi = unsafe { (*self.buckets.as_ptr().add(index)).pi };
            if cur_pi == -1 {
                continue;
            }
            if cur_pi == 0 {
                break;
            }
            let elem_ref = unsafe { &(*self.buckets.as_ptr().add(index)).elem };
            let matches = match self.compare {
                Some(f) => f(elem_ref, &value),
                None => bytes_of(elem_ref) == bytes_of(&value),
            };
            if matches {
                found = true;
                break;
            }
        }
        if found {
            unsafe {
                let p = self.buckets.as_mut_ptr().add(index);
                ptr::drop_in_place(ptr::addr_of_mut!((*p).elem));
                ptr::addr_of_mut!((*p).pi).write(-1);
            }
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
        let h1 = hash1_of(value, self.seed);
        let h2 = hash2_of(value, self.seed);
        let mut iteration: usize = 1;
        let mut found = false;
        loop {
            if iteration - 1 >= cap {
                break;
            }
            let index = (h1.wrapping_add(((iteration - 1) as u64).wrapping_mul(h2))
                % (cap as u64)) as usize;
            iteration += 1;
            let cur_pi = unsafe { (*self.buckets.as_ptr().add(index)).pi };
            if cur_pi == -1 {
                continue;
            }
            if cur_pi == 0 {
                break;
            }
            let elem_ref = unsafe { &(*self.buckets.as_ptr().add(index)).elem };
            let matches = match self.compare {
                Some(f) => f(elem_ref, value),
                None => bytes_of(elem_ref) == bytes_of(value),
            };
            if matches {
                found = true;
                break;
            }
        }
        found
    }

    pub fn iter(&mut self) -> Vec<T> {
        let mut result = Vec::new();
        let len = self.buckets.len();
        let p = self.buckets.as_ptr();
        unsafe {
            for i in 0..len {
                let pi = (*p.add(i)).pi;
                if pi == 0 || pi == -1 {
                    continue;
                }
                let elem_ptr = ptr::addr_of!((*p.add(i)).elem);
                result.push(ptr::read(elem_ptr));
            }
        }
        result
    }

    pub fn set_comparator(&mut self, compare: fn(&T, &T) -> bool) {
        self.compare = Some(compare);
    }

    pub fn clear(&mut self) {
        free_buckets(&mut self.buckets);
        self.buckets = alloc_buckets::<T>(CSET_INITIAL_CAP);
        self.bucket_size = 0;
    }

    fn add_all_from(&mut self, src: &Self) {
        let len = src.buckets.len();
        for i in 0..len {
            let pi = unsafe { (*src.buckets.as_ptr().add(i)).pi };
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem_ref = unsafe { &(*src.buckets.as_ptr().add(i)).elem };
            let elem_copy = unsafe { ptr::read(elem_ref) };
            let cap = self.buckets.len();
            let load = if cap == 0 {
                f64::INFINITY
            } else {
                self.bucket_size as f64 / cap as f64
            };
            if load >= self.max_load_factor {
                let new_cap = if cap == 0 { CSET_INITIAL_CAP } else { cap * 2 };
                self.resize_internal(new_cap);
            }
            let seed = self.seed;
            let compare = self.compare;
            Self::add_to_buckets(
                &mut self.buckets,
                &mut self.bucket_size,
                seed,
                compare,
                elem_copy,
            );
        }
    }

    pub fn intersect(&mut self, first: &Self, second: &Self) {
        let len = first.buckets.len();
        for i in 0..len {
            let pi = unsafe { (*first.buckets.as_ptr().add(i)).pi };
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem_ref = unsafe { &(*first.buckets.as_ptr().add(i)).elem };
            if second.contains_internal(elem_ref) {
                let elem_copy = unsafe { ptr::read(elem_ref) };
                let cap = self.buckets.len();
                let load = if cap == 0 {
                    f64::INFINITY
                } else {
                    self.bucket_size as f64 / cap as f64
                };
                if load >= self.max_load_factor {
                    let new_cap = if cap == 0 { CSET_INITIAL_CAP } else { cap * 2 };
                    self.resize_internal(new_cap);
                }
                let seed = self.seed;
                let compare = self.compare;
                Self::add_to_buckets(
                    &mut self.buckets,
                    &mut self.bucket_size,
                    seed,
                    compare,
                    elem_copy,
                );
            }
        }
    }

    pub fn union(&mut self, first: &Self, second: &Self) {
        self.add_all_from(first);
        self.add_all_from(second);
    }

    pub fn is_disjoint(&mut self, other: &Self) -> bool {
        let len = self.buckets.len();
        for i in 0..len {
            let pi = unsafe { (*self.buckets.as_ptr().add(i)).pi };
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem_ref = unsafe { &(*self.buckets.as_ptr().add(i)).elem };
            if other.contains_internal(elem_ref) {
                return false;
            }
        }
        true
    }

    pub fn difference(&mut self, first: &Self, second: &Self) {
        let len = first.buckets.len();
        for i in 0..len {
            let pi = unsafe { (*first.buckets.as_ptr().add(i)).pi };
            if pi == 0 || pi == -1 {
                continue;
            }
            let elem_ref = unsafe { &(*first.buckets.as_ptr().add(i)).elem };
            if !second.contains_internal(elem_ref) {
                let elem_copy = unsafe { ptr::read(elem_ref) };
                let cap = self.buckets.len();
                let load = if cap == 0 {
                    f64::INFINITY
                } else {
                    self.bucket_size as f64 / cap as f64
                };
                if load >= self.max_load_factor {
                    let new_cap = if cap == 0 { CSET_INITIAL_CAP } else { cap * 2 };
                    self.resize_internal(new_cap);
                }
                let seed = self.seed;
                let compare = self.compare;
                Self::add_to_buckets(
                    &mut self.buckets,
                    &mut self.bucket_size,
                    seed,
                    compare,
                    elem_copy,
                );
            }
        }
    }
}
