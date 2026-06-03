// Rust translation of c_src/src/lib.c
//
// The only publicly exported function from the C library is `arr_push`, which
// exercises an stb_ds-like dynamic array. The full stb_ds machinery is not
// reachable from outside the library, so we translate the publicly exposed
// behavior using idiomatic Rust collections, while still providing translations
// for the helper routines for completeness.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_int;
use std::os::raw::c_char;
use std::ptr;

// --- stb_ds-like constants -------------------------------------------------

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;
const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() * 8) as u32;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

#[inline]
fn rotate_left_usize(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotate_right_usize(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// --- Hash seed -------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x31415926;

#[no_mangle]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

// --- Hashing ---------------------------------------------------------------

/// Translated equivalent of `stbds_hash_string`.
///
/// # Safety
/// `str_ptr` must point to a NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn stbds_hash_string(str_ptr: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    let mut p = str_ptr;
    while *p != 0 {
        hash = rotate_left_usize(hash, 9).wrapping_add(*(p as *mut u8) as usize);
        p = p.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right_usize(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right_usize(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right_usize(hash, 22);
    hash.wrapping_add(seed)
}

#[inline]
fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    // SipHash 2-4. The C code asserts that this is only used in 64-bit builds.
    debug_assert!(std::mem::size_of::<usize>() == 8);

    let mut v0: usize = (((0x736f6d65usize) << 16) << 16)
        .wrapping_add(0x70736575)
        ^ seed;
    let mut v1: usize = (((0x646f7261usize) << 16) << 16)
        .wrapping_add(0x6e646f6d)
        ^ !seed;
    let mut v2: usize = (((0x6c796765usize) << 16) << 16)
        .wrapping_add(0x6e657261)
        ^ seed;
    let mut v3: usize = (((0x74656462usize) << 16) << 16)
        .wrapping_add(0x79746573)
        ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    macro_rules! sip_round {
        () => {{
            v0 = v0.wrapping_add(v1);
            v1 = rotate_left_usize(v1, 13);
            v1 ^= v0;
            v0 = rotate_left_usize(v0, STBDS_SIZE_T_BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = rotate_left_usize(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotate_left_usize(v1, 17);
            v1 ^= v2;
            v2 = rotate_left_usize(v2, STBDS_SIZE_T_BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = rotate_left_usize(v3, 21);
            v3 ^= v0;
        }};
    }

    let mut d = p;
    let mut i: usize = 0;
    let word = std::mem::size_of::<usize>();
    let mut data: usize;

    while i + word <= len {
        unsafe {
            data = *d as usize
                | ((*d.add(1) as usize) << 8)
                | ((*d.add(2) as usize) << 16)
                | ((*d.add(3) as usize) << 24);
            data |= (((*d.add(4) as usize)
                | ((*d.add(5) as usize) << 8)
                | ((*d.add(6) as usize) << 16)
                | ((*d.add(7) as usize) << 24))
                << 16)
                << 16;
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sip_round!();
        }
        v0 ^= data;

        i += word;
        unsafe {
            d = d.add(word);
        }
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let remaining = len - i;
    unsafe {
        // Fall-through behavior matches the C switch.
        if remaining >= 7 {
            data |= ((*d.add(6) as usize) << 24) << 24;
        }
        if remaining >= 6 {
            data |= ((*d.add(5) as usize) << 20) << 20;
        }
        if remaining >= 5 {
            data |= ((*d.add(4) as usize) << 16) << 16;
        }
        if remaining >= 4 {
            data |= (*d.add(3) as usize) << 24;
        }
        if remaining >= 3 {
            data |= (*d.add(2) as usize) << 16;
        }
        if remaining >= 2 {
            data |= (*d.add(1) as usize) << 8;
        }
        if remaining >= 1 {
            data |= *d as usize;
        }
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        sip_round!();
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        sip_round!();
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// Translated equivalent of `stbds_hash_bytes`.
///
/// # Safety
/// `p` must be valid for `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// --- arr_push --------------------------------------------------------------
//
// The C `arr_push` exercises an stb_ds dynamic int array. In Rust we can
// express the same behavior idiomatically with `Vec`. This preserves the
// observable side effects (allocation/growth/free pattern, no mutation of
// global state) while providing the same C ABI signature.

/// Translated equivalent of `arr_push(int num)`.
#[no_mangle]
pub extern "C" fn arr_push(num: c_int) {
    // Mirrors the C version exactly:
    //   int *arr = NULL;
    //   STBDS_ASSERT(arrlen(arr) == 0);
    //   for (i = 0; i < num; i += 50) {
    //       for (j = 0; j < i; ++j) arrpush(arr, j);
    //       arrfree(arr);
    //   }
    let mut arr: Vec<c_int> = Vec::new();
    debug_assert_eq!(arr.len(), 0);

    let mut i: c_int = 0;
    while i < num {
        for j in 0..i {
            arr.push(j);
        }
        // arrfree(arr) - drop and recreate to mirror free-and-reset behavior.
        arr.clear();
        arr.shrink_to_fit();
        // arr is conceptually NULL again; equivalent to letting `arr` be empty.
        i += 50;
    }
    let _ = arr;
}

// --- strkey ----------------------------------------------------------------

static mut BUFFER: [c_char; 256] = [0; 256];

/// Translated equivalent of `strkey(int n)`.
///
/// Returns a pointer to a static buffer; not thread-safe (matches C version).
#[no_mangle]
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    use std::io::Write;
    unsafe {
        // Format "test_%d" into BUFFER. Use a small stack helper.
        let mut tmp = [0u8; 256];
        let written = {
            let mut cursor = &mut tmp[..];
            let _ = write!(cursor, "test_{}\0", n);
            // figure out the length up to (and including) the NUL we wrote
            tmp.iter().position(|&b| b == 0).unwrap_or(tmp.len() - 1) + 1
        };
        // Copy into the static buffer.
        let dst = ptr::addr_of_mut!(BUFFER) as *mut u8;
        ptr::copy_nonoverlapping(tmp.as_ptr(), dst, written.min(256));
        ptr::addr_of_mut!(BUFFER) as *mut c_char
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arr_push_smoke() {
        arr_push(0);
        arr_push(1);
        arr_push(50);
        arr_push(200);
    }

    #[test]
    fn hash_string_basic() {
        unsafe {
            let s = b"hello\0".as_ptr() as *mut c_char;
            let h1 = stbds_hash_string(s, 0);
            let h2 = stbds_hash_string(s, 0);
            assert_eq!(h1, h2);
        }
    }
}
