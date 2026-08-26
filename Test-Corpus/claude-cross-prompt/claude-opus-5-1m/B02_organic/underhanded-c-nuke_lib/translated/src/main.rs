// Translation of the C library at c_src/ to Rust.
//
// The original C source builds as a shared library (libCSpectralContrast)
// and contains no main() function: only the public functions `match` and
// `spectral_contrast`, plus a few static helpers. There is no stdin reading
// and no printf output in the C code. To preserve byte-identical output for
// the same inputs, this executable reads any input that may be supplied and
// produces no output, mirroring the (lack of) I/O behavior of the C library.

mod match_mod;
mod spectral_contrast;

use std::io::Read;

fn main() {
    // Drain stdin (mirroring C's behavior of doing nothing with input).
    let mut buf = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut buf);
    // No output, matching the C library which produces none.
}
