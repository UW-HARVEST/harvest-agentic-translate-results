//! The small test driver that lives at the bottom of the C translation unit:
//! `strkey` and `hm_geti`.
//!
//! `hm_geti` is a macro-heavy function in C; the macro expansions
//! (`hmgeti`, `hmput`, `hmget`, `hmget_ts`, `hmdel`, `hmdefault`, `hmfree`)
//! are inlined here as small helpers that mirror them one-to-one.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::hashmap::{
    stbds_hmdel_key, stbds_hmfree_func, stbds_hmget_key, stbds_hmget_key_ts, stbds_hmput_default,
    stbds_hmput_key,
};
use crate::{hash_to_arr, stbds_assert, temp_get, STBDS_HM_BINARY};

/// `static char buffer[256];`
static mut buffer: [c_char; 256] = [0; 256];

/// ```c
/// char *strkey(int n) { sprintf(buffer, "test_%d", n); return buffer; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let base = ptr::addr_of_mut!(buffer) as *mut u8;

    let mut digits = [0u8; 24];
    let mut ndigits = 0usize;
    let negative = n < 0;
    let mut v: u32 = (n as i32).unsigned_abs();
    if v == 0 {
        digits[ndigits] = b'0';
        ndigits += 1;
    }
    while v > 0 {
        digits[ndigits] = b'0' + (v % 10) as u8;
        ndigits += 1;
        v /= 10;
    }

    let mut o = 0usize;
    for &c in b"test_" {
        *base.add(o) = c;
        o += 1;
    }
    if negative {
        *base.add(o) = b'-';
        o += 1;
    }
    let mut k = ndigits;
    while k > 0 {
        k -= 1;
        *base.add(o) = digits[k];
        o += 1;
    }
    *base.add(o) = 0;

    base as *mut c_char
}

/// `struct { int key; int value; }` as used by `hm_geti`.
#[repr(C)]
#[derive(Copy, Clone)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

const ELEMSIZE: usize = core::mem::size_of::<IntMapEntry>();
const KEYSIZE: usize = core::mem::size_of::<c_int>();

/// `stbds_temp((t)-1)`
#[inline(always)]
unsafe fn im_temp(t: *mut IntMapEntry) -> isize {
    temp_get(hash_to_arr(t as *mut c_void, ELEMSIZE))
}

/// `stbds_hmgeti(t, k)`
#[inline(always)]
unsafe fn hmgeti(t: &mut *mut IntMapEntry, k: c_int) -> isize {
    let mut key = k;
    *t = stbds_hmget_key(
        *t as *mut c_void,
        ELEMSIZE,
        &mut key as *mut c_int as *mut c_void,
        KEYSIZE,
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    im_temp(*t)
}

/// `stbds_hmget(t, k)`
#[inline(always)]
unsafe fn hmget(t: &mut *mut IntMapEntry, k: c_int) -> c_int {
    let _ = hmgeti(t, k);
    (*(*t).wrapping_offset(im_temp(*t))).value
}

/// `stbds_hmgeti_ts(t, k, temp)` / `stbds_hmget_ts(t, k, temp)`
#[inline(always)]
unsafe fn hmget_ts(t: &mut *mut IntMapEntry, k: c_int, temp: &mut isize) -> c_int {
    let mut key = k;
    *t = stbds_hmget_key_ts(
        *t as *mut c_void,
        ELEMSIZE,
        &mut key as *mut c_int as *mut c_void,
        KEYSIZE,
        temp as *mut isize,
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    (*(*t).wrapping_offset(*temp)).value
}

/// `stbds_hmput(t, k, v)`
#[inline(always)]
unsafe fn hmput(t: &mut *mut IntMapEntry, k: c_int, v: c_int) {
    let mut key = k;
    *t = stbds_hmput_key(
        *t as *mut c_void,
        ELEMSIZE,
        &mut key as *mut c_int as *mut c_void,
        KEYSIZE,
        0,
    ) as *mut IntMapEntry;
    (*(*t).wrapping_offset(im_temp(*t))).key = k;
    (*(*t).wrapping_offset(im_temp(*t))).value = v;
}

/// `stbds_hmdefault(t, v)`
#[inline(always)]
unsafe fn hmdefault(t: &mut *mut IntMapEntry, v: c_int) {
    *t = stbds_hmput_default(*t as *mut c_void, ELEMSIZE) as *mut IntMapEntry;
    (*(*t).wrapping_offset(-1)).value = v;
}

/// `stbds_hmdel(t, k)`
#[inline(always)]
unsafe fn hmdel(t: &mut *mut IntMapEntry, k: c_int) -> isize {
    let mut key = k;
    *t = stbds_hmdel_key(
        *t as *mut c_void,
        ELEMSIZE,
        &mut key as *mut c_int as *mut c_void,
        KEYSIZE,
        0, // STBDS_OFFSETOF(t, key)
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    if !(*t).is_null() {
        im_temp(*t)
    } else {
        0
    }
}

/// `stbds_hmfree(p)`
#[inline(always)]
unsafe fn hmfree(t: &mut *mut IntMapEntry) {
    if !(*t).is_null() {
        stbds_hmfree_func((*t).wrapping_offset(-1) as *mut c_void, ELEMSIZE);
    }
    *t = ptr::null_mut();
}

/// ```c
/// void hm_geti(int num)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_geti(num: c_int) {
    let mut intmap: *mut IntMapEntry = ptr::null_mut();
    let mut temp: isize = 0;
    let mut i: c_int;

    i = 1;
    stbds_assert!(hmgeti(&mut intmap, i) == -1);
    hmdefault(&mut intmap, -2);
    stbds_assert!(hmgeti(&mut intmap, i) == -1);
    stbds_assert!(hmget(&mut intmap, i) == -2);

    i = 0;
    while i < num {
        hmput(&mut intmap, i, i.wrapping_mul(5));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            stbds_assert!(hmget(&mut intmap, i) == -2);
        } else {
            stbds_assert!(hmget(&mut intmap, i) == i.wrapping_mul(5));
        }
        if (i & 1) != 0 {
            stbds_assert!(hmget_ts(&mut intmap, i, &mut temp) == -2);
        } else {
            stbds_assert!(hmget_ts(&mut intmap, i, &mut temp) == i.wrapping_mul(5));
        }
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        hmput(&mut intmap, i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            stbds_assert!(hmget(&mut intmap, i) == -2);
        } else {
            stbds_assert!(hmget(&mut intmap, i) == i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 2;
    while i < num {
        hmdel(&mut intmap, i);
        i = i.wrapping_add(4);
    }

    i = 0;
    while i < num {
        if (i & 3) != 0 {
            stbds_assert!(hmget(&mut intmap, i) == -2);
        } else {
            stbds_assert!(hmget(&mut intmap, i) == i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        hmdel(&mut intmap, i);
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        stbds_assert!(hmget(&mut intmap, i) == -2);
        i = i.wrapping_add(1);
    }

    hmfree(&mut intmap);

    i = 0;
    while i < num {
        hmput(&mut intmap, i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }

    hmfree(&mut intmap);
}
