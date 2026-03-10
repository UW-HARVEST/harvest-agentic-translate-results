use std::env;
use std::process;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;

static mut ARRAY: [i32; ARRAY_SIZE] = [0i32; ARRAY_SIZE];

fn perform_expensive_operations() {
    unsafe {
        for i in 0..ARRAY_SIZE {
            let mut x: i32 = ARRAY[i];
            for _ in 0..100 {
                x = x.wrapping_mul(3).wrapping_add(7);
                x = x ^ (x >> 3);
                x = x.wrapping_sub(x.wrapping_shl(1));
                x = x / 2 + x % 7;
            }
            ARRAY[i] = x;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <seed>", args[0]);
        process::exit(1);
    }

    let seed: libc::c_uint = match args[1].parse::<u64>() {
        Ok(v) if v <= u32::MAX as u64 => v as libc::c_uint,
        _ => {
            eprintln!("Invalid seed: '{}'", args[1]);
            process::exit(1);
        }
    };

    unsafe {
        libc::srand(seed);
        for i in 0..ARRAY_SIZE {
            ARRAY[i] = libc::rand();
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations();
    }

    let mut xor_result: i32 = 0;
    unsafe {
        for i in 0..ARRAY_SIZE {
            xor_result ^= ARRAY[i];
        }
    }

    println!("{}", xor_result);
}
