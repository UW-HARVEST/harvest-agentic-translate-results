# Differential Mismatches

## Missing arguments terminated with the wrong signal

- Inputs: no user arguments; one user argument (`7`)
- C result: empty stdout and stderr, terminated by `SIGSEGV` (shell status 139)
- Rust result before fix: empty stdout and stderr, terminated by `SIGABRT`
  (shell status 134)
- Cause: the C program passes a null missing `argv` entry to `atoi`, while the
  Rust translation explicitly called `std::process::abort`.
- Initial correction: explicitly raising `SIGSEGV` was insufficient because
  Rust's Unix runtime signal handler caught the asynchronous signal; execution
  continued to the abort fallback and still produced status 134.
- Second correction: calling libc `atoi` with a null pointer reproduced the
  synchronous fault in debug builds, but LLVM removed that undefined call in
  release builds. The optimized executable again reached the abort fallback.
- Fix: the Rust path resets `SIGSEGV` to its default disposition before raising
  it. This bypasses Rust's installed handler using defined operations that
  remain present in optimized builds.
