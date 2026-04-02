use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

const MAGIC_NUMBER: u32 = 0xDEADBEEF;
const MASK_LOWER: u32 = 0x0000FFFF;

#[repr(C)]
struct ComputeState {
    accumulator: c_int,
    operation_count: c_int,
    checksum: u32,
}

type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

static mut STATIC_MULTIPLIER: c_int = 3;
static mut STATIC_ADDEND: c_int = 100;
static mut STATIC_SHIFT_AMOUNT: c_int = 2;

unsafe extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    unsafe { (a.wrapping_mul(b)).wrapping_mul(STATIC_MULTIPLIER) }
}

unsafe extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    unsafe { (a.wrapping_add(b)).wrapping_add(STATIC_ADDEND) }
}

unsafe extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD_i32
}

unsafe extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    unsafe {
        (a << STATIC_SHIFT_AMOUNT) | ((b as u32 >> STATIC_SHIFT_AMOUNT) as c_int)
    }
}

static mut OPS: [OperationFunc; 4] = [None, None, None, None];
static mut OPS_INITIALIZED: bool = false;

fn get_operation(opcode: c_int) -> OperationFunc {
    unsafe {
        if !OPS_INITIALIZED {
            OPS[0] = Some(multiply_with_static);
            OPS[1] = Some(add_with_static);
            OPS[2] = Some(xor_operation);
            OPS[3] = Some(shift_with_static);
            OPS_INITIALIZED = true;
        }

        if opcode >= 0 && opcode < 4 {
            OPS[opcode as usize]
        } else {
            None
        }
    }
}

unsafe fn execute_operation(
    func: OperationFunc,
    a: c_int,
    b: c_int,
    op_name: *const u8,
) -> c_int {
    unsafe {
        if func.is_none() {
            printf(
                b"Error: Operation function pointer is NULL for %s\n\0".as_ptr(),
                op_name,
            );
            return 0;
        }

        printf(b"Variable a = %d\n\0".as_ptr(), a);
        printf(b"Variable b = %d\n\0".as_ptr(), b);

        let result = func.unwrap()(a, b);
        printf(b"Result of %s: %d\n\0".as_ptr(), op_name, result);

        result
    }
}

fn compute_checksum(values: *const c_int, count: c_int) -> u32 {
    let mut checksum: u32 = 0;
    let mut buffer = [0u8; std::mem::size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count } as usize;
        let byte_count = std::mem::size_of::<c_int>() * copy_count;

        unsafe {
            std::ptr::copy_nonoverlapping(
                values as *const u8,
                buffer.as_mut_ptr(),
                byte_count,
            );
        }

        for i in 0..byte_count {
            checksum = (checksum << 1) ^ (buffer[i] as u32);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

unsafe fn init_state(state: *mut ComputeState, initial_value: c_int) {
    unsafe {
        if state.is_null() {
            printf(b"Error: state pointer is NULL in init_state\n\0".as_ptr());
            return;
        }

        let template = ComputeState {
            accumulator: initial_value,
            operation_count: 0,
            checksum: 0x0000,
        };

        std::ptr::copy_nonoverlapping(
            &template as *const ComputeState as *const u8,
            state as *mut u8,
            std::mem::size_of::<ComputeState>(),
        );

        printf(
            b"State initialized with accumulator = %d\n\0".as_ptr(),
            (*state).accumulator,
        );
    }
}

unsafe fn apply_operation(
    state: *mut ComputeState,
    value: c_int,
    func: OperationFunc,
) {
    unsafe {
        if state.is_null() {
            printf(b"Error: state pointer is NULL in apply_operation\n\0".as_ptr());
            return;
        }

        if func.is_none() {
            printf(
                b"Error: operation function pointer is NULL in apply_operation\n\0".as_ptr(),
            );
            return;
        }

        (*state).accumulator = func.unwrap()((*state).accumulator, value);
        (*state).operation_count += 1;
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
        printf(b"\n=== Starting foo function ===\n\0".as_ptr());
        printf(
            b"Parameters: %d, %d, %d, %d\n\0".as_ptr(),
            param1,
            param2,
            param3,
            param4,
        );

        let state = malloc(std::mem::size_of::<ComputeState>()) as *mut ComputeState;

        if state.is_null() {
            printf(b"Error: Failed to allocate memory for state\n\0".as_ptr());
            return -1;
        }

        init_state(state, param1);

        let params: [c_int; 4] = [param1, param2, param3, param4];

        let mult_op = get_operation(0);
        let add_op = get_operation(1);
        let xor_op = get_operation(2);
        let shift_op = get_operation(3);

        printf(b"\n--- Operation 1: Multiply ---\n\0".as_ptr());
        apply_operation(state, param2, mult_op);

        printf(b"\n--- Operation 2: Add ---\n\0".as_ptr());
        apply_operation(state, param3, add_op);

        printf(b"\n--- Operation 3: XOR ---\n\0".as_ptr());
        let xor_result = execute_operation(
            xor_op,
            (*state).accumulator,
            param4,
            b"XOR\0".as_ptr(),
        );

        printf(b"\n--- Operation 4: Shift ---\n\0".as_ptr());
        let shift_result = execute_operation(
            shift_op,
            xor_result,
            param2,
            b"SHIFT\0".as_ptr(),
        );

        (*state).checksum = compute_checksum(params.as_ptr(), 4);
        printf(
            b"\nComputed checksum: 0x%04X\n\0".as_ptr(),
            (*state).checksum,
        );

        let final_result =
            ((*state).accumulator.wrapping_add(shift_result)) ^ ((*state).checksum as c_int);

        printf(b"\nFinal accumulator: %d\n\0".as_ptr(), (*state).accumulator);
        printf(
            b"Operation count: %d\n\0".as_ptr(),
            (*state).operation_count,
        );
        printf(b"Final result: %d\n\0".as_ptr(), final_result);

        free(state as *mut u8);

        printf(b"=== Ending foo function ===\n\n\0".as_ptr());

        final_result
    }
}
