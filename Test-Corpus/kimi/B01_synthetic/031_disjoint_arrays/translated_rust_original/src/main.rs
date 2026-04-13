use std::io;

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
    let mut data = Vec::new();
    let stdin = io::stdin();
    let mut line = String::new();
    
    while let Ok(n) = stdin.read_line(&mut line) {
        if n == 0 {
            break;
        }
        if let Ok(val) = line.trim().parse::<i32>() {
            data.push(val);
        } else {
            break;
        }
        line.clear();
    }

    let result = call_fma(&data);
    println!("{}", result);
}