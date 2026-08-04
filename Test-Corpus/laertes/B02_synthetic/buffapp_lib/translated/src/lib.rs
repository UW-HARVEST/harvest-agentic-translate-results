extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn sprintf(
        __s: *mut libc::c_char,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn realloc(__ptr: *mut libc::c_void, __size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn strcpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct StringBuffer {
    pub data: *mut libc::c_char,
    pub capacity: libc::c_int,
    pub length: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn create_buffer(
    mut initial_capacity: libc::c_int,
) -> *mut StringBuffer {
    let mut buffer: *mut StringBuffer =
        malloc(std::mem::size_of::<StringBuffer>() as size_t) as *mut StringBuffer;
    if buffer.is_null() {
        return std::ptr::null_mut::<StringBuffer>();
    }
    (*buffer).data = malloc(initial_capacity as size_t) as *mut libc::c_char;
    if (*buffer).data.is_null() {
        free(buffer as *mut libc::c_void);
        return std::ptr::null_mut::<StringBuffer>();
    }
    (*buffer).capacity = initial_capacity;
    (*buffer).length = 0 as libc::c_int;
    *(*buffer).data.offset(0 as libc::c_int as isize) = '\0' as i32 as libc::c_char;
    return buffer;
}
#[no_mangle]
pub unsafe extern "C" fn append_to_buffer(
    mut buffer: *mut StringBuffer,
    mut str: *const libc::c_char,
) -> libc::c_int {
    let mut str_len: libc::c_int = strlen(str) as libc::c_int;
    let mut required_capacity: libc::c_int =
        (*buffer).length + str_len + 1 as libc::c_int;
    if required_capacity > (*buffer).capacity {
        let mut new_capacity: libc::c_int = required_capacity * 2 as libc::c_int;
        let mut new_data: *mut libc::c_char = realloc(
            (*buffer).data as *mut libc::c_void,
            new_capacity as size_t,
        ) as *mut libc::c_char;
        if new_data.is_null() {
            return -(1 as libc::c_int);
        }
        (*buffer).data = new_data;
        (*buffer).capacity = new_capacity;
    }
    strcpy((*buffer).data.offset((*buffer).length as isize), str);
    (*buffer).length += str_len;
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn destroy_buffer(mut buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        if !(*buffer).data.is_null() {
            free((*buffer).data as *mut libc::c_void);
        }
        free(buffer as *mut libc::c_void);
    }
}
#[no_mangle]
pub extern "C" fn get_operation_name(
    mut op_code: libc::c_int,
) -> *const libc::c_char {
    match op_code {
        0 => return b"add\0" as *const u8 as *const libc::c_char,
        1 => return b"subtract\0" as *const u8 as *const libc::c_char,
        2 => return b"multiply\0" as *const u8 as *const libc::c_char,
        3 => return b"divide\0" as *const u8 as *const libc::c_char,
        _ => return b"unknown\0" as *const u8 as *const libc::c_char,
    };
}
#[no_mangle]
pub unsafe extern "C" fn perform_operation(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut operation: *const libc::c_char,
) -> libc::c_int {
    if strcmp(
        operation,
        b"add\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        return a + b;
    } else if strcmp(
        operation,
        b"subtract\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        return a - b;
    } else if strcmp(
        operation,
        b"multiply\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        return a * b;
    } else if strcmp(
        operation,
        b"divide\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        if b != 0 as libc::c_int {
            return a / b;
        }
        return 0 as libc::c_int;
    }
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffapp(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut log_buffer: *mut StringBuffer = create_buffer(32 as libc::c_int);
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut temp: [libc::c_char; 64] = [0; 64];
    (*log_buffer).length = 0 as libc::c_int;
    sprintf(
        &raw mut temp as *mut libc::c_char,
        b"Starting computation with %d parameters\n\0" as *const u8 as *const libc::c_char,
        4 as libc::c_int,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut libc::c_char);
    let mut op1: *const libc::c_char = get_operation_name(param1 % 4 as libc::c_int);
    sprintf(
        &raw mut temp as *mut libc::c_char,
        b"Operation 1: %s(%d, %d)\n\0" as *const u8 as *const libc::c_char,
        op1,
        param1,
        param2,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut libc::c_char);
    let mut intermediate1: libc::c_int = perform_operation(param1, param2, op1);
    result += intermediate1;
    let mut op2: *const libc::c_char = get_operation_name(param3 % 4 as libc::c_int);
    sprintf(
        &raw mut temp as *mut libc::c_char,
        b"Operation 2: %s(%d, %d)\n\0" as *const u8 as *const libc::c_char,
        op2,
        param3,
        param4,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut libc::c_char);
    let mut intermediate2: libc::c_int = perform_operation(param3, param4, op2);
    result += intermediate2;
    let mut op3: *const libc::c_char =
        b"multiply\0" as *const u8 as *const libc::c_char;
    sprintf(
        &raw mut temp as *mut libc::c_char,
        b"Operation 3: %s(%d, %d)\n\0" as *const u8 as *const libc::c_char,
        op3,
        intermediate1,
        intermediate2,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut libc::c_char);
    let mut intermediate3: libc::c_int =
        perform_operation(intermediate1, intermediate2, op3);
    if intermediate3 != 0 as libc::c_int {
        result = result / intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }
    sprintf(
        &raw mut temp as *mut libc::c_char,
        b"Final result: %d\n\0" as *const u8 as *const libc::c_char,
        result,
    );
    append_to_buffer(log_buffer, &raw mut temp as *mut libc::c_char);
    printf(
        b"Computation Log:\n%s\n\0" as *const u8 as *const libc::c_char,
        (*log_buffer).data,
    );
    destroy_buffer(log_buffer);
    return result;
}
