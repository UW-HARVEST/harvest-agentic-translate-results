# Differential verification log — `c_src/src/main.c` vs `translation/`

## What the C program is

The C source is written with ISO 646 digraphs / alternative operator spellings,
which the preprocessor and `<iso646.h>` expand to:

```c
#include <stdio.h>
#include <iso646.h>

void driver(int x, int y) {
    int result = x | ~y;     /* `bitor` == `|`, `compl` == `~` */
    printf("%d", result);
    puts("");                /* newline only */
}

int main() {
    int x = 0, y = 0;
    scanf("%d", &x);
    scanf("%d", &y);
    driver(x, y);
    return 0;
}
```

Observations that drive the whole test plan:

* Neither `scanf` return value is checked. A failed conversion leaves the
  variable at its initialiser `0`, and execution continues regardless.
* `main` always `return 0`, so **exit status is always 0** and **stderr is
  always empty**, for every input including malformed ones.
* Output is always exactly `<int>\n` — `printf("%d")` with no width/precision,
  then a bare `puts("")`.
* `%d` skips *all* leading whitespace, newlines included, so the two reads
  happily span lines (unlike `fgets`).

## How it was verified

* C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
* Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
* `translation/tests/differential.rs` spawns **both binaries as subprocesses**,
  feeds identical bytes on stdin, and asserts stdout, stderr and exit status all
  match byte for byte. 28 tests, none `#[ignore]`d, covering roughly 2,000
  distinct inputs. Nothing is loaded as a library.

## Mismatches found

**None.** Every enumerated input class produced identical stdout, stderr and
exit status. The pre-existing Rust translation already reproduced glibc's
`scanf("%d", …)` semantics, including the parts that look like bugs. The
sections below record the behaviours that *were* checked, because these are the
places a translation of this program normally goes wrong, and they are the
mismatches the suite would have caught.

### 1. `scanf` reads across newlines (would-be mismatch: line-oriented reading)

Translating `scanf("%d")` as "read a line, parse it" breaks on `"3\n4"`,
`"\n\n3\n4"`, `"3\r\n4\r\n"` and on whitespace-only input. The Rust reader skips
the full C whitespace set (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) before the
number and therefore reads across lines. Covered by
`scanf_reads_across_newlines`, `whitespace_only_input`.

### 2. Failed conversions must leave `0`, not abort or exit non-zero

`""`, `"abc"`, `"-"`, `"+"`, `"1 abc"` all reach a matching failure or EOF. The
C program prints `0 | ~0 == -1` (or `x | ~0`) and exits 0. A translation that
`unwrap()`s the parse, or exits 1 / prints a diagnostic to stderr, passes a
stdout-only check on the happy path and fails here — which is exactly why every
assertion covers all three streams. Covered by `empty_input`,
`first_conversion_fails`, `second_conversion_fails`,
`sign_then_eof_or_nondigit`.

### 3. Only **one** character is pushed back on a matching failure

glibc has a single character of pushback (`ungetc`). On `"--5 -1"` the first
`%d` consumes `'-'`, fails on the second `'-'`, and pushes back **only that
second `'-'`** — the first is consumed and lost. The second `%d` then reads
`-5`, so the output is `0 | ~(-5) == 4`, not `-1`. Likewise `"5-3"` yields
`5 | ~(-3) == 7` because the `'-'` that terminated the first number is pushed
back and becomes the sign of the second. A translation that re-scans a whole
token, or that pushes back the sign as well, diverges here. The Rust
`CStdin::ungetc` holds exactly one byte. Covered by
`matching_failure_pushback_is_one_character`, `trailing_garbage_stops_conversion`.

### 4. Overflow saturates then truncates — it does not wrap the digit string

glibc's `%d` converts with `strtol` semantics (saturating at `LONG_MAX` /
`LONG_MIN` on overflow, setting `ERANGE`) and then truncates that `long` to
`int` on assignment. So:

| input `x` | C value of `x` |
|---|---|
| `2147483648` (`INT_MAX+1`) | `-2147483648` |
| `4294967296` (`2^32`) | `0` |
| `9223372036854775808` (`LONG_MAX+1`) | `-1` (from truncating `LONG_MAX`) |
| `99999999999999999999` | `-1` |
| `-99999999999999999999` | `0` (from truncating `LONG_MIN`) |
| `340282366920938463463374607431768211456` | `-1` |

Naive `i32::from_str` (error → `0`) or `i64`-with-wrapping both give different
answers for the last four rows. The Rust `scanf_i32` accumulates into `i64`,
latches a `saturated` flag on overflow while still consuming the remaining
digits, resolves to `i64::MAX`/`i64::MIN`, and then truncates with `as i32`.
Verified against C for 24 boundary values, each paired with `y = -1` so that
`x | ~y == x | 0 == x` exposes the converted value of `x` verbatim. Covered by
`overflow_wraps_and_truncates_like_c`, `int_boundaries`.

*Latent, unobservable difference:* for the exact input `-9223372036854775808`
glibc converts precisely (no `ERANGE`) whereas the Rust code takes the saturated
path and yields `i64::MIN`. Both are `i64::MIN`, so after truncation to `i32`
both are `0`. Confirmed identical by test; noted here so a future reader does
not mistake it for a bug.

### 5. `%d` is base 10 — leading zeros are not octal, `0x` is not hex

`"0012"` is twelve; `"0x10"` converts as `0` and leaves `x10` in the stream, so
the second `%d` fails and `y` stays `0`. `"1e5"` converts as `1`. Covered by
`leading_zeros_are_decimal_not_octal`, `trailing_garbage_stops_conversion`.

### 6. Output formatting

`printf("%d", result)` + `puts("")` is exactly the digits, a `-` if negative,
and a single `\n`. No leading/trailing spaces, no padding, no second newline.
Every test compares stdout byte for byte, so any extra whitespace would fail.

### 7. Byte-level input handling

`\0` and bytes ≥ 0x80 are ordinary non-digit, non-whitespace characters that
cause a matching failure; U+00A0 (as UTF-8) is **not** C whitespace. Covered by
`nul_and_high_bytes_in_input`.

### 8. Buffered reading across chunk boundaries

The Rust reader refills in 4 KiB chunks, so inputs were constructed with
4093–9000 bytes of padding and with 50k–100k-digit numbers to make numbers,
whitespace runs and the pushback slot straddle a refill. Covered by
`buffer_boundary_whitespace_and_digits`, `very_long_digit_runs`.

### 9. I/O error paths

* stdin unreadable / immediately at EOF (`/dev/null`, closed fd) → both print
  `-1` and exit 0.
* stdout writes failing with `ENOSPC` (`/dev/full`) → the C program ignores
  `printf`/`puts` failures and exits 0 with empty stderr. The Rust code
  discards its write and flush results (`let _ = …`) rather than panicking, so
  it also exits 0. Covered by `stdin_at_immediate_eof_from_dev_null`,
  `stdout_write_failure_is_ignored`.

### 10. Ignored extras

`main` takes no parameters, so argv is irrelevant; input beyond the first two
numbers is never read. Covered by `argv_is_ignored`,
`extra_items_after_the_first_two_are_ignored`.

## Randomised differential sweeps

Three deterministic (fixed-seed xorshift) corpora, ~750 inputs, all matching:

* `deterministic_pseudo_random_numeric_pairs` — signed values up to ±5e9 with
  random whitespace separators.
* `deterministic_pseudo_random_junk` — strings over the alphabet `scanf`
  distinguishes: whitespace, digits, `+ - a b c x X e E . , / :`.
* `deterministic_pseudo_random_raw_bytes` — arbitrary bytes 0x00–0xFF.

An additional 900 out-of-suite shell-driven random cases (including 20-to-40
digit numbers and mixed separators) also matched.

## Status

* Both programs build with no errors.
* `cargo test` and `cargo test --release`: **28 passed, 0 failed, 0 ignored.**
* No test is disabled, skipped or `#[ignore]`d.
* `c_src/` is unmodified; only the generated `c_src/build/` tree was created.
