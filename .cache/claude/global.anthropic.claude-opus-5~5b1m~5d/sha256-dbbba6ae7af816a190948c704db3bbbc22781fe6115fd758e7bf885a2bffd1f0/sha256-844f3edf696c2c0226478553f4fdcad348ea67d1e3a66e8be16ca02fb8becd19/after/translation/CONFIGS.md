# CONFIGS.md — Configuration surface (valid inputs)

## Axes the C code actually branches on

Derived from `c_src/CMakeLists.txt`, `c_src/src/mdmacros.h` and the `#ifdef` /
`switch` / token-paste sites in `c_src/src/mdcore.c`:

### Build-time axes (the *only* "options" this API has — there is no runtime setter)

| axis | source | domain | what it toggles |
|------|--------|--------|-----------------|
| `OP` | `CMakeLists.txt:26` `set(OP "add" …)`; `mdmacros.h:27` fallback `add` | `add`, `sub`, `mul` | `OP_FN(OP)` → which `op_*` is called; `STEP_<OP>` (`+=i` / `-=i` / `*=(i+1)`); `INIT_<OP>` (`0`/`0`/`1`); `STR(OP)` → `G_OP_NAME`; the name of the generated `accum_<OP>` |
| `REPEAT` | `CMakeLists.txt:27` `set(REPEAT "5" …)`; `mdmacros.h:30` fallback `5` | `0`..`7` (`REP0`..`REP7` are the only ones defined) | `RUN_LOOP` → the unrolled `REP<REPEAT>` body used by `helper_call` and by `main`; also the argument `main` passes to `use_generated(REPEAT)` |

Cross-product: **3 × 8 = 24** build configurations. In the Rust crate these are
the Cargo features `add`/`sub`/`mul` × `"0"`..`"7"`.

### Input-shape axes (per entry point)

| entry point | signature | shapes the code distinguishes |
|-------------|-----------|-------------------------------|
| `op_add` / `op_sub` / `op_mul` | `int(int,int)` | zero, positive, negative, `INT_MIN`, `INT_MAX`, overflowing pairs, randomized full-range |
| `helper_call` | `int(int,int)` | same as above; additionally the *return* composes `OP_FN(a,b) + REP<REPEAT>` so it depends on `REPEAT` too, **and it prints two values** |
| `helper_ptr` | `int(int,int)` | same as above, dispatched through a local function pointer; **prints one value** |
| `use_generated` | `int(int)` | `n` ∈ `switch` domain `0..=6` (7 distinct in-range shapes) **and** the `default` shape (`n<0`, `n>6`) — see ERRORS.md |
| `G_OP` (data slot) | `int(*)(int,int)` | dereferenced and called; must equal the `op_<OP>` address for the selected `OP` |
| `G_OP_NAME` (data slot) | `const char *` | dereferenced as a NUL-terminated string; must equal `STR(OP)` |
| `main` (whole program) | `argv` | `argc` = 1/2/3/4; operands: decimal, signed, leading blanks, trailing garbage, empty, `INT_MIN`/`INT_MAX`, out-of-`long` values |

## Configuration table

One row per meaningful combination the C treats differently. `<op>` ranges over
`add`, `sub`, `mul`; `<r>` over `0..7`. Every row is exercised for **all 24**
`(OP, REPEAT)` builds by the differential tests, with many randomized inputs per
row (fixed seed, SplitMix64) — not a single hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | ✔ |
|----|----------------|--------------------------------------------|---|
| 1  | `op_add` | any `(OP,REPEAT)` build — `op_add` is defined unconditionally; 512 randomized full-range `(a,b)` pairs | [x] |
| 2  | `op_add` | boundary operands: `(0,0)`, `(0,±1)`, `(INT_MAX,1)`, `(INT_MIN,-1)`, `(INT_MAX,INT_MAX)`, `(INT_MIN,INT_MIN)` | [x] |
| 3  | `op_sub` | any build; 512 randomized full-range `(a,b)` pairs | [x] |
| 4  | `op_sub` | boundary operands incl. `(INT_MIN,1)`, `(INT_MAX,-1)`, `(INT_MIN,INT_MAX)` | [x] |
| 5  | `op_mul` | any build; 512 randomized full-range `(a,b)` pairs | [x] |
| 6  | `op_mul` | boundary operands incl. `(INT_MIN,-1)`, `(INT_MAX,INT_MAX)`, `(INT_MIN,INT_MIN)`, `(x,0)`, `(x,1)` | [x] |
| 7  | `G_OP` data slot | `OP=add` build → slot must equal `dlsym("op_add")`, and calling through it must match `op_add` on randomized inputs | [x] |
| 8  | `G_OP` data slot | `OP=sub` build → slot must equal `dlsym("op_sub")`; call parity on randomized inputs | [x] |
| 9  | `G_OP` data slot | `OP=mul` build → slot must equal `dlsym("op_mul")`; call parity on randomized inputs | [x] |
| 10 | `G_OP_NAME` data slot | `OP=add` → C string `"add"` (byte-for-byte incl. NUL) | [x] |
| 11 | `G_OP_NAME` data slot | `OP=sub` → C string `"sub"` | [x] |
| 12 | `G_OP_NAME` data slot | `OP=mul` → C string `"mul"` | [x] |
| 13 | `helper_ptr` | every `(OP,REPEAT)`; 256 randomized `(a,b)` — return value parity | [x] |
| 14 | `helper_ptr` | every `(OP,REPEAT)`; **captured stdout** must be byte-identical (`helper.ptr=<r>\n`), incl. negative and `INT_MIN` results | [x] |
| 15 | `helper_call` | `OP=add`, `REPEAT` = 0,1,2,3,4,5,6,7 (each build) — return `= a+b + (0+1+…+r-1)`; 256 randomized `(a,b)` | [x] |
| 16 | `helper_call` | `OP=sub`, `REPEAT` = 0..7 — return `= a-b − (0+1+…+r-1)`; 256 randomized `(a,b)` | [x] |
| 17 | `helper_call` | `OP=mul`, `REPEAT` = 0..7 — return `= a*b + r!` (acc starts at `INIT_mul=1`); 256 randomized `(a,b)` | [x] |
| 18 | `helper_call` | every build; **captured stdout** byte-identical (`helper.call=<r> helper.acc=<acc>\n`) | [x] |
| 19 | `use_generated` | every build; `n` = 0 (`REP0`, empty body → returns `INIT`) | [x] |
| 20 | `use_generated` | every build; `n` = 1,2,3,4,5 (each `switch` case, i.e. each `REPn` unroll depth) | [x] |
| 21 | `use_generated` | every build; `n` = 6 (highest in-range case) | [x] |
| 22 | `use_generated` | every build; `n` swept over the full `-8..=16` window plus 256 randomized `i32` values — covers in-range, boundary and `default:` shapes in one property test | [x] |
| 23 | `use_generated` | every build; **captured stdout** byte-identical (`gen.acc=<r>\n`) for all `n` in row 22 | [x] |
| 24 | composed pipeline (low-level, driven like `main`) | every build: `G_OP(a,b)` → `helper_call(a,b)` → `helper_ptr(a,b)` → `use_generated(REPEAT)`, all four calls in `main`'s order on the *same* library handle, summing the results exactly as `mdmain.c:46` does; 128 randomized `(a,b)`. Verifies call-order-dependent state and the interleaved stdout of the whole sequence | [x] |
| 25 | composed pipeline, stdout | every build; the concatenated stdout of the row-24 sequence must be byte-identical — exactly 3 lines (`helper.call=…`, `helper.ptr=…`, `gen.acc=…`) in that order | [x] |
| 26 | whole program (`driver` binary) | every build × argument vectors: `3 4`, `0 0`, `-5 9`, `INT_MAX 1`, `INT_MIN -1`, `INT_MAX INT_MAX`, `+8 -8`, extra 4th arg — stdout + stderr + exit status byte-identical | [x] |
| 27 | default configuration | no `-DOP` / `-DREPEAT` at all (C `#ifndef` fallbacks) vs `cargo` `--features add,5`, i.e. Cargo `default` — must equal the explicit `OP=add REPEAT=5` build | [x] |
| 28 | Rust-only degenerate feature sets | no OP feature / no REPEAT feature / conflicting OP features / conflicting REPEAT features — must still compile and must resolve to the documented priority (`add > sub > mul`, lowest `REPEAT` wins) | [x] |

## Where each row is tested

`tests/valid_paths.rs`, test names carry the row numbers
(`row01_row02_op_add`, `row07_row09_g_op_slot_points_at_selected_op`, …).
Every call is made through `dlopen`/`dlsym` on the C `.so` *and* the Rust `.so`;
the Rust crate is never invoked directly, so the `#[no_mangle]` export wrappers
are part of what is compared.

Run the whole table over every configuration with:

```sh
bash scripts/test_all_features.sh             # the 24 canonical (OP, REPEAT) builds
bash scripts/test_all_features.sh degenerate  # + empty / conflicting feature sets
```

Result: **28/28 rows pass, for all 24 canonical configurations and all 14
degenerate feature sets.**
