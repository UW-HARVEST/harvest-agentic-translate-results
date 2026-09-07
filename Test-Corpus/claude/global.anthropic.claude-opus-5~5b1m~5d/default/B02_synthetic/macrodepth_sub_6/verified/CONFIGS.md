# CONFIGS.md — configuration-surface table (Phase A, tested in Phase B)

## Axes, derived from what the C actually branches on

Everything configurable in this library is *build-time*: there is no runtime
option, mode or flag in the public API (`mdmacros.h` exposes only `op_add`,
`op_sub`, `op_mul`, `G_OP`, `G_OP_NAME`, `helper_call`, `helper_ptr`,
`use_generated`; none of them takes a mode argument).

| axis | source of truth | values |
|------|-----------------|--------|
| `OP` | `CMakeLists.txt:26` `set(OP "add" CACHE STRING ...)`; `mdmacros.h:27` `#ifndef OP / #define OP add`; token-pasted by `OP_FN`, `STEP_<OP>`, `INIT_<OP>`, `DEFINE_ACCUM`, `STR(OP)` | `add`, `sub`, `mul` (Cargo features `add`/`sub`/`mul`) |
| `REPEAT` | `CMakeLists.txt:27` `set(REPEAT "5" ...)`; `mdmacros.h:30` `#ifndef REPEAT / #define REPEAT 5`; selects `REP<REPEAT>` via `CHOOSE_REP` | `0..7` (`REP0`..`REP7` exist; `REP8` does not → build failure). Cargo features `"0"`..`"7"` |
| `INIT_FOR(OP)` | `mdmacros.h:56-58` | `0` for add/sub, `1` for mul — a real behavioural branch, not just a constant |
| `STEP_<OP>` | `mdmacros.h:48-50` | `acc+=i` / `acc-=i` / `acc*=(i+1)` |
| unroll depth reached | `REP0..REP7` chain (`mdmacros.h:63-70`) | 0..7 steps with `i = 0..REPEAT-1` |
| `DISPATCH_REP` case reached | `mdmacros.h:82-93` `switch (n)` with `case 0..6` + `default` | `n` in `0..=6` (accepted) vs. anything else (`default: break`) — an input-*shape* axis of `use_generated` independent of `REPEAT` |
| operand shape | no branches on operand values; but signed wraparound makes value classes distinct | `0`, small +, small -, `INT_MAX`, `INT_MIN`, `INT_MAX/2`, random full-range `int`, pairs that overflow |
| `argv` shape (driver only) | `mdmain.c:29` `argc < 3`, `atoi` | 0/1/2/3+ args; numeric, non-numeric, empty, signed, whitespace, overflowing |

Cross-product: `3 OP x 8 REPEAT = 24` build configurations. Every configuration
is built for both languages (`build_c.sh`, `build_rust.sh`) and every row below
is exercised for its configuration.

## Entry points (all of them, lowest level first)

`op_add` / `op_sub` / `op_mul` (leaf) -> `G_OP` / `G_OP_NAME` (globals initialised
from macro expansion at load time) -> `helper_ptr` (indirect call) ->
`helper_call` (`OP_FN` + `RUN_LOOP`/`REP<REPEAT>` unrolling) -> `use_generated`
(the `static accum_<OP>` + `DISPATCH_REP` switch) -> `driver` `main` (the whole
composed pipeline: `OP_FN`, `RUN_LOOP`, all three helpers, `G_OP`, `G_OP_NAME`).

All `.so` rows call BOTH the C and the Rust library through `libloading` +
`dlsym` and compare the returned `int` **and** the bytes printed on stdout.
Every row uses a fixed-seed PRNG (`SplitMix64`, seed `0x5eed_1234_dead_beef`)
with many inputs per row, plus the boundary set
`{0, 1, -1, 2, -2, 7, 42, INT_MAX, INT_MIN, INT_MAX/2, INT_MIN/2, 65536}`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 0 | build matrix | all 24 `(OP,REPEAT)` C `.so`s + Rust `.so`s exist, export identical symbol sets, and `REPEAT=8` is rejected at build time in both | [x] |
| 1 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=0` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 2 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=0` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 3 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=0` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 4 | `helper_ptr` | `OP=add`, `REPEAT=0` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 5 | `helper_call` | `OP=add`, `REPEAT=0` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP0`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 6 | `use_generated` | `OP=add`, `REPEAT=0` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 7 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=0` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 8 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=1` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 9 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=1` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 10 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=1` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 11 | `helper_ptr` | `OP=add`, `REPEAT=1` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 12 | `helper_call` | `OP=add`, `REPEAT=1` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP1`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 13 | `use_generated` | `OP=add`, `REPEAT=1` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 14 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=1` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 15 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=2` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 16 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=2` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 17 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=2` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 18 | `helper_ptr` | `OP=add`, `REPEAT=2` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 19 | `helper_call` | `OP=add`, `REPEAT=2` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP2`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 20 | `use_generated` | `OP=add`, `REPEAT=2` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 21 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=2` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 22 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=3` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 23 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=3` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 24 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=3` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 25 | `helper_ptr` | `OP=add`, `REPEAT=3` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 26 | `helper_call` | `OP=add`, `REPEAT=3` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP3`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 27 | `use_generated` | `OP=add`, `REPEAT=3` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 28 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=3` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 29 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=4` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 30 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=4` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 31 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=4` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 32 | `helper_ptr` | `OP=add`, `REPEAT=4` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 33 | `helper_call` | `OP=add`, `REPEAT=4` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP4`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 34 | `use_generated` | `OP=add`, `REPEAT=4` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 35 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=4` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 36 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=5` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 37 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=5` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 38 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=5` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 39 | `helper_ptr` | `OP=add`, `REPEAT=5` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 40 | `helper_call` | `OP=add`, `REPEAT=5` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP5`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 41 | `use_generated` | `OP=add`, `REPEAT=5` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 42 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=5` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 43 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=6` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 44 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=6` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 45 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=6` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 46 | `helper_ptr` | `OP=add`, `REPEAT=6` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 47 | `helper_call` | `OP=add`, `REPEAT=6` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP6`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 48 | `use_generated` | `OP=add`, `REPEAT=6` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 49 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=6` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 50 | `op_add`, `op_sub`, `op_mul` | `OP=add`, `REPEAT=7` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 51 | `G_OP` (read global, then call through it) | `OP=add`, `REPEAT=7` — pointer read from `.data` must equal the `op_add` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 52 | `G_OP_NAME` (read global) | `OP=add`, `REPEAT=7` — `STR(OP)` NUL-terminated bytes must be `"add"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 53 | `helper_ptr` | `OP=add`, `REPEAT=7` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 54 | `helper_call` | `OP=add`, `REPEAT=7` — `OP_FN(add)` + `RUN_LOOP` unrolled to `REP7`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 55 | `use_generated` | `OP=add`, `REPEAT=7` — `static accum_add` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 56 | `driver` `main` (whole pipeline) | `OP=add`, `REPEAT=7` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 57 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=0` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 58 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=0` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 59 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=0` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 60 | `helper_ptr` | `OP=sub`, `REPEAT=0` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 61 | `helper_call` | `OP=sub`, `REPEAT=0` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP0`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 62 | `use_generated` | `OP=sub`, `REPEAT=0` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 63 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=0` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 64 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=1` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 65 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=1` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 66 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=1` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 67 | `helper_ptr` | `OP=sub`, `REPEAT=1` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 68 | `helper_call` | `OP=sub`, `REPEAT=1` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP1`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 69 | `use_generated` | `OP=sub`, `REPEAT=1` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 70 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=1` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 71 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=2` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 72 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=2` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 73 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=2` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 74 | `helper_ptr` | `OP=sub`, `REPEAT=2` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 75 | `helper_call` | `OP=sub`, `REPEAT=2` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP2`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 76 | `use_generated` | `OP=sub`, `REPEAT=2` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 77 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=2` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 78 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=3` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 79 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=3` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 80 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=3` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 81 | `helper_ptr` | `OP=sub`, `REPEAT=3` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 82 | `helper_call` | `OP=sub`, `REPEAT=3` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP3`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 83 | `use_generated` | `OP=sub`, `REPEAT=3` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 84 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=3` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 85 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=4` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 86 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=4` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 87 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=4` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 88 | `helper_ptr` | `OP=sub`, `REPEAT=4` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 89 | `helper_call` | `OP=sub`, `REPEAT=4` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP4`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 90 | `use_generated` | `OP=sub`, `REPEAT=4` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 91 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=4` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 92 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=5` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 93 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=5` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 94 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=5` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 95 | `helper_ptr` | `OP=sub`, `REPEAT=5` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 96 | `helper_call` | `OP=sub`, `REPEAT=5` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP5`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 97 | `use_generated` | `OP=sub`, `REPEAT=5` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 98 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=5` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 99 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=6` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 100 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=6` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 101 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=6` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 102 | `helper_ptr` | `OP=sub`, `REPEAT=6` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 103 | `helper_call` | `OP=sub`, `REPEAT=6` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP6`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 104 | `use_generated` | `OP=sub`, `REPEAT=6` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 105 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=6` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 106 | `op_add`, `op_sub`, `op_mul` | `OP=sub`, `REPEAT=7` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 107 | `G_OP` (read global, then call through it) | `OP=sub`, `REPEAT=7` — pointer read from `.data` must equal the `op_sub` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 108 | `G_OP_NAME` (read global) | `OP=sub`, `REPEAT=7` — `STR(OP)` NUL-terminated bytes must be `"sub"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 109 | `helper_ptr` | `OP=sub`, `REPEAT=7` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 110 | `helper_call` | `OP=sub`, `REPEAT=7` — `OP_FN(sub)` + `RUN_LOOP` unrolled to `REP7`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 111 | `use_generated` | `OP=sub`, `REPEAT=7` — `static accum_sub` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 112 | `driver` `main` (whole pipeline) | `OP=sub`, `REPEAT=7` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 113 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=0` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 114 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=0` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 115 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=0` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 116 | `helper_ptr` | `OP=mul`, `REPEAT=0` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 117 | `helper_call` | `OP=mul`, `REPEAT=0` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP0`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 118 | `use_generated` | `OP=mul`, `REPEAT=0` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 119 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=0` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 120 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=1` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 121 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=1` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 122 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=1` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 123 | `helper_ptr` | `OP=mul`, `REPEAT=1` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 124 | `helper_call` | `OP=mul`, `REPEAT=1` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP1`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 125 | `use_generated` | `OP=mul`, `REPEAT=1` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 126 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=1` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 127 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=2` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 128 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=2` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 129 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=2` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 130 | `helper_ptr` | `OP=mul`, `REPEAT=2` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 131 | `helper_call` | `OP=mul`, `REPEAT=2` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP2`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 132 | `use_generated` | `OP=mul`, `REPEAT=2` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 133 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=2` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 134 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=3` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 135 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=3` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 136 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=3` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 137 | `helper_ptr` | `OP=mul`, `REPEAT=3` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 138 | `helper_call` | `OP=mul`, `REPEAT=3` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP3`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 139 | `use_generated` | `OP=mul`, `REPEAT=3` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 140 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=3` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 141 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=4` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 142 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=4` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 143 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=4` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 144 | `helper_ptr` | `OP=mul`, `REPEAT=4` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 145 | `helper_call` | `OP=mul`, `REPEAT=4` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP4`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 146 | `use_generated` | `OP=mul`, `REPEAT=4` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 147 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=4` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 148 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=5` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 149 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=5` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 150 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=5` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 151 | `helper_ptr` | `OP=mul`, `REPEAT=5` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 152 | `helper_call` | `OP=mul`, `REPEAT=5` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP5`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 153 | `use_generated` | `OP=mul`, `REPEAT=5` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 154 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=5` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 155 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=6` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 156 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=6` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 157 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=6` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 158 | `helper_ptr` | `OP=mul`, `REPEAT=6` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 159 | `helper_call` | `OP=mul`, `REPEAT=6` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP6`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 160 | `use_generated` | `OP=mul`, `REPEAT=6` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 161 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=6` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |
| 162 | `op_add`, `op_sub`, `op_mul` | `OP=mul`, `REPEAT=7` — all three leaf ops called directly; 12 boundary pairs x 12 + 512 random `(a,b)` full-range pairs | [x] |
| 163 | `G_OP` (read global, then call through it) | `OP=mul`, `REPEAT=7` — pointer read from `.data` must equal the `op_mul` symbol address of the same library; called with the boundary + 256 random pairs | [x] |
| 164 | `G_OP_NAME` (read global) | `OP=mul`, `REPEAT=7` — `STR(OP)` NUL-terminated bytes must be `"mul"`; pointer must be non-null and the bytes read-only-comparable | [x] |
| 165 | `helper_ptr` | `OP=mul`, `REPEAT=7` — indirect-call path; boundary pairs + 256 random pairs; return value AND `helper.ptr=%d` stdout compared byte-for-byte | [x] |
| 166 | `helper_call` | `OP=mul`, `REPEAT=7` — `OP_FN(mul)` + `RUN_LOOP` unrolled to `REP7`; boundary pairs + 256 random pairs; return value AND `helper.call=%d helper.acc=%d` stdout compared | [x] |
| 167 | `use_generated` | `OP=mul`, `REPEAT=7` — `static accum_mul` + `DISPATCH_REP`; `n` in `-64..=64` (covers every `case` and the `default`), the int extremes, and 512 random `int`s; return value AND `gen.acc=%d` stdout compared | [x] |
| 168 | `driver` `main` (whole pipeline) | `OP=mul`, `REPEAT=7` — end-to-end process run: argv shapes {2 numeric args, extra args, non-numeric, empty, signed, whitespace, `atoi`-overflowing, 1 arg, 0 args}; 128 random arg pairs; stdout+stderr+exit status compared | [x] |

**Total: 168 configuration rows** (24 builds x 7 entry-point/input-shape groups) plus row 0.
All rows are checked off by `./run_tests.sh`, which runs the differential test
suite once per feature combination (`cargo test --no-default-features --features <op>,<rep>`).

## Row -> test mapping

Each configuration `(OP, REPEAT)` is selected by running the suite with
`--no-default-features --features <op>,<rep>`; within a run the seven
entry-point/input-shape groups map to these tests in `tests/phase_b_valid.rs`:

| group | test |
|-------|------|
| build matrix (row 0) | `configs_00_build_matrix_matches` |
| `op_add`/`op_sub`/`op_mul` | `b_01_leaf_ops` |
| `G_OP` | `b_02_g_op_global` |
| `G_OP_NAME` | `b_03_g_op_name_global` |
| `helper_ptr` | `b_04_helper_ptr` |
| `helper_call` | `b_05_helper_call` |
| `use_generated` | `b_06_use_generated` |
| `driver` `main` | `b_07_driver_pipeline` |
| all of the above, against an independent re-derivation of the `printf` text (also proves the stdout capture is real) | `b_08_printf_text_oracle` |

Driver:

```
$ ./build_c.sh && ./build_rust.sh && ./run_tests.sh
...
PASS [add,0] 32 assertions/tests
... (24 (OP,REPEAT) combos)
PASS [__none__] 32 assertions/tests                      # C's #ifndef defaults
PASS [add,sub,mul,0,1,2,3,4,5,6,7] 32 assertions/tests    # degenerate combo
-----
combos passed=26 failed=0
```
