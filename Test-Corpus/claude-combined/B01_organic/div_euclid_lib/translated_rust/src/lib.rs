use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }
    let int_min: c_int = -0x7fffffff - 1;
    let q: c_int;
    let r: c_int;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != int_min {
            // -v2 cannot overflow because v2 != INT_MIN and v2 < 0
            let nv2 = v2.wrapping_neg();
            q = (v1 / nv2).wrapping_neg();
            r = v1 % nv2;
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != int_min {
        let nv1 = v1.wrapping_neg();
        if v2 >= 0 {
            q = (nv1 / v2).wrapping_neg();
            r = (nv1 % v2).wrapping_neg();
        } else if v2 != int_min {
            let nv2 = v2.wrapping_neg();
            q = nv1 / nv2;
            r = (nv1 % nv2).wrapping_neg();
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        // v1 == INT_MIN; v2 > 0 (since v2 != 0 and v2 >= 0)
        let s = v1.wrapping_add(v2).wrapping_neg();
        q = (s / v2).wrapping_neg().wrapping_sub(1);
        r = (s % v2).wrapping_neg();
    } else if v2 != int_min {
        let nv2 = v2.wrapping_neg();
        let s = v1.wrapping_sub(v2).wrapping_neg();
        q = (s / nv2).wrapping_add(1);
        r = (s % nv2).wrapping_neg();
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 {
        q
    } else if v2 > 0 {
        q.wrapping_sub(1)
    } else {
        q.wrapping_add(1)
    }
}
