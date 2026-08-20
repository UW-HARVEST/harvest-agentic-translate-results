# CONFIGS.md — Phase A: configuration-surface table

Derived mechanically from `c_src/include/lib.h` (the whole public API) and the
branches in `c_src/src/lib.c`.

## Full set of public entry points

`lib.h` declares exactly one function. There is no convenience wrapper / one-shot
layer above it and no lower layer beneath it — this *is* the lowest-level entry
point:

| entry point | signature |
|---|---|
| `flip_horizontal` | `void flip_horizontal(cp_image_t *img)` |

## Runtime options / modes / flags

There are **no** option structs, mode enums, flags, setters, or `#ifdef`s
(grep for `enum|switch|#ifdef|#if |flag|option|mode` in the header and source
returns nothing). The *entire* configuration surface is therefore the shape of
the single `cp_image_t` input plus the contents of the pixel buffer it points
at. The axes the C code actually branches on are:

| axis | where the C branches on it | distinct values the code treats differently |
|---|---|---|
| `h` → `flips = h/2` (lib.c:7) and guard `i < flips` (lib.c:8) | outer loop trip count | `h<0` (flips<0, 0 trips) · `h==0`/`h==1` (flips==0, 0 trips) · `h==2` (1 trip) · `h` even>2 · `h` odd>2 (middle row untouched) · `h==INT_MIN` · `h==INT_MAX` |
| `w` → guard `j < w` (lib.c:11) | inner loop trip count | `w<0` (0 trips) · `w==0` (0 trips) · `w==1` (1 trip) · `w>1` · `w==INT_MIN`/`INT_MAX` |
| `w` → `pix + w*i`, `pix + w*(h-i-1)` (lib.c:9-10) | row base addresses, `int` multiply that can overflow | small `w` · large `w` · negative `w` · overflowing `w*i` |
| `h` parity | whether a middle row exists | even (all rows moved) · odd (row `h/2` untouched) |
| `img->pix` | dereferenced at lib.c:12-14 only if both loops are entered | non-null · null-and-never-dereferenced · null-and-dereferenced (→ ERRORS.md 2) |
| pixel byte values (`r`,`g`,`b`,`a`) | copied verbatim by the struct assignment at lib.c:12-14 | randomized over the full `u8` range on all 4 channels |
| buffer length vs `w*h` | no check exists | exactly `w*h` · larger than `w*h` (canary padding must stay pristine) |
| number of calls | function is a pure in-place permutation | 1 call · 2 calls (must restore the original) · 3 calls |

Every row below is a combination of those axes that the C distinguishes. Each is
driven through **both** `.so`s via `libloading` and compared byte-for-byte;
every row marked "randomized" runs many pseudo-random inputs from a fixed seed
(deterministic `SplitMix64`), not one hand-picked value. Rows that reduce to a
no-op are the (S) rows of `ERRORS.md` and are cross-referenced there.

## Configuration-surface table

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `flip_horizontal` | `h==2, w==1`, exact buffer — smallest input that performs work; randomized pixels | `cfg01_min_working` | [x] |
| 2  | `flip_horizontal` | `h==2, w>1` randomized (w in 2..=64), exact buffer, randomized pixels — single row swap | `cfg02_h2_w_random` | [x] |
| 3  | `flip_horizontal` | `h==3` (odd, middle row must be untouched), `w` randomized, randomized pixels | `cfg03_h3_odd_middle_untouched` | [x] |
| 4  | `flip_horizontal` | `h` even in 4..=64, `w` randomized 1..=64, randomized pixels | `cfg04_h_even_random` | [x] |
| 5  | `flip_horizontal` | `h` odd in 5..=65, `w` randomized 1..=64, randomized pixels | `cfg05_h_odd_random` | [x] |
| 6  | `flip_horizontal` | `w==1`, `h` randomized 2..=128 — one pixel per row (degenerate stride) | `cfg06_w1_tall` | [x] |
| 7  | `flip_horizontal` | `h==2`, `w` large (512..=2048) — wide rows, many inner iterations | `cfg07_wide_rows` | [x] |
| 8  | `flip_horizontal` | large area both ways (`w*h` up to ~64K px), randomized | `cfg08_large_area` | [x] |
| 9  | `flip_horizontal` | `h==0`, `pix` non-null, exact-size buffer → no-op (ERRORS 4) | `cfg09_h_zero_valid_pix` | [x] |
| 10 | `flip_horizontal` | `h==0`, `pix == NULL` → no-op, no deref (ERRORS 4/14) | `cfg10_h_zero_null_pix` | [x] |
| 11 | `flip_horizontal` | `h==1`, `pix` non-null → no-op (ERRORS 5) | `cfg11_h_one_valid_pix` | [x] |
| 12 | `flip_horizontal` | `h==1`, `pix == NULL` → no-op (ERRORS 14) | `cfg12_h_one_null_pix` | [x] |
| 13 | `flip_horizontal` | `w==0`, `h` randomized ≥2, `pix` non-null → outer loop spins, inner never (ERRORS 10) | `cfg13_w_zero_valid_pix` | [x] |
| 14 | `flip_horizontal` | `w==0`, `h` randomized ≥2, `pix == NULL` → no-op (ERRORS 10) | `cfg14_w_zero_null_pix` | [x] |
| 15 | `flip_horizontal` | `w<0` randomized, `h` randomized ≥2, `pix` non-null → out-of-range pointers computed, never dereferenced (ERRORS 11/12) | `cfg15_w_negative` | [x] |
| 16 | `flip_horizontal` | `w == INT_MIN`, `h` ≥2 → `w*i` signed-overflows (ERRORS 13) | `cfg16_w_int_min` | [x] |
| 17 | `flip_horizontal` | `h<0` randomized, `w>0`, `pix` non-null → `flips<0`, no-op (ERRORS 6/7/8) | `cfg17_h_negative` | [x] |
| 18 | `flip_horizontal` | `h == INT_MIN`, `w>0` → `flips == -2^30`, no-op (ERRORS 9) | `cfg18_h_int_min` | [x] |
| 19 | `flip_horizontal` | `w == INT_MAX`, `h == 0` → `w` never used (ERRORS 16) | `cfg19_w_int_max_h_zero` | [x] |
| 20 | `flip_horizontal` | buffer allocated LARGER than `w*h` with a poison canary past `w*h` — proves both write the identical address range (ERRORS 3) | `cfg_padding_canary` | [x] |
| 21 | `flip_horizontal` | called TWICE on the same image — must return to the original bytes; compared against C called twice | `cfg21_double_call_involution` | [x] |
| 22 | `flip_horizontal` | called THREE times — must equal one call; differential | `cfg22_triple_call` | [x] |
| 23 | `flip_horizontal` | `cp_image_t` fields (`w`, `h`, `pix`) must be unmodified after the call — struct write-back differential | `cfg23_struct_fields_untouched` | [x] |
| 24 | `flip_horizontal` | all-channel value coverage: pixels set so every byte lane differs (r/g/b/a distinct, incl. 0x00 and 0xFF) | `cfg24_channel_lanes` | [x] |
| 25 | `flip_horizontal` | full randomized sweep over the `(w, h)` cross-product `w,h ∈ -2..=9` plus randomized pixels — hits every sign/parity/boundary combination in one property test | `cfg25_wh_cross_product` | [x] |
| 26 | `flip_horizontal` | randomized `(w, h)` from the whole `int` range restricted to non-dereferencing shapes (`w<=0` or `h<=1`), randomized pixels | `cfg26_random_int_range_nonderef` | [x] |
| 27 | `flip_horizontal` | property sweep: 500 randomized `(w in 1..=32, h in 0..=32)` cases with randomized pixels, exact buffers | `cfg27_property_sweep` | [x] |
| 28 | `flip_horizontal` | MISALIGNED `cp_image_t *` (struct placed at byte offsets 1..7) — C uses plain unaligned `int` loads | `generic_misaligned_image_struct` | [x] |
| 29 | `flip_horizontal` | `img->pix` pointing into the MIDDLE of a larger allocation, poison on both sides | `generic_pix_offset_into_buffer` | [x] |
| 30 | `flip_horizontal` | 2000 randomized cases drawn from the WHOLE `i32` range for both fields (non-dereferencing shapes) | `generic_full_int_range_sweep` | [x] |
| 31 | `flip_horizontal` | zero-length (but non-null, dangling-aligned) buffer, as returned by an empty allocation | `generic_zero_length_buffer` | [x] |
| 32 | `flip_horizontal` | repeated calls (1/2/3/7) on every no-op shape | `generic_repeated_calls_on_noop_shapes` | [x] |

## One configuration that is provably infeasible to test (documented, not hidden)

`pix + w * i` and `pix + w * (h - i - 1)` compute `w * i` in `int`. Reaching a
**signed overflow of that multiply while the result is also dereferenced** is
not testable in any differential harness:

* the inner loop dereferences `a[0..w]` and `b[0..w]`, and the outer loop runs
  `h/2` times, so a run that dereferences at all requires ~`w*h` pixels of
  resident memory;
* the multiply only overflows once `w * (h-1) > 2^31`, i.e. once `w*h`
  exceeds 2^31 pixels ≈ **8 GB** of resident, written memory.

Both conditions cannot hold at once on this machine. The overflow *arithmetic*
is therefore exercised only on the non-dereferencing shapes (rows 15, 16 and
`ERRORS.md` 12, 13), where `w <= 0` makes the inner loop empty. The Rust uses
`i32::wrapping_mul`, which reproduces what gcc emits at the `-O0` used by
`c_src/CMakeLists.txt` (the C is formally UB there). This is the single gap in
the matrix and it is a property of the C's own API shape, not of the Rust.

## Harness self-validation (mutation testing)

To prove the table above is not vacuous, four deliberate bugs were injected into
`src/lib.rs` and every one was DETECTED (see `mutation_check.sh`):

| mutant | injected bug | detected by |
|---|---|---|
| A | `flips = (h + 1) / 2` | `cfg12` (SIGSEGV — the extra iteration dereferences `pix==NULL`) |
| B | mirror columns instead of rows (what the function's *name* suggests) | 15 differential + 4 error-path rows |
| C | alpha channel excluded from the swap | 15 differential + 4 error-path rows |
| D | inner loop `j <= w` (one pixel overrun) | canary rows (20, `err03`) |

## Coverage argument

The two loop guards (lib.c:8, lib.c:11) are the only branches in the library.
Rows 1-8 + 20-27 cover both guards being taken 1, 2, and many times over
randomized data; rows 9-19 cover each guard failing on entry for each distinct
reason; rows 20/23 cover the memory footprint and the input struct itself.
