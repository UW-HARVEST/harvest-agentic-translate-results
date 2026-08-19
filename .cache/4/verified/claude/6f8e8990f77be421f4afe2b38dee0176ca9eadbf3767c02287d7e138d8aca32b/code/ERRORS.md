# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical grep of the whole C library

```sh
grep -nE 'return|assert|NULL|errno|if *\(|switch|else|#if|#ifdef|#define|enum|EXIT|abort|exit|-1' \
     c_src/src/*.c c_src/include/*.h | grep -v ': *//'
```

Output (the only non-comment hits in the entire library):

```text
c_src/include/hello.h:24:#ifndef HELLO_H_      <- include guard, not a config knob
c_src/include/hello.h:25:#define HELLO_H_      <- include guard, not a config knob
c_src/src/hello.c:30:    return 0;            <- the single, unconditional return
```

The complete body of the only function in the library is:

```c
int helloworld() {
    printf("Hello World!\n");
    return 0;
}
```

Therefore, mechanically:

* error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`, error enums): **0**
* `assert` / `abort` / `exit`: **0**
* explicit range checks, null checks, `if`/`switch`/`?:` branches: **0**
* `min`/`max` constants, magic limits, `#define`d tunables: **0**
* function parameters (hence: nothing to validate, nothing to be out of range): **0**
* enum-typed parameters (hence no out-of-range-enum variant to smuggle in): **0**

The library has **no rejection path**. `helloworld` is total: every call returns
`0`. That is a real, testable claim about the error surface, and the rows below
are the exhaustive set of ways an attacker/caller can try to make it fail —
each one must produce the *same* non-error result from C and Rust.

## Error-surface table

Every row is a differential test in `tests/differential.rs`. "expected C result"
is what the C code above actually does, established by reading it and confirmed
by running the C `.so`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| E1 | `helloworld` | `printf` cannot write: `stdout`'s fd is closed (`close(1)`), so the underlying `write(2)` fails with `EBADF`. The C ignores `printf`'s return value. | returns `0`; no crash, no `abort`, no error signalled to the caller |
| E2 | `helloworld` | `printf` cannot write: `stdout`'s fd is redirected to a **read-only** descriptor (`open(path, O_RDONLY)` `dup2`'d onto fd 1) → `write(2)` fails with `EBADF`. | returns `0`; caller sees no error |
| E3 | `helloworld` | `printf` cannot write: `stdout` redirected to a full device — `write(2)` fails with `ENOSPC` (done deterministically by pointing fd 1 at `/dev/full`, in each of the three buffering modes). | returns `0`; caller sees no error |
| E4 | `helloworld` | `printf` writes into a **closed pipe** → `write(2)` fails `EPIPE` (`SIGPIPE` is `SIG_IGN` for the whole process). | returns `0`; caller sees no error |
| E5 | `helloworld` | Called through the K&R/unprototyped declaration `int helloworld();` with **extra unexpected arguments** (1–6 garbage `int`s). C accepts this: the callee never touches the argument registers. | returns `0`, prints exactly `Hello World!\n`; the extra arguments are ignored |
| E6 | `helloworld` | Same as E5 but with arbitrary **out-of-range "enum-like" `int`s** (`INT_MIN`, `-1`, `0x7FFFFFFF`, values that name no valid variant of anything) and with **pointer-shaped garbage** (`NULL`, `0xDEADBEEF`, a dangling pointer) in the argument registers. There is no parameter to validate, so no value is rejected. | returns `0`, prints exactly `Hello World!\n`; nothing is dereferenced, no crash |
| E7 | `helloworld` | Same as E5 but with **floating-point arguments** in `xmm0..xmm3` and `%al` set non-zero (what a varargs call site would do). | returns `0`, prints exactly `Hello World!\n` |
| E8 | `helloworld` | Return value inspected as a **64-bit** quantity (`long` return signature) to catch a translation that leaves garbage in the upper 32 bits of `%rax`. | low 32 bits are `0` (C emits `mov $0x0,%eax`, which zero-extends, so all 64 bits read `0`) |
| E9 | `helloworld` | Called with `stdout` in a hostile buffering state: `setvbuf(stdout, buf, _IOFBF, 1)` (1-byte buffer → a partial-write path inside libc). | returns `0`; the 13 emitted bytes are still exactly `Hello World!\n` |
| E10 | `helloworld` | `stdout`'s `FILE*` is left in its **error state** (`ferror(stdout)` already set by a previously failed write) before the call. The C never checks or clears it. | returns `0`; whether the new bytes appear is decided entirely by libc, identically for both libraries |
| E11 | `helloworld` | Symbol resolved but the library is `dlclose`d and re-`dlopen`ed repeatedly (fresh handle, no init/teardown hooks in C). Also: never-initialised state — the very first call after `dlopen`. | every call returns `0` and prints the same 13 bytes; no per-library state exists |
| E12 | `helloworld` | Called concurrently from many threads (no locking in the C; `printf`/`puts` is the only shared resource), both onto a working `stdout` and onto a failing one (`/dev/full`). | every call returns `0`; the byte stream is a permutation of `Hello World!\n` lines, never a torn line |
| E13 | *(loader surface)* | `dlsym` for a name the library does not define (`helloworld_v2`, `hello_world`, `HelloWorld`, `helloworld2`, `hello`, `""`). | every one fails on the C `.so`; the Rust `.so` must reject exactly the same set, and resolve `helloworld` exactly like the C `.so` |

### Boundaries that do not exist in this API (documented so they aren't silently skipped)

| boundary | why it is not a row |
|----------|---------------------|
| null pointer argument | `helloworld` takes no parameters. Covered anyway, as far as it can be, by **E6** (a `NULL` in `%rdi`). |
| zero / oversized length | no length, size, or buffer parameter exists anywhere in the library. Covered as far as possible by **E5**/**E6**. |
| value one step past a documented valid range | no parameter and no documented range exists. Covered as far as possible by **E6** (`INT_MIN`, `INT_MAX`, `-1`). |
| out-of-range enum across FFI | the library declares no enum and takes no enum parameter. Covered as far as possible by **E6**. |
| error code / sentinel mismatch | the only possible return is the constant `0`; every row above asserts C and Rust return the *same* value, not merely that "both didn't crash". |

## Phase C status — every row has a passing differential test

Tests live in `tests/phase_c.rs` (single `#[test]`, one runner row per table
row). Each row compares a 4-tuple of observables between the two `.so`s —
`(return value as i64, errno after the call, ferror(stdout), bytes emitted)` —
so a divergence in *which* error occurred is a failure, not just "both failed".

| row | test function | asserted C outcome (Rust must equal it) | status |
|-----|---------------|------------------------------------------|--------|
| E1 | `e1_closed_stdout_fd` | `ret=0`, `errno=EBADF (9)`, `ferror` set, 0 bytes | [x] |
| E2 | `e2_readonly_stdout_fd` | `ret=0`, `errno=EBADF (9)`, file still empty | [x] |
| E3 | `e3_device_full_enospc` | `ret=0`, `errno=ENOSPC (28)`, `ferror` set (×3 buffering modes) | [x] |
| E4 | `e4_closed_pipe_epipe` | `ret=0`, `errno=EPIPE (32)`, `ferror` set | [x] |
| E5 | `e5_extra_garbage_int_arguments` | `ret=0`, 13 bytes, arity 1..6 × randomized args | [x] |
| E6 | `e6_out_of_range_and_pointer_garbage` | `ret=0`, 13 bytes, for `INT_MIN/-1/0/1/INT_MAX/…` and `NULL`/`0xDEADBEEF`/`usize::MAX` | [x] |
| E7 | `e7_float_arguments_and_al_set` | `ret=0`, 13 bytes, 8 doubles in `xmm0..7` and a true variadic call site | [x] |
| E8 | `e8_return_upper_bits` | all 64 bits of `%rax` are `0` | [x] |
| E9 | `e9_one_byte_buffer` | `ret=0`, 13 bytes with `setvbuf` sizes 1, 2, 3 | [x] |
| E10 | `e10_sticky_stdout_error_flag` | `ret=0`; identical bytes/flags with `ferror(stdout)` pre-set | [x] |
| E11 | `e11_no_state_across_load_unload` | `ret=0` and 13·n bytes on every fresh `dlopen` (3..12 cycles) | [x] |
| E12 | `e12_concurrent_calls` | every `ret=0`; no torn lines; also all-`0` under `ENOSPC` | [x] |
| E13 | `e13_unknown_symbols_rejected` | 6 bogus names rejected by both; `helloworld` resolved by both | [x] |

Last run: **13 rows run, 0 failed** (`cargo test --test phase_c`), under every
feature combination and both profiles.

### These rows are not vacuous

`./negative_control.sh` builds five deliberately wrong "translations" and checks
that the suite rejects each. The decisive one is `m5`, which propagates the I/O
error the real C code ignores (`if (printf(...) < 0) return -1;`):

```text
mutant m5: phase_b exit=0  rows_failed=0      <- happy-path tests see nothing
           phase_c exit=101 rows_failed=5     <- E1, E2, E3, E4, E12 catch it
           phase_d exit=0  rows_failed=0
```

The other mutants (missing `\n`, `return 1`, renamed symbol, hidden state) are
caught by Phases B/C/D as well: `NEGATIVE CONTROL PASSED: every mutant was
rejected`.
