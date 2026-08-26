# ERRORS.md — Error-surface table (Phase A) / error-path tests (Phase C)

## Mechanical derivation

Every rejection/error construct was grepped for across the *entire* C source
(`c_src/include/driver.h`, `c_src/src/driver.c` — 28 + 36 lines, comments
included):

```sh
grep -nE 'return|assert|NULL|errno|exit|abort|if|else|switch|case|while|for|\?|#if|#ifdef|#ifndef|<|>|MAX|MIN|ERROR|goto' \
    c_src/src/driver.c c_src/include/driver.h
```

The only non-comment hits are:

```
c_src/include/driver.h:24:#ifndef DRIVER_H_      <- include guard
c_src/include/driver.h:29:#endif //DRIVER_H_     <- include guard
c_src/src/driver.c:26:#include <stdio.h>         <- matched on '<' '>'
```

Consequently the C library contains:

* **0** `return` statements (both functions are `void` and fall off the end)
* **0** error-return macros / sentinels / error enums
* **0** `assert`s
* **0** explicit range checks, null checks, or min/max constants
* **0** branches of any kind (`if` / `switch` / `?:` / loops) — confirmed by the
  disassembly, which is straight-line code in both functions
* **0** pointer, length/size, array, or enum parameters
  (`void driver(char)`, `void printHexCharLine(char)`)

**There is therefore no input the C code rejects.** The entire `char` domain
(all 256 bit patterns) is valid input and every one of them produces output.
The classic "one row per distinct `RETURN_ERROR` branch" table is genuinely
empty for this library — that is a derived fact, not an omission.

What remains, and what Phase C must still prove, are (a) the *generic* C-API
boundaries the task mandates, and (b) the boundaries of the narrow-integer
domain and of the implicit arithmetic conversions, which are where this
library's only value-dependent behaviour lives. Those are enumerated below and
each has a differential test.

## Error / boundary table

`out` = bytes written to `stdout`. Both functions return `void`, so `out` (plus
"does it crash") is the *complete* observable behaviour.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|----|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `printHexCharLine` | `0x00` — minimum unsigned value / NUL byte, smallest `%02x` input (needs zero padding) | `out == "00\n"`, no crash | `err_e1_e6_boundary_char_values` | [x] |
| E2 | `printHexCharLine` | `0x7F` — `CHAR_MAX`, last non-negative `char` | `out == "7f\n"` | `err_e1_e6_boundary_char_values` | [x] |
| E3 | `printHexCharLine` | `0x80` — `CHAR_MIN` (`-128`); one step past the positive range, so the `int` promotion **sign-extends** and `%x` prints 8 digits | `out == "ffffff80\n"` | `err_e1_e6_boundary_char_values` | [x] |
| E4 | `printHexCharLine` | `0xFF` — `-1`, the all-ones pattern; maximal sign extension | `out == "ffffffff\n"` | `err_e1_e6_boundary_char_values` | [x] |
| E5 | `driver` | `0x7F` (`CHAR_MAX`) — `data + 1` **overflows the `char` range**; the `int` result `128` is converted back to `char`, wrapping to `-128` | `out == "ffffff80\n"` | `err_e1_e6_boundary_char_values` | [x] |
| E6 | `driver` | `0xFF` (`-1`) — `data + 1` wraps the *other* way, to `0x00` | `out == "00\n"` | `err_e1_e6_boundary_char_values` | [x] |
| E7 | `driver` | `0x80` (`CHAR_MIN`) — smallest input, result `-127` still negative | `out == "ffffff81\n"` | `err_e1_e6_boundary_char_values` | [x] |
| E8 | `printHexCharLine` | value one step past the padded-width range: `0x0F` -> `0x10` transition of `%02x` | `out == "0f\n"` / `"10\n"` | `err_e1_e6_boundary_char_values` | [x] |
| E9 | both | **out-of-range integer passed across the FFI boundary.** The C prototype is `char`, but at the SysV ABI level the argument register is 32-bit and C accepts any `int` here (exactly as a C enum accepts any `int`). Callers pass `0x100`, `0x1FF`, `0x180`, `-1000`, `i32::MIN`, `i32::MAX`, ... The callee must consider **only the low 8 bits**. | identical to passing `(char)(v & 0xFF)`; both `.so`s must agree byte-for-byte | `err_e9_out_of_range_int_arg_via_ffi` | [x] **found a real bug — see below** |
| E10 | both | out-of-range integer, **exhaustive over the low byte** with dirty high bits (`v = 0xDEADBE00 \| b` for all 256 `b`, and `0xFFFFFF00 \| b`) | low byte only; must equal the `char` call | `err_e9_out_of_range_int_arg_via_ffi` | [x] |
| E11 | both | **`stdout` write failure**: `stdout` redirected to `/dev/full`, so the underlying `write()` fails with `ENOSPC`. `printf`'s return value is *ignored* by the C, so this must be silently tolerated. | function returns normally, process exits 0, no bytes reach the fd | `err_e11_stdout_write_fails_dev_full` | [x] |
| E12 | both | **`stdout` not writable**: fd 1 replaced by a **read-only** descriptor, so `write()` fails with `EBADF`. | function returns normally, process exits 0, nothing written | `err_e12_stdout_fd_not_writable` | [x] |
| E13 | both | **`stdout` fully closed** (fd 1 closed before the call) | function returns normally, process exits 0 | `err_e13_stdout_closed` | [x] |

### Rows that are N/A, and why (documented so the gap is deliberate)

The task's generic boundary list also names null pointers, zero/oversized
lengths, and out-of-range enum values. These are **structurally impossible** for
this API rather than untested:

| generic boundary | applicability |
|---|---|
| null pointer argument | **N/A** — neither public function takes a pointer. There is no pointer, array, struct, or string parameter anywhere in `driver.h`/`driver.c`. |
| zero length / oversized length | **N/A** — there is no length, size, count, or capacity parameter; nothing is indexed or allocated. |
| out-of-range enum value | **N/A as a named enum** — no `enum` type exists in the library. Its exact analogue for this API *is* covered: `char` is the narrow integer parameter, and rows **E9/E10** push out-of-range `int` values through the same 32-bit argument register that an out-of-range enum would travel in. |
| error code / sentinel return | **N/A** — both functions are `void`; `printf`'s `int` result is discarded by the C, so no error code is observable. Rows E11–E13 verify the *behaviour* under I/O failure instead. |

All rows E1–E13 are implemented in `tests/differential_errors.rs` and pass
against both `.so`s.

## Row E9/E10 caught a genuine translation defect

This is the one row that found a real bug, and it was invisible in the `debug`
profile — exactly the "class of bug that happy-path tests miss" the row exists
for.

gcc compiles `void printHexCharLine(char)` so that only the low 8 bits of the
argument register are ever read (`movsbl %dil, %esi`, at **every** `-O` level).
The original Rust declared the parameter as `c_char`, which makes rustc attach
LLVM's `signext i8` attribute; with optimisations enabled LLVM then folded the
truncation away and passed the caller's full 32-bit register to `printf`:

| `printHexCharLine(int 0xdeadbe00)` | output |
|---|---|
| C (gcc, ground truth) | `00\n` |
| Rust `debug` (before fix) | `00\n` — accidentally agreed |
| Rust `release` (before fix) | `deadbe00\n` — **divergent** |

Fixed by taking the argument as `c_int` and masking to the low byte explicitly
(`char_arg()` in `src/lib.rs`), which reproduces gcc's `movsbl %dil` and cannot
be optimised away. Both profiles now agree with the C for all 1024+ probed
out-of-range values.

Caveat, recorded for honesty: the upper 24 bits of a narrow argument register are
*unspecified* by the psABI, and C compilers genuinely disagree — a clang `-O2`
build of the same C does **not** truncate. The translation matches **gcc**, which
is what `c_src/CMakeLists.txt` builds with (`/usr/bin/cc` -> GCC 11.5.0, no
`CMAKE_BUILD_TYPE`, no flags) and therefore what "the C is the ground truth"
means for this project.

## How these rows are run

```sh
./run_diff_tests.sh          # every feature combination x {debug, release}
```

Every row above passes in **both** cargo profiles. Rows E11–E13 execute in a
forked child so that a poisoned `stdout` error flag cannot leak into the rest of
the suite.
