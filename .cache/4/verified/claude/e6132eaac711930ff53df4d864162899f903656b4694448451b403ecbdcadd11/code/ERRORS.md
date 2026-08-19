# ERRORS.md — Phase A: the error-surface table

## How this table was derived

`c_src/src/main.c` was grepped for every explicit rejection construct.  The
result is **zero** of them:

| construct | occurrences in `c_src/src/main.c` |
|-----------|-----------------------------------|
| `return -1`, `return NULL`, error enums | 0 |
| `assert` | 0 |
| `RETURN_ERROR`-style macro | 0 |
| `if (...)` range / null / bounds check | 0 |
| `switch`, `goto` | 0 |
| `errno`, `perror`, `fprintf(stderr, ...)` | 0 |
| `exit()`, `abort()` | 0 |
| `#if` / `#ifdef` / `#ifndef` | 0 |
| use of `argc` / `argv` / `getenv` | 0 |

The C code checks **nothing**: `scanf`'s return value is discarded, `div`'s
arguments are unvalidated and `printf`'s return value is discarded.  That does
*not* mean there is no error surface — it means the whole error surface is
**implicit**, and lives in four places that this program can reach:

1. the failure modes of the `"%d %d"` conversion inside glibc's `vfscanf`
   (input failure, matching failure, `strtol` range clamping, `long`→`int`
   truncation), whose effect is "the variable keeps the value it already had";
2. the **undefined behaviour** in `div(x, y)` — glibc compiles `div` to
   `mov %edi,%eax ; cltd ; idiv %esi`, so `y == 0` and `INT_MIN / -1` execute a
   trapping `idiv` and raise a *hardware* `SIGFPE`;
3. the **ignored** `printf` write error, which must not change the exit status
   (Rust's `print!` macro would panic — that is a divergence, not a fix);
4. the **ignored** `lseek` error inside `_IO_cleanup()`'s exit-time rewind of
   stdin, which decides how much of a shared stdin the program leaves behind.

Each distinct trigger below is one row.  "expected C result" is the *measured*
behaviour of `c_src/build/driver` on x86-64 / glibc 2.34, not a guess.

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `scanf` conv #1 | **input failure**: EOF immediately (`""`) | conv #1 aborts scanf, `x` and `y` both keep `1` → `quotient: 1, remainder: 0`, exit 0 | `err_e1_eof_immediately` | [x] |
| E2 | `scanf` conv #1 | **input failure**: only whitespace then EOF (`"  \t\n\v\f\r "`) | as E1 → `quotient: 1, remainder: 0`, exit 0 | `err_e2_whitespace_only` | [x] |
| E3 | `scanf` conv #1 | **matching failure**: first non-ws byte cannot start an int (`"q 7"`, `"."`, `"/"`, `"x"`, `","`) | scanf returns 0 without touching `x`; conv #2 never runs → `quotient: 1, remainder: 0`, exit 0 | `err_e3_matching_failure_first` | [x] |
| E4 | `scanf` conv #1 | **matching failure**: sign then EOF (`"-"`, `"+"`) | `x`,`y` stay `1` → `quotient: 1, remainder: 0`, exit 0 | `err_e4_sign_then_eof` | [x] |
| E5 | `scanf` conv #1 | **matching failure**: sign then non-digit (`"- 5"`, `"+q"`, `"--5 3"`) | `x`,`y` stay `1` → `quotient: 1, remainder: 0`, exit 0 | `err_e5_sign_then_nondigit` | [x] |
| E6 | `scanf` conv #1 | **matching failure**: NUL byte first (`"\0" "5 2"`) | NUL is neither space nor digit → as E3 → `quotient: 1, remainder: 0`, exit 0 | `err_e6_nul_byte_first` | [x] |
| E7 | `scanf` conv #1 | **matching failure**: byte ≥ 0x80 first (`"\xff 3"`, `"\x80"`) | as E3 → `quotient: 1, remainder: 0`, exit 0 | `err_e7_high_byte_first` | [x] |
| E8 | `scanf` conv #2 | **input failure**: EOF after `x` (`"7"`, `"7 "`, `"7\n"`) | `x = 7`, `y` keeps `1` → `quotient: 7, remainder: 0`, exit 0 | `err_e8_eof_after_x` | [x] |
| E9 | `scanf` conv #2 | **matching failure**: non-digit after `x` (`"7 q"`, `"7 ."`, `"7,2"`) | `x = 7`, `y` keeps `1` → `quotient: 7, remainder: 0`, exit 0 | `err_e9_matching_failure_second` | [x] |
| E10 | `scanf` conv #2 | **matching failure**: sign then EOF after `x` (`"7 -"`, `"7 +"`) | `y` keeps `1` → `quotient: 7, remainder: 0`, exit 0 | `err_e10_second_sign_then_eof` | [x] |
| E11 | `scanf` conv #2 | **matching failure**: sign then non-digit after `x` (`"7 -q"`, `"7 - 2"`) | `y` keeps `1` → `quotient: 7, remainder: 0`, exit 0 | `err_e11_second_sign_then_nondigit` | [x] |
| E12 | `scanf` conv #1 | **`strtol` positive range clamp**: magnitude > `LONG_MAX` (`"9223372036854775808 1"`, `"9"×5000`) | value clamps to `LONG_MAX`, then `(int)LONG_MAX == -1` → `x = -1` | `err_e12_positive_long_clamp` | [x] |
| E13 | `scanf` conv #1 | **`strtol` negative range clamp**: magnitude > `LONG_MAX+1` (`"-9223372036854775809 1"`, `"-"+"9"×5000`) | clamps to `LONG_MIN`, then `(int)LONG_MIN == 0` → `x = 0` | `err_e13_negative_long_clamp` | [x] |
| E14 | `scanf` conv #1/#2 | **`long`→`int` truncation**: in-`long`-range but out-of-`int`-range (`"4294967296"`, `"2147483648"`, `"-2147483649"`, `"4294967297"`) | low 32 bits are stored: `0`, `-2147483648`, `2147483647`, `1` | `err_e14_long_to_int_truncation` | [x] |
| E15 | `scanf` conv #1 | **base-prefix quirk**: `"0x10"` — glibc's prefix probe consumes the `0` but `%d` pins base 10 | `x = 0`, `x` leaves `x10` in the stream so conv #2 hits E3 and `y` stays `1` → `quotient: 0, remainder: 0`, exit 0 | `err_e15_hex_prefix_quirk` | [x] |
| E16 | `div` | **division by zero**: `y == 0` (`"5 0"`, `"0 0"`, `"-3 0"`, `"5 -0"`, `"5 +0"`) | trapping `idiv` → killed by **SIGFPE (8)**, core-dump flag set, **no stdout output at all** | `err_e16_division_by_zero` | [x] |
| E17 | `div` | **signed-overflow divide**: `x == INT_MIN && y == -1` (`"-2147483648 -1"`) | trapping `idiv` → killed by **SIGFPE (8)**, no output | `err_e17_int_min_over_minus_one` | [x] |
| E18 | `div` | E16 reached with `SIGFPE` **ignored** and/or **blocked** by the parent | the kernel force-delivers synchronous faults: still killed by SIGFPE (8) | `err_e18_sigfpe_ignored_or_blocked` | [x] |
| E19 | `printf` | **write fails with `EPIPE`** (reader closed) and `SIGPIPE` is `SIG_DFL` | killed by **SIGPIPE (13)**, no output | `err_e19_epipe_sigpipe_default` | [x] |
| E20 | `printf` | **write fails with `EPIPE`** and `SIGPIPE` is `SIG_IGN` | error is discarded, `main` still `return 0` → **exit 0**, nothing on stderr | `err_e20_epipe_sigpipe_ignored` | [x] |
| E21 | `printf` | **write fails with `ENOSPC`** (stdout = `/dev/full`) | error discarded → **exit 0**, nothing on stderr | `err_e21_enospc_dev_full` | [x] |
| E22 | `printf` | **write fails with `EBADF`** (fd 1 closed before `exec`) | error discarded → **exit 0**, nothing on stderr | `err_e22_ebadf_stdout_closed` | [x] |
| E23 | `scanf` | **stdin read fails with `EBADF`** (fd 0 closed before `exec`) | read error ⇒ conv #1 input failure ⇒ E1 → `quotient: 1, remainder: 0`, exit 0 | `err_e23_ebadf_stdin_closed` | [x] |
| E31 | `_IO_new_file_sync` | **`lseek` on stdin fails with `ESPIPE`** (unseekable stdin: pipe / tty) — reached from `_IO_cleanup()` at exit | the error is **swallowed**: exit status stays 0 and the whole `st_blksize` block that was read stays consumed | `err_e31_espipe_swallowed`, `r29_stdin_residual_pipe` | [x] |
| E32 | `_IO_cleanup` | the exit-time stdin rewind is **not reached at all** because the process died from `SIGFPE`/`SIGPIPE` | the descriptor is left wherever the block read put it — the rewind must NOT happen | `err_e32_no_rewind_when_signalled`, `r28_stdin_residual_seekable` | [x] |

## Generic FFI boundary cases (required even though the C has no such checks)

| # | trigger | expected C result | test | ✔ |
|---|---------|-------------------|------|---|
| E24 | zero-length stdin (`read` returns 0 on the first call) | same as E1 | `err_e24_zero_length_input` | [x] |
| E25 | oversized stdin: 1 MiB of digits before any separator | glibc grows its scratch buffer, `strtol` clamps ⇒ E12 | `err_e25_oversized_input` | [x] |
| E26 | one step past the valid `int` range on each side, both operands: `2147483648`, `-2147483649` | wraps by truncation ⇒ E14, **never** an error | `err_e26_one_past_int_range` | [x] |
| E27 | one step past the valid `long` range on each side: `9223372036854775808`, `-9223372036854775809` | clamps ⇒ E12 / E13 | `err_e27_one_past_long_range` | [x] |
| E28 | core-dump flag of the fatal signal, incl. with `SIGFPE` ignored / blocked | `WCOREDUMP` set, `WTERMSIG == 8`. This is what distinguishes a **hardware** `#DE` from a `raise(SIGFPE)` look-alike: `raise` on an ignored or blocked signal returns harmlessly, whereas the kernel force-delivers a synchronous fault. | `err_e28_core_dump_flag` | [x] |
| E30 | out-of-range "enum" value / null pointer / bad length across the FFI boundary | **There is no such parameter to corrupt**: `driver_main` takes *no arguments* (`int main()` / `extern "C" fn driver_main() -> c_int`), and the C source declares no `enum`, no pointer parameter and no length. Tested two ways anyway: (a) `driver_main` is called through a deliberately **mis-declared six-integer-argument** fn pointer stuffed with `u64::MAX`, `0x8000_0000_0000_0000`, `0xDEAD_BEEF…` etc., and both libraries must be bit-identical to the zero-argument call; (b) the real "arbitrary bit pattern from outside" surface is the *stdin byte stream*, exhausted over all 256 single-byte values and a 22×22 two-byte grid, and fuzzed over the full 0–255 alphabet. | `err_e30_garbage_argument_registers`, `err_e29_all_single_bytes`, `r19_arbitrary_bytes` | [x] |
| E29 | every one of the 256 possible single-byte inputs, and every 2-byte input drawn from a hostile alphabet | whichever of E1–E15 applies | `err_e29_all_single_bytes` | [x] |

## Deliberately NOT "fixed" in the Rust translation

* `scanf`'s ignored return value (rows E1–E11) — a failed conversion silently
  leaves `1` behind, and a failure on conversion #1 suppresses conversion #2.
* The `strtol` clamp + `long`→`int` truncation (E12–E14) — the Rust accumulates
  into a saturating `u64`, clamps to `i64::MAX`/`i64::MIN`, then casts to `i32`.
* The `0x` prefix quirk (E15).
* The `SIGFPE` UB (E16–E18) — reproduced with a real `cdq; idiv` via
  `core::arch::asm!`, *not* with `raise()`, so that the fault is still delivered
  when `SIGFPE` is ignored or blocked and the `si_code`/core-dump flag match.
* The ignored `printf` error (E19–E22) — `write_all` + discarded `Result` instead
  of `print!`, and the inherited `SIGPIPE` disposition is captured in an
  `.init_array` constructor and restored, because Rust's runtime forces
  `SIG_IGN` before `main` and the C program never does.
* The stdio buffering side effects on a *shared* stdin (E31–E32) — stdin is read
  in `st_blksize`-sized blocks straight off fd 0 and rewound with `lseek` after
  the stdout flush, reproducing `_IO_file_doallocate` /
  `_IO_new_file_underflow` / `_IO_new_file_sync`.  Rust's `io::stdin()` would
  over-read (8 KiB `BufReader`) and never rewind, so `{ ./driver; cat; } < in`
  would print different bytes.
