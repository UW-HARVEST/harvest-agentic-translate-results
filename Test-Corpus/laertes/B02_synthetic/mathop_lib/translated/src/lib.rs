extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut libc::c_void;
    fn time(__timer: *mut time_t) -> time_t;
}
pub type size_t = usize;
pub type __time_t = libc::c_long;
pub type time_t = __time_t;
pub type Operation = libc::c_uint;
pub const OP_MODULO: Operation = 5;
pub const OP_DIVIDE: Operation = 4;
pub const OP_SUBTRACT: Operation = 3;
pub const OP_MULTIPLY: Operation = 2;
pub const OP_ADD: Operation = 1;
pub type StatusCode = libc::c_int;
pub const STATUS_WARNING: StatusCode = 1;
pub const STATUS_ERROR: StatusCode = -1;
pub const STATUS_SUCCESS: StatusCode = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ComputationResult {
    pub value: libc::c_int,
    pub timestamp: time_t,
    pub status: StatusCode,
}
pub type MathOperation = Option<
    unsafe extern "C" fn(
        libc::c_int,
        libc::c_int,
        libc::c_int,
    ) -> libc::c_int,
>;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub extern "C" fn is_valid_operation(mut op_char: libc::c_char) -> bool {
    let mut valid: libc::c_char = (op_char as libc::c_int != 0
        && (op_char as libc::c_int >= '1' as i32
            && op_char as libc::c_int <= '5' as i32))
        as libc::c_int as libc::c_char;
    return valid != 0;
}
#[no_mangle]
pub extern "C" fn get_operation_priority(mut op: Operation) -> libc::c_int {
    let mut priority: libc::c_int =
        (op as libc::c_uint).wrapping_mul(10 as libc::c_uint) as libc::c_int;
    return priority;
}
#[no_mangle]
pub extern "C" fn add_operation(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused_param: libc::c_int,
) -> libc::c_int {
    return a + b;
}
#[no_mangle]
pub extern "C" fn multiply_operation(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused_param: libc::c_int,
) -> libc::c_int {
    return a * b;
}
#[no_mangle]
pub extern "C" fn subtract_operation(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused_param: libc::c_int,
) -> libc::c_int {
    return a - b;
}
#[no_mangle]
pub extern "C" fn divide_operation(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused_param: libc::c_int,
) -> libc::c_int {
    if b == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    return a / b;
}
#[no_mangle]
pub extern "C" fn modulo_operation(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut unused_param: libc::c_int,
) -> libc::c_int {
    if b == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    return a % b;
}
#[no_mangle]
pub extern "C" fn select_operation(mut op: Operation) -> MathOperation {
    match op as libc::c_uint {
        1 => {
            return Some(
                add_operation
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        2 => {
            return Some(
                multiply_operation
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        3 => {
            return Some(
                subtract_operation
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        4 => {
            return Some(
                divide_operation
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        5 => {
            return Some(
                modulo_operation
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
        _ => {
            return Some(
                add_operation
                    as unsafe extern "C" fn(
                        libc::c_int,
                        libc::c_int,
                        libc::c_int,
                    ) -> libc::c_int,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    time(&raw mut current_time);
    current_time = current_time >> 29 as libc::c_int;
    return current_time;
}
#[no_mangle]
pub unsafe extern "C" fn allocate_results(mut count: libc::c_int) -> *mut ComputationResult {
    let mut results: *mut ComputationResult = calloc(
        count as size_t,
        std::mem::size_of::<ComputationResult>() as size_t,
    ) as *mut ComputationResult;
    return results;
}
#[no_mangle]
pub unsafe extern "C" fn perform_computation_with_history(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut op: Operation,
    mut history: *mut *mut ComputationResult,
    mut history_count: *mut libc::c_int,
) -> libc::c_int {
    let mut math_func: MathOperation = select_operation(op);
    let mut result: libc::c_int =
        math_func.expect("non-null function pointer")(a, b, 0 as libc::c_int);
    if (*history).is_null() {
        *history = allocate_results(10 as libc::c_int);
        *history_count = 0 as libc::c_int;
    }
    if *history_count < 10 as libc::c_int {
        (*(*history).offset(*history_count as isize)).value = result;
        (*(*history).offset(*history_count as isize)).timestamp = get_computation_timestamp();
        (*(*history).offset(*history_count as isize)).status = STATUS_SUCCESS;
        *history_count += 1;
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn mathop(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    static mut computation_history: *mut ComputationResult =
        std::ptr::null::<ComputationResult>() as *mut ComputationResult;
    static mut history_count: libc::c_int = 0 as libc::c_int;
    let mut validation_char: libc::c_char =
        (param1 % 128 as libc::c_int) as libc::c_char;
    let mut is_valid: bool = is_valid_operation(validation_char);
    if !is_valid {
        validation_char = '1' as i32 as libc::c_char;
    }
    let mut selected_op: Operation =
        (param3 % 5 as libc::c_int + 1 as libc::c_int) as Operation;
    let mut operation_priority: libc::c_int = get_operation_priority(selected_op);
    let mut intermediate_result: libc::c_int = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        &raw mut computation_history,
        &raw mut history_count,
    );
    let mut second_op: Operation = ((param4 + 1 as libc::c_int) % 5 as libc::c_int
        + 1 as libc::c_int) as Operation;
    let mut final_result: libc::c_int = perform_computation_with_history(
        intermediate_result,
        param4,
        second_op,
        &raw mut computation_history,
        &raw mut history_count,
    );
    final_result += operation_priority;
    let mut computation_time: time_t = get_computation_timestamp();
    let mut time_modifier: libc::c_int = (computation_time as libc::c_long
        % 100 as libc::c_long)
        as libc::c_int;
    final_result += time_modifier;
    printf(
        b"Computation performed at timestamp: %ld\n\0" as *const u8 as *const libc::c_char,
        computation_time,
    );
    printf(
        b"Operation priority: %d\n\0" as *const u8 as *const libc::c_char,
        operation_priority,
    );
    printf(
        b"History entries: %d\n\0" as *const u8 as *const libc::c_char,
        history_count,
    );
    printf(
        b"Final result: %d\n\0" as *const u8 as *const libc::c_char,
        final_result,
    );
    return final_result;
}
