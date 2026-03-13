use std::io::{self, Read};

fn print_int_line(n: i32) {
    println!("{}", n);
}

fn bad() {
    // C: data = (int *)alloca(10);  — only 10 bytes, not 10 ints (deliberate bug)
    // Then copies 10 ints into it (buffer overflow in C).
    // We replicate the observable behavior: source is all zeros, prints data[0].
    let mut data = [0i32; 10]; // oversized to avoid UB, but same output
    let source = [0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    // C: data = (int *)alloca(10*sizeof(int));  — correct allocation
    let mut data = [0i32; 10];
    let source = [0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn main() {
    // Match C scanf("%d", &x) behavior: skip whitespace, parse integer
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
