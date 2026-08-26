use std::io::{self, Read};

fn fma_array(out_data: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32]) {
    let len = out_data.len();
    for i in 0..len {
        out_data[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn driver(out_data: &mut [i32]) {
    let copy = out_data.to_vec();
    fma_array(out_data, &copy, &copy, &copy);
    for i in 0..out_data.len() {
        println!("{}", out_data[i]);
    }
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
    driver(&mut data);
}