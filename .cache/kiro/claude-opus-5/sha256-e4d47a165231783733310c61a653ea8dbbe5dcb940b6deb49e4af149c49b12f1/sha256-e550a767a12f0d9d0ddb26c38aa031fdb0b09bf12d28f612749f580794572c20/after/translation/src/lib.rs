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

use core::ffi::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// We call the platform `printf` (rather than Rust's own `println!`) so that the
// emitted bytes, the stream used, and the buffering/interleaving behaviour are
// identical to the C library.  Likewise `malloc`/`free` are used so that the
// allocation-failure path of `checkshift` behaves exactly as in C.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
    unsafe fn malloc(size: usize) -> *mut c_void;
    unsafe fn free(ptr: *mut c_void);
}

/// Helper: build a NUL-terminated format string literal usable with `printf`.
macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// ---------------------------------------------------------------------------
// #define OP_ADD 0x01
// #define OP_MULTIPLY 0x02
// #define OP_XOR 0x03
// #define OP_SHIFT 0x04
// #define MAGIC_NUMBER 0xDEADBEEF
// #define MASK_LOWER 0x0000FFFF
// ---------------------------------------------------------------------------
#[allow(dead_code)]
const OP_ADD: c_int = 0x01;
#[allow(dead_code)]
const OP_MULTIPLY: c_int = 0x02;
#[allow(dead_code)]
const OP_XOR: c_int = 0x03;
#[allow(dead_code)]
const OP_SHIFT: c_int = 0x04;
const MAGIC_NUMBER: c_uint = 0xDEAD_BEEF;
const MASK_LOWER: c_uint = 0x0000_FFFF;

/// typedef struct { int accumulator; int operation_count; unsigned int checksum; } ComputeState;
#[repr(C)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

/// typedef int (*operation_func)(int, int);
pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

// ---------------------------------------------------------------------------
// static file-scope state
// ---------------------------------------------------------------------------
static STATIC_MULTIPLIER: c_int = 3;
static STATIC_ADDEND: c_int = 100;
static STATIC_SHIFT_AMOUNT: c_int = 2;

// ---------------------------------------------------------------------------
// int multiply_with_static(int a, int b) { return (a * b) * static_multiplier; }
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

// ---------------------------------------------------------------------------
// int add_with_static(int a, int b) { return (a + b) + static_addend; }
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

// ---------------------------------------------------------------------------
// int xor_operation(int a, int b) { return a ^ b ^ 0xABCD; }
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

// ---------------------------------------------------------------------------
// int shift_with_static(int a, int b) {
//     return (a << static_shift_amount) | (b >> static_shift_amount);
// }
//
// The C code performs a signed left shift (which gcc/clang implement as a plain
// two's-complement bit shift, discarding the high bits) and a signed right
// shift (arithmetic).  `wrapping_shl` / `wrapping_shr` reproduce that without
// panicking in debug builds.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    let amount = STATIC_SHIFT_AMOUNT as u32;
    a.wrapping_shl(amount) | b.wrapping_shr(amount)
}

// ---------------------------------------------------------------------------
// operation_func get_operation(int opcode)
//
// The C version lazily fills a function-static table on first use; the
// observable behaviour is simply "index 0..3 -> the four operations, anything
// else -> NULL".
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn get_operation(opcode: c_int) -> OperationFunc {
    let ops: [OperationFunc; 4] = [
        Some(multiply_with_static),
        Some(add_with_static),
        Some(xor_operation),
        Some(shift_with_static),
    ];

    if opcode >= 0 && opcode < 4 {
        return ops[opcode as usize];
    }

    None
}

// ---------------------------------------------------------------------------
// int execute_operation(operation_func func, int a, int b, const char* op_name)
// ---------------------------------------------------------------------------
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
                c_printf(
                    cstr!("Error: Operation function pointer is NULL for %s\n"),
                    op_name,
                );
            }
            return 0;
        }
        Some(f) => f,
    };

    // LOG_VALUE(a); LOG_VALUE(b);
    unsafe {
        c_printf(cstr!("Variable a = %d\n"), a);
        c_printf(cstr!("Variable b = %d\n"), b);
    }

    let result = unsafe { func(a, b) };
    unsafe {
        c_printf(cstr!("Result of %s: %d\n"), op_name, result);
    }

    result
}

// ---------------------------------------------------------------------------
// unsigned int compute_checksum(int* values, int count)
//
// `count` is clamped to 4, so only the bytes actually copied into `buffer` are
// ever read; the uninitialised tail of the C buffer is never observed.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    let mut buffer = [0u8; core::mem::size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count } as usize;
        let nbytes = core::mem::size_of::<c_int>() * copy_count;

        // memcpy(buffer, values, sizeof(int) * copy_count);
        unsafe {
            core::ptr::copy_nonoverlapping(values as *const u8, buffer.as_mut_ptr(), nbytes);
        }

        for i in 0..nbytes {
            checksum = (checksum << 1) ^ (buffer[i] as c_uint);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

// ---------------------------------------------------------------------------
// void init_state(ComputeState* state, int initial_value)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        unsafe {
            c_printf(cstr!("Error: state pointer is NULL in init_state\n"));
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
        core::ptr::write(state, template);
    }

    unsafe {
        c_printf(
            cstr!("State initialized with accumulator = %d\n"),
            (*state).accumulator,
        );
    }
}

// ---------------------------------------------------------------------------
// void apply_operation(ComputeState* state, int value, operation_func func)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    state: *mut ComputeState,
    value: c_int,
    func: OperationFunc,
) {
    if state.is_null() {
        unsafe {
            c_printf(cstr!("Error: state pointer is NULL in apply_operation\n"));
        }
        return;
    }

    let func = match func {
        None => {
            unsafe {
                c_printf(cstr!(
                    "Error: operation function pointer is NULL in apply_operation\n"
                ));
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

// ---------------------------------------------------------------------------
// int checkshift(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        c_printf(cstr!("\n=== Starting foo function ===\n"));
        c_printf(
            cstr!("Parameters: %d, %d, %d, %d\n"),
            param1,
            param2,
            param3,
            param4,
        );
    }

    let state = unsafe { malloc(core::mem::size_of::<ComputeState>()) } as *mut ComputeState;

    if state.is_null() {
        unsafe {
            c_printf(cstr!("Error: Failed to allocate memory for state\n"));
        }
        return -1;
    }

    unsafe {
        init_state(state, param1);
    }

    let mut params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    unsafe {
        c_printf(cstr!("\n--- Operation 1: Multiply ---\n"));
        apply_operation(state, param2, mult_op);

        c_printf(cstr!("\n--- Operation 2: Add ---\n"));
        apply_operation(state, param3, add_op);
    }

    let xor_result = unsafe {
        c_printf(cstr!("\n--- Operation 3: XOR ---\n"));
        execute_operation(xor_op, (*state).accumulator, param4, cstr!("XOR"))
    };

    let shift_result = unsafe {
        c_printf(cstr!("\n--- Operation 4: Shift ---\n"));
        execute_operation(shift_op, xor_result, param2, cstr!("SHIFT"))
    };

    unsafe {
        (*state).checksum = compute_checksum(params.as_mut_ptr(), 4);
        c_printf(cstr!("\nComputed checksum: 0x%04X\n"), (*state).checksum);
    }

    // (state->accumulator + shift_result) ^ state->checksum
    //
    // The int operand is converted to unsigned int for the XOR, and the
    // unsigned result is then converted back to int on assignment.
    let final_result = unsafe {
        ((((*state).accumulator.wrapping_add(shift_result)) as c_uint) ^ (*state).checksum) as c_int
    };

    unsafe {
        c_printf(cstr!("\nFinal accumulator: %d\n"), (*state).accumulator);
        c_printf(cstr!("Operation count: %d\n"), (*state).operation_count);
        c_printf(cstr!("Final result: %d\n"), final_result);
    }

    unsafe {
        free(state as *mut c_void);
    }

    unsafe {
        c_printf(cstr!("=== Ending foo function ===\n\n"));
    }

    final_result
}
