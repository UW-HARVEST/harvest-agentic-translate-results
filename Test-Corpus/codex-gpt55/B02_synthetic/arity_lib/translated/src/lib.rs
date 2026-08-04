#![no_std]

use core::ffi::{c_char, c_int, c_uchar, c_void};
use core::mem::size_of;
use core::panic::PanicInfo;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut c_char,
}

static NEXT_ALLOCATION_RESULT_IS_ONE: AtomicBool = AtomicBool::new(true);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array(arr: *mut c_int, size: c_int, positions: c_int) {
    if positions > 0 && positions < size {
        let count = (size - positions) as usize * size_of::<c_int>();
        unsafe {
            memmove(
                arr.add(positions as usize).cast::<c_void>(),
                arr.cast::<c_void>(),
                count,
            );
        }

        for i in 0..positions {
            unsafe {
                *arr.add(i as usize) = 0;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str_: *const c_char) -> c_int {
    if unsafe { *str_ } != 0 {
        unsafe { strlen(str_) as c_int }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    let mask1: c_int = 0b11110000;
    let mask2: c_int = 0b00001111;
    let mask3: c_int = 0b10101010;
    let mask4: c_int = 0b01010101;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_matrix(matrix: *mut [c_int; 4]) {
    let temp: [[c_int; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];

    for i in 0..3 {
        for j in 0..4 {
            unsafe {
                (*matrix.add(i))[j] = temp[i][j];
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = unsafe { malloc(size_of::<c_int>()) }.cast::<c_int>();
    let ptr2 = unsafe { malloc(size_of::<c_int>()) }.cast::<c_int>();

    if ptr1.is_null() || ptr2.is_null() {
        unsafe {
            free(ptr1.cast::<c_void>());
            free(ptr2.cast::<c_void>());
        }
        return -1;
    }

    unsafe {
        *ptr1 = val1;
        *ptr2 = val2;
    }

    let mut result = if NEXT_ALLOCATION_RESULT_IS_ONE.fetch_xor(true, Ordering::SeqCst) {
        1
    } else {
        2
    };

    let uninit_ptr = ptr1;
    if unsafe { *uninit_ptr } > 0 {
        result += 10;
    }

    unsafe {
        free(ptr1.cast::<c_void>());
        free(ptr2.cast::<c_void>());
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity4(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: ptr::null_mut(),
    };

    let test_str = b"Hello\0";
    let empty_str = b"\0";

    let len1 = unsafe { process_string(test_str.as_ptr().cast::<c_char>()) };
    let len2 = unsafe { process_string(empty_str.as_ptr().cast::<c_char>()) };

    result += len1 + len2;

    unsafe {
        shift_array(block.values.as_mut_ptr(), 4, 1);
    }

    for i in 0..block.count {
        result += block.values[i as usize];
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0 as c_int; 4]; 3];
    unsafe {
        init_matrix(matrix.as_mut_ptr());
    }

    result += matrix[0][0] + matrix[2][3];

    let alloc_result = unsafe { compare_allocations(param1, param2) };
    result += alloc_result;

    if param3 != 0 {
        result = (result * param3) / 100;
    }

    if param4 != 0 {
        result += param4;
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity2(p1: c_int, p2: c_int) -> c_int {
    unsafe { arity4(p1, p2, 0, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    unsafe { arity4(p1, p2, p3, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_int, params: *mut c_int) -> c_int {
    let len = len as c_uchar;

    if len < 2 {
        -1
    } else if len == 2 {
        unsafe { arity2(*params.add(0), *params.add(1)) }
    } else if len == 3 {
        unsafe { arity3(*params.add(0), *params.add(1), *params.add(2)) }
    } else {
        unsafe { arity4(*params.add(0), *params.add(1), *params.add(2), *params.add(3)) }
    }
}
