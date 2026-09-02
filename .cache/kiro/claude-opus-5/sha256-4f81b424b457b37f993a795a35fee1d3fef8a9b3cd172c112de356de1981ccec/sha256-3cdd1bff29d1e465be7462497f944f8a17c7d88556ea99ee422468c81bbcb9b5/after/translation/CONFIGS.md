# CONFIGS.md — the configuration-surface table (Phase A / Phase B)

Mirror image of `ERRORS.md`, for **valid** inputs. Derived mechanically from
what `c_src` branches on, not from what looks important.

## Axis 1 — build-time options (the only options the API exposes)

`c_src/CMakeLists.txt` declares exactly two cache variables and turns them into
`#define`s:

```cmake
set(OP     "add" CACHE STRING "operation leveraged")
set(REPEAT "5"   CACHE STRING "iterations tested")
set(CMAKE_C_FLAGS "-DOP=${OP} -DREPEAT=${REPEAT}")
```

| axis | values the code accepts | what it toggles (grep of the branches) |
|------|-------------------------|----------------------------------------|
| `OP` | `add`, `sub`, `mul`; **undefined ⇒ `add`** (`mdmacros.h:27`) | `OP_FN(OP)` → `op_add`/`op_sub`/`op_mul` (h:45); `INIT_FOR(OP)` → `0`/`0`/`1` (h:56-59); `STEP_OP(OP,…)` → `+=i` / `-=i` / `*=(i+1)` (h:48-50); `STR(OP)` → the `G_OP_NAME` literal (h:34-35); `ACCUM_FN(OP)` → `accum_add`/`accum_sub`/`accum_mul` (h:101) |
| `REPEAT` | `0`..`7` only — `CHOOSE_REP(n)` pastes `REP##n` and only `REP0..REP7` exist (h:63-70); **undefined ⇒ `5`** (h:30) | `RUN_LOOP(OP, acc, REPEAT)` → an unrolled chain of `REPEAT` steps with the literal indices `0..REPEAT-1`. Referenced from exactly two places: `mdcore.c:42` (`helper_call`) and `mdmain.c:38` (`main`). Also `mdmain.c:42` passes `REPEAT` as the `n` of `use_generated` |

Cross-product: **3 × 8 = 24** build configurations. In the Rust crate these are
Cargo features; `combos.sh` enumerates all spellings (bare CMake value, `op_*` /
`repeat_*` alias, and omission for the `#ifndef` fallback) = **63** feature sets,
all of which `cargo check` cleanly and 49 representative ones of which
`run_all.sh` runs the full suite against.

## Axis 2 — public entry points (complete, lowest level first)

Taken from the `extern` declarations in `mdmacros.h:40-42, 104-110` and from
`nm -D` on the C `.so` — **not** just the convenience wrappers.

| level | entry point | composition |
|-------|-------------|-------------|
| leaf  | `op_add(int,int)` | `a + b`; contains no macro → identical in all 24 configs |
| leaf  | `op_sub(int,int)` | `a - b`; ditto |
| leaf  | `op_mul(int,int)` | `a * b`; ditto |
| data  | `G_OP` | mutable `int (*)(int,int)` object initialised to `OP_FN(OP)`; depends on **OP** |
| data  | `G_OP_NAME` | mutable `const char *` object initialised to `STR(OP)`; depends on **OP** |
| mid   | `helper_ptr(int,int)` | `OP_FN(OP)` through a local fn pointer + `printf`; depends on **OP** |
| mid   | `use_generated(int n)` | `accum_<OP>(n)` = `INIT_FOR(OP)` + `DISPATCH_REP` `switch (n)` + `printf`; depends on **OP** and on **n**, *not* on REPEAT |
| high  | `helper_call(int,int)` | `OP_FN(OP)(a,b)` **and** `RUN_LOOP(OP, acc, REPEAT)` + `printf`, returns `r + acc`; depends on **OP × REPEAT** |
| top   | `main(argc, argv)` | `atoi` ×2, `OP_FN(OP)`, `RUN_LOOP(…, REPEAT)`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, 2 `printf`s; depends on **OP × REPEAT** and on the argv shapes |

## Axis 3 — input shapes the code distinguishes

| shape class | values | why the C distinguishes it |
|-------------|--------|----------------------------|
| `int` pair for `op_*` / `helper_*` | `0`, `±1`, `±2`, `±7`, `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, `65535`, `65536`, `-65536`, `46340`, `46341`, `-46341` (17 boundary values, full 17×17 cross-product) plus randomized values | the bodies are branch-free but *value dependent*: `46340²` is the last non-overflowing square, `46341²` the first overflowing one; `INT_MIN * -1` and `INT_MIN - 1` are the wrap pivots |
| `n` for `use_generated` | `0,1,2,3,4,5,6` — seven distinct `case` arms (h:84-90) | each arm unrolls a different number of `STEP_OP`s |
| `n` for `use_generated` | anything else → `default:` (h:91) | see `ERRORS.md` E-02..E-07 |
| `n` for `use_generated` | `REPEAT` itself, which is what `mdmain.c:42` passes | for `REPEAT == 7` this is the *only* configuration where the driver's own call lands on `default:` |
| `argv` for `main` | plain decimal, signed, leading whitespace, numeric prefix + garbage, non-numeric, empty, `> LONG` magnitude, extra ignored args, `argc < 3` | `atoi` and the `argc` check |
| `G_OP` slot state | pristine, and overwritten by the consumer | the object is mutable; the library's own helpers must keep using `OP_FN(OP)` and ignore it |

## Row table (cross-product, pruned to what the C actually distinguishes)

The pruning is source-derived: `REPEAT` is referenced **only** by `RUN_LOOP`
(`mdcore.c:42`, `mdmain.c:38`) and by `mdmain.c:42`, so only `helper_call` and
`main` vary with it — the other rows would be exact duplicates across the eight
REPEAT values. They are nevertheless re-run in all 24 configurations by
`run_all.sh`, so the un-pruned cross-product is covered too.

Every row uses **many randomized inputs with a fixed seed** (SplitMix64, biased
towards boundary magnitudes) in addition to the exhaustive boundary
cross-product, and asserts the return value **and** the bytes written to stdout
match between the C `.so` and the Rust `.so`.

### Group L — leaf arithmetic (configuration-independent)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| L-01 | `op_add` | any OP/REPEAT; 289 boundary pairs + 4096 randomized pairs | [x] |
| L-02 | `op_sub` | any OP/REPEAT; 289 boundary pairs + 4096 randomized pairs | [x] |
| L-03 | `op_mul` | any OP/REPEAT; 289 boundary pairs + 4096 randomized pairs | [x] |
| L-04 | `op_add`, `op_sub`, `op_mul` | any OP/REPEAT; assert all three write **zero** bytes to stdout | [x] |

### Group G — exported data objects (vary with OP)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| G-01 | `G_OP` (call through the slot) | OP=add; boundary + 2048 randomized pairs; must equal `op_add` | [x] |
| G-02 | `G_OP` | OP=sub; ditto vs `op_sub` | [x] |
| G-03 | `G_OP` | OP=mul; ditto vs `op_mul` | [x] |
| G-04 | `G_OP_NAME` | OP=add; bytes **including the NUL terminator** must be `"add\0"` | [x] |
| G-05 | `G_OP_NAME` | OP=sub; `"sub\0"` | [x] |
| G-06 | `G_OP_NAME` | OP=mul; `"mul\0"` | [x] |
| G-07 | `G_OP` slot store, then `helper_call` / `helper_ptr` | OP=add; overwrite the slot with `op_mul`, verify the store is observable **and** that the helpers (which expand `OP_FN(OP)`, not `G_OP`) are unaffected; restore | [x] |
| G-08 | same | OP=sub | [x] |
| G-09 | same | OP=mul (slot overwritten with `op_add`) | [x] |

### Group P — `helper_ptr` (varies with OP)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| P-01 | `helper_ptr` | OP=add; boundary + 512 randomized pairs; return value **and** `helper.ptr=%d` line | [x] |
| P-02 | `helper_ptr` | OP=sub; ditto | [x] |
| P-03 | `helper_ptr` | OP=mul; ditto | [x] |

### Group H — `helper_call` (varies with OP × REPEAT: the full 24)

Each row: boundary cross-product + 512 randomized pairs; compares the returned
`r + acc` **and** the exact `helper.call=%d helper.acc=%d` line. A second check
per row asserts the `helper.acc=` field is invariant in `(a,b)` — it is a pure
function of `OP` and `REPEAT` — and that both libraries produce the same value.

| # | entry point(s) | configuration | [ ] | | # | entry point(s) | configuration | [ ] |
|---|---|---|---|---|---|---|---|---|
| H-01 | `helper_call` | OP=add REPEAT=0 (`acc`=0) | [x] | | H-13 | `helper_call` | OP=sub REPEAT=4 (`acc`=-6) | [x] |
| H-02 | `helper_call` | OP=add REPEAT=1 (`acc`=0) | [x] | | H-14 | `helper_call` | OP=sub REPEAT=5 (`acc`=-10) | [x] |
| H-03 | `helper_call` | OP=add REPEAT=2 (`acc`=1) | [x] | | H-15 | `helper_call` | OP=sub REPEAT=6 (`acc`=-15) | [x] |
| H-04 | `helper_call` | OP=add REPEAT=3 (`acc`=3) | [x] | | H-16 | `helper_call` | OP=sub REPEAT=7 (`acc`=-21) | [x] |
| H-05 | `helper_call` | OP=add REPEAT=4 (`acc`=6) | [x] | | H-17 | `helper_call` | OP=mul REPEAT=0 (`acc`=1) | [x] |
| H-06 | `helper_call` | OP=add REPEAT=5 (`acc`=10) | [x] | | H-18 | `helper_call` | OP=mul REPEAT=1 (`acc`=1) | [x] |
| H-07 | `helper_call` | OP=add REPEAT=6 (`acc`=15) | [x] | | H-19 | `helper_call` | OP=mul REPEAT=2 (`acc`=2) | [x] |
| H-08 | `helper_call` | OP=add REPEAT=7 (`acc`=21) | [x] | | H-20 | `helper_call` | OP=mul REPEAT=3 (`acc`=6) | [x] |
| H-09 | `helper_call` | OP=sub REPEAT=0 (`acc`=0) | [x] | | H-21 | `helper_call` | OP=mul REPEAT=4 (`acc`=24) | [x] |
| H-10 | `helper_call` | OP=sub REPEAT=1 (`acc`=0) | [x] | | H-22 | `helper_call` | OP=mul REPEAT=5 (`acc`=120) | [x] |
| H-11 | `helper_call` | OP=sub REPEAT=2 (`acc`=-1) | [x] | | H-23 | `helper_call` | OP=mul REPEAT=6 (`acc`=720) | [x] |
| H-12 | `helper_call` | OP=sub REPEAT=3 (`acc`=-3) | [x] | | H-24 | `helper_call` | OP=mul REPEAT=7 (`acc`=5040) | [x] |

### Group U — `use_generated` / `DISPATCH_REP` (varies with OP × `n`)

One row per `switch` arm per OP — 21 in-range rows plus 3 `default:` rows.
(The `default:` rows are also `ERRORS.md` E-02..E-07.)

| # | entry point(s) | configuration | [ ] | | # | entry point(s) | configuration | [ ] |
|---|---|---|---|---|---|---|---|---|
| U-01 | `use_generated` | OP=add n=0 (⇒0) | [x] | | U-13 | `use_generated` | OP=sub n=5 (⇒-10) | [x] |
| U-02 | `use_generated` | OP=add n=1 (⇒0) | [x] | | U-14 | `use_generated` | OP=sub n=6 (⇒-15) | [x] |
| U-03 | `use_generated` | OP=add n=2 (⇒1) | [x] | | U-15 | `use_generated` | OP=mul n=0 (⇒1) | [x] |
| U-04 | `use_generated` | OP=add n=3 (⇒3) | [x] | | U-16 | `use_generated` | OP=mul n=1 (⇒1) | [x] |
| U-05 | `use_generated` | OP=add n=4 (⇒6) | [x] | | U-17 | `use_generated` | OP=mul n=2 (⇒2) | [x] |
| U-06 | `use_generated` | OP=add n=5 (⇒10) | [x] | | U-18 | `use_generated` | OP=mul n=3 (⇒6) | [x] |
| U-07 | `use_generated` | OP=add n=6 (⇒15) | [x] | | U-19 | `use_generated` | OP=mul n=4 (⇒24) | [x] |
| U-08 | `use_generated` | OP=sub n=0 (⇒0) | [x] | | U-20 | `use_generated` | OP=mul n=5 (⇒120) | [x] |
| U-09 | `use_generated` | OP=sub n=1 (⇒0) | [x] | | U-21 | `use_generated` | OP=mul n=6 (⇒720) | [x] |
| U-10 | `use_generated` | OP=sub n=2 (⇒-1) | [x] | | U-22 | `use_generated` | OP=add, `n ∉ 0..6` (`default:`; 7/8/9/100/−1/−2/−7/INT_MIN/INT_MAX + `-16..16` + 256 randomized) | [x] |
| U-11 | `use_generated` | OP=sub n=3 (⇒-3) | [x] | | U-23 | `use_generated` | OP=sub, `n ∉ 0..6` | [x] |
| U-12 | `use_generated` | OP=sub n=4 (⇒-6) | [x] | | U-24 | `use_generated` | OP=mul, `n ∉ 0..6` | [x] |

### Group C — composed library pipeline (interleaved, through the `.so` only)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| C-01 | `op_*`, `helper_call`, `helper_ptr`, `use_generated`, `G_OP`, `G_OP_NAME` interleaved in one sequence, 256 randomized `(a,b)` | all 24 configs (one per `run_all.sh` iteration) — catches ordering / hidden-state divergence a per-function test cannot | [x] |
| C-02 | the same composition `mdmain.c` performs (`op_<OP>` + `helper_call` + `helper_ptr` + `use_generated(REPEAT)` + `G_OP`, summed with wrap-around), 289+256 pairs | all 24 configs | [x] |

### Group M — whole program (`main`), OP × REPEAT = 24

Each row compares stdout, stderr and exit status byte-for-byte between
`cbuild/exe/driver_<op>_<r>` and `target/release/driver`, over 18 fixed argv
shapes (zero/one/negative/`INT_MAX`/`INT_MIN`/`46341`/`65536`/whitespace+prefix/
`> LONG`/non-numeric/empty/extra-args) plus 300 randomized `(a,b)` pairs, plus
the `argc < 3` usage path, plus a check that the five output lines appear in the
`helper.call` → `helper.ptr` → `gen.acc` → `op=…` → `summary=` order (a
stdout-buffering divergence would surface here, since C's `printf` stream is
fully buffered when redirected while Rust's `println!` is line buffered).

| # | configuration | [ ] | | # | configuration | [ ] | | # | configuration | [ ] |
|---|---|---|---|---|---|---|---|---|---|---|
| M-01 | add/0 | [x] | | M-09 | sub/0 | [x] | | M-17 | mul/0 | [x] |
| M-02 | add/1 | [x] | | M-10 | sub/1 | [x] | | M-18 | mul/1 | [x] |
| M-03 | add/2 | [x] | | M-11 | sub/2 | [x] | | M-19 | mul/2 | [x] |
| M-04 | add/3 | [x] | | M-12 | sub/3 | [x] | | M-20 | mul/3 | [x] |
| M-05 | add/4 | [x] | | M-13 | sub/4 | [x] | | M-21 | mul/4 | [x] |
| M-06 | add/5 | [x] | | M-14 | sub/5 | [x] | | M-22 | mul/5 | [x] |
| M-07 | add/6 | [x] | | M-15 | sub/6 | [x] | | M-23 | mul/6 | [x] |
| M-08 | add/7 | [x] | | M-16 | sub/7 | [x] | | M-24 | mul/7 | [x] |

### Group F — feature-spelling / fallback configurations

| # | configuration | [ ] |
|---|---------------|-----|
| F-01 | `op_add`/`op_sub`/`op_mul` alias spellings × bare `"0"`,`"3"`,`"5"`,`"7"` REPEAT spellings (12 combos) | [x] |
| F-02 | no OP feature at all × `repeat_0..7` — `#ifndef OP ⇒ add` (8 combos, compared against the C `add` libraries) | [x] |
| F-03 | `add`/`sub`/`mul` with no REPEAT feature — `#ifndef REPEAT ⇒ 5` (3 combos) | [x] |
| F-04 | no features at all — both fallbacks (`add`, `5`) | [x] |
| F-05 | every feature enabled simultaneously — documented precedence resolves to `sub`, `repeat_0`; the harness cross-checks the resolved OP against the loaded C library | [x] |

## Status

87 rows in groups L/G/P/H/U/C/M plus 5 spelling rows, all checked.

```
$ ./run_all.sh
=== canonical OP x REPEAT (24) ===
PASS add/0 .. PASS mul/7                     (31 tests each)
=== alias spellings (op_<x> and bare "<n>") ===
PASS op_add/"0" .. PASS op_mul/"7"           (31 tests each)
=== #ifndef fallbacks (missing OP => add, missing REPEAT => 5) ===
PASS <no OP>/0 .. PASS <no OP>/<no REPEAT>   (31 tests each)
=== everything enabled at once (documented precedence: sub, repeat_0) ===
PASS all-features                            (31 tests)

ALL FEATURE COMBINATIONS PASS
```

49 feature sets × 31 tests, no divergence remaining.
