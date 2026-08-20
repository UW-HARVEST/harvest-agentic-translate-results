// The program: same behavior as the executable built by c_src/CMakeLists.txt.

#[path = "logic.rs"]
mod logic;

fn main() {
    // `int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }`
    //
    // `true`: returning 0 from the C `main` lands in `__libc_start_main`, which
    // calls `exit`, whose `_IO_cleanup` seeks stdin back over the bytes glibc
    // buffered but never consumed.  The program reproduces that side effect.
    let rc = logic::program_main(true);
    std::process::exit(rc);
}
