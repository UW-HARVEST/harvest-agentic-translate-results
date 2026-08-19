# ERRORS.md — error-surface table (Phase A → verified in Phase C)

Derived mechanically from `c_src/src/main.c` (62 lines). Every rejection /
error / failure path the C code can take is one row. Grep evidence:

```
$ grep -n -E "return|goto|assert|exit|if *\(|scanf" c_src/src/main.c
31:    if (x != 1) {            34:        goto fail;
37:    if (y != 2) {            40:        goto fail;
43:    if (z != 3) {            46:        goto fail;
50:    return result;   (success)
52: fail:                53:  printf("Operation failed\n");   54: return result;
59:    scanf("%d %d %d", &x, &y, &z);      62:    return 0;
$ grep -c assert c_src/src/main.c   -> 0
$ grep -c malloc c_src/src/main.c   -> 0
$ grep -c 'exit('  c_src/src/main.c -> 0
```

There are **no** `assert`s, no allocation, no `NULL` checks, no `errno`
inspection and no non-zero process exit codes in this program: `main` always
`return 0;`. The error surface consists of (a) the three explicit
validation branches of `multi_stage` plus its shared `fail:` epilogue, and
(b) every way the ignored `scanf` call can fail to convert, which leaves the
target variables at their previous values (`x = 0`, `y = 123`, `z = 0`) and
thereby feeds the validation branches.

`R1` = `Error: x != 1`, `R2` = `Error: x == 1 but y != 2`,
`R3` = `Error: x == 1 and y == 2, but z != 3`, `F` = `Operation failed`.
"expected C result" lists the complete stdout and the process exit status.

| # | function | trigger (exact invalid input/condition) | expected C result | ✔ |
|---|----------|------------------------------------------|-------------------|---|
| E1 | `multi_stage` L31 | `x != 1` (e.g. stdin `0 2 3`) | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E2 | `multi_stage` L37 | `x == 1` but `y != 2` (stdin `1 5 3`) | `R2\nF\nResult: 2\n`, exit 0 | [x] |
| E3 | `multi_stage` L43 | `x == 1`, `y == 2`, but `z != 3` (stdin `1 2 9`) | `R3\nF\nResult: 3\n`, exit 0 | [x] |
| E4 | `multi_stage` L52 `fail:` | any of E1–E3 reached → shared epilogue prints `Operation failed` **after** the specific message, and returns the stage code (never 0) | second line is always `F`; `Result:` is 1/2/3 | [x] |
| E5 | `main` L59 `scanf` | **input failure, 0 conversions**: empty stdin (immediate EOF) → `scanf` returns `EOF`, ignored; `x=0,y=123,z=0` | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E6 | `main` L59 `scanf` | **input failure, 0 conversions**: stdin is whitespace only (`" \t\n"`) → EOF while skipping whitespace | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E7 | `main` L59 `scanf` | **matching failure on 1st `%d`**: first non-space byte is not `[0-9+-]` (`"abc"`, `"."`, `","`, `"x 2 3"`) | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E8 | `main` L59 `scanf` | **matching failure on 1st `%d`**: sign with no digit (`"-"`, `"+"`, `"- 1 2"`, `"+x"`) | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E9 | `main` L59 `scanf` | **1 conversion then input failure**: `"1"` (EOF after 1st) → `y` keeps 123, `z` keeps 0 | `R2\nF\nResult: 2\n`, exit 0 | [x] |
| E10 | `main` L59 `scanf` | **1 conversion then matching failure**: `"1 x 3"`, `"1 - 3"`, `"1 . 3"` → `y` keeps 123 | `R2\nF\nResult: 2\n`, exit 0 | [x] |
| E11 | `main` L59 `scanf` | **2 conversions then input failure**: `"1 2"` (EOF after 2nd) → `z` keeps 0 | `R3\nF\nResult: 3\n`, exit 0 | [x] |
| E12 | `main` L59 `scanf` | **2 conversions then matching failure**: `"1 2 x"`, `"1 2 -"`, `"1 2 +"` → `z` keeps 0 | `R3\nF\nResult: 3\n`, exit 0 | [x] |
| E13 | `main` L59 `scanf` | **1 conversion, 2nd fails, but x != 1**: `"7 x 3"` → E1 wins over E2 (order of checks) | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E14 | `main` L59 `scanf` | **out-of-range positive**: value `> LONG_MAX` (`"99999999999999999999"`, `"18446744073709551616"`, 100 000-digit number) → glibc `strtol` saturates to `LONG_MAX`, `ERANGE` ignored, narrowed to `int` = `-1` | `-1` used as x/y/z (e.g. `"99999999999999999999 2 3"` → `R1\nF\nResult: 1\n`) | [x] |
| E15 | `main` L59 `scanf` | **out-of-range negative**: value `< LONG_MIN` → saturates to `LONG_MIN`, narrowed to `int` = `0` | `0` used as x/y/z | [x] |
| E16 | `main` L59 `scanf` | **out-of-`int`, in-`long` range**: `"2147483648"` → `(int)` narrowing = `-2147483648`; `"4294967297"` → `1`; `"-4294967295"` → `1` | narrowed value used (a value that narrows to 1/2/3 *passes* the stage) | [x] |
| E17 | `main` L59 `scanf` | **non-decimal prefix**: `"0x10 2 3"` (`%d` is base 10) → converts `0`, leaves `x` at 0, then `x` stops the scan at `'x'` so `y`/`z` keep 123/0 | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E18 | `main` L59 `scanf` | **float-looking input** `"1.5 2.5 3.5"` → 1st converts `1`, 2nd fails at `'.'` | `R2\nF\nResult: 2\n`, exit 0 | [x] |
| E19 | `main` L59 `scanf` | **NUL / non-ASCII byte** as first non-space byte (`"\0 1 2"`, `"\xff 1 2"`, `"\xc3\xa9"`) → matching failure (byte is neither space nor digit/sign) | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E20 | `main` L59 `scanf` | **stdin closed** (`<&-`, fd 0 not open) → `read` fails `EBADF`, `scanf` reports input failure, nothing assigned | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E21 | `main` L59 `scanf` | **stdin is a directory** (`< /tmp`) → `read` fails `EISDIR` → input failure | `R1\nF\nResult: 1\n`, exit 0 | [x] |
| E22 | `main` L59 `scanf` | `scanf`'s return value is **never checked** — a failed/partial scan produces *no* diagnostic of its own and never changes the exit status | exit status always 0 | [x] |
| E23 | `printf` (all sites) | **write error ignored**: stdout redirected to `/dev/full` (`ENOSPC`) | no output, exit 0 | [x] |
| E24 | `printf` (all sites) | **broken stdout pipe**: reader closed → C process has the default `SIGPIPE` disposition and is **killed by signal 13** | no output, terminated by SIGPIPE (`128+13` = 141 as seen by the shell) | [x] |
| E25 | `main` | **extra/garbage `argv`** (`main()` takes no parameters, argv never inspected) | argv ignored, behaviour identical to no args | [x] |

All 25 rows have a differential test in `tests/errors.rs` (E24 in
`tests/errors.rs::e24_broken_stdout_pipe_sigpipe`), each asserting that the C
and Rust binaries produce byte-identical stdout, byte-identical stderr and the
identical wait status (including "killed by signal 13" for E24).

## Fixes applied to the Rust side as a result of this verification

Two real divergences were found and fixed in `src/main.rs` (the C code was never
touched):

1. **E24 — SIGPIPE disposition.** The Rust standard library sets `SIGPIPE` to
   `SIG_IGN` before calling `main`, so the translation silently ignored the
   `EPIPE` from the failing `printf` and exited **0**, while the C program keeps
   the default disposition and is **killed by signal 13** (status 141 as seen by
   the shell). `main` now calls `restore_default_sigpipe()` as its very first
   statement. Covered by `tests/errors.rs::e24_broken_stdout_pipe_sigpipe`.

2. **Stdin consumption / file offset left behind** (not an error row, but the
   same class of invisible difference; see `CONFIGS.md` rows C26–C28). The
   translation used `std::io::stdin()`, whose `BufReader` reads 8 KiB and never
   gives anything back, whereas glibc buffers `st_blksize` (4096 here) bytes and,
   at `exit()`, `_IO_cleanup` → `_IO_SYNC` returns the unconsumed read-ahead to a
   *seekable* descriptor. This is observable by the next reader of the same
   descriptor:

   | scenario (`{ driver; cat; } < input`) | C leftover | Rust before | Rust after |
   |---|---|---|---|
   | 31-byte file `1 2 3 REST-OF-DATA-AFTER-TOKENS` | ` REST-OF-DATA-AFTER-TOKENS` | *(nothing)* | ` REST-OF-DATA-AFTER-TOKENS` |
   | 20 011-byte file | 20 006 bytes (offset 5) | 11 819 bytes (offset 8192) | 20 006 bytes |
   | 20 011-byte **pipe** | 15 915 bytes (consumed 4096) | 11 819 bytes (consumed 8192) | 15 915 bytes |
   | file with 20 000 leading spaces | 5 bytes (offset 20 005) | 0 bytes (offset 20 010) | 5 bytes |

   `src/main.rs` now models glibc's stream (`CStdin`): buffer size from
   `st_blksize` with a `BUFSIZ` fallback, one `read()` per underflow, pushback by
   rewinding the buffer index, and an exit-time `sync()` that seeks a seekable
   descriptor back to the stream's logical position. Covered by
   `tests/stdin_offset.rs`.

Everything else in this table matched the C implementation on the first run,
including all three stage rejections, every `scanf` failure mode, both
saturation directions, the `int` narrowing, `/dev/full` write errors and the
ignored `argv`.
