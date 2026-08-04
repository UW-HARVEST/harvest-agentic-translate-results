// Translated from c_src/src/lib.c — preserves byte-identical output.

use libc::printf;
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;

const STATIC_MULTIPLIER: c_int = 3;
const STATIC_ADDEND: c_int = 100;
const STATIC_SHIFT_AMOUNT: c_int = 2;

const MAGIC_NUMBER: c_uint = 0xDEADBEEF;
const MASK_LOWER: c_uint = 0x0000FFFF;

#[repr(C)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

pub type OperationFunc = extern "C" fn(c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

#[unsafe(no_mangle)]
pub extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

#[unsafe(no_mangle)]
pub extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    let shift = STATIC_SHIFT_AMOUNT as u32;
    a.wrapping_shl(shift) | b.wrapping_shr(shift)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation(opcode: c_int) -> Option<OperationFunc> {
    if opcode >= 0 && opcode < 4 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execute_operation(
    func: Option<OperationFunc>,
    a: c_int,
    b: c_int,
    op_name: *const c_char,
) -> c_int {
    let func = match func {
        Some(f) => f,
        None => {
            unsafe {
                printf(
                    b"Error: Operation function pointer is NULL for %s\n\0".as_ptr()
                        as *const c_char,
                    op_name,
                );
            }
            return 0;
        }
    };

    unsafe {
        printf(
            b"Variable a = %d\n\0".as_ptr() as *const c_char,
            a,
        );
        printf(
            b"Variable b = %d\n\0".as_ptr() as *const c_char,
            b,
        );
    }

    let result = func(a, b);

    unsafe {
        printf(
            b"Result of %s: %d\n\0".as_ptr() as *const c_char,
            op_name,
            result,
        );
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    let mut buffer = [0u8; std::mem::size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count };
        let byte_count = std::mem::size_of::<c_int>() * copy_count as usize;

        unsafe {
            std::ptr::copy_nonoverlapping(
                values as *const u8,
                buffer.as_mut_ptr(),
                byte_count,
            );
        }

        for i in 0..byte_count {
            checksum = checksum.wrapping_shl(1) ^ (buffer[i] as c_uint);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        unsafe {
            printf(
                b"Error: state pointer is NULL in init_state\n\0".as_ptr() as *const c_char,
            );
        }
        return;
    }

    unsafe {
        (*state).accumulator = initial_value;
        (*state).operation_count = 0;
        (*state).checksum = 0;

        printf(
            b"State initialized with accumulator = %d\n\0".as_ptr() as *const c_char,
            (*state).accumulator,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    state: *mut ComputeState,
    value: c_int,
    func: Option<OperationFunc>,
) {
    if state.is_null() {
        unsafe {
            printf(
                b"Error: state pointer is NULL in apply_operation\n\0".as_ptr()
                    as *const c_char,
            );
        }
        return;
    }

    let func = match func {
        Some(f) => f,
        None => {
            unsafe {
                printf(
                    b"Error: operation function pointer is NULL in apply_operation\n\0"
                        .as_ptr() as *const c_char,
                );
            }
            return;
        }
    };

    unsafe {
        (*state).accumulator = func((*state).accumulator, value);
        (*state).operation_count = (*state).operation_count.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        printf(b"\n=== Starting foo function ===\n\0".as_ptr() as *const c_char);
        printf(
            b"Parameters: %d, %d, %d, %d\n\0".as_ptr() as *const c_char,
            param1,
            param2,
            param3,
            param4,
        );
    }

    let state = unsafe {
        libc::malloc(std::mem::size_of::<ComputeState>()) as *mut ComputeState
    };

    if state.is_null() {
        unsafe {
            printf(
                b"Error: Failed to allocate memory for state\n\0".as_ptr() as *const c_char,
            );
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
        printf(b"\n--- Operation 1: Multiply ---\n\0".as_ptr() as *const c_char);
        apply_operation(state, param2, mult_op);

        printf(b"\n--- Operation 2: Add ---\n\0".as_ptr() as *const c_char);
        apply_operation(state, param3, add_op);

        printf(b"\n--- Operation 3: XOR ---\n\0".as_ptr() as *const c_char);
    }

    let xor_result = unsafe {
        execute_operation(
            xor_op,
            (*state).accumulator,
            param4,
            b"XOR\0".as_ptr() as *const c_char,
        )
    };

    unsafe {
        printf(b"\n--- Operation 4: Shift ---\n\0".as_ptr() as *const c_char);
    }

    let shift_result = unsafe {
        execute_operation(
            shift_op,
            xor_result,
            param2,
            b"SHIFT\0".as_ptr() as *const c_char,
        )
    };

    unsafe {
        (*state).checksum = compute_checksum(params.as_mut_ptr(), 4);
        printf(
            b"\nComputed checksum: 0x%04X\n\0".as_ptr() as *const c_char,
            (*state).checksum,
        );
    }

    let final_result = unsafe {
        ((*state).accumulator.wrapping_add(shift_result)) ^ ((*state).checksum as c_int)
    };

    unsafe {
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

        libc::free(state as *mut libc::c_void);

        printf(b"=== Ending foo function ===\n\n\0".as_ptr() as *const c_char);
    }

    final_result
}
