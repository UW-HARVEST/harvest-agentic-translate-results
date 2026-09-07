# CONFIGS.md — Phase B configuration-surface table

## Axes, derived mechanically from the C source

`c_src/src/staticalias.c` has exactly one branch and one loop (see the grep in
`ERRORS.md`). Enumerating what the code actually distinguishes:

**Axis E — entry point** (from `c_src/include/staticalias.h`, the complete
public API — two symbols, no others; see `SYMBOLS.md`):
- `E1` = `static_alias` called **directly** — the lowest-level entry point.
- `E2` = `driver` — the convenience/one-shot wrapper that loops `static_alias`
  and `printf`s. Its stdout is part of its observable output.
- `E3` = **interleaved** `E1` and `E2` on the same loaded library, exercising the
  interaction through the shared `static int inner`.

**Axis B — branch of `if (*outer >= inner)` (`staticalias.c:30`):**
- `B1` = then-branch (`*outer >= inner`): `inner += *outer`, **returns `&inner`**.
- `B2` = else-branch (`*outer < inner`): `*outer += inner`, **returns `outer`**.

**Axis A — aliasing of the argument** (the reason this library exists; `driver`
feeds the returned pointer straight back in, so after any `B1` the argument
*is* `&inner`):
- `A1` = `outer` points at a caller-owned object distinct from `inner`.
- `A2` = `outer == &inner` (aliased) ⇒ `B1` computes `inner + inner` and `B2` is
  unreachable (`inner >= inner` is always true).

**Axis S — state of the persistent `static int inner` at call time** (it is
process-global and survives across calls; `inner` starts at 1):
- `S1` = fresh library, `inner == 1`.
- `S2` = accumulated positive `inner`.
- `S3` = negative `inner` (reachable: `inner==1`, feed `1`⇒`2`, … or drive it
  negative through overflow, then `*outer >= inner` holds for negatives).
- `S4` = `inner` at/near `INT_MAX` (overflow region).

**Axis V — value shape of `*outer` / `initial_value`** (every `int` is accepted;
the code distinguishes only via `>=`, but the *arithmetic* is value-dependent):
- `V1` positive, `V2` zero, `V3` negative, `V4` exactly `inner`,
  `V5` `inner ± 1`, `V6` `INT_MAX`, `V7` `INT_MIN`, `V8` uniform random `i32`.

**Axis I — `iterations` shape for `driver`** (`for (i = 0; i < iterations; i++)`):
- `I0` = 0, `I1` = negative, `I2` = 1, `I3` = 2 (first state carry-over),
  `I4` = many (3…4096, exercises the `B1`→aliased-`A2` steady state).

**Axis P — observable return-pointer identity** (must be compared, not just the
pointed-to value): `P1` = returned pointer lies in the library's own data
segment (`&inner`, i.e. *not* the caller's object); `P2` = returned pointer is
bit-identical to the `outer` argument passed in.

**No other axes exist in the API.** There is no runtime option, mode, flag, or
setter; there is no `#ifdef` in the C source; `translation/Cargo.toml` declares
**no `[features]`**, so the default build is the only feature configuration
(asserted by `feature_surface_is_singular` in `tests/phase_d_parity.rs`, so it
cannot silently change).

**Axis Z — build configuration.** Because there are no cargo features, the
configurations that *do* differ in generated code are the build profiles. Every
row below is therefore run against four artifacts by `./verify.sh`:
`{default, --no-default-features} × {release, debug}`. The debug profile is not
cosmetic — it enables `debug-assertions`, which inserts `core`'s pointer
preconditions, and it is what exposed the misaligned-pointer divergence recorded
in `ERRORS.md`. The `.so` under test is selected by the `STATICALIAS_C_SO` /
`STATICALIAS_RUST_SO` environment variables.

Every row is run with **many randomized inputs** (`SEED = 0x5A71C_A11A5`,
fixed-seed xorshift64* PRNG) rather than one hand-picked value, and every row
compares, between the C and the Rust `.so`: the returned pointer's *identity
class* (`P1` vs `P2`), the `int` value it points to, the post-call value of the
caller's own object, the post-call value of `inner` (read back via a probe call),
and — for `driver` rows — stdout byte-for-byte.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `static_alias` | `S1` fresh (`inner==1`), `A1` distinct, `B1` then-branch, `V1` random positive `1..=INT_MAX` | `cfg_row01` | [x] |
| 2 | `static_alias` | `S1` fresh, `A1` distinct, `B2` else-branch, `V3`/`V2` random `INT_MIN..=0` | `cfg_row02` | [x] |
| 3 | `static_alias` | `S1` fresh, `A1` distinct, `V4` `*outer == inner` exactly (`==1`) → `B1` boundary | `cfg_row03` | [x] |
| 4 | `static_alias` | `S1` fresh, `A1` distinct, `V5` `*outer == inner-1 == 0` → `B2` boundary | `cfg_row04` | [x] |
| 5 | `static_alias` | `S2` accumulated positive `inner`, `A1` distinct, `B1` (`*outer` random `>= inner`) | `cfg_row05` | [x] |
| 6 | `static_alias` | `S2` accumulated positive `inner`, `A1` distinct, `B2` (`*outer` random `< inner`) | `cfg_row06` | [x] |
| 7 | `static_alias` | `S3` negative `inner`, `A1` distinct, `B1` (random `*outer >= inner`, incl. negatives) | `cfg_row07` | [x] |
| 8 | `static_alias` | `S3` negative `inner`, `A1` distinct, `B2` (random `*outer < inner`) | `cfg_row08` | [x] |
| 9 | `static_alias` | `S4` `inner` near `INT_MAX`, `A1` distinct, `B2` then overflow on `*outer += inner` | `cfg_row09` | [x] |
| 10 | `static_alias` | `A2` **aliased** (`outer == &inner` from a previous return), `S1` fresh ⇒ `inner` doubles | `cfg_row10` | [x] |
| 11 | `static_alias` | `A2` aliased, repeated `k` times (2…40) ⇒ repeated doubling into overflow/wrap | `cfg_row11` | [x] |
| 12 | `static_alias` | `V8` fully random `i32` sequence, `A1`, length 1…64, **branch chosen by the data** (mixed `B1`/`B2`, `S` evolves freely) | `cfg_row12` | [x] |
| 13 | `static_alias` | `V8` random sequence that **feeds the returned pointer back in** (mixed `A1`/`A2` as the data dictates) — the real consumer pattern, low-level | `cfg_row13` | [x] |
| 14 | `static_alias` | `V6`/`V7` extremes only: `*outer ∈ {INT_MIN, -1, 0, 1, INT_MAX}` × `S1`, all 5, each on a fresh library | `cfg_row14` | [x] |
| 15 | `driver` | `I0` `iterations == 0`, `V8` random `initial_value` ⇒ no output | `cfg_row15` | [x] |
| 16 | `driver` | `I1` `iterations` random negative ⇒ no output | `cfg_row16` | [x] |
| 17 | `driver` | `I2` `iterations == 1`, `V1` random positive `initial_value` (`B1` on the first step) | `cfg_row17` | [x] |
| 18 | `driver` | `I2` `iterations == 1`, `V3` random negative `initial_value` (`B2` on the first step) | `cfg_row18` | [x] |
| 19 | `driver` | `I3` `iterations == 2`, `V8` random `initial_value` (first state carry-over, both branch orders) | `cfg_row19` | [x] |
| 20 | `driver` | `I4` `iterations` random `3..=64`, `V8` random `initial_value` — full pipeline, stdout compared | `cfg_row20` | [x] |
| 21 | `driver` | `I4` `iterations == 4096`, `V1` large positive `initial_value` — long run into the aliased steady state and overflow | `cfg_row21` | [x] |
| 22 | `driver` | `I4` `iterations == 4096`, `V7` `initial_value == INT_MIN` — long run from the negative extreme | `cfg_row22` | [x] |
| 23 | `driver` | extremes: `initial_value ∈ {INT_MIN,-1,0,1,INT_MAX}` × `iterations ∈ {0,-1,1,2,5}` (25 combos, fresh library each) | `cfg_row23` | [x] |
| 24 | `E3` interleaved | `driver(v,n)` **then** direct `static_alias` calls on the same library — wrapper-mutated `inner` observed by the low-level entry point | `cfg_row24` | [x] |
| 25 | `E3` interleaved | direct `static_alias` calls **then** `driver(v,n)` — low-level-mutated `inner` observed by the wrapper (stdout compared) | `cfg_row25` | [x] |
| 26 | `E3` interleaved | randomized interleaving of `static_alias` / `driver` (random op choice, values, counts) over 1…32 ops on one library instance | `cfg_row26` | [x] |

## Completion checklist

- [x] Every axis the C branches on is represented; rows are the pruned
      cross-product of the axes the code actually distinguishes.
- [x] The lowest-level entry point (`static_alias`) is driven directly, not only
      through the `driver` wrapper (rows 1–14), **and** the composed pipeline is
      driven end-to-end (rows 15–23), **and** their interaction through the
      shared static is driven (rows 24–26).
- [x] Every row uses many randomized inputs with a fixed seed.
- [x] Every row asserts pointer-identity class, pointed-to value, caller-object
      value, post-call `inner`, and (for `driver`) stdout byte-for-byte.
- [x] Every row passes under all four build configurations swept by
      `./verify.sh` (`{default, --no-default-features} × {release, debug}`).

## Harness validation (why a "pass" here means something)

A differential suite that cannot fail is worthless, so the harness was checked
against eight deliberately mutated Rust builds. Every mutation was caught by
both Phase B and Phase C (failure counts out of 28 / 23 tests):

| mutation | Phase B failures | Phase C failures |
|----------|------------------|------------------|
| `>=` → `>` in the predicate | 6 | 12 |
| then-branch `wrapping_add` → `saturating_add` | 16 | 13 |
| else-branch `wrapping_add` → `saturating_add` | 14 | 8 |
| else-branch returns `&inner` instead of `outer` | 28 | 24 |
| `INNER` initialised to `0` instead of `1` | 28 | 24 |
| `driver` loop `i < iterations` → `i <= iterations` | 12 | 6 |
| `driver` drops the `printf` | 11 | 5 |
| then-branch addend read from `inner` instead of `*outer` (aliasing) | 23 | 16 |

`libraries_are_distinct_objects` additionally proves the two `.so`s are separate
objects with separate `&inner`, so no row can pass by accidentally testing one
library twice, and `built_artifacts_are_not_stale` proves neither `.so` predates
its source.
