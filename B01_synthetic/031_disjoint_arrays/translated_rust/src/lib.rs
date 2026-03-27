use std::io::{self, Read};

#[no_mangle]
pub extern "C" fn fma_array(out: *mut i32, mul1: *const i32, mul2: *const i32, add: *const i32, len: i32) {
    for i in 0..len as usize {
        unsafe {
            *out.add(i) = *mul1.add(i) * (*mul2.add(i)) + *add.add(i);
        }
    }
}

#[no_mangle]
pub extern "C" fn call_fma(data: *const i32, len: i32) -> i32 {
    if len == 0 {
        return 0;
    }
    let len_u = len as usize;
    let mut out = vec![0i32; len_u];
    let ones = vec![1i32; len_u];
    let zeros = vec![0i32; len_u];

    unsafe {
        fma_array(out.as_mut_ptr(), ones.as_ptr(), std::slice::from_raw_parts(data, len_u).as_ptr(), zeros.as_ptr(), len);
    }
    out[len_u - 1]
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let mut data = [0i32; 100];
    let mut i = 0;
    for token in input.split_whitespace() {
        if i >= 100 {
            break;
        }
        if let Ok(val) = token.parse::<i32>() {
            data[i] = val;
            i += 1;
        } else {
            break;
        }
    }

    let result = call_fma(data.as_ptr(), i as i32);
    println!("{}", result);
    0
}
