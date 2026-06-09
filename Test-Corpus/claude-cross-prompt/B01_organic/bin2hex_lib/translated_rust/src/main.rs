// The original C package (c_src/) is a library: it exposes only the function
// `bin2hex` and has no `main`, no stdin reading, and no printf calls.
// Therefore the executable has nothing to print and produces empty output —
// which is byte-identical to what a C program with no I/O would produce.

fn main() {}
