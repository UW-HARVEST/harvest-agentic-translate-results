extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const INIT_add: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn op_add(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a + b;
}
#[no_mangle]
pub unsafe extern "C" fn op_sub(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a - b;
}
#[no_mangle]
pub unsafe extern "C" fn op_mul(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return a * b;
}
unsafe extern "C" fn accum_add(mut n: ::core::ffi::c_int) -> ::core::ffi::c_int {
    let mut acc: ::core::ffi::c_int = INIT_add;
    match n {
        1 => {
            acc += 0 as ::core::ffi::c_int;
        }
        2 => {
            acc += 0 as ::core::ffi::c_int;
            acc += 1 as ::core::ffi::c_int;
        }
        3 => {
            acc += 0 as ::core::ffi::c_int;
            acc += 1 as ::core::ffi::c_int;
            acc += 2 as ::core::ffi::c_int;
        }
        4 => {
            acc += 0 as ::core::ffi::c_int;
            acc += 1 as ::core::ffi::c_int;
            acc += 2 as ::core::ffi::c_int;
            acc += 3 as ::core::ffi::c_int;
        }
        5 => {
            acc += 0 as ::core::ffi::c_int;
            acc += 1 as ::core::ffi::c_int;
            acc += 2 as ::core::ffi::c_int;
            acc += 3 as ::core::ffi::c_int;
            acc += 4 as ::core::ffi::c_int;
        }
        6 => {
            acc += 0 as ::core::ffi::c_int;
            acc += 1 as ::core::ffi::c_int;
            acc += 2 as ::core::ffi::c_int;
            acc += 3 as ::core::ffi::c_int;
            acc += 4 as ::core::ffi::c_int;
            acc += 5 as ::core::ffi::c_int;
        }
        0 | _ => {}
    }
    return acc;
}
#[no_mangle]
pub static mut G_OP: Option<
    unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ::core::ffi::c_int,
> = unsafe {
    Some(
        op_add
            as unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ::core::ffi::c_int,
    )
};
#[no_mangle]
pub static mut G_OP_NAME: *const ::core::ffi::c_char =
    b"add\0" as *const u8 as *const ::core::ffi::c_char;
#[no_mangle]
pub unsafe extern "C" fn helper_call(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut r: ::core::ffi::c_int = op_add(a, b);
    let mut acc: ::core::ffi::c_int = INIT_add;
    acc += 0 as ::core::ffi::c_int;
    acc += 1 as ::core::ffi::c_int;
    acc += 2 as ::core::ffi::c_int;
    acc += 3 as ::core::ffi::c_int;
    acc += 4 as ::core::ffi::c_int;
    printf(
        b"helper.call=%d helper.acc=%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        r,
        acc,
    );
    return r + acc;
}
#[no_mangle]
pub unsafe extern "C" fn helper_ptr(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut fp: Option<
        unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ::core::ffi::c_int,
    > = Some(
        op_add
            as unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ::core::ffi::c_int,
    );
    let mut r: ::core::ffi::c_int = fp.expect("non-null function pointer")(a, b);
    printf(
        b"helper.ptr=%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        r,
    );
    return r;
}
#[no_mangle]
pub unsafe extern "C" fn use_generated(mut n: ::core::ffi::c_int) -> ::core::ffi::c_int {
    let mut r: ::core::ffi::c_int = accum_add(n);
    printf(
        b"gen.acc=%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        r,
    );
    return r;
}
