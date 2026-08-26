// The original C source (c_src/src/lib.c) is a library with no `main`
// function and performs no I/O. This executable mirrors that behavior:
// it reads no input and produces no output.

#[allow(dead_code)]
mod lib_impl {
    include!("lib.rs");
}

fn main() {}
