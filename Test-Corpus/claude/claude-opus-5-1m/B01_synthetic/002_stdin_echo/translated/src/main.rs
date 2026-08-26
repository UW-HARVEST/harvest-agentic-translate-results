// Translation of c_src/src/main.c
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

use std::io::{self, BufReader, Read, Write};

/// Size of the C `char text[128]` buffer.
const BUF_SIZE: usize = 128;

/// Emulate `fgets(buf, BUF_SIZE, stdin)`.
///
/// Reads at most `BUF_SIZE - 1` (127) bytes, stopping early after a newline
/// (the newline is kept in the buffer). Returns `None` when nothing at all was
/// read (EOF or error), which corresponds to `fgets` returning NULL.
/// The returned Vec holds the raw bytes read (the C code's implicit trailing
/// NUL terminator is handled by the caller).
fn c_fgets<R: Read>(reader: &mut R, out: &mut Vec<u8>) -> Option<()> {
    out.clear();
    let mut byte = [0u8; 1];
    while out.len() < BUF_SIZE - 1 {
        match reader.read(&mut byte) {
            Ok(0) => break,                 // EOF
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,                // read error -> fgets returns NULL
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(())
    }
}

fn main() {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let mut text: Vec<u8> = Vec::with_capacity(BUF_SIZE);

    while c_fgets(&mut reader, &mut text).is_some() {
        // fputs writes the C string, i.e. bytes up to the first NUL byte.
        let end = text
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(text.len());
        if writer.write_all(&text[..end]).is_err() {
            break;
        }
        // Keep interactive echo behaviour (C stdout is line buffered on a tty).
        let _ = writer.flush();
    }

    let _ = writer.flush();
}
