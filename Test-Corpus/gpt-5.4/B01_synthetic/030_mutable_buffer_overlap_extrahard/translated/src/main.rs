use std::io::{self, Read};

pub fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

pub fn driver(out: &mut [i32], len: usize) {
    let input = out[..len].to_vec();
    fma_array(out, &input, &input, &input, len);
    for value in &out[..len] {
        println!("{}", value);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let mut data = [0i32; 100];
    let mut i = 0usize;
    for token in input.split_whitespace() {
        if i >= 100 {
            break;
        }
        match token.parse::<i32>() {
            Ok(value) => {
                data[i] = value;
                i += 1;
            }
            Err(_) => break,
        }
    }

    driver(&mut data, i);
}
