# CONFIGS.md — Configuration-surface table (Phase A → Phase B)

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually branches on

**Compile-time axes: none.** No `#ifdef`/`#if` anywhere (`grep -rnE '#if|#ifdef|#else|#elif' c_src/`
matches only the `DRIVER_H_` include guard), no CMake `option()`, no
`target_compile_definitions`, and `Cargo.toml` has no `[features]`. → exactly one
configuration: the empty feature set.

**Runtime options/modes: none.** There is no init/config/setter function, no
global mutable state, and no flags argument. All behaviour is driven by the
arguments to the two exported entry points.

**Public entry points (from `nm -D` on the C `.so`, i.e. the full set, including
the low-level one that is absent from the header):**

| entry point | signature | in `driver.h`? |
|---|---|---|
| `driver` | `void driver(const char *in)` — one-shot convenience wrapper: parse then `run` twice | yes |
| `run` | `void run(house_t *the_house, int extra_bedrooms)` — low-level operation on caller-owned state | **no** (still exported & callable) |

`house_t` (private to the `.c` file but part of `run`'s ABI):
`{ int floors; int bedrooms; double bathrooms; }` — offsets 0/4/8, size 16, align 8.

**Input-shape axes the code distinguishes**

*`run` / `print_house` / `add_floor` / `add_bedrooms`:*
* `floors` (`int`, `%d`): 0 · positive · negative · `INT_MAX` (`++` wraps) · `INT_MIN`
* `bedrooms` (`int`, `%d`): 0 · positive · negative · `INT_MAX`/`INT_MIN` (`+=` wraps)
* `bathrooms` (`double`, `%.1f`, then `+= 1.0` twice per call): halves (`2.5`) ·
  arbitrary finite · round-to-nearest-even tie values (`x.x5`) · `±0.0` ·
  subnormal · `DBL_MAX`/huge (where `+= 1.0` is a no-op) · `±inf` · `±NaN`
* `extra_bedrooms` (`int`): 0 · ±small · `INT_MAX` · `INT_MIN` · arbitrary
* call **multiplicity**: `run` mutates `*the_house`, so 1 call vs. N successive
  calls on the same struct are different code paths (state carry-over); `driver`
  itself calls `run` twice on one house.

*`driver` / `parse_val` (`strtol(str, &endp, 10)`, base fixed at 10):*
* accepted-and-fully-consumed decimal: `0`, `+5`, `-5`, leading zeros
* accepted-with-trailing-garbage (`endp != str` ⇒ **success**): `"5abc"`, `"5 "`,
  `"5.9"`, `"0x1A"` → `0`, `"010"` → `10`, `"1e3"` → `1`, `"12,34"` → `12`
* leading whitespace skipped by `strtol`: `" \t\n\v\f\r-7"`
* `int` boundaries: `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1`
* string length: 1 byte · long digit runs · long whitespace prefix
* (out-of-range / unparsable shapes are the rejection surface → `ERRORS.md`)

## Rows (pruned cross-product of the axes above)

Every row is exercised with **many randomized inputs** (deterministic
`xorshift64*`, fixed seed `0x2545F4914F6CDD1D`) plus the hand-picked boundary
values, comparing C `.so` vs Rust `.so` stdout **byte-for-byte**.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `run` | canonical house `{2,5,2.5}`, `extra_bedrooms = 0` (baseline, single call) | [x] |
| 2 | `run` | `floors`/`bedrooms`/`extra_bedrooms` randomized small (`-1000..1000`), `bathrooms` random half-integers | [x] |
| 3 | `run` | `floors`/`bedrooms`/`extra_bedrooms` randomized over the **full `i32` range**, `bathrooms` random half-integers (exercises `%d` on all int shapes + wrapping `+=`) | [x] |
| 4 | `run` | `bathrooms` = arbitrary random **finite** `f64` (random bit patterns, rejecting non-finite), ints random small — exercises `%.1f` rounding across magnitudes | [x] |
| 5 | `run` | `bathrooms` = round-half tie values `k + 0.05/0.15/…/2.25/…` and `x.x5` decimals — `%.1f` round-half-even boundaries | [x] |
| 6 | `run` | `bathrooms` = special values: `+0.0`, `-0.0`, `f64::MIN_POSITIVE`, subnormal `5e-324`, `DBL_MAX`, `-DBL_MAX`, `1e300`, `1e16` (`+= 1.0` is a no-op), `1e-300` | [x] |
| 7 | `run` | `bathrooms` non-finite: `+inf`, `-inf`, `NaN`, `-NaN` (also ERRORS row 13) | [x] |
| 8 | `run` | `floors = INT_MAX` / `INT_MAX-1` / `INT_MIN` → `add_floor` overflow wrap | [x] |
| 9 | `run` | `bedrooms = INT_MAX` with `extra_bedrooms > 0`, and `bedrooms = INT_MIN` with `extra_bedrooms < 0` → `add_bedrooms` overflow/underflow wrap | [x] |
| 10 | `run` | `extra_bedrooms ∈ {INT_MIN, INT_MAX, -1, 0, 1}` × `bedrooms ∈ {INT_MIN, -1, 0, 1, INT_MAX}` (full boundary cross-product) | [x] |
| 11 | `run` (repeated) | **state carry-over**: same `house_t` passed to `run` N times (N = 1..8, randomized fields) — accumulates `floors += N`, `bathrooms += N`, `bedrooms += N*extra` | [x] |
| 12 | `run` (repeated) | state carry-over with `bathrooms` near the `+= 1.0` resolution limit (`2^52`, `2^53`, `2^53+1`) over N calls | [x] |
| 13 | `driver` | canonical valid input `"0"`, `"1"`, `"-1"` (drives `parse_val` → 2× `run` on `{2,5,2.5}`) | [x] |
| 14 | `driver` | randomized valid decimal strings over the full `int` range (`-2147483648..2147483647`), with and without explicit `+`/`-` sign | [x] |
| 15 | `driver` | `int` boundary literals: `"-2147483648"`, `"-2147483647"`, `"2147483647"`, `"2147483646"`, `"+2147483647"`, `"-0"`, `"+0"` | [x] |
| 16 | `driver` | leading-zero forms: `"000"`, `"0000000000000000005"`, `"-000005"`, `"+0000000000000000000000000042"` | [x] |
| 17 | `driver` | leading-whitespace forms: every `isspace` byte and random whitespace runs (` `, `\t`, `\n`, `\v`, `\f`, `\r`) before the number | [x] |
| 18 | `driver` | trailing-garbage forms that **succeed** (`endp != str`): `"5abc"`, `"5 "`, `"5.9"`, `"1e3"`, `"12,34"`, `"7-"`, `"9\n"`, `"3\0hidden"` | [x] |
| 19 | `driver` | base-10-only interpretation of prefixed forms: `"0x1A"`→0, `"0X"`→0, `"0b101"`→0, `"010"`→10, `"-0x10"`→0 | [x] |
| 20 | `driver` | randomized digit strings of random length 1..25 with random sign — spans in-`int`, in-`long`-only and `ERANGE` shapes in one property test | [x] |
| 21 | `driver` | randomized arbitrary printable-ASCII / full-byte fuzz strings (length 0..24) — mixes accept and reject shapes | [x] |
| 22 | `driver` | 1-byte inputs: every single byte `0x01..0xFF` as a one-char string | [x] |
| 23 | `driver` | oversized inputs: 4096-digit run, 4096-byte whitespace prefix + digit, 4096 `'a'`s | [x] |
| 24 | `driver` then `run` | **composed pipeline / cross-entry-point**: call `driver(s)` and then `run` on a caller-owned house in the same library instance, repeatedly, to prove there is no hidden shared state and that interleaving matches | [x] |
| 25 | `run` | caller-owned `house_t` at a **misaligned** address (byte offsets 0..7 inside a buffer): the C does plain unaligned x86 loads/stores and works, so the Rust must move the same bytes and must not abort | [x] |

## Status

All 25 rows pass under the single (empty) feature combination; see
`tests/phase_b_configs.rs`.

Divergence found and fixed while working through this table (Rust changed, C
untouched): rows 25 / ERRORS 9–10 showed the Rust `.so` aborting (SIGABRT) where
the C `.so` performed the access (misaligned) or faulted (SIGSEGV), because
`(*house).field`, `addr_of!`, `read_volatile::<i32>`, `read_unaligned` and
`copy_nonoverlapping` all carry null/alignment UB-checks under
`-C debug-assertions` (cargo's default for `dev`/`test`). `src/lib.rs` now
accesses the fields through integer address arithmetic + byte-wise
`read_volatile::<u8>`/`write_volatile::<u8>`, which is check-free in every
profile and byte-identical to the C for valid pointers.
