extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
}
pub type size_t = usize;
pub type operation_fn = Option<
    unsafe extern "C" fn(
        ::core::ffi::c_int,
        ::core::ffi::c_int,
        *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ProcessorState {
    pub results: *mut ::core::ffi::c_int,
    pub capacity: size_t,
    pub count: size_t,
    pub operation: operation_fn,
    pub status: ::core::ffi::c_char,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const UINT16_MAX: ::core::ffi::c_int = 65535 as ::core::ffi::c_int;
unsafe extern "C" fn is_valid_state(mut state: *mut ProcessorState) -> bool {
    if (*state).status != 0 {
        return (*state).count < (*state).capacity;
    }
    return false_0 != 0;
}
unsafe extern "C" fn check_char_flag(mut flag: ::core::ffi::c_char) -> bool {
    return flag != 0;
}
#[no_mangle]
pub unsafe extern "C" fn process_value(
    mut value: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
    mut unused_context: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    return value + 10 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn double_value(
    mut value: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
    mut unused_context: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    return value * 2 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn triple_value(
    mut value: ::core::ffi::c_int,
    mut unused_param: ::core::ffi::c_int,
    mut unused_context: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    return value * 3 as ::core::ffi::c_int;
}
unsafe extern "C" fn init_processor(
    mut capacity: size_t,
    mut op: operation_fn,
) -> *mut ProcessorState {
    let mut state: *mut ProcessorState =
        malloc(::core::mem::size_of::<ProcessorState>() as size_t) as *mut ProcessorState;
    if state.is_null() {
        return ::core::ptr::null_mut::<ProcessorState>();
    }
    (*state).results =
        malloc(capacity.wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t))
            as *mut ::core::ffi::c_int;
    if (*state).results.is_null() {
        free(state as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<ProcessorState>();
    }
    (*state).capacity = capacity;
    (*state).count = 0 as size_t;
    (*state).operation = op;
    (*state).status = 1 as ::core::ffi::c_char;
    return state;
}
unsafe extern "C" fn cleanup_processor(mut state: *mut ProcessorState) {
    if !state.is_null() {
        if !(*state).results.is_null() {
            free((*state).results as *mut ::core::ffi::c_void);
        }
        free(state as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn gotomach(
    mut iterations: ::core::ffi::c_int,
    mut seed: ::core::ffi::c_int,
    mut mode: ::core::ffi::c_int,
    mut threshold: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut current_value: ::core::ffi::c_int = 0;
    let mut current_block: u64;
    let mut state: *mut ProcessorState = ::core::ptr::null_mut::<ProcessorState>();
    let mut temp_buffer: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut selected_op: operation_fn = None;
    printf(b"[INFO] Starting gotomach function\n\0" as *const u8 as *const ::core::ffi::c_char);
    if iterations < 0 as ::core::ffi::c_int || iterations > UINT16_MAX {
        printf(b"[ERROR] Invalid iteration count\n\0" as *const u8 as *const ::core::ffi::c_char);
        result = -(1 as ::core::ffi::c_int);
    } else if seed < 0 as ::core::ffi::c_int || seed > UINT16_MAX {
        printf(b"[ERROR] Invalid seed value\n\0" as *const u8 as *const ::core::ffi::c_char);
        result = -(2 as ::core::ffi::c_int);
    } else {
        match mode {
            0 => {
                selected_op = Some(
                    process_value
                        as unsafe extern "C" fn(
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut ::core::ffi::c_void,
                        ) -> ::core::ffi::c_int,
                ) as operation_fn;
            }
            1 => {
                selected_op = Some(
                    double_value
                        as unsafe extern "C" fn(
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut ::core::ffi::c_void,
                        ) -> ::core::ffi::c_int,
                ) as operation_fn;
            }
            2 => {
                selected_op = Some(
                    triple_value
                        as unsafe extern "C" fn(
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut ::core::ffi::c_void,
                        ) -> ::core::ffi::c_int,
                ) as operation_fn;
            }
            _ => {
                printf(
                    b"[WARNING] Invalid mode, using default\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                selected_op = Some(
                    process_value
                        as unsafe extern "C" fn(
                            ::core::ffi::c_int,
                            ::core::ffi::c_int,
                            *mut ::core::ffi::c_void,
                        ) -> ::core::ffi::c_int,
                ) as operation_fn;
            }
        }
        state = init_processor(iterations as size_t, selected_op);
        if state.is_null() {
            printf(
                b"[ERROR] Failed to initialize processor\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            result = -(3 as ::core::ffi::c_int);
        } else {
            temp_buffer = malloc(
                (iterations as size_t)
                    .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
            ) as *mut ::core::ffi::c_int;
            if temp_buffer.is_null() {
                printf(
                    b"[ERROR] Failed to allocate temporary buffer\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                );
                result = -(4 as ::core::ffi::c_int);
            } else if !check_char_flag((*state).status) {
                printf(
                    b"[ERROR] Invalid state status\n\0" as *const u8 as *const ::core::ffi::c_char,
                );
                result = -(5 as ::core::ffi::c_int);
            } else {
                current_value = seed;
                let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                loop {
                    if !(i < iterations) {
                        current_block = 11385396242402735691;
                        break;
                    }
                    if !is_valid_state(state) {
                        printf(
                            b"[ERROR] State became invalid during processing\n\0" as *const u8
                                as *const ::core::ffi::c_char,
                        );
                        result = -(6 as ::core::ffi::c_int);
                        current_block = 11920991149088378298;
                        break;
                    } else {
                        *temp_buffer.offset(i as isize) =
                            (*state).operation.expect("non-null function pointer")(
                                current_value,
                                0 as ::core::ffi::c_int,
                                NULL,
                            );
                        if *temp_buffer.offset(i as isize) < threshold {
                            let fresh0 = (*state).count;
                            (*state).count = (*state).count.wrapping_add(1);
                            *(*state).results.offset(fresh0 as isize) =
                                *temp_buffer.offset(i as isize);
                        }
                        current_value =
                            *temp_buffer.offset(i as isize) % 1000 as ::core::ffi::c_int;
                        if (*state).count >= UINT16_MAX as size_t {
                            printf(
                                b"[WARNING] Reached maximum count\n\0" as *const u8
                                    as *const ::core::ffi::c_char,
                            );
                            current_block = 11385396242402735691;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                }
                match current_block {
                    11920991149088378298 => {}
                    _ => {
                        result = 0 as ::core::ffi::c_int;
                        let mut i_0: size_t = 0 as size_t;
                        while i_0 < (*state).count {
                            result += *(*state).results.offset(i_0 as isize);
                            i_0 = i_0.wrapping_add(1);
                        }
                        printf(
                            b"[INFO] Processing completed successfully\n\0" as *const u8
                                as *const ::core::ffi::c_char,
                        );
                    }
                }
            }
        }
    }
    if !temp_buffer.is_null() {
        free(temp_buffer as *mut ::core::ffi::c_void);
    }
    cleanup_processor(state);
    return result;
}
