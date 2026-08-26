//! `driver` executable: the translated `int main(void)` of `c_src/src/main.c`.

#[macro_use]
mod cio;
mod driver;
mod hashmap;
mod tree;

fn main() {
    let code = unsafe { driver::run_main() };
    // `return 0` from C's `main` reaches `exit()`, which flushes every stream.
    cio::flush_all();
    std::process::exit(code);
}
