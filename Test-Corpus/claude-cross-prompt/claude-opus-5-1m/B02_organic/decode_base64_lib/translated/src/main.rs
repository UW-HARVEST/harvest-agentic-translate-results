// Minimal driver around the translated `decode_base64` library function.
//
// The original C source in c_src/ is a shared library exposing only
// `decode_base64`; there is no `main`. To satisfy the "executable" build
// requirement, this driver reads input from stdin (matching C's fgets-style
// line-oriented reading), decodes it, and writes the decoded bytes to stdout
// up to the first NUL byte (mirroring how a C caller would treat the
// returned NUL-terminated buffer).

use std::io::{Read, Write};

use translated_rust::decode_base64;

fn main() {
    let mut input = Vec::new();
    if std::io::stdin().read_to_end(&mut input).is_err() {
        std::process::exit(1);
    }

    // Strip a single trailing '\n' (and possible '\r') if present, the way
    // a typical C harness using fgets+strip-newline would.
    if input.last() == Some(&b'\n') {
        input.pop();
        if input.last() == Some(&b'\r') {
            input.pop();
        }
    }

    match decode_base64(&input) {
        Some(buf) => {
            // Write bytes up to the first NUL terminator, matching the way
            // a C caller would treat the returned NUL-terminated buffer.
            let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(&buf[..end]);
            let _ = out.flush();
        }
        None => {
            // Empty/NULL input -> produce no output, matching C returning NULL.
        }
    }
}
