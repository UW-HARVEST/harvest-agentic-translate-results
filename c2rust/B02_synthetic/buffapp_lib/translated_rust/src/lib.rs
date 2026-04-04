extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct StringBuffer {
    pub data: *mut ::core::ffi::c_char,
    pub capacity: ::core::ffi::c_int,
    pub length: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn create_buffer(
    mut initial_capacity: ::core::ffi::c_int,
) -> *mut StringBuffer {
    let mut buffer: *mut StringBuffer =
        malloc(::core::mem::size_of::<StringBuffer>() as size_t) as *mut StringBuffer;
    if buffer.is_null() {
        return ::core::ptr::null_mut::<StringBuffer>();
    }
    (*buffer).data = malloc(initial_capacity as size_t) as *mut ::core::ffi::c_char;
    if (*buffer).data.is_null() {
        free(buffer as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<StringBuffer>();
    }
    (*buffer).capacity = initial_capacity;
    (*buffer).length = 0 as ::core::ffi::c_int;
    *(*buffer).data.offset(0 as ::core::ffi::c_int as isize) = '\0' as i32 as ::core::ffi::c_char;
    return buffer;
}
#[no_mangle]
pub unsafe extern "C" fn append_to_buffer(
    mut buffer: *mut StringBuffer,
    mut str: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut str_len: ::core::ffi::c_int = strlen(str) as ::core::ffi::c_int;
    let mut required_capacity: ::core::ffi::c_int =
        (*buffer).length + str_len + 1 as ::core::ffi::c_int;
    if required_capacity > (*buffer).capacity {
        let mut new_capacity: ::core::ffi::c_int = required_capacity * 2 as ::core::ffi::c_int;
        let mut new_data: *mut ::core::ffi::c_char = realloc(
            (*buffer).data as *mut ::core::ffi::c_void,
            new_capacity as size_t,
        ) as *mut ::core::ffi::c_char;
        if new_data.is_null() {
            return -(1 as ::core::ffi::c_int);
        }
        (*buffer).data = new_data;
        (*buffer).capacity = new_capacity;
    }
    strcpy((*buffer).data.offset((*buffer).length as isize), str);
    (*buffer).length += str_len;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn destroy_buffer(mut buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        if !(*buffer).data.is_null() {
            free((*buffer).data as *mut ::core::ffi::c_void);
        }
        free(buffer as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn get_operation_name(
    mut op_code: ::core::ffi::c_int,
) -> *const ::core::ffi::c_char {
    match op_code {
        0 => return b"add\0" as *const u8 as *const ::core::ffi::c_char,
        1 => return b"subtract\0" as *const u8 as *const ::core::ffi::c_char,
        2 => return b"multiply\0" as *const u8 as *const ::core::ffi::c_char,
        3 => return b"divide\0" as *const u8 as *const ::core::ffi::c_char,
        _ => return b"unknown\0" as *const u8 as *const ::core::ffi::c_char,
    };
}
#[no_mangle]
pub unsafe extern "C" fn perform_operation(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut operation: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if strcmp(
        operation,
        b"add\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return a + b;
    } else if strcmp(
        operation,
        b"subtract\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return a - b;
    } else if strcmp(
        operation,
        b"multiply\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return a * b;
    } else if strcmp(
        operation,
        b"divide\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        if b != 0 as ::core::ffi::c_int {
            return a / b;
        }
        return 0 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffapp(
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
    mut param3: ::core::ffi::c_int,
    mut param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut log_buffer: *mut StringBuffer = create_buffer(32 as ::core::ffi::c_int);
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut temp: [::core::ffi::c_char; 64] = [0; 64];
    (*log_buffer).length = 0 as ::core::ffi::c_int;
    sprintf(
        &raw mut temp as *mut ::core::ffi::c_char,
        b"Starting computation with %d parameters\n\0" as *const u8 as *const ::core::ffi::c_char,
        4 as ::core::ffi::c_int,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut ::core::ffi::c_char);
    let mut op1: *const ::core::ffi::c_char = get_operation_name(param1 % 4 as ::core::ffi::c_int);
    sprintf(
        &raw mut temp as *mut ::core::ffi::c_char,
        b"Operation 1: %s(%d, %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
        op1,
        param1,
        param2,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut ::core::ffi::c_char);
    let mut intermediate1: ::core::ffi::c_int = perform_operation(param1, param2, op1);
    result += intermediate1;
    let mut op2: *const ::core::ffi::c_char = get_operation_name(param3 % 4 as ::core::ffi::c_int);
    sprintf(
        &raw mut temp as *mut ::core::ffi::c_char,
        b"Operation 2: %s(%d, %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
        op2,
        param3,
        param4,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut ::core::ffi::c_char);
    let mut intermediate2: ::core::ffi::c_int = perform_operation(param3, param4, op2);
    result += intermediate2;
    let mut op3: *const ::core::ffi::c_char =
        b"multiply\0" as *const u8 as *const ::core::ffi::c_char;
    sprintf(
        &raw mut temp as *mut ::core::ffi::c_char,
        b"Operation 3: %s(%d, %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
        op3,
        intermediate1,
        intermediate2,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut ::core::ffi::c_char);
    let mut intermediate3: ::core::ffi::c_int =
        perform_operation(intermediate1, intermediate2, op3);
    if intermediate3 != 0 as ::core::ffi::c_int {
        result = result / intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }
    sprintf(
        &raw mut temp as *mut ::core::ffi::c_char,
        b"Final result: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        result,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut ::core::ffi::c_char);
    printf(
        b"Computation Log:\n%s\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*log_buffer).data,
    );
    destroy_buffer(log_buffer);
    return result;
}
