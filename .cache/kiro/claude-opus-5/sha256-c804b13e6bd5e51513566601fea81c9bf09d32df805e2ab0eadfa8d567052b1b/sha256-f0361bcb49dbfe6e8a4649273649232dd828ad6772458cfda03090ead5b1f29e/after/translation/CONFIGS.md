# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axes the C actually branches on

The library has **no** runtime options, no global state, no init/teardown, no
`#ifdef`, no flags, and no formats or byte-order handling. Its only branching is
on the single `int pfcn` argument, in three `switch` statements:

1. `call_predict` (line 232) — 12 explicit cases (`0..=11`) + `default:`.
2. `BTAC1C2_GetPredictFunc` (line 185) — 12 explicit cases (`0..=11`) + `default:`.
3. `BTAC1C2_PredictSample` (line 22) — 12 individual cases (`0..=11`), one
   fall-through group (`12,13,14,15`) + `default:`.

Input shapes:

* `pfcn`: one `int`. Distinct classes = each of `0..=11`, the group `12..=15`,
  anything `>15`, anything `<0`, and the two `int` extremes.
* `psamp`: `int[8]` ring buffer, indexed `(idx - k) & 7` for `k` in `1..=8`.
  Value-dependent shapes: all-zero, all-positive, all-negative, mixed sign,
  extreme magnitudes (the arms multiply by up to 76 and sum 8 terms, so overflow
  and negative-value arithmetic-shift / truncating-division behaviour are the
  interesting shapes), and `psamp[i] == INT_MIN/INT_MAX`.
* `idx`: any `int`, including negative and `INT_MIN`/`INT_MAX` (masked with `& 7`).
* `ridx->firfx[4][8]`: `s16` coefficients, only read by arms `12..=15`.

## Public entry points

`call_predict` is the **only** exported symbol (see `SYMBOLS.md`). All the
prediction math lives in `static` functions with internal linkage, so it is not
reachable across the C `.so` boundary.

To avoid leaving that math unverified, the non-default Cargo feature
`test_internals` adds thin `extern "C"` wrappers over the private Rust helpers,
and `harness/wrap.c` (a new file **outside** `c_src/`, which `#include`s the
unmodified `c_src/src/lib.c`) adds the identical wrappers over the C statics.
Rows 13+ drive those lowest-level entry points directly. The default build keeps
exact symbol parity with C.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `call_predict` | `pfcn = 0` (case 0) | [x] |
| 2 | `call_predict` | `pfcn = 1` (case 1) | [x] |
| 3 | `call_predict` | `pfcn = 2` (case 2) | [x] |
| 4 | `call_predict` | `pfcn = 3` (case 3) | [x] |
| 5 | `call_predict` | `pfcn = 4` (case 4) | [x] |
| 6 | `call_predict` | `pfcn = 5` (case 5) | [x] |
| 7 | `call_predict` | `pfcn = 6` (case 6) | [x] |
| 8 | `call_predict` | `pfcn = 7` (case 7) | [x] |
| 9 | `call_predict` | `pfcn = 8` (case 8) | [x] |
| 10 | `call_predict` | `pfcn = 9` (case 9) | [x] |
| 11 | `call_predict` | `pfcn = 10` (case 10) | [x] |
| 12 | `call_predict` | `pfcn = 11` (case 11) | [x] |
| 13 | `call_predict` | exhaustive sweep `pfcn ∈ -64..=64` (spans every case + both defaults) | [x] |
| 14 | `call_predict` | randomized 32-bit `pfcn`, fixed-seed PRNG, 200 000 draws | [x] |
| 15 | `rsw_predict_sample` / C `wrap_predict_sample` | `pfcn ∈ 0..=11`, random `psamp` (small mixed-sign values), random `idx` | [x] |
| 16 | `rsw_predict_sample` / C `wrap_predict_sample` | `pfcn ∈ 0..=11`, random `psamp` full 32-bit range (overflow + `>>` on negatives + truncating `/`) | [x] |
| 17 | `rsw_predict_sample` / C `wrap_predict_sample` | `pfcn ∈ 0..=11`, `psamp` all-zero / all-`INT_MAX` / all-`INT_MIN` / alternating extremes | [x] |
| 18 | `rsw_predict_sample` / C `wrap_predict_sample` | `pfcn ∈ 12..=15` (FIR arms), random `firfx` `s16` coefficients incl. `i16::MIN`/`MAX`, random `psamp` | [x] |
| 19 | `rsw_predict_sample` / C `wrap_predict_sample` | `pfcn` outside `0..=15` (`default:` arm) with arbitrary `psamp` | [x] |
| 20 | `rsw_predict_sample` / C `wrap_predict_sample` | `idx` boundary shapes: `0`, `±1`, `7`, `8`, `-8`, `INT_MIN`, `INT_MAX`, random 32-bit, for every `pfcn` in `0..=15` | [x] |
| 21 | `rsw_pfn0` … `rsw_pfn11` / C `wrap_pfn0` … `wrap_pfn11` | each specialised helper, random small `psamp`, random `idx` | [x] |
| 22 | `rsw_pfn0` … `rsw_pfn11` / C `wrap_pfn0` … `wrap_pfn11` | each specialised helper, random full-32-bit `psamp` (exposes the `Pfn10 >>3` vs `case 10 >>4` and `Pfn11 >>1` vs `case 11 >>3` divergences the C really has) | [x] |
| 23 | `rsw_pfn0` … `rsw_pfn11` / C `wrap_pfn0` … `wrap_pfn11` | each specialised helper, `psamp` extremes + `idx` extremes cross-product | [x] |
| 24 | `rsw_get_predict_func_index` / C `wrap_get_predict_func_index` | dispatcher identity: which helper `BTAC1C2_GetPredictFunc` hands back, swept over `pfcn ∈ -64..=64` | [x] |
| 25 | composed pipeline | for `pfcn ∈ -64..=64`: dispatch via `GetPredictFunc`, then **call through the returned pointer** with random `psamp`/`idx`, comparing C and Rust results — the end-to-end path a real consumer takes | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)`. There is
no driver binary, and `translation/Cargo.toml` declares `crate-type = ["cdylib"]`
with no `[[bin]]`. The "compare stdout of C and Rust binaries" gate is therefore
not applicable.
