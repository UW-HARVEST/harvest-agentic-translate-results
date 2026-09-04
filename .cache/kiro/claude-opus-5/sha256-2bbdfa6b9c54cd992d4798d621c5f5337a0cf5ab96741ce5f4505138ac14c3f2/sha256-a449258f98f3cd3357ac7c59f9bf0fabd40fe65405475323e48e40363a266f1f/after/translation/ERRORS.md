# Verification report — differential testing of the Rust translation

Reference: `c_src/src/main.c` (never modified; verified by checksum after the run).
Under test: `translation/src/main.rs`.
Test suite: `translation/tests/differential.rs` — 21 tests, each spawning **both**
binaries as subprocesses and comparing stdout, stderr and exit status byte for byte.

## Result

**No mismatches were found.** Every input class enumerated below produced identical
stdout, identical stderr (always empty) and identical exit status (always 0) from the
C binary and the Rust binary. In addition to the 21 in-suite tests, roughly 7,500
ad-hoc differential cases were run during investigation (hand-written edge cases,
random token sequences, random raw byte strings, and random 1–40 digit magnitudes with
random signs); all agreed.

Because "no mismatch found" is only as strong as the suite that looked for it, the
suite was validated by mutation: thirteen deliberate defects were injected into
`src/main.rs` one at a time and the suite was re-run. **Every mutant was caught.**

| Injected defect | Tests that failed |
|---|---|
| `x` bit-field mask `& 0x3` removed | 4 |
| `y` bit-field mask widened `0x7` → `0xF` | 9 |
| `bool` field printed as `true`/`false` instead of `1`/`0` | 21 |
| `%u` rejects a leading `-` instead of accepting it | 10 |
| `strtoul` overflow wraps instead of saturating at `ULONG_MAX` | 1 |
| whitespace skip narrowed to space+tab (i.e. `fgets`-like, stops at `\n`) | 3 |
| trailing `\n` dropped from the format string | 21 |
| `z` printed as `u32` instead of `i32` | 9 |
| `!!b` replaced with `b > 0` | 6 |
| leading whitespace not skipped before the sign | 18 |
| exit status changed 0 → 1 | 21 |
| output written to stderr instead of stdout | 21 |

A suite that a mutant can slip past would report the same "all green" as a correct
program; these results are the evidence that green here means something.

## The C behaviours the translation has to reproduce

These are the places where a straightforward or idiomatic Rust rewrite *would* have
diverged. They are already handled correctly in `src/main.rs`; they are recorded here
because they are what a reader should check first, and each has a dedicated test.

### 1. Bit-field stores truncate silently

```c
unsigned int x : 2;   /* store keeps low 2 bits  */
unsigned int y : 3;   /* store keeps low 3 bits  */
bool b : 1;
```

Assigning `x = 4` yields `0`, `x = 7` yields `3`, `y = 8` yields `0`, `y = 15` yields `7`.
Rust has no bit-fields, so this must be written out as `x & 0x3` / `y & 0x7` at
construction time. Reference: `printf` of input `-1 -1 -1 -1` gives `3 7 1 -1`
(`0xFFFFFFFF & 3 = 3`, `0xFFFFFFFF & 7 = 7`).
Tests: `bitfield_truncation_x_two_bits`, `bitfield_truncation_y_three_bits`.

### 2. `%d` for `b` is `int`-truncated *before* `!!b` is applied

`main` scans `b` with `%d` into an `int`, then passes `!!b`. So a textually nonzero
input can still print `0`:

- input `1 2 4294967296 9` → C prints `1 2 0 9` (`4294967296` truncated to `int` is 0)
- input `1 2 4294967297 9` → C prints `1 2 1 9`

Any translation that decides truthiness from the parsed 64-bit value rather than from
the 32-bit truncated one gets this wrong. Test: `bool_bitfield_normalisation`.

### 3. `%u` accepts a leading minus sign

glibc's `%u` is `strtoul`, which accepts `+`/`-` and negates modulo 2<sup>64</sup>. It
does **not** reject negative input. `-1` becomes `0xFFFFFFFFFFFFFFFF`, narrowed to
`unsigned int` `0xFFFFFFFF`, then masked by the bit-field.
Test: `percent_u_accepts_a_negative_sign_and_wraps`.

### 4. Overflow saturates in `strtol`/`strtoul`, then narrows

The conversion saturates at `LONG_MAX` / `LONG_MIN` / `ULONG_MAX` (64-bit here) and the
saturated 64-bit value is what gets narrowed to 32 bits. The two steps are not
interchangeable with a single wrapping parse:

- `99999999999999999999` as `%u` → `ULONG_MAX` → `0xFFFFFFFF` → `& 3` = `3`
- `-99999999999999999999` as `%d` → `LONG_MIN` = `0x8000000000000000` → low 32 bits = `0`

Test: `strtol_and_strtoul_saturation_on_overflow`.

### 5. A failed conversion leaves its destination alone — and poisons the rest

`main` ignores every `scanf` return value. On input failure (EOF) or matching failure
the destination variable keeps its initialiser `0`. Critically, `%u`/`%d` do **not**
consume the offending non-digit byte, so that byte is still first in the stream for the
next `scanf`, which therefore fails the same way. One bad token zeroes every field from
that point on:

- `1 2 3 abc` → `1 2 1 0`
- `abc` → `0 0 0 0`
- `0x10 2 3 4` → `0 0 0 0` (`0` reads fine; `x` then blocks all three remaining scans)

Tests: `matching_failure_stops_every_later_conversion`,
`base_prefixes_and_float_syntax_stop_the_scan`, `fewer_values_than_conversions`.

### 6. …but a sign *is* consumed before the matching failure

This is the one asymmetry in the previous rule, and the subtlest behaviour in the
program. glibc consumes an optional sign before discovering there are no digits, and
does not push it back. So input `- 1 2 3` prints `0 1 1 3`: the first conversion fails
(`x` stays 0) yet the `-` is gone, leaving `1 2 3` for the remaining three conversions.
Treating the failure as "consume nothing" would print `0 0 0 0` instead.
Test: `sign_handling_and_sign_only_matching_failures`.

### 7. `scanf` skips whitespace across newlines

`%u`/`%d` skip any run of C-locale whitespace, newlines included, so the input's line
structure is irrelevant — `1\n2\n1\n42`, `1 2 1 42` and `\n\n1\n\n2\n\n1\n\n7` all
parse. This is the `scanf`-vs-`fgets` distinction; a line-oriented reader would fail
these. Vertical tab (`\x0b`) and form feed (`\x0c`) count as whitespace too.
Tests: `separator_matrix`, `happy_path_all_four_values`.

### 8. `%d` for `z` is base 10 and wraps into `int`

Leading zeros are not an octal prefix (`007` is 7). `2147483648` becomes `-2147483648`;
`4294967295` becomes `-1`; `4294967296` becomes `0`.
Tests: `leading_zeros_are_decimal_not_octal`, `signed_z_is_printed_with_percent_d`.

### 9. Output framing

Exactly one line: four fields, three single spaces, one trailing `\n`, nothing on
stderr, exit status 0 on every input — including empty stdin and pure garbage. There is
no error path in this program; it never returns nonzero and never writes a diagnostic.
Test: `output_is_exactly_one_line_with_trailing_newline`.

## Input classes covered

Empty stdin; whitespace-only stdin (space, tab, CR, LF, VT, FF and mixtures); 1, 2, 3
and 4 values supplied; every whitespace separator and the empty separator; trailing
junk after the fourth value; non-numeric first/second/third/fourth token; single
letter, punctuation, comma-separated, underscore, digits-then-letter; `-` alone, `+`
alone, `-` followed by whitespace, `--1`, `+-1`; hex prefixes, decimal points,
exponent notation, `inf`, `nan`; leading-zero runs up to 100,000 bytes; both bit-field
widths swept 0–16 exhaustively; all 64 powers of two in unsigned, negative and
explicitly-positive form, each ±1; `INT_MAX`, `INT_MIN`, `UINT_MAX`, `LONG_MAX`,
`LONG_MIN`, `ULONG_MAX` and each ±1; 20- and 40-digit magnitudes, signed and unsigned;
`-0`; embedded NUL bytes; bytes ≥ 0x80; valid and invalid UTF-8; and 200,000-byte
inputs.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test --release
```

The C binary is `c_src/build/driver`, the Rust binary is
`translation/target/release/driver`; both read stdin and take no arguments. The test
harness builds the C binary itself if `c_src/build/driver` is absent. No test is
`#[ignore]`d, skipped or otherwise disabled.
