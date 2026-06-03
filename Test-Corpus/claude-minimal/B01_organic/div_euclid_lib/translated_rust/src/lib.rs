#[no_mangle]
pub extern "C" fn div_euclid(v1: i32, v2: i32) -> i32 {
    if v2 == 0 {
        return 0;
    }
    let q: i32;
    let r: i32;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != -0x7fffffff - 1 {
            q = -(v1 / -v2);
            r = v1 % -v2;
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != -0x7fffffff - 1 {
        if v2 >= 0 {
            q = -(-v1 / v2);
            r = -(-v1 % v2);
        } else if v2 != -0x7fffffff - 1 {
            q = -v1 / -v2;
            r = -(-v1 % -v2);
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        q = -(-(v1 + v2) / v2) - 1;
        r = -(-(v1 + v2) % v2);
    } else if v2 != -0x7fffffff - 1 {
        q = (-(v1 - v2) / -v2) + 1;
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
