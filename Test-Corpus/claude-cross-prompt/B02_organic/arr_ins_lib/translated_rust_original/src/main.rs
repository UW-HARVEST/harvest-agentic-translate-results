// Translation of c_src/src/lib.c
//
// The original C code is built as a shared library (per CMakeLists.txt).
// It contains no `main`, no stdin reads, and no calls that produce output
// (the only `sprintf` writes to a static buffer in an unused helper).
// The only externally-visible symbol is `arr_ins(int num)`.
//
// To package this as an executable while matching the (empty) output of
// the C code, we expose the same function in safe Rust and provide a
// `main` that performs no I/O — matching the C library's behavior of
// producing no output unless an external caller invokes `arr_ins`.

mod translated;

#[allow(dead_code)]
fn arr_ins(num: i32) {
    translated::arr_ins(num);
}

fn main() {
    // The C library has no `main`, no stdin parsing, and prints nothing.
    // Match that behavior exactly: produce no output.
}
