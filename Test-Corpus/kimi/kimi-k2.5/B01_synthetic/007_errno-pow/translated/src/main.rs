use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} base exponent", args[0]);
        process::exit(1);
    }
    
    let base: f64 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid numeric input for base: '{}'", args[1]);
            process::exit(1);
        }
    };
    
    let exponent: f64 = match args[2].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid numeric input for exponent: '{}'", args[2]);
            process::exit(1);
        }
    };
    
    let result = base.powf(exponent);
    
    if result.is_nan() {
        eprintln!("Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.", base, exponent);
        process::exit(1);
    }
    
    if result.is_infinite() {
        eprintln!("Range error: pow({:.2}, {:.2}) caused overflow or underflow.", base, exponent);
        process::exit(1);
    }
    
    println!("Result: {:.2}", result);
}
