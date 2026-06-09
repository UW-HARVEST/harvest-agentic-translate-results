// Translated from c_src/src/lib.c
//
// The original C code is a library (no main function). It only defines
// `read_side_info` and a static helper `get_bits`. To make this a buildable
// executable as required, we expose the same functions in a module and
// provide a main that performs no I/O — matching the C library behavior of
// producing no output for any input.

mod mp3lib;

#[allow(unused_imports)]
use mp3lib::{read_side_info, BsT, L3GrInfoT};

fn main() {
    // The original C source has no main and produces no output.
    // Reference the library symbols so they are not stripped.
    let _ = read_side_info as fn(&mut BsT<'_>, &mut [L3GrInfoT], &[u8]) -> i32;
}
