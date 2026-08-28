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
// The `printf!` helper macro already wraps the variadic call in `unsafe`, which
// makes some of the surrounding explicit `unsafe` blocks redundant.
#![allow(unused_unsafe)]

use core::ffi::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C library performs all of its output with `printf` on the C runtime's
// `stdout`.  We call straight into libc so that formatting *and* buffering
// behaviour (and therefore the exact byte stream produced) are identical.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
    unsafe fn malloc(size: usize) -> *mut c_void;
    unsafe fn free(ptr: *mut c_void);
    unsafe fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// Build a NUL-terminated C string literal usable as a `printf` format.
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

/// `printf(fmt, args...)`
macro_rules! printf {
    ($fmt:expr) => {
        unsafe { c_printf(cstr!($fmt)) }
    };
    ($fmt:expr, $($arg:expr),* $(,)?) => {
        unsafe { c_printf(cstr!($fmt), $($arg),*) }
    };
}

// #define STRINGIFY(x) #x
// #define LOG_VALUE(var) printf("Variable " STRINGIFY(var) " = %d\n", var)
macro_rules! log_value {
    ($name:expr, $var:expr) => {
        printf!(concat!("Variable ", $name, " = %d\n"), $var as c_int)
    };
}

// #define OP_ADD 0x01
#[allow(dead_code)]
const OP_ADD: c_int = 0x01;
// #define OP_MULTIPLY 0x02
#[allow(dead_code)]
const OP_MULTIPLY: c_int = 0x02;
// #define OP_XOR 0x03
#[allow(dead_code)]
const OP_XOR: c_int = 0x03;
// #define OP_SHIFT 0x04
#[allow(dead_code)]
const OP_SHIFT: c_int = 0x04;
// #define MAGIC_NUMBER 0xDEADBEEF
const MAGIC_NUMBER: c_uint = 0xDEAD_BEEF;
// #define MASK_LOWER 0x0000FFFF
const MASK_LOWER: c_uint = 0x0000_FFFF;

/// typedef struct { int accumulator; int operation_count; unsigned int checksum; } ComputeState;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

/// typedef int (*operation_func)(int, int);
pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

// static int static_multiplier = 3;
const STATIC_MULTIPLIER: c_int = 3;
// static int static_addend = 100;
const STATIC_ADDEND: c_int = 100;
// static int static_shift_amount = 2;
const STATIC_SHIFT_AMOUNT: c_int = 2;

// ---------------------------------------------------------------------------
// int multiply_with_static(int a, int b)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    // return (a * b) * static_multiplier;
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

// ---------------------------------------------------------------------------
// int add_with_static(int a, int b)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    // return (a + b) + static_addend;
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

// ---------------------------------------------------------------------------
// int xor_operation(int a, int b)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    // return a ^ b ^ 0xABCD;
    a ^ b ^ 0xABCD
}

// ---------------------------------------------------------------------------
// int shift_with_static(int a, int b)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    // return (a << static_shift_amount) | (b >> static_shift_amount);
    //
    // The left shift of a (possibly negative) signed value is what gcc emits:
    // a plain logical shift of the bit pattern.  The right shift of a signed
    // value is an arithmetic shift.
    let left = ((a as u32) << (STATIC_SHIFT_AMOUNT as u32)) as c_int;
    let right = a_shr(b, STATIC_SHIFT_AMOUNT);
    left | right
}

#[inline]
fn a_shr(value: c_int, amount: c_int) -> c_int {
    value >> (amount as u32)
}

// ---------------------------------------------------------------------------
// operation_func get_operation(int opcode)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation(opcode: c_int) -> OperationFunc {
    // static operation_func ops[4] = {NULL, NULL, NULL, NULL};
    //
    // The C code lazily fills the static table on first use; the observable
    // behaviour is simply the fixed mapping below.
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
            printf!(
                "Error: Operation function pointer is NULL for %s\n",
                op_name
            );
            return 0;
        }
        Some(f) => f,
    };

    log_value!("a", a);
    log_value!("b", b);

    let result = unsafe { func(a, b) };
    printf!("Result of %s: %d\n", op_name, result);

    result
}

// ---------------------------------------------------------------------------
// unsigned int compute_checksum(int* values, int count)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    // unsigned char buffer[sizeof(int) * 4];
    let mut buffer = [0u8; core::mem::size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count } as usize;
        let nbytes = core::mem::size_of::<c_int>() * copy_count;

        unsafe {
            memcpy(
                buffer.as_mut_ptr() as *mut c_void,
                values as *const c_void,
                nbytes,
            );
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
        printf!("Error: state pointer is NULL in init_state\n");
        return;
    }

    // ComputeState template = {initial_value, 0, 0x0000};
    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };

    unsafe {
        memcpy(
            state as *mut c_void,
            &template as *const ComputeState as *const c_void,
            core::mem::size_of::<ComputeState>(),
        );

        printf!(
            "State initialized with accumulator = %d\n",
            (*state).accumulator
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
        printf!("Error: state pointer is NULL in apply_operation\n");
        return;
    }

    let func = match func {
        None => {
            printf!("Error: operation function pointer is NULL in apply_operation\n");
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
pub unsafe extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    printf!("\n=== Starting foo function ===\n");
    printf!(
        "Parameters: %d, %d, %d, %d\n",
        param1,
        param2,
        param3,
        param4
    );

    let state = unsafe { malloc(core::mem::size_of::<ComputeState>()) } as *mut ComputeState;

    if state.is_null() {
        printf!("Error: Failed to allocate memory for state\n");
        return -1;
    }

    unsafe { init_state(state, param1) };

    let mut params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = unsafe { get_operation(0) };
    let add_op = unsafe { get_operation(1) };
    let xor_op = unsafe { get_operation(2) };
    let shift_op = unsafe { get_operation(3) };

    printf!("\n--- Operation 1: Multiply ---\n");
    unsafe { apply_operation(state, param2, mult_op) };

    printf!("\n--- Operation 2: Add ---\n");
    unsafe { apply_operation(state, param3, add_op) };

    printf!("\n--- Operation 3: XOR ---\n");
    let xor_result = unsafe {
        execute_operation(xor_op, (*state).accumulator, param4, cstr!("XOR"))
    };

    printf!("\n--- Operation 4: Shift ---\n");
    let shift_result = unsafe { execute_operation(shift_op, xor_result, param2, cstr!("SHIFT")) };

    unsafe {
        (*state).checksum = compute_checksum(params.as_mut_ptr(), 4);
        printf!("\nComputed checksum: 0x%04X\n", (*state).checksum);
    }

    // int final_result = (state->accumulator + shift_result) ^ state->checksum;
    //
    // `int ^ unsigned int` promotes both operands to `unsigned int`; the result
    // is then converted back to `int` (a plain bit-pattern reinterpretation on
    // every mainstream target).
    let final_result = unsafe {
        (((*state).accumulator.wrapping_add(shift_result) as c_uint) ^ (*state).checksum) as c_int
    };

    unsafe {
        printf!("\nFinal accumulator: %d\n", (*state).accumulator);
        printf!("Operation count: %d\n", (*state).operation_count);
    }
    printf!("Final result: %d\n", final_result);

    unsafe { free(state as *mut c_void) };
    // state = NULL;  (dead store in the C source)

    printf!("=== Ending foo function ===\n\n");

    final_result
}
