#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let mut data = [0i32; 100];
    let mut count = 0;

    for token in input.split_whitespace() {
        if count >= 100 {
            break;
        }
        if let Ok(val) = token.parse::<i32>() {
            data[count] = val;
            count += 1;
        } else {
            break;
        }
    }

    driver(data.as_mut_ptr(), count as i32);
    0
}

#[no_mangle]
pub extern "C" fn fma_array(out: *mut i32, mul1: *const i32, mul2: *const i32, add: *const i32, len: i32) {
    for i in 0..len as usize {
        unsafe {
            *out.add(i) = *mul1.add(i) * (*mul2.add(i)) + *add.add(i);
        }
    }
}

#[no_mangle]
pub extern "C" fn driver(out: *mut i32, len: i32) {
    fma_array(out, out, out, out, len);
    for i in 0..len as usize {
        unsafe {
            println!("{}", *out.add(i));
        }
    }
}
