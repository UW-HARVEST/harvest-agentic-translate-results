
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();
    let mut text = String::with_capacity(128);
    // Emulate: while (fgets(text, 128, stdin)) { fputs(text, stdout); }
    // fgets reads at most n-1 chars (127), stops at newline (inclusive) or EOF.
    // We read line-by-line, splitting any line longer than 127 bytes into chunks
    // so each write mirrors a single fgets call.
    loop {
        text.clear();
        match stdin_lock.read_line(&mut text) {
            Ok(0) => break, // EOF with no data -> fgets returns NULL
            Ok(_) => {
                let bytes = text.as_bytes();
                let mut offset = 0usize;
                while offset < bytes.len() {
                    let remaining = bytes.len() - offset;
                    let chunk = remaining.min(127);
                    if stdout_lock.write_all(&bytes[offset..offset + chunk]).is_err() {
                        return 0;
                    }
                    offset += chunk;
                }
            }
            Err(_) => break,
        }
    }
    let _ = stdout_lock.flush();
    0
}