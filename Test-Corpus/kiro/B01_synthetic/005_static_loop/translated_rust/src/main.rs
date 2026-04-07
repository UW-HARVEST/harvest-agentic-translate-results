use std::env;
use std::process;

fn static_sum(sum: &mut i32, update: i32) -> i32 {
    *sum += update;
    *sum
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        process::exit(1);
    }

    // strtol parses leading digits; if none parsed, it's an error.
    // We replicate that: parse the longest leading integer prefix.
    let arg = &args[1];
    let end = arg
        .find(|c: char| !(c == '-' || c == '+' || c.is_ascii_digit()))
        .unwrap_or(arg.len());
    let numeric_prefix = &arg[..end];

    let stride: i32 = match numeric_prefix.parse() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let mut sum = 0i32;
    for i in 0..10 {
        println!("{}", static_sum(&mut sum, i * stride));
    }
}
