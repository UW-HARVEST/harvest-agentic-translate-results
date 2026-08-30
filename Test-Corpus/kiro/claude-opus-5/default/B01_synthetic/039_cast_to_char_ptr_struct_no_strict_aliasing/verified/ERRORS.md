# Differential verification of the C → Rust translation

C ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust translation: `translation/src/main.rs`, built to `translation/target/release/driver`.

Run commands:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

Nothing under `c_src/` was modified. CMake writes only into the out-of-tree
`c_src/build/` directory; `c_src/src/main.c` and `c_src/CMakeLists.txt` are
byte-for-byte the originals.

## What the program does

`main` reads one integer with `scanf("%d", &x)` (starting from `x = 0`), passes
it to `driver`, and always `return 0`. `driver` fills a
`{ int floors; int bedrooms; double bathrooms; }` with `{x, 3, 2.0}`, `memcpy`s
it into a `char raw[sizeof(house)]`, and `print_hex` dumps those bytes as
`%02x` each followed by one `"\n"`.

On the x86-64 SysV ABI the struct is 16 bytes with fields at offsets 0, 4 and 8
and **no padding**, so every dumped byte is determined by the input. The output
is therefore always exactly 33 bytes: 32 lowercase hex digits plus a newline.
`stderr` is always empty and the exit status is always 0 — the program has no
error path that changes either. That was confirmed empirically rather than
assumed, including with stdin unreadable and stdout closed.

## Mismatches found

### 1. Vertical tab (0x0b) was not treated as leading whitespace

**Status:** found and fixed.

`scanf`'s `%d` directive skips leading whitespace using C's `isspace`, which in
the "C" locale is `' '` plus the control range `0x09..=0x0d`, i.e. `\t \n \v \f
\r`. The Rust translation skipped whitespace with
`(input[i] as char).is_ascii_whitespace()`, and Rust's `is_ascii_whitespace`
**deliberately excludes the vertical tab** (its documentation calls this out).

Consequence: for input `"\x0b42"` the C program skipped the `\v`, converted 42
and printed `floors = 42`; the Rust program saw a byte that was neither
whitespace, a sign nor a digit, treated it as a matching failure, left `x = 0`
and printed `floors = 0`.

```
input: "\x0b42"
C   : 2a000000030000000000000000000040
Rust: 00000000030000000000000000000040
       ^^^^^^^^ floors = 0 instead of 42
```

Fix: added an explicit `is_c_space` helper implementing C's set
(`b == b' ' || (0x09..=0x0d).contains(&b)`) and used it for the skip, instead of
Rust's `is_ascii_whitespace`.

Regression coverage: `every_leading_byte_agrees` now runs all 256 possible
leading bytes through both programs, so any future divergence in the whitespace
classification is caught rather than depending on someone thinking of `\v`
again. `whitespace_only_input` and `leading_whitespace_is_skipped` also include
`\v` and `\f` explicitly.

## Behaviours that look like bugs and were deliberately preserved

These were verified against the C binary and are reproduced, not "fixed".

- **`scanf` failure is silently ignored.** `main` never checks the return value,
  so on EOF or a matching failure `x` keeps its initializer `0` and the program
  prints `floors = 0` and exits 0. Empty input, whitespace-only input, `"abc"`,
  `"-"`, `".5"`, `"- 5"` and binary garbage all produce the `floors = 0` output
  with exit status 0 — not an error.

- **Overflow saturates and is then truncated.** glibc converts `%d` into a
  `long` (saturating at `LONG_MAX`/`LONG_MIN`, as `strtol` does) and stores the
  result through an `int *`, discarding the high 32 bits. So:

  | input | `long` value | stored `int` |
  |---|---|---|
  | `2147483648` | 2147483648 | `-2147483648` |
  | `4294967296` | 4294967296 | `0` |
  | `9223372036854775807` | `LONG_MAX` | `-1` |
  | `99999999999999999999` | saturates to `LONG_MAX` | `-1` |
  | `-99999999999999999999` | saturates to `LONG_MIN` | `0` |

  Confirmed against the C binary for all of the above, plus digit runs up to
  5000 characters long.

- **`scanf` reads across newlines.** Unlike `fgets`, `%d` skips any amount of
  whitespace including newlines, so `"\n\n\n\n42\n"` yields 42. Covered by
  `scanf_reads_across_newlines`.

- **Only the first token is consumed.** `"1 2"`, `"42abc"`, `"3.9"`, `"1e5"` and
  `"0x10"` all convert just the leading decimal integer (`1`, `42`, `3`, `1`,
  `0`) and ignore the rest; `%d` is base 10, so `0x10` stops at the `0`.

- **Arguments are ignored.** `int main()` takes no parameters, so `--help` and
  friends change nothing.

## Test inventory

`translation/tests/differential.rs` spawns **both** binaries as subprocesses,
feeds them the same stdin, and asserts stdout, stderr and exit status all match.
The Rust code is never loaded as a library. 28 tests, none `#[ignore]`d,
skipped or disabled.

Input classes covered: empty stdin; whitespace-only; a single value with and
without a trailing newline; `+`/`-` signs; every kind of leading whitespace
including `\v` and `\f`; all 256 leading bytes; all 256 bytes as a digit-run
terminator; all 256 bytes immediately after a sign; matching failures; trailing
unconsumed text; leading zeros; the exact `int` boundaries; `long`-to-`int`
truncation; overflow saturation past `LONG_MAX`/`LONG_MIN`; digit runs of 18 to
5000 characters; embedded NULs and non-UTF-8 bytes; a 64 KiB input the program
never finishes reading; stdin as `/dev/null`; stdin as a directory (read fails
with `EISDIR`); stdout closed before the write; stdout's reader exiting early
(SIGPIPE); command-line arguments; and a deterministic 1500-case randomized
sweep over 32-bit, 64-bit and oversized decimal strings.
