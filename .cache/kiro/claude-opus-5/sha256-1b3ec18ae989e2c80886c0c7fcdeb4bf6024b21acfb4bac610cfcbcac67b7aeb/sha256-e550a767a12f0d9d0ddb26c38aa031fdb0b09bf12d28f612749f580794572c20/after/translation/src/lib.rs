// Rust translation of c_src/src/lib.c
//
// Original copyright 2025 MIT Lincoln Laboratory (MIT license; see c_src).
//
// This crate reproduces the complete public ABI of the C shared library:
//   shift_array, process_string, apply_bitmask, init_matrix,
//   compare_allocations, arity4, arity2, arity3, arity
//
// Behavioral notes / faithfulness decisions:
//  * No NULL checks are added anywhere the C code lacks them.
//  * `compare_allocations` calls the platform `malloc`/`free` so that the
//    pointer-ordering comparison observes real allocator behavior, exactly as
//    the C does (including reading through the `uninit_ptr` alias).
//  * `arity` is defined in lib.c as taking `unsigned char len` while lib.h
//    declares `int len`. GCC compiles the callee to test only the low byte
//    (`cmp $0x1,%dil`), so the parameter is declared here as `c_int` and masked
//    to 8 bits, which reproduces that behavior for any caller.
//  * All signed arithmetic uses explicit wrapping operations to match the
//    two's-complement wrap-around GCC emits, instead of panicking.

use std::ffi::{c_char, c_int, c_uchar, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// C: `typedef struct { int values[4]; int count; char *label; } DataBlock;`
#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut c_char,
}

/// C:
/// ```c
/// void shift_array(int *arr, int size, int positions) {
///     if (positions > 0 && positions < size) {
///         memmove(arr + positions, arr, (size - positions) * sizeof(int));
///         for (int i = 0; i < positions; i++) arr[i] = 0;
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array(arr: *mut c_int, size: c_int, positions: c_int) {
    unsafe {
        if positions > 0 && positions < size {
            let count = (size - positions) as usize;
            // memmove semantics: overlapping copy, source and destination alias.
            std::ptr::copy(arr, arr.add(positions as usize), count);
            for i in 0..positions as usize {
                *arr.add(i) = 0;
            }
        }
    }
}

/// C:
/// ```c
/// int process_string(const char *str) {
///     if (*str) return (int)strlen(str);
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str: *const c_char) -> c_int {
    unsafe {
        if *str != 0 {
            let mut len: usize = 0;
            while *str.add(len) != 0 {
                len += 1;
            }
            len as c_int
        } else {
            0
        }
    }
}

/// C: bitmask switch over four literal masks; `default` returns `value`.
#[unsafe(no_mangle)]
pub extern "C" fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    let mask1: c_int = 0b1111_0000;
    let mask2: c_int = 0b0000_1111;
    let mask3: c_int = 0b1010_1010;
    let mask4: c_int = 0b0101_0101;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

/// C: `void init_matrix(int matrix[3][4])` — the parameter decays to
/// `int (*)[4]`, i.e. `*mut [c_int; 4]` in Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_matrix(matrix: *mut [c_int; 4]) {
    let temp: [[c_int; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];

    unsafe {
        for i in 0..3usize {
            for j in 0..4usize {
                (*matrix.add(i))[j] = temp[i][j];
            }
        }
    }
}

/// C:
/// ```c
/// int compare_allocations(int val1, int val2) {
///     int *ptr1 = malloc(sizeof(int));
///     int *ptr2 = malloc(sizeof(int));
///     int *uninit_ptr;
///     if (ptr1 == NULL || ptr2 == NULL) { free(ptr1); free(ptr2); return -1; }
///     *ptr1 = val1; *ptr2 = val2;
///     int result = 0;
///     if (ptr1 < ptr2) result = 1; else if (ptr1 > ptr2) result = 2; else result = 3;
///     uninit_ptr = ptr1;
///     result += (*uninit_ptr > 0) ? 10 : 0;
///     free(ptr1); free(ptr2);
///     return result;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    unsafe {
        let ptr1 = malloc(std::mem::size_of::<c_int>()) as *mut c_int;
        let ptr2 = malloc(std::mem::size_of::<c_int>()) as *mut c_int;

        let uninit_ptr: *mut c_int;

        if ptr1.is_null() || ptr2.is_null() {
            free(ptr1 as *mut c_void);
            free(ptr2 as *mut c_void);
            return -1;
        }

        *ptr1 = val1;
        *ptr2 = val2;

        let mut result: c_int;

        if ptr1 < ptr2 {
            result = 1;
        } else if ptr1 > ptr2 {
            result = 2;
        } else {
            result = 3;
        }

        uninit_ptr = ptr1;
        result = result.wrapping_add(if *uninit_ptr > 0 { 10 } else { 0 });

        free(ptr1 as *mut c_void);
        free(ptr2 as *mut c_void);

        result
    }
}

/// C: the four-argument entry point that drives every helper above.
#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: std::ptr::null_mut(),
    };

    // char test_str[] = "Hello"; char empty_str[] = "";
    let test_str: [c_char; 6] = [b'H' as c_char, b'e' as c_char, b'l' as c_char, b'l' as c_char, b'o' as c_char, 0];
    let empty_str: [c_char; 1] = [0];

    let len1 = unsafe { process_string(test_str.as_ptr()) };
    let len2 = unsafe { process_string(empty_str.as_ptr()) };

    result = result.wrapping_add(len1.wrapping_add(len2));

    unsafe { shift_array(block.values.as_mut_ptr(), 4, 1) };

    let mut i: c_int = 0;
    while i < block.count {
        result = result.wrapping_add(block.values[i as usize]);
        i += 1;
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix: [[c_int; 4]; 3] = [[0; 4]; 3];
    unsafe { init_matrix(matrix.as_mut_ptr()) };

    result = result.wrapping_add(matrix[0][0].wrapping_add(matrix[2][3]));

    let alloc_result = unsafe { compare_allocations(param1, param2) };
    result = result.wrapping_add(alloc_result);

    if param3 != 0 {
        result = result.wrapping_mul(param3).wrapping_div(100);
    }

    if param4 != 0 {
        result = result.wrapping_add(param4);
    }

    // Suppress "field is never read" for `label`, which exists only for layout.
    let _ = block.label;

    result
}

/// C: `int arity2(int p1, int p2) { return arity4(p1, p2, 0, 0); }`
#[unsafe(no_mangle)]
pub extern "C" fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

/// C: `int arity3(int p1, int p2, int p3) { return arity4(p1, p2, p3, 0); }`
#[unsafe(no_mangle)]
pub extern "C" fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

/// C: `int arity(unsigned char len, int *params)`.
///
/// Declared `int` here to match the public header (`int arity(int, int *)`) and
/// masked to 8 bits to match the `unsigned char` definition that GCC compiled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_int, params: *const c_int) -> c_int {
    let len = (len as u32 & 0xFF) as c_uchar;

    unsafe {
        if len < 2 {
            -1
        } else if len == 2 {
            arity2(*params, *params.add(1))
        } else if len == 3 {
            arity3(*params, *params.add(1), *params.add(2))
        } else {
            arity4(*params, *params.add(1), *params.add(2), *params.add(3))
        }
    }
}
