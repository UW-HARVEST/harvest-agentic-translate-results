
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::Read;

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32]) {
    for (((o, &m1), &m2), &a) in out
        .iter_mut()
        .zip(mul1.iter())
        .zip(mul2.iter())
        .zip(add.iter())
    {
        *o = m1.wrapping_mul(m2).wrapping_add(a);
    }
}

fn call_fma(data: &[i32]) -> i32 {
    if data.is_empty() {
        return 0;
    }
    let len = data.len();
    let mut out = vec![0_i32; len];
    let ones = vec![1_i32; len];
    let zeros = vec![0_i32; len];

    fma_array(&mut out, &ones, data, &zeros);
    out[len - 1]
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);

    let data: Vec<i32> = input
        .split_ascii_whitespace()
        .map_while(|tok| tok.parse::<i32>().ok())
        .take(100)
        .collect();

    let result = call_fma(&data);
    println!("{}", result);

    0
}