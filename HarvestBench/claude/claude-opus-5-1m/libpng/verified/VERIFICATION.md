# VERIFICATION.md — how this translation was verified

The C sources in `c_src/` are the ground truth.  Everything below compares the
translated Rust `cdylib` against the reference C shared library **through their
exported symbols only** — both are `dlopen`ed, every call goes through `dlsym`, so
the `#[no_mangle]` export wrappers are part of what is tested.

```
./check.sh                       # build both libraries + the shim, run everything
./check.sh --test errors         # one test target
tools/check_features.sh          # every feature combination
```

`check.sh` finishes by regenerating the three coverage reports, so the numbers in
this file are reproducible rather than asserted.

## Artifacts

| file | what it is | how it is produced |
|------|------------|--------------------|
| `SYMBOLS.md` | exported-symbol parity, C vs Rust | `tools/gen_symbols.py` from `nm -D` |
| `ERRORS.md` | the error surface: 368 + 118 + 218 + 6 = **710 rejection sites** | `tools/gen_errors.py` from `cc -E` + the C sources |
| `CONFIGS.md` | the configuration surface: **154 rows** | `tools/gen_configs_rows.py`, axes derived from the C branches |
| `ERROR_COVERAGE.md` | which ERRORS.md rows the suite actually reached | `tools/error_coverage.py` from `target/observed/*.txt` |

## Result of the last full run

```
12 test binaries, 136 #[test] functions: 135 passed, 0 failed, 1 ignored
        (the ignored one is the child half of the PNG_ABORT test, which the
         parent re-execs on purpose)
SYMBOLS.md        384 / 384 symbols, 0 missing, 0 extra, 0 type mismatches
CONFIGS.md        154 / 154 rows covered by a passing test
ERROR_COVERAGE.md 270 / 318 checkable diagnostic sites reached (84 %)
                  50 further sites have no literal message to match on
                  48 unreached, each listed with its reason
200,637 differential comparisons (machine-counted, tools/count_cases.py)
```

| test binary | `assert_same` | forked |
|-------------|---------------|--------|
| `simplified` | 102,788 | 82 |
| `errors` | 27,454 | 1,348 |
| `highlevel` | 24,642 | 3 |
| `transforms` | 19,969 | 0 |
| `errors_deep` | 5,276 | 1,280 |
| `sweep` | 0 | 4,801 |
| `chunks` | 3,790 | 0 |
| `progressive` | 3,533 | 3 |
| `lowlevel` | 2,266 | 0 |
| `misc` | 1,720 | 41 |
| `write_read` | 1,638 | 0 |
| `smoke` | 3 | 0 |
| **total** | **193,079** | **7,558** |

## Phase A — the surface

**Symbols.** The C `.so` exports 384 symbols; the Rust `.so` exports the same 384,
with the same `nm` type letters.  0 missing, 0 extra, 0 type mismatches.  Every
undefined symbol of the Rust `.so` is libc, libm, libgcc or zlib — nothing of
libpng is left unimplemented, and nothing is stubbed: `tests/sweep.rs` calls all
381 exported *functions* and `tests/smoke.rs` compares the 3 exported *data*
tables element by element.

Note the split of the 384: 258 are `png.h` public API, 123 are `pngpriv.h`
internal functions that libpng nevertheless exports, and 3 are data tables.  The
internal 123 are the "lowest-level entry points" and are driven directly
(`tests/lowlevel.rs`), not only through the convenience wrappers.

**Error surface.** `tools/gen_error_sites.py` runs the real C preprocessor over
each `c_src/src/*.c` and records every `png_error` / `png_chunk_error` /
`png_fixed_error` / `png_app_error` / `png_app_warning` / `png_benign_error` /
`png_chunk_benign_error` / `png_warning` / `png_chunk_warning` call that the build
actually compiles — 368 sites, 318 of them with a literal message.  Sites in
`#if`-disabled branches are excluded because they are dead code in `libpng.so`.
`tools/gen_errors.py` adds 118 guarded `return 0/NULL/-1` rejections, 218 guarded
`return;` rejections and the 6 generic FFI-boundary rejections.

**Configuration surface.** 20 axes (entry-point family, colour type × bit depth,
interlace, dimensions, filters, zlib knobs, read transforms, write transforms,
gamma/colourspace, ancillary chunks, unknown-chunk policy, CRC policy, user
limits, callbacks, options, MNG, benign errors, progressive chunking, simplified
formats, byte order) pruned to the 154 combinations the C treats differently.

## Phase B — valid-path differential tests

Every one of the 154 `CONFIGS.md` rows is driven with **many randomised inputs**
from a fixed-seed SplitMix64, and both libraries' complete event trace and every
output byte are compared.  `tools/config_coverage.py` maps each row to the test
that covers it and refuses to check a row off unless that test is present in the
run and passed; the current result is **154/154**.

| test binary | `#[test]` fns | what it drives |
|-------------|---------------|----------------|
| `smoke` | 6 | harness self-check, version strings, pure accessors, the 3 data tables |
| `lowlevel` | 21 | the 123 internal exports: `png_muldiv`, `png_reciprocal*`, `png_fixed*`, `png_gamma_*`, `png_XYZ_from_xy`, `png_check_fp_*`, `png_ascii_from_*`, `png_safecat`, `png_format_number`, `png_calculate_crc`, `png_do_bgr/invert/packswap/swap/strip_channel`, `png_read_filter_row`, `png_write_find_filter`, `png_combine_row`, `png_do_*_interlace`, `png_check_IHDR`, `png_check_keyword`, `png_zstream_error`, `png_do_check_palette_indexes`, the time conversions |
| `write_read` | 10 | the low-level writer and sequential reader over all 15 legal (colour type, bit depth) pairs × interlace × 12 sizes, all filter masks, all zlib knobs, buffer sizes, raw chunk writing, flushing, info getters, `png_set_sig_bytes` |
| `transforms` | 11 | 27 read transforms singly, 1200 random 2..6-transform combinations, gamma tables, `png_read_update_info` misuse, 10 write transforms, MNG intrapixel |
| `chunks` | 12 | all 21 ancillary chunk types absent / once / twice, ICC validation, text compression knobs, the unknown-chunk policy cross-product, `png_set_rows`/`png_data_freer`, `png_set_invalid`, the user-chunk callback, chunk ordering |
| `highlevel` | 3 | `png_read_png` / `png_write_png` with every transform flag and 2100 random flag combinations, plus round trips |
| `progressive` | 4 | `png_process_data` at 9 feed granularities × every shape × interlace, `png_process_data_pause` / `_skip`, adversarial split boundaries, transforms from the info callback |
| `simplified` | 4 | the `png_image_*` API in every format (8 base × plain/colormap, 4 linear), every flag, background present/absent, row strides positive/negative/zero, memory/stdio/file, round trips |
| `misc` | 11 | allocators, IO state, all 36 CRC actions, user limits, status callbacks, `png_set_option`, row/pass numbers, `png_set_longjmp_fn`, struct lifecycle, grayscale palettes |
| `errors` | 22 | the error surface (Phase C) |
| `errors_deep` | 28 | the error surface that needed a specially built input (Phase C) |
| `sweep` | 5 | the FFI boundary over all 381 exported functions (Phase C) |

## Phase C — error-path differential tests

`tests/errors.rs`, `tests/errors_deep.rs` and `tests/sweep.rs` construct the
rejection conditions and compare the *exact* outcome: the same fatal/non-fatal
decision, the same message text, in the same order, and — where the C library
dies — death from the same signal.

* **Part 1 (368 diagnostic sites).**  A row is checked off only when the suite
  observed that message coming out of **both** libraries with identical traces:
  `assert_same` / `assert_same_forked` compare *before* the message is recorded,
  so a recorded message is by construction one the two libraries agreed on.
  `tools/error_coverage.py` aggregates the observations of all test processes and
  stamps `ERRORS.md`, whose `seen` column therefore has three machine-derived
  states:

  * `[x]` — **270** sites: the suite observed this exact message from both
    libraries, with identical traces.
  * `[-]` — **50** sites: the message is assembled at run time (a variable, or
    `png_formatted_warning` parameters), so there is no literal to match on.  The
    site is still exercised; it just cannot be checked off *by text*.
  * `[ ]` — **48** sites, every one listed at the end of `ERROR_COVERAGE.md`.
    They fall into three groups, and `tests/errors_deep.rs` records the C
    reasoning for each next to a test that drives its nearest reachable
    neighbour: libpng's own internal-consistency guards (the message says so —
    `"internal error"`, `"BAD internal error"`, `"unexpected …"`); checks that
    need a >2 GiB object or a 32-bit `size_t` (e.g. `pngwutil.c:1589` needs a
    `strlen` above `PNG_UINT_31_MAX`, `pngrutil.c:4649` needs `rowbytes` above
    `2^64-2` while the maximum is `(2^31-1)*8`); and checks *shadowed by an
    identical earlier test in the same function* (e.g. `pngrutil.c:211`'s
    `buf[0] >= 0x80` is exactly the condition `png_get_uint_31` already errored
    on 14 lines earlier, and `pngwutil.c:1148` repeats the comparison
    `pngwutil.c:1137` already made).
* **Parts 2 and 3 (118 + 218 silent rejections).**  These have no message to
  observe, so they are covered exhaustively rather than individually:
  `tests/sweep.rs::null_arguments` calls **all 381 exported functions** with NULL
  in every pointer position and 0 in every scalar position and compares the return
  value; `hostile_scalars_read` / `hostile_scalars_write` repeat that with a live
  `png_ptr` and 14 hostile scalars in every scalar position; `enum_boundaries`
  walks each documented enum range from −2 to one past its end on a set-up struct;
  `length_boundaries` does 0 / 1 / 2 / 7 / 8 / 2³¹−1 / 2³¹ / 2³²−1 / `SIZE_MAX`.
* **Part 4 (6 generic boundaries).**  Out-of-range enum values crossing the FFI
  boundary get their own axis, because a C `enum` accepts any `int` and a value
  with no valid variant is a real input.  `PNG_ABORT()` is covered by
  `errors::png_abort_row_A1`, which re-execs the test binary and requires both
  libraries to die from `SIGABRT`.

Every call that can be fatal to the C library runs in a `fork()`ed child
(`tests/common/forked.rs`), so "the C library segfaults here" is a *compared
observation* instead of the end of the test run.  That is what makes it safe to
call all 381 entry points with garbage.

## Phase D — parity and feature combinations

* `nm -D` diff: **empty** (see `SYMBOLS.md`).
* `Cargo.toml` has no `[features]`, so the power set of the crate's features is
  the single empty combination.  `tools/check_features.sh` derives that list from
  `Cargo.toml` mechanically and runs `cargo check`, `cargo build --release` and
  the whole suite for each element; `--all-features` and the default were run too.
  All green.

## Divergences found and fixed

All five were fixed in the **Rust** side; nothing under `c_src/` was ever
modified.

| # | symptom | root cause | fix |
|---|---------|-----------|-----|
| 1 | `png_sig_cmp(sig, 7, 1)` returned `1` instead of `115` | `png_sig_cmp` *returns* the value of `memcmp`, so its magnitude is observable; the hand-written `memcmp` in `src/util.rs` returned only −1/0/1 while glibc returns the byte difference | `src/util.rs`: call the C library's `memcmp` |
| 2 | `png_chunk_warning(NULL, NULL)` segfaulted; C printed `libpng warning: (null)` | `png_default_warning` (`pngerror.c:703`) passes the message straight to `fprintf`'s `%s` with no NULL check, and glibc prints `(null)`; the Rust helper called `fputs(NULL)` | `src/ffi.rs`: `png_stderr_message` substitutes `(null)` |
| 3 | `png_set_cHRM` reported the *first* bad coordinate, C reported the *last* | the C argument list `png_set_cHRM_fixed(…, png_fixed(…), …)` is evaluated right-to-left by the reference compiler, and `png_fixed` diverges on an out-of-range value | `src/gen/pngset_p01.rs`, `src/gen/pngrtran_p03.rs`: bind the conversions right-to-left in `png_set_cHRM`, `png_set_cHRM_XYZ`, `png_set_cLLI`, `png_set_mDCV`, `png_set_rgb_to_gray` |
| 4 | `png_write_chunk_start(png, NULL, 0)` survived in Rust, segfaulted in C | `PNG_CHUNK_FROM_STRING` dereferences the name in the *argument list*, before the callee's `png_ptr == NULL` guard; LLVM was free to sink the plain loads past that guard because a trapping load is UB in Rust | `src/util.rs`: `PNG_CHUNK_FROM_STRING` uses `read_volatile` |
| 5 | `png_image_write_to_memory` with `width == 0`: C died from `SIGFPE`, Rust from `SIGABRT` | `png_image_write_main` divides by `png_row_stride`, which is 0 (`pngwrite.c:2045`); Rust's `/` panics instead of trapping | `src/util.rs`: `c_div_u32` performs the division with the same `div` instruction, so the same hardware trap is raised; used at `src/gen/pngwrite_p05.rs` |

## Why the tests are believed to bite

A suite that passes on the first try is not evidence of anything, so the
comparison was mutation-tested: single-token changes were injected into the Rust
library and each had to be caught, then reverted and the revert verified against
the C source.  Examples: `png_do_invert`'s loop bound, `png_calculate_crc`'s
result, `png_combine_row`'s `pass < 6`, `png_write_find_filter`'s 128 threshold,
`png_do_check_palette_indexes`' 8-bit branch, `png_read_push_finish_row`'s
`width < 2`, `PNG_PUSH_SAVE_BUFFER_IF_FULL`'s `+4`, `png_write_cLLI_fixed`'s
`maxFALL`, `png_write_image_16bit`'s `+16384` rounding, `png_read_png`'s
`SCALE_16` branch, `png_write_png`'s `INVERT_ALPHA` branch, and a
`(231*gray+128)>>8` rounding constant.  Every one produced a failing test.

`assert_same` also refuses a scenario that recorded nothing at all, so a test
cannot pass by accident of doing nothing.

## Things the C does that are *deliberately* reproduced

* Right-to-left evaluation of the `png_fixed()` arguments (divergence 3).
* `fprintf("%s", NULL)` printing `(null)` (divergence 2).
* Dereferencing a NULL chunk name before the callee's NULL check (divergence 4).
* Trapping on division by zero rather than panicking (divergence 5).
* `png_process_data_skip` being a warning-only stub in this version, and that
  warning being *fatal* because `PNG_RELEASE_BUILD` is false so
  `PNG_FLAG_APP_WARNINGS_WARN` is not set by `png_create_read_struct`.
* `png_free_data(PNG_FREE_TEXT, n)` clearing only `key` and leaving `text`,
  `lang` and `lang_key` dangling.
* `PNG_IGNORE_ADLER32` having no effect, because
  `PNG_DISABLE_ADLER32_CHECK_SUPPORTED` is `#undef` in `pnglibconf.h`.

## Known limits of the comparison

* libpng's `big_row_buf` comes from `png_malloc`, not `png_calloc`, for
  non-interlaced images, and `png_combine_row` preserves the destination's
  padding bits.  Where a transform makes the real row shorter than the buffer, the
  bytes past it are uninitialised heap and legitimately differ between two
  separate libraries.  Tests either supply their own zeroed rows or exclude those
  padding bits, and say so where they do.
* A handful of C sites are unreachable by construction (`"… (internal error)"`,
  `"BAD internal error"`, checks that need a >2 GiB allocation or 32-bit
  `size_t`).  They are listed unchecked in `ERROR_COVERAGE.md` with the reason.
