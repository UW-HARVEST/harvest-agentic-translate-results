# CONFIGS.md — the configuration-surface table (Phase A / Phase B)

## The axes the C code actually branches on

Derived from the `#ifdef`/`switch`/token-paste sites in `c_src/src/mdmacros.h`
and the call sites in `mdcore.c` / `mdmain.c` — not from guesses about what
"matters".

### Axis 1 — `OP` (build-time, 3 values)

`OP` is pasted into identifiers, so it drives **four** separate selections at
once, and they are *not* redundant with each other:

| `OP` | `OP_FN(OP)` → | `STEP_OP` → | `INIT_FOR(OP)` → | `STR(OP)` → | `ACCUM_FN(OP)` → |
|------|---------------|-------------|------------------|-------------|------------------|
| `add` | `op_add` (`a+b`) | `acc += i` | `0` | `"add"` | `accum_add` |
| `sub` | `op_sub` (`a-b`) | `acc -= i` | `0` | `"sub"` | `accum_sub` |
| `mul` | `op_mul` (`a*b`) | `acc *= (i+1)` | `1` | `"mul"` | `accum_mul` |

`mul` is the odd one out on *two* axes (non-zero init, and the step uses `i+1`
rather than `i`), which is why it must be tested separately rather than assumed
to behave like `add`/`sub`. Any `OP` outside this set is a compile error
(`ERRORS.md` row 24).

### Axis 2 — `REPEAT` (build-time, 8 values: `0`..`7`)

`RUN_LOOP(op, acc, REPEAT)` = `CHOOSE_REP(REPEAT)(op, acc)` = `REP<REPEAT>`,
a **statically unrolled** chain applying `STEP_OP` at indices `0 .. REPEAT-1`.
`REPEAT=0` expands to *nothing* (empty body — a distinct code shape, the
accumulator keeps its initial value). Any `REPEAT` outside `0..=7` is a compile
error (`ERRORS.md` row 25). Note the asymmetry that makes `7` special: `REP7`
exists for `RUN_LOOP`, but `DISPATCH_REP`'s `switch` stops at `case 6`.

### Axis 3 — runtime argument `n` to `use_generated` (10 distinct shapes)

`n` is the `switch` selector in `DISPATCH_REP`. The code distinguishes
`0,1,2,3,4,5,6` (seven separate `case` arms, each a different unroll depth) from
everything else (`default:`, no steps at all). Boundary shapes: `-1`, `7`, `8`,
`INT_MIN`, `INT_MAX`.

Crucially, `n` is **independent of the build-time `REPEAT`**: `use_generated(3)`
does 3 steps no matter what `REPEAT` is. `mdmain.c` only ever calls
`use_generated(REPEAT)`, so driving `use_generated` *directly* with all 10 shapes
is the only way to reach the other arms — this is exactly the "test the
lowest-level entry points, not just the convenience wrapper" requirement.

### Axis 4 — runtime arguments `a`, `b` (value shapes)

`op_add`/`op_sub`/`op_mul`/`helper_call`/`helper_ptr` are value-transparent
except for two's-complement wrapping, so the shapes that matter are:
`0`, `1`, `-1`, small ±, `INT_MAX`, `INT_MIN`, and randomised full-range 32-bit
values (which is where wrapping actually gets hit).

### Axis 5 — entry point (the full public surface, 8 symbols)

Lowest level first, which is also the call hierarchy:

1. `op_add`, `op_sub`, `op_mul` — leaves; all three are exported and callable
   **regardless of which one `OP` selected**, so each must be tested in every
   build (a build with `OP=mul` still exports a working `op_add`).
2. `G_OP` (data) — a function pointer the consumer loads and *calls* through.
   Reading it, and calling through it, is a distinct entry path from calling
   `op_*` by name.
3. `G_OP_NAME` (data) — a `const char *` the consumer reads as a C string.
4. `helper_ptr` — calls the selected op through a local pointer.
5. `use_generated` — the only door to the `static accum_<OP>` `switch`.
6. `helper_call` — the composed one: op + full `REPEAT` unroll + sum.
7. `main`/`driver` — the end-to-end pipeline (`op` + unroll + all three helpers
   + `G_OP` call + two `printf`s), plus `atoi` parsing.

### Axis 6 — Cargo feature *representation* (Rust-only)

The C has exactly 24 buildable configurations. Cargo features are additive and
cannot be made mutually exclusive, so `translation/Cargo.toml` exposes 11
features (`add`/`sub`/`mul`, `0`..`7`) whose 2^11 = 2048 subsets must all
*compile*, with documented priority resolution (`mul > sub > add`; highest
`REPEAT` number wins; empty ⇒ the CMake defaults `add`/`5`). Rows 25–28 pin this
down.

## The table

`OP × REPEAT` is a genuine cross-product (both axes independently change the
emitted code), so rows 1–24 enumerate all 24 buildable C configurations. Each
row is tested via **both** `.so`s with the *full* entry-point set (axis 5) and
randomised inputs (axes 3 & 4, fixed seed `0x5EED_C0FFEE`, 2000 random
`(a,b)` pairs and the full `n ∈ {-1..8} ∪ {INT_MIN, INT_MAX} ∪ 2000 random`
sweep per row).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | all 8 symbols + `driver` | `OP=add`, `REPEAT=0` — empty unroll, `INIT=0`, step `+=i`; `n` sweep + random `(a,b)` | [x] |
| 2 | all 8 symbols + `driver` | `OP=add`, `REPEAT=1` | [x] |
| 3 | all 8 symbols + `driver` | `OP=add`, `REPEAT=2` | [x] |
| 4 | all 8 symbols + `driver` | `OP=add`, `REPEAT=3` | [x] |
| 5 | all 8 symbols + `driver` | `OP=add`, `REPEAT=4` | [x] |
| 6 | all 8 symbols + `driver` | `OP=add`, `REPEAT=5` — **the CMake default** | [x] |
| 7 | all 8 symbols + `driver` | `OP=add`, `REPEAT=6` | [x] |
| 8 | all 8 symbols + `driver` | `OP=add`, `REPEAT=7` — `RUN_LOOP` reaches `REP7` but `use_generated(7)` hits `default:` | [x] |
| 9 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=0` — `INIT=0`, step `-=i` ⇒ accumulator goes negative | [x] |
| 10 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=1` | [x] |
| 11 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=2` | [x] |
| 12 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=3` | [x] |
| 13 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=4` | [x] |
| 14 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=5` | [x] |
| 15 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=6` | [x] |
| 16 | all 8 symbols + `driver` | `OP=sub`, `REPEAT=7` | [x] |
| 17 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=0` — `INIT=1`, step `*=(i+1)`; empty unroll ⇒ `acc` stays `1` | [x] |
| 18 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=1` | [x] |
| 19 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=2` | [x] |
| 20 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=3` | [x] |
| 21 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=4` | [x] |
| 22 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=5` | [x] |
| 23 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=6` | [x] |
| 24 | all 8 symbols + `driver` | `OP=mul`, `REPEAT=7` — factorial accumulator `7! = 5040` | [x] |
| 25 | all 8 symbols | Cargo: **no features at all** ⇒ must behave as `OP=add, REPEAT=5` (the `#ifndef` defaults) | [x] |
| 26 | all 8 symbols | Cargo: conflicting OP features (`add,sub`, `add,mul`, `sub,mul`, `add,sub,mul`) ⇒ resolves `mul > sub > add`; must compile and stay self-consistent (`G_OP_NAME`, `INIT`, `step`, `op_fn` all agree) | [x] |
| 27 | all 8 symbols | Cargo: conflicting REPEAT features (e.g. `2,5`, `0,7`, all of `0..7`) ⇒ highest wins; must compile and match the corresponding single-value C build | [x] |
| 28 | all 8 symbols | Cargo: OP feature with no REPEAT feature and vice-versa ⇒ the *missing* axis takes its `#ifndef` default (`add` / `5`) | [x] |

## Detail: what "all 8 symbols" means per row

For each row the differential harness does, against both `.so`s:

| entry point | inputs driven |
|---|---|
| `op_add` / `op_sub` / `op_mul` | 2000 random `(a,b)` + `{0,1,-1,2,-2,INT_MAX,INT_MIN}²` corners |
| `helper_ptr` | same input set |
| `helper_call` | same input set (also fixes the `REPEAT` unroll + the `r+acc` sum) |
| `use_generated` | `n ∈ {INT_MIN,-2,-1,0,1,2,3,4,5,6,7,8,9,INT_MAX}` + 2000 random `i32` |
| `G_OP` | dereferenced and **called** with the same `(a,b)` set; also compared against the address of the expected `op_*` export |
| `G_OP_NAME` | read as a NUL-terminated C string and compared byte-for-byte |
| `driver` (rows 1–24) | stdout+stderr+exit status compared for a set of `A B` argument pairs, incl. `atoi` edge cases |

Additionally, the **stdout side effects** of the three printing exports are
compared byte-for-byte at the `.so` level (`tests/stdout_parity.rs`). Comparing
only the `int` return values would miss a divergence in any of the three format
strings:

```c
printf("helper.call=%d helper.acc=%d\n", r, acc);   /* helper_call   */
printf("helper.ptr=%d\n", r);                        /* helper_ptr    */
printf("gen.acc=%d\n", r);                           /* use_generated */
```

## How the rows are run

| test file | what it covers |
|---|---|
| `tests/valid_paths.rs` | Phase B: all 8 exported symbols, lowest-level first, over axes 3 & 4 |
| `tests/stdout_parity.rs` | Phase B: the `printf` side effects of the `.so` exports |
| `tests/driver_cli.rs` | Phase B: the end-to-end `driver` pipeline; Phase C rows 18–21 |
| `tests/errors.rs` | Phase C rows 1–14 |
| `tests/globals.rs` | Phase C rows 15–17 (needs its own process — it clobbers `.data`) |
| `tests/symbols.rs` | Phase D: `nm -D` parity against all 24 C configurations |

`run_all_configs.sh` builds **and then** tests each configuration; the build step
is mandatory because `cargo test` does not reliably re-emit the `cdylib` when only
the feature set changes. `tests/common/mod.rs` additionally verifies at load time
that the `.so` on disk really was built for the active feature set (it checks
`G_OP_NAME` and `helper_call(0,0)`), turning a stale artifact into an explicit
build error instead of a misleading "divergence".

## Result

All 41 configurations pass all 34 tests, in **both** the dev profile (Rust
arithmetic overflow checks ON, which makes the wrapping-parity assertions
strictly harder) and the release profile:

```
./build_c_so.sh                                     # 24 C .so + 24 C driver
CARGO_TARGET_DIR=.../target-cfg ./run_all_configs.sh
CARGO_EXTRA=--release CARGO_TARGET_DIR=.../target-cfg ./run_all_configs.sh
./check_all_features.sh full                        # 2048 feature subsets
```
