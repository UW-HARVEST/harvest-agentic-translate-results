# CONFIGS.md — Phase B configuration surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axis 0 — compile-time configuration

`grep -c '#if\|#ifdef\|#ifndef' c_src/src/lib.c` → **0**. The C has no
conditional compilation. `translation/Cargo.toml` has **no `[features]`
section**, so the only feature combination is the default (empty) one:

| combo | command |
|-------|---------|
| default (no features exist) | `cargo test` |
| `--no-default-features` | equivalent — no default feature set to remove |

There is no `[[bin]]` target and `add_library(... SHARED)` is the only CMake
target, so **the project builds no binary/driver** — the stdout-comparison
requirement is not applicable.

## Axis 1 — runtime state (the library's real "options")

The C library's behaviour is controlled by three file-scope `static int`s that
the public API mutates. They are the configuration knobs:

| knob | initial | mutated by | branches it drives |
|------|---------|-----------|--------------------|
| `accumulator` | `0` | `add_to_accumulator`, `subtract_from_accumulator` | `accumulator > 0150` (line 142), `!!accumulator` (line 153) |
| `multiplier` | `1` | `multiply_with_multiplier`, `divide_multiplier` | `multiplier > 0100` (line 161), `!!multiplier` (line 154) |
| `operation_count` | `0` | all four `operations[]` | `result += operation_count * 010` (line 166) |

Because state persists across calls, a *configuration* is a (call sequence,
argument shape) pair. Each `.so` instance is loaded from a **freshly copied
unique path** so every row starts from pristine state (`dlopen` of a distinct
path gives fresh statics).

## Axis 2 — dispatch modes

`operations[4]` is the function-pointer table; `mode_add=01`, `mode_multiply=02`,
`mode_subtract=03`, `mode_divide=04` are the thresholds compared against
`active_params` (0–4). Selected slots: `[0]` add, `[1]` multiply, `[2]`
subtract, `[3]` divide.

## Axis 3 — input shapes

Value classes the code actually distinguishes (from `validate_and_normalize`
and the guards): `INT_MIN`, negative, `0`, `1..63` (below `0100`), `63`, `64`,
`65..510`, `511` (`0777`), `512`, large positive, `INT_MAX`. For `char*`:
empty / one char / many chars, needle at first / middle / last / absent
position, needle `> 255` / `< 0` / `0`.

## Configuration rows

Every row is run against **both** `.so`s with **many randomized inputs**
(fixed seed `0x5EED_1234`, xorshift64* PRNG) except where the row names an
exact value. Low-level entry points are driven directly, not only via
`findrep`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `validate_and_normalize` | stateless; randomized full `int` range (all value classes hit) | [x] |
| 2  | `validate_and_normalize` | stateless; every boundary exactly: `INT_MIN, -1, 0, 1, 63, 64, 65, 510, 511, 512, INT_MAX` | [x] |
| 3  | `process_octal_string` | fresh state; randomized `int` (incl. negative → `%o` as unsigned), 100-byte dest, full byte-buffer compared | [x] |
| 4  | `process_octal_string` | fresh state; exact values `0, 1, 7, 8, 0123, -1, INT_MIN, INT_MAX` | [x] |
| 5  | `find_and_replace_char` | needle present exactly once, mid-string | [x] |
| 6  | `find_and_replace_char` | needle present multiple times → only **first** replaced | [x] |
| 7  | `find_and_replace_char` | needle at index 0 / at last index | [x] |
| 8  | `find_and_replace_char` | randomized ASCII strings × randomized needle in `-512..=1023` (exercises `unsigned char` truncation both ways) | [x] |
| 9  | `add_to_accumulator` | fresh state, single call, randomized `(a,b)` incl. overflow-inducing magnitudes | [x] |
| 10 | `add_to_accumulator` | fresh state, long randomized call sequence (state accumulates, wraps) | [x] |
| 11 | `subtract_from_accumulator` | fresh state (`accumulator == 0`), randomized `(a,b)` | [x] |
| 12 | `subtract_from_accumulator` | after `add_to_accumulator` calls (non-zero accumulator), randomized | [x] |
| 13 | `multiply_with_multiplier` | fresh state (`multiplier == 1`), randomized `(a,b)`; drives `multiplier` to 0 / negative / wrapped | [x] |
| 14 | `divide_multiplier` | `multiplier` seeded large positive, randomized `b != 0` incl. negatives | [x] |
| 15 | `divide_multiplier` | `b == 0` guard, repeated (count still increments) | [x] |
| 16 | mixed low-level | randomized interleaving of all four `operations[]` targets, 200 calls, return value compared at **every** step | [x] |
| 17 | `findrep` | fresh state; `active_params == 0` (all params zero) → neither add nor multiply branch | [x] |
| 18 | `findrep` | fresh state; `active_params == 1` (each of the 4 single-nonzero placements) → add only | [x] |
| 19 | `findrep` | fresh state; `active_params == 2` (all 6 placements) → add + multiply | [x] |
| 20 | `findrep` | fresh state; `active_params == 3` (all 4 placements) | [x] |
| 21 | `findrep` | fresh state; `active_params == 4` | [x] |
| 22 | `findrep` | fresh state; params chosen so `accumulator <= 0150` → subtract branch **not** taken | [x] |
| 23 | `findrep` | fresh state; params chosen so `accumulator > 0150` → subtract branch taken | [x] |
| 24 | `findrep` | state pre-driven so `multiplier > 0100` → divide branch taken | [x] |
| 25 | `findrep` | state pre-driven so `multiplier == 0` → `both_active` false | [x] |
| 26 | `findrep` | state pre-driven so `accumulator == 0` (via add/subtract) → `both_active` false | [x] |
| 27 | `findrep` | fresh state; params all negative (pass through `validate_and_normalize` unclamped) | [x] |
| 28 | `findrep` | fresh state; params all `> 0777` (all clamped to 511) | [x] |
| 29 | `findrep` | fresh state; params in `1..63` (all clamped up to 64) | [x] |
| 30 | `findrep` | fresh state; params at exact boundaries `{0,1,63,64,511,512,INT_MIN,INT_MAX}` cross-product sample | [x] |
| 31 | `findrep` | fresh state; single call, fully randomized 4-tuples (wide range) | [x] |
| 32 | `findrep` | fresh state; **repeated** calls (300) with randomized params — state carried between calls, return compared every call | [x] |
| 33 | `findrep` interleaved with low-level ops | randomized mix of `findrep` and the 4 accumulator/multiplier functions, 300 steps, every return compared | [x] |
| 34 | `findrep` + `process_octal_string` + `find_and_replace_char` | full pipeline: run `findrep`, then reproduce its internal string steps externally and compare buffers | [x] |
| 35 | all 8 entry points | one long randomized program (500 steps) over a single library instance, comparing every scalar return **and** every buffer byte | [x] |

## Phase B results

Test file: `tests/phase_b_valid.rs` — one `#[test]` per row, named `rowNN_…`.
Harness: `tests/common/mod.rs`. Both `.so`s are loaded with `libloading`; the
Rust side is only ever reached through its exported C symbols.

```
cargo test --test phase_b_valid
test result: ok. 35 passed; 0 failed
```

All 35 rows pass, under both the `dev` and `release` (`panic = "abort"`,
optimised) profiles, and under both feature combinations (see `scripts/phase_d.sh`).

State isolation: `dlopen` refcounts by path, so a single `Library::new` on the
same path would silently share `accumulator`/`multiplier`/`operation_count`
between tests and make rows pass vacuously. `Pair::fresh()` therefore copies
each `.so` to a unique temp path before loading. `harness_state_isolation` in
`tests/phase_c_errors.rs` asserts the statics really are pristine
(`add(1,1) == 2`, `mul(3,5) == 15`) on every fresh pair.

Randomization: xorshift64\* seeded from the fixed constant `0x5EED_1234`
(xor-ed per row), so every run is reproducible. `Rng::interesting_i32()` mixes
the 20 boundary values the C branches on with narrow-range and full-width
draws. Iteration counts per row range from 200 to 20 000; row 30 is an
exhaustive 8⁴ = 4096 boundary cross-product.
