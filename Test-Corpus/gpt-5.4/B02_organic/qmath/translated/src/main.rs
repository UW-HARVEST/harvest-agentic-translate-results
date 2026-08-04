mod q_shared;
mod q_math;

use q_shared::VectorNormalizeFast;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut inputs = [0.0f32; 3];
    if args.len() != 4 {
        eprintln!("{} requires 4 inputs", args[0]);
        std::process::exit(1);
    }

    inputs[0] = args[1].parse::<f32>().unwrap_or(0.0);
    inputs[1] = args[2].parse::<f32>().unwrap_or(0.0);
    inputs[2] = args[3].parse::<f32>().unwrap_or(0.0);

    VectorNormalizeFast(&mut inputs);

    println!("{} {} {}", inputs[0], inputs[1], inputs[2]);
}
