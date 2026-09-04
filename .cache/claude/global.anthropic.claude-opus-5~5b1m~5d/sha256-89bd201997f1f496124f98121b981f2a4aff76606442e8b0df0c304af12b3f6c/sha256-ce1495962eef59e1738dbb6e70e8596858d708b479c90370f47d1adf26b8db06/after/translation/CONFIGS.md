# CONFIGS.md — Phase B valid-configuration surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`. Axes the C code
actually branches on:

**Axis O — runtime "option" (the only injectable behaviour): `operation_func op`**
`process_with_foreach` takes a function pointer. The four operations the TU defines,
plus arbitrary externally supplied callbacks, are distinct modes:
`add_operation` | `multiply_operation` | `subtract_operation` | `modulo_operation`
| foreign callback | *(the fixed 4-op sequence baked into `arrayfunc`)*.
The provider of the pointer is itself an axis: the **C** `.so`'s exports, the
**Rust** `.so`'s exports, and a **harness** Rust `extern "C"` fn.

**Axis N — `ResultArray::count` shape:** `0` | `1` | `2` | `3..9` (many) | `10`
(full capacity) | clamped-from-`>10` via `init_result_array` | `>10` written directly
into the `count` field, which the C never rejects (rows 42-45).

**Axis V — element value shape:** zeros | small ± | `INT32_MAX`/`INT32_MIN`
boundaries | values whose `*0.75` / `*weight*0.8` saturates | random 32-bit.

**Axis S — `double` shape for the float entry points:** `0.0` / `-0.0` |
`(-1,1)` fractional | integral | `.5` ties | huge | `±INFINITY` | `NaN` | subnormal |
exact `±2^31` boundaries.

**Axis P — call composition:** single call | `init` → `process` (×k) → `weighted_sum`
→ `compare` (the `arrayfunc` pipeline) | repeated `process` on already-mutated state
(the C mutates `value` **and** `scaled` in place, so iteration *k* depends on *k-1*).

**Axis M — struct-memory shape:** freshly zeroed `ResultArray` | pre-poisoned
`ResultArray` (non-zero bytes in `data[count..10]`, so any accidental extra write is
visible) | `count` field written directly by the caller, bypassing `init_result_array`.

There are **no** compile-time options: `c_src/src/lib.c` contains no `#ifdef`, and
`translation/Cargo.toml` declares no `[features]`. Byte order / element type / format
axes do not exist (fixed `int`/`double`, no serialization).

Cross-product, pruned to combinations the C distinguishes:

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `add_operation` | random `(a,b)` over full `i32` incl. overflow pairs; `unused1/2` set to random junk to prove they are ignored | [x] |
| 2  | `multiply_operation` | random `(a,b)` incl. `INT32_MIN*-1`, `MAX*MAX` overflow; random unused args | [x] |
| 3  | `subtract_operation` | random `(a,b)` incl. `INT32_MIN - 1` underflow; random unused args | [x] |
| 4  | `modulo_operation` | random `(a,b)` with `b != 0`, both signs (C truncated remainder keeps the sign of `a`) | [x] |
| 5  | `modulo_operation` | `b == 0` sweep over random `a`, plus the neighbours of the `(INT32_MIN, -1)` trap pair (the pair itself SIGFPEs in C — ERRORS.md row 2) | [x] |
| 6  | `safe_double_to_int` | axis S: every shape above, plus random bit patterns reinterpreted as `f64` (incl. NaN payloads) | [x] |
| 7  | `safe_double_to_int` | random values in the exact-representable boundary band `[2^31-2, 2^31+2]` and its negation | [x] |
| 8  | `compute_scaled_value` | `base` random `i32` × `scale_factor` from axis S (0, ±1, 0.333, 0.75, 1.5, 0.8, huge, inf, NaN, subnormal) | [x] |
| 9  | `compute_scaled_value` | `base = 0` × `scale = ±INFINITY` (⇒ `NaN` ⇒ 0) and `base = 0` × `NaN` | [x] |
| 10 | `init_result_array` | `count = 0`, zeroed `arr` — verify no byte of `data[]` is written | [x] |
| 11 | `init_result_array` | `count = 1`, random value | [x] |
| 12 | `init_result_array` | `count = 2`, random values | [x] |
| 13 | `init_result_array` | `count` in `3..=9`, random values, **poisoned** `arr` (axis M) — verify the tail is left poisoned identically | [x] |
| 14 | `init_result_array` | `count = 10` (boundary, no clamp) | [x] |
| 15 | `init_result_array` | `count` in `11..=64` and `INT32_MAX` ⇒ clamp to 10; `values` buffer over-allocated so only the first 10 may legally be read | [x] |
| 16 | `init_result_array` | values at boundaries (`INT32_MAX`, `INT32_MIN`, `0`, `±1`) — checks the `value*1.5` `scaled` field bit-exactly | [x] |
| 17 | `init_result_array` | called **twice** on the same `arr`, second time with a smaller `count` — stale `data[newcount..oldcount]` must survive identically | [x] |
| 18 | `process_with_foreach` | `op = add_operation`, `count` 1/2/5/10, random values, `arr` built by `init_result_array` | [x] |
| 19 | `process_with_foreach` | `op = multiply_operation`, `count` 1/2/5/10, random values (`value*rank`, so rank 0 zeroes element 0) | [x] |
| 20 | `process_with_foreach` | `op = subtract_operation`, `count` 1/2/5/10, random values | [x] |
| 21 | `process_with_foreach` | `op = modulo_operation`, `count` 1/2/5/10 — `b` is `rank`, so element 0 always hits the `b == 0` guard | [x] |
| 22 | `process_with_foreach` | `count = 0` with each of the four ops (loop never runs; must return 0 and not touch `data[]`) | [x] |
| 23 | `process_with_foreach` | **cross-provider**: `op` taken from the **C** `.so` while the driver is the **Rust** `.so`, and vice versa — proves the function-pointer ABI matches | [x] |
| 24 | `process_with_foreach` | `op` = a **harness** Rust `extern "C"` callback that records `(a,b,unused1,unused2)`; asserts both libraries pass the identical argument sequence, in order, incl. the literal `0,0` for the unused params | [x] |
| 25 | `process_with_foreach` | repeated invocation (axis P): same `arr`, same op, 4 iterations in a row — state carried through the in-place `value`/`scaled` mutation | [x] |
| 26 | `process_with_foreach` | axis P: the exact 4-op sequence `add, multiply, subtract, modulo` on one `arr`, as `arrayfunc` does, over random `count`/values | [x] |
| 27 | `process_with_foreach` | saturating shape: values near `INT32_MAX`/`INT32_MIN` with `multiply_operation` so `result*0.75` clips in `safe_double_to_int` and `total` wraps | [x] |
| 28 | `process_with_foreach` | axis M: `count` written directly (not via `init_result_array`) with a hand-built `data[]` incl. arbitrary `rank` values decoupled from the index | [x] |
| 29 | `compute_weighted_sum` | `count = 1` (only the `weight = 1` branch of the ternary) | [x] |
| 30 | `compute_weighted_sum` | `count = 2..10` (both ternary branches: `i == 0` → 1, `i > 0` → `i`) | [x] |
| 31 | `compute_weighted_sum` | `count = 0` (returns 0) | [x] |
| 32 | `compute_weighted_sum` | boundary values (`INT32_MAX`/`INT32_MIN` in `value`) at every index, so `value*weight*0.8` saturates at various weights | [x] |
| 33 | `compute_weighted_sum` | axis M: hand-built `data[]` (arbitrary `scaled`, `rank`) — confirms only `value` is read and nothing is written | [x] |
| 34 | `compute_weighted_sum` | called after `process_with_foreach` (axis P composition), random shapes | [x] |
| 35 | `compare_results_in_array` | both indices in range, all ordered pairs for `count` in `1..=10` (exhaustive over `idx1,idx2 ∈ 0..count`) | [x] |
| 36 | `compare_results_in_array` | full `arrayfunc`-style sweep `i` vs `i+1` for `count = 8` | [x] |
| 37 | `arrayfunc` | the documented one-shot entry point: 4 random `i32` params, thousands of seeded cases | [x] |
| 38 | `arrayfunc` | boundary params: every combination drawn from `{0, 1, -1, 2, -2, INT32_MAX, INT32_MIN, INT32_MAX/2, INT32_MIN/2}` (9⁴ = 6561 cases, exhaustive) | [x] |
| 39 | `arrayfunc` | small-magnitude params (`-8..=8` cross-product, 17⁴ = 83 521 cases) where no saturation occurs — the "ordinary" arithmetic path | [x] |
| 40 | `arrayfunc` | odd/negative `param4` to exercise C truncation-toward-zero in `param4 / 2` (`-1/2 == 0`, `-3/2 == -1`) | [x] |
| 41 | `Result` / `ResultArray` layout | `sizeof`/offset agreement, proven by writing a poison pattern through one `.so` and reading the mutated bytes back after the other `.so` operates on it | [x] |
| 42 | `compute_weighted_sum` | axis M/N extended: `count` written directly as `11..=73` on an over-allocated buffer. Read-only, so stable — and this is the ONLY way to drive `weight` above 9, which no `init_result_array`-built array can reach | [x] |
| 43 | `process_with_foreach` | `count = 11..=73`, **one pass per buffer**, each of the four ops (see the aliasing note below for why exactly one) | [x] |
| 44 | `process_with_foreach` | the `data[10]` / `count` aliasing itself: `count > 10` and assert both libraries clobber `count` to the *same* value | [x] |
| 45 | `compare_results_in_array` | `count = 11..=73` with in-range, boundary and negative indices; read-only so repeatable | [x] |

All rows are driven with many randomized inputs from a fixed-seed
(`0x2026_09_03_C0FFEE`) SplitMix64 generator, not single hand-picked values.


## Discovered during Phase B: `data[10]` aliases `count`

`sizeof(Result) == 24` and `count` lives at offset `240 == 10 * 24`, so **`data[10].value`
and `count` are the same four bytes**. Consequences, all reproduced by the Rust:

* With `count > 10`, the 11th iteration of `process_with_foreach` writes
  `safe_double_to_int(result * 0.75)` straight over `count`.
* The loop itself survives, because the `FOREACH` macro snapshots `size = (count)`
  once in its initialiser rather than re-reading it.
* Any **subsequent** call then re-reads the corrupted `count` (often `INT32_MAX`) and
  walks off the end of the object. Verified experimentally: a second
  `process_with_foreach` pass with `count = 11` **SIGSEGVs**.

This is exactly what `init_result_array`'s `count < 10 ? count : 10` clamp exists to
prevent. Row 43 therefore uses one pass per freshly cloned buffer — the deepest
well-defined probe of that path — and row 44 pins the aliasing down directly.

## Axes deliberately NOT enumerated, and why

* **Compile-time options** — `c_src/src/lib.c` has no `#ifdef` and
  `translation/Cargo.toml` has no `[features]` table. `scripts/verify_all.sh`
  derives this mechanically and still runs the full suite for `DEFAULT` and
  `--no-default-features`, in both `debug` and `release`, so the "every feature
  combination" gate is met by construction rather than by assumption.
* **Byte order / element type / serialization format** — the API is fixed `int` and
  `double`; nothing is serialized, so these axes do not exist.
* **Enum values** — there is no `enum` anywhere in `c_src/`. The analogous
  "any bit pattern is a legal argument" surface is covered in `ERRORS.md`
  (rows 8-18) and by `generic_extreme_int_arguments_across_ffi`.

## Mutation testing (evidence the suite is not vacuous)

16 mutations were injected into `src/lib.rs` and the whole suite re-run against each.
Every non-equivalent mutation was **killed**: the `count < 10` clamp bound, both
`compare_results_in_array` guards, `total` accumulation, the `rank` value, the `1.5`
and `0.75` and `0.333` scale factors, the `weight = 1` special case for element 0,
`param2 - param3` operand order, and the `+ 1` on `param4 / 2`.

Three survivors were each *proven* semantically equivalent rather than accepted:

| mutation | why it cannot be observed |
|---|---|
| `d >= INT32_MAX` → `d > INT32_MAX` | differs only at `d == 2147483647.0`, where the fallthrough `(int)d` yields the same `INT32_MAX` |
| `count < 10` → `count <= 10` | differs only at `count == 10`, where both store `10` |
| `(v*w)*0.8` → `v*(w*0.8)` | exhaustive search over **all 2^32 values of `value` x every weight `1..=73`** (22.3 billion pairs where the two `double`s genuinely differ) found **0** cases where the truncated `int` differs |
| `if (d != d)` → `if (false)` | Rust's float->int cast saturates NaN to `0`, the same value the guard returns |
