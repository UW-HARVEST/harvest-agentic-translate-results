// The original C source (c_src/src/lib.c) defines only a library function
// `hdr_bitrate` and no `main`. There is no defined I/O behavior to mirror,
// so this binary simply reads bytes from stdin and prints the result of
// `hdr_bitrate` over them, exposing the translated function as an executable.

use std::io::Read;

mod hdr;
use crate::hdr::hdr_bitrate;

fn main() {
    let mut buf = Vec::new();
    if std::io::stdin().read_to_end(&mut buf).is_err() {
        std::process::exit(1);
    }
    if buf.len() < 3 {
        std::process::exit(1);
    }
    let rate = hdr_bitrate(&buf);
    print!("{}\n", rate);
}
