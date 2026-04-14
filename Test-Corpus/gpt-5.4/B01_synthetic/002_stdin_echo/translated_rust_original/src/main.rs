use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut line = String::new();
    let mut handle = stdin.lock();

    loop {
        line.clear();
        match handle.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let _ = stdout.write_all(line.as_bytes());
            }
            Err(_) => break,
        }
    }
}
