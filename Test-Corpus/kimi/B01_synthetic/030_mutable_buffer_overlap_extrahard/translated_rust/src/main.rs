use std::io::{self, BufRead};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32]) {
    for i in 0..out.len() {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn driver(out: &mut [i32]) {
    let len = out.len();
    let mul1: Vec<i32> = out.to_vec();
    let mul2: Vec<i32> = out.to_vec();
    let add: Vec<i32> = out.to_vec();
    fma_array(out, &mul1, &mul2, &add);
    for i in 0..len {
        println!("{}", out[i]);
    }
}

fn main() {
    let stdin = io::stdin();
    let mut data: Vec<i32> = Vec::with_capacity(100);
    
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            if let Ok(num) = line.trim().parse::<i32>() {
                data.push(num);
                if data.len() >= 100 {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
    
    driver(&mut data);
}