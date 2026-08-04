use std::env;
use std::process;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;

fn perform_expensive_operations(array: &mut [i32]) {
    for x in array.iter_mut() {
        let mut val = *x;
        for _ in 0..100 {
            val = val * 3 + 7;
            val = val ^ (val >> 3);
            val = val - (val << 1);
            val = val / 2 + val % 7;
        }
        *x = val;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <seed>", args[0]);
        process::exit(1);
    }

    let seed: u32 = match args[1].parse() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Invalid seed: '{}'", args[1]);
            process::exit(1);
        }
    };

    let mut array: Vec<i32> = Vec::with_capacity(ARRAY_SIZE);
    let mut rng = seed;
    for _ in 0..ARRAY_SIZE {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        array.push((rng >> 16) as i32 & 0x7FFF);
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let xor_result: i32 = array.iter().fold(0, |acc, &x| acc ^ x);

    println!("{}", xor_result);
}
