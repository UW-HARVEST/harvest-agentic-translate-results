# ERRORS.md — error-surface table (Phase A / gate for Phase C)

## How this table was derived

The whole C program is 12 lines of code:

```c
void driver(int x) {
    register int y = 2*x;
    y += 300;
    printf("%d\n", y);
}

int main() {
    int x = 0;
    scanf("%d", &x);
    driver(x);
    return 0;
}
```

A mechanical grep for every rejection construct finds **none** in the C source
itself:

```sh
$ grep -nE 'RETURN_ERROR|return -1|return NULL|assert|errno|exit\(|if *\(' c_src/src/main.c
(no matches)
$ grep -nE 'return' c_src/src/main.c
36:    return 0;          # the single, unconditional success return
```

So the C code contains **zero explicit error checks, asserts, range checks or
null checks**, and it **ignores the return value of `scanf`**. That means the
entire rejection surface is:

1. the failure modes of the one library call it makes — `scanf("%d", &x)`
   (input failure, matching failure, `ERANGE` saturation) — each of which
   manifests only through *whether `x` keeps its initialiser `0`* and through
   *what value is truncated into the `int`*; and
2. the process/FFI boundary conditions any C program of this shape has
   (unreadable `stdin`, unwritable `stdout`, extreme `int` arguments to
   `driver`).

Every row's "expected C result" column below is **measured** from the compiled
C program, never assumed. `y = 2*x + 300` computed with 32-bit wraparound, so
`x = 0` (the "rejected input" case) always prints `300`.

Legend: *out* = bytes on `stdout`, *rc* = exit status.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `main`/`scanf` | **Input failure — immediate EOF.** Empty `stdin` (`< /dev/null`, 0 bytes). `scanf` returns `EOF`, `x` untouched. | out=`300\n`, rc=0 |
| 2 | `main`/`scanf` | **Input failure — EOF while skipping whitespace.** `stdin` = `"   \n\t "` only. | out=`300\n`, rc=0 |
| 3 | `main`/`scanf` | **Input failure — unreadable `stdin`.** fd 0 closed (`0<&-`); `read` fails with `EBADF`. | out=`300\n`, rc=0 |
| 4 | `main`/`scanf` | **Input failure — `stdin` is a directory.** `read` fails with `EISDIR`. | out=`300\n`, rc=0 |
| 5 | `main`/`scanf` | **Matching failure — first non-space byte is a letter.** `"abc"`. `scanf` returns 0, `x` untouched. | out=`300\n`, rc=0 |
| 6 | `main`/`scanf` | **Matching failure — first non-space byte is punctuation/`.`.** `".5"`, `"-.5"`, `"*"`. | out=`300\n`, rc=0 |
| 7 | `main`/`scanf` | **Matching failure — NUL byte first.** `"\0 42"` (a valid byte that is neither `isspace` nor `isdigit`). | out=`300\n`, rc=0 |
| 8 | `main`/`scanf` | **Matching failure — byte ≥ 0x80 first.** `"\x80\x8142"`; not whitespace in the `"C"` locale. | out=`300\n`, rc=0 |
| 9 | `main`/`scanf` | **Matching failure — lone sign then EOF.** `"-"` or `"+"` (glibc: work buffer holds only the sign ⇒ `conv_error`). | out=`300\n`, rc=0 |
| 10 | `main`/`scanf` | **Matching failure — sign followed by non-digit.** `"-abc"`, `"- 42"` (space after sign), `"++5"`, `"--5"`. | out=`300\n`, rc=0 |
| 11 | `main`/`scanf` | **`ERANGE` saturation, positive.** magnitude > `LONG_MAX`: `"9223372036854775808"`, `"99999999999999999999"`, 10⁶ `9`s. `strtol` clamps to `LONG_MAX`; `(int)LONG_MAX == -1`. | out=`298\n`, rc=0 |
| 12 | `main`/`scanf` | **`ERANGE` saturation, negative.** magnitude > `LONG_MAX`+1: `"-9223372036854775809"`, `"-99999999999999999999"`. Clamps to `LONG_MIN`; `(int)LONG_MIN == 0`. | out=`300\n`, rc=0 |
| 13 | `main`/`scanf` | **Exact `long` boundaries (no clamp, but truncated).** `"9223372036854775807"` → `(int)` = `-1`; `"-9223372036854775808"` → `(int)` = `0`. | out=`298\n` / `300\n`, rc=0 |
| 14 | `main`/`scanf` | **Out-of-`int`-range but in-`long`-range.** `"2147483648"` (`INT_MAX`+1) → `(int)` = `INT_MIN`; `"4294967296"` (2³²) → `(int)` = `0`. | out=`300\n` (both), rc=0 |
| 15 | `main`/`scanf` | **Base-prefix rejection.** `"0x10"` with `%d`: base is 10, so glibc keeps the `0`, refuses to consume `x`, converts `0`. | out=`300\n`, rc=0 |
| 16 | `main`/`scanf` | **Grouping rejection.** `"1,000"` — `%d` has no `'` flag, so the `,` terminates the number at `1`. | out=`302\n`, rc=0 |
| 17 | `main`/`scanf` | **Trailing-junk truncation.** `"42abc"`, `"1e5"`, `"42 99"` — conversion stops at the first non-digit; the remainder is never read. | out=`384\n` / `302\n` / `384\n`, rc=0 |
| 18 | `driver`/`printf` | **Unwritable `stdout` — fd 1 closed** (`1>&-`). `printf` returns `-1`; the C code ignores it and still `return 0`. | out=`` (nothing), rc=0 |
| 19 | `driver`/`printf` | **Unwritable `stdout` — closed pipe.** Read end of the `stdout` pipe closed before the write ⇒ `SIGPIPE` with the **default** disposition kills the process. | out=`` (nothing), rc=141 (signal 13) |
| 20 | `driver` | **Signed-overflow of `2*x` (UB in C, wraps on the target).** `x = INT_MIN`, `x = ±2³⁰`, `x = INT_MAX`. Measured: `INT_MIN`→`300`, `2³⁰`→`-2147483348`, `INT_MAX`→`298`. | wrapped 32-bit value, rc=0 |
| 21 | `driver` | **Signed-overflow of `y += 300`.** `x` in `[INT_MAX/2 - 149, INT_MAX/2]`, e.g. `x = 1073741823` → `2147483646 + 300` wraps to `-2147483350`. | wrapped 32-bit value, rc=0 |
| 22 | `driver` (FFI) | **Extreme `c_int` arguments across the FFI boundary.** `INT_MIN`, `INT_MAX`, `-1`, `0` passed to the exported `driver` symbol. | wrapped `2*x+300`, no trap |
| 23 | `main` (process) | **`SIGPIPE` inherited as `SIG_IGN`.** The program is `exec`ed by a parent that ignored `SIGPIPE` (survives `execve`), `stdout` = pipe with no reader. `printf` gets `EPIPE`, which the C ignores. | out=`` (nothing), rc=0, **no signal** |
| 24 | `main`/`driver` | **Allocation failure.** Tight `RLIMIT_AS` (4–64 MiB): glibc's `scanf`/`printf` fall back to the `FILE`'s one-byte `_shortbuf` and keep working. | out=`384\n` for `"42rest"`, rc=0, nothing on `stderr` |

### Notes on classes that do not exist in this API

* **Null-pointer arguments** — neither exported function takes a pointer, so
  there is no null-pointer row. (Row 3/4 cover the equivalent "the one implicit
  input is unusable" case.) The only pointer in the C source, `&x`, is always a
  valid address of a live automatic variable.
* **Out-of-range enum values across FFI** — the API declares no enum and no
  flag/mode parameter; `driver`'s only parameter is `int`, for which *every*
  bit pattern is a valid value. Row 22 therefore covers the whole domain's
  extremes instead, and rows 20–21 cover the values that make the arithmetic
  overflow.
* **Zero / oversized lengths** — there is no length or size parameter. The
  analogous axis is the *input length* on `stdin`, covered by rows 1–2
  (zero-length) and row 11 (a 1 000 000-byte number, far past any internal
  buffer).

### Divergences found and fixed while building this table

Four real divergences were found by deriving the surface mechanically instead of
testing the happy path. All four are fixed in `src/main.rs` and pinned by a test.

1. **Row 19 — `SIGPIPE` killed the C but not the Rust.** Rust's runtime installs
   `SIG_IGN` for `SIGPIPE` before `main` runs, so the translation ignored the
   failing `write` and exited `0` where the C executable is killed by signal 13
   (status 141). Pinned by `err19_stdout_closed_pipe_raises_sigpipe`.

2. **Row 23 — and the reverse.** Simply forcing `SIG_DFL` fixes row 19 but breaks
   the case where `SIGPIPE` was *inherited* as `SIG_IGN` (it survives `execve`;
   `fork`+`exec` daemons and anything launched from CPython leave it that way):
   the C survives and the Rust would die. Fixed by recording the inherited
   disposition in an ELF `.init_array` constructor — which runs *before* Rust's
   runtime initialisation — and reinstalling it at the start of `main`
   (`mod sigpipe`). Pinned by `err23_inherited_sigpipe_ign_is_preserved`.

3. **How much of `stdin` is consumed.** Rust's `io::Stdin` over-reads into an
   8192-byte `BufReader` and never gives the surplus back, whereas glibc reads
   `st_blksize` bytes (capped at `BUFSIZ`) per `read`, pushes the character that
   terminated the conversion back with `ungetc`, and — for a *seekable*
   descriptor — `lseek`s back to the first unconsumed byte when the stream is
   cleaned up at exit. Directly observable to a second reader sharing fd 0: for
   the input `42rest` the C leaves `rest` unread while the original translation
   left nothing, and `{ ./driver; ./driver; ./driver; } < "42 99 7"` printed
   `384 300 300` instead of `384 498 314`. Fixed by the `CStdin` reader. Pinned
   by `CONFIGS.md` rows 29–30.

4. **Row 24 — allocation failure aborted the process.** `vec![0u8; 4096]` for the
   input buffer and `io::stdout()`'s lazy 1024-byte `LineWriter` both abort with
   a `stderr` message when allocation fails, while glibc falls back to the
   `FILE`'s `_shortbuf` and still prints `384`. Under `ulimit -v 3125` the C
   printed `384` (rc 0) and the Rust died with `SIGABRT` (rc 134). Fixed by
   making the translation allocation-free: an inline `[u8; BUFSIZ]` input buffer
   and a stack-formatted, single-`write` output path (`print_d_line`), which
   still produces exactly one 4-byte `write` like glibc. Pinned by
   `err24_tight_address_space_limit_does_not_abort`.

### Residual differences, and why they are not translatable

These were found by the same audit, verified to be real, and deliberately not
"fixed" — each of them requires a *third party inside the same process* to reach
into glibc's `FILE` objects, which a Rust reimplementation of `scanf`/`printf`
cannot share by construction. None of them is reachable through the program's own
inputs, exit status, or output bytes.

| # | condition | C | Rust |
|---|-----------|---|------|
| R1 | `stdbuf -i0` / any `LD_PRELOAD` calling `setvbuf` on `stdin`, with a **pipe** on fd 0 | 1-byte reads, so only the parsed bytes are consumed | reads a full `st_blksize` block (`stdbuf` cannot reach a non-glibc buffer) |
| R2 | an `LD_PRELOAD`ed library that writes to fd 1 from a destructor / `atexit` handler | glibc flushes `stdout` in `_IO_cleanup`, i.e. *after* all handlers ⇒ `[CTOR][DTOR][ATEXIT]384` | the write happens inside `driver` ⇒ `[CTOR]384[DTOR][ATEXIT]` |
| R3 | calling the shared object's exported `main` **more than once** with a non-seekable fd 0 | the persistent `stdin` `FILE` still holds the read-ahead ⇒ `384 498 314` | a fresh reader per call ⇒ `384 300 300` (seekable fd 0 matches, because the exit-time `lseek` lands on the same byte) |
| R4 | a host process that interleaves its own `printf` with `dlsym`ed `main`/`driver` calls | shares one `stdout` buffer ⇒ `A384\nB` | separate write paths ⇒ `384\nAB` |
| R5 | `RLIMIT_AS` below ≈3.1 MB | the 16 KiB C binary still loads | the larger Rust binary fails in `ld.so` (`libc.so.6: failed to map segment`) — a property of binary size, not of behaviour |
| R6 | `RLIMIT_AS` ≈4–16 MB *and* an ≥8-million-digit number | glibc's `%d` needs O(digits) of heap for its work buffer; the allocation fails, `scanf` returns `EOF`, so `x` stays `0` ⇒ `300` | the parser is allocation-free and still converges ⇒ `298` |

R3/R4 concern the `main` symbol of a *shared object built from an executable's
source*, which no real consumer calls; R1/R2/R4 need a co-loaded library; R5/R6
need an artificial address-space limit. For every input, on both a seekable and a
non-seekable `stdin`, with `stdout` on a pipe, a file, a tty, a socket, `/dev/full`
or closed, the two programs are byte-identical.

## Row → test mapping (Phase C gate)

| row(s) | test | test binary |
|--------|------|-------------|
| 1 | `err01_input_failure_empty_stdin` | `tests/phase_c_errors.rs` |
| 2 | `err02_input_failure_whitespace_only` | `tests/phase_c_errors.rs` |
| 3 | `err03_input_failure_stdin_closed` | `tests/phase_c_errors.rs` |
| 4 | `err04_input_failure_stdin_is_directory` | `tests/phase_c_errors.rs` |
| 5 | `err05_matching_failure_letter` | `tests/phase_c_errors.rs` |
| 6 | `err06_matching_failure_punctuation` | `tests/phase_c_errors.rs` |
| 7 | `err07_matching_failure_nul_byte` | `tests/phase_c_errors.rs` |
| 8 | `err08_matching_failure_high_bytes` | `tests/phase_c_errors.rs` |
| 9 | `err09_matching_failure_lone_sign_then_eof` | `tests/phase_c_errors.rs` |
| 10 | `err10_matching_failure_sign_then_non_digit` | `tests/phase_c_errors.rs` |
| 11 | `err11_erange_positive_clamps_to_long_max`, `err20_ffi_multiply_overflow` | `tests/phase_c_errors.rs` |
| 12 | `err12_erange_negative_clamps_to_long_min` | `tests/phase_c_errors.rs` |
| 13 | `err13_long_boundaries_truncate` | `tests/phase_c_errors.rs` |
| 14 | `err14_out_of_int_range_truncates` | `tests/phase_c_errors.rs` |
| 15 | `err15_hex_prefix_rejected` | `tests/phase_c_errors.rs` |
| 16 | `err16_grouping_rejected` | `tests/phase_c_errors.rs` |
| 17 | `err17_trailing_junk_truncates` | `tests/phase_c_errors.rs` |
| 18 | `err18_stdout_closed_is_ignored` | `tests/phase_c_errors.rs` |
| 19 | `err19_stdout_closed_pipe_raises_sigpipe` | `tests/phase_c_errors.rs` |
| 20 | `err20_multiply_overflow_wraps`, `err20_ffi_multiply_overflow` | `tests/phase_c_errors.rs`, `tests/phase_c_ffi.rs` |
| 21 | `err21_add_overflow_wraps`, `err21_ffi_add_overflow` | `tests/phase_c_errors.rs`, `tests/phase_c_ffi.rs` |
| 22 | `err22_driver_extreme_args_via_loader`, `err22_ffi_extreme_args` | `tests/phase_c_errors.rs`, `tests/phase_c_ffi.rs` |
| 23 | `err23_inherited_sigpipe_ign_is_preserved` | `tests/phase_c_errors.rs` |
| 24 | `err24_tight_address_space_limit_does_not_abort` | `tests/phase_c_errors.rs` |
| generic (null/zero/oversized/one-past/enum-like) | `generic_so_main_rejects_identically`, `generic_zero_and_oversized_input`, `generic_one_step_past_every_range`, `generic_ffi_no_invalid_variant_exists`, `generic_ffi_repeated_calls_are_stateless`, `generic_ffi_interleaved_libraries`, `generic_ffi_dev_profile_overflow_checks`, `generic_ffi_no_extra_symbols_resolve` | `tests/phase_c_errors.rs`, `tests/phase_c_ffi.rs` |
