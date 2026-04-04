#[no_mangle]
pub unsafe extern "C" fn encode_quant(
    mut uni: ::core::ffi::c_int,
    mut step: ::core::ffi::c_int,
    mut pred: ::core::ffi::c_int,
    mut tgt: ::core::ffi::c_int,
    mut tgt2: ::core::ffi::c_int,
    mut lsbit: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut uni1: ::core::ffi::c_int = 0;
    let mut uni2: ::core::ffi::c_int = 0;
    let mut diff: ::core::ffi::c_int = 0;
    let mut p0: ::core::ffi::c_int = 0;
    let mut p1: ::core::ffi::c_int = 0;
    let mut p2: ::core::ffi::c_int = 0;
    let mut p3: ::core::ffi::c_int = 0;
    let mut d0: ::core::ffi::c_int = 0;
    let mut d1: ::core::ffi::c_int = 0;
    let mut d2: ::core::ffi::c_int = 0;
    let mut d3: ::core::ffi::c_int = 0;
    uni1 = uni + 1 as ::core::ffi::c_int;
    uni2 = uni - 1 as ::core::ffi::c_int;
    if (uni ^ uni1) & !(7 as ::core::ffi::c_int) != 0 {
        uni1 = uni;
    }
    if (uni ^ uni2) & !(7 as ::core::ffi::c_int) != 0 {
        uni2 = uni;
    }
    if lsbit != 0 {
        if lsbit == 4 as ::core::ffi::c_int {
            uni &= !(1 as ::core::ffi::c_int);
            uni1 &= !(1 as ::core::ffi::c_int);
            uni2 &= !(1 as ::core::ffi::c_int);
            uni |= uni >> 1 as ::core::ffi::c_int
                & uni >> 2 as ::core::ffi::c_int
                & 1 as ::core::ffi::c_int;
            uni1 |= uni1 >> 1 as ::core::ffi::c_int
                & uni1 >> 2 as ::core::ffi::c_int
                & 1 as ::core::ffi::c_int;
            uni2 |= uni2 >> 1 as ::core::ffi::c_int
                & uni2 >> 2 as ::core::ffi::c_int
                & 1 as ::core::ffi::c_int;
        } else if lsbit & 1 as ::core::ffi::c_int != 0 {
            uni |= 1 as ::core::ffi::c_int;
            uni1 |= 1 as ::core::ffi::c_int;
            uni2 |= 1 as ::core::ffi::c_int;
        } else {
            uni &= !(1 as ::core::ffi::c_int);
            uni1 &= !(1 as ::core::ffi::c_int);
            uni2 &= !(1 as ::core::ffi::c_int);
        }
    }
    diff = (2 as ::core::ffi::c_int * (uni & 7 as ::core::ffi::c_int) + 1 as ::core::ffi::c_int)
        * step
        / 8 as ::core::ffi::c_int;
    if uni & 8 as ::core::ffi::c_int != 0 {
        diff = -diff;
    }
    p0 = pred + diff;
    d0 = tgt - p0;
    d0 = d0 ^ d0 >> 31 as ::core::ffi::c_int;
    diff = (2 as ::core::ffi::c_int * (uni1 & 7 as ::core::ffi::c_int) + 1 as ::core::ffi::c_int)
        * step
        / 8 as ::core::ffi::c_int;
    if uni1 & 8 as ::core::ffi::c_int != 0 {
        diff = -diff;
    }
    p1 = pred + diff;
    d1 = tgt - p1;
    d1 = d1 ^ d1 >> 31 as ::core::ffi::c_int;
    diff = (2 as ::core::ffi::c_int * (uni2 & 7 as ::core::ffi::c_int) + 1 as ::core::ffi::c_int)
        * step
        / 8 as ::core::ffi::c_int;
    if uni2 & 8 as ::core::ffi::c_int != 0 {
        diff = -diff;
    }
    p2 = pred + diff;
    d2 = tgt - p2;
    d2 = d2 ^ d2 >> 31 as ::core::ffi::c_int;
    d3 = tgt2 - p0;
    d3 = d3 ^ d3 >> 31 as ::core::ffi::c_int;
    d0 += d3 >> 5 as ::core::ffi::c_int;
    d3 = tgt2 - p1;
    d3 = d3 ^ d3 >> 31 as ::core::ffi::c_int;
    d1 += d3 >> 5 as ::core::ffi::c_int;
    d3 = tgt2 - p2;
    d3 = d3 ^ d3 >> 31 as ::core::ffi::c_int;
    d2 += d3 >> 5 as ::core::ffi::c_int;
    if d1 < d0 {
        uni = uni1;
    }
    if d2 < d0 {
        uni = uni2;
    }
    return uni;
}
