use std::slice;

#[no_mangle]
pub unsafe extern "C" fn fma_array(
    out: *mut i32,
    mul1: *const i32,
    mul2: *const i32,
    add: *const i32,
    len: i32,
) {
    let len = len as usize;
    let out = slice::from_raw_parts_mut(out, len);
    let mul1 = slice::from_raw_parts(mul1, len);
    let mul2 = slice::from_raw_parts(mul2, len);
    let add = slice::from_raw_parts(add, len);
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

#[no_mangle]
pub unsafe extern "C" fn call_fma(data: *const i32, len: i32) -> i32 {
    if len == 0 {
        return 0;
    }
    let len = len as usize;
    let data = slice::from_raw_parts(data, len);
    let mut out = vec![0i32; len];
    let ones = vec![1i32; len];
    let zeros = vec![0i32; len];
    fma_array(
        out.as_mut_ptr(),
        ones.as_ptr(),
        data.as_ptr(),
        zeros.as_ptr(),
        len as i32,
    );
    out[len - 1]
}
