use std::io::{self, Read};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn driver(out: &mut [i32], len: usize) {
    // In C, all four pointer args are the same buffer.
    // out[i] = out[i] * out[i] + out[i] for each i, mutating in place.
    // We must clone to replicate the aliased-pointer semantics exactly.
    let copy = out[..len].to_vec();
    fma_array(out, &copy, &copy, &copy, len);
    for i in 0..len {
        println!("{}", out[i]);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let mut data = [0i32; 100];
    let mut count = 0usize;
    for token in input.split_whitespace() {
        if count >= 100 {
            break;
        }
        if let Ok(v) = token.parse::<i32>() {
            data[count] = v;
            count += 1;
        } else {
            break;
        }
    }

    driver(&mut data, count);
}
