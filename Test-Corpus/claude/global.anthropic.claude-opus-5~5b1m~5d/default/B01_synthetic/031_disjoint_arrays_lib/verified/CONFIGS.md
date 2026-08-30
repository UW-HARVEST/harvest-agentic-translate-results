# CONFIGS.md — Phase B configuration surface (valid inputs)

Mechanically derived from the branches the C source actually takes.

## Axes the C code distinguishes

**Profiles.** Although there are no cargo *features*, the dev and release
profiles are genuinely different artifacts (release is `panic = "abort"`; dev
enables `-C debug-assertions`, which changes how raw-pointer UB is trapped — this
actually surfaced a real bug, see ERRORS.md note C). `scripts/check_features.sh`
therefore runs the entire suite under `{release, dev} x {--no-default-features,
default}`.

**Compile-time / feature axes** — none. `c_src/src/driver.c` contains no
`#ifdef`/`#if` at all (only the `DRIVER_H_` include guard in the header), and
`translation/Cargo.toml` declares no `[features]`. So there is exactly one build
configuration. There are also no runtime option/mode/flag setters in the public
API (no init struct, no context object, no globals): the *only* configuration a
caller can express is the **arguments themselves**.

**Public entry points** (all three exported symbols; the low-level ones are
driven directly, not just through the `driver()` one-shot wrapper):

* `fma_array(out, mul1, mul2, add, len)` — lowest level, raw element kernel.
* `call_fma(data, len)` — mid level; builds `ones`/`zeros` VLAs and calls
  `fma_array`, returns `out[len-1]`.
* `driver(in)` — top level; `sscanf` parse loop → `call_fma` → `printf`.

**Input-shape axes**

| axis | values the C treats differently |
|---|---|
| A1 `len` (`fma_array`, `call_fma`) | `0` (loop/early-out rejected), `1` (single element == returned element), `2`, `3`, small odd/even, `100`, large (`1000`, `100000` → big VLAs) |
| A2 element magnitude | zero, small (`-100..100`), full `i32` range, corners `{INT_MIN, INT_MAX, -1, 0, 1}` → drives the signed-overflow path of `mul*mul+add` |
| A3 pointer aliasing (`fma_array` only — `out` is `restrict`, C built at `-O0`) | all four buffers distinct; `out == mul1`; `out == mul2`; `out == add`; `mul1 == mul2`; `mul1 == add`; all three inputs one buffer; all four one buffer; partial overlap `out = buf`, `mul1 = buf+1`; partial overlap `out = buf+1`, `mul1 = buf` |
| A4 `driver` token count | `0`, `1`, `2`, `3`, `99`, `100` (cap boundary), `101`, `150` |
| A5 `driver` whitespace separators | `" "`, `"  "` (multiple), `"\t"`, `"\n"`, `"\r"`, `"\v"`, `"\f"`, mixed runs, leading whitespace, trailing whitespace |
| A6 `driver` sign / digit form | unsigned digits, `+` prefix, `-` prefix, leading zeros (`"007"`), `INT_MIN`/`INT_MAX` literals |
| A7 `driver` terminator shape | end of string, non-whitespace separator (`","`, `";"`, `"."`, `"/"`), digits glued to letters (`"12abc"`), `"0x10"` form, trailing `-`/`+` |
| A8 `driver` total input length | empty, 1 char, typical, oversized (~100 000 chars) |

## Configuration table

Every row is driven with **many randomized inputs (fixed seed `0x5EED_1234_ABCD_9876`,
SplitMix64)** through both `.so`s and compared byte-for-byte, except where the
row is by construction a single fixed shape (still repeated over randomized
*values*).

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `fma_array` | A1=`0`, A3=distinct, canary-checked `out` (no stores expected) | `cfg01_fma_len0_distinct` | [x] |
| 2  | `fma_array` | A1=`1`, A2=small random, A3=distinct | `cfg02_fma_len1_small` | [x] |
| 3  | `fma_array` | A1=`2`, A2=small random, A3=distinct | `cfg03_fma_len2_small` | [x] |
| 4  | `fma_array` | A1=`3`,`5`,`7`,`8`,`16` (odd+even), A2=small random, A3=distinct | `cfg04_fma_small_lens_small_vals` | [x] |
| 5  | `fma_array` | A1=`64`, A2=full `i32` random (overflow reachable), A3=distinct | `cfg05_fma_len64_full_range` | [x] |
| 6  | `fma_array` | A1=`100`, A2=corner set `{INT_MIN,INT_MAX,-1,0,1}` random mix, A3=distinct | `cfg06_fma_corners` | [x] |
| 7  | `fma_array` | A1=`1000`, A2=full range, A3=distinct | `cfg07_fma_len1000_full_range` | [x] |
| 8  | `fma_array` | A1 random `1..=32`, A3=`out == mul1` (in-place on first factor) | `cfg08_fma_alias_out_eq_mul1` | [x] |
| 9  | `fma_array` | A1 random `1..=32`, A3=`out == mul2` (in-place on second factor) | `cfg09_fma_alias_out_eq_mul2` | [x] |
| 10 | `fma_array` | A1 random `1..=32`, A3=`out == add` (in-place on addend) | `cfg10_fma_alias_out_eq_add` | [x] |
| 11 | `fma_array` | A1 random `1..=32`, A3=`mul1 == mul2` (square), distinct `out` | `cfg11_fma_alias_mul1_eq_mul2` | [x] |
| 12 | `fma_array` | A1 random `1..=32`, A3=`mul1 == mul2 == add` (one input buffer), distinct `out` | `cfg12_fma_alias_all_inputs_same` | [x] |
| 13 | `fma_array` | A1 random `1..=32`, A3=`out == mul1 == mul2 == add` (one buffer for everything) | `cfg13_fma_alias_everything_same` | [x] |
| 14 | `fma_array` | A1 random `1..=32`, A3=partial overlap `out = buf`, `mul1 = buf+1` (forward) | `cfg14_fma_partial_overlap_forward` | [x] |
| 15 | `fma_array` | A1 random `1..=32`, A3=partial overlap `out = buf+1`, `mul1 = buf` (backward) | `cfg15_fma_partial_overlap_backward` | [x] |
| 16 | `call_fma` | A1=`0` (early-out) with non-NULL data | `cfg16_call_fma_len0` | [x] |
| 17 | `call_fma` | A1=`1`, A2=full-range random | `cfg17_call_fma_len1` | [x] |
| 18 | `call_fma` | A1=`2`,`3`,`4`,`5` , A2=full-range random | `cfg18_call_fma_tiny_lens` | [x] |
| 19 | `call_fma` | A1 random `1..=100`, A2=small random | `cfg19_call_fma_random_lens_small` | [x] |
| 20 | `call_fma` | A1 random `1..=100`, A2=corner set `{INT_MIN,INT_MAX,-1,0,1}` | `cfg20_call_fma_corners` | [x] |
| 21 | `call_fma` | A1=`100` exactly (the count `driver` caps at), A2=full range | `cfg21_call_fma_len100` | [x] |
| 22 | `call_fma` | A1=`1000`, `4096`, `100 000`, `600 000`, `1 500 000` (large VLAs; run on a 256 MiB-stack thread because the C VLAs live on the *caller's* stack) | `cfg22_call_fma_large_lens` | [x] |
| 23 | `driver` | A4=`1`, A6=unsigned digits, A5/A7=no separators, A8=short | `cfg23_driver_single_token` | [x] |
| 24 | `driver` | A4=`2..=8` random, A5=single space | `cfg24_driver_space_separated` | [x] |
| 25 | `driver` | A4 random `1..=20`, A5=random mix of `" "`,`"\t"`,`"\n"`,`"\r"`,`"\v"`,`"\f"` runs | `cfg25_driver_mixed_whitespace` | [x] |
| 26 | `driver` | A4 random `1..=20`, A5=leading **and** trailing whitespace runs | `cfg26_driver_leading_trailing_ws` | [x] |
| 27 | `driver` | A4 random `1..=20`, A6=random mix of `+`-prefixed / `-`-prefixed / bare / leading-zero-padded literals | `cfg27_driver_sign_and_leading_zeros` | [x] |
| 28 | `driver` | A4 random `1..=20`, A2=full `i32` range values incl. `INT_MIN`/`INT_MAX` | `cfg28_driver_full_range_values` | [x] |
| 29 | `driver` | A4=`99`, `100`, `101`, `150` (cap boundary sweep), A5=space | `cfg29_driver_count_cap_sweep` | [x] |
| 30 | `driver` | A4 random `2..=12`, A7=non-whitespace separator (`","`,`";"`,`"."`,`"/"`,`":"`) at a random position → truncated parse | `cfg30_driver_nonws_separator` | [x] |
| 31 | `driver` | A4 random `1..=12`, A7=trailing garbage suffix (`"abc"`, `"x10"`, `"e5"`, `"+"`, `"-"`, `"0x10"`) | `cfg31_driver_trailing_garbage` | [x] |
| 32 | `driver` | A8=oversized (~100 000 chars, ≫100 tokens), A5=space | `cfg32_driver_oversized_input` | [x] |
| 33 | `driver` | fully randomized fuzz: random count `0..=140`, random values, random whitespace, random optional garbage tail (2000 cases) | `cfg33_driver_fuzz` | [x] |
| 34 | `driver`+`call_fma`+`fma_array` | end-to-end composed pipeline: parse with `driver`, then reproduce with `call_fma`/`fma_array` on the same data and cross-check all three exports agree between C and Rust *and* that the printed value equals `data[min(n,100)-1]` | `cfg34_pipeline_cross_check` | [x] |

## How `driver`'s stdout is compared

`driver` prints with `printf`, so the differential harness cannot simply compare
return values. `common::capture_stdout` runs the calls in a **forked child**
whose fd 1 is redirected to a temporary file. Forking (rather than redirecting
fd 1 in-process) is essential: `libtest` writes its own `"test foo ... ok"`
progress lines to fd 1 from the main thread, and an in-process redirect spliced
those into the captured bytes and produced bogus divergences. All of a test's
inputs are driven inside **one** child per library, and the transcript is split
back into one line per input (`driver` always prints exactly one `"%d\n"`), which
keeps the cost at two `fork()`s per test while still attributing each output line
to its input.
