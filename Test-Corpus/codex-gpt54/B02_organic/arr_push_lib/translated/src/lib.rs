use std::ffi::{c_int, c_void};
use std::mem::size_of;
use std::ptr;

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

unsafe fn stbds_header(a: *mut c_int) -> *mut StbdsArrayHeader {
    a.cast::<StbdsArrayHeader>().sub(1)
}

unsafe fn stbds_arrlen(a: *mut c_int) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length
    }
}

unsafe fn stbds_arrcap(a: *mut c_int) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

unsafe fn stbds_arrgrowf(
    a: *mut c_int,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_int {
    let min_len = stbds_arrlen(a).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2usize.wrapping_mul(stbds_arrcap(a)) {
        min_cap = 2usize.wrapping_mul(stbds_arrcap(a));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let header_size = size_of::<StbdsArrayHeader>();
    let old_ptr = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a).cast::<c_void>()
    };
    let new_size = elemsize.wrapping_mul(min_cap).wrapping_add(header_size);
    let b = realloc(old_ptr, new_size).cast::<u8>();
    let data = b.add(header_size).cast::<c_int>();

    if a.is_null() {
        (*stbds_header(data)).length = 0;
        (*stbds_header(data)).hash_table = ptr::null_mut();
        (*stbds_header(data)).temp = 0;
    }

    (*stbds_header(data)).capacity = min_cap;
    data
}

unsafe fn stbds_arrmaybegrow(a: *mut c_int, n: usize) -> *mut c_int {
    if a.is_null() || (*stbds_header(a)).length.wrapping_add(n) > (*stbds_header(a)).capacity {
        stbds_arrgrowf(a, size_of::<c_int>(), n, 0)
    } else {
        a
    }
}

unsafe fn stbds_arrput(a: *mut c_int, v: c_int) -> *mut c_int {
    let a = stbds_arrmaybegrow(a, 1);
    let header = stbds_header(a);
    let index = (*header).length;
    *a.add(index) = v;
    (*header).length = index.wrapping_add(1);
    a
}

unsafe fn stbds_arrfree(a: *mut c_int) {
    if !a.is_null() {
        free(stbds_header(a).cast::<c_void>());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_push(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();
    let mut i: c_int = 0;

    unsafe {
        assert_eq!(stbds_arrlen(arr), 0);

        while i < num {
            let mut j: c_int = 0;
            while j < i {
                arr = stbds_arrput(arr, j);
                j += 1;
            }

            stbds_arrfree(arr);
            arr = ptr::null_mut();
            i += 50;
        }
    }
}
