use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        let prog_name = args.get(0).map(|s| s.as_str()).unwrap_or("driver");
        eprintln!("Usage: {} base exponent", prog_name);
        process::exit(1);
    }

    let base_str = &args[1];
    let base: f64 = match base_str.parse() {
        Ok(val) if val.is_infinite() => {
            eprintln!("Range error while converting base '{}'", base_str);
            process::exit(1);
        }
        Ok(val) => val,
        Err(_) => {
            eprintln!("Invalid numeric input for base: '{}'", base_str);
            process::exit(1);
        }
    };

    let exp_str = &args[2];
    let exponent: f64 = match exp_str.parse() {
        Ok(val) if val.is_infinite() => {
            eprintln!("Range error while converting exponent '{}'", exp_str);
            process::exit(1);
        }
        Ok(val) => val,
        Err(_) => {
            eprintln!("Invalid numeric input for exponent: '{}'", exp_str);
            process::exit(1);
        }
    };

    let result = base.powf(exponent);

    if result.is_nan() {
        eprintln!("Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.", base, exponent);
        process::exit(1);
    } else if result.is_infinite() {
        eprintln!("Range error: pow({:.2}, {:.2}) caused overflow or underflow.", base, exponent);
        process::exit(1);
    }

    println!("Result: {:.2}", result);
}
