# CONFIGS.md — Configuration surface for VALID inputs (Phase A)

Mechanically derived from the C source, not from assumptions.

## Axes the C code actually branches on

**Public entry points** (`c_src/include/staticalias.h` — the complete API):

* `int *static_alias(int *outer)` — the **lowest-level** entry point. Tested
  directly, not only through `driver`.
* `void driver(int initial_value, int iterations)` — the convenience/one-shot
  wrapper that composes `static_alias` in a loop and formats with `printf`.

**Runtime options / modes / flags:** there are **none**. No `#ifdef` in the
sources, no global config, no setters, no `enum`/`struct` parameters, and
`Cargo.toml` declares no `[features]`. The only "mode" the library has is the
value of its hidden persistent state.

**Hidden persistent state (the real configuration axis):**
`static int inner = 1` inside `static_alias`. It survives across calls and
across `driver` invocations, so *every* call's behaviour is a function of
(argument, current `inner`). It is reachable/observable because the `if` arm
returns `&inner`, so a test can both read and *set* it. Distinguished states:
`1` (fresh, as loaded), `0`, positive, negative, `INT_MAX`, `INT_MIN`.

**Branches taken on those axes** (`staticalias.c`):

* L30 `if (*outer >= inner)` → arm A: `inner += *outer; return &inner`
  (returned pointer is the library's static, *not* the caller's).
* L33 `else` → arm B: `*outer += inner; return outer`
  (returned pointer **is** the caller's pointer; caller's memory was written).
* L45 `for (i = 0; i < iterations; i++)` → zero-trip vs 1 vs many.
* L46 the returned pointer is fed back as the next `outer` ⇒ aliasing
  `outer == &inner` once arm A is taken, which pins the loop into arm A forever
  (self-doubling).
* L47 `printf("%d\n", *running_sum)` ⇒ exact stdout byte stream is part of the
  observable output, including negative values and stdio buffering.

**Input shapes special-cased:** signed comparison relation (`>`, `==`, `<`);
sign of the operands; boundary magnitudes (`INT_MIN`, `INT_MIN+1`, `-1`, `0`,
`1`, `INT_MAX-1`, `INT_MAX`); whether the addition overflows; pointer aliasing
(`outer` is the caller's own variable vs `outer == &inner`); call-sequence
length (empty / one / many); pointer identity of the result (`ret == outer` vs
`ret == &inner`) and its stability across calls.

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed
`0x5A71C_A11A5` — a deterministic SplitMix64 in the test file) unless the row is
by definition a single exhaustive/edge combination; both `.so`s are driven
through `libloading` in the identical configuration and compared byte-for-byte
(returned value, returned pointer *identity*, the caller's `*outer` after the
call, the resulting `inner`, and — for `driver` — the captured stdout bytes).

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|--------------------------------------------|----|
| 1 | `static_alias` | fresh state (`inner == 1` as loaded), single call, `*outer` random ⇒ mixes arms A and B | [x] |
| 2 | `static_alias` | `inner` preset random, `*outer > inner` (strict) ⇒ arm A; assert result pointer `== &inner` (`!= outer`), `inner` updated, `*outer` untouched | [x] |
| 3 | `static_alias` | `inner` preset random, `*outer == inner` (the `>=` equality boundary) ⇒ arm A | [x] |
| 4 | `static_alias` | `inner` preset random, `*outer < inner` ⇒ arm B; assert result pointer `== outer`, `*outer` updated, `inner` untouched | [x] |
| 5 | `static_alias` | `*outer == inner - 1` and `*outer == inner + 1` (one step either side of the branch boundary), randomized `inner` | [x] |
| 6 | `static_alias` | `inner == 0`, randomized `*outer` (both arms, sign-dependent) | [x] |
| 7 | `static_alias` | `inner` negative, `*outer` negative (arm A/B by relation, negative accumulation) | [x] |
| 8 | `static_alias` | `inner` positive, `*outer` negative ⇒ arm B, `*outer` moves toward positive | [x] |
| 9 | `static_alias` | `inner` negative, `*outer` positive ⇒ arm A | [x] |
| 10 | `static_alias` | `inner == INT_MAX` / `INT_MIN`, `*outer` random ⇒ arm A/B with wrap-around | [x] |
| 11 | `static_alias` | `*outer == INT_MAX` / `INT_MIN` / `0` / `1` / `-1`, `inner` random | [x] |
| 12 | `static_alias` | exhaustive cross-product of the 8 boundary values `{INT_MIN, INT_MIN+1, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` for `inner` × the same 8 for `*outer` (64 combinations) | [x] |
| 13 | `static_alias` | **self-aliasing**: feed the returned `&inner` back in, repeatedly (5..40 randomized repeats) ⇒ pinned in arm A, doubling with wrap | [x] |
| 14 | `static_alias` | **persistence across calls**: long randomized call sequence (256 calls) on independent caller variables, state carried between them; every step compared | [x] |
| 15 | `static_alias` | pointer-identity/stability: `&inner` returned by different calls is the same address; arm B returns exactly the caller's pointer | [x] |
| 16 | `static_alias` | two distinct caller variables used alternately, so arm B writes to different memory each time while `inner` is unchanged | [x] |
| 17 | `driver` | fresh state, `iterations == 0` ⇒ zero-trip loop, empty stdout, `inner` untouched | [x] |
| 18 | `driver` | fresh state, `iterations == 1`, randomized `initial_value` ⇒ single line of stdout | [x] |
| 19 | `driver` | fresh state, `iterations == 2` and `3`, randomized `initial_value` ⇒ first-iteration arm decides whether iteration 2 is self-aliasing (doubling) or another arm-B creep | [x] |
| 20 | `driver` | fresh state, randomized `initial_value >= 1` (arm A first) × randomized `iterations` in `1..=64` ⇒ immediate lock into doubling + overflow wrap | [x] |
| 21 | `driver` | fresh state, `initial_value` in `[0, inner)` and negative (arm B first) × randomized `iterations` ⇒ creep-then-lock path | [x] |
| 22 | `driver` | `inner` preset (positive / negative / `0` / `INT_MAX` / `INT_MIN`) × randomized `initial_value` × randomized `iterations` ⇒ the full state × input cross-product | [x] |
| 23 | `driver` | `initial_value` at boundaries `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` × `iterations` in `{1,2,3,7,33,64}` × presets | [x] |
| 24 | `driver` | many iterations (`128`, `1000`, `4096`) ⇒ long stdout stream, repeated wrap-around, stdio buffering across a large write | [x] |
| 25 | `driver` | **two consecutive `driver` calls** with no state reset in between (state carry-over between wrapper invocations), randomized args | [x] |
| 26 | `driver` + `static_alias` | **interleaved** low-level and wrapper calls on the same shared `inner` (randomized 64-step program mixing both entry points) | [x] |
| 27 | `driver` | caller's argument copy: same `initial_value` passed twice in a row must not be mutated in the caller (by-value parameter), verified against C | [x] |
| 28 | (data segment) | the **as-loaded** state: the `static int inner = 1;` initialiser read out of both freshly `dlopen`ed libraries, plus the fresh-state behaviour it implies. Added after mutation testing showed every other row presets `inner` and so could not catch a wrong initialiser. | [x] |

Rows 1–28 are covered by `translation/tests/valid_paths.rs`.
