# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axis derivation

`c_src/include/lib.h` is one line, so the **full set of public entry points** is:

| entry point | signature | level |
|-------------|-----------|-------|
| `ldexp_q2` | `float ldexp_q2(float y, int exp_q2)` | this is simultaneously the lowest-level and the only entry point — there is no convenience wrapper to hide behind |

There is no init/handle/context object, no setter, no global, no `#ifdef`, no
`switch`. Therefore there are **no runtime option/mode/flag axes**; the entire
configuration surface is the shape of the two arguments. The branches the C
actually takes (lines 8–10) give exactly these axes:

* **A1 — clamp branch** (`(30*4) > exp_q2 ? exp_q2 : (30*4)`, line 8):
  `exp_q2 < 120` (take `exp_q2`) vs `exp_q2 >= 120` (take the constant `120`).
* **A2 — loop trip count** (`while ((exp_q2 -= e) > 0)`, line 10):
  exactly 1 iteration (`exp_q2 <= 120`) vs 2 vs many (`exp_q2 > 240`).
* **A3 — table index** `e & 3` (line 9): the 4 distinct `g_expfrac` entries
  `{0, 1, 2, 3}`, reachable from both positive and negative `e` (two's complement).
* **A4 — shift amount** `e >> 2` (line 9): in-range `0..=30` for
  `0 <= e <= 120`, and out-of-C-range negative for `e < 0`, where the x86
  low-5-bit masking makes the effective count `(e>>2) & 31` — which sweeps the
  whole `0..=31` range again, including `31` (factor becomes `0`) and `0`
  (factor stays `2^30`).
* **A5 — `y` value class** (the `float` operand of the two `mulss`es):
  positive/negative normal, subnormal, `±0.0`, `±inf`, quiet `NaN`, signalling
  `NaN`, `FLT_MAX`, `FLT_MIN`, and arbitrary random bit patterns.
* **A6 — accumulated rounding**: since each iteration is a separate `float`
  multiply, the number of iterations changes the rounding history, so A2 × A5
  is a genuine interaction, not a product of independent effects.

The rows below are the pruned cross-product of A1–A6 — one row per combination
the C code treats differently. Every row is tested in `tests/configs.rs` with
**many randomized `y` values (and randomized `exp_q2` within the row's class)
from a fixed-seed PRNG**, comparing C and Rust `.so` exports bit-for-bit.

## The table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `ldexp_q2` | `exp_q2 == 0`; `y` = random normal bit patterns — 1 iteration, index 0, shift 30 | [x] |
| 2 | `ldexp_q2` | `exp_q2 == 0`; `y` ∈ {`+0.0`, `-0.0`, `+inf`, `-inf`, qNaN, sNaN, `FLT_MAX`, `FLT_MIN`, smallest subnormal} | [x] |
| 3 | `ldexp_q2` | `exp_q2` random in `1..=119` (clamp takes `exp_q2`, 1 iteration); `y` random normal | [x] |
| 4 | `ldexp_q2` | `exp_q2 == 120` exactly (clamp boundary, else-branch, 1 iteration, index 0, shift 30); `y` random normal + specials | [x] |
| 5 | `ldexp_q2` | `exp_q2` in `1..=119` with `exp_q2 & 3 == 0` (index 0); random `y` | [x] |
| 6 | `ldexp_q2` | `exp_q2` in `1..=119` with `exp_q2 & 3 == 1` (index 1); random `y` | [x] |
| 7 | `ldexp_q2` | `exp_q2` in `1..=119` with `exp_q2 & 3 == 2` (index 2); random `y` | [x] |
| 8 | `ldexp_q2` | `exp_q2` in `1..=119` with `exp_q2 & 3 == 3` (index 3); random `y` | [x] |
| 9 | `ldexp_q2` | `exp_q2` sweeping every in-range shift `e >> 2 = 0..=30` (i.e. `exp_q2 = 4*k`, `k = 0..30`); random `y` | [x] |
| 10 | `ldexp_q2` | `exp_q2 == 121` (2 iterations: `e = 120` then `e = 1`); random `y` + specials | [x] |
| 11 | `ldexp_q2` | `exp_q2` random in `121..=240` (exactly 2 iterations, all 4 remainder indices); random `y` | [x] |
| 12 | `ldexp_q2` | `exp_q2 == 240` (2 iterations, remainder exactly `120` → else-branch twice); random `y` | [x] |
| 13 | `ldexp_q2` | `exp_q2 == 241` (3 iterations); random `y` | [x] |
| 14 | `ldexp_q2` | `exp_q2` random in `241..=10_000` (many iterations, accumulated rounding); random `y` | [x] |
| 15 | `ldexp_q2` | `exp_q2` random in `10_001..=1_000_000` (thousands of iterations; result saturates to `0` for finite `y`) | [x] |
| 16 | `ldexp_q2` | `exp_q2` large positive with `y` ∈ {`±inf`, `NaN`, `±0.0`} (specials survive many iterations) | [x] |
| 17 | `ldexp_q2` | `exp_q2 == INT_MAX` (maximum trip count, remainder `7`); `y` = a few normals + specials | [x] |
| 18 | `ldexp_q2` | `exp_q2 == -1` (negative → masked shift count `31` → integer factor `0`, index 3); random `y` | [x] |
| 19 | `ldexp_q2` | `exp_q2 == -2` / `-3` / `-4` (negative, indices 2 / 1 / 0, masked shift `31`); random `y` | [x] |
| 20 | `ldexp_q2` | `exp_q2` random in `-127..=-1` (negative, masked shift `31` or `30`, all 4 indices); random `y` | [x] |
| 21 | `ldexp_q2` | `exp_q2 == -128` (masked shift count wraps to `0` → factor `2^30` unshifted, index 0); random `y` | [x] |
| 22 | `ldexp_q2` | `exp_q2` = negative multiples of `-128` (`-128`, `-256`, `-384`, …) — masked shift `0`, repeated wrap of the count; random `y` | [x] |
| 23 | `ldexp_q2` | `exp_q2` sweeping the full masked-shift range for negatives: `exp_q2 = -4*k` for `k = 1..=64` (covers `(e>>2) & 31` = 31 down to 0 and back); random `y` | [x] |
| 24 | `ldexp_q2` | `exp_q2` random in `INT_MIN..=-1_000_000` (large negatives, masked shift arbitrary, always 1 iteration); random `y` | [x] |
| 25 | `ldexp_q2` | `exp_q2 == INT_MIN` (`e & 3 == 0`, masked shift `0`, `exp_q2 -= e` yields exactly `0`); random `y` + specials | [x] |
| 26 | `ldexp_q2` | `exp_q2 == INT_MIN + 1 .. INT_MIN + 4` (extreme negatives with non-zero indices); random `y` | [x] |
| 27 | `ldexp_q2` | fully unconstrained fuzz: `y` random over **all** 2^32 bit patterns × `exp_q2` random over **all** 2^32 values (biased toward the boundaries above) | [x] |
| 28 | `ldexp_q2` | idempotence/statefulness check: the same `(y, exp_q2)` called repeatedly and interleaved with other inputs — the C `static const` table must not become observable state | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the only combination is
the default/empty one. `run_all.sh` runs the whole suite under both the default
build and `--no-default-features` to make that explicit.
