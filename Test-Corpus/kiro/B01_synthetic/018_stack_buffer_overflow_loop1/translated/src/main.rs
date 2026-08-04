use std::io::{self, Read};

fn print_int_line(n: i32) {
    println!("{}", n);
}

fn bad() {
    // C code allocates only 10 bytes (not 10*sizeof(int)) then writes 10 ints — buffer overflow.
    // source is all zeros, so data[0] = 0 regardless.
    let source = [0i32; 10];
    let mut data = vec![0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let source = [0i32; 10];
    let mut data = vec![0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn main() {
    let mut x: i32 = 0;

    // Mimic scanf("%d", &x): skip whitespace, parse integer
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        let token = input.split_whitespace().next();
        if let Some(s) = token {
            if let Ok(v) = s.parse::<i32>() {
                x = v;
            }
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
