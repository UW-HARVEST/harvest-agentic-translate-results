# CONFIGS.md — Configuration-surface table (Phase B gate)

## Mechanical derivation of the axes

Derived from `c_src/include/driver.h` and `c_src/src/driver.c`, not from
assumptions.

**Axis 1 — public entry points** (`nm -D --defined-only`, both are `T`):

| entry point | level | declared in header? | body |
|-------------|-------|---------------------|------|
| `run(int extra_bedrooms)` | **lowest-level exported entry point** | no (external linkage only) | the full 4-print / 3-mutation pipeline |
| `driver(int x)` | convenience wrapper | yes | `run(x); run(x);` |

Tests must exercise `run` **directly**, not only through `driver`, because
`driver` can only ever produce the *pair* pattern and would hide any divergence
that depends on an odd number of pipeline applications.

**Axis 2 — runtime options / modes / flags:** *none.* `grep -nE 'if|switch|#if'`
over the C source finds no branch on any flag; there is no setter, no init
function, no global config, and no `#ifdef`-selected behaviour (the only `#if` is
the `DRIVER_H_` include guard). The single argument is a pure data value, not a
mode selector.

**Axis 3 — input shape of the one argument (`int`):** zero / positive / negative,
small magnitude / maximum magnitude, one step inside each range end, and
uniformly random over the whole `i32` domain.

**Axis 4 — accumulated global state.** This is the real second dimension of the
configuration surface. `static house_t the_house` (`driver.c:35`) is *persistent
and mutable*, and every `run` mutates all three of its fields:

- `floors` `+1` per `run` → drives the `%d` field width of column 1
- `bathrooms` `+1.0` per `run` → drives the `%.1f` field width of column 3
- `bedrooms` `+= extra_bedrooms` per `run` → the value-dependent, overflow-prone field

So the output of a call is a function of `(argument, entire prior call history)`.
Rows therefore vary *call count*, *call sequencing*, and *mix of `run` vs
`driver`*, not just the argument.

**Axis 5 — call-sequence shape:** empty history (pristine) / one call / many
calls; homogeneous `run` sequence / homogeneous `driver` sequence / interleaved
`run`+`driver`.

## Configuration-surface table

Every row is driven with **many fixed-seed randomized inputs** (a `SplitMix64`
PRNG seeded from a per-row constant in `tests/differential.rs`), not a single
hand-picked value, except where the row's whole point is a specific boundary
value. Every row compares the C `.so` and the Rust `.so` byte-for-byte through
`libloading`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `run` | pristine global state (`floors=2, bedrooms=5, bathrooms=2.5`), `extra_bedrooms = 0` — also pinned against the literal expected text derived from the C source | `cfg_01_pristine_state_exact_text` | [x] |
| 2 | `run` | accumulated state, `extra_bedrooms = 0` (no-op delta) × many iterations | `cfg_02_run_zero_delta_repeated` | [x] |
| 3 | `run` | accumulated state, `extra_bedrooms` random **small positive** (`1..=1000`), 400 iterations | `cfg_03_run_small_positive_random` | [x] |
| 4 | `run` | accumulated state, `extra_bedrooms` random **small negative** (`-1000..=-1`), 400 iterations | `cfg_04_run_small_negative_random` | [x] |
| 5 | `run` | accumulated state, `extra_bedrooms` **uniform over the full `i32` domain**, 500 iterations (exercises wrapping in both directions from arbitrary starting values) | `cfg_05_run_full_i32_random` | [x] |
| 6 | `run` | accumulated state, `extra_bedrooms` ∈ the exhaustive boundary set `{0, 1, -1, 2, -2, i32::MAX, i32::MIN, i32::MAX-1, i32::MIN+1}` | `cfg_06_run_boundary_set` | [x] |
| 7 | `driver` | accumulated state, `x = 0` — verifies the wrapper applies `run` exactly twice | `cfg_07_driver_zero_delta` | [x] |
| 8 | `driver` | accumulated state, `x` random small positive/negative, 300 iterations | `cfg_08_driver_small_random` | [x] |
| 9 | `driver` | accumulated state, `x` uniform over the full `i32` domain, 400 iterations (double wrap per call) | `cfg_09_driver_full_i32_random` | [x] |
| 10 | `driver` | accumulated state, `x` ∈ the same exhaustive boundary set as row 6 | `cfg_10_driver_boundary_set` | [x] |
| 11 | `run` + `driver` **interleaved** | randomized choice of entry point per step (so `run` is applied an odd *and* even number of times), random full-`i32` arguments, 500 steps — the composed-pipeline row that per-wrapper tests cannot see | `cfg_11_interleaved_run_and_driver_random` | [x] |
| 12 | `run` | long homogeneous sequence (1 200 calls) so `floors` crosses 1→2→3→4 digits and `bathrooms` crosses `9.5→10.5`, `99.5→100.5`, `999.5→1000.5` — `printf` field-width growth in two columns at once | `cfg_12_long_sequence_field_width_growth` | [x] |
| 13 | `run` | state walked to `bedrooms == i32::MAX` exactly, then stepped across the overflow boundary with `+1`, `+2`, random | `cfg_13_walk_to_max_then_cross` | [x] |
| 14 | `run` | state walked to `bedrooms == i32::MIN` exactly, then stepped across the underflow boundary with `-1`, `-2`, random | `cfg_14_walk_to_min_then_cross` | [x] |
| 15 | `run` | state walked to `bedrooms == 0` and to `bedrooms == -1` exactly (sign-transition of the `%d` column) | `cfg_15_bedrooms_sign_transition` | [x] |
| 16 | `run` / `driver` | same argument replayed many times consecutively (`run(k)` ×50, then `driver(k)` ×50) for several random `k` — confirms identical *accumulation*, not just identical single-shot output | `cfg_16_repeated_same_argument_accumulation` | [x] |
| 17 | `run` / `driver` | high-volume mixed soak: 3 000 randomized steps over the full `i32` domain with a fixed seed, comparing every step | `cfg_17_soak_mixed_random` | [x] |

Rows deliberately **not** in the table, with justification:

- Element types / widths / byte order / element counts / formats: no such axis
  exists — the entire ABI is `void f(int)`, with no buffers, arrays, or
  serialization.
- Empty / one / many *collections*: no collection parameters. The analogous axis
  is call-history length, covered by rows 1 (empty), 2/7 (one) and 12/17 (many).
- Feature combinations: `Cargo.toml` declares no `[features]`, so the
  cross-product has exactly one member (see `SYMBOLS.md`).
