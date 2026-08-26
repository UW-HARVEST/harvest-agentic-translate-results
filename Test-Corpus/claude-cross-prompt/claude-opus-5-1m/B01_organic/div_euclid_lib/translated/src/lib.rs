/// Translation of the C `div_euclid` function from c_src/src/lib.c.
///
/// This reproduces the exact behavior (including any quirks) of the C
/// implementation. Wrapping arithmetic is used to mirror the two's-complement
/// behavior typically produced by C compilers and to avoid Rust overflow
/// panics in debug builds, even though the original computation is structured
/// to avoid genuine overflow in most branches.
pub fn div_euclid(v1: i32, v2: i32) -> i32 {
    if v2 == 0 {
        return 0;
    }
    let q: i32;
    let r: i32;
    if v1 >= 0 {
        if v2 >= 0 {
            return v1 / v2;
        } else if v2 != i32::MIN {
            q = (v1 / v2.wrapping_neg()).wrapping_neg();
            r = v1 % v2.wrapping_neg();
        } else {
            q = 0;
            r = v1;
        }
    } else if v1 != i32::MIN {
        if v2 >= 0 {
            q = (v1.wrapping_neg() / v2).wrapping_neg();
            r = (v1.wrapping_neg() % v2).wrapping_neg();
        } else if v2 != i32::MIN {
            q = v1.wrapping_neg() / v2.wrapping_neg();
            r = (v1.wrapping_neg() % v2.wrapping_neg()).wrapping_neg();
        } else {
            q = 1;
            r = v1.wrapping_sub(q.wrapping_mul(v2));
        }
    } else if v2 >= 0 {
        q = (v1.wrapping_add(v2).wrapping_neg() / v2)
            .wrapping_neg()
            .wrapping_sub(1);
        r = (v1.wrapping_add(v2).wrapping_neg() % v2).wrapping_neg();
    } else if v2 != i32::MIN {
        q = (v1.wrapping_sub(v2).wrapping_neg() / v2.wrapping_neg()).wrapping_add(1);
        r = (v1.wrapping_sub(v2).wrapping_neg() % v2.wrapping_neg()).wrapping_neg();
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
