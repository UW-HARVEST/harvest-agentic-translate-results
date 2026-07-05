
#[no_mangle]
pub fn div_euclid(v1: i32, v2: i32) -> i32 {
    if v2 == 0 {
        return 0;
    }

    let q = v1.div_euclid(v2);
    let r = v1.rem_euclid(v2);

    if r == 0 {
        q
    } else {
        v1 / v2 - ((v1 % v2 != 0 && (v1 < 0) != (v2 < 0)) as i32)
    }
}

