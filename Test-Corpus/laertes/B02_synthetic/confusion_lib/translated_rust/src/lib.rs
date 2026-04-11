use ::c2rust_bitfields;
extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn snprintf(
        __s: *mut libc::c_char,
        __maxlen: size_t,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn memchr(
        __s: *const libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct PackedFlags {
    #[bitfield(name = "flag1", ty = "libc::c_uint", bits = "0..=0")]
    #[bitfield(name = "flag2", ty = "libc::c_uint", bits = "1..=1")]
    #[bitfield(name = "flag3", ty = "libc::c_uint", bits = "2..=2")]
    #[bitfield(name = "counter", ty = "libc::c_uint", bits = "3..=7")]
    #[bitfield(name = "mode", ty = "libc::c_uint", bits = "8..=10")]
    #[bitfield(name = "status", ty = "libc::c_uint", bits = "11..=15")]
    #[bitfield(name = "reserved", ty = "libc::c_uint", bits = "16..=31")]
    pub flag1_flag2_flag3_counter_mode_status_reserved: [u8; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union TypeConfusion {
    pub int_val: libc::c_int,
    pub float_val: libc::c_float,
    pub uint_val: libc::c_uint,
    pub bytes: [libc::c_char; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ProcessState {
    pub flags: PackedFlags,
    pub data: TypeConfusion,
    pub buffer: *mut libc::c_char,
    pub capacity: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn create_state(
    mut initial_val: libc::c_int,
    mut capacity: libc::c_int,
) -> *mut ProcessState {
    let mut state: *mut ProcessState =
        malloc(std::mem::size_of::<ProcessState>() as size_t) as *mut ProcessState;
    if state.is_null() {
        printf(
            b"Error: Failed to allocate memory for state\n\0" as *const u8
                as *const libc::c_char,
        );
        return std::ptr::null_mut::<ProcessState>();
    }
    (*state)
        .flags
        .set_flag1(1 as libc::c_uint as libc::c_uint);
    (*state)
        .flags
        .set_flag2(0 as libc::c_uint as libc::c_uint);
    (*state)
        .flags
        .set_flag3(1 as libc::c_uint as libc::c_uint);
    (*state)
        .flags
        .set_counter(0 as libc::c_uint as libc::c_uint);
    (*state)
        .flags
        .set_mode(3 as libc::c_uint as libc::c_uint);
    (*state)
        .flags
        .set_status(15 as libc::c_uint as libc::c_uint);
    (*state)
        .flags
        .set_reserved(0 as libc::c_uint as libc::c_uint);
    (*state).data.int_val = initial_val;
    (*state).capacity = capacity;
    (*state).buffer = malloc(capacity as size_t) as *mut libc::c_char;
    if (*state).buffer.is_null() {
        printf(b"Error: Failed to allocate buffer\n\0" as *const u8 as *const libc::c_char);
        free(state as *mut libc::c_void);
        return std::ptr::null_mut::<ProcessState>();
    }
    snprintf(
        (*state).buffer,
        capacity as size_t,
        b"State:%d:Mode:%d\0" as *const u8 as *const libc::c_char,
        initial_val,
        (*state).flags.mode() as libc::c_int,
    );
    return state;
}
#[no_mangle]
pub unsafe extern "C" fn destroy_state(mut state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            free((*state).buffer as *mut libc::c_void);
        }
        free(state as *mut libc::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn process_buffer(
    mut state: *mut ProcessState,
    mut target: libc::c_char,
) -> libc::c_int {
    if state.is_null() || (*state).buffer.is_null() {
        printf(
            b"Error: Null pointer in process_buffer\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    let mut count: libc::c_int = 0 as libc::c_int;
    let mut ptr: *mut libc::c_char = (*state).buffer;
    let mut remaining: size_t = strlen((*state).buffer);
    while remaining > 0 as size_t {
        let mut found: *mut libc::c_char = memchr(
            ptr as *const libc::c_void,
            target as libc::c_int,
            remaining,
        ) as *mut libc::c_char;
        if found.is_null() {
            break;
        }
        count += 1;
        printf(
            b"Operation: memchr_found with value %d\n\0" as *const u8 as *const libc::c_char,
            count,
        );
        remaining = (remaining as libc::c_ulong).wrapping_sub(
            (found.offset_from(ptr) as libc::c_long + 1 as libc::c_long)
                as libc::c_ulong,
        ) as size_t as size_t;
        ptr = found.offset(1 as libc::c_int as isize);
    }
    return count;
}
#[no_mangle]
pub unsafe extern "C" fn update_flags(mut state: *mut ProcessState, mut param: libc::c_int) {
    if state.is_null() {
        return;
    }
    (*state).flags.set_counter(
        ((*state).flags.counter() as libc::c_int + 1 as libc::c_int
            & 0x1f as libc::c_int) as libc::c_uint as libc::c_uint,
    );
    (*state)
        .flags
        .set_flag1((param & 1 as libc::c_int) as libc::c_uint as libc::c_uint);
    (*state).flags.set_flag2(
        ((param & 2 as libc::c_int) >> 1 as libc::c_int) as libc::c_uint
            as libc::c_uint,
    );
    (*state).flags.set_flag3(
        ((param & 4 as libc::c_int) >> 2 as libc::c_int) as libc::c_uint
            as libc::c_uint,
    );
    (*state).flags.set_mode(
        (param >> 3 as libc::c_int & 0x7 as libc::c_int) as libc::c_uint
            as libc::c_uint,
    );
    printf(
        b"Debug: state->flags.counter = %d\n\0" as *const u8 as *const libc::c_char,
        (*state).flags.counter() as libc::c_int,
    );
    printf(
        b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0" as *const u8
            as *const libc::c_char,
        (*state).flags.flag1() as libc::c_int,
        (*state).flags.flag2() as libc::c_int,
        (*state).flags.flag3() as libc::c_int,
        (*state).flags.mode() as libc::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn confuse_types(
    mut state: *mut ProcessState,
    mut operation: libc::c_int,
) -> libc::c_int {
    if state.is_null() {
        return 0 as libc::c_int;
    }
    let mut result: libc::c_int = 0 as libc::c_int;
    match operation {
        0 => {
            (*state).data.int_val = 1078530011 as libc::c_int;
            printf(
                b"Set as int: %d\n\0" as *const u8 as *const libc::c_char,
                (*state).data.int_val,
            );
        }
        1 => {
            printf(
                b"Read as float: %f\n\0" as *const u8 as *const libc::c_char,
                (*state).data.float_val as libc::c_double,
            );
            result = ((*state).data.float_val * 100 as libc::c_int as libc::c_float)
                as libc::c_int;
        }
        2 => {
            printf(
                b"Read as uint: %u\n\0" as *const u8 as *const libc::c_char,
                (*state).data.uint_val,
            );
            result = ((*state).data.uint_val & 0xff as libc::c_uint) as libc::c_int;
        }
        3 => {
            printf(
                b"Read as bytes: [%d, %d, %d, %d]\n\0" as *const u8 as *const libc::c_char,
                (*state).data.bytes[0 as libc::c_int as usize] as libc::c_int,
                (*state).data.bytes[1 as libc::c_int as usize] as libc::c_int,
                (*state).data.bytes[2 as libc::c_int as usize] as libc::c_int,
                (*state).data.bytes[3 as libc::c_int as usize] as libc::c_int,
            );
            result = (*state).data.bytes[0 as libc::c_int as usize] as libc::c_int
                + (*state).data.bytes[1 as libc::c_int as usize] as libc::c_int;
        }
        _ => {}
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn confusion(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    printf(
        b"Debug: param1 = %d\n\0" as *const u8 as *const libc::c_char,
        param1,
    );
    printf(
        b"Debug: param2 = %d\n\0" as *const u8 as *const libc::c_char,
        param2,
    );
    printf(
        b"Debug: param3 = %d\n\0" as *const u8 as *const libc::c_char,
        param3,
    );
    printf(
        b"Debug: param4 = %d\n\0" as *const u8 as *const libc::c_char,
        param4,
    );
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut state: *mut ProcessState = create_state(param1, 128 as libc::c_int);
    if state.is_null() {
        return -(1 as libc::c_int);
    }
    update_flags(state, param2);
    let mut search_char: libc::c_char =
        ('0' as i32 + param3 % 10 as libc::c_int) as libc::c_char;
    let mut found_count: libc::c_int = process_buffer(state, search_char);
    result += found_count * 10 as libc::c_int;
    let mut confusion_result: libc::c_int =
        confuse_types(state, param4 % 4 as libc::c_int);
    result += confusion_result;
    result += (*state).flags.counter() as libc::c_int * 5 as libc::c_int;
    result += (*state).flags.mode() as libc::c_int * 3 as libc::c_int;
    printf(
        b"Final result: %d\n\0" as *const u8 as *const libc::c_char,
        result,
    );
    destroy_state(state);
    return result;
}
