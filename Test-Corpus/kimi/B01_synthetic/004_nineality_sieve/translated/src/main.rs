use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Error: should only be a single (integer) argument!");
        process::exit(1);
    }

    let val: i64 = match args[1].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let mut current = val;
    loop {
        println!("{}", current);
        if current % 10 == 9 {
            break;
        }
        current += 1;
    }
}
