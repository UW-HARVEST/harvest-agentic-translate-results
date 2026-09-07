# Phase A.3 — Configuration-surface table

Derived mechanically from the C source and the public header, the same way
`ERRORS.md` is derived.

## Axis enumeration (what the C actually branches on)

### Axis 1 — public entry points (the FULL set, lowest level included)

`nm -D --defined-only c_src/build/libdriver.so` gives one strong symbol, and
`c_src/include/driver.h` declares one prototype:

```c
void driver(int x);
```

There is therefore exactly **one** public entry point, and it *is* the
lowest-level one — there is no convenience/one-shot wrapper layered over a
lower-level API, and no `static` internal helper that a wrapper composes. No
context/handle object exists, so there is no setup-then-run pipeline to drive.
Axis 1 has a single value: `driver`.

### Axis 2 — runtime options / modes / flags

`grep -nE 'if|switch|#if|else|enum|struct|typedef|extern|global|static' c_src/src/driver.c c_src/include/driver.h`
returns only the `DRIVER_H_` include guard. The header exposes no setter, no
option struct, no flags parameter, and no global variable; the `.so` exports no
data symbols (`nm -D --defined-only` shows no `B`/`D`/`R` entries). There is
**no runtime option and no compile-time `#ifdef` configuration** to toggle.
Axis 2 has a single value: *none*.

### Axis 3 — input shape / value class

The only input is one `int` passed by value. There is no size, count, width,
element type, format, byte order, or emptiness axis — no buffer, no length, no
array. What the C *does* distinguish on that `int` is the arithmetic and the
`%d` conversion:

* `2*x` — wraps (two's complement) once `|x| > INT_MAX/2`.
* `y += 300` — wraps once `2*x > INT_MAX - 300`.
* `printf("%d\n", y)` — glibc's `%d` branches on `y`'s sign (emits `-` or not),
  on `y == 0` (special-cased digit emission), and on the decimal digit count
  (1..10 digits) when converting.

Giving these value classes: zero, small ±, the exact `y == 0` preimage
(`x == -150`), sign-flip neighbourhood of `y`, every decimal-digit-count
boundary of `y`, the `2*x` overflow boundary, the `+300` overflow boundary, and
the domain extremes `INT_MIN`/`INT_MAX`.

### Axis 4 — observable output channel state

`driver` has no return value; its **entire** observable effect is bytes written
to the process's `stdout` via libc `printf`. A real consumer's `stdout` is in
one of glibc's three buffering modes, and the consumer interleaves its own
stdio writes with calls into the library. Buffering mode and interleaving are
genuine state the code interacts with (writing via a different mechanism —
e.g. Rust's `std::io::stdout`, which holds its own buffer — would reorder bytes
relative to a C caller's writes while still producing the right bytes in
isolation). Axis 4 values: fully buffered (redirected fd), line buffered,
unbuffered, plus call-multiplicity (one call / many calls per flush) and
interleaving with the caller's own `printf`.

## CONFIGURATION-SURFACE TABLE

Pruned cross-product of Axis 3 × Axis 4 (Axes 1 and 2 are single-valued).
Every row is run against BOTH `.so`s through `libloading` and compared
byte-for-byte; rows marked *randomized* use many property-style inputs from a
fixed-seed PRNG.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | options: none. `x == 0` (zero value) → `y == 300`; stdout fully buffered | [x] |
| 2 | `driver` | options: none. `x` small positive, `1..=1000` exhaustive; fully buffered | [x] |
| 3 | `driver` | options: none. `x` small negative, `-1000..=-1` exhaustive; fully buffered | [x] |
| 4 | `driver` | options: none. `x == -150`, the exact preimage of `y == 0` (`printf` zero special case); fully buffered | [x] |
| 5 | `driver` | options: none. sign-flip neighbourhood of `y`: `x ∈ {-152,-151,-150,-149,-148}` (`y` crosses 0, `-` sign appears/disappears); fully buffered | [x] |
| 6 | `driver` | options: none. every decimal-digit-count boundary of `y` (1→2→…→10 digits, both signs): `x` such that `y ∈ {±1,±9,±10,±99,±100,±999,±1000,…,±1000000000, INT_MAX, INT_MIN}`; fully buffered | [x] |
| 7 | `driver` | options: none. `2*x` signed-overflow boundary: `x ∈ {INT_MAX/2-1, INT_MAX/2, INT_MAX/2+1, INT_MIN/2-1, INT_MIN/2, INT_MIN/2+1}`; fully buffered | [x] |
| 8 | `driver` | options: none. `y += 300` overflow boundary: `x ∈ [INT_MAX/2 - 151, INT_MAX/2 - 147]` (`2*x + 300` crosses `INT_MAX`); fully buffered | [x] |
| 9 | `driver` | options: none. domain extremes `x ∈ {INT_MIN, INT_MIN+1, INT_MAX-1, INT_MAX}`; fully buffered | [x] |
| 10 | `driver` | options: none. *randomized* full `i32` domain, uniform, fixed seed, 20000 draws; fully buffered | [x] |
| 11 | `driver` | options: none. *randomized* near-overflow band `x ∈ [INT_MAX/2 - 4096, INT_MAX/2 + 4096]`, fixed seed, 8000 draws; fully buffered | [x] |
| 12 | `driver` | options: none. *randomized* near-overflow band `x ∈ [INT_MIN/2 - 4096, INT_MIN/2 + 4096]`, fixed seed, 8000 draws; fully buffered | [x] |
| 13 | `driver` | options: none. *randomized* narrow band around zero `x ∈ [-4096, 4096]`, fixed seed, 8000 draws; fully buffered | [x] |
| 14 | `driver` | options: none. many calls (5000, *randomized* fixed seed) accumulated in ONE stdout flush — checks byte ordering across a whole buffered run, not per call; fully buffered | [x] |
| 15 | `driver` | options: none. stdout **line buffered** (`setvbuf _IOLBF`), *randomized* inputs, many calls | [x] |
| 16 | `driver` | options: none. stdout **unbuffered** (`setvbuf _IONBF`), *randomized* inputs, many calls | [x] |
| 17 | `driver` | options: none. caller's own `printf` writes **interleaved** between library calls, fully buffered — checks the library shares the caller's stdio buffer rather than a private one | [x] |
| 18 | `driver` | options: none. caller's own `write(2)` (raw fd, unbuffered) interleaved with library calls under a fully buffered stdout — checks flush timing matches | [x] |
| 19 | `driver` | options: none. *randomized* full domain, the SAME input replayed through C then Rust then C again — checks the call is stateless/idempotent in both (no hidden accumulator) | [x] |
| 20 | `driver` | options: none. exhaustive sweep of a strided slice of the full domain (`x = INT_MIN + k*3_999_971`, all `k` until wrap, ~1074 points) — covers all magnitude decades and both signs uniformly | [x] |

All 20 rows pass. There is one feature combination (`Cargo.toml` has no
`[features]` section), so the table is complete as written.

## Harness validation (why "all rows pass" is meaningful here)

A green differential suite proves nothing unless the harness can actually see a
divergence. Five deliberately mutated Rust implementations were built in a
throwaway copy of the crate (`/tmp`, since deleted; neither `c_src/` nor the real
crate was touched) and run through the same suite. Every mutant was caught:

| mutant | tests that FAILED |
|--------|-------------------|
| `y += 301` instead of `300` | 27 of 31 |
| `saturating_mul` / `saturating_add` instead of `wrapping_*` | 18 |
| `i64` widening + `clamp` instead of two's-complement wraparound | 17 |
| `println!` instead of libc `printf` (correct digits, wrong output channel) | 1 — **only row 18** |
| `#[no_mangle]` removed (impl present, symbol not exported) | 29, incl. both `sym_*` tests |

The fourth mutant is the point of rows 17–18: it emits byte-identical digits and
passes all 16 value-shape rows, and is caught only by the row that interleaves a
raw `write(2)` with buffered library output. Without axis 4 in this table, that
class of divergence would have gone unnoticed.
