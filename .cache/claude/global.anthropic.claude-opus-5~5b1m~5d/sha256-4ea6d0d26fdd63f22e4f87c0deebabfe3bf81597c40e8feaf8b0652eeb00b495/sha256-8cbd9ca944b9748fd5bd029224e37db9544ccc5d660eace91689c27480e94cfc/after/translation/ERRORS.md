# ERRORS.md — error-surface table

Mechanically derived from `c_src/src/lib.c` (48 lines) and
`c_src/include/lib.h` (1 line).

## Mechanical grep results

```
grep -nE 'return|assert|NULL|errno|-1|error|ERROR|MAX|MIN|<|>|==' c_src/src/lib.c
```

* `return;`                    — line 14, the *bare early return* of the
  `s == 0` grey short-circuit. It is a control-flow return, **not** an error
  return: the function is `void` and has already written all three outputs.
* No `assert`, no `NULL` check, no `errno`, no error enum, no error code, no
  sentinel value, no `RETURN_ERROR`-style macro, no min/max constant, no
  length/count/size parameter, and no allocation anywhere in the file.
* The only comparisons are the *value dispatch* on `s` and `h`
  (lines 10, 19, 23, 27, 31, 35, 39) — these select an output formula, they do
  not reject input.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | `hsl_to_rgb` | *(none)* | The C function has **no error surface at all**: it is `void`, validates nothing, allocates nothing, and returns no status. Every finite/infinite/NaN input value is accepted and produces three stores. |

There is genuinely **zero** rejection logic to mirror, so there is no error
code, sentinel, or enum for the Rust to match. The rows below therefore cover
the *generic* C-API boundaries that the task requires be probed even when they
are absent from the table — for each one the "expected result" is the observed
C behaviour, and the Rust must match it (or be equally undefined).

| # | boundary probed | C behaviour | differential test |
|---|-----------------|-------------|-------------------|
| E1 | `s == 0.0f` exactly → early `return` at line 14 | writes `l, l, l`; `c`/`m`/`x` never computed, so a NaN/Inf `h` is *ignored* | `err_s_zero_short_circuits` |
| E2 | `s == -0.0f` (negative zero compares equal to `0`) | same early return as E1 | `err_s_negative_zero` |
| E3 | `s` denormal-but-nonzero (`1e-45`, smallest subnormal) → does **not** short-circuit | full formula runs | `err_s_smallest_subnormal` |
| E4 | `s = NaN` — `NaN == 0` is false, so no short-circuit | full formula runs, NaN propagates into `c`, `m`, `x` | `err_s_nan_no_short_circuit` |
| E5 | `h = NaN` → every `>=`/`<` comparison is false → final `else` | writes `m, m, m` | `err_h_nan_falls_to_else` |
| E6 | `h` one step *past* each dispatch boundary (`nextafter` of `0, 60, 120, 180, 240, 300, 360` in both directions) | selects the neighbouring branch; note the `h < 120.0f && h < 180.0f` quirk on line 27 | `err_h_boundary_nextafter` |
| E7 | `h < 0` (negative hue, incl. `-0.0f`, `-1e-45`, `-360`) | line 19 fails, line 23 fails, **line 27 succeeds** (`h < 120 && h < 180`) → `m, c+m, x+m` | `err_h_negative_takes_quirk_branch` |
| E8 | `h >= 360.0f` (`360.0`, `1e30`, `+INF`) | all six branches fail → final `else` → `m, m, m` | `err_h_ge_360_and_inf` |
| E9 | `h = -INF` | line 27 quirk branch (`-INF < 120 && -INF < 180`) | `err_h_neg_inf` |
| E10 | `l = ±INF` / `l = NaN` | no check; `c`, `m` become `±INF`/`NaN`, stored verbatim | `err_l_inf_and_nan` |
| E11 | `s = ±INF` | no check; `c = (1-|2l-1|)*INF` → `±INF` or `NaN` (when the first factor is `0`) | `err_s_inf` |
| E12 | all three inputs NaN with *distinct* payloads / signs | payload+sign of the surviving NaN must match bit-for-bit | `err_nan_payload_propagation` |
| E13 | `dest == src` (fully aliased in/out buffers) | C reads all of `src[0..3]` into locals before any store, so aliasing is benign and well-defined | `err_dest_aliases_src` |
| E14 | `dest == src + 1` / `src - 1` (partially overlapping buffers) | same reasoning — all loads precede all stores | `err_partial_overlap` |
| E15 | out-of-range *enum* value across FFI | **N/A** — the API has no `enum`, no flag, and no `int` parameter; the only parameters are two `float*`. Nothing to probe. | — |
| E16 | null `dest` or null `src` | **Undefined behaviour in C** (unconditional dereference of `src[0]`, unconditional store to `dest[0]`); the C segfaults. Not a testable "same error code" case, and the Rust is documented with the identical safety contract. Deliberately **not** invoked. | *documented, not executed* |
| E17 | buffer shorter than 3 floats | Undefined behaviour in C for the same reason as E16 — no length parameter exists to validate. Not executed. | *documented, not executed* |
