use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }

    let (q, r);
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != c_int::MIN {
            q = -(v1 / -v2);
            r = v1 % -v2;
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != c_int::MIN {
        if v2 >= 0 {
            q = -(-v1 / v2);
            r = -(-v1 % v2);
        } else if v2 != c_int::MIN {
            q = -v1 / -v2;
            r = -(-v1 % -v2);
        } else {
            q = 1;
            r = v1 - q * v2;
        }
    } else if v2 >= 0 {
        q = -(-(v1 + v2) / v2) - 1;
        r = -(-(v1 + v2) % v2);
    } else if v2 != c_int::MIN {
        q = (-(v1 - v2) / -v2).wrapping_add(1);
        r = -(-(v1 - v2) % -v2);
    } else {
        q = 1;
        r = 0;
    }

    if r >= 0 {
        q
    } else {
        q + if v2 > 0 { -1 } else { 1 }
    }
}
