# CONFIGS.md — Configuration-surface table (valid paths)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually branches on

**A. Public entry points.** `nm -D` exports exactly two. `driver.h` declares
only `driver`, but `run` has external linkage and is exported, so it is a real
public entry point and the *lowest-level* one available to a consumer:

| entry point | level | in header? | exported? |
|-------------|-------|-----------|-----------|
| `run(int extra_bedrooms)` | **low-level** — one 4-print mutation pass | no | **yes** |
| `driver(const char *in)`  | high-level wrapper — parse, then `run(x); run(x);` | yes | yes |

**B. Runtime options/flags.** There are **none** — no setters, no mode enum, no
globals a caller can configure, no `#ifdef`. The only caller-controlled inputs
are `run`'s `int` and `driver`'s string.

**C. Hidden persistent state — the real configuration axis.** The single
`static house_t the_house = {2, 5, 2.5}` is process-global and **mutated by
every call**. So the output of a call depends on the entire *history* of prior
calls. Each `run()` pass advances state by: `floors += 1`, `bathrooms += 1.0`,
`bedrooms += extra_bedrooms`. This makes *call sequence* an axis; `driver`
advances it twice per call.

**D. Input shapes the code special-cases.**
* `run`'s `int`: `0` / `+1` / small± / `INT_MAX` / `INT_MIN` — `bedrooms +=`
  **signed-overflow wraps** (gcc `-O0`).
* `driver`'s string, as distinguished by `strtol(.., 10)`: leading whitespace,
  explicit `+`/`-`, leading zeros, full-consumption vs. trailing garbage,
  INT/LONG boundary magnitudes.
* `bathrooms` is a `double` printed `%.1f`: exact halves accumulate
  (2.5, 3.5, …); large call counts probe `%.1f` rounding and float growth.
* `floors`/`bedrooms` are `int` printed `%d`.

## Configuration-surface table

One row per combination the C treats differently. Every row is driven through
**both** `.so`s via `libloading` and compared byte-for-byte on `stdout`, with
**many randomized inputs per row** (fixed seed, xorshift64\*), except where the
row is a fixed boundary.

### Low-level entry point `run` — called directly

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|-----|
| 1 | `run` | pristine state, single call, `extra_bedrooms == 0` (identity add) | [x] |
| 2 | `run` | pristine state, single call, `extra_bedrooms == 1` | [x] |
| 3 | `run` | pristine state, single call, small positive (randomized `1..=1000`) | [x] |
| 4 | `run` | pristine state, single call, small negative (randomized `-1000..=-1`) — drives `bedrooms` negative | [x] |
| 5 | `run` | pristine state, single call, `INT_MAX` — **signed overflow** of `bedrooms += ` must wrap identically | [x] |
| 6 | `run` | pristine state, single call, `INT_MIN` — **signed underflow** must wrap identically | [x] |
| 7 | `run` | pristine state, single call, full-domain randomized `int` (any bit pattern, 512 draws) | [x] |
| 8 | `run` | **accumulated state**: long randomized sequence of `run` calls (256 calls, mixed signs/magnitudes), output compared after *every* call — catches state drift | [x] |
| 9 | `run` | **repeated overflow**: many consecutive `INT_MAX` / `INT_MIN` calls, so `bedrooms` wraps repeatedly | [x] |
| 10 | `run` | **`bathrooms` double growth + `%.1f`**: 2 000 sequential calls (`bathrooms` 2.5 → 2002.5, `floors` 2 → 2002) | [x] |
| 11 | `run` | **`floors`/`bathrooms` at scale**: 20 000 calls, exercising `%d` widening and continued exact-half `%.1f` formatting | [x] |

### High-level entry point `driver` — string parsing + double `run`

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|-----|
| 12 | `driver` | pristine state, plain decimal digits, randomized in-range values (512 draws) | [x] |
| 13 | `driver` | `"0"` and `"-0"` (zero, both signs) | [x] |
| 14 | `driver` | explicit `'+'` sign prefix, e.g. `"+42"` (randomized) | [x] |
| 15 | `driver` | explicit `'-'` sign prefix, e.g. `"-42"` (randomized) | [x] |
| 16 | `driver` | leading whitespace before the number: `" 42"`, `"\t\n\v\f\r 42"` (randomized ws prefix) | [x] |
| 17 | `driver` | leading zeros: `"007"`, `"0000000000042"` — base 10, **not** octal | [x] |
| 18 | `driver` | trailing garbage (C **accepts**, `endp != str` suffices): `"42abc"`, `"7 8"`, `"1,000"`, `"5-"`, `"0x1F"` → parses `0` | [x] |
| 19 | `driver` | whitespace + sign + zeros + garbage **combined**: `"  -0042xyz"` (randomized composition of all four shapes) | [x] |
| 20 | `driver` | INT boundary magnitudes that must SUCCEED: `"2147483647"` (INT_MAX), `"-2147483648"` (INT_MIN) → double overflow of `bedrooms` via two `run`s | [x] |
| 21 | `driver` | values near the boundary: INT_MAX−1, INT_MIN+1, ±2147483646 (randomized ±small offsets from the extremes) | [x] |
| 22 | `driver` | **accumulated state**: long randomized sequence of `driver` calls (256 calls), output compared after each — 4 state advances per call | [x] |
| 23 | `driver` | **interleaved with `run`**: randomized mix of `driver` and `run` calls (512 ops) on one shared state — the composed pipeline both entry points share | [x] |
| 24 | `driver` | interleaved mix where `driver` inputs are randomly valid **or** invalid, so error and success paths alternate against evolving state (invalid must leave state untouched) | [x] |

## Feature/build combinations

Only one exists (see `SYMBOLS.md`): the default, empty feature set. Every row
above is run under `cargo test --no-default-features` (≡ `cargo test`).

## Findings from mutation testing (`./mutation_check.sh`)

All 24 rows pass differentially across their randomized inputs, under both the
dev and release profiles. To prove the rows are not vacuous, 25 deliberate bugs
were injected into `src/lib.rs` one at a time; **every one was caught**,
including: `bedrooms` saturating instead of wrapping (caught by rows 5/6/9),
`bedrooms` subtracting instead of adding, `%.1f` → `%.2f`/`%.0f`, swapping the
`floors`/`bedrooms` printf arguments, `floors += 2`, each of the three initial
`the_house` field values, `strtol` base 16 and base 0 (caught by row 17's
leading-zero/octal row), `bathrooms += 0.5`, an off-by-one in the parsed value,
`driver` calling `run` once or three times instead of twice, and two statement
reorderings inside `run`.

One mutation is **expected** not to be caught and is documented rather than
"fixed": `floors` saturating instead of wrapping, which differs only after 2^31
`run()` calls (see `ERRORS.md` → Findings).
