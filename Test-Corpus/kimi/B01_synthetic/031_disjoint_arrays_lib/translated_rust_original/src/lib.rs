use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32]) {
    for i in 0..out.len() {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[i32]) -> i32 {
    if data.is_empty() {
        return 0;
    }
    let len = data.len();
    let mut out = vec![0i32; len];
    let ones = vec![1i32; len];
    let zeros = vec![0i32; len];

    fma_array(&mut out, &ones, data, &zeros);
    out[len - 1]
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_ptr: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(in_ptr) };
    let s = c_str.to_str().unwrap_or("");
    let mut data = Vec::with_capacity(100);
    let mut remaining = s;

    for _ in 0..100 {
        if remaining.is_empty() {
            break;
        }
        if let Some((num, rest)) = parse_int(remaining) {
            data.push(num);
            remaining = rest;
        } else {
            break;
        }
    }

    let result = call_fma(&data);
    println!("{}", result);
}

fn parse_int(s: &str) -> Option<(i32, &str)> {
    let s = s.trim_start();
    let mut end = 0;
    let mut chars = s.chars().peekable();

    if let Some(&c) = chars.peek() {
        if c == '+' || c == '-' {
            end += c.len_utf8();
            chars.next();
        }
    }

    let start = end;
    while let Some(c) = chars.peek() {
        if c.is_ascii_digit() {
            end += c.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    if start == end {
        return None;
    }

    s[..end].parse::<i32>().ok().map(|n| (n, &s[end..]))
}
