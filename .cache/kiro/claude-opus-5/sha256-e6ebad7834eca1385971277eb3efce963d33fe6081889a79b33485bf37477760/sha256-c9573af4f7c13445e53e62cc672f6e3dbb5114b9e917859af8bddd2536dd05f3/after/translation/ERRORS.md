# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical inventory of the C source

Grep for every rejection / error construct:

| construct | occurrences in `c_src/` |
|---|---|
| `RETURN_ERROR`-style macro | 0 (none defined) |
| `assert` / `NDEBUG` | 0 |
| `return NULL` | 0 |
| `return -1` / negative sentinel | 0 |
| error `enum` / `typedef enum` | 0 (**no enums exist in this API at all**) |
| explicit NULL check | 0 |
| named min/max constant (`#define`) | 0 |
| `return 0` (early-out / rejection) | **1** — `get_bits`, line 8 |
| conditional branch that *skips* work | 3 — `if (ba != 0)`, `if (ba < 17)`, `while ((shl -= 8) > 0)` |
| loop guard that can reject all work | 3 — `j < 4`, `i < 2 * sci->total_bands`, `k < group_size` |
| unconditional success return | 1 — `return group_size * 4;` |

**Key finding:** `dequantize_granule` has **no error return path at all**. It
always returns `group_size * 4`. The *entire* error surface of this library is
(a) `get_bits`'s bitstream-exhaustion early-out, (b) branches that skip work,
and (c) undefined-behaviour classes the C nevertheless executes deterministically
at `-O0` (out-of-bounds `bitalloc` reads, over-wide shifts, signed overflow,
unsigned wraparound). Each of these is a row below because the Rust must
reproduce it bit-for-bit.

There are **no C enums** in the public header, so "out-of-range enum value across
FFI" has no direct instance. Its closest analogue — an integer field with no
valid variant — is `bitalloc[i]`, a `uint8_t` whose meaningful domain the code
splits at `0`, `1..16`, `17..`; values `17..255` include many that a real MPEG
bitstream never produces. Rows 9–13 cover that full `0..=255` domain.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `get_bits` | `bs->limit == 0`, `bs->pos == 0`, any `n > 0` → `(bs->pos += n) > bs->limit` | returns `0`; `bs->pos` **is still advanced** by `n`; `bs->buf` is **not read** | `row01_limit_zero_rejects_every_read` | [x] |
| 2 | `get_bits` | `bs->pos > bs->limit` already on entry (exhausted stream, e.g. after row 1) | every subsequent call returns `0` and keeps advancing `bs->pos`; `dequantize_granule` therefore writes `-half` (linear path) / `0 - mod/2` (grouped path) | `row02_pos_already_past_limit` | [x] |
| 3 | `get_bits` | `bs->pos + n == bs->limit` exactly (one step *inside* the range) | **no** early-out — the read is performed; boundary is `>`, not `>=` | `row03_row04_limit_boundary_is_strictly_greater` | [x] |
| 4 | `get_bits` | `bs->pos + n == bs->limit + 1` (one step past the valid range) | early-out, returns `0` | `row03_row04_limit_boundary_is_strictly_greater` | [x] |
| 5 | `get_bits` | grouped path yields a huge `n`: `ba` with `(ba-17)&31 == 24` → `mod == 0x0200_0001` → `n == 29_360_131`, against `limit == 1000` | every call returns `0`; `bs->pos` ends at exactly `16 * n`; buffer never read | `row05_huge_grouped_n_is_rejected` | [x] |
| 6 | `get_bits` | `bs->limit < 0` (`-1`, `-1000`, `INT_MIN+1`, `INT_MIN`) | first call already early-outs, returns `0` | `row06_negative_limit` | [x] |
| 7 | `get_bits` | `bs->pos < 0` (`-64`, `-1`) and `bs->pos + n <= bs->limit` | `bs->pos >> 3` is an **arithmetic** shift → `p = bs->buf + negative` reads *before* the buffer; `s = bs->pos & 7` is still `0..7` | `row07_negative_pos_reads_before_buffer` | [x] |
| 8 | `get_bits` | `n + s >= 40`, i.e. loop reaches `shl >= 32` in `cache \|= next << shl` (`ba == 22` → `mod == 65` → `n == 59`) | shift-count overflow (UB). At `-O0` gcc emits `shl %cl`, so the count is **masked to 5 bits**; the hand-computed sample value is `28.0` | `row08_over_wide_shift_is_masked` | [x] |
| 9 | `dequantize_granule` | `bitalloc[i] == 0` | band is **skipped entirely**: `grbuf` untouched, `bs->pos == 0`, but `dst += choff; choff = 18 - choff` still executes | `row09_zero_bitalloc_skips_band` | [x] |
| 10 | `dequantize_granule` | `bitalloc[i] == 17` (first grouped value) | `mod = (2 << 0) + 1 = 3`, `n = 3 + 2 - 0 = 5` | `row10_ba_17_smallest_grouped` | [x] |
| 11 | `dequantize_granule` | `bitalloc[i]` with `(ba-17)&31 == 30` (`ba == 47, 79, 111, 143, 175, 207, 239`) | `2 << 30` **overflows `int`** (UB) → `0x80000000` → `+1` → `mod == 0x80000001u`; `mod/2 == 0x40000000`; every sample is `-1_073_741_824.0` | `row11_signed_overflow_in_mod` | [x] |
| 12 | `dequantize_granule` | `bitalloc[i]` with `(ba-17)&31 == 31` (`ba == 48, 80, …, 240`) | `2 << 31` == `0` (masked shift) → `mod == 1` → `code % 1 == 0`, `mod/2 == 0` → every `dst[k] = 0.0f`; `n = 3` | `row12_mod_one_yields_zero_samples` | [x] |
| 13 | `dequantize_granule` | `bitalloc[i] >= 49` up to `255` — the *shift count* `ba-17` exceeds 31 (UB) | count masked to 5 bits, so behaviour is **periodic with period 32** in `ba`; asserted by comparing `ba` and `ba + 32k` byte-for-byte for every residue; no rejection, no error code | `row13_shift_count_wraps_with_period_32` | [x] |
| 14 | `dequantize_granule` | grouped path where `code % mod < mod / 2` | `code % mod - mod / 2` is computed in **`unsigned`** and cast to `int`; both signs appear and every sample lies in `[-mod/2, mod-1-mod/2]` | `row14_unsigned_difference_cast_to_int` | [x] |
| 15 | `dequantize_granule` | `sci->total_bands` such that `i >= 64` (any `total_bands >= 33`) | `sci->bitalloc[i]` reads **out of bounds**: `i` in `64..127` reads `scfcod[i-64]`, `i` in `128..129` the trailing padding, `i >= 130` **past the end of the object** (max `i` = `509` → offset `1279`, 380 B past `sizeof == 900`). Proven by placing the allocation only at index ≥ 64 / ≥ 130 and checking the exact bit consumption. No bounds check, no error | `row15_out_of_bounds_bitalloc_read` | [x] |
| 16 | `dequantize_granule` | `sci->total_bands == 0` | `i` loop never entered for any `j`; returns `group_size * 4`; `grbuf` untouched; `bs->pos == 0`; `choff` stays `576` | `row16_total_bands_zero` | [x] |
| 17 | `dequantize_granule` | `group_size == 0` | `k` loop never entered → no writes. **Linear** path consumes nothing; **grouped** path still consumes `n` bits per band because `get_bits` precedes the `k` loop. Returns `0` | `row17_group_size_zero` | [x] |
| 18 | `dequantize_granule` | `group_size < 0` (`-1`, `-2`, `-18`, `-576`, `-100000`, `-0x40000000`, `INT_MIN`, `INT_MIN+1`) | `k < group_size` false → no writes; `dst = grbuf + group_size * j` is a wild (negative) pointer but never dereferenced; returns `group_size * 4` **wrapped** (`INT_MIN * 4 == 0`) | `row18_negative_group_size` | [x] |
| 19 | `dequantize_granule` | `grbuf == NULL` with `total_bands == 0`, all-zero `bitalloc`, `group_size == 0` (both paths), or `group_size < 0` (both paths) | no dereference occurs → returns `group_size * 4` without faulting | `row19_null_grbuf_when_never_dereferenced` | [x] |
| 20 | `dequantize_granule` | `bs == NULL` with `total_bands == 0`, all-zero `bitalloc`, or a linear path with `group_size <= 0`; also both pointers null at once | `bs` never dereferenced → returns `group_size * 4` without faulting | `row20_null_bs_when_never_dereferenced` | [x] |
| 21 | `dequantize_granule` | `group_size * 4` overflows `int` (`0x40000000`, `0x20000000`, `0x60000000`, `0x7FFFFFFF`, `INT_MIN`) | signed overflow (UB); at `-O0` wraps — `0x40000000*4 == 0`, `0x20000000*4 == INT_MIN`. Tested with `total_bands == 0` and `grbuf == NULL` so no 4 GiB buffer is needed | `row21_return_value_overflow` | [x] |
| 22 | `get_bits` | `bs->pos & 7 != 0` (all of `s = 0..7`) | first byte is masked with `255 >> s`, discarding the `s` already-consumed high bits; the resulting sample is hand-recomputed for each `s` | `row22_unaligned_first_byte_is_masked` | [x] |

All rows are implemented in `tests/phase_c_error_paths.rs`, which asserts both
that the two libraries agree **and** that the result is the specific documented
sentinel — not merely that both "failed somehow".

### Generic boundaries beyond the table

`generic_boundaries_one_step_past_ranges` additionally covers, for both
implementations:

* `ba` at `0, 1, 15, 16, 17, 18` — one step either side of the `ba != 0` and
  `ba < 17` branch boundaries;
* `total_bands` at `0, 1, 32, 33, 64, 65, 129, 130, 254, 255` — one step either
  side of the `bitalloc` array end (32/33) and the struct end (129/130);
* `group_size` at `0, 1, 2, 575, 576, 577, 1024` — zero, one, and oversized;
* `bs->limit` at exactly `n` and `n - 1` for every linear `ba` in `1..=16`;
* `bs->pos` at `-400000, -100000, -1, 0, 1, 500000`.

`generic_boundaries_full_uint8_domains` sweeps the **entire** `0..=255` domain of
both `uint8_t` mode selectors (`bitalloc[i]` and `total_bands`) across the FFI
boundary. The public header declares **no `enum`**, so there is no literal
"out-of-range enum variant" to pass; `bitalloc[i]` is the closest analogue (a
C-side mode selector whose meaningful values are `0`, `1..16`, `17..`, but which
accepts any of 256 byte values), and every one of its 256 values is tested — as
is every `total_bands`, including the values that push the read 380 bytes past
the end of the struct.

### Rows deliberately NOT differential-tested (documented instead)

* **`group_size * 4` overflow with real writes** — needs a >4 GiB buffer. The
  *return value* half of row 21 **is** tested.
* **`sci == NULL`** — `2 * sci->total_bands` is evaluated as the very first loop
  condition, so *both* C and Rust segfault unconditionally. A crash is not a
  comparable "same error code" result and there is no branch to verify. Noted
  for completeness, not tested.
* **`k >= 25` grouped reads with a finite `limit`** — `bs->pos += n` is
  unconditional, so two such calls overflow `int` and the **C itself** then
  dereferences `bs->buf + 234 MB`. Rows 11 and 63–68 cover this `ba` range with
  `limit = INT_MIN`, which keeps `get_bits` on its early-out path; see the note
  in `CONFIGS.md`.
