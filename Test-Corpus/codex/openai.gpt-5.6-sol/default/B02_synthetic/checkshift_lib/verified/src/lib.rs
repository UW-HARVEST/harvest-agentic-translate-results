use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem::size_of;
use std::ptr;

type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

#[repr(C)]
pub struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: c_uint,
}

const MAGIC_NUMBER: c_uint = 0xDEAD_BEEF;
const MASK_LOWER: c_uint = 0x0000_FFFF;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[inline(never)]
unsafe fn allocate_state() -> *mut ComputeState {
    unsafe { malloc(size_of::<ComputeState>()) }.cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(3)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(100)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_shl(2) | (b >> 2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation(opcode: c_int) -> OperationFunc {
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
    let Some(func) = func else {
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

    let result = unsafe { func(a, b) };
    unsafe {
        printf(c"Result of %s: %d\n".as_ptr(), op_name, result);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum = 0_u32;
    let mut buffer = [0_u8; size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = count.min(4) as usize;
        let byte_count = size_of::<c_int>() * copy_count;
        unsafe {
            ptr::copy_nonoverlapping(values.cast::<u8>(), buffer.as_mut_ptr(), byte_count);
        }

        for byte in &buffer[..byte_count] {
            checksum = checksum.wrapping_shl(1) ^ c_uint::from(*byte);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
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
        ptr::write(state, template);
        printf(
            c"State initialized with accumulator = %d\n".as_ptr(),
            ptr::addr_of!((*state).accumulator).read(),
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

    let Some(func) = func else {
        unsafe {
            printf(c"Error: operation function pointer is NULL in apply_operation\n".as_ptr());
        }
        return;
    };

    let accumulator = unsafe { ptr::addr_of!((*state).accumulator).read() };
    let result = unsafe { func(accumulator, value) };
    unsafe {
        ptr::addr_of_mut!((*state).accumulator).write(result);
        let operation_count = ptr::addr_of!((*state).operation_count).read();
        ptr::addr_of_mut!((*state).operation_count).write(operation_count.wrapping_add(1));
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
        printf(c"\n=== Starting foo function ===\n".as_ptr());
        printf(
            c"Parameters: %d, %d, %d, %d\n".as_ptr(),
            param1,
            param2,
            param3,
            param4,
        );
    }

    let state = unsafe { allocate_state() };
    if state.is_null() {
        unsafe {
            printf(c"Error: Failed to allocate memory for state\n".as_ptr());
        }
        return -1;
    }

    unsafe {
        init_state(state, param1);
    }
    let mut params = [param1, param2, param3, param4];

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
    let xor_result = unsafe {
        execute_operation(
            xor_op,
            ptr::addr_of!((*state).accumulator).read(),
            param4,
            c"XOR".as_ptr(),
        )
    };

    unsafe {
        printf(c"\n--- Operation 4: Shift ---\n".as_ptr());
    }
    let shift_result =
        unsafe { execute_operation(shift_op, xor_result, param2, c"SHIFT".as_ptr()) };

    let checksum = unsafe { compute_checksum(params.as_mut_ptr(), 4) };
    unsafe {
        ptr::addr_of_mut!((*state).checksum).write(checksum);
        printf(c"\nComputed checksum: 0x%04X\n".as_ptr(), checksum);
    }

    let accumulator = unsafe { ptr::addr_of!((*state).accumulator).read() };
    let operation_count = unsafe { ptr::addr_of!((*state).operation_count).read() };
    let final_result = (accumulator.wrapping_add(shift_result) as c_uint ^ checksum) as c_int;

    unsafe {
        printf(c"\nFinal accumulator: %d\n".as_ptr(), accumulator);
        printf(c"Operation count: %d\n".as_ptr(), operation_count);
        printf(c"Final result: %d\n".as_ptr(), final_result);
        free(state.cast::<c_void>());
        printf(c"=== Ending foo function ===\n\n".as_ptr());
    }

    final_result
}
