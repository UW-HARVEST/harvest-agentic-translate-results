# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c` (34 lines) and `c_src/include/lib.h`.

## Mechanical grep results

```
$ grep -nE 'RETURN_ERROR|return -1|return NULL|assert|errno|if *\(' c_src/src/lib.c
4:    if (sample >= 32766.5)
6:    if (sample <= -32767.5)
```

* Error-return macros (`RETURN_ERROR`, …): **none**
* `return -1` / `return NULL` / error enums: **none** (`synth_pair` returns `void`)
* `assert` / `abort` / `errno`: **none**
* Null-pointer checks: **none**
* Range checks on `nch`: **none**
* Explicit numeric range/limit constants: `32766.5`, `-32767.5` (line 4, 6),
  and the saturation results `32767`, `-32768` (line 5, 7).

So the library has **no error-return surface**. Its entire "rejection" surface is
*value clamping* inside `mp3d_scale_pcm`, plus the out-of-contract inputs an
external FFI caller can legally pass across the boundary. Each such condition is
one row below, and each row is asserted to produce the **same 16-bit sample
value(s)** (or the same observable non-behaviour) from C and Rust.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[0]`) | accumulator `a >= 32766.5` (line 4 taken) | returns `32767` (`INT16_MAX`); `pcm[0] == 32767` | [x] |
| 2 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[0]`) | accumulator `a <= -32767.5` (line 6 taken) | returns `-32768` (`INT16_MIN`); `pcm[0] == -32768` | [x] |
| 3 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[16*nch]`) | second accumulator `a >= 32766.5` | `pcm[16*nch] == 32767` | [x] |
| 4 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[16*nch]`) | second accumulator `a <= -32767.5` | `pcm[16*nch] == -32768` | [x] |
| 5 | `mp3d_scale_pcm` | `a` exactly `32766.5f` (boundary, `>=` is inclusive) | `32767` (clamp path, not round path) | [x] |
| 6 | `mp3d_scale_pcm` | `a` exactly `-32767.5f` (boundary, `<=` is inclusive) | `-32768` (clamp path) | [x] |
| 7 | `mp3d_scale_pcm` | `a` one ULP below `32766.5f` (`nextafter` down) | round path: `(int16_t)(a+.5f)` == `32766` | [x] |
| 8 | `mp3d_scale_pcm` | `a` one ULP above `-32767.5f` (`nextafter` up) | round path: `-32767` | [x] |
| 9 | `mp3d_scale_pcm` | `a < 0` after truncation → `s -= (s < 0)` fires (line 9) | result is `trunc(a+.5f) - 1` | [x] |
| 10 | `mp3d_scale_pcm` | `a` in `(-1.5, -0.5)` → `s == 0` from truncation, `s < 0` false | result `0`, **no** decrement (sign-of-zero edge) | [x] |
| 11 | `mp3d_scale_pcm` | `a == -0.0f` | both comparisons false, `-0.0+.5 = 0.5`, `s == 0`, result `0` | [x] |
| 12 | `mp3d_scale_pcm` | `a == NaN` (e.g. `z` contains NaN) | both `>=` and `<=` are false → `(int16_t)(NaN+.5f)`; on x86-64 `cvttss2si` yields `0x80000000`, narrowed to `int16_t` == `0`; `s<0` false → `0` | [x] |
| 13 | `mp3d_scale_pcm` | `a == +INFINITY` (`z` contains `+inf`) | line 4 taken → `32767` | [x] |
| 14 | `mp3d_scale_pcm` | `a == -INFINITY` (`z` contains `-inf`) | line 6 taken → `-32768` | [x] |
| 15 | `mp3d_scale_pcm` | `a == NaN` produced by `inf - inf` / `inf*0` inside the dot product | `0` (see row 12) | [x] |
| 16 | `synth_pair` | `nch == 0` → `pcm[16*nch]` aliases `pcm[0]`; the **second** store overwrites the first | `pcm[0]` holds the *second* accumulator's sample | [x] |
| 17 | `synth_pair` | `nch < 0` (e.g. `-1`, `-2`) — no validation, negative index `16*nch` | writes *before* `pcm`; caller must supply the buffer. Same negative offset from C and Rust | [x] |
| 18 | `synth_pair` | `nch` huge (`INT_MAX`, `INT_MIN`) → `16 * nch` overflows `int` in C | C: signed overflow (UB) computes wrapped `int` offset on x86-64 `lea`/`imul`; not exercised as a write. Only the *no-overflow* large-`nch` case is differentially tested (see CONFIGS row 12); `INT_MAX`/`INT_MIN` are documented, not stored to | n/a |
| 19 | `synth_pair` | `pcm == NULL` | no null check → segfault (UB). Rust likewise segfaults; identical *absence* of a check verified by inspection, not by a crashing test | n/a |
| 20 | `synth_pair` | `z == NULL` | no null check → segfault (UB) reading `z[14*64]`. Same in Rust | n/a |
| 21 | `synth_pair` | `z` array shorter than `2 + 14*64 + 1 = 899` floats | out-of-bounds read, no check. Both read the same offsets; verified by asserting the exact offset set touched (rows in CONFIGS) | [x] |
| 22 | `mp3d_scale_pcm` | subnormal / `FLT_MIN`-magnitude accumulator | round path, no clamp; result `0` (positive) or `0` (negative-subnormal, row 10) | [x] |

Rows 18–20 are true C undefined behaviour with no observable, testable return
value; they are recorded for completeness and the Rust code is confirmed by
inspection to contain the *same* (absent) checks. Every other row has an
executing differential test in `tests/differential.rs`.
