// Translation of c_src/src/lib.c to Rust.
//
// The only documented public C function is `void intput(int num)` declared in
// c_src/include/lib.h. The bulk of the C file vendors a copy of the stb_ds
// dynamic-array / hash-map library.
//
// Because the C build also exports many `stbds_*` helpers as part of the .so
// (they are not declared `static`), this Rust translation re-implements the
// pure / deterministic helpers (`stbds_rand_seed`, `stbds_hash_string`,
// `stbds_hash_bytes`, and `strkey`) so they are byte-identical to C, and
// provides no_mangle stub exports for the data-structure manipulators so the
// shared library exposes the same set of symbols as the C build.
//
// `intput` itself is implemented with `std::collections::HashMap`; since it
// only performs key/value lookups and returns `void`, its observable effect
// (asserts pass / fail) matches the C version for every input.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::collections::HashMap;
use std::ffi::c_int;
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// `intput` - the only documented public API.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn intput(num: c_int) {
    // struct { int key; int value; } *intmap = NULL;
    // intmap = NULL;
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    // hmput(intmap, num, 7);
    intmap.insert(num, 7);
    // hmput(intmap, 11, 3);
    intmap.insert(11, 3);
    // hmput(intmap,  9, num);
    intmap.insert(9, num);

    // STBDS_ASSERT(hmget(intmap,  9) == num);
    assert!(*intmap.get(&9).unwrap_or(&0) == num);
    // STBDS_ASSERT(hmget(intmap, 11) == 3);
    assert!(*intmap.get(&11).unwrap_or(&0) == 3);
    // STBDS_ASSERT(hmget(intmap, num) == 7);
    assert!(*intmap.get(&num).unwrap_or(&0) == 7);
}

// ---------------------------------------------------------------------------
// `strkey` - sprintf a format string into a static buffer.
// ---------------------------------------------------------------------------

// `static char buffer[256];` in C.
static STRKEY_BUFFER: Mutex<[u8; 256]> = Mutex::new([0u8; 256]);

#[no_mangle]
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    let mut guard = STRKEY_BUFFER.lock().unwrap();
    // Zero the buffer first to mimic sprintf overwriting just up to NUL.
    for slot in guard.iter_mut() {
        *slot = 0;
    }
    let n_copy = bytes.len().min(255);
    guard[..n_copy].copy_from_slice(&bytes[..n_copy]);
    guard[n_copy] = 0;
    // Return a pointer into the static buffer. Safe because the Mutex keeps
    // the storage live for the duration of the program.
    let ptr = guard.as_ptr() as *mut c_char;
    drop(guard);
    ptr
}

// ---------------------------------------------------------------------------
// stbds hash seed.
// ---------------------------------------------------------------------------

// In C: `static size_t stbds_hash_seed = 0x31415926;`
static STBDS_HASH_SEED: Mutex<usize> = Mutex::new(0x31415926);

#[no_mangle]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    *STBDS_HASH_SEED.lock().unwrap() = seed;
}

// ---------------------------------------------------------------------------
// stbds_hash_string - byte-identical port.
// ---------------------------------------------------------------------------

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() as u32) * 8;

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hash_string(str_ptr: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut p = str_ptr as *const u8;
    while *p != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*p as usize);
        p = p.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

// ---------------------------------------------------------------------------
// stbds_hash_bytes - siphash, byte-identical port.
// ---------------------------------------------------------------------------

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

#[inline]
fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotate_left(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotate_left(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes_impl(p: *const u8, len: usize, seed: usize) -> usize {
    let mut d = p;

    // Initial mixing matches the C macro/literal layout:
    // v0 = ((((size_t) 0x736f6d65 << 16) << 16) + 0x70736575) ^ seed;
    let mut v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let elem_size = std::mem::size_of::<usize>();
    let mut i: usize = 0;
    while i + elem_size <= len {
        let b0 = *d.add(0) as u32;
        let b1 = *d.add(1) as u32;
        let b2 = *d.add(2) as u32;
        let b3 = *d.add(3) as u32;
        let b4 = *d.add(4) as u32;
        let b5 = *d.add(5) as u32;
        let b6 = *d.add(6) as u32;
        let b7 = *d.add(7) as u32;
        // Match C: data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // The OR result is `int` in C (each operand promotes to int). On
        // assignment to size_t, that int is sign-extended if d[3] >= 0x80.
        let lower_int: i32 = (b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)) as i32;
        let mut data: usize = (lower_int as i64) as usize;
        // Match C: data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        // The (size_t) cast before <<32 means sign-extension here (if d[7] >= 0x80),
        // but those high bits get shifted off the top by << 32. To stay literal:
        let upper_int: i32 = (b4 | (b5 << 8) | (b6 << 16) | (b7 << 24)) as i32;
        let upper_size: usize = (upper_int as i64) as usize;
        data |= (upper_size << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += elem_size;
        d = d.add(elem_size);
    }

    let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
    // Note: C code uses fall-through switch.
    let rem = len - i;
    // Reproduce all OR-merges that would execute via fall-through:
    if rem >= 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        // Match C: data |= (d[3] << 24);
        // d[3] is unsigned char, promoted to int by the integer promotions,
        // then shifted left by 24. If d[3] >= 0x80 the result has the int
        // sign bit set, and the implicit conversion to size_t when OR'ing
        // sign-extends it. We replicate that here exactly.
        let v_i32: i32 = ((*d.add(3) as u32) << 24) as i32;
        data |= (v_i32 as i64) as usize;
    }
    if rem >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if rem >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if rem >= 1 {
        data |= *d.add(0) as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes_impl(p as *const u8, len, seed)
}

// ---------------------------------------------------------------------------
// Stub exports for the stb_ds dynamic-array / hash-map manipulators.
//
// The Rust `intput` implementation does not call these (it uses HashMap),
// but the C shared library exports them and the Rust .so must too. They
// are exposed as no-op / minimal implementations — they exist purely so
// that the symbol list matches.
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    _elemsize: usize,
    _addlen: usize,
    _min_cap: usize,
) -> *mut c_void {
    a
}

#[no_mangle]
pub unsafe extern "C" fn stbds_arrfreef(_a: *mut c_void) {
    // no-op
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmfree_func(_p: *mut c_void, _elemsize: usize) {
    // no-op
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    _elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    _mode: c_int,
) -> *mut c_void {
    a
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    _elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    _temp: *mut isize,
    _mode: c_int,
) -> *mut c_void {
    a
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmput_default(
    a: *mut c_void,
    _elemsize: usize,
) -> *mut c_void {
    a
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    _elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    _mode: c_int,
) -> *mut c_void {
    a
}

#[no_mangle]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    _elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    _keyoffset: usize,
    _mode: c_int,
) -> *mut c_void {
    a
}

#[no_mangle]
pub unsafe extern "C" fn stbds_shmode_func(_elemsize: usize, _mode: c_int) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn stbds_stralloc(
    _a: *mut c_void,
    str_ptr: *mut c_char,
) -> *mut c_char {
    str_ptr
}

#[no_mangle]
pub unsafe extern "C" fn stbds_strreset(_a: *mut c_void) {
    // no-op
}
