use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Error: should only be a single (integer) argument!");
        process::exit(1);
    }

    let mut val: i32 = match args[1].parse() {
        Ok(n) => n,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    loop {
        println!("{}", val);
        if val % 10 == 9 {
            break;
        }
        val += 1;
    }
}
