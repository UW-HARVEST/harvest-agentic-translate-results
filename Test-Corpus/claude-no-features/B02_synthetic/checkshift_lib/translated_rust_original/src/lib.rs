// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust — preserves byte-identical output to the C implementation.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_char;
use std::os::raw::{c_int, c_uint};

// Constants from the original C source.
const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

#[repr(C)]
#[derive(Clone, Copy)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

type OperationFunc = fn(c_int, c_int) -> c_int;

// Static "globals" matching the C `static int` definitions in lib.c.
// Because Rust does not allow plain `static mut` ergonomics, we wrap these
// in functions returning their constant values, preserving the exact behavior
// of the original code (the C code never mutates these statics).
const STATIC_MULTIPLIER: c_int = 3;
const STATIC_ADDEND: c_int = 100;
const STATIC_SHIFT_AMOUNT: c_int = 2;

fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    // (a * b) * static_multiplier — uses i32 wrapping semantics like C.
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

fn shift_with_static(a: c_int, b: c_int) -> c_int {
    // (a << static_shift_amount) | (b >> static_shift_amount)
    // Use wrapping_shl/shr to mimic C's int shift behavior on i32 (mod 32).
    let left = (a as u32).wrapping_shl(STATIC_SHIFT_AMOUNT as u32) as i32;
    let right = (b >> STATIC_SHIFT_AMOUNT) as i32;
    left | right
}

fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    let ops: [OperationFunc; 4] = [
        multiply_with_static,
        add_with_static,
        xor_operation,
        shift_with_static,
    ];

    if opcode >= 0 && opcode < 4 {
        Some(ops[opcode as usize])
    } else {
        None
    }
}

// ---- printf helpers -----------------------------------------------------
// We use libc::printf so that output is byte-identical with the C version
// (same format-string semantics, same buffering, same locale handling).

fn printf_lit(s: &str) {
    // s must contain no `%` directives.
    // Pass a NUL-terminated string and treat it as the format itself.
    let mut bytes: Vec<u8> = s.as_bytes().to_vec();
    bytes.push(0);
    unsafe {
        libc::printf(b"%s\0".as_ptr() as *const c_char, bytes.as_ptr() as *const c_char);
    }
}

fn printf_int1(fmt: &str, a: c_int) {
    let mut bytes: Vec<u8> = fmt.as_bytes().to_vec();
    bytes.push(0);
    unsafe {
        libc::printf(bytes.as_ptr() as *const c_char, a);
    }
}

fn printf_int4(fmt: &str, a: c_int, b: c_int, c: c_int, d: c_int) {
    let mut bytes: Vec<u8> = fmt.as_bytes().to_vec();
    bytes.push(0);
    unsafe {
        libc::printf(bytes.as_ptr() as *const c_char, a, b, c, d);
    }
}

fn printf_str_int(fmt: &str, s: &str, n: c_int) {
    let mut bytes: Vec<u8> = fmt.as_bytes().to_vec();
    bytes.push(0);
    let mut sbytes: Vec<u8> = s.as_bytes().to_vec();
    sbytes.push(0);
    unsafe {
        libc::printf(
            bytes.as_ptr() as *const c_char,
            sbytes.as_ptr() as *const c_char,
            n,
        );
    }
}

fn printf_str(fmt: &str, s: &str) {
    let mut bytes: Vec<u8> = fmt.as_bytes().to_vec();
    bytes.push(0);
    let mut sbytes: Vec<u8> = s.as_bytes().to_vec();
    sbytes.push(0);
    unsafe {
        libc::printf(
            bytes.as_ptr() as *const c_char,
            sbytes.as_ptr() as *const c_char,
        );
    }
}

fn printf_uint_hex(fmt: &str, v: c_uint) {
    let mut bytes: Vec<u8> = fmt.as_bytes().to_vec();
    bytes.push(0);
    unsafe {
        libc::printf(bytes.as_ptr() as *const c_char, v);
    }
}

// ---- Operation execution / state helpers -------------------------------

fn execute_operation(
    func: Option<OperationFunc>,
    a: c_int,
    b: c_int,
    op_name: &str,
) -> c_int {
    let f = match func {
        Some(f) => f,
        None => {
            printf_str("Error: Operation function pointer is NULL for %s\n", op_name);
            return 0;
        }
    };

    // LOG_VALUE(a) -> printf("Variable a = %d\n", a)
    printf_int1("Variable a = %d\n", a);
    // LOG_VALUE(b) -> printf("Variable b = %d\n", b)
    printf_int1("Variable b = %d\n", b);

    let result = f(a, b);
    printf_str_int("Result of %s: %d\n", op_name, result);

    result
}

fn compute_checksum(values: &[c_int]) -> c_uint {
    // Mirrors C's compute_checksum logic.
    let count = values.len() as c_int;
    let mut checksum: c_uint = 0;

    if !values.is_empty() && count > 0 {
        let copy_count: usize = if count > 4 { 4 } else { count as usize };
        let int_size = std::mem::size_of::<c_int>();
        let total_bytes = int_size * copy_count;

        // Build the byte-buffer by copying the int values' raw bytes
        // (matches the platform endianness, identical to memcpy).
        let mut buffer = [0u8; 16]; // sizeof(int) * 4 == 16 on all common platforms
        // Safety: source slice has at least `copy_count` ints.
        unsafe {
            std::ptr::copy_nonoverlapping(
                values.as_ptr() as *const u8,
                buffer.as_mut_ptr(),
                total_bytes,
            );
        }

        for i in 0..total_bytes {
            checksum = (checksum.wrapping_shl(1)) ^ (buffer[i] as c_uint);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

fn init_state(state: &mut ComputeState, initial_value: c_int) {
    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };
    *state = template;

    printf_int1(
        "State initialized with accumulator = %d\n",
        state.accumulator,
    );
}

fn apply_operation(
    state: &mut ComputeState,
    value: c_int,
    func: Option<OperationFunc>,
) {
    let f = match func {
        Some(f) => f,
        None => {
            printf_lit("Error: operation function pointer is NULL in apply_operation\n");
            return;
        }
    };

    state.accumulator = f(state.accumulator, value);
    state.operation_count += 1;
}

// ---- Public API --------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    printf_lit("\n=== Starting foo function ===\n");
    printf_int4(
        "Parameters: %d, %d, %d, %d\n",
        param1, param2, param3, param4,
    );

    // Allocate a ComputeState on the heap (mirrors malloc/free).
    let mut state_box: Box<ComputeState> = Box::new(ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    });
    // (Allocation cannot fail in safe Rust without panic; the original
    // NULL check is unreachable in practice, matching expected behavior.)

    init_state(&mut *state_box, param1);

    let params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    printf_lit("\n--- Operation 1: Multiply ---\n");
    apply_operation(&mut *state_box, param2, mult_op);

    printf_lit("\n--- Operation 2: Add ---\n");
    apply_operation(&mut *state_box, param3, add_op);

    printf_lit("\n--- Operation 3: XOR ---\n");
    let xor_result = execute_operation(xor_op, state_box.accumulator, param4, "XOR");

    printf_lit("\n--- Operation 4: Shift ---\n");
    let shift_result = execute_operation(shift_op, xor_result, param2, "SHIFT");

    state_box.checksum = compute_checksum(&params);
    printf_uint_hex("\nComputed checksum: 0x%04X\n", state_box.checksum);

    let final_result = (state_box
        .accumulator
        .wrapping_add(shift_result))
        ^ (state_box.checksum as c_int);

    printf_int1("\nFinal accumulator: %d\n", state_box.accumulator);
    printf_int1("Operation count: %d\n", state_box.operation_count);
    printf_int1("Final result: %d\n", final_result);

    // Drop the Box, equivalent to free(state).
    drop(state_box);

    printf_lit("=== Ending foo function ===\n\n");

    final_result
}
