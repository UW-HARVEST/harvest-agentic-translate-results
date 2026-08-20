# CONFIGS.md — Phase A: configuration-surface table

Derived mechanically from the branches the C code actually takes.

## Axes the C code distinguishes

### Axis 1 — build-time `OP` (`CMakeLists.txt: set(OP ...)`, `mdmacros.h:27`)

Selects, by token pasting, **four** different things at once:

| `OP` | `OP_FN(OP)` = `op_<OP>` | `INIT_FOR(OP)` | `STEP_OP(OP,acc,i)` | `STR(OP)` (`G_OP_NAME`) |
|------|-------------------------|----------------|---------------------|--------------------------|
| `add` | `op_add` (`a + b`) | `0` | `acc += i`       | `"add"` |
| `sub` | `op_sub` (`a - b`) | `0` | `acc -= i`       | `"sub"` |
| `mul` | `op_mul` (`a * b`) | `1` | `acc *= (i + 1)` | `"mul"` |

→ Cargo features `add` / `sub` / `mul`. 3 values (plus "unspecified", which
`mdmacros.h:28` defaults to `add`).

### Axis 2 — build-time `REPEAT` (`CMakeLists.txt: set(REPEAT ...)`, `mdmacros.h:30`)

`RUN_LOOP(op, acc, REPEAT)` → `CHOOSE_REP(REPEAT)` → `REP<REPEAT>`, so only
`REP0 .. REP7` (`mdmacros.h:63-70`) are legal → `REPEAT ∈ 0..=7`. It changes the
unrolled step count used by `helper_call` **and** the `n` that `main` passes to
`use_generated(REPEAT)`.

→ Cargo features `0`..`7` (aliases `repeat_0`..`repeat_7`). 8 values (plus
"unspecified", which `mdmacros.h:31` defaults to `5`).

Axis 1 × Axis 2 = **24 build configurations**, all built by
`scripts/build_artifacts.sh` and all exercised by every row below.

### Axis 3 — public entry points

Full set, from `mdmacros.h:40-42`, `:104-105`, `:108-110` and `mdmain.c`.
The *lowest-level* ones (`op_add`/`op_sub`/`op_mul`, and the raw `G_OP` /
`G_OP_NAME` data objects) are driven directly, not only through the
`helper_*` wrappers:

`op_add`, `op_sub`, `op_mul`, `helper_call`, `helper_ptr`, `use_generated`,
`G_OP` (read / call-through / **write** — it is a writable `.data` object),
`G_OP_NAME` (read), and `main` (the composed pipeline, via the executables).

### Axis 4 — input shapes the code distinguishes

* `(a, b)` for the binary entry points: `(0,0)`; one operand `0`; both positive;
  both negative; mixed sign; `±1` (identity/absorbing for `mul`); `INT_MAX`,
  `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`; overflow-producing pairs
  (`INT_MAX+1`, `INT_MIN-1`, `INT_MAX*INT_MAX`, `46341*46341`, `65536*65536`);
  uniformly random full-range `i32`.
* `n` for `use_generated`: every in-`switch` value `0,1,2,3,4,5,6`
  (`mdmacros.h:84-90` — seven distinct `case`s, so seven distinct shapes),
  and everything that lands on `default:` (see `ERRORS.md` rows 4–10).
* `argv` for `main`: `argc` = 0,1,2,3,4; decimal, whitespace-prefixed, signed,
  non-numeric, partially numeric, empty, `> INT_MAX`, `> LONG_MAX`,
  hex/octal/exponent-looking.

## Configuration table

Every row is executed for **all 24** `(OP, REPEAT)` configurations, and every row
marked "randomized" uses ≥ 512 pseudo-random inputs from a fixed-seed
xorshift64\* generator (seeded per row, so the sequence is reproducible).
"C↔Rust" means: load both `.so`s with `libloading` and compare the return value
**and** the captured stdout bytes.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `op_add` | all 24 cfgs × 41-value edge-value cross product (`0, ±1, ±2, ±3, 46341, ±65536, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1, …`) — 1681 pairs | `b04_op_fns_edge_cross_product` | [x] |
| 2 | `op_sub` | idem | `b04_op_fns_edge_cross_product` | [x] |
| 3 | `op_mul` | idem, including every overflow-producing pair | `b04_op_fns_edge_cross_product` | [x] |
| 4 | `op_add` | randomized full-range `i32` pairs (4096) | `b01_op_add_randomized` | [x] |
| 5 | `op_sub` | randomized full-range `i32` pairs (4096) | `b02_op_sub_randomized` | [x] |
| 6 | `op_mul` | randomized full-range `i32` pairs (4096) | `b03_op_mul_randomized` | [x] |
| 7 | `helper_call` | selected `OP` × selected `REPEAT`, edge-value `(a,b)` cross product (41×41): checks the returned `r + acc` **and** the exact `helper.call=%d helper.acc=%d\n` line | `b05_helper_call_edge_cross_product` | [x] |
| 8 | `helper_call` | selected `OP` × selected `REPEAT`, randomized full-range `(a,b)` (2048), return + stdout | `b06_helper_call_randomized` | [x] |
| 9 | `helper_ptr` | edge-value `(a,b)` cross product (41×41), return + `helper.ptr=%d\n` | `b07_helper_ptr_edge_cross_product` | [x] |
| 10 | `helper_ptr` | randomized full-range `(a,b)` (2048), return + stdout | `b08_helper_ptr_randomized` | [x] |
| 11 | `use_generated` | `n = 0` (`case 0:` → `REP0`, empty body) | `b09_use_generated_each_switch_case` | [x] |
| 12 | `use_generated` | `n = 1` (`case 1:` → `REP1`) | `b09_use_generated_each_switch_case` | [x] |
| 13 | `use_generated` | `n = 2` (`case 2:` → `REP2`) | `b09_use_generated_each_switch_case` | [x] |
| 14 | `use_generated` | `n = 3` (`case 3:` → `REP3`) | `b09_use_generated_each_switch_case` | [x] |
| 15 | `use_generated` | `n = 4` (`case 4:` → `REP4`) | `b09_use_generated_each_switch_case` | [x] |
| 16 | `use_generated` | `n = 5` (`case 5:` → `REP5`) | `b09_use_generated_each_switch_case` | [x] |
| 17 | `use_generated` | `n = 6` (`case 6:` → `REP6`) | `b09_use_generated_each_switch_case` | [x] |
| 18 | `use_generated` | randomized `n` uniformly over the full `int` range (4096) — mixes in-`switch` and `default:` shapes | `b10_use_generated_randomized_full_range` | [x] |
| 19 | `use_generated` | `n == REPEAT` for the selected `REPEAT`, i.e. exactly what `main` passes (pins the `REPEAT=7` ⇒ `default:` asymmetry) | `b11_use_generated_at_repeat` | [x] |
| 20 | `G_OP` (read) | read the `.data` word from both `.so`s; assert it equals that library's own `op_<OP>` address (i.e. `OP_FN(OP)` resolved identically) | `b12_g_op_points_at_selected_op` | [x] |
| 21 | `G_OP` (call through) | call through the loaded pointer with the 41-value edge cross product + 2048 randomized pairs | `b13_g_op_call_through` | [x] |
| 22 | `G_OP` (**write** then call) | store each of `op_add`, `op_sub`, `op_mul` (resolved from the *same* library) into the writable `G_OP` object, call through it, restore — proves the object is writable in both and dispatches identically | `b14_g_op_writable_then_call_through` | [x] |
| 23 | `G_OP_NAME` (read) | read the `char*` and compare the NUL-terminated bytes; must be exactly `"add"`/`"sub"`/`"mul"` for the selected `OP` | `b15_g_op_name_bytes` | [x] |
| 24 | composed pipeline over the `.so` exports | replicate `mdmain.c`'s body through the two `.so`s: `G_OP(a,b)`, `RUN_LOOP` accumulator, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP(a,b)` again, then the two `printf` lines and the `summary` — one captured byte stream per input, edge cross product + 1024 randomized pairs | `b16_composed_pipeline_like_main` | [x] |
| 25 | call-ordering / buffering | 256 randomized *sequences* of interleaved `helper_call` / `helper_ptr` / `use_generated` calls captured as **one** stdout stream, to catch output-ordering and buffering divergence | `b17_interleaved_call_sequences` | [x] |
| 26 | `main` (executable) | `argc == 3`, randomized decimal argument pairs (512) — stdout + stderr + exit status | `b18_main_randomized_decimal_args` | [x] |
| 27 | `main` (executable) | `argc == 3`, edge-value decimal arguments (`0`, `±1`, `INT_MAX`, `INT_MIN`, `2147483648`, `-2147483649`, …) | `b19_main_edge_decimal_args` | [x] |
| 28 | `main` (executable) | `argc == 3`, `atoi` input *shapes*: every C whitespace byte (` `, `\t`, `\n`, `\v`, `\f`, `\r`) as a prefix, `+`/`-` signs, doubled signs, leading zeros, mixed alnum, embedded space, empty string, `0x`/`0b`/octal/exponent/underscore forms, `INT`/`LONG` boundaries -- 47x47 = 2209 argument pairs | `b20_main_atoi_input_shapes` | [x] |
| 29 | `main` (executable) | `argc == 4, 5` — surplus arguments present but unused | `c15_extra_args_ignored` | [x] |
| 30 | `main` (executable) | `argv[0]` and/or the two numeric arguments contain non-UTF-8 bytes (raw `argv` fidelity) | `c21_non_utf8_argv` | [x] |
| 31 | all printing entry points + `main` | `stdout` is unwritable (`/dev/full`) / is `/dev/null` | `c22_unwritable_stdout` | [x] |
| 32 | build-time defaults | `--no-default-features --features <op>` (no `REPEAT` feature) must behave like `-DREPEAT` undefined ⇒ `5`; `--features <n>` (no `OP` feature) must behave like `-DOP` undefined ⇒ `add`; `--features default` ⇒ `add,5` | `scripts/run_all.sh` "default-fallback" stage | [x] |
| 33 | feature aliases | `repeat_0`..`repeat_7` must select the same `REPEAT` as the bare `0`..`7` features | `scripts/run_all.sh` "alias" stage | [x] |

## Coverage of the `(OP, REPEAT)` cross product

Rows 1–31 are each run 24 times (once per `(OP, REPEAT)`), which covers the full
Axis 1 × Axis 2 cross product; rows 32–33 cover the "cache variable not set" and
alias-spelling forms of the same two axes (36 extra `cargo` configurations, all
compared against the matching C build). `scripts/run_all.sh` drives all of it.
