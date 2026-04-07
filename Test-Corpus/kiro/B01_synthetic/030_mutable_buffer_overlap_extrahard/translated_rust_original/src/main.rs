use std::io::{self, Read};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn driver(data: &mut [i32], len: usize) {
    // In C, all four pointers are the same array, so we must snapshot values
    // before the in-place fma to match the C behavior where each element
    // reads only its own index (which hasn't been written yet at that point
    // only if we go sequentially — but since out[i] depends only on index i
    // from all arrays, and they're all the same pointer, the C code reads
    // out[i] before writing out[i] within the same iteration).
    // Actually in C: out[i] = out[i] * out[i] + out[i] — the right-hand side
    // is evaluated fully before assignment, so we can just compute per-element.
    let snapshot: Vec<i32> = data[..len].to_vec();
    fma_array(data, &snapshot, &snapshot, &snapshot, len);
    for i in 0..len {
        println!("{}", data[i]);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let mut data = [0i32; 100];
    let mut count = 0;

    for token in input.split_whitespace() {
        if count >= 100 {
            break;
        }
        if let Ok(val) = token.parse::<i32>() {
            data[count] = val;
            count += 1;
        } else {
            break;
        }
    }

    driver(&mut data, count);
}
