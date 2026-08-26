# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options or conditional sources. The full valid feature set is therefore:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|--------|
| B1 | `--no-default-features --features ""` | default, PIC enabled | [x] |

## Runtime and Input Configurations

Mechanical branch/shape analysis found no `if`, `switch`, or runtime option in
the C API. The behavior-changing input axes are the three bit-field writes in
`driver`: `x & 3`, `y & 7`, and the low bit of `b`. The `z` field is copied
unchanged. `print_foo` reads those fields directly from the compiler-emitted
layout: packed bits in byte 0 and `z` at byte offset 4.

Every row below includes randomized `z` values spanning negative, zero,
positive, `INT_MIN`, and `INT_MAX`. In-range classes include their zero and
maximum boundaries; truncating classes include one-past-width values and
`UINT_MAX`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| V1 | `print_foo` | direct packed struct; randomized `x` (2 bits), `y` (3 bits), `b` (1 bit), `z`; zero unused/padding bits | [x] |
| V2 | `print_foo` | direct packed struct; randomized fields and arbitrary ignored bits/padding bytes | [x] |
| V3 | `driver` | `x` in 0..3; `y` in 0..7; `b = false` | [x] |
| V4 | `driver` | `x` in 0..3; `y` in 0..7; `b = true` | [x] |
| V5 | `driver` | `x > 3` truncates; `y` in 0..7; `b = false` | [x] |
| V6 | `driver` | `x > 3` truncates; `y` in 0..7; `b = true` | [x] |
| V7 | `driver` | `x` in 0..3; `y > 7` truncates; `b = false` | [x] |
| V8 | `driver` | `x` in 0..3; `y > 7` truncates; `b = true` | [x] |
| V9 | `driver` | `x > 3` and `y > 7` both truncate; `b = false` | [x] |
| V10 | `driver` | `x > 3` and `y > 7` both truncate; `b = true` | [x] |

The public entry-point set is complete: `driver` (declared convenience entry
point) and `print_foo` (lowest-level dynamic export).
