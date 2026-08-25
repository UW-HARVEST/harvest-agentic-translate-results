use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem::size_of;
use std::ptr;

pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

#[repr(C)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(3)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(100)
}

#[unsafe(no_mangle)]
pub extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

#[unsafe(no_mangle)]
pub extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_shl(2) | (b >> 2)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation(opcode: c_int) -> OperationFunc {
    match opcode {
        0 => Some(multiply_with_static),
        1 => Some(add_with_static),
        2 => Some(xor_operation),
        3 => Some(shift_with_static),
        _ => None,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn execute_operation(
    func: OperationFunc,
    a: c_int,
    b: c_int,
    op_name: *const c_char,
) -> c_int {
    let Some(operation) = func else {
        unsafe {
            printf(
                c"Error: Operation function pointer is NULL for %s\n".as_ptr(),
                op_name,
            );
        }
        return 0;
    };

    unsafe {
        printf(c"Variable a = %d\n".as_ptr(), a);
        printf(c"Variable b = %d\n".as_ptr(), b);
    }

    let result = unsafe { operation(a, b) };
    unsafe {
        printf(c"Result of %s: %d\n".as_ptr(), op_name, result);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;

    if !values.is_null() && count > 0 {
        let copy_count = count.min(4) as usize;
        let byte_count = size_of::<c_int>() * copy_count;
        let bytes = unsafe { std::slice::from_raw_parts(values.cast::<u8>(), byte_count) };

        for &byte in bytes {
            checksum = checksum.wrapping_shl(1) ^ c_uint::from(byte);
        }

        checksum ^= 0xDEAD_BEEF;
    }

    checksum & 0x0000_FFFF
}

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
        checksum: 0,
    };

    unsafe {
        ptr::copy_nonoverlapping(&template, state, 1);
        printf(
            c"State initialized with accumulator = %d\n".as_ptr(),
            (*state).accumulator,
        );
    }
}

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

    let Some(operation) = func else {
        unsafe {
            printf(c"Error: operation function pointer is NULL in apply_operation\n".as_ptr());
        }
        return;
    };

    unsafe {
        (*state).accumulator = operation((*state).accumulator, value);
        (*state).operation_count = (*state).operation_count.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn checkshift(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        printf(c"\n=== Starting foo function ===\n".as_ptr());
        printf(
            c"Parameters: %d, %d, %d, %d\n".as_ptr(),
            param1,
            param2,
            param3,
            param4,
        );

        let state = malloc(size_of::<ComputeState>()).cast::<ComputeState>();
        if state.is_null() {
            printf(c"Error: Failed to allocate memory for state\n".as_ptr());
            return -1;
        }

        init_state(state, param1);

        let mut params = [param1, param2, param3, param4];
        let mult_op = get_operation(0);
        let add_op = get_operation(1);
        let xor_op = get_operation(2);
        let shift_op = get_operation(3);

        printf(c"\n--- Operation 1: Multiply ---\n".as_ptr());
        apply_operation(state, param2, mult_op);

        printf(c"\n--- Operation 2: Add ---\n".as_ptr());
        apply_operation(state, param3, add_op);

        printf(c"\n--- Operation 3: XOR ---\n".as_ptr());
        let xor_result = execute_operation(xor_op, (*state).accumulator, param4, c"XOR".as_ptr());

        printf(c"\n--- Operation 4: Shift ---\n".as_ptr());
        let shift_result = execute_operation(shift_op, xor_result, param2, c"SHIFT".as_ptr());

        (*state).checksum = compute_checksum(params.as_mut_ptr(), 4);
        printf(c"\nComputed checksum: 0x%04X\n".as_ptr(), (*state).checksum);

        let final_result =
            (*state).accumulator.wrapping_add(shift_result) ^ ((*state).checksum as c_int);

        printf(c"\nFinal accumulator: %d\n".as_ptr(), (*state).accumulator);
        printf(c"Operation count: %d\n".as_ptr(), (*state).operation_count);
        printf(c"Final result: %d\n".as_ptr(), final_result);

        free(state.cast::<c_void>());

        printf(c"=== Ending foo function ===\n\n".as_ptr());
        final_result
    }
}
