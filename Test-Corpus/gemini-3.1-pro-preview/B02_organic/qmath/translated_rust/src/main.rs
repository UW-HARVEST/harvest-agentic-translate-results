pub mod surfaceflags;
pub mod q_shared;
pub mod q_math;

use q_shared::*;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut inputs: vec3_t = [0.0; 3];

    if args.len() != 4 {
        eprintln!("{} requires 4 inputs", args[0]);
        process::exit(1);
    }

    inputs[0] = args[1].parse().unwrap_or(0.0);
    inputs[1] = args[2].parse().unwrap_or(0.0);
    inputs[2] = args[3].parse().unwrap_or(0.0);

    VectorNormalizeFast(&mut inputs);

    println!("{} {} {}", inputs[0], inputs[1], inputs[2]);
}
