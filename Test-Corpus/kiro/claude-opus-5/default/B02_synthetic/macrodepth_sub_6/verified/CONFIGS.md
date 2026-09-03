# CONFIGS.md — configuration surface (valid inputs)

Axes derived from the branches the C source actually takes.

## Build-time axes (the only two, both from `CMakeLists.txt` cache variables)

| axis | values the C accepts | how the C branches on it | Cargo feature |
|------|----------------------|--------------------------|---------------|
| `OP` | `add`, `sub`, `mul` | `OP_FN` → `CAT(op_, OP)` (`mdmacros.h:45`), `STEP_OP` → `CAT(STEP_, OP)` (`:52`), `INIT_FOR` → `CAT(INIT_, OP)` (`:59`), `ACCUM_FN` → `CAT(accum_, OP)` (`:101`), `STR(OP)` for `G_OP_NAME` (`mdcore.c:37`) | `add` / `sub` / `mul` |
| `REPEAT` | `0`..`7` | `RUN_LOOP` → `CHOOSE_REP(REPEAT)` → `REP<REPEAT>` (`mdmacros.h:73-79`); `use_generated(REPEAT)` feeds `DISPATCH_REP`'s `switch` (`:82-93`) | `0`..`7` |
| fallback | `OP` / `REPEAT` left undefined → `#ifndef` gives `add` / `5` (`mdmacros.h:27-32`) | — | no features at all |

Cross product = **24 configurations**, each of which changes the behaviour of every
exported function. The derived constants (verified against the reference binaries):

| OP | `INIT` | `STR(OP)` | `RUN_LOOP` result by `REPEAT` 0→7 | `use_generated(REPEAT)` by `REPEAT` 0→7 |
|----|--------|-----------|-----------------------------------|------------------------------------------|
| `add` | 0 | `"add"` | 0, 0, 1, 3, 6, 10, 15, 21 (`n(n-1)/2`) | 0, 0, 1, 3, 6, 10, 15, **0** |
| `sub` | 0 | `"sub"` | 0, 0, -1, -3, -6, -10, -15, -21 | 0, 0, -1, -3, -6, -10, -15, **0** |
| `mul` | 1 | `"mul"` | 1, 1, 2, 6, 24, 120, 720, 5040 (`n!`) | 1, 1, 2, 6, 24, 120, 720, **1** |

The bold entries are the `REPEAT=7` asymmetry: `DISPATCH_REP` has no `case 7:`, so
`use_generated(7)` returns `INIT` while `RUN_LOOP` (which uses `REP7` directly)
returns the 7-step value. Any row with `REPEAT=7` exercises it.

## Runtime axes

**Entry points** — the complete exported surface, lowest level first, not just the
composed helpers:

1. `op_add(int,int)`, `op_sub(int,int)`, `op_mul(int,int)` — leaf arithmetic; all three
   are exported in *every* configuration, so the two that `OP` did not select must
   still be driven directly.
2. `G_OP` — data slot: read the pointer, call through it, and (it is non-`const`)
   overwrite it and call through it again.
3. `G_OP_NAME` — data slot: read the `const char *` and compare the pointed-to bytes.
4. `helper_ptr(int,int)` — `OP_FN` through a local function pointer + one `printf`.
5. `helper_call(int,int)` — `OP_FN` + `RUN_LOOP(OP, acc, REPEAT)` + one `printf`;
   the only entry point that depends on **both** `OP` and `REPEAT`.
6. `use_generated(int)` — the `static accum_<OP>` behind `DISPATCH_REP`'s `switch`;
   the only entry point taking a runtime selector.
7. `main(argc, argv)` — the composed pipeline: `atoi` × 2, then `OP_FN`, `RUN_LOOP`,
   `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`, and two `printf`s
   whose second line is the wrapping sum of all six results. Driven as a process.

**Input shapes the code special-cases:**

- `op_*` operands: zero / one / small positive / small negative / mixed sign /
  `INT_MAX` / `INT_MIN` / overflow-producing pairs / uniformly random `i32`.
- `use_generated(n)` selector: each `switch` arm `0,1,2,3,4,5,6` individually; the
  `default` partition `n=7`, `n>7`, `n<0`, `INT_MIN`, `INT_MAX`.
- `main` argv shapes: `argc >= 3`; leading whitespace; explicit `+`/`-`; empty string;
  no digits; trailing garbage; positive and negative magnitude overflow; extra
  arguments beyond `argv[2]` (ignored).
- Observable output: every helper writes a line to **stdout** via `printf`. Return
  value parity alone is insufficient — each row compares captured stdout bytes too.

## Configuration table

`{op}` = each of add/sub/mul, `{r}` = each of 0..7. Every row is run for all 24
`op × r` builds, i.e. the table is the cross product of these rows with the 24
build configurations. Rows are checked off only after passing across randomized
inputs (seeded `SplitMix64`, seed `0x5EED_1234_ABCD_9876`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `op_add` | `{op}/{r}`; 4096 random `(i32,i32)` pairs | [x] |
| 2 | `op_add` | `{op}/{r}`; boundary pairs: `(0,0) (0,1) (1,0) (-1,1) (INT_MAX,0) (INT_MAX,1) (INT_MIN,0) (INT_MIN,-1) (INT_MAX,INT_MAX) (INT_MIN,INT_MIN) (INT_MAX,INT_MIN)` | [x] |
| 3 | `op_sub` | `{op}/{r}`; 4096 random `(i32,i32)` pairs | [x] |
| 4 | `op_sub` | `{op}/{r}`; same boundary pair set | [x] |
| 5 | `op_mul` | `{op}/{r}`; 4096 random `(i32,i32)` pairs | [x] |
| 6 | `op_mul` | `{op}/{r}`; same boundary pair set, plus `(65536,65536) (-1,INT_MIN) (INT_MIN,-1)` | [x] |
| 7 | `op_add`/`op_sub`/`op_mul` | `{op}/{r}`; the **non-selected** ops driven directly, confirming all three stay exported and correct regardless of which one `OP` picked | [x] |
| 8 | `helper_ptr` | `{op}/{r}`; 1024 random pairs; return value **and** captured stdout `helper.ptr=<r>\n` | [x] |
| 9 | `helper_ptr` | `{op}/{r}`; boundary pair set; stdout compared | [x] |
| 10 | `helper_call` | `{op}/{r}`; 1024 random pairs; return value **and** stdout `helper.call=<r> helper.acc=<acc>\n` — `acc` is the `REP<r>` unrolling, so this row is where `REPEAT` shows up | [x] |
| 11 | `helper_call` | `{op}/{r}`; boundary pair set; stdout compared (covers `r + acc` wrapping at `INT_MAX`) | [x] |
| 12 | `use_generated` | `{op}/{r}`; `n = 0` (`case 0:` → `REP0`, empty body) | [x] |
| 13 | `use_generated` | `{op}/{r}`; `n = 1` | [x] |
| 14 | `use_generated` | `{op}/{r}`; `n = 2` | [x] |
| 15 | `use_generated` | `{op}/{r}`; `n = 3` | [x] |
| 16 | `use_generated` | `{op}/{r}`; `n = 4` | [x] |
| 17 | `use_generated` | `{op}/{r}`; `n = 5` | [x] |
| 18 | `use_generated` | `{op}/{r}`; `n = 6` (last `case`) | [x] |
| 19 | `use_generated` | `{op}/{r}`; `n = REPEAT` — the call `mdmain.c:42` actually makes; hits `default` when `REPEAT == 7` | [x] |
| 20 | `use_generated` | `{op}/{r}`; 2048 random `i32` selectors spanning `default` and the seven cases; return value **and** stdout `gen.acc=<r>\n` | [x] |
| 21 | `G_OP` | `{op}/{r}`; read the slot, call through it with 1024 random pairs; result must equal the directly-called `op_{op}` | [x] |
| 22 | `G_OP` | `{op}/{r}`; the loaded pointer's value must equal the address of that `.so`'s own `op_{op}` export (correct macro selection, not just correct arithmetic) | [x] |
| 23 | `G_OP` | `{op}/{r}`; slot is non-`const`: overwrite with each of the other two `op_*` addresses, call through, restore — both sides must dispatch to the stored pointer | [x] |
| 24 | `G_OP_NAME` | `{op}/{r}`; dereference and compare NUL-terminated bytes; must be `"add"`/`"sub"`/`"mul"` per `STR(OP)` | [x] |
| 25 | full pipeline in one process | `{op}/{r}`; `helper_call` → `helper_ptr` → `use_generated(REPEAT)` → `G_OP` called back-to-back on the same loaded library, with **all** stdout captured as one stream — catches ordering/buffering divergence invisible to per-call tests | [x] |
| 26 | `main` (process) | `{op}/{r}`; `argc >= 3` with 512 random `(i32,i32)` operand pairs rendered as decimal; full stdout + stderr + exit status | [x] |
| 27 | `main` (process) | `{op}/{r}`; argv lexical shapes: `"  -12abc" "+9"`, `"007" "-0"`, `"" ""`, `"12x" "7"`, `"2147483647" "2"`, `"-2147483648" "-1"`, `"99999999999999999999" "3"`, `"-99999999999999999999" "3"`, `"9223372036854775807" "1"`, `"-9223372036854775808" "1"`, plus a 4th ignored argument | [x] |
| 28 | build fallback | no `-D` at all / no Cargo features: `#ifndef` must give `OP=add`, `REPEAT=5`; compare against the `add`/`5` build | [x] |

Rows 1–25 run through `libloading` against both `.so`s inside one test process.
Rows 26–28 run the two `driver` executables. All 28 rows are re-run for each of the
24 feature combinations by `sweep_so.sh`.

## Status

All 28 rows pass. Row-to-test mapping:

| rows | test(s) | file |
|------|---------|------|
| 1–2 | `cfg_01_02_op_add_random_and_boundaries` | `tests/phase_b_valid.rs` |
| 3–4 | `cfg_03_04_op_sub_random_and_boundaries` | `tests/phase_b_valid.rs` |
| 5–6 | `cfg_05_06_op_mul_random_and_boundaries` | `tests/phase_b_valid.rs` |
| 7 | `cfg_07_non_selected_ops_still_exported_and_correct` | `tests/phase_b_valid.rs` |
| 8–9 | `cfg_08_helper_ptr_random`, `cfg_09_helper_ptr_boundaries` | `tests/phase_b_valid.rs` |
| 10–11 | `cfg_10_helper_call_random`, `cfg_10b_helper_call_acc_matches_rep_unrolling`, `cfg_11_helper_call_boundaries` | `tests/phase_b_valid.rs` |
| 12–18 | `cfg_12_use_generated_case_0` … `cfg_18_use_generated_case_6` | `tests/phase_b_valid.rs` |
| 19 | `cfg_19_use_generated_of_repeat` | `tests/phase_b_valid.rs` |
| 20 | `cfg_20_use_generated_random_selectors` | `tests/phase_b_valid.rs` |
| 21–23 | `cfg_21_g_op_dispatches_like_selected_op`, `cfg_22_g_op_points_at_own_op_export`, `cfg_23_g_op_writable` | `tests/phase_b_valid.rs` |
| 24 | `cfg_24_g_op_name_bytes` | `tests/phase_b_valid.rs` |
| 25 | `cfg_25_full_pipeline_single_stream` | `tests/phase_b_valid.rs` |
| 26 | `cfg_26_main_random_operand_pairs`, `cfg_26b_main_boundary_operand_pairs` | `tests/phase_b_exe.rs` |
| 27 | `cfg_27_main_argv_lexical_shapes`, `cfg_27b_main_repeat_dependent_lines` | `tests/phase_b_exe.rs` |
| 28 | `cfg_28_no_define_falls_back_to_add_5` | `tests/phase_d_symbols.rs` |

`./sweep_so.sh` runs the whole table for all 26 configurations (24 `OP × REPEAT`,
plus no-features and `--all-features`).

Rows that assert absolute values as well as C/Rust equality — so a jointly wrong
pair could not pass — are 7, 10b, 19, 21, 22, 23, 24, 27b and 28.
