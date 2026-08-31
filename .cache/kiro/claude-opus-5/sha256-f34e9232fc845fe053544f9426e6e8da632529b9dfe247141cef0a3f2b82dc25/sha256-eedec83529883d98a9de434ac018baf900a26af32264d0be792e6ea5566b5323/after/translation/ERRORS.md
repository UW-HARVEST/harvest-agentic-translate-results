# ERRORS.md — differential verification of the C → Rust translation

Scope: `c_src/src/main.c` (a single translation unit) vs `translation/src/main.rs`.
Compared by execution only: both binaries run as subprocesses on identical
stdin, and stdout, stderr and exit status are diffed byte for byte.

## Outcome

**No mismatches were found.** Every enumerated input class, plus ~700
pseudo-random inputs, produced byte-identical stdout, byte-identical stderr and
an identical exit status. No change to `translation/src/main.rs` was needed, and
nothing in `c_src/` was modified (only the out-of-tree `c_src/build/` directory
was created, as the build instructions direct).

Because there is nothing to record under "mismatch found and fixed", the rest of
this document records the audit instead: for each place where this program
*could* diverge, what the C does, what the Rust does, and the evidence that the
test suite would have caught a divergence. That evidence comes from mutation
testing — deliberately breaking the Rust and confirming the suite fails.

## What the program actually computes

```
main:      read up to 100 ints with scanf("%d"), stopping at the first failure
call_fma:  if len == 0 -> 0
           else out[i] = ones[i] * data[i] + zeros[i]  ==  1 * data[i] + 0
           return out[len-1]
printf("%d\n", result)
```

So the observable behaviour is: **print the last integer successfully read, or
`0` if none were read; always exit 0; never write to stderr.** `out[0] = 0`
before the `fma_array` call is dead for every `len >= 1`, since `fma_array`
overwrites all of `out`. The uninitialised `int data[100]` is never read beyond
index `i-1`, so it has no observable effect: when `i == 0` the `len == 0` early
return means `data` is not touched at all.

## Divergence risks audited

### 1. `scanf` skips whitespace across newlines (not line-oriented)

C: one `scanf("%d")` consumes arbitrary leading whitespace, newlines included,
so line structure is irrelevant. A translation written with `read_line` +
`trim().parse()` would break on `"1\n\n\n2"`, on `"1 2 3"` on one line, and on
whitespace-only input.

Rust: `scanf_d` loops `getc` while `is_space`, matching glibc's `isspace` set
exactly — space, `\t`, `\n`, `\x0b` (VT), `\x0c` (FF), `\r`.

Verified: `scanf_reads_across_newlines`, `multiple_items_various_separators`,
`whitespace_only_input`. Mutations that narrowed the whitespace set were caught:
dropping VT/FF failed 2 tests; skipping only `' '` failed 5.

### 2. Integer conversion truncates, it does not saturate

C: glibc's `%d` converts with `strtol` semantics into a `long` (saturating at
`LONG_MAX`/`LONG_MIN` on overflow, with `ERANGE`) and then **assigns that
`long` to an `int`**, truncating to the low 32 bits. The consequences are
counter-intuitive and were confirmed against the real binary:

| input | C prints | why |
|---|---|---|
| `2147483648` (`INT_MAX+1`) | `-2147483648` | fits a `long`, truncated to `int` |
| `4294967296` (`2^32`) | `0` | low 32 bits are zero |
| `9223372036854775807` (`LONG_MAX`) | `-1` | low 32 bits are all ones |
| `9223372036854775808` (`LONG_MAX+1`) | `-1` | saturates to `LONG_MAX`, then truncates |
| `-9223372036854775809` | `0` | saturates to `LONG_MIN`, then truncates |
| 400 `9`s | `-1` | saturates to `LONG_MAX`, then truncates |
| `-` + 400 `9`s | `0` | saturates to `LONG_MIN`, then truncates |

A `str::parse::<i32>()`-based translation would reject all of these, and an
`i32`-clamping translation would print `2147483647`/`-2147483648` instead.

Rust: accumulates the magnitude in a `u64`, tracks overflow against the
`strtol` cutoff (`i64::MAX`, or `i64::MAX + 1` when negative), saturates to
`i64::MAX`/`i64::MIN`, then does `as i32` for the truncating assignment.

Verified: `int_boundaries`, `long_boundaries_and_saturation`,
`very_long_digit_strings`, `overflow_values_in_the_middle_and_at_the_end`.
Mutations were caught: removing positive saturation failed 4 tests; clamping to
`i32` range instead of truncating failed 5.

### 3. Leading zeros are decimal, never octal

C: `%d` is base 10 unconditionally (unlike `%i`), so `010` is ten and 500 zeros
followed by `7` is seven.

Rust: plain base-10 digit accumulation, no `0`/`0x` prefix handling.

Verified: `leading_zeros_are_decimal_not_octal`.

### 4. Sign handling, including signs that lead nowhere

C: `%d` accepts one optional `+` or `-`. A sign not followed by a digit is a
failure, and glibc can only push back one character, so the sign is consumed
and lost. `--5`, `+-5`, `- 5`, `-\n5` and a lone `-` at EOF all fail the
conversion, which breaks the loop.

Rust: consumes at most one sign; if the next byte is not a digit it `ungetc`s
that byte and returns 0.

Verified: `sign_handling_and_lone_signs`. Removing `+` support failed 2 tests.

Note: `scanf` returns `0` on a matching failure and `EOF` (`-1`) on an input
failure, but `main` only tests `!= 1`, so the two are indistinguishable here.
The `ungetc` is likewise unobservable, because the failure immediately `break`s
the loop and nothing reads the stream again. Mutants that changed only these
details survived, correctly — they are behaviourally equivalent, not gaps.

### 5. The 100-item cap and the unread tail

C: `for (i = 0; i < 100; i++)` stops after 100 successful reads. Item 101 and
anything after it stay in the stream unread, so a malformed or overflowing tail
after 100 valid items has no effect on the output.

Rust: `while i < 100` with the same break condition.

Verified: `item_counts_around_the_limit` (0, 1, 2, 3, 98, 99, 100, 101, 102,
150, 200 items), `exactly_one_hundred_items`,
`beyond_the_limit_ignores_the_tail`. Changing the cap to 99 failed 3 tests.

### 6. Output formatting and exit status

C: exactly `printf("%d\n", result)` — no width, no precision, no padding, one
trailing newline, nothing on stderr, `return 0` on every path.

Rust: `write!(stdout, "{}\n", result)` and no stderr output; `main` returns
unit, so the process exits 0.

Verified: `output_has_exactly_one_trailing_newline`, and every `assert_same`
call checks stderr and exit status alongside stdout. Dropping the newline,
doubling it, or adding a leading space each failed all 23 tests.

### 7. Bytes that are neither digits nor whitespace

C: NUL, bytes ≥ 0x80 and ordinary letters are all plain matching failures; NUL
in particular is not a terminator, because the stream is not a C string.

Rust: byte-oriented throughout — no UTF-8 validation, so no lossy conversion or
panic on invalid input.

Verified: `embedded_nul_and_non_ascii_bytes`, `matching_failure_on_first_item`,
`matching_failure_mid_stream`.

## Test suite

`translation/tests/differential.rs` — 23 tests, ~900 distinct inputs, all
passing. It builds `c_src` with CMake on first use if `c_src/build/driver` is
absent, then drives both executables through `std::process::Command`. The Rust
code is never linked as a library. Nothing is `#[ignore]`d, skipped or disabled.

Broken-pipe errors while writing stdin are tolerated, since a program is
entitled to stop reading before the writer is done; both programs here do read
to EOF for inputs under the 100-item cap.

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
cd translation && cargo test
```

## Mutation-testing summary

Each mutation was applied to `translation/src/main.rs` alone, with the C left
untouched; the source was restored afterwards and the suite re-confirmed green.

| mutation | tests failed |
|---|---|
| no-op control (unchanged behaviour) | 0 (expected) |
| item cap 100 → 99 | 3 |
| drop positive overflow saturation | 4 |
| clamp to `i32` instead of truncating | 5 |
| VT/FF no longer whitespace | 2 |
| skip only `' '` as whitespace | 5 |
| drop `+` sign support | 2 |
| return `out[0]` instead of `out[len-1]` | 14 |
| remove the trailing newline | 23 |
| emit two newlines | 23 |
| prepend a space to the output | 23 |
| drop negative overflow saturation | 0 — equivalent (`i64::MIN as i32 == 0`) |
| remove `ungetc` on matching failure | 0 — equivalent (stream is never read again) |

The two surviving mutants are behaviourally indistinguishable from the original
through the program's observable interface, so no test can or should catch them.
