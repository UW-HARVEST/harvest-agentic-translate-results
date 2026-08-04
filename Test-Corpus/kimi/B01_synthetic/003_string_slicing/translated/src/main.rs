use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 4 || args.len() == 1 {
        eprintln!("Error: there should be one to three arguments passed:");
        eprintln!("<string> [start] [stop]");
        std::process::exit(1);
    }

    let s = &args[1];
    let len = s.len();

    let start: usize;

    if args.len() >= 3 {
        match args[2].parse::<usize>() {
            Ok(n) => start = n,
            Err(_) => {
                eprintln!("Second argument must be an integer!");
                std::process::exit(1);
            }
        }
        if start > len {
            eprintln!("Error: start is off the end of the string!");
            std::process::exit(1);
        }
    } else {
        start = 0;
    }

    let stop: usize;

    if args.len() == 4 {
        match args[3].parse::<usize>() {
            Ok(n) => stop = n,
            Err(_) => {
                eprintln!("Third argument must be an integer!");
                std::process::exit(1);
            }
        }

        if stop > len {
            eprintln!("Error: stop is off the end of the string!");
            std::process::exit(1);
        }

        if stop <= start {
            eprintln!("Error: stop must come after start!");
            std::process::exit(1);
        }
    } else {
        stop = len;
    }

    println!("{}", &s[start..stop]);
}
