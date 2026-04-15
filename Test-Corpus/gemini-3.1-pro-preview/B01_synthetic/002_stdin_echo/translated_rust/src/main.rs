use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut stdout = io::stdout();
    let mut buffer = Vec::new();

    while let Ok(bytes) = stdin_lock.read_until(b'\n', &mut buffer) {
        if bytes == 0 {
            break;
        }
        let _ = stdout.write_all(&buffer);
        let _ = stdout.flush();
        buffer.clear();
    }
}
