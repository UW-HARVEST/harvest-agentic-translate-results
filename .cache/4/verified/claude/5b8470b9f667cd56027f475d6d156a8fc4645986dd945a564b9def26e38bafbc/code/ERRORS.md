# ERRORS.md — error-surface table (Phase A, gates Phase C)

## Mechanical derivation

Every rejection/error construct was grepped for across the whole of `c_src/`:

```sh
grep -nE "return|assert|NULL|errno|if|else|switch|while|for|goto|exit|abort|<|>|==|!=|#if" \
    c_src/src/driver.c c_src/include/driver.h
```

Result (excluding the 23-line licence header and the `#include`s /
`#ifndef DRIVER_H_` include guard): **no matches**.

```text
c_src/include/driver.h:24:#ifndef DRIVER_H_      <- include guard
c_src/include/driver.h:29:#endif //DRIVER_H_     <- include guard
c_src/src/driver.c:26..29: #include <ctype.h> <locale.h> <stdio.h> <stdlib.h>
```

So, mechanically:

* error-return macros (`RETURN_ERROR`, …): **0**
* `return -1` / `return NULL` / error enums: **0** (`driver` returns `void`)
* `assert`: **0**
* explicit range checks / null checks: **0**
* min/max constants: **0**
* `#ifdef` configuration branches: **0** (only the header include guard)

`driver` has exactly one parameter, `char c`, no pointer parameters, no
return value, and no enum parameters. It therefore has **no error surface of
its own**: every `char` bit pattern is a valid, accepted input.

That does not mean there is nothing to verify. The rejection/edge behaviour
that *is* reachable through this API lives in (a) the libc calls `driver`
makes and how their failures are (not) handled, and (b) the FFI/ABI boundary
itself. Those are enumerated below, and each row has a differential test that
asserts C and Rust behave identically — same bytes, same sentinel, same
survival.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `driver` | Any `char` value at all — there is no validation, no `assert`, no range check, no error return. Asserted for the **entire** input domain, all 256 bit patterns `-128 ..= 127`. | No rejection possible: always prints exactly 14 lines and returns `void`. Never a sentinel, never a crash. | [x] |
| 2 | `driver` → `setlocale(LC_ALL, "C")` | `setlocale` fails and returns `NULL` (return value is **discarded** by the C code — the one "unchecked error" in the file). Forced by pre-setting a foreign global locale + a foreign thread locale, then observing that the discarded result cannot change behaviour. | Return value ignored; no error propagated; the 14 lines are printed regardless. `"C"` is the always-available builtin locale, so the call cannot actually fail — the *handling* (none) is what must match. | [x] |
| 3 | `driver` → `printf` | `printf` fails: `stdout`'s fd is **closed** (`EBADF`). 14 failing `printf` calls. | All 14 return values discarded, no `errno` check, no abort; `driver` runs to completion and returns normally, producing 0 bytes of output. | [x] |
| 4 | `driver` → `printf` | `printf` fails: `stdout`'s fd redirected to a **read-only** fd (`EBADF` on write) / to `/dev/full` (`ENOSPC` on flush). | Same as row 3: return values discarded, completes normally. | [x] |
| 5 | `driver` → `tolower`/`toupper` (glibc range guard `__c >= -128 && __c < 256`) | The *one* explicit range check reachable from this code, inside glibc: an argument outside `-128 ..= 255`. Unreachable from a `char` (`-128 ..= 127` is a strict subset), so the guard's else-branch (`return __c` unchanged) must be shown to be dead for both implementations. Probed by calling the exported symbol through a **widened** prototype `void driver(int)` with `256`, `300`, `-129`, `-1000`, `65536`, `0x1234`, `INT_MIN`, `INT_MAX` + 100 seeded random out-of-range values. | Callee truncates to the low byte (`char` parameter — GCC emits `mov %al` at every `-O` level), so the guard never trips and the result equals `driver((char)v)`. **This row caught a real bug:** the optimised Rust build used the full 32-bit value as the ctype-table index (`control: 0` where C prints `control: 2`, plus an out-of-bounds table read). Fixed in `src/lib.rs`; see `CONFIGS.md` § "Divergence found and fixed". | [x] |
| 6 | `driver` | Boundary `char` values one step past each documented sub-range: `0` (NUL), `-1` (`0xFF`), `-128` (`0x80`, most-negative → most negative table index), `127` (`0x7F` DEL), `31`/`32` (cntrl↔print edge), `126`/`127` (print↔cntrl edge), `47`/`48`/`57`/`58` (digit edges), `64`/`65`/`90`/`91` (upper edges), `96`/`97`/`122`/`123` (lower edges), `70`/`71`/`102`/`103` (xdigit edges). | Each is a plain table lookup; no rejection. The negative indices (`-128 ..= -1`) are in-bounds for glibc's tables, which span `-128 ..= 255`. | [x] |
| 7 | `driver` | `c == 0`: `printf("to lower: %c\n", 0)` writes a **raw NUL byte** into the output stream — the degenerate "zero length"/embedded-terminator case for this API. | Output contains a literal `\0` between `": "` and `"\n"`; the line is 12 bytes, not a truncated 11. | [x] |
| 8 | `driver` | `%c` with a **negative** conversion result (only possible if the active `tolower`/`toupper` table maps a byte to a negative `int`; probed under all 8 test locales for all 256 inputs). `printf` converts the `int` to `unsigned char`. | Low byte of the value is emitted; no error, no multi-byte escape. | [x] |
| 9 | `driver` | No null-pointer surface exists to test: `driver` takes no pointers. The only pointer the C code passes to libc is the `'static` `"C"` literal, which it never derives from user input. Asserted by inspection of the signature (`void driver(char)`) — there is no way for a caller to pass a null pointer. | n/a — documented as "no such input exists", not silently skipped. | [x] |
| 10 | `driver` | No out-of-range **enum** surface exists in the C API (`grep -n "enum" c_src/` → no matches; the sole parameter is `char`, not an enum). The closest analogue — an out-of-range *integer* arriving where a narrow type is expected — is covered by row 5. | n/a — documented; the integer analogue is tested in row 5. | [x] |

## Row → test mapping

All in `tests/phase_c_errors.rs`; every checkbox is a runnable test.

| row | test function |
|---|---|
| 1 | `e1_no_input_is_ever_rejected` (all 256 bit patterns) |
| 2 | `e2_setlocale_result_is_discarded` (bogus `LC_ALL` in the environment + every available locale pre-set) |
| 3 | `e3_printf_failure_with_stdout_closed_unbuffered`, `e3b_printf_failure_with_stdout_closed_buffered` |
| 4 | `e4_printf_failure_on_a_read_only_fd`, `e4b_printf_failure_on_dev_full`, `e4c_printf_failure_on_dev_full_buffered` |
| 5 | `e5_out_of_range_int_arguments` (19 hand-picked + 100 seeded random out-of-range `int`s) |
| 6 | `e6_class_boundary_values` (34 boundary values; also × 8 locales in `CONFIGS.md` row B16) |
| 7 | `e7_nul_char_emits_a_raw_nul_byte` |
| 8 | `e8_high_byte_conversion_results` (8 locales × 256 chars; 2048 high-bit conversion results observed) |
| 9, 10 | `e9_e10_public_api_has_no_pointer_length_or_enum_parameters` (re-derives the API shape from `driver.h` and re-greps `driver.c`, so the "no such input exists" reasoning cannot silently rot) |
| extra | `e11_no_stub_or_extra_entry_points` — `dlsym` probes (`driver_impl`, `driver_ffi`, `rust_driver`, `driver2`, …) must resolve identically in both libraries, i.e. Rust exposes no stub entry points the C lacks |

Rows 3 and 4 run the call in a **forked child** with the broken `stdout`, both
because redirecting fd 1 in-process would also break libtest's own output, and
because the child's exit status then proves `driver` *returned* instead of
aborting: an aborting Rust build would die with `SIGABRT` and never send its
report, while C exits 0 after silently ignoring all 14 failed `printf`s.

## Notes on rows 3/4 (the only observable "failure" mode)

`driver`'s contract is "print 14 lines and return". When the underlying write
fails, C's behaviour is *silence*: `printf`'s `int` result is dropped on all
14 lines, `ferror(stdout)` is never consulted, and control returns normally.
The Rust translation must not "improve" on this — it must not panic, must not
`unwrap()` an I/O error, and must not abort the process (which, with
`panic = "abort"` in the release profile, would be a hard crash for a caller
that C serves happily). Rows 3 and 4 assert exactly that: both `.so`s return
normally and both produce zero bytes.
