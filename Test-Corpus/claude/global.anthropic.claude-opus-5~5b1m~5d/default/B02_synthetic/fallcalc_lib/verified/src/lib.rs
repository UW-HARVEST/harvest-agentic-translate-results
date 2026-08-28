// Rust translation of c_src/src/lib.c (public API in c_src/include/lib.h).
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(clippy::missing_safety_doc)]

use core::hint::black_box;
use core::mem::size_of;
use std::ffi::{c_double, c_int, c_void};

// #define OCTAL_MASK_1 0777
const OCTAL_MASK_1: c_int = 0o777;
// #define OCTAL_MASK_2 0100
const OCTAL_MASK_2: c_int = 0o100;
// #define OCTAL_FLAG   0200
const OCTAL_FLAG: c_int = 0o200;
// #define OCTAL_BASE   010
const OCTAL_BASE: c_int = 0o10;

// typedef struct { int value; double coefficient; } DataPoint;
#[repr(C)]
#[derive(Copy, Clone)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

// The C code uses malloc()/free() from <stdlib.h>; use the very same allocator so
// that allocation-failure behaviour (including malloc(0) and huge/negative sizes)
// is reproduced bit-for-bit.
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// Call libc `malloc` in a way the optimizer cannot reason away.
///
/// Without this, at `-O2`+ LLVM recognises a *constant-size* `malloc` whose
/// pointer does not escape and applies "heap-to-stack": the allocation becomes
/// an `alloca`, which cannot fail, so the `if (p == NULL) return -1;` branch is
/// folded away as unreachable. The C reference (built by CMake with no
/// `CMAKE_BUILD_TYPE`, i.e. `-O0`) always performs the real call and always
/// honours the NULL check, so eliding it is an observable behavioural
/// divergence whenever `malloc` actually fails (memory pressure, `RLIMIT_AS`,
/// an interposed allocator...). `fallcalc`'s `malloc(5 * sizeof(int))` is
/// exactly such a constant-size allocation.
///
/// Hiding the size behind `black_box` keeps the genuine libc call, and hiding
/// the result keeps the NULL comparison, so the `-1` error path stays reachable
/// in every profile.
#[inline]
unsafe fn c_malloc(size: usize) -> *mut c_void {
    black_box(unsafe { malloc(black_box(size)) })
}

/// int safe_double_to_int(double d)
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { c_int::MAX } else { c_int::MIN };
    }

    if d >= c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d <= c_int::MIN as c_double {
        return c_int::MIN;
    }

    // Value is strictly inside the representable range here, so the C cast and
    // Rust's saturating `as` conversion agree (both truncate toward zero).
    d as c_int
}

/// int process_array_reverse(int *end, int count)
///
/// Walks *backwards* from `end`, summing `count` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr: *mut c_int = end;

    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = ptr.wrapping_offset(-1);
        i = i.wrapping_add(1);
    }

    sum
}

/// int switch_fallthrough_calculator(int value, int operation)
///
/// The C `switch` deliberately falls through: 0 -> 1 -> 2 (break),
/// 3 -> 4 (break), everything else -> 0.
#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result: c_int = value;

    match operation {
        0 => {
            // case 0: falls through into 1 and 2
            result = result.wrapping_mul(OCTAL_BASE);
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        1 => {
            // case 1: falls through into 2
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        2 => {
            result &= OCTAL_MASK_1;
        }
        3 => {
            // case 3: falls through into 4
            result = result.wrapping_mul(3);
            result = result.wrapping_add(OCTAL_MASK_2);
        }
        4 => {
            result = result.wrapping_add(OCTAL_MASK_2);
        }
        _ => {
            result = 0;
        }
    }

    result
}

/// int allocate_and_compute(int size, double multiplier)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_and_compute(size: c_int, multiplier: c_double) -> c_int {
    // C: malloc(size * sizeof(DataPoint)) -- `size` (int) is converted to size_t
    // before the multiplication, so negative sizes become enormous requests.
    let bytes = (size as usize).wrapping_mul(size_of::<DataPoint>());
    let points = unsafe { c_malloc(bytes) } as *mut DataPoint;

    if points.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < size {
        let p = points.wrapping_offset(i as isize);
        unsafe {
            (*p).value = i.wrapping_mul(OCTAL_BASE);
            (*p).coefficient = (i as c_double) * multiplier;
        }
        i = i.wrapping_add(1);
    }

    let mut sum: c_double = 0.0;
    let mut i: c_int = 0;
    while i < size {
        let p = points.wrapping_offset(i as isize);
        unsafe {
            sum += (*p).value as c_double * (*p).coefficient;
        }
        i = i.wrapping_add(1);
    }

    let result = safe_double_to_int(sum);

    unsafe { free(points as *mut c_void) };

    result
}

/// int foreach_sum(int *array, int count)
///
/// Uses the FOREACH macro, which expands to a nested-for construct that visits
/// `array[0] .. array[count - 1]` exactly once each.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;

    let size: c_int = count;
    let mut idx: c_int = 0;
    while idx < size {
        let element: c_int = unsafe { *array.wrapping_offset(idx as isize) };
        total = total.wrapping_add(element);
        idx = idx.wrapping_add(1);
    }

    total
}

/// int fallcalc(int param1, int param2, int param3, int param4)
#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int;

    let base_value: c_int = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let data_array =
        unsafe { c_malloc((array_size as usize).wrapping_mul(size_of::<c_int>())) } as *mut c_int;

    if data_array.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < array_size {
        unsafe {
            *data_array.wrapping_offset(i as isize) =
                i.wrapping_add(1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
        }
        i = i.wrapping_add(1);
    }

    let foreach_result = unsafe { foreach_sum(data_array, array_size) };

    let last_element = data_array.wrapping_offset((array_size - 1) as isize);
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };

    let switch_result = switch_fallthrough_calculator(param2, param3.wrapping_rem(5));

    let floating_calc: c_double =
        (param1 as c_double) * 3.7 + (param2 as c_double) * 2.3 - (param3 as c_double) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = unsafe { allocate_and_compute(param4.wrapping_rem(10).wrapping_add(1), 1.5) };

    result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    unsafe { free(data_array as *mut c_void) };

    result &= OCTAL_MASK_1;

    result
}
