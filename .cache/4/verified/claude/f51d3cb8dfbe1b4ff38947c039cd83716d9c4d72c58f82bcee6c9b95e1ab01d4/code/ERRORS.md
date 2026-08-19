# ERRORS.md — error-surface table (Phase A / Phase C)

## Mechanical derivation

The complete C source is 5 non-comment, non-blank lines:

```c
#include <stdio.h>

int main() {
    printf("Hello World!\n");
    return 0;
}
```

Grep of `c_src/src/main.c` for every rejection construct — counts are literal
`grep -c` results, not judgement calls:

| construct | matches | construct | matches |
|---|---|---|---|
| `RETURN_ERROR` | 0 | `if (` / `if(` | 0 |
| `return -` | 0 | `switch` | 0 |
| `return NULL` | 0 | `goto` | 0 |
| `assert` | 0 | `for` / `while` | 0 |
| `errno` | 0 | `exit(` | 0 |
| `abort` | 0 | `MAX` / `MIN` | 0 |
| `ERR` | 0 | `<` `>` `<=` `>=` | 0 |
| `fprintf` / `stderr` | 0 | `malloc`/`free`/`calloc` | 0 |
| `argc` / `argv` | 0 | `scanf`/`fgets`/`getchar`/`read` | 0 |
| `#if` / `#ifdef` | 0 | `signal`/`setvbuf`/`fflush` | 0 |

**There are no explicit error returns, no validation, no range checks, no null
checks, no asserts, no enums, and no min/max constants.** The function takes no
parameters (`int main()` — no `argc`/`argv`), reads no input, and returns the
constant `0` on every path. There is exactly one path.

Therefore the error surface is not "unknown" — it is *provably* empty of
explicit rejections. What remains is the surface the C code exposes
**implicitly**: `printf` can fail, and its return value is discarded. Those are
the rows below, plus the generic FFI/process boundaries Phase C requires.

## Error-surface table

`main` is the only entry point (both as the executable entry and as the `.so`
export). "Result" is what the C actually does, measured — not assumed.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|---|---|---|---|
| E1 | `main` | `printf` fails with `EPIPE`: stdout is a pipe whose read end is closed. Process-level, `SIGPIPE` at default disposition. | **Process killed by `SIGPIPE` (signal 13, wait status 141)**; 0 bytes of output. Return value never observed. | `proc_differential.rs::e1_sigpipe_killed` |
| E2 | `main` | `printf`/flush fails with `EPIPE` while `SIGPIPE` is **ignored** (the FFI case: host process has `SIGPIPE=SIG_IGN`). | `printf`'s error is discarded → **returns 0**; 0 bytes written. No crash. | `ffi_error_epipe.rs::e2_epipe_ignored_returns_zero` |
| E3 | `main` | `printf` fails with `ENOSPC`: stdout is `/dev/full`. | Error discarded → **exit status 0**, 0 bytes of output, nothing on stderr. | `proc_differential.rs::e3_dev_full`, `ffi_error_devfull.rs::e3_dev_full_ffi` |
| E4 | `main` | `printf` fails with `EBADF`: fd 1 is **closed** before entry. | Error discarded → **exit status 0**, nothing on stderr. | `proc_differential.rs::e4_closed_stdout` |
| E5 | `main` | fd 1 closed *and* fd 2 closed (both std streams unusable). | **exit status 0**, no output anywhere. | `proc_differential.rs::e5_closed_stdout_and_stderr` |
| E6 | `main` | stdout is a **read-only** fd (opened `O_RDONLY`) → `write` fails `EBADF`. | Error discarded → **exit status 0**. | `proc_differential.rs::e6_readonly_stdout` |
| E7 | `main` | stdout is a directory fd → `write` fails `EBADF`/`EISDIR`. | Error discarded → **exit status 0**. | `proc_differential.rs::e7_directory_stdout` |
| E8 | `main` | fd 0 (stdin) closed — `main` never reads it, so this must *not* be an error. | **exit status 0**, full normal output. | `proc_differential.rs::e8_closed_stdin` |

## Generic boundaries Phase C additionally requires

The C signature is `int main(void)`: no pointers, no lengths, no enums, no
buffers. So the classic boundary classes are covered as follows — each is a real
input to *this* API, mapped to what it means here:

| class | how it applies to `int main(void)` | expected | test |
|---|---|---|---|
| null pointers | No pointer parameters exist. The nearest equivalent: pass junk in the argument registers across FFI. Because the C ABI for `main(void)` ignores them, both must ignore them. | identical output, both return 0 | `ffi_differential.rs::g1_junk_arguments_ignored` |
| oversized / zero lengths | No length parameter exists. Nearest equivalent: argv count of 0 extra args, 1 arg, and 4096 args; and a single ~100 KiB argument (just under the kernel's 128 KiB MAX_ARG_STRLEN). `main()` declares no `argc`/`argv`, so all must behave identically. | identical output, exit 0 | `proc_differential.rs::g2_argv_counts_and_huge_arg` |
| one past a valid range | No numeric range exists. Nearest equivalents: `argv[0]` empty, `argv[0]` non-UTF-8, environment empty, environment oversized. | identical output, exit 0 | `proc_differential.rs::g3_arg0_and_env_edges` |
| out-of-range enum values | **No enum parameters exist** in the C API (`grep` for `enum`: 0 matches), so there is no invalid-variant value to smuggle across the boundary. Documented as N/A rather than skipped; the register-junk test (`g1`) is the strongest available analogue for "a value the C accepts that has no valid meaning". | identical | `ffi_differential.rs::g1_junk_arguments_ignored` |
| repeated / re-entrant calls | Calling the `.so`'s `main` many times, and interleaving C and Rust calls on one fd. | each call appends one identical line; always returns 0 | `ffi_differential.rs::b_repeated_and_interleaved` |
| return-value width | `int` return truncation: value must be exactly `0` in all 32 bits. | `0` | every FFI test asserts `== 0` |

## Row status

All rows E1–E8 and all generic-boundary rows have a differential test that
constructs the exact condition, runs **both** implementations, and asserts the
*same* result (same wait status / same signal / same byte count / same return
value) — not merely "both failed".

- [x] E1 [x] E2 [x] E3 [x] E4 [x] E5 [x] E6 [x] E7 [x] E8
- [x] g1 [x] g2 [x] g3 [x] repeated/re-entrant [x] return-value width

### Bug found via this table

**E1 was a real divergence.** Rust's runtime sets `SIGPIPE` to `SIG_IGN` before
`main`, so the original translation exited **0** where the C program is **killed
by signal 13 (status 141)**:

```
C_pipestatus=141      # killed by signal 13
R_pipestatus=0        # before fix
```

Fixed in `src/main.rs` by restoring `SIG_DFL` for `SIGPIPE` at process start
(modelling the C *runtime*, not the body of `main` — the exported `.so` `main` in
`src/lib.rs` correctly leaves signal dispositions alone, exactly as the C
library's `main` does, which is what row E2 pins down).
