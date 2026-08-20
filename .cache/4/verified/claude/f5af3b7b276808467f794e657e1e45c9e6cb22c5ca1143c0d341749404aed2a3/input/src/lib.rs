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

// The C code prints with `printf(3)`. To guarantee byte-identical output (and
// identical stdio buffering / interleaving semantics with any C code in the
// same process), the real libc `printf` is used rather than Rust's own
// formatting machinery.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// C `INT_MAX` / `INT_MIN` from <limits.h>.
const INT_MAX: c_int = c_int::MAX;
const INT_MIN: c_int = c_int::MIN;

/// typedef struct { int id; double value; char label[20]; } DataBlock;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

/// int safe_double_to_int(double d)
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > INT_MAX as c_double {
        INT_MAX
    } else if d < INT_MIN as c_double {
        INT_MIN
    } else if d.is_nan() {
        0
    } else {
        // In-range truncating conversion, identical to C's `(int)d`.
        d as c_int
    }
}

/// int process_with_fallthrough(int code, int base_value)
///
/// The original C `switch` deliberately falls through from case 5 down to
/// case 1; that behaviour is reproduced verbatim below.
#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result: c_int = base_value;

    match code {
        5 => {
            // case 5: falls through 4, 3, 2, 1
            result = result.wrapping_add(50);
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        4 => {
            // case 4: falls through 3, 2, 1
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        3 => {
            // case 3: falls through 2, 1
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        2 => {
            // case 2: falls through 1
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

/// void copy_data_block(DataBlock *dest, const DataBlock *src)
///
/// Reproduces `memcpy(dest, src, sizeof(DataBlock))`, i.e. a raw byte copy of
/// the whole structure (padding bytes included).
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

/// int handle_pointer_operations(int value)
#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value: c_int = value.wrapping_mul(2);

    let ptr: &c_int = &local_value;

    let result: c_int = (*ptr).wrapping_add(100);

    result
}

/// int overunder(int a, int b, int c, int d)
#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    // `int total = 0;` in the C source (overwritten later, as in the original).
    let mut total: c_int = 0;

    let result_1: c_int = a;
    let result_2: c_int = b;
    let _result_3: c_int = c;
    let _result_4: c_int = d;

    unsafe {
        // PRINT_VAR(result_1); PRINT_VAR(result_2);
        printf(c"result_1 = %d\n".as_ptr(), result_1);
        printf(c"result_2 = %d\n".as_ptr(), result_2);
    }

    let temp1: c_double = (a as c_double) * 1.5;
    let temp2: c_double = (b as c_double) * 2.7;
    let temp3: c_double = (c as c_double) / 3.3;
    // `d * d + a * a` is computed in `int` arithmetic in C before the cast.
    let temp4: c_double =
        ((d.wrapping_mul(d)).wrapping_add(a.wrapping_mul(a)) as c_double).sqrt();

    let conv1: c_int = safe_double_to_int(temp1);
    let conv2: c_int = safe_double_to_int(temp2);
    let conv3: c_int = safe_double_to_int(temp3);
    let conv4: c_int = safe_double_to_int(temp4);

    unsafe {
        printf(
            c"Converted values: %d, %d, %d, %d\n".as_ptr(),
            conv1,
            conv2,
            conv3,
            conv4,
        );
    }

    // C's `%` truncates toward zero, so a negative `a` yields a negative
    // remainder (which lands in the `default` branch).
    let switch_result: c_int = process_with_fallthrough(a % 6, b);
    unsafe {
        printf(c"Switch fall-through result: %d\n".as_ptr(), switch_result);
    }

    let mut source_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    source_block.id = a;
    source_block.value = temp1;
    // strncpy(source_block.label, "Source", sizeof(label) - 1) copies "Source"
    // and zero-pads the remaining 13 bytes of the first 19; label[19] = '\0'.
    {
        let src = b"Source";
        for (i, byte) in src.iter().enumerate() {
            source_block.label[i] = *byte as c_char;
        }
        for i in src.len()..20 {
            source_block.label[i] = 0;
        }
    }

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    unsafe {
        copy_data_block(&mut dest_block, &source_block);
    }

    unsafe {
        printf(
            c"Copied block: id=%d, value=%.2f, label=%s\n".as_ptr(),
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result: c_int = handle_pointer_operations(c);
    unsafe {
        printf(c"Pointer operation result: %d\n".as_ptr(), ptr_result);
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
        printf(c"Overflow protected conversion: %d\n".as_ptr(), safe_conv);
    }

    let underflow_test: c_double = -1e15;
    let safe_conv2: c_int = safe_double_to_int(underflow_test);
    unsafe {
        printf(c"Underflow protected conversion: %d\n".as_ptr(), safe_conv2);
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
        printf(c"Array copied via memcpy: ".as_ptr());
        for i in 0..5 {
            printf(c"%d ".as_ptr(), array2[i]);
            total = total.wrapping_add(array2[i]);
        }
        printf(c"\n".as_ptr());
    }

    total
}
