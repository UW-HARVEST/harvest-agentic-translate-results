use std::io::{self, Read};

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Mirrors C `bad()`: allocates only 10 BYTES on the stack for 10 i32s
/// (intentional buffer overflow reproduced via unsafe).
fn bad() {
    unsafe {
        // alloca(10): 10 bytes, but we write 10 i32s (40 bytes) — deliberate overflow
        let mut buf = [0u8; 10];
        let data = buf.as_mut_ptr() as *mut i32;

        let source: [i32; 10] = [0; 10];
        for i in 0..10 {
            data.add(i).write(source[i]);
        }
        print_int_line(data.read());
    }
}

/// Mirrors C `good()`: allocates 10*sizeof(int) bytes correctly.
fn good() {
    let mut data = [0i32; 10];
    let source = [0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn main() {
    // Read an integer from stdin matching scanf("%d", &x) behavior:
    // skip leading whitespace, parse an integer.
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
