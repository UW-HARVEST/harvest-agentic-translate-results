//! The public entry points at the bottom of `c_src/src/lib.c`
//! (`strkey` and `intput`, the latter being `lib.h`'s only declaration).

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::hmap::{stbds_hmget_key, stbds_hmput_key};
use crate::types::*;

/// `typedef struct { int key,b,c,d; } stbds_struct;`      (unused by the ABI)
/// `typedef struct { int key[2],b,c,d; } stbds_struct2;`  (unused by the ABI)

/// `static char buffer[256];`
static mut BUFFER: [c_char; 256] = [0; 256];

/// ```c
/// char *strkey(int n) { sprintf(buffer, "test_%d", n); return buffer; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = ptr::addr_of_mut!(BUFFER) as *mut u8;

    // sprintf(buffer, "test_%d", n)
    let mut out = 0usize;
    for &c in b"test_" {
        *buf.add(out) = c;
        out += 1;
    }

    // "%d" for a C int, including INT_MIN.
    let mut digits = [0u8; 10];
    let mut ndig = 0usize;
    let mut mag = (n as i64).unsigned_abs();
    if mag == 0 {
        digits[0] = b'0';
        ndig = 1;
    } else {
        while mag > 0 {
            digits[ndig] = b'0' + (mag % 10) as u8;
            ndig += 1;
            mag /= 10;
        }
    }
    if n < 0 {
        *buf.add(out) = b'-';
        out += 1;
    }
    while ndig > 0 {
        ndig -= 1;
        *buf.add(out) = digits[ndig];
        out += 1;
    }
    *buf.add(out) = 0;

    buf as *mut c_char
}

/// `struct { int key; int value; }` — the anonymous map element type of `intput`.
#[repr(C)]
#[derive(Copy, Clone)]
struct IntMapEntry {
    key: c_int,
    value: c_int,
}

const INTMAP_ELEMSIZE: usize = core::mem::size_of::<IntMapEntry>();
const INTMAP_KEYSIZE: usize = core::mem::size_of::<c_int>();

/// `stbds_hmput(t, k, v)`:
/// ```c
/// ((t) = stbds_hmput_key((t), sizeof *(t), (void*) STBDS_ADDRESSOF((t)->key, (k)), sizeof (t)->key, 0),
///  (t)[stbds_temp((t)-1)].key = (k),
///  (t)[stbds_temp((t)-1)].value = (v))
/// ```
unsafe fn hmput(t: &mut *mut IntMapEntry, k: c_int, v: c_int) {
    let mut key_slot: c_int = k; // (int[1]){k}
    *t = stbds_hmput_key(
        *t as *mut c_void,
        INTMAP_ELEMSIZE,
        ptr::addr_of_mut!(key_slot) as *mut c_void,
        INTMAP_KEYSIZE,
        0,
    ) as *mut IntMapEntry;
    let idx = stbds_temp(stbds_hash_to_arr(*t as *mut c_void, INTMAP_ELEMSIZE));
    (*(*t).offset(idx)).key = k;
    let idx = stbds_temp(stbds_hash_to_arr(*t as *mut c_void, INTMAP_ELEMSIZE));
    (*(*t).offset(idx)).value = v;
}

/// `stbds_hmget(t, k)` == `stbds_hmgetp(t,k)->value`, i.e.
/// ```c
/// ((t) = stbds_hmget_key((t), sizeof *(t), (void*) STBDS_ADDRESSOF((t)->key, (k)), sizeof (t)->key, STBDS_HM_BINARY),
///  &(t)[stbds_temp((t)-1)])->value
/// ```
unsafe fn hmget(t: &mut *mut IntMapEntry, k: c_int) -> c_int {
    let mut key_slot: c_int = k; // (int[1]){k}
    *t = stbds_hmget_key(
        *t as *mut c_void,
        INTMAP_ELEMSIZE,
        ptr::addr_of_mut!(key_slot) as *mut c_void,
        INTMAP_KEYSIZE,
        STBDS_HM_BINARY,
    ) as *mut IntMapEntry;
    let idx = stbds_temp(stbds_hash_to_arr(*t as *mut c_void, INTMAP_ELEMSIZE));
    (*(*t).offset(idx)).value
}

/// ```c
/// void intput(int num)
/// {
///   struct { int   key;        int value; }  *intmap  = NULL;
///
///   intmap = NULL;
///   hmput(intmap, num, 7);
///   hmput(intmap, 11, 3);
///   hmput(intmap,  9, num);
///   STBDS_ASSERT(hmget(intmap, 9) == num);
///   STBDS_ASSERT(hmget(intmap, 11) == 3);
///   STBDS_ASSERT(hmget(intmap, num) == 7);
/// }
/// ```
#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn intput(num: c_int) {
    let mut intmap: *mut IntMapEntry = ptr::null_mut();

    intmap = ptr::null_mut();
    hmput(&mut intmap, num, 7);
    hmput(&mut intmap, 11, 3);
    hmput(&mut intmap, 9, num);
    stbds_assert!(
        hmget(&mut intmap, 9) == num,
        "hmget(intmap, 9) == num\0",
        953,
        "intput\0"
    );
    stbds_assert!(
        hmget(&mut intmap, 11) == 3,
        "hmget(intmap, 11) == 3\0",
        954,
        "intput\0"
    );
    stbds_assert!(
        hmget(&mut intmap, num) == 7,
        "hmget(intmap, num) == 7\0",
        955,
        "intput\0"
    );
}
