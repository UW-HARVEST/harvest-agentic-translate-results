use std::ffi::c_int;

const INT_MIN: c_int = -0x7fffffff - 1;

#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }
    let q: c_int;
    let r: c_int;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != INT_MIN {
            let neg_v2 = v2.wrapping_neg();
            q = (v1 / neg_v2).wrapping_neg();
            r = v1 % neg_v2;
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != INT_MIN {
        let neg_v1 = v1.wrapping_neg();
        if v2 >= 0 {
            q = (neg_v1 / v2).wrapping_neg();
            r = (neg_v1 % v2).wrapping_neg();
        } else if v2 != INT_MIN {
            let neg_v2 = v2.wrapping_neg();
            q = neg_v1 / neg_v2;
            r = (neg_v1 % neg_v2).wrapping_neg();
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        let t = v1.wrapping_add(v2).wrapping_neg();
        q = (t / v2).wrapping_neg().wrapping_sub(1);
        r = (t % v2).wrapping_neg();
    } else if v2 != INT_MIN {
        let t = v1.wrapping_sub(v2).wrapping_neg();
        let neg_v2 = v2.wrapping_neg();
        q = (t / neg_v2).wrapping_add(1);
        r = (t % neg_v2).wrapping_neg();
    } else {
        q = 1;
        r = 0;
    }
    if r >= 0 {
        q
    } else {
        q.wrapping_add(if v2 > 0 { -1 } else { 1 })
    }
}
