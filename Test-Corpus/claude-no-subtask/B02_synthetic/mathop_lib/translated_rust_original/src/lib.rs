// Translated from c_src/src/lib.c
//
// The original C code is licensed by MIT Lincoln Laboratory (see c_src/src/lib.c).

use std::ffi::c_int;
use std::ptr;

// time_t is a signed long on glibc/musl Linux (and on macOS for 64-bit).
#[allow(non_camel_case_types)]
type time_t = i64;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types, dead_code)]
enum Operation {
    OP_ADD = 1,
    OP_MULTIPLY = 2,
    OP_SUBTRACT = 3,
    OP_DIVIDE = 4,
    OP_MODULO = 5,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types, dead_code)]
enum StatusCode {
    STATUS_SUCCESS = 0,
    STATUS_ERROR = -1,
    STATUS_WARNING = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ComputationResult {
    value: c_int,
    timestamp: time_t,
    status: StatusCode,
}

impl Default for ComputationResult {
    fn default() -> Self {
        ComputationResult {
            value: 0,
            timestamp: 0,
            status: StatusCode::STATUS_SUCCESS,
        }
    }
}

type MathOperation = fn(c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    // libc time(time_t* tloc)
    fn time(tloc: *mut time_t) -> time_t;
}

fn is_valid_operation(op_char: i8) -> bool {
    // C: char valid = op_char && (op_char >= '1' && op_char <= '5');
    // The result of the && expressions is an int (0 or 1) cast to char,
    // then implicitly converted back to int for the bool return.
    let valid: i8 = if op_char != 0 && (op_char >= b'1' as i8 && op_char <= b'5' as i8) {
        1
    } else {
        0
    };
    valid != 0
}

fn get_operation_priority(op_value: c_int) -> c_int {
    // In C the parameter is `Operation` (an enum with int storage). Casting an
    // arbitrary int to the enum keeps the int value, so we work with c_int
    // directly to faithfully reproduce out-of-range inputs.
    let priority = op_value * 10;
    priority
}

fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a + b
}

fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a * b
}

fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a - b
}

fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a / b
}

fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a % b
}

fn select_operation(op: Operation) -> MathOperation {
    match op {
        Operation::OP_ADD => add_operation,
        Operation::OP_MULTIPLY => multiply_operation,
        Operation::OP_SUBTRACT => subtract_operation,
        Operation::OP_DIVIDE => divide_operation,
        Operation::OP_MODULO => modulo_operation,
    }
}

fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    unsafe {
        time(&mut current_time as *mut time_t);
    }
    current_time = current_time >> 29;
    current_time
}

fn allocate_results(count: c_int) -> *mut ComputationResult {
    // C: calloc(count, sizeof(ComputationResult)) — zero-initialized buffer.
    if count <= 0 {
        return ptr::null_mut();
    }
    let len = count as usize;
    let layout =
        std::alloc::Layout::array::<ComputationResult>(len).expect("layout overflow");
    unsafe {
        let raw = std::alloc::alloc_zeroed(layout) as *mut ComputationResult;
        raw
    }
}

fn op_from_int(value: c_int) -> Operation {
    match value {
        1 => Operation::OP_ADD,
        2 => Operation::OP_MULTIPLY,
        3 => Operation::OP_SUBTRACT,
        4 => Operation::OP_DIVIDE,
        5 => Operation::OP_MODULO,
        // C casts arbitrary integers to the enum; default in select_operation
        // returns add_operation. We model that by returning OP_ADD here so the
        // priority computation still uses the underlying integer separately.
        _ => Operation::OP_ADD,
    }
}

fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: Operation,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    let math_func = select_operation(op);

    let result = math_func(a, b, 0);

    unsafe {
        if (*history).is_null() {
            *history = allocate_results(10);
            *history_count = 0;
        }

        if *history_count < 10 {
            let idx = *history_count as isize;
            let entry = (*history).offset(idx);
            (*entry).value = result;
            (*entry).timestamp = get_computation_timestamp();
            (*entry).status = StatusCode::STATUS_SUCCESS;
            *history_count += 1;
        }
    }

    result
}

// Static mutable state mirroring the C `static` locals in mathop().
static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    // Mirror the C static-local variables.
    let history_ptr: *mut *mut ComputationResult =
        &raw mut COMPUTATION_HISTORY;
    let count_ptr: *mut c_int = &raw mut HISTORY_COUNT;

    // C: char validation_char = (char)(param1 % 128);
    // `char` is typically signed on x86_64 Linux. `param1 % 128` is in
    // [-127, 127] which fits in an i8 either way.
    let mut validation_char: i8 = (param1.rem_euclid_signed_c(128)) as i8;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as i8;
    }
    // validation_char is otherwise unused after this point in the C code.
    let _ = validation_char;

    // C: Operation selected_op = (Operation)((param3 % 5) + 1);
    let selected_op_int: c_int = (param3 % 5) + 1;
    let selected_op = op_from_int(selected_op_int);

    let operation_priority = get_operation_priority(selected_op_int);

    let intermediate_result = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        history_ptr,
        count_ptr,
    );

    // C: Operation second_op = (Operation)(((param4 + 1) % 5) + 1);
    let second_op_int: c_int = ((param4 + 1) % 5) + 1;
    let second_op = op_from_int(second_op_int);
    let mut final_result = perform_computation_with_history(
        intermediate_result,
        param4,
        second_op,
        history_ptr,
        count_ptr,
    );

    final_result += operation_priority;

    let computation_time = get_computation_timestamp();

    let time_modifier: c_int = (computation_time % 100) as c_int;
    final_result += time_modifier;

    let history_count_now = unsafe { *count_ptr };

    // Match the exact printf format strings.
    print!(
        "Computation performed at timestamp: {}\n",
        computation_time as i64
    );
    print!("Operation priority: {}\n", operation_priority);
    print!("History entries: {}\n", history_count_now);
    print!("Final result: {}\n", final_result);

    final_result
}

// Helper trait so we can make the C semantics of `%` explicit.
// In C, `(-a) % 128` follows truncation-toward-zero, which is what Rust's `%`
// already does for the built-in integer types, so this is just a renamed
// passthrough that documents the intent.
trait CMod {
    fn rem_euclid_signed_c(self, rhs: Self) -> Self;
}

impl CMod for c_int {
    fn rem_euclid_signed_c(self, rhs: c_int) -> c_int {
        self % rhs
    }
}
