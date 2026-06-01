// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior must match the C original byte-for-byte.

use std::ffi::{c_char, c_int, c_uint, c_void};

// External C library functions for byte-identical stdout output and memory ops.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

// #define MAGIC_NUMBER 0xDEADBEEF
const MAGIC_NUMBER: c_uint = 0xDEADBEEF;
// #define MASK_LOWER 0x0000FFFF
const MASK_LOWER: c_uint = 0x0000FFFF;

// typedef struct { int accumulator; int operation_count; unsigned int checksum; } ComputeState;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

// typedef int (*operation_func)(int, int);
type OperationFunc = unsafe extern "C" fn(c_int, c_int) -> c_int;

// static int static_multiplier = 3;
// static int static_addend = 100;
// static int static_shift_amount = 2;
// These are mutable in C but never written; we model them as plain constants.
const STATIC_MULTIPLIER: c_int = 3;
const STATIC_ADDEND: c_int = 100;
const STATIC_SHIFT_AMOUNT: c_int = 2;

// int multiply_with_static(int a, int b) { return (a * b) * static_multiplier; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

// int add_with_static(int a, int b) { return (a + b) + static_addend; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

// int xor_operation(int a, int b) { return a ^ b ^ 0xABCD; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

// int shift_with_static(int a, int b) { return (a << static_shift_amount) | (b >> static_shift_amount); }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    // C signed shift: << of negative int is UB in standard C, but in practice
    // it is defined as logical shift for two's-complement. >> of a negative int
    // is implementation-defined but usually arithmetic shift on x86/GCC/Clang.
    // We replicate by using wrapping shifts on the unsigned bit pattern for <<,
    // and arithmetic shift for >>, which matches typical x86_64 GCC behavior.
    let shifted_left = (a as u32).wrapping_shl(STATIC_SHIFT_AMOUNT as u32) as c_int;
    let shifted_right = b >> (STATIC_SHIFT_AMOUNT as u32);
    shifted_left | shifted_right
}

// operation_func get_operation(int opcode);
// In the C source, this uses a lazy-initialised static array. The contents are
// always the same once initialised, so we can simply dispatch on opcode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    if (0..4).contains(&opcode) {
        match opcode {
            0 => Some(multiply_with_static),
            1 => Some(add_with_static),
            2 => Some(xor_operation),
            3 => Some(shift_with_static),
            _ => None,
        }
    } else {
        None
    }
}

// int execute_operation(operation_func func, int a, int b, const char* op_name);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execute_operation(
    func: Option<OperationFunc>,
    a: c_int,
    b: c_int,
    op_name: *const c_char,
) -> c_int {
    if func.is_none() {
        // printf("Error: Operation function pointer is NULL for %s\n", op_name);
        printf(
            b"Error: Operation function pointer is NULL for %s\n\0".as_ptr() as *const c_char,
            op_name,
        );
        return 0;
    }

    // LOG_VALUE(a); => printf("Variable a = %d\n", a);
    printf(b"Variable a = %d\n\0".as_ptr() as *const c_char, a);
    // LOG_VALUE(b); => printf("Variable b = %d\n", b);
    printf(b"Variable b = %d\n\0".as_ptr() as *const c_char, b);

    let result = (func.unwrap())(a, b);
    // printf("Result of %s: %d\n", op_name, result);
    printf(
        b"Result of %s: %d\n\0".as_ptr() as *const c_char,
        op_name,
        result,
    );

    result
}

// unsigned int compute_checksum(int* values, int count);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    let mut buffer: [u8; std::mem::size_of::<c_int>() * 4] =
        [0; std::mem::size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count };
        let bytes_to_copy = std::mem::size_of::<c_int>() * (copy_count as usize);
        memcpy(
            buffer.as_mut_ptr() as *mut c_void,
            values as *const c_void,
            bytes_to_copy,
        );

        // for (int i = 0; i < sizeof(int) * copy_count; i++) ...
        for i in 0..bytes_to_copy {
            checksum = (checksum << 1) ^ (buffer[i] as c_uint);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

// void init_state(ComputeState* state, int initial_value);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        printf(b"Error: state pointer is NULL in init_state\n\0".as_ptr() as *const c_char);
        return;
    }

    // ComputeState template = {initial_value, 0, 0x0000};
    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0,
    };

    // memcpy(state, &template, sizeof(ComputeState));
    memcpy(
        state as *mut c_void,
        &template as *const ComputeState as *const c_void,
        std::mem::size_of::<ComputeState>(),
    );

    // printf("State initialized with accumulator = %d\n", state->accumulator);
    printf(
        b"State initialized with accumulator = %d\n\0".as_ptr() as *const c_char,
        (*state).accumulator,
    );
}

// void apply_operation(ComputeState* state, int value, operation_func func);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    state: *mut ComputeState,
    value: c_int,
    func: Option<OperationFunc>,
) {
    if state.is_null() {
        printf(b"Error: state pointer is NULL in apply_operation\n\0".as_ptr() as *const c_char);
        return;
    }

    if func.is_none() {
        printf(
            b"Error: operation function pointer is NULL in apply_operation\n\0".as_ptr()
                as *const c_char,
        );
        return;
    }

    (*state).accumulator = (func.unwrap())((*state).accumulator, value);
    (*state).operation_count += 1;
}

// int checkshift(int param1, int param2, int param3, int param4);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    printf(b"\n=== Starting foo function ===\n\0".as_ptr() as *const c_char);
    printf(
        b"Parameters: %d, %d, %d, %d\n\0".as_ptr() as *const c_char,
        param1,
        param2,
        param3,
        param4,
    );

    let state = malloc(std::mem::size_of::<ComputeState>()) as *mut ComputeState;

    if state.is_null() {
        printf(b"Error: Failed to allocate memory for state\n\0".as_ptr() as *const c_char);
        return -1;
    }

    init_state(state, param1);

    let mut params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    printf(b"\n--- Operation 1: Multiply ---\n\0".as_ptr() as *const c_char);
    apply_operation(state, param2, mult_op);

    printf(b"\n--- Operation 2: Add ---\n\0".as_ptr() as *const c_char);
    apply_operation(state, param3, add_op);

    printf(b"\n--- Operation 3: XOR ---\n\0".as_ptr() as *const c_char);
    let xor_result = execute_operation(
        xor_op,
        (*state).accumulator,
        param4,
        b"XOR\0".as_ptr() as *const c_char,
    );

    printf(b"\n--- Operation 4: Shift ---\n\0".as_ptr() as *const c_char);
    let shift_result = execute_operation(
        shift_op,
        xor_result,
        param2,
        b"SHIFT\0".as_ptr() as *const c_char,
    );

    (*state).checksum = compute_checksum(params.as_mut_ptr(), 4);
    printf(
        b"\nComputed checksum: 0x%04X\n\0".as_ptr() as *const c_char,
        (*state).checksum,
    );

    let final_result =
        ((*state).accumulator.wrapping_add(shift_result)) ^ ((*state).checksum as c_int);

    printf(
        b"\nFinal accumulator: %d\n\0".as_ptr() as *const c_char,
        (*state).accumulator,
    );
    printf(
        b"Operation count: %d\n\0".as_ptr() as *const c_char,
        (*state).operation_count,
    );
    printf(
        b"Final result: %d\n\0".as_ptr() as *const c_char,
        final_result,
    );

    free(state as *mut c_void);
    // state = NULL; (cannot reassign here without a mutable local; pointer goes out of scope)

    printf(b"=== Ending foo function ===\n\n\0".as_ptr() as *const c_char);

    final_result
}
