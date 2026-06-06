// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust, preserving exact behavior and output.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;
use std::ffi::CStr;

const OP_ADD: c_int = 0x01;
const OP_MULTIPLY: c_int = 0x02;
const OP_XOR: c_int = 0x03;
const OP_SHIFT: c_int = 0x04;
const MAGIC_NUMBER: c_uint = 0xDEADBEEF;
const MASK_LOWER: c_uint = 0x0000FFFF;

// Suppress unused-const warnings for the constants that aren't directly referenced
// but are part of the original C source.
#[allow(dead_code)]
const _UNUSED_OP_ADD: c_int = OP_ADD;
#[allow(dead_code)]
const _UNUSED_OP_MULTIPLY: c_int = OP_MULTIPLY;
#[allow(dead_code)]
const _UNUSED_OP_XOR: c_int = OP_XOR;
#[allow(dead_code)]
const _UNUSED_OP_SHIFT: c_int = OP_SHIFT;

#[repr(C)]
#[derive(Copy, Clone)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

type OperationFunc = fn(c_int, c_int) -> c_int;

static mut STATIC_MULTIPLIER: c_int = 3;
static mut STATIC_ADDEND: c_int = 100;
static mut STATIC_SHIFT_AMOUNT: c_int = 2;

fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    unsafe { (a.wrapping_mul(b)).wrapping_mul(STATIC_MULTIPLIER) }
}

fn add_with_static(a: c_int, b: c_int) -> c_int {
    unsafe { (a.wrapping_add(b)).wrapping_add(STATIC_ADDEND) }
}

fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

fn shift_with_static(a: c_int, b: c_int) -> c_int {
    unsafe {
        // Match C semantics: signed left shift on int can overflow; use wrapping
        // For shift by a constant amount, we do a logical shift on the bits, then reinterpret.
        let left = ((a as u32).wrapping_shl(STATIC_SHIFT_AMOUNT as u32)) as i32;
        // Right-shift on signed int in C is implementation-defined; on x86/GCC/Clang,
        // it's an arithmetic shift. Use Rust's signed >> which is arithmetic.
        let right = b >> STATIC_SHIFT_AMOUNT;
        left | right
    }
}

fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    // ops[4] indexed 0..3
    let ops: [OperationFunc; 4] = [
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

fn execute_operation(
    func: Option<OperationFunc>,
    a: c_int,
    b: c_int,
    op_name: &str,
) -> c_int {
    let func = match func {
        Some(f) => f,
        None => {
            print!("Error: Operation function pointer is NULL for {}\n", op_name);
            return 0;
        }
    };

    // LOG_VALUE(a); LOG_VALUE(b);
    print!("Variable a = {}\n", a);
    print!("Variable b = {}\n", b);

    let result = func(a, b);
    print!("Result of {}: {}\n", op_name, result);

    result
}

fn compute_checksum(values: *const c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    let mut buffer: [u8; 4 * 4] = [0; 16]; // sizeof(int) * 4 = 16 on standard platforms

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count };
        let byte_count = (std::mem::size_of::<c_int>() as c_int * copy_count) as usize;
        unsafe {
            std::ptr::copy_nonoverlapping(
                values as *const u8,
                buffer.as_mut_ptr(),
                byte_count,
            );
        }

        for i in 0..byte_count {
            checksum = (checksum << 1) ^ (buffer[i] as c_uint);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        print!("Error: state pointer is NULL in init_state\n");
        return;
    }

    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };

    unsafe {
        std::ptr::copy_nonoverlapping(&template, state, 1);
        print!(
            "State initialized with accumulator = {}\n",
            (*state).accumulator
        );
    }
}

fn apply_operation(state: *mut ComputeState, value: c_int, func: Option<OperationFunc>) {
    if state.is_null() {
        print!("Error: state pointer is NULL in apply_operation\n");
        return;
    }

    let func = match func {
        Some(f) => f,
        None => {
            print!("Error: operation function pointer is NULL in apply_operation\n");
            return;
        }
    };

    unsafe {
        (*state).accumulator = func((*state).accumulator, value);
        (*state).operation_count += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    print!("\n=== Starting foo function ===\n");
    print!(
        "Parameters: {}, {}, {}, {}\n",
        param1, param2, param3, param4
    );

    // Allocate ComputeState on the heap to mirror malloc semantics.
    let state_box: Box<ComputeState> = Box::new(ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    });
    let state: *mut ComputeState = Box::into_raw(state_box);

    if state.is_null() {
        print!("Error: Failed to allocate memory for state\n");
        return -1;
    }

    init_state(state, param1);

    let params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    print!("\n--- Operation 1: Multiply ---\n");
    apply_operation(state, param2, mult_op);

    print!("\n--- Operation 2: Add ---\n");
    apply_operation(state, param3, add_op);

    print!("\n--- Operation 3: XOR ---\n");
    let acc = unsafe { (*state).accumulator };
    let xor_result = execute_operation(xor_op, acc, param4, "XOR");

    print!("\n--- Operation 4: Shift ---\n");
    let shift_result = execute_operation(shift_op, xor_result, param2, "SHIFT");

    let checksum = compute_checksum(params.as_ptr(), 4);
    unsafe {
        (*state).checksum = checksum;
    }
    print!("\nComputed checksum: 0x{:04X}\n", checksum);

    let final_result = unsafe {
        ((*state).accumulator.wrapping_add(shift_result)) ^ ((*state).checksum as c_int)
    };

    unsafe {
        print!("\nFinal accumulator: {}\n", (*state).accumulator);
        print!("Operation count: {}\n", (*state).operation_count);
    }
    print!("Final result: {}\n", final_result);

    // free(state)
    unsafe {
        let _ = Box::from_raw(state);
    }

    print!("=== Ending foo function ===\n\n");

    final_result
}

// Silence unused warnings for items that mirror the original C file but aren't
// strictly required by the FFI surface.
#[allow(dead_code)]
fn _silence_unused_warnings(s: *const c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CStr::from_ptr(s);
        }
    }
}
