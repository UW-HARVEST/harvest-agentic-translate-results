use std::io::{self, Write};

fn helloworld() -> i32 {
    print!("Hello World!\n");
    io::stdout().flush().expect("failed to flush stdout");
    0
}

fn main() {
    std::process::exit(helloworld());
}
