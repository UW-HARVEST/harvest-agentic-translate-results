use std::io::{self, Write};

fn main() {
    io::stdout().write_all(b"Hello World!\n").unwrap();
}
