use std::env;
use std::process;

fn q_rsqrt(number: f32) -> f32 {
    let x2: f32 = number * 0.5;
    let mut y: f32 = number;
    let mut i: u32 = y.to_bits();
    i = 0x5f3759df_u32.wrapping_sub(i >> 1);
    y = f32::from_bits(i);
    y = y * (1.5f32 - (x2 * y * y));
    y
}

fn vector_normalize_fast(v: &mut [f32; 3]) {
    let ilength = q_rsqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("{} requires 4 inputs", args[0]);
        process::exit(1);
    }

    let mut inputs: [f32; 3] = [
        args[1].parse::<f64>().unwrap() as f32,
        args[2].parse::<f64>().unwrap() as f32,
        args[3].parse::<f64>().unwrap() as f32,
    ];

    vector_normalize_fast(&mut inputs);

    println!("{:.6} {:.6} {:.6}", inputs[0], inputs[1], inputs[2]);
}
