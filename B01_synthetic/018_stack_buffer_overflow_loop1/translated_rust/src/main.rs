use std::io::{self, Read};

fn print_int_line(n: i32) {
    println!("{}", n);
}

fn bad() {
    // alloca(10) — only 10 bytes, but we write 10 i32s (bug reproduced)
    let mut buf = [0u8; 10];
    let data = buf.as_mut_ptr() as *mut i32;
    let source: [i32; 10] = [0; 10];
    for i in 0..10 {
        unsafe { *data.add(i) = source[i]; }
    }
    print_int_line(unsafe { *data });
}

fn good() {
    // alloca(10 * sizeof(int)) — correct size
    let mut buf = [0u8; 10 * std::mem::size_of::<i32>()];
    let data = buf.as_mut_ptr() as *mut i32;
    let source: [i32; 10] = [0; 10];
    for i in 0..10 {
        unsafe { *data.add(i) = source[i]; }
    }
    print_int_line(unsafe { *data });
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);
    if x != 0 {
        good();
    } else {
        bad();
    }
}
