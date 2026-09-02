# CONFIGS.md — Configuration surface of `c_src/src/lib.c`

## How this table was derived

There are **no** compile-time options: `c_src/CMakeLists.txt` sets no
`option()`/`add_definitions`, and `lib.c` contains **no** `#ifdef` at all.
`translation/Cargo.toml` declares **no** `[features]`. So the "configuration"
axes are entirely *runtime* — the argument values that steer the `if` / `switch`
/ loop branches the C actually takes.

Axes enumerated from the source:

| axis | where it branches | distinct states |
|------|-------------------|-----------------|
| A. mode string | `classify_mode` `strcmp` chain, lib.c:30–39 | `"standard"`, `"enhanced"`, `"turbo"`, `"extreme"`, unmatched → 5 |
| B. `apply_multiplier` `level` | `switch`, lib.c:45–59, with fall-through 4→3→2→1→0 | `4`, `3`, `2`, `1`, `0`, `default` → 6 |
| C. `apply_multiplier` `base` | pure accumulator, no branch, but value-dependent overflow | small, `INT_MAX`, `INT_MIN`, random → 4 shapes |
| D. `(int)double` cast range | lib.c:66, lib.c:74 | in-range, over `INT_MAX`, under `INT_MIN`, NaN, ±inf, ±0, subnormal → 7 |
| E. `get_modified_time` offsets | lib.c:78–83 `int` products/sum | zero, positive, negative, overflowing → 4 |
| F. `hash_time_value` byte pattern | 8-iteration loop, `i % 4` shift, lib.c:88–93 | zero, all-`0xFF`, high-bit-set bytes, positive, negative, random → 6 |
| G. `modeselect` `mode_selector % 4` | lib.c:101 index into 4-elem array | `0`, `1`, `2`, `3` (negatives = UB, see ERRORS.md E29) → 4 |
| H. `modeselect` `complexity % 5` | feeds axis B | `0`,`1`,`2`,`3`,`4`, negative → 6 |
| I. `modeselect` `seed % 24` | feeds axis E as `offset_hours` | zero, positive, negative → 3 |
| J. `modeselect` `time_offset` | feeds axis E as `offset_days` **and** axis D via `factor2` | zero, positive, negative, overflowing → 4 |
| K. stdout side effects | 8 `printf` calls in `modeselect` | every call site's exact bytes, incl. `%.2e`, `%X` of negatives, `%ld` | — |

## Public entry points

All seven exported symbols are driven directly via the `.so`, lowest-level
first — **not** only through the `modeselect` convenience wrapper:

1. `classify_mode` (leaf)
2. `apply_multiplier` (leaf)
3. `convert_time_factor` (leaf)
4. `convert_negative_overflow` (leaf)
5. `hash_time_value` (leaf)
6. `get_modified_time` (leaf, calls `time()`)
7. `modeselect` (composes 1,2,5,6,3,4 + 8 `printf`s)

## Table

Every row is exercised against **both** `.so`s with many randomized inputs from a
fixed-seed PRNG (seed `0x5EED_1234_ABCD_0001`), not a single hand-picked value.
Row `[x]` means it passed across the whole randomized sweep.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1  | `classify_mode` | exact literal `"standard"` | [x] |
| C2  | `classify_mode` | exact literal `"enhanced"` | [x] |
| C3  | `classify_mode` | exact literal `"turbo"` | [x] |
| C4  | `classify_mode` | exact literal `"extreme"` | [x] |
| C5  | `classify_mode` | randomized ASCII strings, len 0..16 (mostly unmatched) | [x] |
| C6  | `classify_mode` | randomized byte strings over full `0x01..=0xFF`, len 0..16 (high-bit / `unsigned char` comparison) | [x] |
| C7  | `classify_mode` | randomized single-byte mutations of each of the 4 literals (one char flipped/dropped/appended) | [x] |
| C8  | `classify_mode` | very long strings (len 1..4096) sharing a prefix with a literal | [x] |
| C9  | `apply_multiplier` | `level = 4`, randomized `base` over full `i32` | [x] |
| C10 | `apply_multiplier` | `level = 3`, randomized `base` over full `i32` | [x] |
| C11 | `apply_multiplier` | `level = 2`, randomized `base` over full `i32` | [x] |
| C12 | `apply_multiplier` | `level = 1`, randomized `base` over full `i32` | [x] |
| C13 | `apply_multiplier` | `level = 0`, randomized `base` over full `i32` | [x] |
| C14 | `apply_multiplier` | `level` randomized over full `i32` × `base` randomized over full `i32` (hits `default` overwhelmingly) | [x] |
| C15 | `apply_multiplier` | `base` ∈ {`INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, `0`, `-1`} × `level` ∈ `-2..=6` (full cross-product, overflow boundary) | [x] |
| C16 | `convert_time_factor` | in-range: randomized `factor` in `(-2.147e-3, 2.147e-3)` so `factor*1e12` fits `int` | [x] |
| C17 | `convert_time_factor` | randomized `factor` in `(-1.0, 1.0)` (mixed in-range / overflow) | [x] |
| C18 | `convert_time_factor` | randomized `factor` over full magnitude ladder `1e-320 .. 1e308`, both signs | [x] |
| C19 | `convert_time_factor` | randomized arbitrary bit patterns reinterpreted as `f64` (includes NaNs, subnormals, inf) | [x] |
| C20 | `convert_time_factor` | boundary sweep: `factor*1e12` ≈ `INT_MAX`, `INT_MAX±1`, `INT_MIN`, `INT_MIN±1`, `±0.0`, `±1e-12` | [x] |
| C21 | `convert_negative_overflow` | in-range: randomized `value` in `(-2.147e-6, 2.147e-6)` so `value*-1e15` fits `int` | [x] |
| C22 | `convert_negative_overflow` | randomized `value` in `(-1.0, 1.0)` | [x] |
| C23 | `convert_negative_overflow` | randomized `value` over full magnitude ladder `1e-320 .. 1e308`, both signs | [x] |
| C24 | `convert_negative_overflow` | randomized arbitrary bit patterns reinterpreted as `f64` | [x] |
| C25 | `convert_negative_overflow` | boundary sweep incl. sign-flip cases `±0.0`, `∓INT_MIN`, `2147483648/-1e15` | [x] |
| C26 | `hash_time_value` | `t = 0`, `t = -1` (all `0xFF`), `t = i64::MIN`, `t = i64::MAX` | [x] |
| C27 | `hash_time_value` | randomized full-range `i64` (all 8 bytes vary, exercises `i % 4` shift wrap and high-bit `<< 24`) | [x] |
| C28 | `hash_time_value` | randomized small non-negative `t` (realistic `time_t >> 29` shape) | [x] |
| C29 | `hash_time_value` | randomized single-byte-set patterns `1 << k` for `k` in `0..64` | [x] |
| C30 | `get_modified_time` | `offset_days = 0`, `offset_hours = 0` | [x] |
| C31 | `get_modified_time` | randomized small positive `offset_days` (0..1000) × `offset_hours` (0..23) | [x] |
| C32 | `get_modified_time` | randomized small negative offsets, both args | [x] |
| C33 | `get_modified_time` | randomized full-range `i32` × `i32` (products and sum overflow `int`) | [x] |
| C34 | `get_modified_time` | boundary cross-product: both args ∈ {`INT_MIN`, `INT_MIN+1`, `-24855`, `-1`, `0`, `1`, `24855`, `INT_MAX-1`, `INT_MAX`} | [x] |
| C35 | `modeselect` | `mode_selector % 4 == 0` (`"standard"`), all other args 0 | [x] |
| C36 | `modeselect` | `mode_selector % 4 == 1` (`"enhanced"`), all other args 0 | [x] |
| C37 | `modeselect` | `mode_selector % 4 == 2` (`"turbo"`), all other args 0 | [x] |
| C38 | `modeselect` | `mode_selector % 4 == 3` (`"extreme"`), all other args 0 | [x] |
| C39 | `modeselect` | full cross-product `mode_selector % 4` (0..3) × `complexity % 5` (0..4) — 20 combos, return value **and** stdout bytes | [x] |
| C40 | `modeselect` | `complexity` negative (→ `apply_multiplier` `default`) × each `mode_selector % 4` | [x] |
| C41 | `modeselect` | `seed` negative → `seed % 24` negative `offset_hours`, × each `mode_selector % 4` | [x] |
| C42 | `modeselect` | `time_offset` negative → negative `offset_days` **and** positive `factor2`, × each `mode_selector % 4` | [x] |
| C43 | `modeselect` | `time_offset` / `seed` large enough to overflow `offset_days*86400` inside `get_modified_time` | [x] |
| C44 | `modeselect` | `seed` large so `factor1 = seed*1e8` overflows the `(int)` cast; incl. `INT_MAX`, `INT_MIN` | [x] |
| C45 | `modeselect` | randomized `(mode_selector≥0, time_offset, complexity, seed)` over full `i32` — return value only | [x] |
| C46 | `modeselect` | randomized as C45 — **stdout byte-for-byte** comparison of all 8 `printf` calls | [x] |
| C47 | `modeselect` | boundary cross-product: `mode_selector` ∈ {`0`,`4`,`INT_MIN`,`INT_MAX-3`,`INT_MAX`} × `complexity` ∈ {`INT_MIN`,`-1`,`0`,`4`,`INT_MAX`} × `seed` ∈ {`INT_MIN`,`-1`,`0`,`23`,`INT_MAX`} × `time_offset` ∈ {`INT_MIN`,`-1`,`0`,`1`,`INT_MAX`} | [x] |
| C48 | pipeline | `get_modified_time` → `hash_time_value` composed (the exact call chain lib.c:114–115), randomized offsets | [x] |
| C49 | pipeline | `classify_mode` fed by the same 4-element mode table `modeselect` uses, all indices | [x] |
| C50 | pipeline | `convert_time_factor`/`convert_negative_overflow` fed by `seed*1e8` / `time_offset*-1e7` exactly as lib.c:120–121, randomized | [x] |

## Where each row is tested

- **C1–C34, C48–C50** → `translation/tests/phase_b_leaves.rs` (30 tests), one per
  row, named after the row. These drive the six LOW-LEVEL entry points directly
  through `dlsym`, not through `modeselect`.
- **C35–C47** → `translation/tests/phase_b_modeselect.rs` (12 tests), the composed
  pipeline.

Randomized rows use 4 000 inputs each (8 000–16 000 for the `hash_time_value`
and `get_modified_time` rows, whose whole point is value-dependent overflow),
all from the fixed SplitMix64 seed `0x5EED_1234_ABCD_0001`, so any failure is
reproducible.

## Output comparison

`modeselect` writes 8 `printf` lines as well as returning an `int`, so rows C35–
C47 compare **both**. Both `.so`s call the process's libc `printf`, so fd 1 is
redirected to a temp file around each call.

The first implementation of this did the redirect in-process and produced
spurious mismatches: under `cargo test`'s default parallel harness, libtest
writes its own progress lines (`test foo ... ok`) straight to fd 1 from other
threads, and those landed inside the capture. The capture now happens in a
**forked child** (`common::capture_forked_i32`), which gets its own copy of the
file-descriptor table, so the redirect is invisible to every other test thread
and the buffer contains only what the library printed. The child returns the
`int` through a separate temp file, so the value and the bytes always come from
the same invocation.

`tests/meta_harness.rs` guards against the suite passing vacuously:

- `harness_loads_two_distinct_libraries` — asserts all seven C and Rust function
  pointers have different addresses, so loading the same `.so` twice cannot pass.
- `harness_stdout_capture_is_not_vacuous` — asserts the capture is non-empty and
  contains all 8 `printf` call sites and exactly 9 lines, so a comparison of two
  empty buffers cannot pass.
- `harness_detects_a_deliberate_difference` — asserts the comparison helpers
  really do panic on differing bytes and differing ints.
- `harness_c_library_is_the_expected_ground_truth` — pins 8 literal C return
  values, so a broken `dlopen` returning zeros could not pass.

## Coverage evidence

`mutation_check.py` injects 43 deliberate behavioural bugs into
`translation/src/lib.rs` and requires the suite to reject each. **43/43 caught**,
including every `printf` format change, every arithmetic constant, the `switch`
fall-through structure, and the byte order and mask in `hash_time_value`. Three
provably equivalent mutants are excluded with reasons — see the mutation section
of `ERRORS.md`.

## Result

All 50 rows pass. Totals: **87 tests, 0 failures**, under the `debug` and
`release` profiles and under both feature invocations (there are no features).
Driver: `./verify.sh`.
