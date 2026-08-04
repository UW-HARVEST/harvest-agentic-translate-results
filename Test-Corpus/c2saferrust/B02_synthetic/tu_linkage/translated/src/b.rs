pub type size_t = usize;
#[inline]
unsafe extern "C" fn b_twist_call(
    mut fp: Option<unsafe extern "C" fn(::core::ffi::c_int) -> ::core::ffi::c_int>,
    mut x: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return fp.expect("non-null function pointer")(
        (x + 9 as ::core::ffi::c_int ^ 0x2222 as ::core::ffi::c_int) - 17 as ::core::ffi::c_int,
    );
}
static mut flipflop: ::core::ffi::c_int = 0;
unsafe extern "C" fn target(mut code: ::core::ffi::c_int) -> ::core::ffi::c_int {
    flipflop ^= 1 as ::core::ffi::c_int;
    if code < 0 as ::core::ffi::c_int {
        return if flipflop != 0 {
            2 as ::core::ffi::c_int
        } else {
            6 as ::core::ffi::c_int
        };
    }
    let mut z: ::core::ffi::c_int = (code
        ^ (if flipflop != 0 {
            0x7f as ::core::ffi::c_int
        } else {
            0x1f as ::core::ffi::c_int
        }))
        % 8 as ::core::ffi::c_int;
    if z == 0 as ::core::ffi::c_int || z == 7 as ::core::ffi::c_int {
        return 4 as ::core::ffi::c_int;
    }
    if z == 1 as ::core::ffi::c_int || z == 2 as ::core::ffi::c_int {
        return 3 as ::core::ffi::c_int;
    }
    if z == 3 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if z == 4 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    if z == 5 as ::core::ffi::c_int {
        return 5 as ::core::ffi::c_int;
    }
    return 7 as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn w2(mut x: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return target(x + 9 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn call_b_once(mut x: ::core::ffi::c_int) -> ::core::ffi::c_int {
    let mut fp: Option<unsafe extern "C" fn(::core::ffi::c_int) -> ::core::ffi::c_int> =
        Some(target as unsafe extern "C" fn(::core::ffi::c_int) -> ::core::ffi::c_int);
    let mut a: ::core::ffi::c_int = target(x);
    let mut b: ::core::ffi::c_int = w2(a);
    let mut c: ::core::ffi::c_int = b_twist_call(
        Some(target as unsafe extern "C" fn(::core::ffi::c_int) -> ::core::ffi::c_int),
        a,
    );
    let mut d: ::core::ffi::c_int = fp.expect("non-null function pointer")(c ^ x);
    return a << 1 as ::core::ffi::c_int
        ^ b << 2 as ::core::ffi::c_int
        ^ c << 3 as ::core::ffi::c_int
        ^ d << 4 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn process_b_stream(
    mut xs: *const ::core::ffi::c_int,
    mut n: size_t,
) -> ::core::ffi::c_int {
    let mut acc: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut i: size_t = 0 as size_t;
    while i < n {
        let mut v: ::core::ffi::c_int = *xs.offset(i as isize);
        let mut iter: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        loop {
            iter += 1;
            if !(iter <= 4 as ::core::ffi::c_int) {
                break;
            }
            let mut t: ::core::ffi::c_int = target(v - iter);
            if t == 6 as ::core::ffi::c_int {
                acc -= t;
                break;
            } else {
                if t == 3 as ::core::ffi::c_int {
                    continue;
                }
                acc = acc * 3 as ::core::ffi::c_int ^ t;
            }
        }
        i = i.wrapping_add(1);
    }
    return acc;
}
