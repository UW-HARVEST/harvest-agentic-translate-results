//! Terminating the process the way the C program does when it traps.
//!
//! `stb_perlin_noise3_wrap_nonpow2` has two trapping paths for inputs the C
//! code never validates: `INT_MIN % -1` (the `idiv` gcc emits raises `#DE` ->
//! `SIGFPE`) and table indices that leave the program's mapped data pages
//! (`SIGSEGV`).
//!
//! The distinction matters for a differential test: a process *killed by* a
//! signal is not the same wait status as a process that *exits with code*
//! 128+signum. `waitpid` reports `WIFSIGNALED` for the former and `WIFEXITED`
//! for the latter, so `std::process::exit(136)` would not match a C program
//! that died of `SIGFPE`.
//!
//! Rust's runtime installs its own `SIGSEGV`/`SIGBUS` handler (for stack
//! overflow reporting), so the disposition is reset to `SIG_DFL` before
//! raising.

pub const SIGFPE: i32 = 8;
pub const SIGSEGV: i32 = 11;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
}

/// Die from `sig` with the same wait status the C program would produce.
/// Nothing has been written to stdout at this point, so stdout stays empty.
pub fn die(sig: i32) -> ! {
    unsafe {
        // SIG_DFL == 0
        signal(sig, 0);
        raise(sig);
    }
    // `raise` with the default disposition for SIGFPE/SIGSEGV does not return.
    std::process::abort()
}
