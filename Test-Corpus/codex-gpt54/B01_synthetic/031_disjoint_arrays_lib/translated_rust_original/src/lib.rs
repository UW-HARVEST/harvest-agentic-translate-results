use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn sscanf(input: *const c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[c_int], len: usize) -> c_int {
    if len == 0 {
        return 0;
    }

    let mut out = vec![0; len];
    let mut ones = vec![0; len];
    let mut zeros = vec![0; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(mut input: *const c_char) {
    let mut data = [0; 100];
    let mut i = 0usize;

    while i < 100 {
        let mut nb = 0usize;
        if unsafe {
            sscanf(
                input,
                c"%d%zn".as_ptr(),
                std::ptr::addr_of_mut!(data[i]),
                std::ptr::addr_of_mut!(nb),
            )
        } != 1
        {
            break;
        }

        input = unsafe { input.add(nb) };
        i += 1;
    }

    let result = call_fma(&data, i);
    unsafe {
        printf(c"%d\n".as_ptr(), result);
    }
}
