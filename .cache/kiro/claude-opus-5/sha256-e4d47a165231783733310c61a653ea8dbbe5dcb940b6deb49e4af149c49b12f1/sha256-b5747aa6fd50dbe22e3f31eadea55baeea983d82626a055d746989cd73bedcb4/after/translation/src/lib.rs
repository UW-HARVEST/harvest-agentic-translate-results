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

use std::ffi::{c_char, c_int, c_uint, c_void};

// Output goes through C's `printf` so that formatting, and stdout buffering
// interleaving with any C caller, is byte-for-byte identical to the original.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// #define MAGIC_NUMBER 0xDEADBEEF
const MAGIC_NUMBER: c_uint = 0xDEAD_BEEF;
// #define MASK_LOWER 0x0000FFFF
const MASK_LOWER: c_uint = 0x0000_FFFF;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

// static int static_multiplier = 3;
static STATIC_MULTIPLIER: c_int = 3;
// static int static_addend = 100;
static STATIC_ADDEND: c_int = 100;
// static int static_shift_amount = 2;
static STATIC_SHIFT_AMOUNT: c_int = 2;

/// int multiply_with_static(int a, int b) { return (a * b) * static_multiplier; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

/// int add_with_static(int a, int b) { return (a + b) + static_addend; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

/// int xor_operation(int a, int b) { return a ^ b ^ 0xABCD; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

/// int shift_with_static(int a, int b) {
///     return (a << static_shift_amount) | (b >> static_shift_amount);
/// }
///
/// Reproduces the C semantics as compiled on x86: the left shift wraps (the
/// original is UB for negative `a` / overflow), and the right shift on a signed
/// int is arithmetic (sign extending).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    let shift = STATIC_SHIFT_AMOUNT as u32;
    let left = ((a as u32) << shift) as c_int;
    let right = b >> shift;
    left | right
}

/// operation_func get_operation(int opcode)
///
/// The C version lazily fills a function-pointer table on first call; the
/// observable result is just the indexed lookup, with NULL outside [0, 4).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation(opcode: c_int) -> OperationFunc {
    let ops: [unsafe extern "C" fn(c_int, c_int) -> c_int; 4] = [
        multiply_with_static,
        add_with_static,
        xor_operation,
        shift_with_static,
    ];

    if opcode >= 0 && opcode < 4 {
        return Some(ops[opcode as usize]);
    }

    None
}

/// int execute_operation(operation_func func, int a, int b, const char* op_name)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execute_operation(
    func: OperationFunc,
    a: c_int,
    b: c_int,
    op_name: *const c_char,
) -> c_int {
    let func = match func {
        None => {
            unsafe {
                printf(
                    c"Error: Operation function pointer is NULL for %s\n".as_ptr(),
                    op_name,
                );
            }
            return 0;
        }
        Some(f) => f,
    };

    // LOG_VALUE(a); LOG_VALUE(b);  -- the macro stringifies the argument name.
    unsafe {
        printf(c"Variable a = %d\n".as_ptr(), a);
        printf(c"Variable b = %d\n".as_ptr(), b);
    }

    let result = unsafe { func(a, b) };
    unsafe {
        printf(c"Result of %s: %d\n".as_ptr(), op_name, result);
    }

    result
}

/// unsigned int compute_checksum(int* values, int count)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    // unsigned char buffer[sizeof(int) * 4];
    let mut buffer = [0u8; 4 * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count } as usize;
        let byte_len = size_of::<c_int>() * copy_count;

        // memcpy(buffer, values, sizeof(int) * copy_count);
        let src = unsafe { std::slice::from_raw_parts(values as *const u8, byte_len) };
        buffer[..byte_len].copy_from_slice(src);

        for i in 0..byte_len {
            checksum = (checksum << 1) ^ (buffer[i] as c_uint);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

/// void init_state(ComputeState* state, int initial_value)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        unsafe {
            printf(c"Error: state pointer is NULL in init_state\n".as_ptr());
        }
        return;
    }

    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };

    // memcpy(state, &template, sizeof(ComputeState));
    unsafe {
        std::ptr::write(state, template);
        printf(
            c"State initialized with accumulator = %d\n".as_ptr(),
            (*state).accumulator,
        );
    }
}

/// void apply_operation(ComputeState* state, int value, operation_func func)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    state: *mut ComputeState,
    value: c_int,
    func: OperationFunc,
) {
    if state.is_null() {
        unsafe {
            printf(c"Error: state pointer is NULL in apply_operation\n".as_ptr());
        }
        return;
    }

    let func = match func {
        None => {
            unsafe {
                printf(
                    c"Error: operation function pointer is NULL in apply_operation\n".as_ptr(),
                );
            }
            return;
        }
        Some(f) => f,
    };

    unsafe {
        (*state).accumulator = func((*state).accumulator, value);
        (*state).operation_count = (*state).operation_count.wrapping_add(1);
    }
}

/// int checkshift(int param1, int param2, int param3, int param4)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        printf(c"\n=== Starting foo function ===\n".as_ptr());
        printf(
            c"Parameters: %d, %d, %d, %d\n".as_ptr(),
            param1,
            param2,
            param3,
            param4,
        );
    }

    let state = unsafe { malloc(size_of::<ComputeState>()) } as *mut ComputeState;

    if state.is_null() {
        unsafe {
            printf(c"Error: Failed to allocate memory for state\n".as_ptr());
        }
        return -1;
    }

    unsafe {
        init_state(state, param1);
    }

    let mut params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = unsafe { get_operation(0) };
    let add_op = unsafe { get_operation(1) };
    let xor_op = unsafe { get_operation(2) };
    let shift_op = unsafe { get_operation(3) };

    unsafe {
        printf(c"\n--- Operation 1: Multiply ---\n".as_ptr());
        apply_operation(state, param2, mult_op);

        printf(c"\n--- Operation 2: Add ---\n".as_ptr());
        apply_operation(state, param3, add_op);

        printf(c"\n--- Operation 3: XOR ---\n".as_ptr());
    }
    let xor_result =
        unsafe { execute_operation(xor_op, (*state).accumulator, param4, c"XOR".as_ptr()) };

    unsafe {
        printf(c"\n--- Operation 4: Shift ---\n".as_ptr());
    }
    let shift_result =
        unsafe { execute_operation(shift_op, xor_result, param2, c"SHIFT".as_ptr()) };

    let final_result;
    unsafe {
        (*state).checksum = compute_checksum(params.as_mut_ptr(), 4);
        printf(c"\nComputed checksum: 0x%04X\n".as_ptr(), (*state).checksum);

        // (int)((unsigned)(accumulator + shift_result) ^ checksum)
        final_result =
            (((*state).accumulator.wrapping_add(shift_result) as c_uint) ^ (*state).checksum)
                as c_int;

        printf(c"\nFinal accumulator: %d\n".as_ptr(), (*state).accumulator);
        printf(c"Operation count: %d\n".as_ptr(), (*state).operation_count);
        printf(c"Final result: %d\n".as_ptr(), final_result);

        free(state as *mut c_void);

        printf(c"=== Ending foo function ===\n\n".as_ptr());
    }

    final_result
}
