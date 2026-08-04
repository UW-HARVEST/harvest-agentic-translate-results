use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    
    for line in stdin.lock().lines() {
        if let Ok(text) = line {
            if writeln!(stdout_lock, "{}", text).is_err() {
                break;
            }
        } else {
            break;
        }
    }
}
