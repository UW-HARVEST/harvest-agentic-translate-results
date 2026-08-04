use std::io::{self, Read};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    let mut out = vec![0i32; len];
    let ones = vec![1i32; len];
    let zeros = vec![0i32; len];

    out[0] = 0;
    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
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

    let result = call_fma(&data[..i], i);
    println!("{}", result);
}
