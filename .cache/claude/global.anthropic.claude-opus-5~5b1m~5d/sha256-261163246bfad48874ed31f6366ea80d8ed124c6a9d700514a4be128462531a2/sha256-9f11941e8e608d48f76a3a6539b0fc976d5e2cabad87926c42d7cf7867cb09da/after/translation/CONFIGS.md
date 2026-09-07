# CONFIGS.md — configuration-surface table (valid inputs)

## Axes, derived from the C source

### Build-time axes (the only "options" this library has)

`c_src/CMakeLists.txt` → `set(CMAKE_C_FLAGS "-DOP=${OP} -DREPEAT=${REPEAT}")`.

| axis | values the C actually branches on | where the C branches |
|------|-----------------------------------|----------------------|
| `OP` | `add`, `sub`, `mul` | `OP_FN(op)`→`CAT(op_,op)` (`mdmacros.h:45`), `STEP_add/STEP_sub/STEP_mul` (`:48-50`), `INIT_add 0`/`INIT_sub 0`/`INIT_mul 1` (`:56-58`), `DEFINE_ACCUM`→`accum_<OP>` (`:95`), `STR(OP)` for `G_OP_NAME` (`mdcore.c:37`) |
| `REPEAT` | `0,1,2,3,4,5,6,7` (`REP0..REP7` are the only ones defined; `REP8` ⇒ compile error) | `CHOOSE_REP(n)`→`CAT(REP,n)` (`:73-74`), `RUN_LOOP` (`:79`) |

⇒ **24 build configurations.** Rust mirror: features `add|sub|mul` × `"0".."7"`.

### Runtime axes

| axis | values | where the C branches |
|------|--------|----------------------|
| `n` for `use_generated` | `DISPATCH_REP`'s `switch`: `0,1,2,3,4,5,6` each a distinct case, plus `default` (everything else — see ERRORS.md E7–E9) | `mdmacros.h:82-93` |
| `a`,`b` for `op_*`/`helper_*`/`G_OP` | pure arithmetic, no branches — but value-dependent (overflow/sign). Shapes: `0`, `±1`, small, large, `INT_MAX`, `INT_MIN`, random | `mdcore.c:28-30,39-52` |
| driver `argc` | `<3` (reject, see ERRORS.md) vs `>=3` (extra argv ignored) | `mdmain.c:29` |
| driver argv strings | `atoi` shapes: plain, signed, whitespace-led, garbage-led, garbage-trailing, `int`-overflow, `long`-overflow | `mdmain.c:33-34` |

### Full set of public entry points (from `nm -D` / `mdmacros.h:40-42,104-110`)

Lowest level first — all are tested **directly** through the `.so`, not only via
the driver:

1. `op_add(int,int)` 2. `op_sub(int,int)` 3. `op_mul(int,int)` — the primitives
4. `G_OP` — data symbol, `int(*)(int,int)`, initialised at load time to `OP_FN(OP)`
5. `G_OP_NAME` — data symbol, `const char*` = `STR(OP)`
6. `helper_ptr(int,int)` — indirect call through a local function pointer
7. `helper_call(int,int)` — op + `INIT_FOR(OP)` + `RUN_LOOP(OP,acc,REPEAT)` (the composed pipeline; the only place `REPEAT` reaches the `.so` API)
8. `use_generated(int)` — the macro-generated `static accum_<OP>` behind `DISPATCH_REP`
9. `driver` executable — end-to-end composition of 1–8 + `atoi`

### Pruning rationale

`op_add`/`op_sub`/`op_mul` bodies do not depend on `OP` or `REPEAT`, and
`helper_ptr` / `use_generated` / `G_OP` / `G_OP_NAME` do not depend on `REPEAT`
(`REPEAT` only enters through `RUN_LOOP`, i.e. `helper_call` and `main`). Rows are
pruned accordingly; `helper_call` and the driver are expanded over the **full
24-way cross product**.

Every row is exercised with **many seeded-random inputs** (xorshift64*, fixed
seed `0x5EED_1234_ABCD_0001`) plus the boundary set
`{0, 1, -1, 2, -2, 7, -7, 65535, 65536, INT_MAX, INT_MAX-1, INT_MIN, INT_MIN+1}`
crossed with itself, comparing C `.so` vs Rust `.so` return values byte-for-byte.

---

## Table

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| C1 | `op_add` | all 24 `(OP,REPEAT)` builds; 2000 seeded-random `(a,b)` i32 pairs | [x] |
| C2 | `op_add` | all 24 builds; boundary set × boundary set (169 pairs) | [x] |
| C3 | `op_sub` | all 24 builds; 2000 seeded-random pairs | [x] |
| C4 | `op_sub` | all 24 builds; boundary × boundary | [x] |
| C5 | `op_mul` | all 24 builds; 2000 seeded-random pairs | [x] |
| C6 | `op_mul` | all 24 builds; boundary × boundary | [x] |
| C7 | `G_OP` | `OP=add`, all `REPEAT`; deref data symbol, call with random + boundary pairs; also assert `G_OP == op_add` address | [x] |
| C8 | `G_OP` | `OP=sub`, all `REPEAT`; same, `G_OP == op_sub` | [x] |
| C9 | `G_OP` | `OP=mul`, all `REPEAT`; same, `G_OP == op_mul` | [x] |
| C10 | `G_OP_NAME` | `OP=add` → C string `"add"`, byte-compared C vs Rust | [x] |
| C11 | `G_OP_NAME` | `OP=sub` → `"sub"` | [x] |
| C12 | `G_OP_NAME` | `OP=mul` → `"mul"` | [x] |
| C13 | `helper_ptr` | `OP=add`; random + boundary pairs | [x] |
| C14 | `helper_ptr` | `OP=sub`; random + boundary pairs | [x] |
| C15 | `helper_ptr` | `OP=mul`; random + boundary pairs | [x] |
| C16 | `use_generated` | `OP=add`; `n` sweeps every `switch` case `0..=6` | [x] |
| C17 | `use_generated` | `OP=sub`; `n ∈ 0..=6` | [x] |
| C18 | `use_generated` | `OP=mul`; `n ∈ 0..=6` (INIT=1, `*= i+1` → factorial shape) | [x] |
| C19 | `use_generated` | all 3 `OP`s; `n = REPEAT` (the value `main` passes), for every `REPEAT ∈ 0..7` incl. the `default`-hitting `7` | [x] |
| C20 | `use_generated` | all 3 `OP`s; 512 seeded-random `n` over full i32 range + the 13 boundary values + `0..9,100,-1,-100` | [x] |
| C21 | `helper_call` | `OP=add, REPEAT=0`; random + boundary pairs | [x] |
| C22 | `helper_call` | `OP=add, REPEAT=1` | [x] |
| C23 | `helper_call` | `OP=add, REPEAT=2` | [x] |
| C24 | `helper_call` | `OP=add, REPEAT=3` | [x] |
| C25 | `helper_call` | `OP=add, REPEAT=4` | [x] |
| C26 | `helper_call` | `OP=add, REPEAT=5` (default config) | [x] |
| C27 | `helper_call` | `OP=add, REPEAT=6` | [x] |
| C28 | `helper_call` | `OP=add, REPEAT=7` | [x] |
| C29 | `helper_call` | `OP=sub, REPEAT=0` | [x] |
| C30 | `helper_call` | `OP=sub, REPEAT=1` | [x] |
| C31 | `helper_call` | `OP=sub, REPEAT=2` | [x] |
| C32 | `helper_call` | `OP=sub, REPEAT=3` | [x] |
| C33 | `helper_call` | `OP=sub, REPEAT=4` | [x] |
| C34 | `helper_call` | `OP=sub, REPEAT=5` | [x] |
| C35 | `helper_call` | `OP=sub, REPEAT=6` | [x] |
| C36 | `helper_call` | `OP=sub, REPEAT=7` | [x] |
| C37 | `helper_call` | `OP=mul, REPEAT=0` (acc stays `INIT_mul=1`) | [x] |
| C38 | `helper_call` | `OP=mul, REPEAT=1` | [x] |
| C39 | `helper_call` | `OP=mul, REPEAT=2` | [x] |
| C40 | `helper_call` | `OP=mul, REPEAT=3` | [x] |
| C41 | `helper_call` | `OP=mul, REPEAT=4` | [x] |
| C42 | `helper_call` | `OP=mul, REPEAT=5` | [x] |
| C43 | `helper_call` | `OP=mul, REPEAT=6` | [x] |
| C44 | `helper_call` | `OP=mul, REPEAT=7` (acc = 7! = 5040) | [x] |
| C45 | full pipeline through the `.so` | all 24 builds: for each random `(a,b)` call `op_*`, `helper_ptr`, `helper_call`, `use_generated(REPEAT)`, `G_OP` **in the same order `main` does** and compare the composed `summary` value across the FFI boundary | [x] |
| C46 | `driver` binary | all 24 builds; `argc==3`, plain decimal `a b` (random + boundary) — stdout byte-compared | [x] |
| C47 | `driver` binary | all 24 builds; `argc>3` (extra ignored args) — stdout byte-compared | [x] |
| C48 | `driver` binary | all 24 builds; `atoi` shapes: `" 12"`, `"+12"`, `"-0"`, `"12abc"`, `"abc"`, `""`, `"2147483648"`, `"-2147483649"`, `"99999999999999999999"` — stdout + stderr + exit code byte-compared | [x] |
| C49 | `driver` binary | all 24 builds; stdout **and** exit status compared for the `argc<3` usage path (see ERRORS.md E1/E2) | [x] |
| C50 | `helper_call` **printed output** | all 24 builds; the library's `printf` bytes (not just the return value) compared via the `examples/socall` subprocess over the boundary cross product + 24 random pairs | [x] |
| C51 | `helper_ptr` **printed output** | all 24 builds; same corpus | [x] |
| C52 | `use_generated` **printed output** | all 24 builds; `n ∈ {-1..8, 100, REPEAT}` + boundary set + 24 random | [x] |
| C53 | `op_add`/`op_sub`/`op_mul`/`G_OP`/`G_OP_NAME` from a **separate process** | all 24 builds; proves the exports are consumable by a genuinely external `dlopen`ing process, not only in-process | [x] |
| C54 | every entry point | the Cargo feature combinations that have **no CMake counterpart** — no `OP` feature, no `REPEAT` feature, and several `OP`s / `REPEAT`s at once — resolved by `src/mdconfig.rs`'s documented priority (`add>sub>mul`, default `add`; lowest `REPEAT` wins, default `5`) and diffed against the C build for the resolved config. 15 combos, driven by `run_odd_combos.sh` | [x] |

---

## How the rows are executed

```
./build_c.sh <OP> <REPEAT>     # C .so (gcc -shared on mdcore.c) + CMake driver
./run_diff.sh                 # the 24 CMake-representable configs, end to end
./run_odd_combos.sh           # the 15 Cargo-only / degenerate feature combos
```

Each invocation does, per configuration: build the C `.so` + C `driver`, build
the Rust `cdylib` + Rust `driver`, diff `nm -D`, then run all 40 tests in
`tests/differential.rs`. Result: **39 configurations x 40 tests, 0 failures,
symbol diff empty everywhere** (logs in `../logs/`).

## Harness sensitivity (fault injection)

To prove the rows above are actually discriminating and not vacuously passing,
six single-line mutations were injected into the Rust and every one was caught:

| mutation | tests that failed |
|----------|-------------------|
| `op_add` returns `a+b+1` | 22 |
| `dispatch_rep` accepts `n == 7` (C's `switch` has no `case 7`) | 4 |
| `INIT_mul` 1 -> 2 | 22 |
| `run_loop` iterates `REPEAT-1` times | 14 |
| `atoi` overflow saturates to `0` instead of `i64::MAX` | 2 |
| `G_OP_NAME` `"sub"` -> `"SUB"` | 11 |

All mutations were reverted and the sources verified byte-identical afterwards.
