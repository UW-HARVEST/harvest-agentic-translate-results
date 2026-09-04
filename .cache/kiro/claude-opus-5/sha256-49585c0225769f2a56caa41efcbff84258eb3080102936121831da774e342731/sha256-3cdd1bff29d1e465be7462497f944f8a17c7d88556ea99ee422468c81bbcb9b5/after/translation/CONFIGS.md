# CONFIGS.md — configuration-surface table (Phase B)

## Axes, derived mechanically from `c_src/`

### Build-time axes (the only "options" this library has)

`c_src/CMakeLists.txt` defines exactly two cache variables, injected as `-D`
into `CMAKE_C_FLAGS`:

| axis | source | values | what it selects in the C |
|------|--------|--------|--------------------------|
| `OP` | `CMakeLists.txt:26`, `mdmacros.h:27-29` | `add`, `sub`, `mul` | `OP_FN(OP)`→`op_<OP>` (`mdmacros.h:44`), `INIT_FOR(OP)`→`INIT_<OP>` = `0/0/1` (`:56-58`), `STEP_OP`→`STEP_<OP>` = `+= i` / `-= i` / `*= (i+1)` (`:47-49`), `STR(OP)`→`G_OP_NAME` (`mdcore.c:37`), `DEFINE_ACCUM(OP)`→`accum_<OP>` (`:96`) |
| `REPEAT` | `CMakeLists.txt:27`, `mdmacros.h:30-32` | `0`–`7` | `CHOOSE_REP(REPEAT)`→`REP<REPEAT>` (`:73-74`), i.e. how many unrolled `STEP_OP`s `RUN_LOOP` emits inside `helper_call` and `main`; also the `n` that `main` passes to `use_generated` |

Any other value is a **compile error** (see `ERRORS.md`, build-time section), so
the configuration space is exactly the 3 × 8 = **24** combinations. Cargo mirrors
them as `--features <add|sub|mul>,<repeat_0..repeat_7>` (with `"0".."7"` as
aliases of `repeat_0..repeat_7`).

**Every row below is executed under all 24 build configurations** by
`cbuild/run_all.sh`, which rebuilds the Rust `.so`/binary per feature combo and
points the tests at the matching `cbuild/libcmd_${op}_${rep}.so`. There is no
runtime option/flag/mode in this API — the mutable global `G_OP` is the only
piece of caller-settable state (rows 22–24).

### Public entry points (the full set, lowest level first)

From `mdmacros.h` + `nm -D`: `op_add`, `op_sub`, `op_mul` (lowest level, leaves),
then `helper_ptr` and `helper_call` (call the leaf through a token-pasted name /
a function pointer and fold in `RUN_LOOP`), then `use_generated` (drives the
`static accum_<OP>` `switch`), plus the two data symbols `G_OP` / `G_OP_NAME`,
plus `main` in `mdmain.c` which composes all of them. All are driven directly,
not only through the composed `main`.

### Input-shape axes

* `(a, b)` for the three binary entry points and the two helpers: `0`, `±1`,
  small, `INT_MAX`, `INT_MIN`, overflow-producing pairs, and seeded-random
  full-range `i32` (256 pairs/row, `splitmix64`, fixed seed `0x5EED_1234_ABCD_9876`).
* `n` for `use_generated`: each `case` of `DISPATCH_REP` is a *separate* code
  path, so `0,1,2,3,4,5,6` each get their own row; `default:` gets rows 18–19.
* argv strings for the `main` row group: well-formed, signed, whitespace-padded,
  trailing-junk, empty, non-numeric, out-of-`int`-range.

## Table

`INIT` = `INIT_<OP>` (`0` for add/sub, `1` for mul). `step(acc,i)` =
`acc+i` / `acc-i` / `acc*(i+1)`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `op_add` | all 24 build cfgs (behaviour must be OP/REPEAT-independent) × identity/small shapes: `(0,0) (0,1) (1,0) (1,-1) (-1,-1) (7,3)` | [x] |
| 2 | `op_add` | all 24 cfgs × 256 seeded-random full-range `i32` pairs | [x] |
| 3 | `op_add` | all 24 cfgs × boundary shapes `(INT_MAX,0) (INT_MAX,1) (INT_MIN,-1) (INT_MAX,INT_MAX) (INT_MIN,INT_MIN) (INT_MAX,INT_MIN)` | [x] |
| 4 | `op_sub` | all 24 cfgs × identity/small shapes | [x] |
| 5 | `op_sub` | all 24 cfgs × 256 seeded-random pairs | [x] |
| 6 | `op_sub` | all 24 cfgs × boundary shapes `(INT_MIN,1) (INT_MAX,-1) (INT_MIN,INT_MAX) (0,INT_MIN)` | [x] |
| 7 | `op_mul` | all 24 cfgs × identity/small shapes `(0,x) (1,x) (-1,x) (3,5)` | [x] |
| 8 | `op_mul` | all 24 cfgs × 256 seeded-random pairs | [x] |
| 9 | `op_mul` | all 24 cfgs × boundary shapes `(INT_MAX,2) (INT_MIN,-1) (65536,65536) (INT_MAX,INT_MAX) (INT_MIN,INT_MIN)` | [x] |
| 10 | `helper_ptr` | all 24 cfgs × identity/small `(a,b)` — exercises `int (*fp)(int,int) = OP_FN(OP)` indirect call | [x] |
| 11 | `helper_ptr` | all 24 cfgs × 256 seeded-random pairs | [x] |
| 12 | `helper_ptr` | all 24 cfgs × overflow boundary pairs | [x] |
| 13 | `helper_call` | all 24 cfgs × identity/small `(a,b)` — return is `OP_FN(OP)(a,b) + RUN_LOOP(INIT, REPEAT)`, so this row is where the `OP` × `REPEAT` cross-product actually bites | [x] |
| 14 | `helper_call` | all 24 cfgs × 256 seeded-random pairs | [x] |
| 15 | `helper_call` | all 24 cfgs × overflow boundary pairs (`r + acc` wrap) | [x] |
| 16 | `helper_call` | all 24 cfgs × cross-check decomposition: result == `op_<OP>(a,b) + (helper_call(0,0) - op_<OP>(0,0))` for random `a,b`, pinning the unrolled `acc` independently of `a,b` | [x] |
| 17 | `use_generated` | all 24 cfgs × `n ∈ {0,1,2,3,4,5,6}` — one `DISPATCH_REP` `case` each (`REP0` empty … `REP6` six steps) | [x] |
| 18 | `use_generated` | all 24 cfgs × `n = 7` (first value past the last `case` ⇒ `default:`) | [x] |
| 19 | `use_generated` | all 24 cfgs × `n ∈ {8, 9, 100, 65536, -1, -7, INT_MIN, INT_MAX}` ⇒ `default:` | [x] |
| 20 | `use_generated` | all 24 cfgs × 256 seeded-random full-range `i32` values of `n` | [x] |
| 21 | `use_generated` | all 24 cfgs × `n = REPEAT` (exactly what `main` does; note `REPEAT=7` lands on `default:`) | [x] |
| 22 | `G_OP` (read) + `op_add`/`op_sub`/`op_mul` | all 24 cfgs: call through the `G_OP` global and assert it equals `op_<OP>` for identity/small/boundary/256 random pairs | [x] |
| 23 | `G_OP` (write) | all 24 cfgs × store each of `op_add`,`op_sub`,`op_mul` into the exported global, then call through it (random pairs); also assert `helper_call`/`helper_ptr` are **unaffected** because they use `OP_FN(OP)` directly, not `G_OP` | [x] |
| 24 | `G_OP_NAME` (read + write) | all 24 cfgs: NUL-terminated string equals `STR(OP)`; then repoint the global at a caller-owned string and read it back | [x] |
| 25 | `helper_call` stdout | all 24 cfgs × identity/small/random `(a,b)`: captured stdout bytes must equal `helper.call=<r> helper.acc=<acc>\n` identically for C and Rust | [x] |
| 26 | `helper_ptr` stdout | all 24 cfgs × identity/small/random `(a,b)`: captured stdout equals `helper.ptr=<r>\n` | [x] |
| 27 | `use_generated` stdout | all 24 cfgs × `n ∈ 0..=8, -1, INT_MIN, INT_MAX`: captured stdout equals `gen.acc=<r>\n` | [x] |
| 28 | `main` (`driver` executable) | all 24 cfgs × valid operand pairs `0 0`, `7 3`, `-5 9`, `2147483647 1`, `-2147483648 -1`, plus 64 seeded-random pairs: full stdout+stderr+exit-status byte comparison (covers `printf("op=%s call=%d acc=%d g.call=%d")` and `printf("summary=%d")`) | [x] |
| 29 | `main` (`driver` executable) | all 24 cfgs × operand *string shapes*: `"  12"`, `"+12"`, `"-12"`, `"12abc"`, `"abc"`, `""`, `"0x10"`, `"007"`, `"2147483648"`, `"-2147483649"`, `"99999999999999999999"` — `atoi` behaviour, still exit 0 | [x] |
| 30 | `main` (`driver` executable) | all 24 cfgs × `argv[0]`-sensitive invocation (relative vs absolute path) with valid operands, checking the `op=<name>` line carries `STR(OP)` | [x] |

## Where each row lives

| rows | test file | mechanism |
|------|-----------|-----------|
| 1–21 | `tests/differential.rs` | `dlopen` both `.so`s, compare return values |
| 22–24 | `tests/globals.rs` | read *and write* the exported `.data` objects |
| 25–27 | `tests/stdout_capture.rs` | `dup2` fd 1 into a temp file around each call, compare the emitted bytes |
| 28–30 | `tests/driver_cli.rs` | spawn both `driver` executables with an identical `argv` (`arg0` set explicitly) and compare stdout + stderr + exit status |

`tests/common/mod.rs` picks the C `.so` from the *test binary's own* Cargo
features (`libcmd_<op>_<repeat>.so`) using the direct `repeat_N -> N` mapping, on
purpose: it does not reuse `src/mdmacros.rs`'s cfg-priority chain, so a bug in
that chain surfaces as a C/Rust divergence rather than cancelling out.

Randomised inputs come from a seeded `splitmix64`
(`SEED = 0x5EED_1234_ABCD_9876`, per-row salts), 256 values per row for the leaf
and helper rows.

## Verification evidence

```
$ ./cbuild/build_c.sh                     # 24 C .so + 24 C driver executables
$ ./cbuild/run_all.sh                     # 42 tests x 24 (OP, REPEAT) combos
PASS OP=add REPEAT=0  (42 tests, 7 suites)
...
PASS OP=mul REPEAT=7  (42 tests, 7 suites)
configurations passed: 24   failed: 0

$ PROFILE=release ./cbuild/run_all.sh
configurations passed: 24   failed: 0
```

Also verified separately:

* the feature *aliases* `"0".."7"` (`--features add,5`, `--features 3`, …) — 42/42;
* the **no-feature** build (`cargo check/test --no-default-features` with neither
  an `OP` nor a `REPEAT` feature), which must reproduce `mdmacros.h`'s
  `#ifndef OP → add` / `#ifndef REPEAT → 5` defaults — 42/42 against
  `libcmd_add_5.so`;
* the default feature set (`add,repeat_5`) — 42/42;
* a `-O0` C build for `{add,sub,mul} x {0,5,7}` — 42/42 each, confirming the
  comparisons do not depend on the C optimisation level.

### Non-vacuity (fault injection)

Faults injected into the Rust and then reverted, with the number of Phase-B/C
tests that failed as a result (0 would mean the row was untested):

| injected fault | tests failed |
|----------------|--------------|
| `run_loop` uses `i <= REPEAT` | 14 |
| `accum` handles `n == 7` instead of falling to `default:` | 8 |
| `OP_NAME` misspelled `"Add"` | 6 |
| `STEP_mul` uses `acc *= i` | 17 |
| `G_OP` reverted to an immutable `static` | 1 (`globals.rs`; a SIGSEGV before the writability pre-flight was added) |
