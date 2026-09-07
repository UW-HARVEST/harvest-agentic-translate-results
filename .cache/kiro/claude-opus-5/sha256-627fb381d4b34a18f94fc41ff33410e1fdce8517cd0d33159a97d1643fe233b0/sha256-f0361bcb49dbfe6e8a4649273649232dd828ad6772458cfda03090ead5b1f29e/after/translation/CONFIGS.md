# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

## Axes actually present in the C source

Derived mechanically, not guessed:

1. **Cargo features / `#ifdef`s** — `grep -n '#if\|#ifdef\|#ifndef' c_src/src/driver.c`
   yields only the `DRIVER_H_` include guard in the header. `translation/Cargo.toml`
   has **no `[features]` table**, so the feature cross-product is the single
   default configuration. (Verified by script; see "Feature combinations" below.)
2. **Runtime options / modes / flags** — none. There is no init function, no
   setter, no flag argument. The only mutable configuration is the *implicit*
   one: the file-scope `static house_t the_house` accumulates mutations, so the
   **call history is the configuration**. That makes call sequence / repetition
   count a first-class axis.
3. **Public entry points** (the FULL set, incl. the low-level one, not just the
   convenience wrapper):
   - `driver(const char *in)` — the one-shot convenience wrapper: parse, then
     `run(x); run(x);`
   - `run(int extra_bedrooms)` — the low-level entry point, exported but absent
     from `driver.h`. Called directly, it bypasses parsing entirely and admits
     any `int`, including values `driver` could never produce.
4. **Input shapes the code special-cases** — from the branches in `parse_val`
   (`endp != str`, `errno`, `INT_MIN`, `INT_MAX`) and from `strtol(…, 10)`
   semantics: empty, whitespace prefix, explicit `+`/`-`, leading zeros,
   trailing garbage, digit count (1 / many / ERANGE-sized), and the four
   numeric boundaries `INT_MIN`, `INT_MAX`, `0`, `±1`.
5. **State-value shapes reachable through the printer** — `print_the_house`
   formats `int floors`, `int bedrooms`, `double bathrooms` with
   `"%d … %d … %.1f"`. Distinguished shapes: negative `bedrooms`, `int`
   wraparound of `bedrooms` (`+=` overflow), and `bathrooms` growing by `1.0`
   per `run`, i.e. `%.1f` on `.5` vs `.0` fractions (banker's-vs-half-up
   rounding of the `.5` tie is a real divergence risk).

## Rows (pruned cross-product of the axes above)

Every row is exercised against BOTH `.so`s with **many randomized inputs**
(fixed seed `0x5EED_1234`, xorshift64\*), except where the row *is* a fixed
boundary. Each row loads a **fresh copy of each library** so the mutable
`the_house` starts from `{2, 5, 2.5}` on both sides, then compares captured
stdout byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `run` | single call, `extra_bedrooms = 0` — the minimal low-level path from pristine state | [x] |
| 2 | `run` | single call, randomized small positive `extra_bedrooms` (1..=1000) | [x] |
| 3 | `run` | single call, randomized small negative `extra_bedrooms` (-1000..=-1) → negative-`bedrooms` `%d` shape | [x] |
| 4 | `run` | single call, randomized full-width `int` (any bit pattern), incl. values that wrap `bedrooms += extra` | [x] |
| 5 | `run` | fixed boundaries `INT_MAX`, `INT_MIN`, `1`, `-1`, `0` — one row-set of exact edge values | [x] |
| 6 | `run` | **many** calls (2, 3, 8, 64, 257) with randomized args → accumulating `floors`, alternating `%.1f` `.5`/`.0` fraction, `floors` and `bedrooms` drift | [x] |
| 7 | `run` | 1000 calls with randomized args → long-run state accumulation, `bathrooms` = 1002.5 magnitude, repeated `int` wraparound | [x] |
| 8 | `driver` | valid canonical decimal, randomized in `INT_MIN..=INT_MAX`, no sign prefix quirks (`"%d"` formatted) → parse + `run` twice | [x] |
| 9 | `driver` | valid with explicit `+` sign, randomized non-negative | [x] |
| 10 | `driver` | valid with explicit `-` sign, randomized non-positive | [x] |
| 11 | `driver` | valid with randomized leading whitespace (` `, `\t`, `\n`, `\v`, `\f`, `\r` mixes) before the number | [x] |
| 12 | `driver` | valid with randomized leading zeros (1..40 zeros) — base-10 means these are decimal, not octal | [x] |
| 13 | `driver` | valid digits + randomized trailing garbage (accepted by the C; `endp` is never required to reach NUL) | [x] |
| 14 | `driver` | exact boundary literals `"2147483647"`, `"-2147483648"`, `"0"`, `"-0"`, `"+0"`, `"1"`, `"-1"` | [x] |
| 15 | `driver` | rejected input shape (any ERANGE / non-numeric / out-of-`int` value) — valid-path row asserting the *no-mutation* consequence: `driver(bad)` then `run(0)` must print pristine-state numbers on both | [x] |
| 16 | `driver` + `run` | **interleaved** low-level and wrapper calls in randomized order (e.g. `run`, `driver(ok)`, `run`, `driver(bad)`, `driver(ok)`) — the composed pipeline where state coupling bugs live | [x] |
| 17 | `driver` | repeated calls with the same valid input (2, 5, 20 times) → 4 prints × 2 runs × N | [x] |
| 18 | `driver` | randomized string built from the *full* shape grammar (whitespace? sign? zeros? digits? garbage?) — property-style fuzz, 4000 cases, covering valid and invalid uniformly | [x] |
| 19 | `run` | `bathrooms` `%.1f` tie-rounding sweep: enough calls that `bathrooms` passes through `x.5` values of increasing magnitude (2.5 … 1002.5) | [x] |
| 20 | `driver` | oversized valid-ish inputs: 1-, 2-, 10-, 4096-digit strings (last two ERANGE) | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares **only** `add_library(driver SHARED src/driver.c)`
— there is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **The project builds no driver
binary, so the "compare C and Rust stdout of the binaries" clause is vacuous.**
Equivalent coverage is obtained by rows 1–20, which compare stdout of the two
`.so`s byte-for-byte.

## Feature combinations

`translation/Cargo.toml` has no `[features]` section → exactly one
configuration (`default`, which is empty). Verified by
`tests/check_feature_combos.sh`, which enumerates features from `Cargo.toml`
and runs `cargo check`/`cargo test` over the resulting combinations:
`--no-default-features` and default are the only two, and both are identical
builds.

## Harness validity (negative controls)

Passing tests only mean something if the harness can fail. Two deliberate
mutations were injected into `translation/src/lib.rs`, rebuilt, and reverted:

| injected mutation | expected detector | result |
|-------------------|-------------------|--------|
| `add_floor`: `wrapping_add(1)` → `wrapping_add(2)` | every state-printing row | rows 1–20 failed with `divergence at op #0` |
| `parse_val`: added `*endp == 0` to the accept condition (the "obvious fix") | ERRORS.md row 11 | `errors_row11_trailing_garbage_accepted` failed |

Both mutations were caught and both were reverted (`diff` against the pre-mutation
copy is empty), which also proves the harness really loads the Rust `cdylib`
(`target/release/libdriver.so`, 403 KB) and not a second copy of the C `.so`
(16 KB).

Note on the first control attempt: changing `bathrooms += 1.0` to `+= 1.0000001`
did **not** fail any test — correctly so, since `printf("%.1f")` rounds the
difference away and the two libraries remain observably identical. That is a
property of the C's own output format, not a harness gap.
