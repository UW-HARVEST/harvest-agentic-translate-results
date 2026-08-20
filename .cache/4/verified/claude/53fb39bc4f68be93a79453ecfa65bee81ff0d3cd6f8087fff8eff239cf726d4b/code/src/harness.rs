//! Translation of the test-driver part of `c_src/src/lib.c` (`strkey` and
//! `sh_geti`), including the fully expanded `stbds_*` macros.

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::stb_ds::*;
use crate::{printf, stbds_assert};

// ---------------------------------------------------------------------------
// static char buffer[256];
// char *strkey(int n) { sprintf(buffer, "test_%d", n); return buffer; }
// ---------------------------------------------------------------------------

static mut buffer: [c_char; 256] = [0; 256];

#[inline]
fn buffer_ptr() -> *mut c_char {
    (&raw mut buffer) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = buffer_ptr() as *mut u8;
    let mut pos: usize = 0;

    // "test_"
    for &c in b"test_" {
        *buf.add(pos) = c;
        pos += 1;
    }

    // "%d" applied to an int
    let mut digits = [0u8; 24];
    let mut nd: usize = 0;
    let neg = n < 0;
    let mut v: u64 = if neg {
        (n as i64).unsigned_abs()
    } else {
        n as i64 as u64
    };
    if v == 0 {
        digits[nd] = b'0';
        nd += 1;
    }
    while v > 0 {
        digits[nd] = b'0' + (v % 10) as u8;
        nd += 1;
        v /= 10;
    }
    if neg {
        *buf.add(pos) = b'-';
        pos += 1;
    }
    while nd > 0 {
        nd -= 1;
        *buf.add(pos) = digits[nd];
        pos += 1;
    }
    *buf.add(pos) = 0;

    buffer_ptr()
}

// ---------------------------------------------------------------------------
// void sh_geti(int num)
//
//   struct { char *key; int value; } *strmap = NULL, s;
//
// The anonymous struct is 16 bytes wide (8 byte pointer + 4 byte int + 4 byte
// tail padding), so every macro expansion below uses `elemsize == 16` and
// `keysize == sizeof(t->key) == 8`.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct ShEntry {
    key: *mut c_char,
    value: c_int,
}

const ELEMSIZE: usize = size_of::<ShEntry>();
const KEYSIZE: usize = size_of::<*mut c_char>();

/// `(t)-1` for a `ShEntry *`
#[inline]
fn minus_one(t: *mut ShEntry) -> *mut c_void {
    (t as *mut u8).wrapping_sub(ELEMSIZE) as *mut c_void
}

/// `stbds_shgeti(t,k)`
#[inline]
unsafe fn shgeti(t: &mut *mut ShEntry, k: *const c_char) -> isize {
    *t = stbds_hmget_key(
        *t as *mut c_void,
        ELEMSIZE,
        k as *mut c_void,
        KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut ShEntry;
    stbds_temp(minus_one(*t))
}

/// `stbds_shput(t,k,v)`
#[inline]
unsafe fn shput(t: &mut *mut ShEntry, k: *const c_char, v: c_int) {
    *t = stbds_hmput_key(
        *t as *mut c_void,
        ELEMSIZE,
        k as *mut c_void,
        KEYSIZE,
        STBDS_HM_STRING,
    ) as *mut ShEntry;
    let idx = stbds_temp(minus_one(*t));
    (*(*t).offset(idx)).value = v;
}

/// `stbds_shget(t,k)`
#[inline]
unsafe fn shget(t: &mut *mut ShEntry, k: *const c_char) -> c_int {
    let _ = shgeti(t, k);
    let idx = stbds_temp(minus_one(*t));
    (*(*t).offset(idx)).value
}

/// `stbds_shdel(t,k)`
#[inline]
unsafe fn shdel(t: &mut *mut ShEntry, k: *const c_char) -> isize {
    *t = stbds_hmdel_key(
        *t as *mut c_void,
        ELEMSIZE,
        k as *mut c_void,
        KEYSIZE,
        0, // STBDS_OFFSETOF((t),key)
        STBDS_HM_STRING,
    ) as *mut ShEntry;
    if !(*t).is_null() {
        stbds_temp(minus_one(*t))
    } else {
        0
    }
}

/// `stbds_shlen(t)` == `stbds_hmlen(t)`
#[inline]
unsafe fn shlen(t: *mut ShEntry) -> isize {
    if !t.is_null() {
        (*stbds_header(minus_one(t))).length as isize - 1
    } else {
        0
    }
}

/// `stbds_shdefault(t,v)` == `stbds_hmdefault(t,v)`
#[inline]
unsafe fn shdefault(t: &mut *mut ShEntry, v: c_int) {
    *t = stbds_hmput_default(*t as *mut c_void, ELEMSIZE) as *mut ShEntry;
    (*(*t).offset(-1)).value = v;
}

/// `stbds_shfree(t)` == `stbds_hmfree(t)`
#[inline]
unsafe fn shfree(t: &mut *mut ShEntry) {
    if !(*t).is_null() {
        stbds_hmfree_func(minus_one(*t), ELEMSIZE);
    }
    *t = ptr::null_mut();
}

/// `stbds_sh_new_strdup(t)`
#[inline]
unsafe fn sh_new_strdup(t: &mut *mut ShEntry) {
    *t = stbds_shmode_func(ELEMSIZE, STBDS_SH_STRDUP) as *mut ShEntry;
}

/// `stbds_sh_new_arena(t)`
#[inline]
unsafe fn sh_new_arena(t: &mut *mut ShEntry) {
    *t = stbds_shmode_func(ELEMSIZE, STBDS_SH_ARENA) as *mut ShEntry;
}

const FOO: &[u8; 4] = b"foo\0";
const FMT: &[u8; 7] = b"%s %d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_geti(num: c_int) {
    let mut strmap: *mut ShEntry = ptr::null_mut();
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut i: c_int;
    let mut j: c_int;

    let foo = FOO.as_ptr() as *const c_char;

    i = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i += 1;
    }
    stbds_strreset(&mut sa);

    j = 0;
    while j < 2 {
        stbds_assert(
            shgeti(&mut strmap, foo) == -1,
            "assertion failed: shgeti(strmap,\"foo\") == -1\n",
        );
        if j == 0 {
            sh_new_strdup(&mut strmap);
        } else {
            sh_new_arena(&mut strmap);
        }
        stbds_assert(
            shgeti(&mut strmap, foo) == -1,
            "assertion failed: shgeti(strmap,\"foo\") == -1\n",
        );
        shdefault(&mut strmap, -2);
        stbds_assert(
            shgeti(&mut strmap, foo) == -1,
            "assertion failed: shgeti(strmap,\"foo\") == -1\n",
        );
        i = 0;
        while i < num {
            shput(&mut strmap, strkey(i), i.wrapping_mul(3));
            i += 2;
        }

        // printf("%s %d\n", strmap[z], strmap[z].value);
        //
        // The struct argument is passed in two integer registers (key, then
        // value in the low half of the second one), so `%s` consumes the key
        // pointer and `%d` consumes the value; the third argument is never
        // read by printf.
        let mut z: c_int = 0;
        while (z as isize) < shlen(strmap) {
            let e = strmap.offset(z as isize);
            printf(FMT.as_ptr() as *const c_char, (*e).key, (*e).value);
            z += 1;
        }

        i = 0;
        while i < num {
            if i & 1 != 0 {
                stbds_assert(
                    shget(&mut strmap, strkey(i)) == -2,
                    "assertion failed: shget(strmap, strkey(i)) == -2\n",
                );
            } else {
                stbds_assert(
                    shget(&mut strmap, strkey(i)) == i.wrapping_mul(3),
                    "assertion failed: shget(strmap, strkey(i)) == i*3\n",
                );
            }
            i += 1;
        }
        i = 2;
        while i < num {
            shdel(&mut strmap, strkey(i));
            i += 4;
        }
        i = 0;
        while i < num {
            if i & 3 != 0 {
                stbds_assert(
                    shget(&mut strmap, strkey(i)) == -2,
                    "assertion failed: shget(strmap, strkey(i)) == -2\n",
                );
            } else {
                stbds_assert(
                    shget(&mut strmap, strkey(i)) == i.wrapping_mul(3),
                    "assertion failed: shget(strmap, strkey(i)) == i*3\n",
                );
            }
            i += 1;
        }
        i = 0;
        while i < num {
            shdel(&mut strmap, strkey(i));
            i += 1;
        }
        i = 0;
        while i < num {
            stbds_assert(
                shget(&mut strmap, strkey(i)) == -2,
                "assertion failed: shget(strmap, strkey(i)) == -2\n",
            );
            i += 1;
        }

        shfree(&mut strmap);

        j += 1;
    }
}
