extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn time(__timer: *mut time_t) -> time_t;
}
pub type size_t = usize;
pub type __time_t = ::core::ffi::c_long;
pub type time_t = __time_t;
pub type Operation = ::core::ffi::c_uint;
pub const OP_MODULO: Operation = 5;
pub const OP_DIVIDE: Operation = 4;
pub const OP_SUBTRACT: Operation = 3;
pub const OP_MULTIPLY: Operation = 2;
pub const OP_ADD: Operation = 1;
pub type StatusCode = ::core::ffi::c_int;
pub const STATUS_WARNING: StatusCode = 1;
pub const STATUS_ERROR: StatusCode = -1;
pub const STATUS_SUCCESS: StatusCode = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ComputationResult {
    pub value: ::core::ffi::c_int,
    pub timestamp: time_t,
    pub status: StatusCode,
}
pub type MathOperation = Option<
    unsafe extern "C" fn(
        ::core::ffi::c_int,
        ::core::ffi::c_int,
        ::core::ffi::c_int,
    ) -> ::core::ffi::c_int,
>;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn is_valid_operation(mut op_char: ::core::ffi::c_char) -> bool {
    let mut valid: ::core::ffi::c_char = (op_char as ::core::ffi::c_int != 0
        && (op_char as ::core::ffi::c_int >= '1' as i32
            && op_char as ::core::ffi::c_int <= '5' as i32))
        as ::core::ffi::c_int as ::core::ffi::c_char;
    return valid != 0;
}
#[no_mangle]
pub unsafe extern "C" fn get_operation_priority(mut op: Operation) -> ::core::ffi::c_int {
    let mut priority: ::core::ffi::c_int =
        (op as ::core::ffi::c_uint).wrapping_mul(10 as ::core::ffi::c_uint) as ::core::ffi::c_int;
    return priority;
}
#[no_mangle]
pub unsafe extern "C" fn add_operation(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a + b;
}
#[no_mangle]
pub unsafe extern "C" fn multiply_operation(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a * b;
}
#[no_mangle]
pub unsafe extern "C" fn subtract_operation(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a - b;
}
#[no_mangle]
pub unsafe extern "C" fn divide_operation(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if b == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    return a / b;
}
#[no_mangle]
pub unsafe extern "C" fn modulo_operation(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if b == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    return a % b;
}
#[no_mangle]
pub unsafe extern "C" fn select_operation(mut op: Operation) -> MathOperation {
    match op as ::core::ffi::c_uint {
        1 => {
            return Some(
                add_operation
                    as unsafe extern "C" fn(
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                    ) -> ::core::ffi::c_int,
            );
        }
        2 => {
            return Some(
                multiply_operation
                    as unsafe extern "C" fn(
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                    ) -> ::core::ffi::c_int,
            );
        }
        3 => {
            return Some(
                subtract_operation
                    as unsafe extern "C" fn(
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                    ) -> ::core::ffi::c_int,
            );
        }
        4 => {
            return Some(
                divide_operation
                    as unsafe extern "C" fn(
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                    ) -> ::core::ffi::c_int,
            );
        }
        5 => {
            return Some(
                modulo_operation
                    as unsafe extern "C" fn(
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                    ) -> ::core::ffi::c_int,
            );
        }
        _ => {
            return Some(
                add_operation
                    as unsafe extern "C" fn(
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                        ::core::ffi::c_int,
                    ) -> ::core::ffi::c_int,
            );
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    time(&raw mut current_time);
    current_time = current_time >> 29 as ::core::ffi::c_int;
    return current_time;
}
#[no_mangle]
pub unsafe extern "C" fn allocate_results(mut count: ::core::ffi::c_int) -> *mut ComputationResult {
    let mut results: *mut ComputationResult = calloc(
        count as size_t,
        ::core::mem::size_of::<ComputationResult>() as size_t,
    ) as *mut ComputationResult;
    return results;
}
#[no_mangle]
pub unsafe extern "C" fn perform_computation_with_history(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut op: Operation,
    mut history: *mut *mut ComputationResult,
    mut history_count: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut math_func: MathOperation = select_operation(op);
    let mut result: ::core::ffi::c_int =
        math_func.expect("non-null function pointer")(a, b, 0 as ::core::ffi::c_int);
    if (*history).is_null() {
        *history = allocate_results(10 as ::core::ffi::c_int);
        *history_count = 0 as ::core::ffi::c_int;
    }
    if *history_count < 10 as ::core::ffi::c_int {
        (*(*history).offset(*history_count as isize)).value = result;
        (*(*history).offset(*history_count as isize)).timestamp = get_computation_timestamp();
        (*(*history).offset(*history_count as isize)).status = STATUS_SUCCESS;
        *history_count += 1;
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn mathop(
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
    mut param3: ::core::ffi::c_int,
    mut param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    static mut computation_history: *mut ComputationResult =
        ::core::ptr::null::<ComputationResult>() as *mut ComputationResult;
    static mut history_count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut validation_char: ::core::ffi::c_char =
        (param1 % 128 as ::core::ffi::c_int) as ::core::ffi::c_char;
    let mut is_valid: bool = is_valid_operation(validation_char);
    if !is_valid {
        validation_char = '1' as i32 as ::core::ffi::c_char;
    }
    let mut selected_op: Operation =
        (param3 % 5 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as Operation;
    let mut operation_priority: ::core::ffi::c_int = get_operation_priority(selected_op);
    let mut intermediate_result: ::core::ffi::c_int = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        &raw mut computation_history,
        &raw mut history_count,
    );
    let mut second_op: Operation = ((param4 + 1 as ::core::ffi::c_int) % 5 as ::core::ffi::c_int
        + 1 as ::core::ffi::c_int) as Operation;
    let mut final_result: ::core::ffi::c_int = perform_computation_with_history(
        intermediate_result,
        param4,
        second_op,
        &raw mut computation_history,
        &raw mut history_count,
    );
    final_result += operation_priority;
    let mut computation_time: time_t = get_computation_timestamp();
    let mut time_modifier: ::core::ffi::c_int = (computation_time as ::core::ffi::c_long
        % 100 as ::core::ffi::c_long)
        as ::core::ffi::c_int;
    final_result += time_modifier;
    printf(
        b"Computation performed at timestamp: %ld\n\0" as *const u8 as *const ::core::ffi::c_char,
        computation_time,
    );
    printf(
        b"Operation priority: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        operation_priority,
    );
    printf(
        b"History entries: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        history_count,
    );
    printf(
        b"Final result: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        final_result,
    );
    return final_result;
}
