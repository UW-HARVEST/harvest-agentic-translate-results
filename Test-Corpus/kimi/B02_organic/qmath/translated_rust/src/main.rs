use std::env;
use std::process::exit;

mod q_math;
use q_math::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("{} requires 4 inputs", args[0]);
        exit(1);
    }

    let mut inputs: Vec3 = [
        args[1].parse().unwrap_or(0.0),
        args[2].parse().unwrap_or(0.0),
        args[3].parse().unwrap_or(0.0),
    ];

    vector_normalize_fast(&mut inputs);

    println!("{} {} {}", inputs[0], inputs[1], inputs[2]);
}
