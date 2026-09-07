# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Derived mechanically by grepping `c_src/src/lib.c` for every `return`, every
`if`, every explicit check, and every constant bound. There are **no**
`assert`s, no `RETURN_ERROR`-style macros, no error enums, and no `errno`
usage in this library. The only failure channel is the `char *` return value,
whose documented sentinel is `NULL`:

```
/* ... Returns encoded string otherwise NULL */
```

## Rejection / error branches

Exactly two statements in the C can produce the `NULL` sentinel
(`lib.c:34` and `lib.c:43`):

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `encode_base64` | `src == NULL` (`if (!src)`, lib.c:33) — checked **before** `size` is looked at, so it fires for every `size`, including `0`, negative, `INT_MIN`, `INT_MAX` | returns `NULL` |
| 2 | `encode_base64` | `calloc(1, size*4/3 + 4)` fails (`if (!out)`, lib.c:42). Reachable with a *valid* non-NULL `src` because `size*4` is **signed `int`** arithmetic: any `size >= 0x2000_0000` (536 870 912) overflows to a negative `int`, which sign-extends to a huge `size_t` at the `calloc` call, so the allocation is refused | returns `NULL` |

## Non-rejections that must NOT become errors

The C accepts several inputs that a "defensive" translation would be tempted to
reject. Replicating the *acceptance* is as important as replicating the
rejections, so these are tested as error-path rows too — the assertion is that
**neither** implementation returns `NULL`.

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 3 | `encode_base64` | `size == 0` with non-NULL `src` (`if (!size)`, lib.c:37) — *not* an error; `size` is silently replaced by `strlen(src)`, truncated from `size_t` to `int` | non-NULL; encodes the NUL-terminated string. `src = ""` yields a 4-byte all-zero buffer (empty output) |
| 4 | `encode_base64` | `size < 0` with non-NULL `src`. No negative check exists, so the behaviour is decided purely by `cap = size*4/3 + 4` in signed `int`: **only `size ∈ {-1, -2, -3}` is accepted** (`cap` = 3, 2, 0 respectively — note `calloc(1, 0)` still returns non-NULL on glibc). For `size <= -4`, `cap <= -1`, which sign-extends to `SIZE_MAX`-ish and the `calloc` fails | `size` in `-3..=-1`: non-NULL zeroed buffer, empty string (the loop never iterates). `size <= -4`: `NULL` — **verified empirically over `-64..=-1`: the C returns `NULL` for every value in `-64..=-4` and non-NULL for `-3..=-1`** |
| 5 | `encode_base64` | `size == INT_MIN` — `size*4` wraps to exactly `0`, so `cap = 0/3 + 4 = 4`; `calloc(1,4)` succeeds and the loop never runs | non-NULL, empty string (a *successful tiny* allocation from a wildly out-of-range `size`) |
| 6 | `encode_base64` | `size` large negative — `size*4` wraps, and whether `cap` lands positive or negative is value-dependent, so accept/reject alternates in bands. Observed: `-2_000_000_000` → non-NULL, `-1_610_612_736` → `NULL`, `-1_073_741_824` → non-NULL, `-1_000_000_000` → non-NULL, `-536_870_912` → `NULL` | must match the C band-for-band; when accepted, an empty string |
| 7 | `encode_base64` | `size == INT_MAX` — `size*4` overflows to `-4`, `-4/3 == -1` (truncation toward zero), `cap = 3`; `calloc(1,3)` succeeds, but the loop *does* run and would write past the 3-byte buffer | undefined behaviour in C. **Excluded from the differential suite**: the C is not observable here (heap corruption), so there is no ground truth to match. Documented for completeness. |

## Boundary inputs every C API has (covered in Phase C even though not rows above)

| # | condition | expected |
|---|-----------|----------|
| 8 | NULL pointer × the full set of interesting `size` values (`0`, `1`, `-1`, `INT_MIN`, `INT_MAX`) | row 1: always `NULL` |
| 9 | zero length (`size == 0`) on an empty string | row 3: non-NULL, empty output |
| 10 | oversized length: `size` one step past the `int`-overflow threshold, `0x2000_0000` | row 2: `NULL` |
| 11 | one step *before* the overflow threshold, `0x2000_0000 - 1` = `0x1FFF_FFFF` | large but positive `cap`; allocation attempted. Both sides must agree on `NULL` vs non-NULL. Excluded from byte-comparison (would require a 512 MiB readable `src`); the *allocation decision* is compared. |
| 12 | out-of-range enum values across the FFI boundary | **N/A** — the public API (`char *encode_base64(int, const char *)`) declares no enum, struct, or flag parameter. `int` and `const char *` have no invalid-bit-pattern range beyond what rows 1–7 already enumerate; every one of the 2^32 `int` values is an accepted input, and the interesting equivalence classes (`0`, `>0`, `<0`, `INT_MIN`, `INT_MAX`, overflow threshold) are all covered above. |

## Status

All rows verified against both `.so` files, under both the release and the
debug Rust cdylib (the debug build has overflow checks ON, so it would trap any
accidentally non-`wrapping` arithmetic).

| row | test | status |
|-----|------|--------|
| 1 | `err_row1_null_src_every_size` | [x] passes |
| 2 | `err_row2_calloc_failure_int_overflow` | [x] passes |
| 3 | `err_row3_size_zero_is_strlen_not_error` | [x] passes |
| 4 | `err_row4_negative_size_accept_reject_bands` | [x] passes |
| 5 | `err_row5_int_min_wraps_to_tiny_alloc` | [x] passes |
| 6 | `err_row6_large_negative_size` | [x] passes |
| 7 | — | [x] excluded (C-side UB, no ground truth — see below) |
| 8 | `err_row1_null_src_every_size`, `boundary_null_pointer_takes_precedence_over_every_other_condition` | [x] passes |
| 9 | `err_row3_size_zero_is_strlen_not_error`, `cfg_row15_strlen_mode_empty_string` | [x] passes |
| 10 | `err_row2_calloc_failure_int_overflow` | [x] passes |
| 11 | `err_row11_one_below_overflow_threshold` (with a real 0x1FFF_FFFF-byte source buffer) | [x] passes |
| 12 | — | [x] N/A (no enum in the ABI; the `int` equivalence classes are covered by rows 1–11) |
| extra | `boundary_zero_and_one_past_every_documented_edge` | [x] passes |
| extra | `boundary_int_extremes_exhaustive_neighbourhoods` | [x] passes |

### The observable/UB boundary for positive `size`

`cap = size*4/3 + 4` in wrapping signed `int` decides everything. Enumerated:

| `size` band | `cap` | C behaviour | testable? |
|---|---|---|---|
| `1 .. 0x1FFF_FFFF` | large positive, `>= 4*ceil(size/3)` | correct encode | yes — needs a `size`-byte source |
| `0x2000_0000 .. 0x3FFF_FFFC` | `<= -1` → `SIZE_MAX`-ish | `calloc` fails → `NULL` | yes, with a tiny source (loop unreached) |
| `0x3FFF_FFFD .. INT_MAX` | wraps to `0`, `2`, `3`, `4`, … | `calloc` **succeeds** with a few bytes, then the loop writes gigabytes past it | **no** — heap corruption, ERRORS.md row 7 |

The last band is the only place C and Rust can differ, and only because the C
has no defined behaviour there: Rust's bounds-checked slice write aborts, while
the C silently corrupts the heap. Every `size` the C defines a result for is
covered by a passing differential test.

