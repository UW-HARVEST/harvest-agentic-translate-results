# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

## The axes the C code actually branches on

Derived from `c_src/CMakeLists.txt`, `c_src/src/mdmacros.h` (the public header)
and the two translation units — not from assumptions.

### Build-time axes (`#ifdef` / token-paste axes)

| axis | source | values | C effect |
|------|--------|--------|----------|
| `OP` | `CMakeLists.txt:26` → `-DOP=`; `mdmacros.h:27-29` (`#ifndef OP → add`) | `add`, `sub`, `mul` | selects `OP_FN(OP)` (`op_add`/`op_sub`/`op_mul`), `STEP_OP` (`+= i` / `-= i` / `*= (i+1)`), `INIT_FOR(OP)` (`0`/`0`/`1`), `STR(OP)` (`"add"`/`"sub"`/`"mul"`), and which `accum_<OP>` is generated |
| `REPEAT` | `CMakeLists.txt:27` → `-DREPEAT=`; `mdmacros.h:30-32` (`#ifndef REPEAT → 5`) | `0,1,2,3,4,5,6,7` (`REP0`..`REP7` are the only ones defined — `mdmacros.h:63-70`) | `RUN_LOOP(OP, acc, REPEAT)` = `CHOOSE_REP(REPEAT)` = the statically unrolled `REP<REPEAT>` chain in `helper_call` and in `main`; also the argument `main` passes to `use_generated(REPEAT)` |

Cargo mirror: features `add`/`sub`/`mul` and `0`..`7`; selecting none gives the
CMake defaults (`add`, `5`). **24 valid configurations** = 3 `OP` × 8 `REPEAT`
(plus the equivalent "feature omitted → default" spellings, which
`scripts/combos.sh` also enumerates → 36 `cargo check` combinations, and 8
further conflicting ones that must still compile).

Runners:

| script | what it covers |
|--------|----------------|
| `scripts/check_features.sh` | `cargo check --no-default-features --features …` for all 36 valid spellings + 8 conflicting ones (44 total, all clean) |
| `scripts/run_all.sh` | the 24 `(OP, REPEAT)` pairs: build C `.so` + C CMake executable + Rust `cdylib`, run the whole differential suite against each |
| `scripts/run_combos.sh --spellings` | the 18 remaining feature spellings (omitted defaults → CMake defaults; conflicting sets → documented `mul > sub > add` / highest-`REPEAT` priority) |
| `scripts/check_symbols.sh` | `nm -D` parity for all 24 configurations |

### Runtime state axes (mutable exported globals — `mdcore.c:36-37`)

| axis | values exercised | who observes it |
|------|------------------|-----------------|
| `G_OP` (writable `.data` function pointer) | (a) untouched (build-time default), (b) overwritten with `op_add`/`op_sub`/`op_mul` — i.e. an op that disagrees with the build-time `OP`, (c) `NULL` (→ `ERRORS.md` E17) | only `main` (`int g = G_OP(a,b)`); `helper_ptr`/`helper_call` must **not** observe it |
| `G_OP_NAME` (writable `.data` `const char *`) | (a) untouched, (b) repointed at another string, (c) `NULL` (→ E18) | only `main`'s first `printf` (`op=%s`) |

### Input-shape axes

| entry point | shape axes |
|-------------|-----------|
| `op_add`/`op_sub`/`op_mul(a,b)` | zero, ±1, small, `INT_MAX`/`INT_MIN` boundaries, sign combinations, random 32-bit |
| `helper_call(a,b)`, `helper_ptr(a,b)` | same operand shapes; result also depends on `OP`+`REPEAT` |
| `use_generated(n)` | `n` in-range `0`,`1`,`2..5`,`6` (each a *different* `case` arm / unroll depth) and out-of-range (→ `ERRORS.md`) |
| `main(argc, argv)` | `argc` = 3 vs `> 3` vs `< 3` (→ `ERRORS.md`); `argv[1]/argv[2]` decimal text: zero, negative, `INT_MAX`/`INT_MIN` text, leading spaces/`+`, random |

### Full set of public entry points (`mdmacros.h:104-110` + `mdmain.c`)

Lowest level first: `op_add`, `op_sub`, `op_mul` (leaf ops) → `helper_ptr`
(indirect call through a local fn pointer) → `helper_call` (op + unrolled
`REP<REPEAT>` accumulator) → `use_generated` (the macro-generated
`accum_<OP>` dispatcher) → `main` (composes all of them + both globals). Data
symbols `G_OP` and `G_OP_NAME` are entry points too (read *and* write).

## Row table

Every row is run for **all 24 `(OP, REPEAT)` builds** unless the row's
configuration column pins the axis, and every row uses **many randomized inputs
with a fixed seed** (`tests/common/mod.rs`, xorshift64\*, `SEED =
0x0020_2409_17C0_FFEE`, each test using a distinct `SEED ^ k` stream) in addition
to the listed boundary values. A row is checked only when it passes byte-for-byte
(return value **and** captured stdout/stderr) for every one of those 24 builds —
plus the 18 further feature *spellings* (omitted-default and conflicting feature
sets) run by `scripts/run_combos.sh --spellings`, i.e. 42 builds in total.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|-----|
| C01 | `op_add` | any build; `(0,0)`, `(0,±1)`, `(±1,0)` | [x] |
| C02 | `op_add` | any build; small random pairs (256 randomized) | [x] |
| C03 | `op_add` | any build; `INT_MAX`/`INT_MIN` boundary pairs (wrap) | [x] |
| C04 | `op_add` | any build; full-range random 32-bit pairs (1024 randomized) | [x] |
| C05 | `op_sub` | any build; `(0,0)`, `(0,±1)`, `(±1,0)` | [x] |
| C06 | `op_sub` | any build; small random pairs | [x] |
| C07 | `op_sub` | any build; `INT_MAX`/`INT_MIN` boundary pairs (wrap, incl. `INT_MIN - 1`) | [x] |
| C08 | `op_sub` | any build; full-range random 32-bit pairs | [x] |
| C09 | `op_mul` | any build; `(0,x)`, `(1,x)`, `(-1,x)` | [x] |
| C10 | `op_mul` | any build; small random pairs | [x] |
| C11 | `op_mul` | any build; `INT_MAX`/`INT_MIN` boundary pairs (incl. `INT_MIN * -1`, `INT_MIN * INT_MIN`) | [x] |
| C12 | `op_mul` | any build; full-range random 32-bit pairs | [x] |
| C13 | `helper_ptr` | `OP=add`, every `REPEAT`; boundary + random operands; stdout `helper.ptr=%d` compared | [x] |
| C14 | `helper_ptr` | `OP=sub`, every `REPEAT`; boundary + random operands | [x] |
| C15 | `helper_ptr` | `OP=mul`, every `REPEAT`; boundary + random operands | [x] |
| C16 | `helper_ptr` after caller writes a *different* op into `G_OP` | every `(OP, REPEAT)`; must still use the build-time op (state axis (b)) | [x] |
| C17 | `helper_call` | `OP=add` × `REPEAT=0` (empty `REP0` unroll, `acc` stays `INIT`) | [x] |
| C18 | `helper_call` | `OP=add` × `REPEAT=1` | [x] |
| C19 | `helper_call` | `OP=add` × `REPEAT=2,3,4` | [x] |
| C20 | `helper_call` | `OP=add` × `REPEAT=5` (CMake default) | [x] |
| C21 | `helper_call` | `OP=add` × `REPEAT=6,7` | [x] |
| C22 | `helper_call` | `OP=sub` × `REPEAT=0..7` (negative accumulator) | [x] |
| C23 | `helper_call` | `OP=mul` × `REPEAT=0..7` (`INIT=1`, factorial accumulator) | [x] |
| C24 | `helper_call` | every `(OP, REPEAT)`; operands `INT_MAX`/`INT_MIN` so `r + acc` wraps | [x] |
| C25 | `helper_call` | every `(OP, REPEAT)`; 512 randomized full-range operand pairs | [x] |
| C26 | `use_generated` | every `(OP, REPEAT)`; `n = 0` (`case 0` → `REP0`, returns `INIT`) | [x] |
| C27 | `use_generated` | every `(OP, REPEAT)`; `n = 1` (`case 1`) | [x] |
| C28 | `use_generated` | every `(OP, REPEAT)`; `n = 2,3,4,5` (each `case`) | [x] |
| C29 | `use_generated` | every `(OP, REPEAT)`; `n = 6` (last `case`) | [x] |
| C30 | `use_generated` | every `(OP, REPEAT)`; `n` swept `-8..=16` plus randomized `n` (mixes valid/invalid arms) | [x] |
| C31 | `G_OP` (read) | every `(OP, REPEAT)`; the stored pointer must resolve to the same exported op symbol as in C (compared by dlsym address) and calling through it must match | [x] |
| C32 | `G_OP` (write) | every `(OP, REPEAT)`; overwrite with each of `op_add`/`op_sub`/`op_mul`, then re-read and call through it (proves the object lives in writable `.data`) | [x] |
| C33 | `G_OP_NAME` (read) | every `(OP, REPEAT)`; the pointed-to C string must equal `STR(OP)` byte-for-byte | [x] |
| C34 | `G_OP_NAME` (write) | every `(OP, REPEAT)`; repoint at another string, re-read, and observe it through `main`'s `op=%s` output | [x] |
| C35 | `main` | every `(OP, REPEAT)`; `argc=3`, `argv = ["prog","3","4"]` (the documented happy path; full stdout compared) | [x] |
| C36 | `main` | every `(OP, REPEAT)`; `argc=3` with negative/zero operand text (`"0"`,`"-7"`,`"-0"`,`"+7"`,`"  12"`) | [x] |
| C37 | `main` | every `(OP, REPEAT)`; `argc=3` with `INT_MAX`/`INT_MIN` operand text (`summary` wraps) | [x] |
| C38 | `main` | every `(OP, REPEAT)`; `argc=3` with 128 randomized decimal operand pairs | [x] |
| C39 | `main` | every `(OP, REPEAT)`; `argc > 3` (extra args ignored) | [x] |
| C40 | `main` | every `(OP, REPEAT)`; after the caller overwrote `G_OP` with a different op — only `g.call`/`summary` change | [x] |
| C41 | `main` | every `(OP, REPEAT)`; after the caller repointed `G_OP_NAME` — only `op=` changes | [x] |
| C42 | pipeline (`helper_call` → `helper_ptr` → `use_generated` → `main`, in one process, sharing the globals) | every `(OP, REPEAT)`; randomized operand sequence, all four called in the C `.so` and the Rust `.so` in the same order, with the interleaved stdout of the whole sequence compared as one byte stream | [x] |
