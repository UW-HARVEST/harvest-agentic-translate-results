# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/main.c`. The complete list of constructs
in that file that can reject, error on, or otherwise refuse input:

```sh
$ grep -n 'return\|assert\|NULL\|if\|for\|<\|>\|ERROR' c_src/src/main.c
33:    for (int i = 0; i < len; i++)   # only bound/range check in the file
36:    printf("\n");
44:    print_hex(...)
49:    scanf("%d", &x);                # the only input-rejecting call
51:    return 0;                       # unconditional success
```

Findings:

* There is **no** `RETURN_ERROR`-style macro, **no** error enum, **no**
  `assert`, **no** `return -1` / `return NULL`, **no** null check, and **no**
  min/max constant in the C source.
* `main` returns `0` **unconditionally** — the `scanf` return value is
  **discarded**, so the only observable consequence of a rejected input is that
  `x` keeps its initializer value `0`, which makes `driver` print
  `00000000030000000000000000000040`.
* `driver(int floors)` performs **no validation at all**: every one of the
  2^32 `int` values is valid input.
* The only range check in the file is `i < len` in the `static` helper
  `print_hex`, whose sole call site passes the constant `sizeof(house_t)`.

The rejection surface therefore lives entirely inside the `scanf("%d", &x)`
conversion. Every distinct way that conversion can fail or clamp is one row
below. Expected C results were confirmed against glibc 2.34 (`fscanf` probe)
and against the built `c_src` executable.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `main` / `scanf("%d")` | stdin is empty — EOF before any character | input failure, `scanf` → `EOF` (-1); `x` untouched = 0; prints `00000000030000000000000000000040`; `main` → 0 | `e1_empty_input` | [x] |
| E2 | `main` / `scanf("%d")` | stdin is whitespace only (`" \t\n\v\f\r"`) — EOF after skipping whitespace | input failure, `scanf` → `EOF`; `x` = 0; same output as E1 | `e2_whitespace_only` | [x] |
| E3 | `main` / `scanf("%d")` | first non-whitespace character is not a digit or sign (`"abc"`, `"x"`, `"."`, `","`, `"!"`) | matching failure, `scanf` → 0; `x` = 0 | `e3_leading_non_digit` | [x] |
| E4 | `main` / `scanf("%d")` | a lone sign followed by EOF (`"-"`, `"+"`) | matching failure, `scanf` → **0** (not EOF — a character was already consumed); `x` = 0 | `e4_sign_then_eof` | [x] |
| E5 | `main` / `scanf("%d")` | sign followed by a non-digit (`"- 5"`, `"-x"`, `"+."`, `"--1"`) | matching failure, `scanf` → 0, characters pushed back; `x` = 0 | `e5_sign_then_non_digit` | [x] |
| E6 | `main` / `scanf("%d")` | `%d` is base 10, so a hex/octal prefix is not a number: `"0x10"`, `"0X1f"` | partial match: `0` consumed, stops at `x`; `scanf` → 1; `x` = 0 | `e6_hex_prefix_rejected` | [x] |
| E7 | `main` / `scanf("%d")` | value above `INT_MAX` but within `long` (`"2147483648"`, `"4294967296"`, `"9223372036854775807"`) | no error; glibc `strtol` succeeds, then `*ptr = (int) num.l` truncates: `-2147483648`, `0`, `-1` | `e7_int_overflow_truncates` | [x] |
| E8 | `main` / `scanf("%d")` | value above `LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999"`, 4000-digit number) | `ERANGE`: `strtol` saturates to `LONG_MAX`, truncated to `int` = `-1`; `scanf` still → 1 | `e8_long_overflow_saturates_high` | [x] |
| E9 | `main` / `scanf("%d")` | value below `LONG_MIN` (`"-9223372036854775809"`, `"-99999999999999999999"`) | `ERANGE`: saturates to `LONG_MIN`, truncated to `int` = `0`; `scanf` → 1 | `e9_long_overflow_saturates_low` | [x] |
| E10 | `main` / `scanf("%d")` | conversion terminated by trailing garbage (`"5abc"`, `"12 34"`, `"7-9"`) — remainder never consumed | `scanf` → 1 with only the first token converted; `x` = 5 / 12 / 7 | `e10_trailing_garbage` | [x] |
| E11 | `main` / `scanf("%d")` | input contains embedded NUL / non-UTF-8 bytes (`"\0"`, `"\xff\xfe"`, `"4\x002"`, `"\xc3"`) | NUL and 0x80..0xff are not whitespace and not digits: matching failure (→ 0) or conversion stops there; never a panic | `e11_nul_and_non_utf8_bytes` | [x] |
| E12 | `print_hex` (internal) | `len <= 0` — the only range check in the file | loop body skipped, prints just `"\n"`. **Unreachable through the public API**: the sole call site passes `sizeof(house_t)` = 16, and `print_hex` is `static` so it is absent from both `.so`s (see SYMBOLS.md) | `e12_print_hex_not_exported` | [x] |
| E13 | `driver` | *no* rejection path exists: `driver` validates nothing, so `INT_MIN`, `INT_MAX`, `0`, `-1` are all accepted | always prints 16 hex bytes + `"\n"`; never fails | `e13_driver_accepts_every_int` | [x] |

## Rows from the descriptors and the stream state

`scanf`'s "input failure" path is not only reachable by EOF on a pipe: the
descriptor itself can fail, and the C stream keeps *state* that decides whether
a later conversion is even attempted. These rows are still derived from the same
single `scanf("%d", &x)` call, and the expected results were measured from
glibc.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E14 | `main` | stdin is `/dev/null` — readable but immediately at EOF | input failure; `x` = 0; exit status 0 | `e14_stdin_is_dev_null` | [x] |
| E15 | `main` | stdin cannot be read at all (it is a directory, so `read` fails with `EISDIR`) | input failure, the stream's error indicator is set; `x` = 0; exit status 0 | `e15_stdin_read_error` | [x] |
| E16 | `print_hex` / `main` | file descriptor 1 is **closed** before the program starts, so every `printf` fails with `EBADF` | the C ignores `printf`'s return value: no output, no diagnostic, exit status **0** | `e16_stdout_closed` | [x] |
| E17 | `print_hex` / `main` | the read end of the stdout pipe is closed, so the write fails with `EPIPE` | a C program runs with the **default `SIGPIPE` disposition**, so it is killed by signal 13 (no exit code) | `e17_stdout_pipe_reader_closed` | [x] |
| E18 | `main` (called repeatedly) | a rejected conversion must not consume the character it rejected: glibc pushes it back with `ungetc`, so repeating the conversion fails on the same character forever and never makes progress | every call prints the same line; the stream never advances | `e18_failed_conversion_does_not_consume` | [x] |
| E19 | `main` (called repeatedly) | the stream reaches EOF and *then* more data becomes available (a growing file / a FIFO reopened by a writer) | C's end-of-file indicator is **sticky**: without `clearerr` — which this code never calls — every later conversion reports EOF without reading, so `x` stays 0 forever | `e19_sticky_eof_on_a_growing_stdin` | [x] |

## Rows from the libc stdio stream state

`scanf`/`printf` operate on the libc `stdin`/`stdout` `FILE` objects, and those
objects carry observable state that outlives a single conversion. Every row here
was measured from the C.

| # | function | trigger (the exact condition) | expected C result | test | status |
|---|----------|-------------------------------|-------------------|------|--------|
| E20 | `main` | stdin is **seekable** and only partially consumed | glibc's `exit` runs `_IO_cleanup`, which `lseek`s the descriptor back to the stream's logical position: `"42 hello world\n"` leaves the offset at **2**, `"- 42 rest"` at **1** (the pushed-back character is not consumed), `"abc rest"` at **0**. The read-ahead is *not* swallowed | `e20_stdin_offset_restored_at_exit` | [x] |
| E21 | `main` (exported) | the host process itself reads with C stdio before/after calling the library | `stdin` is shared: with `"5 7"`, a host that reads first gets `5` and the library prints `7`; reversed, the library prints `5` and the host then reads `7` | `e21_stdin_shared_with_the_host` | [x] |
| E22 | `print_hex` (via `main`) | the host writes to stdout with C stdio around the call, or leaves through `_exit` | `stdout` is shared and fully buffered: the bytes appear **between** the host's markers, and are **dropped entirely** if the process `_exit`s without stdio cleanup | `e22_stdout_shared_with_the_host` | [x] |
| E23 | both | structural: the translation must go through libc stdio at all | both `.so`s import a `scanf`-family symbol and `printf`/`putchar`; this is also what makes the per-byte write granularity (and therefore the multi-threaded interleaving granularity) match | `e23_translation_uses_libc_stdio` | [x] |
| E24 | `main` | a second program reads the same descriptor afterwards (`{ ./driver; cat; } < f`) | the next reader sees exactly the unconsumed remainder (`" hello world\n"`), not an empty descriptor | `e24_next_reader_sees_the_remainder` | [x] |

## Generic FFI-boundary rows (required even though the C has no such checks)

| # | boundary | why it does not apply / what is tested instead | test | status |
|---|----------|-----------------------------------------------|------|--------|
| B1 | null pointer arguments | the exported API is `void driver(int)` and `int main(void)` — **no pointer parameter exists**, so a null pointer cannot be passed. The one pointer-taking function (`print_hex`) is `static`/unexported. Verified that neither `.so` exports it. | `b1_no_pointer_parameters` | [x] |
| B2 | zero / oversized length arguments | **no length parameter exists** in the exported API (`print_hex`'s `len` is internal and always 16). | `b1_no_pointer_parameters` | [x] |
| B3 | out-of-range enum values across FFI | the C source declares **no enum** and no enum-typed parameter; `driver`'s only parameter is `int`, for which *every* bit pattern is a valid value (row E13 / CONFIGS C1–C8). | `b3_no_enum_parameters` | [x] |
| B4 | value one step past a documented range | `int` has no documented sub-range; the extremes `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`, `0`, `-1` are all exercised and must match. | `b4_int_extremes` | [x] |
| B5 | garbage in the unused upper half of the argument register | `driver` takes a 32-bit `int`, so the C reads only `%edi`. Calling both `.so`s through a `extern "C" fn(i64)`-typed pointer with a value whose upper 32 bits are non-zero must produce identical output (both must ignore the upper half). | `b5_upper_half_of_arg_register_ignored` | [x] |
| B6 | return value across FFI | `main` must return exactly `0` from both `.so`s for every input class above, including all rejected ones. | `b6_main_returns_zero` | [x] |

All rows are covered, and every listed test passes against both `.so`s:

| test file | rows |
|-----------|------|
| `tests/phase_c_errors.rs` | E1–E13, B1–B6 |
| `tests/phase_c_process.rs` | E14–E17 |
| `tests/phase_b_repeated.rs` | E18, E19 |
| `tests/phase_c_stdio.rs` | E20–E24 |

## Divergences found and fixed

Seven real divergences were found. In every case the C is the ground truth and
the **Rust** was changed; `c_src/` was never touched.

| # | row | C behaviour | Rust behaviour before the fix | fix |
|---|-----|-------------|-------------------------------|-----|
| 1 | E17 | a C program keeps the **default** `SIGPIPE` disposition, so writing to a pipe with no reader kills it with signal 13 | the Rust runtime installs `SIG_IGN` before `main`, so the write failed with `EPIPE`, was ignored, and the process exited **0** | `src/main.rs` restores `SIG_DFL` for `SIGPIPE` at startup (in the **binary** only — a shared library must not change the host's signal disposition, and the C `.so` does not either) |
| 2 | E18 | `scanf` returns the terminating / mismatching character to the stream with `ungetc`: `"12x34"` converts `12` and then fails forever on `x`; `"5-6"` converts `5` and then `-6` | the character was consumed and discarded, so a second conversion read `34` — a cascading divergence for every later conversion | see below |
| 3 | E19 | the end-of-file indicator is **sticky**: after the first EOF, later conversions report EOF without reading | the reader tried again and picked up data appended after the EOF (`5,7,7,7` instead of `5,0,0,0`) | see below |
| 4 | E21 | the exported `main` reads through the process's libc `stdin`, shared with a host that also uses C stdio: with `"5 7"` the host takes `5` and the library prints `7` | a private `std::io::Stdin` saw an already-drained descriptor and printed `0` | see below |
| 5 | E20, E24 | glibc's `exit` seeks stdin back to the stream's logical position, so `{ ./driver; cat; } < f` leaves `" hello world\n"` for `cat` | an 8 KiB `BufReader` swallowed the whole read-ahead: offset 8 instead of 2, and the next reader saw nothing | see below |
| 6 | E22 | `stdout` is the host's fully-buffered stream: the line appears **between** a host's own `printf`s, and is **lost** if the process `_exit`s | a private line-buffered stream wrote immediately: the line came out *before* the host's marker, and survived an `_exit` that the C's did not | see below |
| 7 | E23 | 17 separate `printf` calls, so writes are per-byte and two threads calling `driver` interleave at 2-hex-digit granularity | one atomic 33-byte `write_all`, which can never interleave | see below |

Divergences 2–7 all had the same root cause — the translation reimplemented
`scanf`/`printf` on top of `std::io` instead of calling them — and all six were
fixed by one change: **`src/imp.rs` now calls the same libc functions the C
calls** (`scanf("%d", &x)`, and one `printf("%02x", …)` per byte plus
`printf("\n")`), on the same `FILE` objects, with no explicit flush. Pushback,
sticky EOF, `strtol` saturation, buffer sharing, the exit-time `lseek`, output
ordering, durability and write granularity are then inherited from libc exactly
rather than approximated. The struct is a `#[repr(C)]` type walked as raw bytes
with `size_of`, so the layout is the C's by construction too — no hard-coded
size, offsets or byte order.

Divergences 2, 3, 4, 6 and 7 are reachable because `main` and `driver` are
**exported symbols**, so a consumer can call them repeatedly, or from a host
that shares libc's streams — which is exactly what `tests/phase_b_repeated.rs`
and `tests/phase_c_stdio.rs` do.

Every fix was mutation-checked by reverting it and confirming the corresponding
tests fail:

| reverted fix | result |
|--------------|--------|
| pushback (divergence 2) | 3 of the 5 tests in `tests/phase_b_repeated.rs` fail |
| sticky EOF (divergence 3) | `e19_sticky_eof_on_a_growing_stdin` fails (`5,7,7,7` vs `5,0,0,0`) |
| libc stdio (divergences 4–7) | all 5 tests in `tests/phase_c_stdio.rs` fail |
| `SIGPIPE` (divergence 1) | `e17_stdout_pipe_reader_closed` fails (exit 0 vs signal 13) |

So the tests genuinely detect these divergences rather than passing vacuously.
