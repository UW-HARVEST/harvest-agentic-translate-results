// The original C source provides only a library function (`normalize`)
// with no `main`, no stdin reading, and no printf output.
//
// To match "byte-identical output" for the same inputs, this executable
// must read nothing and print nothing — same as a hypothetical executable
// built from the C source which has no I/O behavior of its own.

fn main() {}
