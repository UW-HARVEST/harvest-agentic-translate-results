use std::io::{self, Read};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn driver(out: &mut [i32], len: usize) {
    // Mirror the C behavior of `fma_array(out, out, out, out, len)`:
    // each element becomes out[i] * out[i] + out[i].
    let snapshot: Vec<i32> = out[..len].to_vec();
    fma_array(out, &snapshot, &snapshot, &snapshot, len);
    for i in 0..len {
        println!("{}", out[i]);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");

    let mut data: [i32; 100] = [0; 100];
    let mut i: usize = 0;
    for token in input.split_ascii_whitespace() {
        if i >= 100 {
            break;
        }
        match token.parse::<i32>() {
            Ok(v) => {
                data[i] = v;
                i += 1;
            }
            Err(_) => break,
        }
    }

    driver(&mut data, i);
}
