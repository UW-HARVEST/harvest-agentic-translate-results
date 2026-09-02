// Rust translation of c_src/src/lib.c
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

#![allow(non_snake_case)]

use std::ffi::{c_char, c_double, c_int};
use std::mem::MaybeUninit;

// The C code writes its output with `printf` from libc. To guarantee
// byte-identical output (and identical stream/buffering behaviour when the
// library is loaded next to other C code), we call libc's printf directly
// rather than going through Rust's own stdout.
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
    #[link_name = "sqrt"]
    safe fn c_sqrt(x: c_double) -> c_double;
}

/// C: `INT_MAX` from <limits.h>
const INT_MAX: c_int = c_int::MAX;
/// C: `INT_MIN` from <limits.h>
const INT_MIN: c_int = c_int::MIN;

/// C:
/// ```c
/// typedef struct {
///     int id;
///     double value;
///     char label[20];
/// } DataBlock;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

/// `int safe_double_to_int(double d)`
///
/// The order of the checks is preserved exactly as written in the C source.
/// Note that the `isnan` test comes *after* the two range comparisons; since
/// both comparisons are false for NaN, a NaN input still reaches the `isnan`
/// branch and yields 0.
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > INT_MAX as c_double {
        INT_MAX
    } else if d < INT_MIN as c_double {
        INT_MIN
    } else if d.is_nan() {
        0
    } else {
        // Guarded by the two comparisons above, so this truncating conversion
        // matches C's `(int)d` for every value that reaches here.
        d as c_int
    }
}

/// `int process_with_fallthrough(int code, int base_value)`
///
/// Reproduces the C switch statement's fall-through behaviour: cases 5, 4, 3
/// and 2 all fall into the following case, accumulating their increments.
#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result: c_int = base_value;

    match code {
        5 => {
            result = result.wrapping_add(50);
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        4 => {
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        3 => {
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        2 => {
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        1 => {
            result = result.wrapping_add(10);
        }
        0 => {
            result = 0;
        }
        _ => {
            result = -1;
        }
    }

    result
}

/// `void copy_data_block(DataBlock *dest, const DataBlock *src)`
///
/// A raw `memcpy` of `sizeof(DataBlock)` bytes, padding included.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    unsafe {
        std::ptr::copy_nonoverlapping(
            src as *const u8,
            dest as *mut u8,
            std::mem::size_of::<DataBlock>(),
        );
    }
}

/// `int handle_pointer_operations(int value)`
#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value: c_int = value.wrapping_mul(2);

    let ptr: &c_int = &local_value;

    let result: c_int = (*ptr).wrapping_add(100);

    result
}

/// `int overunder(int a, int b, int c, int d)`
#[unsafe(no_mangle)]
#[allow(unused_assignments)] // `int total = 0;` mirrors the C source
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int = 0;

    // MAKE_VAR_NAME(result, _N) token-pastes into result_1 .. result_4.
    let result_1: c_int = a;
    let result_2: c_int = b;
    let _result_3: c_int = c;
    let _result_4: c_int = d;

    // PRINT_VAR(name) stringizes the identifier: printf(#name " = %d\n", name)
    unsafe {
        c_printf(b"result_1 = %d\n\0".as_ptr() as *const c_char, result_1);
        c_printf(b"result_2 = %d\n\0".as_ptr() as *const c_char, result_2);
    }

    let temp1: c_double = a as c_double * 1.5;
    let temp2: c_double = b as c_double * 2.7;
    let temp3: c_double = c as c_double / 3.3;
    // `d * d + a * a` is computed in `int` in the C source (wrapping on
    // overflow as produced by the reference build) before widening to double.
    let temp4: c_double = c_sqrt(
        (d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a))) as c_double,
    );

    let conv1: c_int = safe_double_to_int(temp1);
    let conv2: c_int = safe_double_to_int(temp2);
    let conv3: c_int = safe_double_to_int(temp3);
    let conv4: c_int = safe_double_to_int(temp4);

    unsafe {
        c_printf(
            b"Converted values: %d, %d, %d, %d\n\0".as_ptr() as *const c_char,
            conv1,
            conv2,
            conv3,
            conv4,
        );
    }

    // C's `%` truncates toward zero, matching Rust's `%` for i32.
    let switch_result: c_int = process_with_fallthrough(a % 6, b);
    unsafe {
        c_printf(
            b"Switch fall-through result: %d\n\0".as_ptr() as *const c_char,
            switch_result,
        );
    }

    // `DataBlock source_block;` -- only some fields are assigned below, so the
    // struct starts out uninitialised (padding bytes included).
    let mut source_block_uninit: MaybeUninit<DataBlock> = MaybeUninit::uninit();
    let source_block: &mut DataBlock = unsafe {
        let p = source_block_uninit.as_mut_ptr();
        (*p).id = a;
        (*p).value = temp1;
        // strncpy(dst, "Source", sizeof(label) - 1) copies "Source" and then
        // zero-pads out to 19 bytes; label[19] is then explicitly cleared.
        let label = &mut (*p).label;
        let src = b"Source";
        let n = label.len() - 1; // 19
        let mut i = 0usize;
        while i < n && i < src.len() {
            label[i] = src[i] as c_char;
            i += 1;
        }
        while i < n {
            label[i] = 0;
            i += 1;
        }
        label[label.len() - 1] = 0;
        &mut *p
    };

    let mut dest_block_uninit: MaybeUninit<DataBlock> = MaybeUninit::uninit();
    unsafe {
        copy_data_block(
            dest_block_uninit.as_mut_ptr(),
            source_block as *const DataBlock,
        );
    }
    let dest_block: DataBlock = unsafe { dest_block_uninit.assume_init() };

    unsafe {
        c_printf(
            b"Copied block: id=%d, value=%.2f, label=%s\n\0".as_ptr() as *const c_char,
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result: c_int = handle_pointer_operations(c);
    unsafe {
        c_printf(
            b"Pointer operation result: %d\n\0".as_ptr() as *const c_char,
            ptr_result,
        );
    }

    total = conv1
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: c_double = 1e15;
    let safe_conv: c_int = safe_double_to_int(overflow_test);
    unsafe {
        c_printf(
            b"Overflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv,
        );
    }

    let underflow_test: c_double = -1e15;
    let safe_conv2: c_int = safe_double_to_int(underflow_test);
    unsafe {
        c_printf(
            b"Underflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv2,
        );
    }

    let array1: [c_int; 5] = [a, b, c, d, a.wrapping_add(b)];
    let mut array2: [c_int; 5] = [0; 5];

    unsafe {
        std::ptr::copy_nonoverlapping(
            array1.as_ptr() as *const u8,
            array2.as_mut_ptr() as *mut u8,
            std::mem::size_of::<[c_int; 5]>(),
        );
    }

    unsafe {
        c_printf(b"Array copied via memcpy: \0".as_ptr() as *const c_char);
    }
    for i in 0..5usize {
        unsafe {
            c_printf(b"%d \0".as_ptr() as *const c_char, array2[i]);
        }
        total = total.wrapping_add(array2[i]);
    }
    unsafe {
        c_printf(b"\n\0".as_ptr() as *const c_char);
    }

    total
}
