# ERRORS.md — error-surface table (Phase A.2)

## Mechanical derivation

Every non-comment line of C source, in full (there is no other C code):

```c
/* c_src/src/sillymain.h */               /* c_src/src/sillymain.c */
#ifndef SILLYMAIN_H_                      #include <stdio.h>
#define SILLYMAIN_H_                      #include "sillymain.h"
int helloworld();                         int helloworld() {
#endif //SILLYMAIN_H_                         printf("Hello World!\n");
                                              return 0;
/* c_src/src/main.c */                    }
#include "sillymain.h"
int main() {
    return helloworld();
}
```

Grep for every rejection construct the task lists:

| construct grepped | hits in `c_src/` |
|---|---|
| `RETURN_ERROR` / error macros / error enums | 0 |
| `return -1` / negative or sentinel returns | 0 (`return 0;` and `return helloworld();` only) |
| `return NULL` | 0 |
| `assert` | 0 |
| explicit range / bounds check (`if`, `switch`, `?:`, `goto`) | 0 — the sources contain **no** conditional or branch of any kind |
| null-pointer check | 0 (no function takes a pointer; no pointer variable exists) |
| min/max constant, magic limit | 0 |
| `errno` inspection | 0 |
| `#ifdef` / `#if` build switch | 0 apart from the `SILLYMAIN_H_` include guard |
| function parameters that could be invalid | 0 — both functions take no parameters |

So **the C library rejects nothing and cannot fail**: there is no input to
validate. That is a finding, not an excuse to skip Phase C — the rows below are
the failure modes the C code *does* have (they come from the discarded `printf`
result and from what an FFI caller can legally throw across the boundary at an
unprototyped, parameterless C function), plus the generic boundaries required by
the task. Every row is a real, executable differential test.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `helloworld` | `printf` fails with `ENOSPC`: fd 1 redirected to `/dev/full`. The result of `printf` is discarded by `sillymain.c`. | returns `0`; no abort, no `errno` propagation, nothing on fd 1 | `err01_helloworld_enospc_dev_full` | [x] |
| 2 | `helloworld` | `printf` fails with `EBADF`: fd 1 **closed** before the call, so there is no open file description. | returns `0` | `err02_helloworld_ebadf_closed_fd1` | [x] |
| 3 | `helloworld` | `printf` fails with `EBADF`: fd 1 redirected to a fd opened **`O_RDONLY`** (description not writable). | returns `0` | `err03_helloworld_ebadf_readonly_fd1` | [x] |
| 4 | `helloworld` | `printf` fails with `EPIPE`: fd 1 is a pipe with no reader, `SIGPIPE` set to `SIG_IGN`. | returns `0`; process survives | `err04_helloworld_epipe_broken_pipe` | [x] |
| 5 | `helloworld` | Retry **after** a failed call: glibc leaves the stream's error indicator set, so every later call fails too. | still returns `0`, every time | `err05_helloworld_repeat_after_failure` | [x] |
| 6 | `main` | Rows 1–4 through `main`, whose value **is** `helloworld()`'s value — a non-zero propagation would change the program's exit status. | returns `0`, exit status `0` | `err06_main_write_failures_propagate_zero` | [x] |
| 7 | `helloworld` | Extra arguments across the FFI boundary. `int helloworld();` is an *unprototyped* (K&R) declarator, so a C caller may legally pass anything; the callee must ignore it. Exercised through `fn(int)`, `fn(int,int)`, `fn(size_t)`, `fn(int×6)` (all integer registers) and a mixed int/pointer/`double`/`float`/stack signature, with randomized values. | returns `0`, arguments ignored, output unchanged | `err07_extra_args_ignored` | [x] |
| 8 | `main` | The conventional `int main(int, char **)` shape, including `argc = -1`, `argc = INT_MIN`, `argc = INT_MAX` and `argv = NULL` / `argv = (char **)-1` — an out-of-range count and a null pointer, both of which the parameterless C `main()` must ignore. | returns `0` | `err08_main_extra_args_and_null_argv` | [x] |
| 9 | `helloworld`, `main` | Null and garbage pointer arguments (`NULL`, `0x1`, `0x8`, `0xdead_beef`, `usize::MAX`, a non-canonical address, plus randomized values): none may ever be dereferenced. | returns `0`, no segfault | `err09_null_and_garbage_pointer_args` | [x] |
| 10 | `helloworld`, `main` | Out-of-range **enum** value across the FFI boundary. The C API declares no enum, so the generic case is covered: an `int` with no valid variant — `INT_MIN`, `INT_MIN+1`, `-2`, `-1`, `0`, `1`, `2`, `255`, `256`, `INT_MAX-1`, `INT_MAX`, `0xdeadbeef` reinterpreted. | returns `0`, value ignored | `err10_out_of_range_enum_values` | [x] |
| 11 | `helloworld`, `main` | Zero and oversized length arguments: `0`, `1`, `SIZE_MAX`, `SIZE_MAX-1`, `SSIZE_MAX`, `SSIZE_MAX+1` (i.e. `isize::MIN` reinterpreted), `1<<62`, `UINT32_MAX`. | returns `0`, ignored | `err11_zero_and_oversized_lengths` | [x] |
| 12 | `helloworld`, `main` | Return value read through a `long`-returning signature, i.e. the full 64-bit return register. | the C `int` half is `0` for both (the upper half is ABI-undefined for a function returning `int` and is deliberately **not** compared) | `err12_return_value_is_c_int_zero` | [x] |
| 13 | `helloworld` | Called concurrently from 2–6 threads while the stream is in an error state. C stdio locks `stdout`. | every call returns `0`; no partial lines | `err13_concurrent_calls_no_partial_lines` | [x] |
| 14 | `helloworld`, `main` | The failing destinations again after `dlclose` + `dlopen` (fresh relocations). | returns `0`, identical bytes | `err14_reload_library` | [x] |
| 15 | *(symbol surface)* | `dlsym` for a name the C library does **not** define (`helloworld_`, `_helloworld`, `HelloWorld`, `hello_world`, `sillymain`, `driver_main`). | lookup fails for both libraries — the Rust `.so` must not export look-alike or stub symbols either | `err15_absent_symbols_are_absent_in_both` | [x] |
| 16 | `helloworld` | Broken pipe with `SIGPIPE` at its **default** disposition (what a C process starts with): the write raises the signal. | process is **killed by signal 13**, no call returns | `err16_broken_pipe_default_sigpipe_kills_process` | [x] |
| 17 | whole program | `driver` run with stdout on a pipe that has no reader. | process is **killed by signal 13** (status `-13`) | `err17_program_broken_pipe_status_matches` | [x] |
| 18 | `helloworld`, `main` | `printf` **itself** returns a negative value, rather than failing only at the later flush. Requires an unbuffered / line-buffered / too-small-buffer stream on a rejecting fd, so every row 1–6 is run in four buffering modes: default, `_IONBF`, `_IOLBF`, and `_IOFBF` with an 8-byte buffer (shorter than the 13-byte line). | returns `0` — the discarded `printf` result must never become the return value | covered by `err01`–`err06` (buffering-mode sub-cases) | [x] |

All 18 rows pass, in every configuration verified by `./verify.sh` (see
`CONFIGS.md` for the configuration matrix).

## Findings: what this table caught

**1. Row 18 was a blind spot until mutation testing exposed it.** With the
default buffering of a file, a pipe or a character device, `printf` only fills
the stream buffer and *succeeds*; the `write(2)` — and therefore the failure —
happens later, inside `fflush`/`exit`. A mutant that returned `-1` when `printf`
failed therefore passed rows 1–6 unnoticed. Forcing `_IONBF` / `_IOLBF` /
`_IOFBF(8)` makes `printf` do the write itself and return the error, which is
what actually distinguishes "discards the result" (C) from "propagates it".
`negative_control.sh`'s `propagates_error` mutant is the regression guard.

**2. Rows 16 and 17 caught a real divergence in the translation.** Rust's runtime
installs `SIG_IGN` for `SIGPIPE` before calling `main`, which a C program does
not. With stdout on a pipe that has no reader, `c_src`'s `driver` is killed by
`SIGPIPE` (status 13) while the unmodified Rust binary ignored the `EPIPE` and
exited 0 — a different exit status for the same input. `src/main.rs` now restores
the default disposition before doing anything else, and both programs die
identically. (The `cdylib` deliberately does **not** touch signal dispositions,
just like the C `.so`, which is what row 16 pins down.)
