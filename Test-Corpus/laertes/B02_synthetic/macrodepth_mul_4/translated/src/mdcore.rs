extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub const INIT_add: libc::c_int = 0 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn op_add(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    return a + b;
}
#[no_mangle]
pub unsafe extern "C" fn op_sub(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    return a - b;
}
#[no_mangle]
pub unsafe extern "C" fn op_mul(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    return a * b;
}
unsafe extern "C" fn accum_add(mut n: libc::c_int) -> libc::c_int {
    let mut acc: libc::c_int = INIT_add;
    match n {
        1 => {
            acc += 0 as libc::c_int;
        }
        2 => {
            acc += 0 as libc::c_int;
            acc += 1 as libc::c_int;
        }
        3 => {
            acc += 0 as libc::c_int;
            acc += 1 as libc::c_int;
            acc += 2 as libc::c_int;
        }
        4 => {
            acc += 0 as libc::c_int;
            acc += 1 as libc::c_int;
            acc += 2 as libc::c_int;
            acc += 3 as libc::c_int;
        }
        5 => {
            acc += 0 as libc::c_int;
            acc += 1 as libc::c_int;
            acc += 2 as libc::c_int;
            acc += 3 as libc::c_int;
            acc += 4 as libc::c_int;
        }
        6 => {
            acc += 0 as libc::c_int;
            acc += 1 as libc::c_int;
            acc += 2 as libc::c_int;
            acc += 3 as libc::c_int;
            acc += 4 as libc::c_int;
            acc += 5 as libc::c_int;
        }
        0 | _ => {}
    }
    return acc;
}
#[no_mangle]
pub static mut G_OP: Option<
    unsafe extern "C" fn(libc::c_int, libc::c_int) -> libc::c_int,
> = unsafe {
    Some(
        op_add
            as unsafe extern "C" fn(libc::c_int, libc::c_int) -> libc::c_int,
    )
};
#[no_mangle]
pub static mut G_OP_NAME: *const libc::c_char =
    b"add\0" as *const u8 as *const libc::c_char;
#[no_mangle]
pub unsafe extern "C" fn helper_call(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    let mut r: libc::c_int = op_add(a, b);
    let mut acc: libc::c_int = INIT_add;
    acc += 0 as libc::c_int;
    acc += 1 as libc::c_int;
    acc += 2 as libc::c_int;
    acc += 3 as libc::c_int;
    acc += 4 as libc::c_int;
    printf(
        b"helper.call=%d helper.acc=%d\n\0" as *const u8 as *const libc::c_char,
        r,
        acc,
    );
    return r + acc;
}
#[no_mangle]
pub unsafe extern "C" fn helper_ptr(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    let mut fp: Option<
        unsafe extern "C" fn(libc::c_int, libc::c_int) -> libc::c_int,
    > = Some(
        op_add
            as unsafe extern "C" fn(libc::c_int, libc::c_int) -> libc::c_int,
    );
    let mut r: libc::c_int = fp.expect("non-null function pointer")(a, b);
    printf(
        b"helper.ptr=%d\n\0" as *const u8 as *const libc::c_char,
        r,
    );
    return r;
}
#[no_mangle]
pub unsafe extern "C" fn use_generated(mut n: libc::c_int) -> libc::c_int {
    let mut r: libc::c_int = accum_add(n);
    printf(
        b"gen.acc=%d\n\0" as *const u8 as *const libc::c_char,
        r,
    );
    return r;
}
