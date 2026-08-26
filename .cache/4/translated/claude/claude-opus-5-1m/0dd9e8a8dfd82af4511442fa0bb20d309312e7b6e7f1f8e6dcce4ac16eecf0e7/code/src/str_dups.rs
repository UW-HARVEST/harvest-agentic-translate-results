//! Translation of the test/driver part at the end of `c_src/src/lib.c`
//! (`strkey` and `str_dups`, plus the file static `buffer`).

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::ffi::*;
use crate::stb_ds::*;

/// `static char buffer[256];`
static mut BUFFER: [c_char; 256] = [0; 256];

/// ```c
/// char *strkey(int n)
/// {
///    sprintf(buffer, "test_%d", n);
///    return buffer;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buffer = ptr::addr_of_mut!(BUFFER) as *mut c_char;
    sprintf(buffer, b"test_%d\0".as_ptr() as *const c_char, n);
    buffer
}

/// The anonymous struct used by `str_dups`:
/// `struct { char *key; int value; } *strmap = NULL, s;`
#[repr(C)]
#[derive(Clone, Copy)]
struct StrMapEntry {
    key: *mut c_char,
    value: c_int,
}

/// ```c
/// void str_dups(int num)
/// ```
#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn str_dups(num: c_int) {
    // `struct { char *key; int value; } *strmap = NULL`
    let mut strmap: *mut StrMapEntry = ptr::null_mut();
    // `s` is uninitialised in C; both members are assigned before any use.
    let mut s = StrMapEntry {
        key: ptr::null_mut(),
        value: 0,
    };
    // stbds_string_arena sa = { 0 };
    let mut sa = stbds_string_arena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut i: c_int;

    i = 0;
    while i < num {
        stbds_stralloc(&mut sa, strkey(i));
        i = i.wrapping_add(1);
    }
    stbds_strreset(&mut sa);

    {
        // s.key = "a", s.value = num;
        s.key = b"a\0".as_ptr() as *mut c_char;
        s.value = num;

        // sh_new_strdup(strmap):
        //   (t) = stbds_shmode_func(sizeof *(t), STBDS_SH_STRDUP)
        strmap = stbds_shmode_func(size_of::<StrMapEntry>(), STBDS_SH_STRDUP) as *mut StrMapEntry;

        // shputs(strmap, s):
        //   (t) = stbds_hmput_key((t), sizeof *(t), (void*) (s).key, sizeof (s).key, STBDS_HM_STRING),
        //   (t)[stbds_temp((t)-1)] = (s),
        //   (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1)
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            size_of::<StrMapEntry>(),
            s.key as *mut c_void,
            size_of::<*mut c_char>(),
            STBDS_HM_STRING,
        ) as *mut StrMapEntry;
        let raw: *mut c_void = strmap.wrapping_sub(1) as *mut c_void;
        *strmap.wrapping_offset((*stbds_header(raw)).temp) = s;
        (*strmap.wrapping_offset((*stbds_header(raw)).temp)).key =
            *((*stbds_header(raw)).hash_table as *mut *mut c_char);

        stbds_assert!(
            *(*strmap.wrapping_offset(0)).key == b'a' as c_char,
            b"*strmap[0].key == 'a'\0",
            960,
            b"str_dups\0"
        );
        stbds_assert!(
            (*strmap.wrapping_offset(0)).key != s.key,
            b"strmap[0].key != s.key\0",
            961,
            b"str_dups\0"
        );
        stbds_assert!(
            (*strmap.wrapping_offset(0)).value == s.value,
            b"strmap[0].value == s.value\0",
            962,
            b"str_dups\0"
        );

        // for (int z=0; z < shlen(strmap); ++z)
        //     printf("%s %d\n", strmap[z], strmap[z].value);
        //
        // shlen(t) == stbds_hmlen(t) == ((t) ? (ptrdiff_t) stbds_header((t)-1)->length-1 : 0)
        //
        // NOTE: the first variadic argument is the *whole* 16 byte struct, which
        // the SysV AMD64 ABI passes in two integer registers: the first holds
        // `key` (consumed by %s), the second holds `value` in its low 32 bits
        // (consumed by %d).  The third argument is never consumed by the format
        // string.  Passing (key, value, value) reproduces this exactly.
        let mut z: c_int = 0;
        while (z as isize)
            < (if !strmap.is_null() {
                ((*stbds_header(raw)).length as isize).wrapping_sub(1)
            } else {
                0
            })
        {
            let e: StrMapEntry = *strmap.wrapping_offset(z as isize);
            printf(
                b"%s %d\n\0".as_ptr() as *const c_char,
                e.key,
                e.value,
                e.value,
            );
            z = z.wrapping_add(1);
        }

        // shfree(strmap):
        //   ((void) ((p) != NULL ? stbds_hmfree_func((p)-1,sizeof*(p)),0 : 0),(p)=NULL)
        if !strmap.is_null() {
            stbds_hmfree_func(
                strmap.wrapping_sub(1) as *mut c_void,
                size_of::<StrMapEntry>(),
            );
        }
        strmap = ptr::null_mut();
        let _ = strmap;
    }
}
