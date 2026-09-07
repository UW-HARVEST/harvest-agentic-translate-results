//! The exercise/driver entry points defined at the bottom of
//! `c_src/src/lib.c`: `strkey` and `str_put`.

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::c;
use crate::hash::{stbds_hmfree_func, stbds_hmput_key};
use crate::strings::{stbds_stralloc, stbds_strreset};
use crate::types::*;

/// `static char buffer[256];` — file-scope, therefore not exported.
static BUFFER: CGlobal<[c_char; 256]> = CGlobal::new([0; 256]);

/// `char *strkey(int n)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buffer = BUFFER.get() as *mut c_char;
    c::sprintf(buffer, c"test_%d".as_ptr(), n);
    buffer
}

/// The anonymous `struct { char *key; int value; }` used by `str_put`.
#[repr(C)]
#[derive(Clone, Copy)]
struct StrMap {
    key: *mut c_char,
    value: c_int,
}

/// `void str_put(int num)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_put(num: c_int) {
    let elemsize = size_of::<StrMap>();

    // struct { char *key; int value; } *strmap = NULL, s;
    let mut strmap: *mut StrMap = ptr::null_mut();
    let s: StrMap;
    // stbds_string_arena sa = { 0 };
    let mut sa = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    // for (i=0; i < num; ++i) stralloc(&sa, strkey(i));
    let mut i: c_int = 0;
    while i < num {
        stbds_stralloc(&raw mut sa, strkey(i));
        i += 1;
    }
    // strreset(&sa);
    stbds_strreset(&raw mut sa);

    // s.key = "a", s.value = num;
    s = StrMap {
        key: c"a".as_ptr() as *mut c_char,
        value: num,
    };

    // shputs(strmap, s):
    //   (t) = stbds_hmput_key((t), sizeof *(t), (void*)(s).key, sizeof (s).key, STBDS_HM_STRING),
    //   (t)[stbds_temp((t)-1)] = (s),
    //   (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1)
    strmap = stbds_hmput_key(
        strmap as *mut c_void,
        elemsize,
        s.key as *mut c_void,
        size_of::<*mut c_char>(),
        STBDS_HM_STRING,
    ) as *mut StrMap;
    let base = strmap.wrapping_sub(1) as *mut c_void;
    *strmap.wrapping_offset(*stbds_temp_ptr(base)) = s;
    (*strmap.wrapping_offset(*stbds_temp_ptr(base))).key = *stbds_temp_key_ptr(base);

    stbds_assert!(
        *(*strmap.wrapping_add(0)).key == b'a' as c_char,
        "*strmap[0].key == 'a'",
        958,
        "str_put"
    );
    stbds_assert!(
        (*strmap.wrapping_add(0)).key == s.key,
        "strmap[0].key == s.key",
        959,
        "str_put"
    );
    stbds_assert!(
        (*strmap.wrapping_add(0)).value == s.value,
        "strmap[0].value == s.value",
        960,
        "str_put"
    );

    // for (int z=0; z < shlen(strmap); ++z)
    //     printf("%s %d\n", strmap[z], strmap[z].value);
    //
    // `strmap[z]` is a 16-byte POD struct, so the SysV AMD64 ABI classifies it
    // as two INTEGER eightbytes and passes it in two registers: the `key`
    // pointer followed by an eightbyte whose low 32 bits hold `value`.  The
    // conversion `%s` therefore consumes `key`, `%d` consumes the second
    // eightbyte (i.e. `value`), and the explicit `strmap[z].value` argument is
    // never read by the format string.  Passing key, value, value reproduces
    // the exact same register state and hence the same output.
    let mut z: c_int = 0;
    while (z as isize) < shlen(strmap) {
        let e = *strmap.wrapping_offset(z as isize);
        c::printf(c"%s %d\n".as_ptr(), e.key, e.value, e.value);
        z += 1;
    }

    // shfree(strmap);
    if !strmap.is_null() {
        stbds_hmfree_func(strmap.wrapping_sub(1) as *mut c_void, elemsize);
    }
    strmap = ptr::null_mut();
    let _ = strmap;
}

/// `stbds_shlen(t)` — `((t) ? (ptrdiff_t) stbds_header((t)-1)->length-1 : 0)`
#[inline]
unsafe fn shlen(t: *mut StrMap) -> isize {
    if !t.is_null() {
        (*stbds_header(t.wrapping_sub(1) as *mut c_void)).length as isize - 1
    } else {
        0
    }
}
