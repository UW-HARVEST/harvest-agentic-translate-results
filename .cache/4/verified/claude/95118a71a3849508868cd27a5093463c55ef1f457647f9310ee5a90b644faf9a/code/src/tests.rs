//! The test/utility helpers that the C library exports: `strkey` and `arr_del`.

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr::null_mut;

use crate::*;

/// `static char buffer[256];`
static mut buffer: [u8; 256] = [0; 256];

/// ```c
/// char *strkey(int n)
/// {
///    sprintf(buffer, "test_%d", n);
///    return buffer;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf: *mut u8 = (&raw mut buffer) as *mut u8;

    // sprintf(buffer, "test_%d", n) -- "%d" for an int, then a NUL terminator.
    let mut tmp = [0u8; 32];
    let mut len: usize = 0;

    for &c in b"test_" {
        tmp[len] = c;
        len += 1;
    }

    // decimal rendering of a signed 32-bit int (handles INT_MIN)
    let neg = n < 0;
    let mut mag: u32 = if neg {
        (n as i64).unsigned_abs() as u32
    } else {
        n as u32
    };
    let mut digits = [0u8; 10];
    let mut ndigits = 0usize;
    loop {
        digits[ndigits] = b'0' + (mag % 10) as u8;
        ndigits += 1;
        mag /= 10;
        if mag == 0 {
            break;
        }
    }
    if neg {
        tmp[len] = b'-';
        len += 1;
    }
    while ndigits > 0 {
        ndigits -= 1;
        tmp[len] = digits[ndigits];
        len += 1;
    }
    tmp[len] = 0;

    memcpy(
        buf as *mut c_void,
        tmp.as_ptr() as *const c_void,
        len.wrapping_add(1),
    );

    buf as *mut c_char
}

// ---------------------------------------------------------------------------
// Expansions of the array macros for `int *`, as used by arr_del().
// ---------------------------------------------------------------------------

/// `stbds_arrmaybegrow(a,n)` followed by `stbds_arrgrow`.
#[inline]
unsafe fn arrmaybegrow_int(a: &mut *mut c_int, n: usize) {
    if a.is_null()
        || (*stbds_header(*a as *mut c_void)).length.wrapping_add(n)
            > (*stbds_header(*a as *mut c_void)).capacity
    {
        *a = stbds_arrgrowf(*a as *mut c_void, size_of::<c_int>(), n, 0) as *mut c_int;
    }
}

/// `stbds_arrput(a,v)` == `stbds_arrpush(a,v)`
#[inline]
unsafe fn arrput_int(a: &mut *mut c_int, v: c_int) {
    arrmaybegrow_int(a, 1);
    let h = stbds_header(*a as *mut c_void);
    let idx = (*h).length;
    (*h).length = idx.wrapping_add(1);
    *a.wrapping_add(idx) = v;
}

/// `stbds_arrdeln(a,i,n)`
#[inline]
unsafe fn arrdeln_int(a: *mut c_int, i: usize, n: usize) {
    let h = stbds_header(a as *mut c_void);
    memmove(
        a.wrapping_add(i) as *mut c_void,
        a.wrapping_add(i.wrapping_add(n)) as *const c_void,
        size_of::<c_int>().wrapping_mul((*h).length.wrapping_sub(n).wrapping_sub(i)),
    );
    (*h).length = (*h).length.wrapping_sub(n);
}

/// `stbds_arrdel(a,i)`
#[inline]
unsafe fn arrdel_int(a: *mut c_int, i: usize) {
    arrdeln_int(a, i, 1);
}

/// `stbds_arrdelswap(a,i)`
#[inline]
unsafe fn arrdelswap_int(a: *mut c_int, i: usize) {
    let h = stbds_header(a as *mut c_void);
    // (a)[i] = stbds_arrlast(a), stbds_header(a)->length -= 1
    *a.wrapping_add(i) = *a.wrapping_add((*h).length.wrapping_sub(1));
    (*h).length = (*h).length.wrapping_sub(1);
}

/// `stbds_arrfree(a)`
#[inline]
unsafe fn arrfree_int(a: &mut *mut c_int) {
    if !a.is_null() {
        STBDS_FREE(stbds_header(*a as *mut c_void) as *mut c_void);
    }
    *a = null_mut();
}

/// ```c
/// void arr_del(int num)
/// {
///   int *arr=NULL;
///   int i,j;
///
///   for (i=0; i < 4; ++i) {
///     arrpush(arr,num); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
///     arrdel(arr,i);
///     arrfree(arr);
///     arrpush(arr,num); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
///     arrdelswap(arr,i);
///     arrfree(arr);
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(num: c_int) {
    let mut arr: *mut c_int = null_mut();

    let mut i: c_int = 0;
    while i < 4 {
        arrput_int(&mut arr, num);
        arrput_int(&mut arr, 2);
        arrput_int(&mut arr, 3);
        arrput_int(&mut arr, 4);
        arrdel_int(arr, i as usize);
        arrfree_int(&mut arr);
        arrput_int(&mut arr, num);
        arrput_int(&mut arr, 2);
        arrput_int(&mut arr, 3);
        arrput_int(&mut arr, 4);
        arrdelswap_int(arr, i as usize);
        arrfree_int(&mut arr);
        i += 1;
    }
}
