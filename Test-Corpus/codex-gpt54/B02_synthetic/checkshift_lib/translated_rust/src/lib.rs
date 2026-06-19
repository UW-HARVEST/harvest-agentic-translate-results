use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem::size_of;
use std::ptr;

const OP_MULTIPLIER: c_int = 3;
const OP_ADDEND: c_int = 100;
const OP_SHIFT_AMOUNT: u32 = 2;
const MAGIC_NUMBER: c_uint = 0xDEADBEEF;
const MASK_LOWER: c_uint = 0x0000FFFF;

#[repr(C)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

type OperationFunc = extern "C" fn(c_int, c_int) -> c_int;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(OP_MULTIPLIER)
}

extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(OP_ADDEND)
}

extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_shl(OP_SHIFT_AMOUNT) | (b >> OP_SHIFT_AMOUNT)
}

fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    match opcode {
        0 => Some(multiply_with_static),
        1 => Some(add_with_static),
        2 => Some(xor_operation),
        3 => Some(shift_with_static),
        _ => None,
    }
}

fn execute_operation(
    func: Option<OperationFunc>,
    a: c_int,
    b: c_int,
    op_name: *const c_char,
) -> c_int {
    if let Some(func) = func {
        unsafe {
            printf(b"Variable a = %d\n\0".as_ptr().cast(), a);
            printf(b"Variable b = %d\n\0".as_ptr().cast(), b);
        }

        let result = func(a, b);

        unsafe {
            printf(b"Result of %s: %d\n\0".as_ptr().cast(), op_name, result);
        }

        result
    } else {
        unsafe {
            printf(
                b"Error: Operation function pointer is NULL for %s\n\0"
                    .as_ptr()
                    .cast(),
                op_name,
            );
        }
        0
    }
}

fn compute_checksum(values: *const c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    let mut buffer = [0u8; size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count } as usize;

        unsafe {
            ptr::copy_nonoverlapping(
                values.cast::<u8>(),
                buffer.as_mut_ptr(),
                size_of::<c_int>() * copy_count,
            );
        }

        for byte in &buffer[..size_of::<c_int>() * copy_count] {
            checksum = checksum.wrapping_shl(1) ^ c_uint::from(*byte);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        unsafe {
            printf(
                b"Error: state pointer is NULL in init_state\n\0"
                    .as_ptr()
                    .cast(),
            );
        }
        return;
    }

    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };

    unsafe {
        ptr::write(state, template);
        printf(
            b"State initialized with accumulator = %d\n\0"
                .as_ptr()
                .cast(),
            (*state).accumulator,
        );
    }
}

fn apply_operation(state: *mut ComputeState, value: c_int, func: Option<OperationFunc>) {
    if state.is_null() {
        unsafe {
            printf(
                b"Error: state pointer is NULL in apply_operation\n\0"
                    .as_ptr()
                    .cast(),
            );
        }
        return;
    }

    let Some(func) = func else {
        unsafe {
            printf(
                b"Error: operation function pointer is NULL in apply_operation\n\0"
                    .as_ptr()
                    .cast(),
            );
        }
        return;
    };

    unsafe {
        (*state).accumulator = func((*state).accumulator, value);
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
        printf(b"\n=== Starting foo function ===\n\0".as_ptr().cast());
        printf(
            b"Parameters: %d, %d, %d, %d\n\0".as_ptr().cast(),
            param1,
            param2,
            param3,
            param4,
        );
    }

    let state = unsafe { malloc(size_of::<ComputeState>()).cast::<ComputeState>() };

    if state.is_null() {
        unsafe {
            printf(
                b"Error: Failed to allocate memory for state\n\0"
                    .as_ptr()
                    .cast(),
            );
        }
        return -1;
    }

    init_state(state, param1);

    let params = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    unsafe {
        printf(b"\n--- Operation 1: Multiply ---\n\0".as_ptr().cast());
    }
    apply_operation(state, param2, mult_op);

    unsafe {
        printf(b"\n--- Operation 2: Add ---\n\0".as_ptr().cast());
    }
    apply_operation(state, param3, add_op);

    unsafe {
        printf(b"\n--- Operation 3: XOR ---\n\0".as_ptr().cast());
    }
    let xor_result = execute_operation(xor_op, unsafe { (*state).accumulator }, param4, b"XOR\0".as_ptr().cast());

    unsafe {
        printf(b"\n--- Operation 4: Shift ---\n\0".as_ptr().cast());
    }
    let shift_result =
        execute_operation(shift_op, xor_result, param2, b"SHIFT\0".as_ptr().cast());

    unsafe {
        (*state).checksum = compute_checksum(params.as_ptr(), 4);
        printf(
            b"\nComputed checksum: 0x%04X\n\0".as_ptr().cast(),
            (*state).checksum,
        );
    }

    let final_result = unsafe { ((*state).accumulator).wrapping_add(shift_result) }
        ^ unsafe { (*state).checksum as c_int };

    unsafe {
        printf(
            b"\nFinal accumulator: %d\n\0".as_ptr().cast(),
            (*state).accumulator,
        );
        printf(
            b"Operation count: %d\n\0".as_ptr().cast(),
            (*state).operation_count,
        );
        printf(b"Final result: %d\n\0".as_ptr().cast(), final_result);
        free(state.cast::<c_void>());
        printf(b"=== Ending foo function ===\n\n\0".as_ptr().cast());
    }

    final_result
}
