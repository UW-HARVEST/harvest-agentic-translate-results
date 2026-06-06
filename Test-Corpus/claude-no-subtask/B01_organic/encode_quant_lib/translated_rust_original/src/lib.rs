use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn encode_quant(
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) -> c_int {
    let mut uni = uni;
    let mut uni1: c_int;
    let mut uni2: c_int;
    let mut diff: c_int;
    let p0: c_int;
    let p1: c_int;
    let p2: c_int;
    let mut d0: c_int;
    let mut d1: c_int;
    let mut d2: c_int;
    let mut d3: c_int;

    uni1 = uni.wrapping_add(1);
    uni2 = uni.wrapping_sub(1);

    if (uni ^ uni1) & !7i32 != 0 {
        uni1 = uni;
    }
    if (uni ^ uni2) & !7i32 != 0 {
        uni2 = uni;
    }
    if lsbit != 0 {
        if lsbit == 4 {
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if lsbit & 1 != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
        }
    }

    diff = ((2i32.wrapping_mul(uni & 7).wrapping_add(1)).wrapping_mul(step)) / 8;
    if uni & 8 != 0 {
        diff = diff.wrapping_neg();
    }
    p0 = pred.wrapping_add(diff);
    d0 = tgt.wrapping_sub(p0);
    d0 = d0 ^ (d0 >> 31);

    diff = ((2i32.wrapping_mul(uni1 & 7).wrapping_add(1)).wrapping_mul(step)) / 8;
    if uni1 & 8 != 0 {
        diff = diff.wrapping_neg();
    }
    p1 = pred.wrapping_add(diff);
    d1 = tgt.wrapping_sub(p1);
    d1 = d1 ^ (d1 >> 31);

    diff = ((2i32.wrapping_mul(uni2 & 7).wrapping_add(1)).wrapping_mul(step)) / 8;
    if uni2 & 8 != 0 {
        diff = diff.wrapping_neg();
    }
    p2 = pred.wrapping_add(diff);
    d2 = tgt.wrapping_sub(p2);
    d2 = d2 ^ (d2 >> 31);

    d3 = tgt2.wrapping_sub(p0);
    d3 = d3 ^ (d3 >> 31);
    d0 = d0.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p1);
    d3 = d3 ^ (d3 >> 31);
    d1 = d1.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p2);
    d3 = d3 ^ (d3 >> 31);
    d2 = d2.wrapping_add(d3 >> 5);

    if d1 < d0 {
        uni = uni1;
    }
    if d2 < d0 {
        uni = uni2;
    }
    uni
}
