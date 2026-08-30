# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for VALID inputs. Derived mechanically from the axes
`c_src/src/driver.c` actually branches on.

## Axes the C code branches on

Grep basis: `grep -n 'if (\|switch\|#if\|static\|printf' c_src/src/driver.c`

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| A1 — public entry points | exactly one: `driver(int x, int local_y, int z)` (the `.so`'s only export). Lowest-level function `multi_stage(int x, int z)` is `static`, so it is only reachable *through* `driver`; it is exercised via `driver` in every row. | `driver.h`, `driver.c:31,59` |
| A2 — `x` | `x == 1` vs `x != 1` | `driver.c:33` |
| A3 — `y` (set from `local_y`, arg 2) | `y == 2` vs `y != 2` | `driver.c:39,60` |
| A4 — `z` | `z == 3` vs `z != 3` | `driver.c:45` |
| A5 — control-flow shape | straight-line success `return result` vs `goto fail` epilogue | `driver.c:51-56` |
| A6 — persistent state | file-scope `static int y = 123;` survives across calls; `driver` unconditionally overwrites it before use, so the initialiser `123` is never observable, but multi-call sequences are a distinct shape | `driver.c:29,60` |
| A7 — value shape / width | all three parameters are C `int`: negative, zero, positive, `INT_MIN`, `INT_MAX`; no other element types, no widths, no byte-order, no counts, no formats, no runtime option flags, no `#ifdef` modes exist in this library | `driver.h:27` |
| A8 — output channel | stdout via `printf`; 5 distinct format strings, 4 of them constant-only (compiled to `puts`) and one `%d` conversion | `driver.c:34,40,46,51,55,62` |

There are **no runtime options, modes or flags** in this API (no setters, no
context struct, no env vars, no compile-time `#ifdef`s), so the
cross-product is exactly A2 × A3 × A4 × A7, plus the call-sequence shape A6.

Observable output = the exact bytes written to `stdout` by one `driver` call
(the function returns `void`). Every row compares C vs Rust stdout
byte-for-byte.

## Configuration rows

Each row is run against BOTH `.so`s through `libloading` with **many
randomized inputs** (fixed-seed xorshift64* PRNG, seed `0x2025_0828_D817_ACE1`),
except where the row pins exact values by construction.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` → `multi_stage` | success path: `x=1, y=2, z=3` exactly (the only accepting configuration); repeated many times to confirm idempotence | [x] |
| C2 | `driver` → `multi_stage` | `x != 1`, `y` and `z` both randomized over the full `i32` range (branch A2 taken, A3/A4 unreached) | [x] |
| C3 | `driver` → `multi_stage` | `x != 1`, `y = 2`, `z = 3` (only `x` invalid — proves the `x` check is evaluated first even when the rest is valid) | [x] |
| C4 | `driver` → `multi_stage` | `x = 1`, `y != 2` randomized, `z` randomized over full `i32` (branch A3 taken, A4 unreached) | [x] |
| C5 | `driver` → `multi_stage` | `x = 1`, `y != 2` randomized, `z = 3` (only `y` invalid) | [x] |
| C6 | `driver` → `multi_stage` | `x = 1`, `y = 2`, `z != 3` randomized over full `i32` (branch A4 taken) | [x] |
| C7 | `driver` → `multi_stage` | fully unconstrained random triples over the full `i32` range (all four outcomes reached by chance, incl. occasional `1`/`2`/`3` hits) | [x] |
| C8 | `driver` → `multi_stage` | small-neighbourhood exhaustive sweep: `x,y,z ∈ [-4, 8]` — the complete 13³ = 2197 cross-product around the accepting point | [x] |
| C9 | `driver` → `multi_stage` | boundary/extreme value shape: each of `x,y,z` drawn from `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, 3, 4, 123, INT_MAX-1, INT_MAX}` — full 12³ = 1728 cross-product | [x] |
| C10 | `driver` → `multi_stage` | randomized inputs biased toward the interesting values `{1,2,3}` so mixed pass/fail per-argument combinations occur densely | [x] |
| C11 | `driver` (state shape A6) | first-ever call after library load, before `static y` has been written: verifies the `y = 123` initialiser is equally unobservable in both builds | [x] |
| C12 | `driver` (state shape A6) | multi-call sequences: randomized sequences of 64 calls per library, run in the same order against both, comparing the concatenated stdout — catches any divergence in persistence of the `static y` | [x] |
| C13 | `driver` (state shape A6) | success-then-failure and failure-then-success alternation, plus success repeated after `y` was clobbered by a failing call | [x] |
| C14 | `driver` (output shape A8) | `Result: %d` conversion for every reachable status code `{0,1,2,3}` — exact byte-level check of the one non-constant format string | [x] |
| C15 | `driver` (output shape A8) | no trailing/leading extra bytes: total stdout length and full byte content asserted equal, not just prefix-matched, for all rows | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
build configurations are:

| combo | command |
|-------|---------|
| default (= empty feature set) | `cargo test --release` |
| `--no-default-features` (identical, no features exist) | `cargo test --release --no-default-features` |

Both are run by `run_all.sh`; there are no other feature axes to cross.
