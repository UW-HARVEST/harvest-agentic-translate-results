// Rust translation of c_src/src/lib.c
// Calls into libc's printf directly to guarantee byte-identical output.

use std::ffi::{c_char, c_int, c_uint};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

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

#[repr(C)]
#[derive(Clone, Copy)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

type OperationFunc = fn(c_int, c_int) -> c_int;

// The C code uses non-const file-scope statics that are never reassigned in
// this translation unit. Modeling them as immutable constants matches behavior.
const STATIC_MULTIPLIER: c_int = 3;
const STATIC_ADDEND: c_int = 100;
const STATIC_SHIFT_AMOUNT: c_int = 2;

fn multiply_with_static(a: c_int, b: c_int) -> c_int {
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
    let shift = STATIC_SHIFT_AMOUNT as u32;
    let left = (a as u32).wrapping_shl(shift) as c_int;
    let right = b >> STATIC_SHIFT_AMOUNT;
    left | right
}

fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    // Initialize "table" lazily as the C code does — but the result is
    // observationally identical to a simple match.
    if opcode >= 0 && opcode < 4 {
        match opcode {
            0 => Some(multiply_with_static as OperationFunc),
            1 => Some(add_with_static as OperationFunc),
            2 => Some(xor_operation as OperationFunc),
            3 => Some(shift_with_static as OperationFunc),
            _ => None,
        }
    } else {
        None
    }
}

fn execute_operation(
    func: Option<OperationFunc>,
    a: c_int,
    b: c_int,
    op_name: *const c_char,
) -> c_int {
    if func.is_none() {
        unsafe {
            printf(
                c"Error: Operation function pointer is NULL for %s\n".as_ptr(),
                op_name,
            );
        }
        return 0;
    }

    // LOG_VALUE(a); LOG_VALUE(b);
    unsafe {
        printf(c"Variable a = %d\n".as_ptr(), a);
        printf(c"Variable b = %d\n".as_ptr(), b);
    }

    let result = (func.unwrap())(a, b);
    unsafe {
        printf(c"Result of %s: %d\n".as_ptr(), op_name, result);
    }
    result
}

fn compute_checksum(values: *const c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    // sizeof(int) * 4 == 16 bytes
    let mut buffer = [0u8; 16];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count };
        let bytes_to_copy = (std::mem::size_of::<c_int>() * copy_count as usize) as usize;

        unsafe {
            std::ptr::copy_nonoverlapping(
                values as *const u8,
                buffer.as_mut_ptr(),
                bytes_to_copy,
            );
        }

        // for (int i = 0; i < sizeof(int) * copy_count; i++)
        for i in 0..bytes_to_copy {
            checksum = checksum.wrapping_shl(1) ^ (buffer[i] as c_uint);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

unsafe fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        unsafe {
            printf(c"Error: state pointer is NULL in init_state\n".as_ptr());
        }
        return;
    }

    // Equivalent to: ComputeState template = {initial_value, 0, 0};
    //                memcpy(state, &template, sizeof(ComputeState));
    unsafe {
        (*state).accumulator = initial_value;
        (*state).operation_count = 0;
        (*state).checksum = 0;
        printf(
            c"State initialized with accumulator = %d\n".as_ptr(),
            (*state).accumulator,
        );
    }
}

unsafe fn apply_operation(
    state: *mut ComputeState,
    value: c_int,
    func: Option<OperationFunc>,
) {
    if state.is_null() {
        unsafe {
            printf(c"Error: state pointer is NULL in apply_operation\n".as_ptr());
        }
        return;
    }

    let f = match func {
        Some(f) => f,
        None => {
            unsafe {
                printf(
                    c"Error: operation function pointer is NULL in apply_operation\n"
                        .as_ptr(),
                );
            }
            return;
        }
    };

    unsafe {
        (*state).accumulator = f((*state).accumulator, value);
        (*state).operation_count = (*state).operation_count.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn checkshift(
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

    // Equivalent to: ComputeState* state = malloc(sizeof(ComputeState));
    let mut state_box: Box<ComputeState> = Box::new(ComputeState {
        accumulator: 0,
        operation_count: 0,
        checksum: 0,
    });
    let state: *mut ComputeState = &mut *state_box;

    unsafe {
        init_state(state, param1);
    }

    let params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    unsafe {
        printf(c"\n--- Operation 1: Multiply ---\n".as_ptr());
        apply_operation(state, param2, mult_op);

        printf(c"\n--- Operation 2: Add ---\n".as_ptr());
        apply_operation(state, param3, add_op);

        printf(c"\n--- Operation 3: XOR ---\n".as_ptr());
    }
    let xor_result = unsafe {
        execute_operation(xor_op, (*state).accumulator, param4, c"XOR".as_ptr())
    };

    unsafe {
        printf(c"\n--- Operation 4: Shift ---\n".as_ptr());
    }
    let shift_result = execute_operation(shift_op, xor_result, param2, c"SHIFT".as_ptr());

    unsafe {
        (*state).checksum = compute_checksum(params.as_ptr(), 4);
        printf(
            c"\nComputed checksum: 0x%04X\n".as_ptr(),
            (*state).checksum,
        );
    }

    // C: int final_result = (state->accumulator + shift_result) ^ state->checksum;
    // The XOR mixes (int) and (unsigned int); per usual arithmetic conversions
    // both become unsigned int and the result is then assigned to int.
    let final_result = unsafe {
        let sum = (*state).accumulator.wrapping_add(shift_result);
        ((sum as c_uint) ^ (*state).checksum) as c_int
    };

    unsafe {
        printf(c"\nFinal accumulator: %d\n".as_ptr(), (*state).accumulator);
        printf(
            c"Operation count: %d\n".as_ptr(),
            (*state).operation_count,
        );
        printf(c"Final result: %d\n".as_ptr(), final_result);
    }

    // free(state); state = NULL;
    drop(state_box);

    unsafe {
        printf(c"=== Ending foo function ===\n\n".as_ptr());
    }

    final_result
}
