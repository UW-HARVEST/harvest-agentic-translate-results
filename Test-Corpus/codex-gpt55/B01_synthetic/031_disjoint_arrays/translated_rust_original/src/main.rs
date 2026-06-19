use core::ffi::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
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

    let mut out = vec![0 as c_int; len];
    let mut ones = vec![0 as c_int; len];
    let mut zeros = vec![0 as c_int; len];

    out[0] = 0;
    for i in 0..len {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn main() {
    let mut data = [0 as c_int; 100];
    let mut i = 0usize;

    while i < 100 {
        let scanned = unsafe { scanf(b"%d\0".as_ptr().cast::<c_char>(), &mut data[i]) };
        if scanned != 1 {
            break;
        }
        i += 1;
    }

    let result = call_fma(&data, i);
    unsafe {
        printf(b"%d\n\0".as_ptr().cast::<c_char>(), result);
    }
}
