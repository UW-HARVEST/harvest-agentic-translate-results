use std::env;
use std::process;

fn parse_arg(value: &str, name: &str) -> Result<f64, String> {
    match value.parse::<f64>() {
        Ok(v) => Ok(v),
        Err(_) => Err(format!("Invalid numeric input for {}: '{}'", name, value)),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} base exponent", args.first().map(String::as_str).unwrap_or("driver"));
        process::exit(1);
    }

    let base = match parse_arg(&args[1], "base") {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    };

    let exponent = match parse_arg(&args[2], "exponent") {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    };

    let result = base.powf(exponent);

    if result.is_nan() {
        eprintln!("Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.", base, exponent);
        process::exit(1);
    } else if result.is_infinite() || (result == 0.0 && base != 0.0 && exponent.is_finite()) {
        eprintln!("Range error: pow({:.2}, {:.2}) caused overflow or underflow.", base, exponent);
        process::exit(1);
    }

    println!("Result: {:.2}", result);
}
