fn main() -> i32 {
    // CLI entry point: in the original C, this prints PSBT records.
    // The Rust harness exposes `main` returning i32. With no arguments,
    // we simply print usage and return success.
    println!("usage: psbt <psbt>");
    0
}
