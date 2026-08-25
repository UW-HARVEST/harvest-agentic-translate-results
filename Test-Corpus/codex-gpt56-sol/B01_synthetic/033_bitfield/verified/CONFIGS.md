# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table, so the complete Rust feature power set
contains one combination: the empty set (`--no-default-features`). The CMake
file defines no options, and the C source contains no preprocessor
conditionals, so C also has one build-time configuration.

## Runtime configurations

The C data layout is an 8-byte `foo_t`: `x` occupies bits 0-1, `y` bits 2-4,
`b` bit 5, and `z` is a 32-bit `int` at byte offset 4. The rows below are the
cross-product the C treats differently through bit-field assignment, boolean
conversion, direct field access, or one of the four sequential `scanf` calls.
Every randomized row includes `INT_MIN`, `INT_MAX`, zero, and full-range
fixed-seed values for unconstrained integer fields.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `print_foo` | Direct 8-byte `foo_t`; all `x=0..3`, `y=0..7`, `b=0/1`, arbitrary padding bits, full-range `z` | [x] |
| 2 | `driver` | `x<=3`, `y<=7`, `b=false`, full-range `z` | [x] |
| 3 | `driver` | `x<=3`, `y<=7`, `b=true`, full-range `z` | [x] |
| 4 | `driver` | `x<=3`, `y>7`, `b=false`, full-range `z` | [x] |
| 5 | `driver` | `x<=3`, `y>7`, `b=true`, full-range `z` | [x] |
| 6 | `driver` | `x>3`, `y<=7`, `b=false`, full-range `z` | [x] |
| 7 | `driver` | `x>3`, `y<=7`, `b=true`, full-range `z` | [x] |
| 8 | `driver` | `x>3`, `y>7`, `b=false`, full-range `z` | [x] |
| 9 | `driver` | `x>3`, `y>7`, `b=true`, full-range `z` | [x] |
| 10 | `main` | All scans succeed; `x<=3`, `y<=7`, scanned `b=0`, full-range `z` | [x] |
| 11 | `main` | All scans succeed; `x<=3`, `y<=7`, scanned `b!=0`, full-range `z` | [x] |
| 12 | `main` | All scans succeed; `x<=3`, `y>7`, scanned `b=0`, full-range `z` | [x] |
| 13 | `main` | All scans succeed; `x<=3`, `y>7`, scanned `b!=0`, full-range `z` | [x] |
| 14 | `main` | All scans succeed; `x>3`, `y<=7`, scanned `b=0`, full-range `z` | [x] |
| 15 | `main` | All scans succeed; `x>3`, `y<=7`, scanned `b!=0`, full-range `z` | [x] |
| 16 | `main` | All scans succeed; `x>3`, `y>7`, scanned `b=0`, full-range `z` | [x] |
| 17 | `main` | All scans succeed; `x>3`, `y>7`, scanned `b!=0`, full-range `z` | [x] |
| 18 | `main` | EOF before the first (`x`) scan; all initialized fields remain zero | [x] |
| 19 | `main` | EOF before the second (`y`) scan; parsed `x` survives, remaining fields stay zero | [x] |
| 20 | `main` | EOF before the third (`b`) scan; parsed `x/y` survive, `b/z` stay zero | [x] |
| 21 | `main` | EOF before the fourth (`z`) scan; parsed `x/y/b` survive, `z` stays zero | [x] |
| 22 | `main` | Non-numeric token at the first (`x`) scan; all initialized fields remain zero | [x] |
| 23 | `main` | Non-numeric token at the second (`y`) scan; parsed `x` survives, remaining fields stay zero | [x] |
| 24 | `main` | Non-numeric token at the third (`b`) scan; parsed `x/y` survive, `b/z` stay zero | [x] |
| 25 | `main` | Non-numeric token at the fourth (`z`) scan; parsed `x/y/b` survive, `z` stays zero | [x] |
