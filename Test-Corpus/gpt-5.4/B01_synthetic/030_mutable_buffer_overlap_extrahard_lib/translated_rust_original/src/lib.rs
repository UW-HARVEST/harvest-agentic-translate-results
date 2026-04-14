use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    if len <= 0 {
        return;
    }
    let len = len as usize;
    unsafe {
        let out_slice = std::slice::from_raw_parts_mut(out, len);
        let mul1_slice = std::slice::from_raw_parts(mul1, len);
        let mul2_slice = std::slice::from_raw_parts(mul2, len);
        let add_slice = std::slice::from_raw_parts(add, len);
        for i in 0..len {
            out_slice[i] = mul1_slice[i] * mul2_slice[i] + add_slice[i];
        }
    }
}

fn inner(out: &mut [c_int]) {
    let input = out.to_vec();
    for i in 0..out.len() {
        out[i] = input[i] * input[i] + input[i];
    }
    for value in out.iter() {
        println!("{}", value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: *const c_int, len: c_int) {
    if len <= 0 {
        return;
    }
    let len = len as usize;
    let out = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    let mut out = out;
    inner(&mut out);
}
