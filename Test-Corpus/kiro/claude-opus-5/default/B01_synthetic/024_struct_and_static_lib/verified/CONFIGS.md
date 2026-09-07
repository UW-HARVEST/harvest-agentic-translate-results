# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

The C library is one straight-line translation unit with no conditionals
(see `ERRORS.md` for the grep proving 0 `if`/`switch`/`#ifdef`). The axes it
actually distinguishes therefore come from its **state** and its **entry
points**, not from option flags:

**Axis 1 — entry point.** Two exported symbols (`SYMBOLS.md`):
- `run(int)` — the LOW-LEVEL entry point. Not declared in `driver.h`, but
  non-`static`, hence part of the ABI. Performs one 4-print pass.
- `driver(int)` — the convenience wrapper: `run(x); run(x);`.
Both must be driven directly; `driver` alone would never show a single-pass
state transition, and `run` alone would never show the compounded double pass.

**Axis 2 — persistent mutable global state.** `static house_t the_house` is
file-scope, mutated in place, and **never reset**. Each call permanently
advances it, so the output of call *N* depends on all of calls *1..N−1*:
- `floors` : starts `2`, `+1` per `run` (`add_floor_to_the_house`)
- `bathrooms` : starts `2.5`, `+= 1.0` per `run` (direct write in `run`)
- `bedrooms` : starts `5`, `+= extra_bedrooms` per `run` (`add_bedrooms`)
This makes *call-sequence length* and *call-sequence composition* real
configuration axes. Every row below is therefore executed in a **freshly
`exec`'d process per library** so the starting state is pristine and the C and
Rust runs are directly comparable.

**Axis 3 — argument value shape** (the sole parameter is a bare `int`):
zero / one / minus-one / small / large / `INT_MAX` / `INT_MIN` / the exact
overflow boundary of `5 + extra_bedrooms` / uniformly random over all 2^32.

**Axis 4 — printed-value shape**, i.e. what `printf("%d ... %d ... %.1f")` is
handed: positive, negative (leading `-`), the widest `%d` (`-2147483648`),
1-to-4-digit `floors`, and `%.1f` of a `double` that is always exactly
`2.5 + k` (so the fraction digit is exact and no rounding mode is involved) at
small and large magnitude.

**Axis 5 — ABI-level call shape**: argument passed as a proper 32-bit `int`
vs. a 64-bit value whose high half is non-zero (register-truncation behaviour).

There are **no** runtime options, modes, flags, byte-order choices, element
types, or formats: the public surface is `void run(int)` / `void driver(int)`.
Stating otherwise would be inventing configuration the C does not have.

## Table

Each row is run against BOTH `.so`s through `libloading` in separate processes
and stdout is compared byte-for-byte. "randomized" = SplitMix64 with a fixed
seed (`0x5eed_1234_5678_9abc`), reproducible.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `run` | fresh state, `extra_bedrooms = 0` | [x] |
| 2 | `run` | fresh state, `extra_bedrooms = 1` | [x] |
| 3 | `run` | fresh state, `extra_bedrooms = -1` | [x] |
| 4 | `run` | fresh state, `extra_bedrooms = INT_MAX` (bedroom add overflows) | [x] |
| 5 | `run` | fresh state, `extra_bedrooms = INT_MIN` (largest magnitude negative; `5 + INT_MIN` does *not* wrap — boundary check) | [x] |
| 6 | `run` | fresh state, exact overflow boundary pair `INT_MAX-5` (last non-wrapping) and `INT_MAX-4` (first wrapping) | [x] |
| 7 | `run` | fresh state, `extra_bedrooms = -5` (bedrooms lands exactly on `0`) and `-6` (lands on `-1`) | [x] |
| 8 | `run` | fresh state, 64 randomized full-range `i32` values, one fresh process per value | [x] |
| 9 | `run` | fresh state, 64 randomized *small* values in `[-1000, 1000]`, one fresh process per value | [x] |
| 10 | `driver` | fresh state, `x = 0` (double pass; floors 2→4, bathrooms 2.5→4.5) | [x] |
| 11 | `driver` | fresh state, `x = 1` and `x = -1` | [x] |
| 12 | `driver` | fresh state, `x = INT_MAX` (overflow applied twice, compounding) | [x] |
| 13 | `driver` | fresh state, `x = INT_MIN` (underflow applied twice; second pass *does* wrap) | [x] |
| 14 | `driver` | fresh state, 64 randomized full-range `i32` values, one fresh process per value | [x] |
| 15 | `run` ×2 | accumulated state: same value twice — must equal `driver(value)` output exactly (cross-check of the composed pipeline) | [x] |
| 16 | `run` ×16 | accumulated state, 16 randomized small values in one process (floors 2→18, bathrooms 2.5→18.5) | [x] |
| 17 | `run` ×16 | accumulated state, 16 randomized **full-range** values (repeated wrap-around of `bedrooms`) | [x] |
| 18 | `driver` ×8 | accumulated state, 8 randomized full-range values (16 passes) | [x] |
| 19 | `run` + `driver` mixed | accumulated state, 32 randomized calls with randomized entry-point choice and randomized values | [x] |
| 20 | `run` ×4096 | long accumulated sequence, `extra_bedrooms = 0`: `floors` grows 1→4 digits, `bathrooms` reaches `4098.5` (`%.1f` at large magnitude) | [x] |
| 21 | `run` ×64 | engineered sequence `+= INT_MAX/2` repeatedly: `bedrooms` crosses `INT_MAX` and wraps mid-sequence, repeatedly | [x] |
| 22 | `run` ×64 | engineered sequence `+= INT_MIN/2` repeatedly: `bedrooms` crosses `INT_MIN` and wraps mid-sequence, repeatedly | [x] |
| 23 | `run` ×3 | engineered so `bedrooms` lands exactly on `INT_MAX`, then on `INT_MIN`, then on `-2147483648` printed by `%d` (widest field) | [x] |
| 24 | `run` | ABI: symbol called through an `extern "C" fn(i64)` signature with high-half-dirty values (`0x1_0000_0000`, `0xFFFF_FFFF_0000_0007`, `i64::MAX`, `i64::MIN`, `-1`) | [x] |
| 25 | `driver` | same ABI truncation set as row 24 | [x] |
| 26 | `run` / `driver` | 256 randomized mixed-length sequences (length 1–12, random entry points, random full-range values), each in its own process pair | [x] |
