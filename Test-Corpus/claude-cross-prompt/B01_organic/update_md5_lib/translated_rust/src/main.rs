// The original C code (c_src/) defines a SHARED library only — there is no main()
// function. To satisfy the "executable" requirement while preserving byte-identical
// behavior to the (empty) C entry point, this binary performs no I/O and exits.
//
// The translated library is exposed via src/lib.rs.

fn main() {
    let _ = tflac::Tflac::default();
}
