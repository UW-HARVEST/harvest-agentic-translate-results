# CONFIGS.md — configuration-surface table (valid inputs)

## Axes the C code actually branches / depends on

Derived from `c_src/include/lib.h` + `c_src/src/lib.c`:

* **Public entry points** — exactly one, and it *is* the lowest level one:
  `char *custom_strdup(const char *str)`. There is no convenience wrapper, no
  context/handle object, no init/teardown, no callback, no lower-level helper
  (no `static` functions in the TU).
* **Runtime options / modes / flags** — none. The function takes a single
  `const char *` and has no option struct, no global setter, no environment
  variable, no byte-order or format selector. So there is no option
  cross-product to enumerate; the configuration surface is entirely the shape
  and content of the input buffer plus the caller-visible properties of the
  returned block.
* **Branches taken** — `if(!str)` (see `ERRORS.md` E1) and `if(!newstr)`
  (`ERRORS.md` E2). All other behaviour is data-driven through
  `strlen`/`malloc`/`memcpy`, so the meaningful axes are:
  * length: 0 (empty), 1, small, word/16-byte/32-byte SIMD boundaries of
    `strlen`/`memcpy`, page boundaries, multi-megabyte;
  * content: NUL placement (only the first NUL matters — trailing bytes after
    it must NOT be copied), byte values incl. `0x80..0xFF` / invalid UTF-8,
    `0x7F`, embedded newlines;
  * source pointer alignment / offset within its buffer;
  * mapping shape: NUL as the last readable byte before an unmapped page
    (over-read detection);
  * call sequencing: repeated calls, interleaved C/Rust calls, aliasing between
    successive results (no shared state, distinct blocks);
  * post-conditions on the result: allocated with the *C* allocator (must be
    releasable by `free()` from the test process), independent of the source.

## Configuration rows

Each row is exercised against **both** `.so`s via `libloading` and compared
byte-for-byte. Rows marked "randomized" use many inputs per row from a seeded
deterministic PRNG (SplitMix64, fixed seed `0x5EED_1234_ABCD_EF01`).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `custom_strdup` | empty string `""` (len 0) | `cfg_c1_empty_string` | [x] |
| C2 | `custom_strdup` | every single-byte string `"\x01".."\xFF"` (255 inputs, all non-NUL byte values) | `cfg_c2_all_single_bytes` | [x] |
| C3 | `custom_strdup` | random printable-ASCII strings, lengths 1..=64, 400 randomized inputs | `cfg_c3_random_ascii_short` | [x] |
| C4 | `custom_strdup` | random arbitrary non-NUL bytes (`0x01..=0xFF`, invalid UTF-8 included), lengths 1..=256, 400 randomized inputs | `cfg_c4_random_binary` | [x] |
| C5 | `custom_strdup` | exhaustive lengths 0..=300 of pseudo-random bytes (covers word/16/32/64-byte `strlen`/`memcpy` step boundaries) | `cfg_c5_exhaustive_small_lengths` | [x] |
| C6 | `custom_strdup` | page-size boundary lengths: 4093..4099, 8189..8195, 65535..65537, plus ±1 around 1024/2048 | `cfg_c6_page_boundary_lengths` | [x] |
| C7 | `custom_strdup` | large inputs: 1 MiB and 4 MiB random content (malloc goes through `mmap` for these sizes) | `cfg_c7_large_inputs` | [x] |
| C8 | `custom_strdup` | source pointer at byte offsets 0..=16 inside a heap buffer (unaligned source), randomized contents | `cfg_c8_unaligned_source_offsets` | [x] |
| C9 | `custom_strdup` | trailing garbage after the terminating NUL (bytes after the first NUL must not be copied); first NUL at every position 0..=64 of a 128-byte junk-filled buffer | `cfg_c9_trailing_garbage_after_nul` | [x] |
| C10 | `custom_strdup` | NUL terminator is the last readable byte of a mapped region, next page unmapped (proves both read exactly `strlen+1` bytes); lengths 0..=64 at the page edge | `cfg_c10_nul_at_page_edge` | [x] |
| C11 | `custom_strdup` | repeated / interleaved invocation: 500 randomized calls alternating C, Rust, C, Rust with results kept alive simultaneously (no shared state, distinct non-overlapping blocks, source never modified) | `cfg_c11_interleaved_repeated_calls` | [x] |
| C12 | `custom_strdup` | result-block post-conditions: `!= NULL`, `!= str`, `free()`-able by the caller (C allocator), writable, and writing to it leaves the source intact — checked for both `.so`s over randomized inputs | `cfg_c12_result_block_properties` | [x] |
| C13 | `custom_strdup` | source in read-only mapped memory (`mprotect(PROT_READ)`) — proves the function does not write through `str` | `cfg_c13_readonly_source` | [x] |
| C14 | `custom_strdup` | source is a static/`.rodata` string literal and a stack buffer (non-heap storage classes) | `cfg_c14_non_heap_sources` | [x] |
| C15 | `custom_strdup` | idempotence/round-trip: duplicate the result of a duplicate (`custom_strdup(custom_strdup(x))`) across C→Rust and Rust→C, 200 randomized inputs | `cfg_c15_chained_duplication` | [x] |

## Build configurations

`Cargo.toml` has no `[features]`; `CMakeLists.txt` has no `option()` and the C
source has no `#ifdef`. The only two (identical) Cargo feature combinations are
the default set and `--no-default-features`; every row above is run under both
(see `run_all.sh`).
