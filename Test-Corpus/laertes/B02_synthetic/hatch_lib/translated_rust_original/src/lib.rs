extern "C" {
    fn snprintf(
        __s: *mut libc::c_char,
        __maxlen: size_t,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn memmove(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn time(__timer: *mut time_t) -> time_t;
    fn difftime(__time1: time_t, __time0: time_t) -> libc::c_double;
}
pub type size_t = usize;
pub type __time_t = libc::c_long;
pub type time_t = __time_t;
pub type operation_func = Option<
    unsafe extern "C" fn(
        libc::c_int,
        libc::c_int,
        libc::c_int,
    ) -> libc::c_int,
>;
pub type modifier_func = Option<unsafe extern "C" fn(libc::c_int, libc::c_int) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataRecord {
    pub id: libc::c_int,
    pub value: libc::c_int,
    pub timestamp: time_t,
    pub name: [libc::c_char; 32],
}
static mut global_counter: libc::c_int = 0 as libc::c_int;
static mut global_accumulator: libc::c_int = 0 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn increment_counter(
    mut value: libc::c_int,
    mut unused_param: libc::c_int,
) {
    global_counter += value;
}
#[no_mangle]
pub unsafe extern "C" fn update_accumulator(
    mut value: libc::c_int,
    mut unused_param: libc::c_int,
) {
    global_accumulator = global_accumulator * 2 as libc::c_int + value;
}
#[no_mangle]
pub unsafe extern "C" fn apply_operation(
    mut op: operation_func,
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
) -> libc::c_int {
    return op.expect("non-null function pointer")(a, b, c);
}
#[no_mangle]
pub extern "C" fn add_three(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
) -> libc::c_int {
    return a + b + c;
}
#[no_mangle]
pub extern "C" fn multiply_add(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
) -> libc::c_int {
    return a * b + c;
}
#[no_mangle]
pub unsafe extern "C" fn complex_calc(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
) -> libc::c_int {
    return (a - b) * c + global_counter;
}
#[no_mangle]
pub unsafe extern "C" fn shift_array_data(
    mut arr: *mut libc::c_int,
    mut size: libc::c_int,
    mut shift_by: libc::c_int,
) {
    if shift_by > 0 as libc::c_int && shift_by < size {
        memmove(
            arr as *mut libc::c_void,
            arr.offset(shift_by as isize) as *const libc::c_void,
            ((size - shift_by) as size_t)
                .wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
        );
        memset(
            arr.offset((size - shift_by) as isize) as *mut libc::c_void,
            0 as libc::c_int,
            (shift_by as size_t)
                .wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn process_pointer_data(
    mut ptr: *mut libc::c_int,
    mut multiplier: libc::c_int,
) -> libc::c_int {
    let mut value: libc::c_int = *ptr;
    return value * multiplier + global_accumulator;
}
#[no_mangle]
pub unsafe extern "C" fn compute_with_dynamic_memory(
    mut base: libc::c_int,
    mut count: libc::c_int,
) -> libc::c_int {
    let mut temp_array: *mut libc::c_int = malloc(
        (count as size_t).wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
    ) as *mut libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < count {
        *temp_array.offset(i as isize) = base + i * 3 as libc::c_int;
        i += 1;
    }
    let mut sum: libc::c_int = 0 as libc::c_int;
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < count {
        sum += *temp_array.offset(i_0 as isize);
        i_0 += 1;
    }
    free(temp_array as *mut libc::c_void);
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn get_time_based_value(mut seed: libc::c_int) -> libc::c_int {
    let mut current_time: time_t = 0;
    let mut reference_time: time_t = 0;
    time(&raw mut current_time);
    reference_time = (current_time as libc::c_long
        - (seed * 3600 as libc::c_int) as libc::c_long)
        as time_t;
    let mut diff: libc::c_double = difftime(current_time, reference_time);
    return (diff / 100 as libc::c_int as libc::c_double) as libc::c_int
        + seed;
}
#[no_mangle]
pub unsafe extern "C" fn manipulate_records(
    mut records: *mut DataRecord,
    mut num_records: libc::c_int,
    mut shift: libc::c_int,
) -> libc::c_int {
    let mut total: libc::c_int = 0 as libc::c_int;
    if shift > 0 as libc::c_int && shift < num_records {
        memmove(
            records as *mut libc::c_void,
            records.offset(shift as isize) as *const libc::c_void,
            ((num_records - shift) as size_t)
                .wrapping_mul(std::mem::size_of::<DataRecord>() as size_t),
        );
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < num_records - shift {
        total += (*records.offset(i as isize)).value;
        i += 1;
    }
    return total;
}
#[no_mangle]
pub unsafe extern "C" fn hatch(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut mod_func: modifier_func = None;
    mod_func = Some(
        increment_counter as unsafe extern "C" fn(libc::c_int, libc::c_int) -> (),
    ) as modifier_func;
    mod_func.expect("non-null function pointer")(param1, 999 as libc::c_int);
    mod_func = Some(
        update_accumulator as unsafe extern "C" fn(libc::c_int, libc::c_int) -> (),
    ) as modifier_func;
    mod_func.expect("non-null function pointer")(param2, 888 as libc::c_int);
    let mut op_func: operation_func = None;
    op_func = Some(
        add_three
            as unsafe extern "C" fn(
                libc::c_int,
                libc::c_int,
                libc::c_int,
            ) -> libc::c_int,
    ) as operation_func;
    result += apply_operation(op_func, param1, param2, param3);
    op_func = Some(
        multiply_add
            as unsafe extern "C" fn(
                libc::c_int,
                libc::c_int,
                libc::c_int,
            ) -> libc::c_int,
    ) as operation_func;
    result += apply_operation(op_func, param2, param3, param4);
    op_func = Some(
        complex_calc
            as unsafe extern "C" fn(
                libc::c_int,
                libc::c_int,
                libc::c_int,
            ) -> libc::c_int,
    ) as operation_func;
    result += apply_operation(op_func, param1, param3, param4);
    let mut dynamic_data: *mut libc::c_int =
        malloc((10 as size_t).wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t))
            as *mut libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 10 as libc::c_int {
        *dynamic_data.offset(i as isize) = param1 + i;
        i += 1;
    }
    result += process_pointer_data(
        dynamic_data.offset(5 as libc::c_int as isize) as *mut libc::c_int,
        param2,
    );
    shift_array_data(
        dynamic_data,
        10 as libc::c_int,
        3 as libc::c_int,
    );
    result += *dynamic_data.offset(0 as libc::c_int as isize);
    free(dynamic_data as *mut libc::c_void);
    result += get_time_based_value(param3);
    let mut records: *mut DataRecord =
        malloc((5 as size_t).wrapping_mul(std::mem::size_of::<DataRecord>() as size_t))
            as *mut DataRecord;
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < 5 as libc::c_int {
        (*records.offset(i_0 as isize)).id = i_0;
        (*records.offset(i_0 as isize)).value = param4 + i_0 * 10 as libc::c_int;
        time(&raw mut (*records.offset(i_0 as isize)).timestamp);
        snprintf(
            &raw mut (*records.offset(i_0 as isize)).name as *mut libc::c_char,
            32 as size_t,
            b"Record_%d\0" as *const u8 as *const libc::c_char,
            i_0,
        );
        i_0 += 1;
    }
    result += manipulate_records(records, 5 as libc::c_int, 2 as libc::c_int);
    free(records as *mut libc::c_void);
    result += compute_with_dynamic_memory(param1, 8 as libc::c_int);
    result += global_counter + global_accumulator;
    return result;
}
