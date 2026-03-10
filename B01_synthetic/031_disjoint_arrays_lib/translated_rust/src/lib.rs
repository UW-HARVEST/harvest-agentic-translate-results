use std::ffi::c_char;
use std::os::raw::c_int;

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: c_int) {
    for i in 0..len as usize {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[c_int], len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    let len_u = len as usize;
    let mut out = vec![0 as c_int; len_u];
    let ones = vec![1 as c_int; len_u];
    let zeros = vec![0 as c_int; len_u];

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len_u - 1]
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let mut ptr = input;
    let mut data = [0 as c_int; 100];
    let mut i = 0usize;

    while i < 100 {
        // skip leading whitespace
        unsafe {
            while *ptr == b' ' as c_char
                || *ptr == b'\t' as c_char
                || *ptr == b'\n' as c_char
                || *ptr == b'\r' as c_char
            {
                ptr = ptr.add(1);
            }
        }

        let ch = unsafe { *ptr };
        if ch == 0 {
            break;
        }

        // parse optional sign and digits (mimics sscanf %d)
        let start = ptr;
        unsafe {
            if *ptr == b'-' as c_char || *ptr == b'+' as c_char {
                ptr = ptr.add(1);
            }
        }
        let digit_start = ptr;
        unsafe {
            while (*ptr) >= b'0' as c_char && (*ptr) <= b'9' as c_char {
                ptr = ptr.add(1);
            }
        }
        if ptr == digit_start {
            // no digits found — sscanf would return 0, break
            break;
        }

        // convert the parsed region to an integer
        let slice = unsafe {
            let len = ptr.offset_from(start) as usize;
            std::slice::from_raw_parts(start as *const u8, len)
        };
        let s = std::str::from_utf8(slice).unwrap();
        let val: c_int = s.parse().unwrap();
        data[i] = val;
        i += 1;
    }

    let result = call_fma(&data, i as c_int);
    println!("{}", result);
}
