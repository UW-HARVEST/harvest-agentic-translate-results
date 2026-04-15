use std::io::{self, Read};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32]) {
    for i in 0..out.len() {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[i32]) -> i32 {
    let len = data.len();
    if len == 0 {
        return 0;
    }
    let mut out = vec![0; len];
    let ones = vec![1; len];
    let zeros = vec![0; len];

    fma_array(&mut out, &ones, data, &zeros);
    out[len - 1]
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let mut data = Vec::new();
    for s in input.split_whitespace() {
        if let Ok(num) = s.parse::<i32>() {
            data.push(num);
            if data.len() == 100 {
                break;
            }
        } else {
            break;
        }
    }

    let result = call_fma(&data);
    println!("{}", result);
}
