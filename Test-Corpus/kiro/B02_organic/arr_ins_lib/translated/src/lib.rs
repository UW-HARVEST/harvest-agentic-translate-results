#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    clippy::all
)]

use std::ffi::c_int;
use std::ptr;

// ---- stbds_array_header ----
#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut u8,
    temp: isize,
}

unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

// realloc via libc
unsafe fn stbds_realloc(p: *mut u8, s: usize) -> *mut u8 {
    libc::realloc(p as *mut libc::c_void, s) as *mut u8
}

unsafe fn stbds_free(p: *mut u8) {
    libc::free(p as *mut libc::c_void);
}

// ---- stbds_arrgrowf ----
unsafe fn stbds_arrgrowf(
    a: *mut u8,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut u8 {
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    let double_cap = stbds_arrcap(a).wrapping_mul(2);
    if min_cap < double_cap {
        min_cap = double_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(std::mem::size_of::<stbds_array_header>());
    let old_ptr = if !a.is_null() {
        stbds_header(a) as *mut u8
    } else {
        ptr::null_mut()
    };
    let b_raw = stbds_realloc(old_ptr, alloc_size);
    let b = b_raw.add(std::mem::size_of::<stbds_array_header>());
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;
    b
}

// ---- arr_ins: the only public function ----
// The C code does:
//   void arr_ins(int num) {
//     int *arr=NULL;
//     for (i=0; i<5; ++i) {
//       arrpush(arr,1); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
//       stbds_arrins(arr,i,num);
//       assert(arr[i] == num);
//       if (i < 4) assert(arr[4] == 4);
//       arrfree(arr);
//     }
//   }
//
// Macros expand to direct pointer manipulation on the stbds array.

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    unsafe {
        let elemsize = std::mem::size_of::<c_int>();
        let mut arr: *mut c_int = ptr::null_mut();

        for i in 0..5i32 {
            // arrpush(arr, 1..4)
            for v in 1..=4i32 {
                // stbds_arrmaybegrow
                let a = arr as *mut u8;
                if a.is_null()
                    || (*stbds_header(a)).length + 1 > (*stbds_header(a)).capacity
                {
                    arr = stbds_arrgrowf(a, elemsize, 1, 0) as *mut c_int;
                }
                let a = arr as *mut u8;
                let idx = (*stbds_header(a)).length;
                (*stbds_header(a)).length += 1;
                *arr.add(idx) = v;
            }

            // stbds_arrins(arr, i, num)
            // expands to: stbds_arrinsn(arr, i, 1), arr[i] = num
            // stbds_arrinsn(a,i,n) = stbds_arraddn(a,n), memmove(...)
            // stbds_arraddn(a,n) = stbds_arraddnindex(a,n) (result discarded)
            // stbds_arraddnindex(a,n) = stbds_arrmaybegrow(a,n), length+=n, length-n
            {
                let n: usize = 1;
                let a = arr as *mut u8;
                if a.is_null()
                    || (*stbds_header(a)).length + n > (*stbds_header(a)).capacity
                {
                    arr = stbds_arrgrowf(a, elemsize, n, 0) as *mut c_int;
                }
                let a = arr as *mut u8;
                (*stbds_header(a)).length += n;

                // memmove(&arr[i+n], &arr[i], sizeof(int) * (length - n - i))
                let len = (*stbds_header(a)).length;
                let count = len - n - (i as usize);
                ptr::copy(
                    arr.add(i as usize),
                    arr.add(i as usize + n),
                    count,
                );
                *arr.add(i as usize) = num;
            }

            // assert(arr[i] == num)
            assert_eq!(*arr.add(i as usize), num);
            // if (i < 4) assert(arr[4] == 4)
            if i < 4 {
                assert_eq!(*arr.add(4), 4);
            }

            // arrfree(arr)
            if !arr.is_null() {
                stbds_free(stbds_header(arr as *mut u8) as *mut u8);
            }
            arr = ptr::null_mut();
        }
    }
}
