# CONFIGS.md — configuration-surface table (Phase B)

## Axes, derived from the C source

**Build-time axes** (`CMakeLists.txt` → `CMAKE_C_FLAGS "-DOP=${OP} -DREPEAT=${REPEAT}"`,
consumed by `mdmacros.h`; mirrored by Cargo features of the same lowercase names):

| axis | values | what it switches in the C |
|------|--------|----------------------------|
| `OP` | `add`, `sub`, `mul` (+ *unset* → `#ifndef OP` default `add`) | `OP_FN(OP)` → which `op_*` is called by `helper_call`/`helper_ptr`/`G_OP`; `STEP_<OP>` → `+=i` / `-=i` / `*=(i+1)`; `INIT_<OP>` → `0`/`0`/`1`; `STR(OP)` → `G_OP_NAME`; `accum_<OP>` name+body |
| `REPEAT` | `0`..`7` (+ *unset* → `#ifndef REPEAT` default `5`) | `CHOOSE_REP(REPEAT)` → which `REP0..REP7` unrolling is spliced into `helper_call` (and `main`); also the literal `n` `main` passes to `use_generated`. Verified: `REPEAT=8` **fails to compile** (`REP8` undefined), so `0..7` is the exact valid range |

Cross product = **3 × 8 = 24** buildable configurations, plus the two *unset*
fallbacks (no `OP` feature, no `REPEAT` feature) which must reproduce `add`/`5`.
The Cargo feature matrix is `{∅,add,sub,mul} × {∅,0..7}` = **36** combinations.

**Runtime axes.** `mdmacros.h` exposes no runtime option/mode/flag setter; the only
runtime state is the two writable globals and the integer arguments. The runtime
axes the C actually branches on are therefore:

| axis | values | where the C branches |
|------|--------|----------------------|
| `use_generated(n)` | `0,1,2,3,4,5,6` (seven distinct `case` labels) and *everything else* (`default:`) | `DISPATCH_REP` `switch (n)`, `mdmacros.h:83-92` |
| `G_OP` contents | initial `op_<OP>`, or any of `op_add`/`op_sub`/`op_mul` stored by the caller | indirect call through the writable global |
| `int` argument shape | `0`, `1`, `-1`, `INT_MAX`, `INT_MIN`, random, overflow-inducing pairs | no explicit branch, but value-dependent (wrapping) results |
| `argc` / `argv` shape (executable) | `argc<3` vs `>=3`; numeric / non-numeric / overflowing argument text | `mdmain.c:29` guard + `atoi` |

**Entry points.** The full public surface, lowest-level first — *not* only the
convenience wrappers. `op_add`/`op_sub`/`op_mul` are the lowest level (leaf
arithmetic, always compiled in **all three** variants regardless of `OP`);
`accum_<OP>` is `static` and reached only through `use_generated`; `helper_call`,
`helper_ptr`, `use_generated` are the composed layer; `G_OP`/`G_OP_NAME` are data;
`main` is the top-level driver.

Every row below is exercised with **many randomized inputs** (`SEED = 0x5EED_1234`,
a fixed-seed xorshift64* in `tests/common/mod.rs`, so runs are reproducible) plus
the fixed boundary set `{0, 1, -1, 2, -2, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1}`,
and asserts the C and Rust `.so`s agree on **both** the `int` return value **and**
the exact `printf` bytes written to stdout.

## Table

Legend: “all 24” = the row is executed under every one of the 24 `OP` × `REPEAT`
build configurations (`check_all.sh` drives the loop; each `cargo test` run
compiles the matching C `.so` with the same `-DOP/-DREPEAT`).

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1 | `op_add` | leaf call, 256 random `(a,b)` + 81-pair boundary cross-product; OP/REPEAT-independent, so run under all 24 | [x] |
| 2 | `op_sub` | leaf call, same input set, all 24 | [x] |
| 3 | `op_mul` | leaf call, same input set, all 24 | [x] |
| 4 | `op_add`/`op_sub`/`op_mul` | the **non-selected** operations must still be exported and correct (e.g. `op_mul` in an `OP=add` build) — asserts the C compiles all three unconditionally | [x] |
| 5 | `helper_ptr` | `OP=add`, indirect call through a local `int(*fp)(int,int)`; random + boundary `(a,b)`; stdout `helper.ptr=%d`; REPEAT-independent → run under all 8 REPEATs | [x] |
| 6 | `helper_ptr` | `OP=sub`, ditto | [x] |
| 7 | `helper_ptr` | `OP=mul`, ditto | [x] |
| 8 | `helper_call` | `OP=add` × `REPEAT=0` — unrolled loop empty, `acc` stays `0`; `return r+acc` | [x] |
| 9 | `helper_call` | `OP=add` × `REPEAT=1` (`acc=0`) | [x] |
| 10 | `helper_call` | `OP=add` × `REPEAT=2` (`acc=1`) | [x] |
| 11 | `helper_call` | `OP=add` × `REPEAT=3` (`acc=3`) | [x] |
| 12 | `helper_call` | `OP=add` × `REPEAT=4` (`acc=6`) | [x] |
| 13 | `helper_call` | `OP=add` × `REPEAT=5` (`acc=10`, default config) | [x] |
| 14 | `helper_call` | `OP=add` × `REPEAT=6` (`acc=15`) | [x] |
| 15 | `helper_call` | `OP=add` × `REPEAT=7` (`acc=21`) — `REP7`, the max valid unrolling | [x] |
| 16 | `helper_call` | `OP=sub` × `REPEAT=0..7` (`acc` = `0,0,-1,-3,-6,-10,-15,-21`), 8 sub-rows | [x] |
| 17 | `helper_call` | `OP=mul` × `REPEAT=0..7` (`acc` = `1,1,2,6,24,120,720,5040`), 8 sub-rows | [x] |
| 18 | `use_generated` | `OP=add`, `n` = each of `0..6` (every `switch` `case`) → `0,0,1,3,6,10,15`; all 8 REPEATs (result is REPEAT-**independent**, which is itself asserted) | [x] |
| 19 | `use_generated` | `OP=sub`, `n` = `0..6` → `0,0,-1,-3,-6,-10,-15` | [x] |
| 20 | `use_generated` | `OP=mul`, `n` = `0..6` → `1,1,2,6,24,120,720` | [x] |
| 21 | `use_generated` | `n` swept over `-8..=15` ∪ `{INT_MIN, INT_MIN+1, INT_MAX-1, INT_MAX}` — mixes valid `case`s with the `default:` arm, all 24 | [x] |
| 22 | `G_OP` (data) | initial value equals the exported `op_<OP>` of the *same* library, for each `OP`; called with random + boundary `(a,b)` | [x] |
| 23 | `G_OP` (data) | overwritten with `op_add`, then `op_sub`, then `op_mul`, calling after each store — writable-global semantics, all 24 | [x] |
| 24 | `G_OP_NAME` (data) | pointed-to C string is exactly `"add"`/`"sub"`/`"mul"` per `OP`; byte-compared incl. NUL | [x] |
| 25 | `G_OP_NAME` (data) | overwritten with another string pointer and read back — writable-global semantics | [x] |
| 26 | all 6 functions | **interleaved** call sequence in one process (`helper_call`→`use_generated`→`helper_ptr`→`op_*`→`G_OP`, repeated) with random inputs — catches cross-call state leakage and `printf` buffering differences invisible to per-function tests | [x] |
| 27 | all 6 functions | composed pipeline: output of one call fed as input to the next for 64 generations (value-dependent path coverage incl. self-induced overflow), all 24 | [x] |
| 28 | `driver` executable | `OP=add` × `REPEAT=0..7`, args `("3","4")` — full stdout (5 `printf` lines incl. `summary=`), stderr, exit status | [x] |
| 29 | `driver` executable | `OP=sub` × `REPEAT=0..7`, args `("3","4")` | [x] |
| 30 | `driver` executable | `OP=mul` × `REPEAT=0..7`, args `("3","4")` | [x] |
| 31 | `driver` executable | all 24 configs × random argument pairs (40 pairs incl. negatives and `INT_MAX`/`INT_MIN` decimal text) | [x] |
| 32 | `driver` executable | all 24 configs — note `main` calls `use_generated(REPEAT)`, so at `REPEAT=7` the `switch` `default:` arm is hit and `x3` collapses to `INIT`; the `summary=` line must reflect that | [x] |
| 33 | Cargo defaults | **no `OP` feature** + `REPEAT=0..7` ≡ C built with `-DOP=add` (`#ifndef OP` fallback) — same library + executable comparison | [x] |
| 34 | Cargo defaults | **no `REPEAT` feature** + `OP=add/sub/mul` ≡ C built with `-DREPEAT=5` (`#ifndef REPEAT` fallback) | [x] |
| 35 | Cargo defaults | **neither feature** ≡ C built with no `-D` at all (`OP=add`, `REPEAT=5`) | [x] |
| 36 | feature precedence | multi-feature combos (`add,sub`, `mul,3,5`, …) resolve to one deterministic config and still compile — all 36 Cargo combinations `cargo check`ed | [x] |
| 37 | CMake build | the CMake-generated `driver` (default cache values `OP=add`, `REPEAT=5`, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`) produces byte-identical stdout to the plain-`gcc` executable used by the harness, validating the harness's flag reproduction | [x] |
| 38 | `nm -D` parity | C `.so` vs Rust `.so` symbol sets, per configuration | [x] |
| 39 | `helper_ptr` vs `G_OP` | `helper_ptr` copies `OP_FN(OP)` into a *local* `fp`, so it must ignore `G_OP` even after the global is overwritten — pins a plausible mistranslation that routes it through the global | [x] |
| 40 | `driver` executable | non-UTF-8 argument bytes, empty `argv[0]`, and randomized `atoi` fuzz (see `ERRORS.md` G6–G8) | [x] |
| 41 | release profile | all of the above re-run with `--release` (`panic = "abort"` + optimised codegen) | [x] |

## Result

Every row passes. Coverage was executed as
`./check_all.sh test` and `./check_all.sh release`:

* **45 Cargo feature combinations** verified (the 36 of the `{∅,add,sub,mul}` ×
  `{∅,0..7}` matrix, plus 9 multi-feature combinations that only Cargo can
  express), each rebuilt from scratch and diffed against a C `.so` + C executable
  compiled with the corresponding `-DOP`/`-DREPEAT`.
* **54 tests per combination** → 2 430 test executions per profile, 4 860 in
  total, all passing.
* Harness sensitivity is itself verified: `./mutation_check.sh` injects 12 known
  divergences (wrong `INIT_mul`, `STEP_mul` missing its `+1`, off-by-one
  `RUN_LOOP`, `default:` arm computing instead of returning `INIT`, a one-space
  `printf` change, a dropped `#[no_mangle]`, `G_OP` demoted to a read-only
  `static`, `atoi` saturating instead of truncating, …) and confirms the suite
  catches **all 12**.
