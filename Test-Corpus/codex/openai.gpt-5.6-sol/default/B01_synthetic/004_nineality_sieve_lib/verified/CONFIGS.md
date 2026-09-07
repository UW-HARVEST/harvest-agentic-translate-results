# Configuration Surface

Mechanically derived from the sole public declaration in
`../c_src/include/sieve.h` and the `if (val % 10 == 9)` branch in
`../c_src/src/sieve.c`.

There are no runtime options, modes, flags, element types, formats, byte-order
choices, alternate entry points, compile-time features, or binaries. The input
is one C `int`. Nonnegative decimal residue classes produce distinct output
lengths, while negative inputs take the distinct path of counting through zero
before the stopping condition can become true.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `sieve` | negative `int`; count crosses zero and stops at positive 9 | [x] |
| 2 | `sieve` | nonnegative `int`, `val % 10 == 9`; immediate stop | [x] |
| 3 | `sieve` | nonnegative `int`, `val % 10 == 0`; 10 emitted values (includes zero) | [x] |
| 4 | `sieve` | nonnegative `int`, `val % 10 == 1`; 9 emitted values | [x] |
| 5 | `sieve` | nonnegative `int`, `val % 10 == 2`; 8 emitted values | [x] |
| 6 | `sieve` | nonnegative `int`, `val % 10 == 3`; 7 emitted values | [x] |
| 7 | `sieve` | nonnegative `int`, `val % 10 == 4`; 6 emitted values | [x] |
| 8 | `sieve` | nonnegative `int`, `val % 10 == 5`; 5 emitted values | [x] |
| 9 | `sieve` | nonnegative `int`, `val % 10 == 6`; 4 emitted values | [x] |
| 10 | `sieve` | nonnegative `int`, `val % 10 == 7`; 3 emitted values | [x] |
| 11 | `sieve` | nonnegative `int`, `val % 10 == 8`; 2 emitted values | [x] |

Feature combinations: one (the crate declares no Cargo features).
