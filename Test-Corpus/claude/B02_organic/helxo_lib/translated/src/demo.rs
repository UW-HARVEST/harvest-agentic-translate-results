//! The demo/test helpers that the C file also exports: `strkey` and `helxo`.

use core::ffi::{c_char, c_int, c_void};

use crate::ffi::*;
use crate::hashmap::{stbds_hmfree_func, stbds_hmput_key};

/// `typedef struct { int key,b,c,d; } stbds_struct;` (unused by the exported API,
/// kept for documentation parity with the C source)
#[repr(C)]
#[allow(dead_code)]
struct StbdsStruct {
    key: c_int,
    b: c_int,
    c: c_int,
    d: c_int,
}

/// `typedef struct { int key[2],b,c,d; } stbds_struct2;`
#[repr(C)]
#[allow(dead_code)]
struct StbdsStruct2 {
    key: [c_int; 2],
    b: c_int,
    c: c_int,
    d: c_int,
}

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
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buffer = core::ptr::addr_of_mut!(BUFFER) as *mut c_char;
        sprintf(buffer, b"test_%d\0".as_ptr() as *const c_char, n);
        buffer
    }
}

/// The anonymous struct used by `helxo`:
/// `struct { char *key; char value; } *hash = NULL;`
#[repr(C)]
struct ShEntry {
    key: *mut c_char,
    value: c_char,
}

/// ```c
/// void helxo(char letter)
/// {
///   {
///     struct { char *key; char value; } *hash = NULL;
///     char name[4] = "jen";
///     shput(hash, "bob"   , 'h');
///     shput(hash, "sally" , 'e');
///     shput(hash, "fred"  , 'l');
///     shput(hash, "jen"   , 'x');
///     shput(hash, "doug"  , 'o');
///
///     shput(hash, name    , letter);
///
///     for (int z=0; z < shlen(hash); ++z)
///        printf("%s %c\n", hash[z], hash[z].value);
///
///     shfree(hash);
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    unsafe {
        let elemsize = core::mem::size_of::<ShEntry>(); // sizeof *(t)
        let keysize = core::mem::size_of::<*mut c_char>(); // sizeof (t)->key

        let mut hash: *mut ShEntry = core::ptr::null_mut();
        let mut name: [c_char; 4] = [b'j' as c_char, b'e' as c_char, b'n' as c_char, 0];

        // #define stbds_shput(t, k, v)
        //   ((t) = stbds_hmput_key((t), sizeof *(t), (void*) (k), sizeof (t)->key, STBDS_HM_STRING),
        //    (t)[stbds_temp((t)-1)].value = (v))
        let shput = |t: &mut *mut ShEntry, k: *mut c_char, v: c_char| {
            *t = stbds_hmput_key(
                *t as *mut c_void,
                elemsize,
                k as *mut c_void,
                keysize,
                STBDS_HM_STRING,
            ) as *mut ShEntry;
            let idx = (*stbds_header(stbds_hash_to_arr(*t as *mut c_void, elemsize))).temp;
            (*(*t).offset(idx)).value = v;
        };

        shput(
            &mut hash,
            b"bob\0".as_ptr() as *mut c_char,
            b'h' as c_char,
        );
        shput(
            &mut hash,
            b"sally\0".as_ptr() as *mut c_char,
            b'e' as c_char,
        );
        shput(
            &mut hash,
            b"fred\0".as_ptr() as *mut c_char,
            b'l' as c_char,
        );
        shput(
            &mut hash,
            b"jen\0".as_ptr() as *mut c_char,
            b'x' as c_char,
        );
        shput(
            &mut hash,
            b"doug\0".as_ptr() as *mut c_char,
            b'o' as c_char,
        );

        shput(&mut hash, name.as_mut_ptr(), letter);

        // #define stbds_shlen(t) ((t) ? (ptrdiff_t) stbds_header((t)-1)->length-1 : 0)
        let shlen = |t: *mut ShEntry| -> isize {
            if !t.is_null() {
                (*stbds_header(stbds_hash_to_arr(t as *mut c_void, elemsize))).length as isize - 1
            } else {
                0
            }
        };

        let mut z: c_int = 0;
        while (z as isize) < shlen(hash) {
            // printf("%s %c\n", hash[z], hash[z].value);
            // The struct is passed by value: under the SysV ABI its first
            // eightbyte (the key pointer) satisfies "%s" and its second
            // eightbyte (holding `value`) satisfies "%c".
            let e = hash.offset(z as isize);
            printf(
                b"%s %c\n\0".as_ptr() as *const c_char,
                (*e).key,
                (*e).value as c_int,
            );
            z += 1;
        }

        // #define stbds_hmfree(p) ((void) ((p) != NULL ? stbds_hmfree_func((p)-1,sizeof*(p)),0 : 0),(p)=NULL)
        if !hash.is_null() {
            stbds_hmfree_func(hash.offset(-1) as *mut c_void, elemsize);
        }
        hash = core::ptr::null_mut();
        let _ = hash;
    }
}
