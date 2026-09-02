# Differential verification log — `c_src/src/main.c` → `translation/src/main.rs`

## What the program does

The C source is written with digraphs and `<iso646.h>` alternative operator
spellings, which de-obfuscates to:

```c
#include <stdio.h>
#include <iso646.h>

void driver(int x, int y) {
    int result = x | ~y;
    printf("%d", result);
    puts("");
}

int main() {
    int x = 0, y = 0;
    scanf("%d", &x);
    scanf("%d", &y);
    driver(x, y);
    return 0;
}
```

There are no `if` statements, no early `return`s, and no explicit error paths.
Every behavioural branch lives inside `scanf("%d", …)` and in the C integer
semantics of `|`, `~` and the `int` store. The return values of both `scanf`
calls are discarded, so a failed conversion is silent: the corresponding
variable keeps its initialiser `0`, and the process still exits `0`.

Consequences that tests must pin down:

- On total input failure both variables stay `0`, so the output is
  `0 | ~0 == -1`.
- Whenever `y` ends up `0` (including every second-conversion failure) the
  output is `x | -1 == -1` regardless of `x`. Cases that are meant to exercise
  `x` parsing therefore **must** supply a non-zero `y`, otherwise they pass
  vacuously. The test suite does this deliberately in
  `long_range_saturation` and `out_of_int_range_truncates`.

## Build and run commands (Phase A)

| | command | binary |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` | `translation/target/release/driver` |

Both build with no errors and no warnings. The Rust crate needed no source
changes to compile.

## Mismatches found

**None.** Across every enumerated input class and roughly 1,400 additional
pseudo-random cases (`randomized_integer_pairs`,
`randomized_garbage_streams`, `randomized_long_digit_strings`), plus a
separate throwaway 4,800-case fuzz run performed during investigation, the
Rust binary produced byte-identical stdout, byte-identical stderr (always
empty) and exit status `0`, matching the C binary in every case.

The only failure encountered during this work was a **defect in my own test**,
not in the translation: the smoke test `both_binaries_run` asserted the
reference output for input `1 2` was `-1\n`, when `1 | ~2 == 1 | -3 == -3`.
Fixed by correcting the expected literal to `-3\n`. The differential
assertions for that same input passed throughout — the Rust and C outputs
agreed with each other before and after the fix.

## Traps that were checked and confirmed already correct

These are the places where a naive translation diverges. Each is covered by a
named test; all matched the C.

1. **`scanf` skips whitespace across newlines.** `%d` consumes any run of
   `isspace` bytes (space, `\t`, `\n`, `\v`, `\f`, `\r`) before converting, so
   `"5\n7"`, `"5\r\n7"` and `"\v5\f7"` all parse two integers. A `fgets` or
   line-oriented reader would only see one. Covered by
   `two_items_separators`.

2. **Failure leaves the variable untouched, and exit status stays 0.** A Rust
   translation that treats a parse error as a fatal error (`expect`,
   `process::exit(1)`, writing to stderr) would diverge on stdout, stderr and
   the exit code simultaneously. Covered by `first_conversion_fails`,
   `second_conversion_fails`, `no_input_at_all`.

3. **Partial match leaves the offending byte queued for the next `scanf`.**
   `"0x10 5"` converts `x = 0`, stops at `x`, and then the second `scanf` fails
   on that same `x`, so `y` stays `0` (output `-1`, not `16` and not `5`).
   Likewise `"1,2"`, `"1.5 2.5"`, `"1e5 2"`, `"12abc 34"`. Covered by
   `partial_matches_and_queued_bytes`.

4. **A lone sign is a matching failure, but glibc still consumes it.** `"-"`,
   `"+"`, `"--5"`, `"-+5"` all fail conversion. `"- 5"` fails the first
   conversion at the space and then succeeds on `5` for `y`. Covered by
   `first_conversion_fails` and `signs_and_leading_zeros`.

5. **Out-of-`int` values truncate rather than saturate.** glibc converts the
   digit string with `strtol` (64-bit `long` here) and the `%d` store narrows
   to `int`, so `"1234 2147483648"` yields `y == -2147483648` and prints
   `2147483647`. Covered by `out_of_int_range_truncates`. Rust's
   `str::parse::<i32>()` would reject these inputs outright — the translation
   correctly reproduces the C by converting to `i64` and casting with `as i32`.

6. **Out-of-`long` values saturate at `LONG_MAX`/`LONG_MIN`, and the
   *saturated* value is then narrowed.** `"5 99999999999999999999"` saturates
   `y` to `0x7FFFFFFFFFFFFFFF`, narrows to `-1`, so `~y == 0` and the output is
   `5`. The negative form `"-99999999999999999999 3"` saturates to `LONG_MIN`,
   narrows to `0`, and prints `0 | ~3 == -4`. A translation that saturated
   directly in `i32`, or that wrapped the full big-integer value, gets both of
   these wrong. Covered by `long_range_saturation`.

7. **`printf("%d", …)` then `puts("")`.** Exactly one newline, no space, no
   leading text. A `println!("{}", result)` happens to match; a
   `println!("result = {}", …)` or an extra flush-newline would not. Covered
   everywhere, byte for byte.

8. **Buffered reading must not lose tokens at a refill boundary.** The Rust
   reader refills a 4,096-byte buffer, so a digit run or sign that straddles
   byte 4,096 is a real risk. Covered by `buffer_boundary_cases` (digit run
   spanning 4,095 zeros, 4,096 newlines of leading whitespace, a sign landing
   on the boundary, 10,000-digit tokens, a 100,000-byte stream).

9. **Bytes that are not C whitespace must not be skipped.** UTF-8 NBSP
   (`\xc2\xa0`), `\x7f`, NUL and bytes ≥ `0x80` are all conversion-stopping,
   not whitespace. Covered by `binary_and_non_ascii_bytes`.

10. **`main()` takes no parameters, so `argv` cannot change anything.** A
    translation that adds `--help` or positional-argument handling would
    diverge. Covered by `argv_is_ignored`.

## Test inventory

`translation/tests/differential.rs` — 19 `#[test]` functions, none `#[ignore]`d,
none skipped. Every assertion compares all three of stdout, stderr and exit
status between the two subprocesses. Neither the Rust crate nor the C code is
linked as a library; both are spawned as processes with piped stdio, and the
C binary is built on demand via CMake if `c_src/build/driver` is absent.

Status: `cargo test` and `cargo test --release` both pass, 19/19.
