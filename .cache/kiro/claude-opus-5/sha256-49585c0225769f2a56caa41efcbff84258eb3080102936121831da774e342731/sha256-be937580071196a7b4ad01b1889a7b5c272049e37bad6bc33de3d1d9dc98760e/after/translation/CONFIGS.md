# CONFIGS.md — configuration surface (valid inputs) of the C library

Mirror of `ERRORS.md` for inputs the C code *accepts*. Rows are the pruned
cross-product of the axes the C source actually branches on.

## Axes, derived from the source

### Build-time axes (`c_src/CMakeLists.txt` cache variables → `-D` → Cargo features)

| axis | values the C accepts | what it switches inside the C |
|------|----------------------|-------------------------------|
| `OP` | `add`, `sub`, `mul`, or **undefined** → `mdmacros.h:27 #define OP add` | `OP_FN(op)` → `CAT(op_, op)` picks `op_add`/`op_sub`/`op_mul`; `STEP_OP` → `STEP_add`(`+=i`) / `STEP_sub`(`-=i`) / `STEP_mul`(`*=(i+1)`); `INIT_FOR` → `INIT_add`=0 / `INIT_sub`=0 / `INIT_mul`=1; `STR(OP)` → the `G_OP_NAME` literal; `DEFINE_ACCUM(OP)` → `accum_add`/`accum_sub`/`accum_mul` |
| `REPEAT` | `0`..`7`, or **undefined** → `mdmacros.h:30 #define REPEAT 5`. Only `0..7` are valid because the header defines exactly `REP0`..`REP7`; `CHOOSE_REP(8)` would name an undefined `REP8`. | `RUN_LOOP(op, acc, REPEAT)` → `CHOOSE_REP(REPEAT)` → one of `REP0`..`REP7`, i.e. the step applied with literal indices `0 .. REPEAT-1`. Also fixes the argument of `use_generated(REPEAT)` in `main`. |

Cargo mirrors these as `add`/`sub`/`mul` and `repeat_0`..`repeat_7` (with the
bare CMake-value aliases `"0"`..`"7"`), giving **3 × 8 = 24** buildable
configurations, plus the two "unset" fallbacks (C: no `-D` flags; Rust:
`--no-default-features`) which must equal `add`/`5`.

### Runtime axes

| axis | values |
|------|--------|
| entry point | the FULL exported set, lowest level first: `op_add`, `op_sub`, `op_mul` (leaf primitives) → `G_OP` / `G_OP_NAME` (exported data) → `helper_ptr`, `helper_call`, `use_generated` (composed) → the `driver` executable (the whole pipeline). No convenience wrapper is used as a substitute for the leaf entry point. |
| `(a, b)` shape | `0/0`; small positive; small negative; mixed sign; the boundary set `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` squared; overflow-inducing pairs; randomized over the full `i32` range with a fixed seed |
| `n` shape for `use_generated` | each `DISPATCH_REP` `case` `0`..`6` separately (they are seven *different* unrolled bodies, not one loop), plus the `default:` values covered in `ERRORS.md` |
| exported-global mutability | read-only use vs. **store** into `G_OP` / `G_OP_NAME` (both declared non-`const` in `mdmacros.h`, both in writable `.data` in the C `.so`) |
| `driver` argv shape | `argc` 0/1/2/3/>3; operand text shapes accepted by `atoi` (plain decimal, leading whitespace, explicit `+`/`-`, digit-prefixed garbage, `long`-overflowing, `int`-truncating) |

Every row is exercised with **many randomized inputs** (fixed seed `0x5EED`,
`SplitMix64`) rather than one hand-picked value, and both the return value and
the emitted stdout bytes are compared.

<!-- BEGIN GENERATED ROWS (gen_configs.sh) -->
### G1 — the three operation primitives (`OP`-independent code, present in every build)

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G1-0 | `op_add` | any OP build x any REPEAT build (24 configs); both operands 0 | [x] |
| G1-1 | `op_add` | any OP build x any REPEAT build (24 configs); small positives (1..1000), randomized, seed 0x5EED | [x] |
| G1-2 | `op_add` | any OP build x any REPEAT build (24 configs); small negatives (-1000..-1), randomized | [x] |
| G1-3 | `op_add` | any OP build x any REPEAT build (24 configs); mixed signs, randomized over full i32 | [x] |
| G1-4 | `op_add` | any OP build x any REPEAT build (24 configs); boundary set {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX} x itself (full 7x7) | [x] |
| G1-5 | `op_add` | any OP build x any REPEAT build (24 configs); overflow-inducing pairs (INT_MAX+1, INT_MIN-1, INT_MAX*INT_MAX, INT_MIN*-1) | [x] |
| G1-6 | `op_sub` | any OP build x any REPEAT build (24 configs); both operands 0 | [x] |
| G1-7 | `op_sub` | any OP build x any REPEAT build (24 configs); small positives (1..1000), randomized, seed 0x5EED | [x] |
| G1-8 | `op_sub` | any OP build x any REPEAT build (24 configs); small negatives (-1000..-1), randomized | [x] |
| G1-9 | `op_sub` | any OP build x any REPEAT build (24 configs); mixed signs, randomized over full i32 | [x] |
| G1-10 | `op_sub` | any OP build x any REPEAT build (24 configs); boundary set {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX} x itself (full 7x7) | [x] |
| G1-11 | `op_sub` | any OP build x any REPEAT build (24 configs); overflow-inducing pairs (INT_MAX+1, INT_MIN-1, INT_MAX*INT_MAX, INT_MIN*-1) | [x] |
| G1-12 | `op_mul` | any OP build x any REPEAT build (24 configs); both operands 0 | [x] |
| G1-13 | `op_mul` | any OP build x any REPEAT build (24 configs); small positives (1..1000), randomized, seed 0x5EED | [x] |
| G1-14 | `op_mul` | any OP build x any REPEAT build (24 configs); small negatives (-1000..-1), randomized | [x] |
| G1-15 | `op_mul` | any OP build x any REPEAT build (24 configs); mixed signs, randomized over full i32 | [x] |
| G1-16 | `op_mul` | any OP build x any REPEAT build (24 configs); boundary set {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX} x itself (full 7x7) | [x] |
| G1-17 | `op_mul` | any OP build x any REPEAT build (24 configs); overflow-inducing pairs (INT_MAX+1, INT_MIN-1, INT_MAX*INT_MAX, INT_MIN*-1) | [x] |

### G2 — `G_OP`: the exported function-pointer datum (selected by `OP`)

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G2-18 | `G_OP` (dlsym, deref, call) | OP=add, REPEAT=* ; randomized full-i32 pairs, 256 draws, fixed seed | [x] |
| G2-19 | `G_OP` (dlsym, deref, call) | OP=add, REPEAT=* ; boundary set {INT_MIN..INT_MAX} 7x7 | [x] |
| G2-20 | `G_OP` vs `op_add` | OP=add ; identity check - the stored pointer must equal the address of the exported `op_add` in the same `.so` | [x] |
| G2-21 | `G_OP` (dlsym, deref, call) | OP=sub, REPEAT=* ; randomized full-i32 pairs, 256 draws, fixed seed | [x] |
| G2-22 | `G_OP` (dlsym, deref, call) | OP=sub, REPEAT=* ; boundary set {INT_MIN..INT_MAX} 7x7 | [x] |
| G2-23 | `G_OP` vs `op_sub` | OP=sub ; identity check - the stored pointer must equal the address of the exported `op_sub` in the same `.so` | [x] |
| G2-24 | `G_OP` (dlsym, deref, call) | OP=mul, REPEAT=* ; randomized full-i32 pairs, 256 draws, fixed seed | [x] |
| G2-25 | `G_OP` (dlsym, deref, call) | OP=mul, REPEAT=* ; boundary set {INT_MIN..INT_MAX} 7x7 | [x] |
| G2-26 | `G_OP` vs `op_mul` | OP=mul ; identity check - the stored pointer must equal the address of the exported `op_mul` in the same `.so` | [x] |

### G3 — `G_OP_NAME`: the exported string-pointer datum (`STR(OP)`)

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G3-27 | `G_OP_NAME` (dlsym, deref, read NUL-terminated bytes) | OP=add, REPEAT=* ; expect exactly `"add"` + NUL | [x] |
| G3-28 | `G_OP_NAME` (dlsym, deref, read NUL-terminated bytes) | OP=sub, REPEAT=* ; expect exactly `"sub"` + NUL | [x] |
| G3-29 | `G_OP_NAME` (dlsym, deref, read NUL-terminated bytes) | OP=mul, REPEAT=* ; expect exactly `"mul"` + NUL | [x] |

### G4 — `helper_ptr` (calls through a local fn pointer; `REPEAT`-independent)

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G4-30 | `helper_ptr` | OP=add, REPEAT=* ; randomized full-i32 pairs, 256 draws; compare return value AND the `helper.ptr=%d` stdout line | [x] |
| G4-31 | `helper_ptr` | OP=add, REPEAT=* ; boundary set 7x7 incl. overflow; compare return AND stdout | [x] |
| G4-32 | `helper_ptr` | OP=sub, REPEAT=* ; randomized full-i32 pairs, 256 draws; compare return value AND the `helper.ptr=%d` stdout line | [x] |
| G4-33 | `helper_ptr` | OP=sub, REPEAT=* ; boundary set 7x7 incl. overflow; compare return AND stdout | [x] |
| G4-34 | `helper_ptr` | OP=mul, REPEAT=* ; randomized full-i32 pairs, 256 draws; compare return value AND the `helper.ptr=%d` stdout line | [x] |
| G4-35 | `helper_ptr` | OP=mul, REPEAT=* ; boundary set 7x7 incl. overflow; compare return AND stdout | [x] |

### G5 — `helper_call` (the only function whose result depends on BOTH `OP` and `REPEAT`)

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G5-36 | `helper_call` | OP=add, REPEAT=0 (=> `RUN_LOOP` expands to `REP0`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-37 | `helper_call` | OP=add, REPEAT=1 (=> `RUN_LOOP` expands to `REP1`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-38 | `helper_call` | OP=add, REPEAT=2 (=> `RUN_LOOP` expands to `REP2`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-39 | `helper_call` | OP=add, REPEAT=3 (=> `RUN_LOOP` expands to `REP3`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-40 | `helper_call` | OP=add, REPEAT=4 (=> `RUN_LOOP` expands to `REP4`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-41 | `helper_call` | OP=add, REPEAT=5 (=> `RUN_LOOP` expands to `REP5`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-42 | `helper_call` | OP=add, REPEAT=6 (=> `RUN_LOOP` expands to `REP6`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-43 | `helper_call` | OP=add, REPEAT=7 (=> `RUN_LOOP` expands to `REP7`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-44 | `helper_call` | OP=sub, REPEAT=0 (=> `RUN_LOOP` expands to `REP0`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-45 | `helper_call` | OP=sub, REPEAT=1 (=> `RUN_LOOP` expands to `REP1`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-46 | `helper_call` | OP=sub, REPEAT=2 (=> `RUN_LOOP` expands to `REP2`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-47 | `helper_call` | OP=sub, REPEAT=3 (=> `RUN_LOOP` expands to `REP3`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-48 | `helper_call` | OP=sub, REPEAT=4 (=> `RUN_LOOP` expands to `REP4`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-49 | `helper_call` | OP=sub, REPEAT=5 (=> `RUN_LOOP` expands to `REP5`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-50 | `helper_call` | OP=sub, REPEAT=6 (=> `RUN_LOOP` expands to `REP6`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-51 | `helper_call` | OP=sub, REPEAT=7 (=> `RUN_LOOP` expands to `REP7`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-52 | `helper_call` | OP=mul, REPEAT=0 (=> `RUN_LOOP` expands to `REP0`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-53 | `helper_call` | OP=mul, REPEAT=1 (=> `RUN_LOOP` expands to `REP1`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-54 | `helper_call` | OP=mul, REPEAT=2 (=> `RUN_LOOP` expands to `REP2`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-55 | `helper_call` | OP=mul, REPEAT=3 (=> `RUN_LOOP` expands to `REP3`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-56 | `helper_call` | OP=mul, REPEAT=4 (=> `RUN_LOOP` expands to `REP4`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-57 | `helper_call` | OP=mul, REPEAT=5 (=> `RUN_LOOP` expands to `REP5`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-58 | `helper_call` | OP=mul, REPEAT=6 (=> `RUN_LOOP` expands to `REP6`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |
| G5-59 | `helper_call` | OP=mul, REPEAT=7 (=> `RUN_LOOP` expands to `REP7`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the `helper.call=%d helper.acc=%d` stdout line | [x] |

### G6 — `use_generated` -> `accum_<OP>` (one row per distinct `DISPATCH_REP` `case`)

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G6-60 | `use_generated` | OP=add, REPEAT=* ; n=0 -> `case 0: REP0` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-61 | `use_generated` | OP=add, REPEAT=* ; n=1 -> `case 1: REP1` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-62 | `use_generated` | OP=add, REPEAT=* ; n=2 -> `case 2: REP2` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-63 | `use_generated` | OP=add, REPEAT=* ; n=3 -> `case 3: REP3` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-64 | `use_generated` | OP=add, REPEAT=* ; n=4 -> `case 4: REP4` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-65 | `use_generated` | OP=add, REPEAT=* ; n=5 -> `case 5: REP5` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-66 | `use_generated` | OP=add, REPEAT=* ; n=6 -> `case 6: REP6` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-67 | `use_generated` | OP=sub, REPEAT=* ; n=0 -> `case 0: REP0` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-68 | `use_generated` | OP=sub, REPEAT=* ; n=1 -> `case 1: REP1` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-69 | `use_generated` | OP=sub, REPEAT=* ; n=2 -> `case 2: REP2` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-70 | `use_generated` | OP=sub, REPEAT=* ; n=3 -> `case 3: REP3` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-71 | `use_generated` | OP=sub, REPEAT=* ; n=4 -> `case 4: REP4` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-72 | `use_generated` | OP=sub, REPEAT=* ; n=5 -> `case 5: REP5` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-73 | `use_generated` | OP=sub, REPEAT=* ; n=6 -> `case 6: REP6` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-74 | `use_generated` | OP=mul, REPEAT=* ; n=0 -> `case 0: REP0` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-75 | `use_generated` | OP=mul, REPEAT=* ; n=1 -> `case 1: REP1` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-76 | `use_generated` | OP=mul, REPEAT=* ; n=2 -> `case 2: REP2` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-77 | `use_generated` | OP=mul, REPEAT=* ; n=3 -> `case 3: REP3` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-78 | `use_generated` | OP=mul, REPEAT=* ; n=4 -> `case 4: REP4` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-79 | `use_generated` | OP=mul, REPEAT=* ; n=5 -> `case 5: REP5` ; compare return AND the `gen.acc=%d` stdout line | [x] |
| G6-80 | `use_generated` | OP=mul, REPEAT=* ; n=6 -> `case 6: REP6` ; compare return AND the `gen.acc=%d` stdout line | [x] |

### G7 — writable exported globals (`mdmacros.h` declares both non-`const`)

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G7-81 | `G_OP` (store, then read back and call) | OP=add ; overwrite the global with the address of each of `op_add`/`op_sub`/`op_mul` from the same `.so`, then call through it on randomized pairs. Must not trap (C `.data` is writable). | [x] |
| G7-82 | `G_OP_NAME` (store, then read back) | OP=add ; overwrite the pointer with another string address inside the same `.so`, then read it back. Must not trap. | [x] |
| G7-83 | `G_OP` restore + `helper_call`/`helper_ptr` | OP=add ; after mutating `G_OP`, `helper_call`/`helper_ptr` must be UNAFFECTED (they use `OP_FN(OP)` directly, not the global) | [x] |
| G7-84 | `G_OP` (store, then read back and call) | OP=sub ; overwrite the global with the address of each of `op_add`/`op_sub`/`op_mul` from the same `.so`, then call through it on randomized pairs. Must not trap (C `.data` is writable). | [x] |
| G7-85 | `G_OP_NAME` (store, then read back) | OP=sub ; overwrite the pointer with another string address inside the same `.so`, then read it back. Must not trap. | [x] |
| G7-86 | `G_OP` restore + `helper_call`/`helper_ptr` | OP=sub ; after mutating `G_OP`, `helper_call`/`helper_ptr` must be UNAFFECTED (they use `OP_FN(OP)` directly, not the global) | [x] |
| G7-87 | `G_OP` (store, then read back and call) | OP=mul ; overwrite the global with the address of each of `op_add`/`op_sub`/`op_mul` from the same `.so`, then call through it on randomized pairs. Must not trap (C `.data` is writable). | [x] |
| G7-88 | `G_OP_NAME` (store, then read back) | OP=mul ; overwrite the pointer with another string address inside the same `.so`, then read it back. Must not trap. | [x] |
| G7-89 | `G_OP` restore + `helper_call`/`helper_ptr` | OP=mul ; after mutating `G_OP`, `helper_call`/`helper_ptr` must be UNAFFECTED (they use `OP_FN(OP)` directly, not the global) | [x] |

### G8 — the `driver` executable (`mdmain.c`), end-to-end stdout+stderr+exit-status

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G8-90 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=0 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-91 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=1 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-92 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=2 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-93 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=3 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-94 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=4 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-95 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=5 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-96 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=6 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-97 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=add, REPEAT=7 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-98 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=0 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-99 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=1 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-100 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=2 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-101 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=3 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-102 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=4 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-103 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=5 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-104 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=6 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-105 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=sub, REPEAT=7 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-106 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=0 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-107 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=1 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-108 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=2 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-109 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=3 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-110 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=4 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-111 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=5 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-112 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=6 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |
| G8-113 | `driver A B` (full pipeline: `OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, `G_OP_NAME`, `summary`) | OP=mul, REPEAT=7 ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + `atoi` shapes (non-numeric, digit-prefix, whitespace, explicit sign, `long` overflow, `int` truncation) + extra trailing argv + argc<3 | [x] |

### G9 — the implicit "no macro defined" build

| # | entry point(s) | configuration (options set + input shape) | [x] pass |
|---|----------------|--------------------------------------------|-----|
| G9-114 | all 8 exported symbols + `driver` | C compiled with **no** `-DOP`/`-DREPEAT` (`#ifndef` fallbacks `OP=add`, `REPEAT=5`) vs Rust `--no-default-features` with **no** feature at all; same input shapes as G1-G8 | [x] |
| G9-115 | all 8 exported symbols + `driver` | Cargo default feature set (`add`,`repeat_5`) vs C `-DOP=add -DREPEAT=5` | [x] |
| G9-116 | all 8 exported symbols | bare-CMake-value REPEAT aliases `"0"`..`"7"` must resolve identically to `repeat_0`..`repeat_7` (48 `cargo check` combos + artifact comparison) | [x] |

Total rows: 117
<!-- END GENERATED ROWS -->

## Row → test mapping

Every row is checked off only because a differential test passed it, in every
one of the 50 build configurations exercised by `../run_all_features.sh`
(24 canonical `OP`×`repeat_N`, the 24 bare-value aliases `"0"`..`"7"`, the
no-feature build, and the default feature set). Rows naming a specific
`OP`/`REPEAT` are asserted by the test binary compiled for that configuration.

| rows | test in `tests/phase_b_valid.rs` | what it compares |
|------|----------------------------------|------------------|
| G1-1 .. G1-18 | `g1_op_add`, `g1_op_sub`, `g1_op_mul` | return value **and** captured stdout bytes of `op_add`/`op_sub`/`op_mul`, over all six shapes (0/0, 64 small positives, 64 small negatives, 256 full-`i32` draws, the 7×7 boundary square, 12 overflow pairs) |
| G1-19 .. G2-24 | `g2_g_op_call` | calls through the `G_OP` datum on 256 random + 49 boundary + 12 overflow pairs |
| G2-25 .. G2-27 (identity) | `g2_g_op_identity` | the stored pointer equals that library's own exported `op_<OP>`, in both `.so`s |
| G3-28 .. G3-30 | `g3_g_op_name` | NUL-terminated bytes behind `G_OP_NAME`, and that they equal `STR(OP)` |
| G4-31 .. G4-36 | `g4_helper_ptr` | return value + the `helper.ptr=%d` line |
| G5-37 .. G5-60 | `g5_helper_call`, `g5_helper_call_acc_is_rep_repeat` | return value + the `helper.call=%d helper.acc=%d` line; the `acc` value is additionally pinned against an independent re-derivation of `REP<REPEAT>` |
| G6-61 .. G6-81 | `g6_use_generated_each_case`, `g6_use_generated_matches_unrolled_chain` | one assertion per `DISPATCH_REP` `case 0..6`, return value + `gen.acc=%d`, pinned against the unrolled chain |
| G7-82 .. G7-90 | `g7_g_op_is_writable`, `g7_g_op_name_is_writable`, `g7_mutating_g_op_does_not_affect_helpers` | stores into the exported globals must succeed (not trap) and must not change `helper_call`/`helper_ptr` |
| G8-91 .. G8-114 | `g8_driver_stdout_matches`, `g8_driver_reports_active_config` | the `driver` executables' stdout, stderr and exit status, byte for byte, over ~150 argv shapes, invoked as `./driver` from their own directory so `argv[0]` matches |
| G9-115 .. G9-117 | `g9_unset_macros_equal_add_repeat_5`, plus the `<none>` and `default` runs of `run_all_features.sh`, plus the 48-combo `cargo check`/alias sweep | the `#ifndef` fallbacks, and that the bare `"0"`..`"7"` aliases resolve identically to `repeat_0`..`repeat_7` |

Symbol-parity rows are in `SYMBOLS.md` and enforced by
`tests/phase_d_symbols.rs` (`d1`..`d5`), which runs in every configuration too.
