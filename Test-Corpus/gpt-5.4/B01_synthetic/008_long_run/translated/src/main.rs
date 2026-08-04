use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::env;
use std::process;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

fn perform_expensive_operations(array: &mut [i32]) {
    for x in array.iter_mut() {
        let mut v = *x;
        for _ in 0..100 {
            v = v.wrapping_mul(3).wrapping_add(7);
            v ^= v >> 3;
            v = v.wrapping_sub(v.wrapping_shl(1));
            let div = if v == i32::MIN { i32::MIN / 2 } else { v / 2 };
            let rem = if v == i32::MIN { i32::MIN % 7 } else { v % 7 };
            v = div.wrapping_add(rem);
        }
        *x = v;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <seed>", args[0]);
        process::exit(1);
    }

    let temp_seed: u64 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid seed: '{}'", args[1]);
            process::exit(1);
        }
    };

    if temp_seed > u32::MAX as u64 {
        eprintln!("Invalid seed: '{}'", args[1]);
        process::exit(1);
    }

    let seed = temp_seed as u32;
    let mut rng = StdRng::seed_from_u64(seed as u64);
    let mut array = vec![0i32; ARRAY_SIZE];

    for x in array.iter_mut() {
        *x = rng.gen::<i32>() & libc::RAND_MAX;
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let xor_result = array.iter().fold(0i32, |acc, &x| acc ^ x);
    println!("{}", xor_result);
}
