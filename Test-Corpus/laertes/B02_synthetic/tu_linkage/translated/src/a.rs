pub type size_t = usize;
#[inline]
unsafe extern "C" fn a_bias_call(
    mut fp: Option<unsafe extern "C" fn(libc::c_int) -> libc::c_int>,
    mut x: libc::c_int,
) -> libc::c_int {
    return fp.expect("non-null function pointer")(
        (x ^ 0x55 as libc::c_int) + 7 as libc::c_int,
    );
}
static mut state_a: libc::c_int = 0;
unsafe extern "C" fn target(mut code: libc::c_int) -> libc::c_int {
    if code < 0 as libc::c_int {
        return if state_a & 1 as libc::c_int != 0 {
            6 as libc::c_int
        } else {
            5 as libc::c_int
        };
    }
    state_a = state_a ^ code << 1 as libc::c_int;
    let mut k: libc::c_int =
        (code >> 2 as libc::c_int ^ state_a) & 7 as libc::c_int;
    match k {
        0 => return 0 as libc::c_int,
        1 => return 2 as libc::c_int,
        2 => return 4 as libc::c_int,
        3 => return 1 as libc::c_int,
        4 => return 3 as libc::c_int,
        5 | 6 => return 5 as libc::c_int,
        _ => return 7 as libc::c_int,
    };
}
#[inline]
 extern "C" fn wrap(mut x: libc::c_int) -> libc::c_int {
    return target(x - 5 as libc::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn call_a_once(mut x: libc::c_int) -> libc::c_int {
    let mut fp: Option<unsafe extern "C" fn(libc::c_int) -> libc::c_int> =
        Some(target as unsafe extern "C" fn(libc::c_int) -> libc::c_int);
    let mut a: libc::c_int = fp.expect("non-null function pointer")(x);
    let mut b: libc::c_int = wrap(a);
    let mut c: libc::c_int = target(b ^ 3 as libc::c_int);
    let mut d: libc::c_int = a_bias_call(
        Some(target as unsafe extern "C" fn(libc::c_int) -> libc::c_int),
        b,
    );
    return a
        ^ b << 1 as libc::c_int
        ^ c << 2 as libc::c_int
        ^ d << 3 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn process_a_stream(
    mut xs: *const libc::c_int,
    mut n: size_t,
) -> libc::c_int {
    let mut acc: size_t = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while i < n {
        let mut v: libc::c_int = *xs.offset(i as isize);
        let mut j: libc::c_int = 0 as libc::c_int;
        while j < 3 as libc::c_int {
            let mut t: libc::c_int = target(v + j);
            if t & 1 as libc::c_int == 0 as libc::c_int {
                acc = (acc as libc::c_ulong).wrapping_add(t as libc::c_ulong)
                    as size_t as size_t;
            } else {
                acc = (acc as libc::c_ulong ^ (t << j) as libc::c_ulong) as size_t;
                if t == 5 as libc::c_int {
                    break;
                }
            }
            j += 1;
        }
        i = i.wrapping_add(1);
    }
    if acc as libc::c_ulonglong > 0x7fffffff as libc::c_ulonglong {
        acc = 0x7fffffff as libc::c_longlong as size_t;
    }
    if (acc as libc::c_ulonglong)
        < -(0x80000000 as libc::c_longlong) as libc::c_ulonglong
    {
        acc = -(0x80000000 as libc::c_longlong) as size_t;
    }
    return acc as libc::c_int;
}
