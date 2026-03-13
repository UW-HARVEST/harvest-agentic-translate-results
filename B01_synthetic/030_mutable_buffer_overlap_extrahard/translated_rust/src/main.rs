use std::io::{self, BufRead};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn driver(data: &mut [i32], len: usize) {
    // In C, all four pointers alias the same buffer.
    // Since fma_array reads index i before writing index i,
    // the effective operation is: out[i] = out[i] * out[i] + out[i]
    // We snapshot to replicate the aliased-pointer semantics.
    let snap = data[..len].to_vec();
    fma_array(&mut data[..len], &snap, &snap, &snap, len);
    for i in 0..len {
        println!("{}", data[i]);
    }
}

fn main() {
    let mut data = [0i32; 100];
    let mut count = 0usize;
    let stdin = io::stdin();
    // scanf("%d") skips whitespace (including newlines) and reads one int.
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        for token in line.split_whitespace() {
            if count >= 100 {
                break;
            }
            match token.parse::<i32>() {
                Ok(v) => {
                    data[count] = v;
                    count += 1;
                }
                Err(_) => {
                    // scanf returns != 1 on parse failure -> break
                    driver(&mut data, count);
                    return;
                }
            }
        }
        if count >= 100 {
            break;
        }
    }
    driver(&mut data, count);
}
