# CONFIGS.md — Configuration-surface table (Phase A → tested in Phase B)

## Axes mechanically derived from the C source

Public entry points (`nm -D` on the C `.so`, plus `include/lib.h`):

* `gotomach(int iterations, int seed, int mode, int threshold)` — the one-shot
  driver (`include/lib.h`).
* `process_value(int, int, void*)` — **low-level** `operation_fn`, exported.
* `double_value(int, int, void*)` — **low-level** `operation_fn`, exported.
* `triple_value(int, int, void*)` — **low-level** `operation_fn`, exported.

Branch axes the C code actually distinguishes:

| axis | values the C branches on | source |
|------|--------------------------|--------|
| `mode` (op selector) | `0` → `process_value` (`v+10`), `1` → `double_value` (`v*2`), `2` → `triple_value` (`v*3`), anything else → `default:` warn + `process_value` | `switch (mode)` lines 126–140 |
| `iterations` (capacity **and** loop count **and** both `malloc` sizes) | `0` (empty: `malloc(0)`, loop skipped, empty sum), `1`, `2`, many, `65534`, `65535` (== `UINT16_MAX`, the only value that can make `count` hit the saturation `break`) | lines 114, 142, 149, 163 |
| `seed` (initial `current_value`) | `0` (fixed point for `*2`/`*3`), `1`, mid, `65534`, `65535` (max, only value where the very first product exceeds 4 digits) | lines 120, 162 |
| `threshold` (acceptance filter, **strict** `<`) | `INT_MIN` → accept none (`count == 0`, sum `0`), `< 0` → accept none, `0` → accept none (all produced values are `>= 0` for valid seeds), mid → accept some, `== produced value` → reject that value (strict `<` boundary), `INT_MAX` → accept all | line 172 |
| `state->count` saturation | `count >= UINT16_MAX` → `[WARNING] Reached maximum count` + `break` before the final `i++` | line 178 |
| feedback shape | `current_value = produced % 1000` makes the sequence value-dependent and mode-dependent (cycles: `+10` walks 0…999 and wraps; `*2` reaches the `0`/`>=500` doubling region; `*3` likewise) | line 176 |
| `unused_param` / `unused_context` of the three ops | ignored in every branch — but they are real FFI inputs (`NULL` and non-`NULL`) | lines 59–75 |
| build-time config | **none** — no `[features]` in `Cargo.toml`, no `#ifdef`/`option()`/`target_compile_definitions` in `CMakeLists.txt` | — |

Every row below is driven with **many randomized inputs** for the free axes
(fixed-seed xorshift64\* PRNG, so runs are reproducible), and both the C `.so`
and the Rust `.so` are called through `libloading`; the `int` return **and** the
captured `stdout` bytes are compared.

## Rows

### `gotomach` — mode × threshold regime (free axes: `iterations` ∈ [0,300], `seed` ∈ [0,65535], 200 random draws per row)

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `gotomach` | `mode=0` (`+10`), `threshold=INT_MIN` → accept none | [x] |
| 2  | `gotomach` | `mode=0`, `threshold` random negative → accept none | [x] |
| 3  | `gotomach` | `mode=0`, `threshold=0` → accept none | [x] |
| 4  | `gotomach` | `mode=0`, `threshold` random in [1,1100] → partial accept | [x] |
| 5  | `gotomach` | `mode=0`, `threshold` == a value the sequence actually produces → strict-`<` boundary rejects it | [x] |
| 6  | `gotomach` | `mode=0`, `threshold=INT_MAX` → accept all | [x] |
| 7  | `gotomach` | `mode=1` (`*2`), `threshold=INT_MIN` | [x] |
| 8  | `gotomach` | `mode=1`, `threshold` random negative | [x] |
| 9  | `gotomach` | `mode=1`, `threshold=0` | [x] |
| 10 | `gotomach` | `mode=1`, `threshold` random in [1,2100] → partial accept | [x] |
| 11 | `gotomach` | `mode=1`, `threshold` == produced value (strict-`<` boundary) | [x] |
| 12 | `gotomach` | `mode=1`, `threshold=INT_MAX` | [x] |
| 13 | `gotomach` | `mode=2` (`*3`), `threshold=INT_MIN` | [x] |
| 14 | `gotomach` | `mode=2`, `threshold` random negative | [x] |
| 15 | `gotomach` | `mode=2`, `threshold=0` | [x] |
| 16 | `gotomach` | `mode=2`, `threshold` random in [1,3100] → partial accept | [x] |
| 17 | `gotomach` | `mode=2`, `threshold` == produced value (strict-`<` boundary) | [x] |
| 18 | `gotomach` | `mode=2`, `threshold=INT_MAX` | [x] |
| 19 | `gotomach` | `mode` random ∉ {0,1,2} (`default:` warn path), `threshold=INT_MIN` | [x] |
| 20 | `gotomach` | `mode` ∉ {0,1,2}, `threshold` random negative | [x] |
| 21 | `gotomach` | `mode` ∉ {0,1,2}, `threshold=0` | [x] |
| 22 | `gotomach` | `mode` ∉ {0,1,2}, `threshold` random in [1,1100] | [x] |
| 23 | `gotomach` | `mode` ∉ {0,1,2}, `threshold` == produced value | [x] |
| 24 | `gotomach` | `mode` ∉ {0,1,2}, `threshold=INT_MAX` | [x] |

### `gotomach` — `iterations` shape boundaries (free axes: `seed`, `threshold`; all four `mode` classes each; 64 random draws per row)

| #  | entry point(s) | configuration | [ ] |
|----|----------------|---------------|-----|
| 25 | `gotomach` | `iterations=0` — `malloc(0)`, loop skipped, empty sum | [x] |
| 26 | `gotomach` | `iterations=1` — single element | [x] |
| 27 | `gotomach` | `iterations=2` — first feedback step exercised | [x] |
| 28 | `gotomach` | `iterations=3` | [x] |
| 29 | `gotomach` | `iterations` random in [4,64] | [x] |
| 30 | `gotomach` | `iterations` random in [1000,4096] — long feedback cycles | [x] |
| 31 | `gotomach` | `iterations=65534` (one below `UINT16_MAX`) | [x] |
| 32 | `gotomach` | `iterations=65535` (`== UINT16_MAX`, max capacity) | [x] |

### `gotomach` — `seed` shape boundaries (free axes: `iterations`, `threshold`, all mode classes; 64 random draws per row)

| #  | entry point(s) | configuration | [ ] |
|----|----------------|---------------|-----|
| 33 | `gotomach` | `seed=0` — fixed point of `*2` and `*3` | [x] |
| 34 | `gotomach` | `seed=1` | [x] |
| 35 | `gotomach` | `seed=999` / `1000` / `1001` (the `% 1000` boundary) | [x] |
| 36 | `gotomach` | `seed=65534` | [x] |
| 37 | `gotomach` | `seed=65535` (max valid; first product is 5–6 digits) | [x] |

### `gotomach` — `count` saturation interaction

| #  | entry point(s) | configuration | [ ] |
|----|----------------|---------------|-----|
| 38 | `gotomach` | `iterations=65535`, `threshold=INT_MAX`, `mode=0` → `count` reaches `UINT16_MAX`, `[WARNING] Reached maximum count` + `break` | [x] |
| 39 | `gotomach` | `iterations=65535`, `threshold=INT_MAX`, `mode=1` → saturation | [x] |
| 40 | `gotomach` | `iterations=65535`, `threshold=INT_MAX`, `mode=2` → saturation | [x] |
| 41 | `gotomach` | `iterations=65535`, `threshold=INT_MAX`, `mode=7` (`default:`) → saturation | [x] |
| 42 | `gotomach` | `iterations=65535`, `threshold` mid (partial accept, `count` stays below `UINT16_MAX`, no warning), all modes | [x] |

### Low-level `operation_fn` entry points called directly (the exported ops)

| #  | entry point(s) | configuration | [ ] |
|----|----------------|---------------|-----|
| 43 | `process_value` | `value` ∈ full-range random `i32`, `unused_param` random, `unused_context = NULL` | [x] |
| 44 | `process_value` | `value` ∈ {`INT_MIN`, `INT_MIN+1`, `-11`,`-10`,`-9`, `-1`, `0`, `1`, `INT_MAX-10`, `INT_MAX-9`, `INT_MAX-1`, `INT_MAX`} (wrap boundary), `unused_context` non-NULL | [x] |
| 45 | `double_value`  | `value` ∈ full-range random `i32`, `unused_context = NULL` | [x] |
| 46 | `double_value`  | `value` ∈ {`INT_MIN`, `INT_MIN/2`, `-1`, `0`, `1`, `INT_MAX/2`, `INT_MAX/2+1`, `INT_MAX`} (wrap boundary), `unused_context` non-NULL | [x] |
| 47 | `triple_value`  | `value` ∈ full-range random `i32`, `unused_context = NULL` | [x] |
| 48 | `triple_value`  | `value` ∈ {`INT_MIN`, `INT_MIN/3`, `-1`, `0`, `1`, `INT_MAX/3`, `INT_MAX/3+1`, `INT_MAX`} (wrap boundary), `unused_context` non-NULL | [x] |
| 49 | `process_value`, `double_value`, `triple_value` | composed the way `gotomach` composes them: feed `produced % 1000` back in for 4096 steps from random seeds, comparing every step (pipeline-level differential, not per-call) | [x] |

### Exhaustive / saturating sweeps (cheap because the domain is small)

| #  | entry point(s) | configuration | [ ] |
|----|----------------|---------------|-----|
| 50 | `gotomach` | exhaustive over `mode` ∈ [-4,6] × `iterations` ∈ [0,40] × `seed` ∈ {0,1,7,999,1000,65535} × `threshold` ∈ {INT_MIN,-1,0,1,15,1000,1010,2000,3000,INT_MAX} | [x] |
| 51 | `gotomach` | 20 000 fully-random `(iterations, seed, mode, threshold)` tuples drawn from the *whole* `i32` domain (valid and invalid mixed) | [x] |

## Phase B result

All 51 rows verified: `tests/phase_b_valid.rs` contains exactly one `#[test]`
per row (`row_01_…` … `row_51_…`). Each row drives BOTH `.so`s through
`libloading` and compares, byte for byte:

* the `int` return value of every single input, and
* the **complete stdout byte stream** of the whole batch (the `printf`/`puts`
  log lines), captured by redirecting fd 1.

```
running 51 tests
...
test result: ok. 51 passed; 0 failed
```

Run count per configuration: ~68 000 `gotomach` calls and ~110 000 direct
`operation_fn` calls per implementation.

### Anti-vacuity evidence (mutation testing)

The harness was validated by injecting deliberate bugs into `src/lib.rs`,
rebuilding, and confirming the suite fails — then restoring the original:

| injected bug | detected |
|--------------|----------|
| `process_value` returns `v+11` | yes (26 rows failed) |
| `double_value` returns `v*3` | yes (21 rows) |
| acceptance test `<=` instead of `<` | yes (23 rows) |
| feedback `% 1001` instead of `% 1000` | yes (30 rows) |
| final sum skips the last element | yes (31 rows) |
| `iterations` upper bound off by one | yes (7 rows) |
| saturation check `>` instead of `>=` | yes (7 rows) |
| `mode == 3` mapped to `double_value` | yes |
| `iterations`/`seed` guard order swapped | yes |
| `malloc(0)` treated as an allocation failure | yes (17 rows) |
| `init_processor` sets `status = 0` | yes (44 rows) |
| `[WARNING] Invalid mode…` log removed | yes (23 rows — stdout diff) |
| `[INFO] Starting gotomach function` log removed | yes (12 rows) |
| `free(temp_buffer)` removed (leak) | yes (`d5` heap-growth test) |
| `is_valid_state` `<=` instead of `<` | **not detected — provably equivalent**: `count <= i < iterations == capacity`, so `count < capacity` always holds and that branch is unreachable through the public API (see `ERRORS.md` rows 8/17) |
