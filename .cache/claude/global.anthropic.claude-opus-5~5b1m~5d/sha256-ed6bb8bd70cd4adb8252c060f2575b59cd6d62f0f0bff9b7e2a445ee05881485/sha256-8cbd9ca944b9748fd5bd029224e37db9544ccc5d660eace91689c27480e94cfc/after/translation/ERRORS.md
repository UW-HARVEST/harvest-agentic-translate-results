# ERRORS.md — Phase C error-surface table

Derived mechanically from the complete C source. The whole library is
`c_src/src/driver.c` (40 lines, 22 of which are the licence header) plus
`c_src/include/driver.h`.

## Mechanical grep for every rejection mechanism

Every grep below was run over **all** C sources and headers:

| pattern searched | hits (excluding comments / include-guard / loop condition) |
|------------------|-----------------------------------------------------------|
| `return` (any value) | 0 |
| `return -1` / `return NULL` / error sentinels | 0 |
| `assert` / `static_assert` / `abort` / `exit` | 0 |
| `NULL` / null-pointer checks | 0 |
| `errno` | 0 |
| `if` / `switch` / ternary (any branch) | 0 |
| range checks (`<`, `>`, `<=`, `>=`) | 1 — `i < len`, the `for`-loop bound in `print_hex`, **not** a rejection |
| `#if` / `#ifdef` conditional compilation | 1 — `#ifndef DRIVER_H_` include guard, **not** a rejection |
| MIN/MAX constants, `limits.h` | 0 |
| error enums / status codes | 0 |
| `perror` / `fprintf(stderr, …)` | 0 |

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|--------------------------------------------|-------------------|
| — | — | — | — |

**The table is intentionally empty: the C library has NO error surface.**

Justification (not an assumption — read off the source):

- The single public entry point is `void driver(int x)`. It returns `void`, so
  there is no channel through which an error code or sentinel could be
  reported.
- Its only parameter is a by-value `int`. **Every** bit pattern of an `int` is a
  valid `int`, so there is no such thing as an out-of-range argument: there is
  no pointer to be null, no length to be negative or oversized, no enum whose
  integer value could fall outside the valid variants, and no buffer to
  overflow.
- `driver` performs an unconditional `memcpy` of `sizeof(x)` bytes from `&x`
  into a `sizeof(x)`-byte local array, then calls `print_hex(raw, sizeof raw)`.
  Both the source and destination sizes are compile-time constants and equal,
  so the copy can never be short or long.
- `print_hex` is `static` and is only ever reached from `driver` with
  `len == sizeof(int)` and a valid non-null pointer, so its `i < len` bound can
  never be violated and its pointer can never be null. It is not reachable
  across the FFI boundary at all (it is not exported — see `SYMBOLS.md`).

## Phase C obligations actually discharged

Because the table has no rows, Phase C reduces to the "generic boundaries every
C API has" clause of the instructions. Those that are *expressible* for a
`void driver(int)` signature are covered by the differential tests in
`tests/differential.rs`:

| generic boundary | expressible for `void driver(int)`? | test |
|------------------|--------------------------------------|------|
| null pointer argument | no — no pointer parameter exists | n/a |
| zero length | no — no length parameter exists | n/a |
| oversized length | no — no length parameter exists | n/a |
| one step past a valid range | yes, vacuously — the range is all of `int`; the extremes and their wrap-around neighbours are tested | `boundary_extremes`, `phase_c_no_error_surface_extremes` |
| out-of-range enum value across FFI | no — no enum parameter exists; the `int` domain is total | covered by exhaustive-byte + extremes tests |
| return-value / error-code parity | vacuous — return type is `void` | asserted as "neither side aborts, both produce identical stdout" for every input |

`ERRORS.md` therefore has **0 unchecked rows**. ✅
