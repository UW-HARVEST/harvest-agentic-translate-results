# Differential-testing findings

C ground truth: `c_src/src/main.c` (built with CMake → `c_src/build/driver`).
Rust under test: `translation/src/main.rs` (→ `translation/target/release/driver`).
Harness: `translation/tests/differential.rs` — runs both as subprocesses and
compares stdout bytes, stderr bytes and exit status.

## Program surface

```c
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

One input: stdin, consumed by a single `scanf("%d", &x)`. `driver()` and
`print_hex()` contain no data-dependent branches — the loop always runs
`sizeof(house_t) == 16` times — so the only observable variation is the four
bytes of `house.floors`. Output is always 32 hex digits plus `\n` (33 bytes),
stderr is always empty, exit status is always 0.

All behavioural risk therefore lives in reproducing glibc's `%d` conversion
exactly.

## Mismatches found

### 1. Vertical tab (0x0B) was not treated as whitespace — FIXED

Status: real mismatch, found by differential testing, now fixed.

- Input: `"\v7"` (0x0B, 0x37)
- C output: `07000000030000000000000000000040` (converted 7)
- Rust output (before fix): `00000000030000000000000000000040` (conversion
  failed, `x` stayed 0)

Cause: the scanner skipped leading whitespace with Rust's
`char::is_ascii_whitespace()`, whose set is `{' ', '\t', '\n', '\f', '\r'}`.
C's `isspace()` in the `"C"` locale also includes the vertical tab `'\v'`
(0x0B). `scanf`'s `%d` directive skips *C* whitespace, so the C program stepped
over the 0x0B and converted `7`, while Rust stopped on it and reported a
matching failure.

Fix (`translation/src/main.rs`): replaced the call with an explicit helper that
spells out C's set.

```rust
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}
```

Regression coverage: `whitespace_is_skipped_including_vertical_tab`,
`exhaustive_two_byte_inputs_over_interesting_alphabet`, `deterministic_fuzz`.

## Behaviours deliberately replicated (verified matching, not "fixed")

These looked like candidates for a mismatch and were each confirmed identical.
None of them is a bug to be corrected — the C is the specification.

- **`scanf` reads across newlines.** `%d` skips *all* leading whitespace,
  newlines included, so `"\n\n\n99"` yields 99 rather than a failure. Verified.
- **Input failure leaves `x` at its initializer.** Empty stdin, whitespace-only
  stdin and `scanf` returning `EOF` all leave `x == 0`, and the program still
  prints a full line and exits 0. Verified for `""`, `" "`, `"\n"`, `"\v"`,
  1024 NUL bytes and 1024 `0xff` bytes.
- **Matching failure also leaves `x` at 0**, and the consumed sign character is
  not restored: `"-"`, `"+"`, `"-\n5"`, `"--5"`, `"-+5"` all print the `x == 0`
  line. Verified.
- **glibc saturates in `long`, then truncates to `int`.** glibc's `%d` runs
  `strtol` and assigns the `long` result to an `int`:
  - `"2147483648"` → fits in `long`, truncates to `INT_MIN` → `00000080`
  - `"5000000000"` → truncates to 705032704 → `00f2052a`
  - `"9223372036854775808"` → over `LONG_MAX`, saturates to `LONG_MAX`, whose
    low 32 bits are `0xffffffff` → `ffffffff`
  - `"-9223372036854775809"` → saturates to `LONG_MIN`, low 32 bits `0` →
    `00000000`
  The Rust scanner reproduces this with a saturating `i64` accumulator followed
  by `value as i32`. Verified up to 400-digit inputs.
- **Conversion stops at the first non-digit, which is pushed back.**
  `"0x10"` → 0 (stops at `x`), `"42abc"` → 42, `"1e5"` → 1, `"3.14"` → 3,
  `"1,234"` → 1, `"7 8"` → 7. Verified.
- **Leading zeros are decimal, not octal.** `%d` is base 10, so `"010"` → 10
  and 300 zeros → 0. Verified.
- **Only the first conversion is consumed.** Trailing input is never read; the
  program may exit while the writer still has bytes queued, so the harness
  tolerates `EPIPE` on its own side. Verified with a 200 KB trailing payload.
- **Struct object representation, padding included.** `house_t` on LP64 x86-64
  is `floors@0, bedrooms@4, bathrooms@8`, size 16, with *no* padding bytes, so
  `house_t house = {0}` followed by three field assignments leaves nothing
  uninitialised and the dump is deterministic: `bedrooms = 3` → `03000000`,
  `bathrooms = 2.0` → `0000000000000040` (little-endian IEEE-754). The Rust
  side uses `#[repr(C)]` and native-endian byte conversion.
  `known_layout_for_input_one` pins the exact expected bytes so a silent drift
  in *both* programs would still be caught.
- **`printf("%02x")` formatting.** Lowercase, zero-padded to two digits, no
  separators, single trailing `\n`. Verified byte-for-byte.

## Harness validation (mutation testing)

To confirm the suite is not vacuous, three defects were injected into the Rust
program and the suite was re-run; each was caught, then reverted:

| Injected defect | Result |
|---|---|
| drop `0x0b` from `is_c_space` (the original bug) | 3 tests FAILED |
| `std::process::exit(1)` at end of `main` | 23 tests FAILED (exit-status assertion) |
| `eprintln!("noise")` at end of `main` | 22 tests FAILED (stderr assertion) |

## Coverage summary

23 tests, ~2400 distinct inputs, all three of stdout/stderr/exit status asserted
on every one:

- all 256 single-byte inputs, exhaustively
- all 400 two-byte combinations over the alphabet `%d` reacts to
  (`0-9 + - space \t \n \v \f \r . x e E , NUL a`)
- every bit position of the resulting `int`, plus `±(1<<n)` and `(1<<n)-1`
- digit-count sweep 1..=25 digits, signed and unsigned
- `int` and `long` boundaries, and the saturation range beyond `LONG_MAX`
- 1200 deterministic pseudo-random inputs (fixed xorshift seed — no flakiness):
  raw bytes, numeric-looking text, and random-length digit runs
- 200 KB inputs to check that unread trailing input is irrelevant

No test is `#[ignore]`d, skipped or disabled. Nothing in `c_src/` was modified.
