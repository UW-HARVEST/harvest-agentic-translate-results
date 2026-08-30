use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    let mut out = vec![0; len];
    let ones = vec![1; len];
    let zeros = vec![0; len];

    out[0] = 0;
    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn main() {
    let mut data = [0_i32; 100];
    let mut len = 0;

    while len < data.len() {
        // SAFETY: the format expects one int pointer, and data[len] is valid and writable.
        let converted = unsafe {
            scanf(
                b"%d\0".as_ptr().cast::<c_char>(),
                &mut data[len] as *mut i32,
            )
        };
        if converted != 1 {
            break;
        }
        len += 1;
    }

    let result = call_fma(&data, len);
    println!("{result}");
}
