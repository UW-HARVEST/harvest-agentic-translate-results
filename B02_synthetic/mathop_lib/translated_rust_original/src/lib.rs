use std::ffi::c_int;
use std::sync::Mutex;

extern "C" {
    fn time(t: *mut i64) -> i64;
    fn calloc(nmemb: usize, size: usize) -> *mut u8;
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[repr(C)]
struct ComputationResult {
    value: c_int,
    timestamp: i64,
    status: c_int,
}

type MathOperation = fn(c_int, c_int, c_int) -> c_int;

fn is_valid_operation(op_char: u8) -> bool {
    let valid = op_char != 0 && op_char >= b'1' && op_char <= b'5';
    valid
}

fn get_operation_priority(op: c_int) -> c_int {
    op * 10
}

fn add_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    a.wrapping_add(b)
}

fn multiply_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    a.wrapping_mul(b)
}

fn subtract_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    a.wrapping_sub(b)
}

fn divide_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    if b == 0 { return 0; }
    a.wrapping_div(b)
}

fn modulo_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    if b == 0 { return 0; }
    a.wrapping_rem(b)
}

fn select_operation(op: c_int) -> MathOperation {
    match op {
        1 => add_operation,
        2 => multiply_operation,
        3 => subtract_operation,
        4 => divide_operation,
        5 => modulo_operation,
        _ => add_operation,
    }
}

fn get_computation_timestamp() -> i64 {
    let mut current_time: i64 = 0;
    unsafe { time(&mut current_time); }
    current_time >> 29
}

fn allocate_results(count: usize) -> *mut ComputationResult {
    unsafe { calloc(count, std::mem::size_of::<ComputationResult>()) as *mut ComputationResult }
}

fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: c_int,
    history: &mut *mut ComputationResult,
    history_count: &mut c_int,
) -> c_int {
    let math_func = select_operation(op);
    let result = math_func(a, b, 0);

    if history.is_null() {
        *history = allocate_results(10);
        *history_count = 0;
    }

    if *history_count < 10 {
        unsafe {
            let entry = &mut *history.add(*history_count as usize);
            entry.value = result;
            entry.timestamp = get_computation_timestamp();
            entry.status = 0; // STATUS_SUCCESS
        }
        *history_count += 1;
    }

    result
}

struct History {
    ptr: *mut ComputationResult,
    count: c_int,
}

unsafe impl Send for History {}

static HISTORY: Mutex<Option<History>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut guard = HISTORY.lock().unwrap();
    let h = guard.get_or_insert(History {
        ptr: std::ptr::null_mut(),
        count: 0,
    });

    let validation_char = (param1 % 128) as u8;
    let _is_valid = is_valid_operation(validation_char);

    let selected_op = (param3 % 5) + 1;
    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = perform_computation_with_history(
        param1, param2, selected_op, &mut h.ptr, &mut h.count,
    );

    let second_op = ((param4 + 1) % 5) + 1;
    let mut final_result = perform_computation_with_history(
        intermediate_result, param4, second_op, &mut h.ptr, &mut h.count,
    );

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();
    let time_modifier = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    let history_count = h.count;
    drop(guard);

    unsafe {
        printf(b"Computation performed at timestamp: %ld\n\0".as_ptr(), computation_time);
        printf(b"Operation priority: %d\n\0".as_ptr(), operation_priority);
        printf(b"History entries: %d\n\0".as_ptr(), history_count);
        printf(b"Final result: %d\n\0".as_ptr(), final_result);
    }

    final_result
}
