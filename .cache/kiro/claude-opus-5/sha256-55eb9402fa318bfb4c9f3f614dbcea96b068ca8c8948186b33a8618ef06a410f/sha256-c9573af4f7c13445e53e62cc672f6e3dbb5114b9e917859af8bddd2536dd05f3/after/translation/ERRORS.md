# ERRORS.md — error / rejection surface table

## Mechanical derivation

Exhaustive grep of the whole C source (`c_src/src/lib.c`, `c_src/include/lib.h`
— 52 + 22 lines):

```
$ grep -nE 'return|assert|NULL|if|while|for|switch|#if|enum|ERROR|-1' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:24:    if (m->pos >= 64) {
c_src/src/lib.c:27:        while (bytes--) {
c_src/src/lib.c:37:    for (int i = 0; i <= 4; i++) {
c_src/src/lib.c:50:    return b;
```

Findings, stated precisely because they define the shape of this table:

* **Zero** error-return macros (`RETURN_ERROR` &c.), **zero** `assert`,
  **zero** `NULL` checks, **zero** explicit range checks, **zero** error enums,
  **zero** named min/max constants, **zero** `#ifdef` branches.
* **Zero** `enum` types anywhere in the public API, so the "out-of-range enum
  value across FFI" class has **no applicable row** — there is no enum
  parameter to pass an invalid discriminant to. The only non-pointer scalar
  parameters are `tflac_u32 bits` and `tflac_u64 n`, both of which accept the
  entire value range by construction (rows 5–11 below cover them).
* The only `return` is `return b;` in `update_md5`, which returns a computed
  `tflac_u32` unconditionally. It is **not** a status code and has **no**
  sentinel value: every one of the 2^32 results is a legitimate output.
  `tflac_pack_u64le` and `tflac_md5_addsample` return `void`.
* The only branch in the library is `if (m->pos >= 64)` (`lib.c:24`).

So this library **rejects nothing**. Its error surface is therefore entirely
made of *implicit* rejections: inputs for which the C performs an out-of-bounds
or null access, or silently wraps. Each row below is one such distinct
condition, derived from an actual dereference or arithmetic op in the C, and
each has a differential test asserting C and Rust behave identically.

`buffer` is `tflac_u8 buffer[64 + 8]`, i.e. **72** bytes at struct offset 16, so
the last in-bounds index is `buffer[71]`. `sizeof(tflac_md5) == 88`,
`sizeof(tflac) == 96`.

## Table

Legend for "expected C result": `SIGSEGV` = process dies on an unconditional
dereference; `wrap` = defined unsigned modular arithmetic; `OOB` = reads bytes
past the end of `buffer` (deterministic given a fixed surrounding allocation,
so still differentially testable).

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `tflac_pack_u64le` | `d == NULL` — `lib.c:6` stores to `d[0]` with no check | SIGSEGV (no return, no error code) |
| 2  | `tflac_md5_addsample` | `m == NULL` — `lib.c:20` does `m->total += bits` with no check | SIGSEGV |
| 3  | `update_md5` | `t == NULL` — `lib.c:33` reads `t->cur_blocksize` with no check | SIGSEGV |
| 4  | `update_md5` | `samples == NULL` — `lib.c:38` reads `samples[0]` with no check | SIGSEGV |
| 5  | `tflac_md5_addsample` | `bits == 0` (zero "length"): `bytes = 0/8 = 0` | no rejection; `total += 0`, still writes 8 bytes at `buffer[pos%64]`, `pos` unchanged; `if (pos>=64)` decided by the *incoming* `pos` |
| 6  | `tflac_md5_addsample` | `bits` not a multiple of 8 (`1..7`): integer division truncates | no rejection; `bytes = 0` (so `pos` unchanged) while `total += bits` adds the *full* unrounded value |
| 7  | `tflac_md5_addsample` | `bits == 65`, i.e. one step past the 64 that `update_md5` always passes | no rejection; `bytes = 8` (65/8 truncates), `total += 65` |
| 8  | `tflac_md5_addsample` | `bits == 0xFFFFFFFF` (oversized length): `bytes = 0x1FFFFFFF` | no rejection; `pos += 0x1FFFFFFF` wraps mod 2^32, `total += 0xFFFFFFFF` |
| 9  | `tflac_md5_addsample` | `bits` chosen so `pos + bits/8 == 64` exactly (e.g. `pos=56, bits=64`) — the `>=` boundary of the sole branch | branch taken, `pos %= 64` → 0, `bytes = 0`, so `while (bytes--)` is **not** entered (tests the pre-decrement condition) |
| 10 | `tflac_md5_addsample` | `m->pos == 63`, `bits = 64` — largest documented `pos`; `pos` → 71 → 7, copies `buffer[0..6] = buffer[64..70]` | in-bounds but at the very last usable source byte region (`buffer[70]`, len 72) |
| 11 | `tflac_md5_addsample` | `m->pos == 64`, one step past the valid `0..63` range; `pos` → 72 → 8, `bytes = 8` | reads `buffer[64+7] == buffer[71]` — exactly the final in-bounds byte |
| 12 | `tflac_md5_addsample` | `m->pos == 65`, two steps past valid range; `pos` → 73 → 9, `bytes = 9` | **first OOB**: reads `buffer[72]`, 1 byte past the array — lands on `tflac.cur_blocksize` when `m` is the embedded `md5_ctx` |
| 13 | `tflac_md5_addsample` | `m->pos == 1000` (grossly out of range); `pos` → 1008 → 48, `bytes = 48` | OOB: reads `buffer[64..111]`, up to 40 bytes past the array (past the end of `struct tflac` itself) |
| 14 | `tflac_md5_addsample` | `m->pos == 0xFFFFFFFF`, `bits = 64`: `pos + 8` **wraps** to 7 | wrap; `7 >= 64` is false so the branch is **not** taken and `pos` is left at 7 |
| 15 | `tflac_md5_addsample` | `m->total == 0xFFFFFFFFFFFFFFFF`: `total += bits` overflows u64 | wrap mod 2^64, no rejection |
| 16 | `tflac_md5_addsample` | write boundary: `m->pos % 64 == 63` → `tflac_pack_u64le(&buffer[63], …)` writes `buffer[63..70]` | in-bounds by exactly 1 byte (len 72); no rejection |
| 17 | `update_md5` | `t->channels == 0` (and/or `cur_blocksize == 0`) → `b = 0`, then five `b -= 8` | wrap: returns `0 - 40 = 0xFFFFFFD8` |
| 18 | `update_md5` | `cur_blocksize * channels < 40` but non-zero (e.g. `4 * 2 = 8`) | wrap: returns `8 - 40 = 0xFFFFFFE0` |
| 19 | `update_md5` | `cur_blocksize * channels == 40` exactly (the boundary at which the result stops underflowing) | returns `0` |
| 20 | `update_md5` | `cur_blocksize * channels` overflows u32 (e.g. `0x10000 * 0x10000`) | wrap: product is `0`, returns `0xFFFFFFD8` |
| 21 | `update_md5` | `samples` array shorter than **136** `tflac_s32`s: the pointer advances `8*sizeof(tflac_s32) == 32` elements per iteration for 5 iterations, so the last read is `samples[128..135]` | OOB read past the caller's array; no check, no rejection |
| 22 | `update_md5` | `t->md5_ctx.pos` out of range on entry (64, 65, 1000, 0xFFFFFFFF) — propagates rows 11–14 through the composed pipeline | same OOB / wrap behaviour, five times over, plus the return value |
| 23 | `update_md5` | `t->md5_ctx.total` near `u64::MAX` — five `+= 64` overflow it | wrap mod 2^64 |
| 24 | *(N/A)* | out-of-range **enum** value across the FFI boundary | **no applicable row**: `grep -n enum` over the C source finds none; the API has no enum parameter |
| 25 | *(N/A)* | negative or zero **length** argument | **no applicable row**: no function takes a length/count/size parameter. `bits` is the nearest analogue and is covered by rows 5–8 |

## Status

All 25 rows are covered by `translation/tests/differential.rs`:

* Rows 1–4 (`SIGSEGV`) — `errors::row01_..row04_..`, each `fork()`s a child per
  side and asserts **both** die with the **same** signal (SIGSEGV), rather than
  merely "both failed".
* Rows 5–23 — differential tests over a fixed-seed randomized input sweep,
  comparing the returned `tflac_u32` **and** the full byte image of a padded
  512-byte allocation containing the struct (so OOB writes/reads are compared
  byte-for-byte).
* Rows 24–25 are documented non-applicable, with a `enum_and_length_surface_is_empty`
  test that re-greps the C source at test time to assert no `enum` / length
  parameter has appeared.

| row | test | checked |
|-----|------|---------|
| 1  | `row01_pack_null_dst`                  | [x] |
| 2  | `row02_addsample_null_ctx`             | [x] |
| 3  | `row03_update_md5_null_t`              | [x] |
| 4  | `row04_update_md5_null_samples`        | [x] |
| 5  | `row05_bits_zero`                      | [x] |
| 6  | `row06_bits_not_multiple_of_8`         | [x] |
| 7  | `row07_bits_65_one_past`               | [x] |
| 8  | `row08_bits_u32_max`                   | [x] |
| 9  | `row09_pos_plus_bytes_exactly_64`      | [x] |
| 10 | `row10_pos_63_max_valid`               | [x] |
| 11 | `row11_pos_64_one_past_valid`          | [x] |
| 12 | `row12_pos_65_first_oob`               | [x] |
| 13 | `row13_pos_1000_deep_oob`              | [x] |
| 14 | `row14_pos_u32_max_wraps_below_64`     | [x] |
| 15 | `row15_total_u64_max_wraps`            | [x] |
| 16 | `row16_write_boundary_pos63`           | [x] |
| 17 | `row17_channels_zero`                  | [x] |
| 18 | `row18_product_below_40`               | [x] |
| 19 | `row19_product_exactly_40`             | [x] |
| 20 | `row20_product_overflows_u32`          | [x] |
| 21 | `row21_samples_shorter_than_136`       | [x] |
| 22 | `row22_ctx_pos_out_of_range`           | [x] |
| 23 | `row23_total_near_u64_max`             | [x] |
| 24 | `enum_and_length_surface_is_empty`     | [x] |
| 25 | `enum_and_length_surface_is_empty`     | [x] |

## Divergence found and fixed (rows 1 and 4)

The first run of the error-path suite failed on rows 1 and 4:

```
REJECTION DIVERGENCE in row01 pack(NULL, 0x0..0): C terminated with Signal(11) but Rust terminated with Signal(6)
REJECTION DIVERGENCE in row04 update_md5(t, NULL):  C terminated with Signal(11) but Rust terminated with Signal(6)
```

Cause: the Rust translation dereferenced the caller's pointer directly
(`*d.add(0) = …`, `*samples.add(0)`), and with `-C debug-assertions=on` — the
default for `cargo build`/`cargo test` — rustc inserts a MIR-level null check on
every raw-pointer dereference. A null argument therefore produced
`panicked at 'null pointer dereference occurred'` → `abort()` → **SIGABRT (6)**,
where the C's unchecked store/load raises **SIGSEGV (11)**. Rows 2 and 3 passed
only by accident: their first access is at a non-zero struct offset
(`m + 8`, `t + 88`), so the pointer being checked was not literally null.

Fix (in the Rust only): every access to caller-supplied memory now goes through
a call to libc `memcpy` via small `ld_*`/`st_*` helpers, and all pointer
arithmetic uses `wrapping_add`. An FFI call is opaque to rustc's instrumentation,
so the load/store faults exactly where the C's does. The stores in
`tflac_pack_u64le` are still emitted one byte at a time in the C's order, so a
partially mapped destination faults after the same prefix has been written.
This also makes the crate behave identically in debug and release, rather than
only matching the C when optimisations happen to be on.

## Negative control

`mutation_check.sh` deliberately breaks the Rust translation 11 different ways
(wrong `samples` stride, carry-down source off by one, `pos` advanced from
`pos % 64`, wrong `step`, `total` updated with `bits/8`, `> 64` instead of
`>= 64`, `while (bytes--)` mistranslated as a pre-decrement, a checked raw deref
instead of the unchecked `memcpy`, wrong mask, wrong shift, 4 iterations instead
of 5) and asserts the suite FAILS for each. All 11 are detected, so the passing
result above is not vacuous.
