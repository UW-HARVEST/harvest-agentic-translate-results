# CONFIGS.md — configuration surface (valid inputs)

## Axes the C code actually branches on

Derived from the source, not from guesses.

### Build-time axes (`c_src/CMakeLists.txt` → `CMAKE_C_FLAGS "-DOP=${OP} -DREPEAT=${REPEAT}"`)

| axis | values the code accepts | what it selects | Cargo feature |
|------|------------------------|-----------------|---------------|
| `OP` | `add`, `sub`, `mul` — bounded by which `op_<OP>` (`mdmacros.h:41-43`), `STEP_<OP>` (`:48-50`) and `INIT_<OP>` (`:56-58`) exist | `OP_FN(OP)` (which function `G_OP`, `helper_call`, `helper_ptr` dispatch to), `STEP_OP` (the accumulator step), `INIT_FOR(OP)` (seed `0`/`0`/`1`), `STR(OP)` (`G_OP_NAME`), and the name of the `static` `accum_<OP>` | `add` / `sub` / `mul` |
| `REPEAT` | `0`–`7` — bounded by `REP0`…`REP7` (`mdmacros.h:65-72`) being the only ones defined; `CHOOSE_REP(8)` fails to compile | how many unrolled `STEP_OP` applications `RUN_LOOP` emits in `helper_call`/`main`, and the `n` that `main` passes to `use_generated` | `0` … `7` |

3 × 8 = **24 build configurations**, all of which compile in C and in Rust.

### Runtime axes

| axis | distinct shapes the code distinguishes | why |
|------|----------------------------------------|-----|
| `(a, b)` operand pair | zero, one, positive, negative, mixed sign, `INT_MAX`, `INT_MIN`, overflow-producing pairs, random `i32` | `op_*` are pure arithmetic; only value classes and 32-bit wraparound differ |
| dispatch route | direct call (`op_<OP>`), macro-expanded call (`helper_call`), local function pointer (`helper_ptr`), global function pointer (`G_OP`) | `mdcore.c` deliberately exercises all four spellings of the same call (`:40`, `:47-48`, `:36`) |
| accumulator selector `n` for `use_generated` | `0`, `1`, `2`…`6` (each its own `case`), `7`, `>7`, negative, `INT_MIN`/`INT_MAX` | `DISPATCH_REP` is a `switch` with a `case` per value 0–6 plus `default:` |
| accumulator path | unrolled `RUN_LOOP` (compile-time `REPEAT`) vs `switch`-dispatched `DISPATCH_REP` (runtime `n`) | two independent code paths that must agree for `n == REPEAT ≤ 6` and **disagree** at `REPEAT == 7` |
| `G_OP` mutability | read the load-time value; overwrite with a caller-supplied `extern "C"` pointer and call again | `G_OP` is a non-`const` global |
| `argc` / operand strings (driver) | `argc` 1/2/3/4+; operands empty, whitespace-padded, signed, non-numeric, digit-prefixed, `int`-overflowing | `mdmain.c:29` branches on `argc`; `atoi` on `:33-34` |

### Public entry points (full set, from `mdmacros.h`)

Lowest level first — every row below is driven through the `.so` exports, never
through the convenience path only.

1. `op_add`, `op_sub`, `op_mul` — leaf arithmetic (`mdmacros.h:41-43`)
2. `G_OP` — data symbol, a callable `int (*)(int,int)` (`:104`)
3. `G_OP_NAME` — data symbol, `const char *` (`:105`)
4. `helper_ptr` — leaf + local function pointer (`:109`)
5. `helper_call` — leaf + full `RUN_LOOP` unrolling (`:108`)
6. `use_generated` — the `switch`-dispatched generated accumulator (`:110`)
7. `main` / the `driver` executable — composes all of the above (`mdmain.c`)

## Configuration table

Every row is executed for **all 24 `(OP, REPEAT)` build configurations** and, where
inputs are involved, over **512 seeded-random cases** (`SplitMix64`, seed
`0x5DEECE66D` — see `tests/common/mod.rs`) *plus* the fixed boundary set
`{0, 1, -1, 2, -2, 7, -7, i32::MAX, i32::MIN, i32::MAX-1, i32::MIN+1, 0x7FFF, -0x8000, 65535, 65536}`
crossed with itself. C and Rust are both loaded with `libloading` and compared on
(return value, bytes written to stdout).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `op_add` | all 24 configs × random `(a,b)`; `op_add` is `OP`-independent so it must be identical in every config | [x] |
| 2 | `op_add` | all 24 configs × boundary cross-product (incl. `INT_MAX+1`, `INT_MIN-1` wraparound) | [x] |
| 3 | `op_sub` | all 24 configs × random `(a,b)` | [x] |
| 4 | `op_sub` | all 24 configs × boundary cross-product (incl. `INT_MIN - 1`, `0 - INT_MIN`) | [x] |
| 5 | `op_mul` | all 24 configs × random `(a,b)` | [x] |
| 6 | `op_mul` | all 24 configs × boundary cross-product (incl. `INT_MAX*INT_MAX`, `INT_MIN*-1`) | [x] |
| 7 | `G_OP` (data symbol) | all 24 configs: load the pointer, call it over random + boundary `(a,b)`; result must equal `op_<OP>` for that config's `OP` | [x] |
| 8 | `G_OP` (data symbol) | all 24 configs: overwrite `G_OP` with a caller-supplied `extern "C" fn` and confirm both libraries dispatch to the new target, then restore | [x] |
| 9 | `G_OP_NAME` (data symbol) | all 24 configs: read the `const char *`, compare the NUL-terminated bytes; must be exactly `"add"`/`"sub"`/`"mul"` per `OP`, 3 bytes + NUL | [x] |
| 10 | `helper_ptr` | all 24 configs × random `(a,b)`: return value **and** the `helper.ptr=%d\n` line | [x] |
| 11 | `helper_ptr` | all 24 configs × boundary cross-product (checks `%d` rendering of `INT_MIN`) | [x] |
| 12 | `helper_call` | `OP=add` × `REPEAT=0..7` × random `(a,b)`: return `r+acc` and the `helper.call=%d helper.acc=%d\n` line; `acc` sweeps `0,0,1,3,6,10,15,21` | [x] |
| 13 | `helper_call` | `OP=sub` × `REPEAT=0..7` × random `(a,b)`: `acc` sweeps `0,0,-1,-3,-6,-10,-15,-21` | [x] |
| 14 | `helper_call` | `OP=mul` × `REPEAT=0..7` × random `(a,b)`: `acc` sweeps the factorials `1,1,2,6,24,120,720,5040` (`INIT_mul` is `1`, not `0`) | [x] |
| 15 | `helper_call` | all 24 configs × boundary `(a,b)` incl. overflow: the `r` half wraps while the `acc` half is `REPEAT`-only | [x] |
| 16 | `use_generated` | all 24 configs × `n = 0`: empty unrolling, returns `INIT_FOR(OP)` | [x] |
| 17 | `use_generated` | all 24 configs × `n = 1`: single `STEP_OP(op, acc, 0)` — for `add`/`sub` a no-op (`i == 0`), for `mul` `*1` | [x] |
| 18 | `use_generated` | all 24 configs × `n = 2..6`: one row per `case` arm of `DISPATCH_REP` | [x] |
| 19 | `use_generated` | all 24 configs × `n` swept over `-16..=16`: covers all seven `case`s and the `default:` arm from both sides | [x] |
| 20 | `use_generated` | all 24 configs × 512 random `i32` `n`: catches any value-dependent selector bug | [x] |
| 21 | `use_generated` vs `helper_call` cross-check | all 24 configs: `use_generated(REPEAT)` must equal `helper_call`'s `acc` for `REPEAT ≤ 6` and must **differ** (be `INIT`) at `REPEAT == 7` — the unrolled and `switch` paths agreeing/disagreeing exactly as C does | [x] |
| 22 | all six functions, one loaded library, sequential calls | all 24 configs: drive the library the way `main` does — `op_<OP>`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP` in that order on the same `(a,b)`, comparing the whole concatenated stdout transcript and the `summary` sum | [x] |
| 23 | repeated invocation / statelessness | all 24 configs: call every entry point twice with the same input and once with a different input interleaved; outputs must be identical across repeats (no hidden state in either implementation) | [x] |
| 24 | `driver` executable, `argc == 3` | all 24 configs × operand pairs: numeric, signed, zero, `INT_MAX`/`INT_MIN`, whitespace-padded, non-numeric, `int`-overflowing strings — full stdout+stderr+exit-status compare | [x] |
| 25 | `driver` executable, `argc == 4+` | all 24 configs: extra operands ignored | [x] |
| 26 | build surface | all 24 `(OP, REPEAT)` pairs compile in C **and** in Rust; `cargo check --no-default-features --features <op>,<r>` clean for each | [x] |
| 27 | symbol surface | all 24 configs: `nm -D` defined-symbol sets equal, symbol classes equal, no unresolved non-libc symbols in Rust | [x] |

## Row → test traceability

Every row is driven through `dlopen`/`dlsym` on both `.so` files
(`tests/common/mod.rs`); rows 24–25 additionally run the two `driver`
executables as subprocesses.

| CONFIGS.md row | test |
|---|---|
| 1 | `configs::row01_op_add_random` |
| 2 | `configs::row02_op_add_bounds` |
| 3 | `configs::row03_op_sub_random` |
| 4 | `configs::row04_op_sub_bounds` |
| 5 | `configs::row05_op_mul_random` |
| 6 | `configs::row06_op_mul_bounds` |
| 7 | `configs::row07_g_op_dispatch` |
| 8 | `configs::row_g_op_rebind` |
| 9 | `configs::row09_g_op_name` |
| 10 | `configs::row10_helper_ptr_random`, `stdout_bytes::helper_ptr_line_bytes` |
| 11 | `configs::row11_helper_ptr_bounds` |
| 12–14 | `configs::row12_13_14_helper_call_random`, `configs::row12_13_14_helper_call_acc_table`, `stdout_bytes::helper_call_line_bytes` |
| 15 | `configs::row15_helper_call_bounds` |
| 16 | `configs::row16_use_generated_zero` |
| 17 | `configs::row17_use_generated_one` |
| 18 | `configs::row18_use_generated_cases_2_to_6` |
| 19 | `configs::row19_use_generated_sweep` |
| 20 | `configs::row20_use_generated_random`, `stdout_bytes::gen_acc_line_bytes` |
| 21 | `configs::row21_unrolled_vs_switch` |
| 22 | `configs::row22_full_pipeline_transcript` |
| 23 | `configs::row23_repeated_calls_are_stateless` |
| 24 | `driver_cli::row24_numeric_operands`, `driver_cli::driver_output_shape`, `driver_cli::driver_gen_acc_reflects_dispatch_rep`, `driver_cli::atoi_randomized_strings` |
| 25 | `driver_cli::row08_extra_args_ignored` |
| 26 | `build_surface::row26_this_build_config_is_valid`, `check_features.sh` |
| 27 | `build_surface::row27_symbol_parity`, `build_surface::row27_no_unresolved_symbols`, `check_symbols.sh` |

Harness self-checks, so that a byte comparison cannot pass vacuously:
`stdout_bytes::capture_actually_captures` (fd-1 capture really sees output from
both libraries) and `stdout_bytes::leaf_ops_print_nothing` (the empty-output
baseline is real).

## Result

`./run_all.sh` — 48 tests × 24 `(OP, REPEAT)` combinations = **1 152 test
executions, 0 failures**:

```
add,0 .. mul,7   PASS  (48 tests each)
combinations passed: 24   failed: 0
```

Additional feature sets, all 48 tests passing (`check_features.sh` +
`cargo test`):

| feature set | resolves to | note |
|---|---|---|
| `default` | `add`, `5` | the CMake defaults |
| `--no-default-features` (nothing) | `add`, `5` | matches the `#ifndef OP` / `#ifndef REPEAT` fallback in `mdmacros.h`; verified against `gcc` with no `-D` flags at all |
| `add,sub,mul,5` | `mul`, `5` | over-specified OP; documented precedence `mul > sub > add` |
| `add,0,…,7` | `add`, `0` | over-specified REPEAT; documented precedence `0 > 1 > … > 4 > 6 > 7 > 5` |
| `--all-features` | `mul`, `0` | both axes over-specified |

## Divergence found and fixed

One real divergence surfaced, in `src/cstdlib.rs` (`atoi`, reached from
`mdmain.c:33-34`):

`atoi("-9223372036854775808")` returned **1** in Rust but **0** in C. The Rust
version accumulated the magnitude with `saturating_mul`/`saturating_add`, clamping
at `i64::MAX`, and only then negated. glibc's `strtol` instead accumulates the
magnitude in an `unsigned long` against a *sign-dependent* cutoff — `LONG_MAX`
(2^63 − 1) when positive but `-(unsigned long)LONG_MIN` (2^63) when negative — so
`LONG_MIN` is exactly representable and is not an overflow. Clamping first gave
`(int)(-LONG_MAX) == 1` instead of `(int)LONG_MIN == 0`.

Fixed by reproducing the unsigned accumulation and the asymmetric cutoff. The
C behaviour was confirmed empirically across the boundary before the fix
(`…806 → -2`, `…807 → -1`, `…808 → -1`, `-…807 → 1`, `-…808 → 0`, `-…809 → 0`)
and is now covered by `driver_cli::row10_out_of_range_operands` and
`driver_cli::atoi_randomized_strings`.

## Negative controls

The suite was validated by injecting six divergences into the Rust source and
confirming each is caught (then reverting):

| injected defect | caught by |
|---|---|
| `DISPATCH_REP` gains a `case 7` | `configs::row19_use_generated_sweep`, `row20_use_generated_random` |
| `op_add` saturates instead of wrapping | `configs::row02_op_add_bounds`, `row07_g_op_dispatch`, `row10_helper_ptr_random` |
| `helper.ptr=%d` spelled `helper.ptr = %d` | `configs::row10_helper_ptr_random`, `row11_helper_ptr_bounds`, `row22_full_pipeline_transcript` |
| the original `atoi` magnitude clamp restored | `driver_cli::row10_out_of_range_operands` |
| `G_OP_NAME` set to `"ADD"` | `configs::row09_g_op_name`, `row23_repeated_calls_are_stateless` |
| `op_sub` export renamed away | `build_surface::row27_symbol_parity` (and every `op_sub` row fails to `dlsym`) |
