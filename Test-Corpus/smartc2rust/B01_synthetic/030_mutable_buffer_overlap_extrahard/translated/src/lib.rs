
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, BufRead, Write};

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    // Stub implementation
    let stdin = io::stdin();
    let mut data: [i32; 100] = [0; 100];
    let mut i: usize = 0;
    let mut input = String::new();
    let mut handle = stdin.lock();
    while i < 100 {
        input.clear();
        match handle.read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {
                match input.trim().parse::<i32>() {
                    Ok(v) => { data[i] = v; i += 1; }
                    Err(_) => break,
                }
            }
            Err(_) => break,
        }
    }

    // driver: fma_array(out, out, out, out, i) then print
    for j in 0..i {
        data[j] = data[j] * data[j] + data[j];
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for j in 0..i {
        writeln!(out, "{}", data[j]).ok();
    }
    0
}



fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn driver(out: &mut [i32], len: usize) {
    // In C: fma_array(out, out, out, out, len) with all aliasing pointers.
    // Equivalent aliased semantics: out[i] = out[i] * out[i] + out[i]
    for slot in out.iter_mut().take(len) {
        let v = *slot;
        *slot = v * v + v;
    }
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for v in out.iter().take(len) {
        writeln!(handle, "{}", v).ok();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_030_mutable_buffer_overlap_extrahard_main() -> i32 {
    let stdin = io::stdin();
    let mut data: [i32; 100] = [0; 100];
    let mut count: usize = 0;
    let mut input = String::new();
    let mut handle = stdin.lock();

    while count < data.len() {
        input.clear();
        match handle.read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => match input.trim().parse::<i32>() {
                Ok(v) => {
                    data[count] = v;
                    count += 1;
                }
                Err(_) => break,
            },
            Err(_) => break,
        }
    }

    driver(&mut data, count);
    0
}
