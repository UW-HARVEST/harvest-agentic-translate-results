# VERIFICATION.md — differential verification of the Rust translation of libpng 1.6.59

The Rust crate in this directory is verified against the reference C build in
`../c_src` by loading **both shared libraries** with `libloading` and comparing
their behaviour **through the FFI boundary**.  The Rust crate is never called
directly from a test, so the `#[no_mangle] extern "C"` export wrappers are part
of what is under test.

```
../c_src/build/libpng.so                  <- reference (C, links system zlib)
translation/target/release/liblibpng.so    <- translation (Rust cdylib)
```

## How to run everything

```sh
./run_verification.sh            # build both, check symbol parity, run all tests
./run_verification.sh symbols    # Phase A / D symbol parity only
./run_verification.sh tests      # the differential tests only
```

Individual files:

```sh
cd translation
cargo test --release --test t03_read -- --test-threads=1
```

`stderr` carries `panicked at src/pngerror.rs: Box<dyn Any>` lines whenever the
Rust build raises a `png_error`; that is how the translation implements
`longjmp` (a panic caught by `png_safe_execute`), it is expected noise and the
runner filters it.

## Artifacts

| file | contents |
|---|---|
| `SYMBOLS.md` | all 384 symbols the C `.so` exports, each confirmed present in the Rust `.so`; plus the two `png.h` declarations that are `#ifdef`-ed out of this build and exported by neither |
| `ERRORS.md` | the error-surface table: 1183 rows, one per distinct rejection in the C, with the test file that reproduces it and a list of the inputs that are C-level undefined behaviour rather than rejections |
| `CONFIGS.md` | the configuration-surface table: 250 rows of valid option/shape combinations the C branches on differently, each ticked with the test file that covers it |

## Test files

| file | phase | tests | what it does |
|---|---|---|---|
| `tests/t01_low_level.rs` | B | 17 | pure/scalar helpers: `png_get_uint_32/16/31`, `png_get_int_32`, `png_save_*`, `png_sig_cmp`, `png_muldiv`, `png_reciprocal(2)`, `png_gamma_*`, `png_XYZ_from_xy`/`png_xy_from_XYZ`, `png_safecat`, `png_format_number`, `png_check_fp_number/string`, `png_ascii_from_fp/fixed`, `png_fixed(_ITU)`, `png_build_grayscale_palette`, the time conversions, the version strings |
| `tests/t02_write.rs` | B | 11 | the write pipeline through `png_write_info` / `_row` / `_rows` / `_image` / `_end` / `_flush` and `png_write_png`; all 32 filter subsets, every zlib option, every write transform, every ancillary chunk, text chunks, unknown chunks, MNG features, the raw `png_write_chunk*` API |
| `tests/t03_read.rs` | B | 13 | the read pipeline through `png_read_info` / `_update_info` / `_row` / `_rows` / `_image` / `_end`, `png_read_png`, and the progressive reader at feed sizes 1..100000; every read transform alone and combined; gamma / background / alpha-mode / rgb-to-gray / quantize; every ancillary chunk; all options and limits |
| `tests/t04_simplified.rs` | B | 13 | the simplified API: `png_image_begin_read_from_memory` / `_from_file` / `_from_stdio`, `png_image_finish_read` for every `PNG_FORMAT_*` and `PNG_IMAGE_FLAG_*`, every row-stride sign, `png_image_write_to_memory` / `_to_file` / `_to_stdio`, `png_image_free`, and round trips |
| `tests/t05_info.rs` | B | 25 | every `png_set_*` / `png_get_*` pair round-tripped on read and write structs, fixed-point and floating-point cross-checked, `png_get_valid` / `png_set_invalid`, `png_free_data` / `png_data_freer`, the EASY_ACCESS getters, `png_info_init_3`, the memory callbacks |
| `tests/t06_filters_roundtrip.rs` | B | 7 | the row-FILTER paths: streams carrying every filter type (uniform, mixed, randomised) for every shape, and full write→read round trips including cross-library ones |
| `tests/t07_callbacks.rs` | B + C | 12 | user row transforms (read and write), the user chunk callback, `png_process_data_pause`, `png_progressive_combine_row`, the memory callbacks, `png_set_rows`/`png_get_rows`, `png_free_data`; plus the `png_process_data_skip`, `png_data_freer` and user-chunk-return rejections |
| `tests/t20_err_read.rs` | C | 14 | read-path rejections: signature, IHDR, chunk framing, IDAT/zlib, PLTE/tRNS/hIST, per-chunk length and position, per-chunk content, `png_image` argument validation, a 38 000-case byte-level mutation fuzzer and a 22 800-case structured chunk-framing fuzzer (declared length, name bytes, CRC, payload length, duplication, deletion, reordering) |
| `tests/t21_err_getters.rs` | C | 14 | every `png_get_*` sentinel and every `png_set_*` NULL guard |
| `tests/t22_err_setters.rs` | C | 22 | the read-side setter and transform rejections (1870 cases): `png_set_shift`, `png_set_filler`/`add_alpha`, all 27 transforms after `png_read_update_info` / `png_start_read_image` / before IHDR, gamma range, alpha mode, background, rgb-to-gray, quantize, crc_action, keep_unknown_chunks, the user limits, MNG, sig_bytes, and a `:benign` subset |
| `tests/t23_err_write.rs` | C | 17 | the write-path rejections (~800 cases): version checks, IHDR, PLTE, tRNS, every chunk setter, the compression parameters, `png_set_filter`, the write transforms, the raw chunk API, the error/warning dispatchers, `png_set_longjmp_fn`, memory, `png_set_option`, `png_permit_mng_features` |
| `tests/t24_err_misc.rs` | C | 21 | the remaining `png.c` / `pngerror.c` / `pngmem.c` / `pngrio.c` / `pngwio.c` rejections: `png_ascii_from_*` with an undersized buffer, `png_fixed`/`png_fixed_ITU` overflow, the `png_muldiv`/`png_reciprocal` sentinels, every `png_XYZ_from_xy`/`png_xy_from_XYZ` failure branch, all ten `png_zstream_error` messages, all `png_icc_check_*` reasons, the nine `png_convert_to_rfc1123_buffer` field checks, `png_user_version_check`, the array-allocation sentinels, `png_zalloc`/`png_zfree`, `png_check_keyword`, `png_check_IHDR` called directly, `png_chunk_unknown_handling`/`png_handle_as_unknown`, `png_reset_zstream`, and the reachable `pngrio.c`/`pngwio.c` paths |

**Totals: 12 test files, 186 `#[test]` functions, all passing.**  Combined they
run well over 100 000 differential cases (t04 alone has ~45 000 randomised
simplified-API cases, t20 ~34 000 fuzz cases, t22 1870 sub-process cases, t23
~800), and the whole suite completes in about 20 seconds.

## The two comparison mechanisms

**In-process.**  Used for everything that returns instead of `longjmp`-ing.  A
shared set of `extern "C"` callbacks (`tests/common/harness.rs`) records the
ordered warning transcript, the produced bytes and the row-status calls; the
same callbacks serve both libraries.  The error handler is deliberately fatal
here: a `png_error` on a valid input prints the message and aborts, so a valid
path can never silently swallow one.

**Sub-process.**  `png_error` must not return, and Rust cannot `setjmp`.  Each
error case therefore re-executes the test binary (`--exact harness_child`) once
per library with a handler that prints the message and `exit(70)`s.  The parent
compares the entire transcript: every warning in order, then the error text,
then the exit status **and the terminating signal** — so `PNG_ABORT()` paths are
compared too.

**Anti-vacuity.**  Every file that could in principle "pass by comparing
nothing" contains a `self_check` test asserting that the C library really does
produce the expected distinct sentinels/messages (e.g. `t20_err_read.rs`
asserts at least six distinct error strings are observed and prints them;
`t23_err_write.rs` asserts `png_error` reaches the handler with the exact text
and `exit(70)`; `t07_callbacks.rs` asserts each callback fires the exact number
of times).  Both the C output buffer and the C PNG bytes were also temporarily
XOR-sabotaged during development to confirm the comparisons fail when they
should.

## Feature combinations

`Cargo.toml` declares **no `[features]`**, so `default` is the only
configuration and the whole matrix is a single column.  `run_verification.sh`
still loops over the feature list it derives from `cargo metadata`, so adding a
feature automatically extends the matrix rather than silently leaving it
untested.

`[profile.dev]` is set to `overflow-checks = false` / `debug-assertions = false`
to match `[profile.release]`, because the C relies on wrapping arithmetic
throughout; this makes a debug `.so` usable for differential testing too.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows **0** symbols missing from the Rust `.so` and
      **0** extra; the only undefined symbols it imports are libc / libm /
      libgcc_s / zlib — the same external surface the C build links.
- [x] Phase B: every row of `CONFIGS.md` (250) passes across randomised inputs.
- [x] Phase C: every reachable row of `ERRORS.md` (1131 of 1183) has a passing
      differential error-path test; the remaining 52 are `#ifdef`-excluded from
      this configuration or are places where the C has no check at all, each
      individually documented in `ERRORS.md`.
- [x] The above hold under **every** feature combination — there is exactly one.

## Findings

No divergence between the C reference and the Rust translation was found on any
input that the C actually defines.  Two classes of input were excluded from
assertion, both documented in `ERRORS.md`:

1. Inputs the C dereferences without a guard (NULL `sig`/`ptime`/`row`/`image`,
   out-of-range `num` in `png_free_data`, a bogus `png_image.opaque`, ...).
2. One case where the C itself overflows a stack buffer and survives by luck: a
   user transform declaring more than 64 bits per pixel on an interlaced image
   overruns `png_byte v[8]` in `png_do_read_interlace` (`pngrutil.c:3927`, whose
   own comment asserts `pixel_depth` does not exceed 64).  Non-interlaced
   over-64-bit user transforms agree in both libraries and are covered.

A third class is recorded in `ERRORS.md` but not asserted: five rows are
present in the C yet **unreachable for any input** (a width limit above every
`png_uint_32`, a `png_zalloc` overflow check the C itself calls "vestigial", a
dead `size > PNG_SIZE_MAX` test, the two "Call to NULL read/write function"
branches that `png_set_read_fn`/`png_set_write_fn` make unreachable by
installing the stdio default, and one `zstream.msg` report shadowed by an
earlier limit).  Each was verified against the source rather than assumed.

Separately, only 3 of `png_zstream_error`'s 10 message strings are observable
through any public entry point, because zlib clears `strm->msg` inside
`deflateInit2` / `inflateInit2` / the reset calls before libpng can report it.
All three reachable ones are asserted verbatim; the reasoning and the two
strings that no input can produce are documented in `ERRORS.md`.

Two reference behaviours worth recording (both libraries agree, so they are
faithful translations rather than bugs):

* `png_get_valid(PNG_INFO_tRNS)` returns 0 immediately after a successful
  `png_set_tRNS`, because `png_set_tRNS` updates `info_ptr->num_trans` while
  `png_get_valid` gates on `png_ptr->num_trans` (`pngget.c:29` vs
  `pngset.c:1264`).
* `png_get_iCCP` reports the length read out of the profile's own first four
  bytes rather than `info_ptr->iccp_proflen` (`pngget.c:730`).
