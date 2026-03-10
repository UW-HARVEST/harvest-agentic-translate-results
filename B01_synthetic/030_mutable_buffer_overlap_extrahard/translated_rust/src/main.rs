use std::io::{self, Read};

fn driver(out: &mut [i32], len: usize) {
    // In C, all four pointers are the same (out, out, out, out).
    // Each iteration reads out[i] three times then writes out[i].
    // Since only index i is accessed per iteration, we can snapshot and compute in place.
    for i in 0..len {
        let v = out[i];
        out[i] = v * v + v;
    }
    for i in 0..len {
        println!("{}", out[i]);
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
        if let Ok(v) = token.parse::<i32>() {
            data[count] = v;
            count += 1;
        } else {
            break;
        }
    }

    driver(&mut data, count);
}
