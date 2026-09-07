# ERRORS.md — Phase C error-surface table

Mechanically derived from the complete C source (`c_src/src/hello.c`,
`c_src/include/hello.h`). Greps performed over all of `c_src/`:

```
grep -rnE 'return|assert|RETURN_ERROR|NULL|errno|if *\(|switch|#ifdef|<|>|==|!=' c_src/src c_src/include
```

Findings: the only executable statements in the entire library are

```c
int helloworld() {
    printf("Hello World!\n");
    return 0;
}
```

There is:

* **no** parameter (so no null check, no range check, no enum, no length),
* **no** `if` / `switch` / `assert` / `errno` inspection,
* **no** error-return macro, no `-1`, no `NULL` return,
* **exactly one** `return` statement, with the constant `0`,
* the return value of `printf` is **discarded** — i.e. the C code deliberately
  ignores I/O failure and still reports success.

The error surface therefore consists of the *environmental* failure modes that
this function can be subjected to, plus the FFI-boundary edge cases a C caller
can legally create against a `()`-prototyped (unprototyped, K&R) function.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `helloworld` | Write failure: `stdout` redirected to `/dev/full` (every write returns `ENOSPC`) | `printf` fails, its return value is discarded → function still returns `0`; nothing readable is produced. Rust must also return `0` and must NOT panic/abort. |
| 2 | `helloworld` | `stdout`'s underlying fd 1 is **closed** before the call (`close(1)`) | `printf` fails (`EBADF`), value discarded → returns `0`, no output, no crash. |
| 3 | `helloworld` | `stdout` redirected to a **read-only** fd (opened `O_RDONLY`), so writes fail with `EBADF` | returns `0`, no output, no crash. |
| 4 | `helloworld` | `stdout` is a **pipe whose read end is closed** → `SIGPIPE`/`EPIPE` on flush | identical behaviour between C and Rust (both go through the same libc `stdout`); with `SIGPIPE` ignored, `printf`/`fflush` fails, `helloworld` returns `0`. |
| 5 | `helloworld` | `stdout` set **unbuffered** (`setvbuf(stdout, NULL, _IONBF, 0)`) then write target fails | returns `0` (failure surfaces immediately inside `printf` rather than at flush); still `0`. |
| 6 | `helloworld` | Called through a **wrongly-typed function pointer with extra arguments** — legal in C because `int helloworld()` has no prototype: `((int(*)(int,int,int))helloworld)(1,2,3)` | Extra SysV register arguments are ignored; prints `Hello World!\n`, returns `0`. Rust `extern "C" fn()` must behave identically. |
| 7 | `helloworld` | Called through a function pointer declared to return a **wider/narrower type** than `int` (`long`/`short` reinterpretation of the return register) | Low 32 bits of the return register are `0`; C and Rust must agree bit-for-bit on the returned `int` (`0`, not a garbage sentinel). |
| 8 | `helloworld` | Called with a **NULL `this`/no arguments at all** — i.e. the degenerate "no input" case, which is the only input the prototype admits | returns `0`, prints once. (Baseline: there is no invalid *argument* to construct, so the only in-range/out-of-range distinction is "called" vs "not called".) |
| 9 | `helloworld` | Invoked a very large number of times in a row (exhaustion / state-corruption probe: 100 000 calls) | returns `0` every time; output is exactly 100 000 repetitions; no internal state, no leak, no drift. Rust must not accumulate state. |
| 10 | `helloworld` | Called concurrently from **many threads** while `stdout` is shared (locking-error probe: glibc `printf` takes the FILE lock) | No interleaving *within* a line, no crash, all calls return `0`; total byte count is exact. Rust must use the same locked libc path, not an unlocked writer. |
| 11 | `helloworld` | `stdout` `FILE` re-pointed with `freopen` to a new file mid-lifetime | Output goes to the *new* target; returns `0`. Confirms Rust resolves `stdout` dynamically through libc rather than caching a handle. |

## Status

| # | test | result |
|---|------|--------|
| 1 | `err_01_dev_full` | [x] pass |
| 2 | `err_02_closed_fd1` | [x] pass |
| 3 | `err_03_readonly_fd` | [x] pass |
| 4 | `err_04_broken_pipe` | [x] pass |
| 5 | `err_05_unbuffered_failing_target` | [x] pass |
| 6 | `err_06_extra_args_unprototyped` | [x] pass |
| 7 | `err_07_return_register_width` | [x] pass |
| 8 | `err_08_baseline_no_args` | [x] pass |
| 9 | `err_09_many_calls` | [x] pass |
| 10 | `err_10_threaded` | [x] pass |
| 11 | `err_11_freopen` | [x] pass |
