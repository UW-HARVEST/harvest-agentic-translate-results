use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    fn raise(signal: c_int) -> c_int;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn sprintf(buffer: *mut c_char, format: *const c_char, ...) -> c_int;
    fn strcmp(lhs: *const c_char, rhs: *const c_char) -> c_int;
    fn strcpy(destination: *mut c_char, source: *const c_char) -> *mut c_char;
    fn strlen(value: *const c_char) -> usize;
}

const ADD: &[u8] = b"add\0";
const SUBTRACT: &[u8] = b"subtract\0";
const MULTIPLY: &[u8] = b"multiply\0";
const DIVIDE: &[u8] = b"divide\0";
const UNKNOWN: &[u8] = b"unknown\0";

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn divide_c_int(dividend: c_int, divisor: c_int) -> c_int {
    let mut quotient = dividend;
    unsafe {
        std::arch::asm!(
            "cdq",
            "idiv {divisor:e}",
            divisor = in(reg) divisor,
            inout("eax") quotient,
            out("edx") _,
            options(nomem, nostack),
        );
    }
    quotient
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn divide_c_int(dividend: c_int, divisor: c_int) -> c_int {
    dividend.wrapping_div(divisor)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn null_dereference() -> ! {
    unsafe {
        std::arch::asm!(
            "mov eax, dword ptr [{address}]",
            "ud2",
            address = in(reg) 0usize,
            options(noreturn, nostack),
        );
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
unsafe fn null_dereference() -> ! {
    unsafe {
        raise(11);
        std::process::abort();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    unsafe {
        let buffer = malloc(size_of::<StringBuffer>()).cast::<StringBuffer>();
        if buffer.is_null() {
            return std::ptr::null_mut();
        }

        let data = malloc(initial_capacity as usize).cast::<c_char>();
        if data.is_null() {
            free(buffer.cast::<c_void>());
            return std::ptr::null_mut();
        }

        (*buffer).data = data;
        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        *data = 0;
        buffer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(
    buffer: *mut StringBuffer,
    string: *const c_char,
) -> c_int {
    unsafe {
        let string_length = strlen(string) as c_int;
        if buffer.is_null() {
            null_dereference();
        }
        let required_capacity = (*buffer).length.wrapping_add(string_length).wrapping_add(1);

        if required_capacity > (*buffer).capacity {
            let new_capacity = required_capacity.wrapping_mul(2);
            let new_data =
                realloc((*buffer).data.cast::<c_void>(), new_capacity as usize).cast::<c_char>();
            if new_data.is_null() {
                return -1;
            }

            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }

        strcpy((*buffer).data.offset((*buffer).length as isize), string);
        (*buffer).length = (*buffer).length.wrapping_add(string_length);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    unsafe {
        if !buffer.is_null() {
            if !(*buffer).data.is_null() {
                free((*buffer).data.cast::<c_void>());
            }
            free(buffer.cast::<c_void>());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(operation_code: c_int) -> *const c_char {
    match operation_code {
        0 => ADD.as_ptr().cast::<c_char>(),
        1 => SUBTRACT.as_ptr().cast::<c_char>(),
        2 => MULTIPLY.as_ptr().cast::<c_char>(),
        3 => DIVIDE.as_ptr().cast::<c_char>(),
        _ => UNKNOWN.as_ptr().cast::<c_char>(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(a: c_int, b: c_int, operation: *const c_char) -> c_int {
    unsafe {
        if strcmp(operation, ADD.as_ptr().cast::<c_char>()) == 0 {
            a.wrapping_add(b)
        } else if strcmp(operation, SUBTRACT.as_ptr().cast::<c_char>()) == 0 {
            a.wrapping_sub(b)
        } else if strcmp(operation, MULTIPLY.as_ptr().cast::<c_char>()) == 0 {
            a.wrapping_mul(b)
        } else if strcmp(operation, DIVIDE.as_ptr().cast::<c_char>()) == 0 {
            if b != 0 { divide_c_int(a, b) } else { 0 }
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn buffapp(
    parameter1: c_int,
    parameter2: c_int,
    parameter3: c_int,
    parameter4: c_int,
) -> c_int {
    const START_FORMAT: &[u8] = b"Starting computation with %d parameters\n\0";
    const OPERATION_FORMAT: &[u8] = b"Operation %d: %s(%d, %d)\n\0";
    const FINAL_FORMAT: &[u8] = b"Final result: %d\n\0";
    const LOG_FORMAT: &[u8] = b"Computation Log:\n%s\n\0";

    unsafe {
        let log_buffer = create_buffer(32);
        let mut result: c_int = 0;
        let mut temporary = [0 as c_char; 64];

        (*log_buffer).length = 0;

        sprintf(
            temporary.as_mut_ptr(),
            START_FORMAT.as_ptr().cast::<c_char>(),
            4 as c_int,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());

        let operation1 = get_operation_name(parameter1 % 4);
        sprintf(
            temporary.as_mut_ptr(),
            OPERATION_FORMAT.as_ptr().cast::<c_char>(),
            1 as c_int,
            operation1,
            parameter1,
            parameter2,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());

        let intermediate1 = perform_operation(parameter1, parameter2, operation1);
        result = result.wrapping_add(intermediate1);

        let operation2 = get_operation_name(parameter3 % 4);
        sprintf(
            temporary.as_mut_ptr(),
            OPERATION_FORMAT.as_ptr().cast::<c_char>(),
            2 as c_int,
            operation2,
            parameter3,
            parameter4,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());

        let intermediate2 = perform_operation(parameter3, parameter4, operation2);
        result = result.wrapping_add(intermediate2);

        let operation3 = MULTIPLY.as_ptr().cast::<c_char>();
        sprintf(
            temporary.as_mut_ptr(),
            OPERATION_FORMAT.as_ptr().cast::<c_char>(),
            3 as c_int,
            operation3,
            intermediate1,
            intermediate2,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());

        let intermediate3 = perform_operation(intermediate1, intermediate2, operation3);
        if intermediate3 != 0 {
            result = divide_c_int(result, intermediate3);
        } else {
            result = parameter1
                .wrapping_add(parameter2)
                .wrapping_add(parameter3)
                .wrapping_add(parameter4);
        }

        sprintf(
            temporary.as_mut_ptr(),
            FINAL_FORMAT.as_ptr().cast::<c_char>(),
            result,
        );
        append_to_buffer(log_buffer, temporary.as_ptr());

        printf(
            LOG_FORMAT.as_ptr().cast::<c_char>(),
            (*log_buffer).data.cast_const(),
        );
        destroy_buffer(log_buffer);
        result
    }
}
