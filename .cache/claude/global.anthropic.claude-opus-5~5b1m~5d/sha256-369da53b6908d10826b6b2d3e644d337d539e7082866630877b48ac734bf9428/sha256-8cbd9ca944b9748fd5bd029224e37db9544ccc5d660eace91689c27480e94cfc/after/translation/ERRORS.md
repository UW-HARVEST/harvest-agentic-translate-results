# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c` (24 lines) and `c_src/include/lib.h`.

## Exhaustive grep of every rejection / error construct in the C source

```
$ grep -n 'return\|assert\|NULL\|RETURN_ERROR\|ERROR\|if\|while\|<\|>\|==\|!=' c_src/src/lib.c
11:    while ((bw->bits + bits >= (8 * sizeof(tflac_uint))) && i < 100)   # loop guard
13:    b = b > bits ? bits : b;                                           # clamp, not a rejection
23:    return 0;                                                          # the ONLY return
```

Findings:

* `return` statements in the whole library: **1** (`return 0`).
* `assert` / `NULL` checks / `RETURN_ERROR` macros / error enums / range
  validation of `bits` / capacity checks against `bw->len` / `bw->pos` /
  `bw->buffer`: **0** — the C performs **no input validation at all**.
* Therefore `bitwriter_add` has an *unconditional* success result and the
  "error surface" consists solely of implicit / UB-adjacent conditions whose
  observable behaviour still has to match byte-for-byte.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `bitwriter_add` | any well-formed call at all (no validation exists) | returns `0`; never a non-zero code | [x] |
| 2 | `bitwriter_add` | `bits == 0` → `val <<= (64 - 0)` = shift-by-64, C UB | compiled code emits `shlq %cl` → count masked to `& 63` = 0, so `val` unchanged; returns `0` | [x] |
| 3 | `bitwriter_add` | `bits > 64` (e.g. 65, 100, 0xFFFF_FFFF) → `64 - bits` underflows `tflac_u32` | `64 - bits` wraps mod 2^32, shift count masked `& 63`; loop runs and clamps `b` to `bits`; returns `0` | [x] |
| 4 | `bitwriter_add` | `bits == 64` → `val <<= 0`, `bw->bits + 64 >= 64` always true | enters loop; returns `0` | [x] |
| 5 | `bitwriter_add` | `bw->bits` already `>= 64` on entry → `val >> bw->bits` is shift-by->=64, C UB | `shr %cl` masks count `& 63`; `bw->bits` keeps growing with wrapping `u32` add; returns `0` | [x] |
| 6 | `bitwriter_add` | `bw->bits == 63` → `b = 64 - 63 - 1 = 0`, so `b` clamps to `0` and `bits` never decreases | loop spins until the `i < 100` guard stops it at exactly 100 iterations; returns `0` | [x] |
| 7 | `bitwriter_add` | `bw->bits > 63` → `64 - bw->bits - 1` underflows `tflac_u32` to a huge value, then clamped by `b > bits ? bits : b` | `b = bits`, one iteration consumes all bits; returns `0` | [x] |
| 8 | `bitwriter_add` | `bw->bits + bits` overflows `tflac_u32` (e.g. `bw->bits = 0xFFFF_FFFF`, `bits = 1` → `0`) | wraps to `0`, `0 >= 64` false, loop skipped; returns `0` | [x] |
| 9 | `bitwriter_add` | `bw->tot += bits` overflows `tflac_u32` | wraps mod 2^32; returns `0` | [x] |
| 10 | `bitwriter_add` | `bw->buffer` is `NULL` and/or `bw->len == 0` / `bw->pos > bw->len` | never dereferenced by this function → no crash, no error; `pos`/`len`/`buffer` left untouched; returns `0` | [x] |
| 11 | `bitwriter_add` | `bw == NULL` | C dereferences unconditionally → SIGSEGV. Rust `&mut *bw` likewise. **Not differentially tested** (a crash would abort the harness); documented as identical UB. | [n/a] |
| 12 | `bitwriter_add` | out-of-range "enum" value passed across FFI | **no enum exists** in the public header (`bits` is a plain `tflac_u32`, every one of the 2^32 values is accepted). Covered by rows 2–4 and the exhaustive `bits` sweep. | [x] |

## Notes on non-existent error paths

There is no `tflac_bitwriter_init`, no `flush`, no error enum, no return-code
constant other than the literal `0`, and no bounds check involving `pos`,
`len`, or `buffer` anywhere in `c_src/`. Any test asserting a non-zero return
would be asserting behaviour the C does not have; instead every error-path test
below asserts `rc_c == rc_rust == 0` **and** full struct equality, which is the
strictest observable contract this API offers.
