# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep of every rejection-capable construct

```
$ grep -n -E "return|assert|NULL|error|ERROR|if|switch|case|default|INT_|MAX|MIN" \
      c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:12:    if (s == 0) {
c_src/src/lib.c:16:        return;          <- plain `return;`, void, NOT an error return
c_src/src/lib.c:24:    switch (i) {
c_src/src/lib.c:25:    case 0:
c_src/src/lib.c:30:    case 1:
c_src/src/lib.c:35:    case 2:
c_src/src/lib.c:40:    case 3:
c_src/src/lib.c:45:    case 4:
c_src/src/lib.c:50:    default:
```

Findings (these are facts about the C, not assumptions):

* the public API is a single `void`-returning function — **there is no error
  channel at all**: no `return -1`, no `return NULL`, no `errno`, no status
  enum, no out-parameter flag;
* there are **zero** `assert()` / `static_assert` / `abort()` calls;
* there are **zero** null-pointer checks (`dest` and `src` are dereferenced
  unconditionally);
* there are **zero** range checks on `h`, `s`, `v` (no clamping to `[0,360)`
  or `[0,1]`, no `fmodf` wrap);
* there are **zero** length/count parameters, therefore no zero-length or
  oversized-length rejection can exist;
* there are **zero** enum parameters; the only enum-like selector is the
  *internal* `int i`, and the `switch` handles every out-of-domain value with
  `default:` (gcc emits `cmpl $4,i; ja default`, i.e. an **unsigned** bound
  check, so all negative `i` reach `default` too).

Consequently the "error surface" of this library consists of (a) the one
short-circuit branch, (b) the `default:` catch-all that absorbs every
out-of-domain selector value, and (c) the undefined-behaviour cases whose
*observable* behaviour in the compiled `.so` is the ground truth the Rust must
reproduce. Each of those is one row below.

`INT_MIN` below means `0x8000_0000` — the "integer indefinite" value that
`cvttss2si` (the instruction gcc emits for `(int)floorf(h)`, verified by
`objdump -d`) returns for NaN and for every operand outside `[-2^31, 2^31)`.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `hsv_to_rgb` | `s == 0.0f` (line 12) — short-circuit "rejects" the whole HSV computation | writes `dest[0]=dest[1]=dest[2]=v`, returns; `h` is never used, `src[0]` is still loaded | `err01_s_is_positive_zero` | [x] |
| 2 | `hsv_to_rgb` | `s == -0.0f` — IEEE `-0.0 == 0` is *true*, so the same short-circuit fires | same as row 1 (NOT the main path) | `err02_s_is_negative_zero` | [x] |
| 3 | `hsv_to_rgb` | `s` is NaN — `ucomiss` sets PF, `s == 0` is *false*, so NaN is **not** rejected | falls through to the main path; NaN propagates into `p`/`q`/`t` | `err03_s_is_nan_takes_main_path` | [x] |
| 4 | `hsv_to_rgb` | `s` is a *signalling* NaN | same as row 3 (comparison quiets it, no trap) | `err04_s_is_signalling_nan` | [x] |
| 5 | `hsv_to_rgb` | `i = (int)floorf(h/60) > 4`, i.e. `h >= 300` (`i == 5`) — first value past the last `case` | `switch` `default:` arm: `r=v, g=p, b=q` | `err05_i_one_past_last_case` | [x] |
| 6 | `hsv_to_rgb` | `i` far above the case range (`h >= 360`, no hue wrap-around) | `default:` arm | `err06_i_far_above_range` | [x] |
| 7 | `hsv_to_rgb` | `i < 0` (`h < 0`, one step below `case 0`) — reaches `default` via the **unsigned** `ja` bound check | `default:` arm | `err07_i_negative` | [x] |
| 8 | `hsv_to_rgb` | `h` is NaN → `floorf(NaN)=NaN` → `(int)NaN` is UB; `cvttss2si` yields `INT_MIN` | `i == INT_MIN` → `default:` arm; `f = NaN`, `q = NaN` | `err08_h_nan_gives_int_min` | [x] |
| 9 | `hsv_to_rgb` | `h` is a signalling NaN | quieted by `floorf`, then as row 8 | `err09_h_signalling_nan` | [x] |
| 10 | `hsv_to_rgb` | `h/60 >= 2^31` (incl. `h == +INFINITY`) → float→int conversion out of range (UB) | `cvttss2si` yields `INT_MIN` → `default:` arm | `err10_h_above_int_range` | [x] |
| 11 | `hsv_to_rgb` | `h/60 <= -2^31` (incl. `h == -INFINITY`) → out of range (UB) | `cvttss2si` yields `INT_MIN` → `default:` arm | `err11_h_below_int_range` | [x] |
| 12 | `hsv_to_rgb` | `h/60 == -2147483648.0f` exactly — the in-range boundary that Rust's saturating `as` and `cvttss2si` agree on | `i == INT_MIN` → `default:` arm | `err12_h_at_int_min_boundary` | [x] |
| 13 | `hsv_to_rgb` | `s` outside the documented `[0,1]`: `s < 0` (one step past the low bound) | no check; `p/q/t` may exceed `[0,1]` | `err13_s_below_range` | [x] |
| 14 | `hsv_to_rgb` | `s > 1` (one step past the high bound) | no check; `p = v*(1-s)` goes negative | `err14_s_above_range` | [x] |
| 15 | `hsv_to_rgb` | `s == ±INFINITY` | no check; `1-s = ∓inf`, products are `±inf` or NaN | `err15_s_infinite` | [x] |
| 16 | `hsv_to_rgb` | `v` outside `[0,1]` (negative / `>1` / huge) | no check; outputs simply scale | `err16_v_out_of_range` | [x] |
| 17 | `hsv_to_rgb` | `v == ±INFINITY` or NaN | no check; propagates | `err17_v_inf_or_nan` | [x] |
| 18 | `hsv_to_rgb` | invalid-operation producing operand pair, e.g. `v == 0` with `s == ±inf` → `0 * inf` | default QNaN `0xffc0_0000` in the affected channel | `err18_zero_times_inf_qnan` | [x] |
| 19 | `hsv_to_rgb` | `dest == NULL` (no null check, line 13/56) | UB: `SIGSEGV` on the first store — for **both** the `s==0` path and the main path | `err19_null_dest_crash_parity` | [x] |
| 20 | `hsv_to_rgb` | `src == NULL` (no null check, line 8) | UB: `SIGSEGV` on the `src[0]` load | `err20_null_src_crash_parity` | [x] |
| 21 | `hsv_to_rgb` | both pointers NULL | UB: `SIGSEGV` (load faults first) | `err21_both_null_crash_parity` | [x] |
| 22 | `hsv_to_rgb` | `src[0]` unreadable while `src[1..2]` are readable **and** `s == 0` — line 8 loads `src[0]` *before* the line-12 short-circuit, so the early-return path still faults | UB: `SIGSEGV` even though `h` is unused | `err22_unconditional_h_load_faults` | [x] |
| 23 | `hsv_to_rgb` | `dest[2]` unwritable, `dest[0..1]` writable | UB: `SIGSEGV` *after* `dest[0]`/`dest[1]` were already stored (partial write is observable) | `err23_partial_store_before_fault` | [x] |
| 24 | `hsv_to_rgb` | out-of-range selector sweep: every `i` in `{-2^31 … -1} ∪ {5 … 2^31-1}` reachable from a float `h`, including `INT_MIN`, `INT_MAX`, `±1`, `5`, `6` | always `default:`; never an out-of-bounds jump-table read | `err24_selector_sweep` | [x] |
| 25 | `hsv_to_rgb` | misaligned `src`/`dest` (not 4-byte aligned) — no alignment check, gcc emits unaligned `movss` | no fault, same results as aligned | `err25_misaligned_pointers` | [x] |

There is deliberately **no** row for "returns an error code": the C cannot.
Every row above is verified by a differential test that asserts the *same*
concrete outcome (identical 3×`u32` output bit patterns, or the identical
termination signal for the UB/crash rows) from the C `.so` and the Rust `.so`.
