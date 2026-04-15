use std::env;
use std::process;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

fn perform_expensive_operations(array: &mut [i32]) {
    for x in array.iter_mut() {
        let mut val = *x;
        for _ in 0..100 {
            val = val.wrapping_mul(3).wrapping_add(7);
            val ^= val >> 3;
            val = val.wrapping_sub(val << 1);
            val = (val / 2).wrapping_add(val % 7);
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

    let mut array = vec![0i32; ARRAY_SIZE];

    unsafe {
        libc::srand(seed as libc::c_uint);
        for i in 0..ARRAY_SIZE {
            array[i] = libc::rand() as i32;
        }
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result = 0i32;
    for &val in array.iter() {
        xor_result ^= val;
    }

    println!("{}", xor_result);
}
