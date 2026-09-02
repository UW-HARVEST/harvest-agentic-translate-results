// Rust translation of c_src/src/main.c
//
// Original C:
//     int main() {
//         char text[128];
//         while (fgets(text, 128, stdin)) {
//             fputs(text, stdout);
//         }
//         return 0;
//     }
//
// interactive echo; ignores arguments, copies stdin to stdout

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

/// Buffer size passed to fgets in the C original (`char text[128]`).
const BUF_SIZE: usize = 128;

/// Emulate `fgets(buf, BUF_SIZE, stdin)`.
///
/// Reads at most `BUF_SIZE - 1` bytes, stopping early after a newline (which is
/// retained). Returns `None` when no bytes could be read before EOF, matching
/// fgets returning NULL. The returned Vec holds the raw bytes read, without the
/// implicit NUL terminator that C appends.
fn fgets<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut out: Vec<u8> = Vec::with_capacity(BUF_SIZE);
    let limit = BUF_SIZE - 1;

    while out.len() < limit {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }

    if out.is_empty() {
        // Nothing read: EOF (or error) with an empty buffer -> fgets returns NULL.
        Ok(None)
    } else {
        Ok(Some(out))
    }
}

fn restore_default_sigpipe() {
    #[cfg(unix)]
    {
        const SIGPIPE: i32 = 13;
        const SIG_DFL: usize = 0;
        extern "C" {
            fn signal(signum: i32, handler: usize) -> usize;
        }
        // SAFETY: installing the default disposition for SIGPIPE is a simple,
        // well-defined libc call with no memory involved.
        unsafe {
            signal(SIGPIPE, SIG_DFL);
        }
    }
}

fn main() -> ExitCode {
    // Rust's runtime masks SIGPIPE, whereas a C program keeps the default
    // disposition and is killed by SIGPIPE when its stdout reader goes away
    // (e.g. `driver | head -1`). Restore the default so termination behavior
    // matches the C original.
    restore_default_sigpipe();

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    loop {
        match fgets(&mut reader) {
            Ok(Some(text)) => {
                // fputs writes the C string, i.e. bytes up to the first NUL.
                // A NUL byte in the input therefore truncates what is echoed
                // for that chunk; reproduce that behavior exactly.
                let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
                if writer.write_all(&text[..end]).is_err() {
                    // fputs failure is ignored by the C original.
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => break, // fgets returns NULL on read error as well
        }
    }

    let _ = writer.flush();
    ExitCode::from(0)
}
