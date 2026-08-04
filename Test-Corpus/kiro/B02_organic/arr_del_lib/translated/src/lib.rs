use std::alloc::{self, Layout};
use std::ffi::c_int;
use std::ptr;

/// Mirrors the C stbds_array_header struct layout.
/// Placed immediately before the data pointer in memory.
#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut u8,
    temp: isize,
}

/// Get a pointer to the header preceding the data pointer `t`.
unsafe fn stbds_header<T>(t: *mut T) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).offset(-1)
}

/// Get the length of the array (or 0 if null).
unsafe fn stbds_arrlen<T>(a: *mut T) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// Get the capacity of the array (or 0 if null).
unsafe fn stbds_arrcap<T>(a: *mut T) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

/// Grow the array. Mirrors stbds_arrgrowf from the C code.
unsafe fn stbds_arrgrowf<T>(a: *mut T, addlen: usize, min_cap: usize) -> *mut T {
    let elemsize = std::mem::size_of::<T>();
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize) + addlen;

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + std::mem::size_of::<StbdsArrayHeader>();
    let layout = Layout::from_size_align(alloc_size, std::mem::align_of::<StbdsArrayHeader>()).unwrap();

    let b: *mut u8 = if a.is_null() {
        alloc::alloc(layout)
    } else {
        let old_alloc_size =
            elemsize * (*stbds_header(a)).capacity + std::mem::size_of::<StbdsArrayHeader>();
        let old_layout =
            Layout::from_size_align(old_alloc_size, std::mem::align_of::<StbdsArrayHeader>())
                .unwrap();
        alloc::realloc(stbds_header(a) as *mut u8, old_layout, alloc_size)
    };

    let data = b.add(std::mem::size_of::<StbdsArrayHeader>()) as *mut T;

    if a.is_null() {
        (*stbds_header(data)).length = 0;
        (*stbds_header(data)).hash_table = ptr::null_mut();
        (*stbds_header(data)).temp = 0;
    }

    (*stbds_header(data)).capacity = min_cap;

    data
}

/// Free the array.
unsafe fn stbds_arrfree<T>(a: *mut T) {
    if !a.is_null() {
        let hdr = stbds_header(a);
        let alloc_size = std::mem::size_of::<T>() * (*hdr).capacity
            + std::mem::size_of::<StbdsArrayHeader>();
        let layout =
            Layout::from_size_align(alloc_size, std::mem::align_of::<StbdsArrayHeader>()).unwrap();
        alloc::dealloc(hdr as *mut u8, layout);
    }
}

/// Push a value onto the array, growing if needed.
unsafe fn arrpush(a: &mut *mut c_int, v: c_int) {
    // arrmaybegrow
    if (*a).is_null() || (*stbds_header(*a)).length + 1 > (*stbds_header(*a)).capacity {
        *a = stbds_arrgrowf(*a, 1, 0);
    }
    let hdr = stbds_header(*a);
    let idx = (*hdr).length;
    *(*a).add(idx) = v;
    (*hdr).length += 1;
}

/// Delete element at index i by shifting (memmove). Mirrors stbds_arrdeln(a,i,1).
unsafe fn arrdel(a: *mut c_int, i: usize) {
    let hdr = stbds_header(a);
    let n: usize = 1;
    let count = (*hdr).length - n - i;
    ptr::copy(a.add(i + n), a.add(i), count);
    (*hdr).length -= n;
}

/// Delete element at index i by swapping with last. Mirrors stbds_arrdelswap(a,i).
unsafe fn arrdelswap(a: *mut c_int, i: usize) {
    let hdr = stbds_header(a);
    let last = (*hdr).length - 1;
    *a.add(i) = *a.add(last);
    (*hdr).length -= 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    unsafe {
        let mut arr: *mut c_int = ptr::null_mut();

        for i in 0..4u32 {
            arrpush(&mut arr, num);
            arrpush(&mut arr, 2);
            arrpush(&mut arr, 3);
            arrpush(&mut arr, 4);
            arrdel(arr, i as usize);
            stbds_arrfree(arr);
            arr = ptr::null_mut();

            arrpush(&mut arr, num);
            arrpush(&mut arr, 2);
            arrpush(&mut arr, 3);
            arrpush(&mut arr, 4);
            arrdelswap(arr, i as usize);
            stbds_arrfree(arr);
            arr = ptr::null_mut();
        }
    }
}
