use std::env;
use std::io::{self, Write};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc > 4 || argc == 1 {
        println!("Error: there should be one to three arguments passed:");
        println!("<string> [start] [stop]");
        process::exit(1);
    }

    let s = &args[1];
    let len = s.len();
    let mut start: usize = 0;
    let mut stop: usize = len;

    if argc >= 3 {
        match args[2].parse::<isize>() {
            Ok(n) => {
                if n < 0 || n as usize > len {
                    println!("Error: start is off the end of the string!");
                    process::exit(1);
                }
                start = n as usize;
            }
            Err(_) => {
                print!("Second argument must be an integer!");
                process::exit(1);
            }
        }
    }

    if argc == 4 {
        match args[3].parse::<isize>() {
            Ok(n) => {
                if n < 0 || n as usize > len {
                    println!("Error: stop is off the end of the string!");
                    process::exit(1);
                }
                stop = n as usize;
            }
            Err(_) => {
                print!("Third argument must be an integer!");
                process::exit(1);
            }
        }
        if stop <= start {
            println!("Error: stop must come after start!");
            process::exit(1);
        }
    }

    let bytes = s.as_bytes();
    let slice = &bytes[start..stop];
    let mut stdout = io::stdout();
    let _ = stdout.write_all(slice);
    let _ = stdout.write_all(b"\n");
}
