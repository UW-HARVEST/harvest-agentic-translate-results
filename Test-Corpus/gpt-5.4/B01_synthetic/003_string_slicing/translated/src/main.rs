use std::env;
use std::process;

fn parse_index(s: &str) -> Option<usize> {
    s.parse::<usize>().ok()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc > 4 || argc == 1 {
        print!("Error: there should be one to three arguments passed:\n");
        print!("<string> [start] [stop]\n");
        process::exit(1);
    }

    let input = &args[1];
    let len = input.len();

    let start = if argc >= 3 {
        match parse_index(&args[2]) {
            Some(v) => {
                if v > len {
                    print!("Error: start is off the end of the string!\n");
                    process::exit(1);
                }
                v
            }
            None => {
                print!("Second argument must be an integer!");
                process::exit(1);
            }
        }
    } else {
        0
    };

    let stop = if argc == 4 {
        match parse_index(&args[3]) {
            Some(v) => {
                if v > len {
                    print!("Error: stop is off the end of the string!\n");
                    process::exit(1);
                }
                if v <= start {
                    print!("Error: stop must come after start!\n");
                    process::exit(1);
                }
                v
            }
            None => {
                print!("Third argument must be an integer!");
                process::exit(1);
            }
        }
    } else {
        len
    };

    println!("{}", &input[start..stop]);
}
