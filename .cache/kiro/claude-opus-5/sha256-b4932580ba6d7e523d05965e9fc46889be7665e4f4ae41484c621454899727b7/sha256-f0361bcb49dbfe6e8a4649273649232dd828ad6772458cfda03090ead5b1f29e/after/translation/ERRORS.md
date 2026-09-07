# ERRORS.md — Phase C error-surface table

## How this table was derived

Mechanical grep of the whole C source for every rejection construct:

```
$ grep -nE "return|assert|NULL|errno|RETURN_ERROR|if *\(|switch|enum|#ifdef|#if |min|max|MIN|MAX|<=|>=|!=|==" \
      include/lib.h src/lib.c
NO MATCHES AT ALL
```

**`premultiply` has no explicit error surface at all**: it returns `void`, has no
`assert`, no null check, no range check, no error enum, and no named min/max
constant. It cannot report failure to its caller.

Its *implicit* rejection surface therefore comes from exactly two places, and
every row below is derived from one of them:

1. **The loop guard** `i < (int)stride * h` (asm: 32-bit `imul` + signed `jl`,
   recomputed every iteration at `-O0`). Whenever the bound is `<= 0` the
   function silently performs no work — this is the only way the C "rejects" an
   input. The bound is computed with wrapping 32-bit arithmetic:
   `stride = wrap32(w << 2)`, `end = wrap32(stride * h)`,
   `iterations = if end > 0 { end / 4 } else { 0 }`.
2. **Unchecked dereferences** `img->w` and `data[i+k]`, which fault (SIGSEGV)
   rather than returning an error.

Note on out-of-range enum values: **the API declares no enums**, so there is no
enum variant to overflow. The equivalent "any `int` is a legal FFI input" surface
is the `int w` / `int h` fields, and rows 9–20 cover them at `INT_MIN`,
`INT_MAX`, the wrap boundaries `2^29`/`2^30`, and one step past each.

`sizeof(cp_pixel_t) == 4` throughout. "no-op" below means: 0 loop iterations,
buffer bytes bit-identical to the input, function returns normally.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `premultiply` | `img == NULL` | no null check → deref of `img->w` faults → SIGSEGV (signal 11); both libs must die the same way |
| 2 | `premultiply` | `img->pix == NULL`, `w = 4`, `h = 4` (bound `> 0`) | no null check → `data[3]` read faults → SIGSEGV (signal 11) |
| 3 | `premultiply` | `img->pix == NULL`, `w = 0`, `h = 4` (bound `== 0`) | bound is 0, loop body never runs, `pix` never dereferenced → returns normally, **no crash** |
| 4 | `premultiply` | `img->pix == NULL`, `w = 4`, `h = 0` | same: `end = 0` → returns normally, no crash |
| 5 | `premultiply` | `w = 0`, `h > 0`, valid `pix` | `stride = 0` → `end = 0` → no-op |
| 6 | `premultiply` | `h = 0`, `w > 0`, valid `pix` | `end = 0` → no-op |
| 7 | `premultiply` | `w = 0`, `h = 0` | `end = 0` → no-op |
| 8 | `premultiply` | `w < 0`, `h > 0` (e.g. `w = -4`, `h = 4`) | `stride = -16`, `end = -64 < 0` → no-op |
| 9 | `premultiply` | `w > 0`, `h < 0` (e.g. `w = 4`, `h = -4`) | `stride = 16`, `end = -64 < 0` → no-op |
| 10 | `premultiply` | `w < 0` **and** `h < 0` (e.g. `w = -2`, `h = -3`) | quirk: two negatives → `end = (-8) * (-3) = 24 > 0` → **loop RUNS**, 6 pixels premultiplied forward from `pix`. Must NOT be "fixed". |
| 11 | `premultiply` | `w = 0x40000000` (2^30), `h = 1` | `w << 2` wraps to `0` → `end = 0` → no-op (no OOB access despite the huge nominal size) |
| 12 | `premultiply` | `w = 0x20000000` (2^29), `h = 1` | `stride = wrap32(2^31) = INT_MIN` → `end = INT_MIN < 0` → no-op |
| 13 | `premultiply` | `w = 0x20000000`, `h = 2` | `stride = INT_MIN`, `wrap32(INT_MIN * 2) = 0` → no-op |
| 14 | `premultiply` | `w = 0x20000000`, `h = 3` | `stride = INT_MIN`, `wrap32(INT_MIN * 3) = INT_MIN < 0` → no-op |
| 15 | `premultiply` | `w = INT_MAX`, `h = 1` | `stride = wrap32(0x1FFFFFFFC) = -4` → `end = -4 < 0` → no-op |
| 16 | `premultiply` | `w = INT_MAX`, `h = -1` | `stride = -4`, `end = 4 > 0` → **loop RUNS exactly 1 iteration** (pixel 0 only) |
| 17 | `premultiply` | `w = INT_MIN`, `h = 1` | `stride = wrap32(INT_MIN << 2) = 0` → `end = 0` → no-op |
| 18 | `premultiply` | `w = INT_MIN`, `h = -1` / `h = INT_MIN` | `stride = 0` → `end = 0` → no-op |
| 19 | `premultiply` | `w = 1`, `h = INT_MAX` | `stride = 4`, `wrap32(4 * INT_MAX) = -4 < 0` → no-op |
| 20 | `premultiply` | `w = 1`, `h = INT_MIN` | `stride = 4`, `wrap32(4 * INT_MIN) = 0` → no-op |
| 21 | `premultiply` | `w = 1000`, `h = 1000000` — `stride * h` overflows `int` | `4000 * 1000000 = 4e9` wraps to `-294967296 < 0` → no-op. A non-wrapping impl would run 10^9 iterations and segfault. |
| 22 | `premultiply` | `w = 65536`, `h = 16385` — overflow wrapping to a *small positive* bound | `stride = 262144`; `wrap32(262144 * 16385) = 262144` → **65536 iterations only** (not 2^30), touching bytes `0..262143` |
| 23 | `premultiply` | `w = 3`, `h = 5` but `pix` buffer holds only 1 pixel (undersized buffer) | no bounds check → reads/writes `60` bytes past the logical end (UB in C). Both libs must touch the **identical byte span with identical values**; tested against an over-allocated arena so the access stays mapped. |
| 24 | `premultiply` | oversized length: `w = 0x7FFFFFFE`, `h = 0x7FFFFFFE` | `stride = wrap32(-8) = -8`; `wrap32(-8 * 0x7FFFFFFE) = 16` → 4 iterations only |
| 25 | `premultiply` | **misaligned `img`**: a `cp_image_t *` at an address not 8-byte (nor 4-byte) aligned, shifted by 1..7 bytes, with otherwise valid contents | gcc reads `img->w`/`img->h`/`img->pix` with plain `mov`s and no alignment check, so on x86-64 the fields load correctly and the call succeeds normally. The Rust must be no stricter than the hardware. |

## Divergences found and fixed

Both were found by Phase C rows, both were invisible to the happy path, and both
appeared **only under the debug cdylib** — the release build happened to match,
which is exactly why the suite is run against both profiles.

| row | symptom | cause | fix |
|-----|---------|-------|-----|
| 1 | C died with **SIGSEGV (11)**, Rust with **SIGABRT (6)** on `img == NULL` | `(*img).w` in Rust emits a debug-only `ub_checks` null/alignment assertion; the panic cannot unwind across `extern "C"` so it aborts. The C has no check and simply faults. | read the header fields with unchecked byte-wise volatile loads (`load_bytes` in `src/lib.rs`) |
| 25 | Rust aborted with `ptr::read_volatile requires that the pointer argument is aligned` where the C succeeded | the first fix used `ptr::read_volatile::<c_int>`, which carries a debug alignment precondition the C does not have. `ptr::read_unaligned` was not an option either — it carries a **non-null** precondition, which would have re-broken row 1. | load one `u8` at a time: alignment 1 is satisfied by every address and a `u8` volatile read performs no null check, so both row 1 and row 25 match |
| — | (test-side, not a translation defect) `ERRORS.md` row 21's original example `100000 x 100000` was wrong: it wraps to a **positive** bound of `1345294336` → 336,323,584 iterations, not a no-op. | bad hand-derivation | row 21 now uses only verified negative-wrap examples; the large-positive-wrap case is covered by `tests/phase_c_large.rs::overflow_to_large_positive_bound` |

## Checklist

- [x] 1 — `img == NULL` → same fatal signal (subprocess-compared)
- [x] 2 — `pix == NULL` with positive bound → same fatal signal
- [x] 3 — `pix == NULL`, `w = 0` → both return normally
- [x] 4 — `pix == NULL`, `h = 0` → both return normally
- [x] 5 — `w = 0` → no-op
- [x] 6 — `h = 0` → no-op
- [x] 7 — `w = h = 0` → no-op
- [x] 8 — `w < 0`, `h > 0` → no-op
- [x] 9 — `w > 0`, `h < 0` → no-op
- [x] 10 — `w < 0` and `h < 0` → loop runs (quirk preserved)
- [x] 11 — `w = 2^30` → stride wraps to 0
- [x] 12 — `w = 2^29`, `h = 1` → stride `INT_MIN`
- [x] 13 — `w = 2^29`, `h = 2` → bound wraps to 0
- [x] 14 — `w = 2^29`, `h = 3` → bound `INT_MIN`
- [x] 15 — `w = INT_MAX`, `h = 1` → no-op
- [x] 16 — `w = INT_MAX`, `h = -1` → exactly 1 iteration
- [x] 17 — `w = INT_MIN`, `h = 1` → no-op
- [x] 18 — `w = INT_MIN`, `h < 0` → no-op
- [x] 19 — `w = 1`, `h = INT_MAX` → no-op
- [x] 20 — `w = 1`, `h = INT_MIN` → no-op
- [x] 21 — `int` overflow to negative bound → no-op
- [x] 22 — `int` overflow to small positive bound → truncated iteration count
- [x] 23 — undersized buffer → identical OOB span
- [x] 24 — both dimensions oversized → 4 iterations
- [x] 25 — misaligned `img` pointer → both succeed, same values

## Test mapping

Every row above is checked by a test in `tests/phase_c_errors.rs` named
`errNN_*` (rows 1–24) plus `generic_misaligned_image_struct_pointer` (row 25),
and the generic boundary sweeps `generic_every_dimension_boundary_and_one_past`
(2229 in-process `(w,h)` combinations, every `int` boundary and ±1/±2 around it)
and `generic_null_pix_across_all_zero_bound_shapes`. The 172 combinations whose
wrapped bound exceeds 1 Mi pixels are covered by `tests/phase_c_large.rs`, which
runs every one of their 16 distinct trip counts up to 536,870,911 pixels.

Rows 1 and 2 compare the exact terminating **signal** of a re-exec'd subprocess
for each library, not merely "both failed".
