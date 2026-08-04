#[no_mangle]
pub unsafe extern "C" fn div_euclid(
    mut v1: ::core::ffi::c_int,
    mut v2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if v2 == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    let mut q: ::core::ffi::c_int = 0;
    let mut r: ::core::ffi::c_int = 0;
    if v1 >= 0 as ::core::ffi::c_int {
        if v2 >= 0 as ::core::ffi::c_int {
            return v1 / v2;
        } else if v2 != -(0x7fffffff as ::core::ffi::c_int) - 1 as ::core::ffi::c_int {
            q = -(v1 / -v2);
            r = v1 % -v2;
        } else {
            q = 0 as ::core::ffi::c_int;
            r = v1;
        }
    } else if v1 != -(0x7fffffff as ::core::ffi::c_int) - 1 as ::core::ffi::c_int {
        if v2 >= 0 as ::core::ffi::c_int {
            q = -(-v1 / v2);
            r = -(-v1 % v2);
        } else if v2 != -(0x7fffffff as ::core::ffi::c_int) - 1 as ::core::ffi::c_int {
            q = -v1 / -v2;
            r = -(-v1 % -v2);
        } else {
            q = 1 as ::core::ffi::c_int;
            r = v1 - q * v2;
        }
    } else if v2 >= 0 as ::core::ffi::c_int {
        q = -(-(v1 + v2) / v2) - 1 as ::core::ffi::c_int;
        r = -(-(v1 + v2) % v2);
    } else if v2 != -(0x7fffffff as ::core::ffi::c_int) - 1 as ::core::ffi::c_int {
        q = -(v1 - v2) / -v2 + 1 as ::core::ffi::c_int;
        r = -(-(v1 - v2) % -v2);
    } else {
        q = 1 as ::core::ffi::c_int;
        r = 0 as ::core::ffi::c_int;
    }
    if r >= 0 as ::core::ffi::c_int {
        return q;
    } else {
        return q
            + (if v2 > 0 as ::core::ffi::c_int {
                -(1 as ::core::ffi::c_int)
            } else {
                1 as ::core::ffi::c_int
            });
    };
}
